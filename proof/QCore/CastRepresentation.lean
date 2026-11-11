-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

/-! # Casts as representations and type-query operands (CG08, rule 11b)

Card CG08-CASTS-AND-TYPE-QUERIES. The route-G elaborator reads a C cast by
its row of `cast_representation::CAST_TABLE` and a `sizeof`/`_Alignof` by its
operand's evaluation class (`cast_representation::query_operand`). Bound to
Rust by `cast_representation::tests` (`castTable` against `CAST_TABLE`,
`queryVectors` through Clang and `query_operand`, `constantVectors` against
Clang's own constant evaluator) and by `legacy_call_argument_access`'s test of
`foldVectors` against `fold_type_sizes`.

Stated here:
* **Casts.** No admitted cast changes the custody state. A pointer converted
  to an integer is refused when it carries an identity (an escape), and is
  data otherwise. An integer converted to a pointer yields a pointer with no
  identity. `PointerToBoolean` is exactly the null test. A pointer-to-pointer
  representation keeps the identity. A kind the table does not list refuses.
* **Type queries.** An unevaluated operand contributes no event; an evaluated
  (variably modified) operand contributes its size expressions' events exactly
  once, and only reads; `_Alignof` is never evaluated.
* **Consumed values.** A consumer of a query's number takes Clang's evaluated
  constant or refuses; the operand-class marker is never consumed.
* **The type-size fold** (`SIZEOF-TYPE-FOLD-NO-LEAN`). `fold_type_sizes` folds
  an operand only when every token is `*`, a type keyword, a tag or a typedef
  the reader read; a non-typedef, non-tag name never folds.

Premises stated, not proved: faithful Clang extraction of cast kinds, types
and constant values (trust boundary 28), and C11 6.3.2.3p7 (a correctly
aligned object-pointer conversion keeps the address). Numeric transfer is a
parameter (`transfer`), never a claim (CG20). -/
namespace QCore.CastRepresentation

/-! ## The table -/

inductive Preservation where
  | preserving
  | possiblyLossy
  deriving DecidableEq, Repr

inductive Provenance where
  | kept
  | lost
  | notApplicable
  deriving DecidableEq, Repr

inductive Admission where
  | scalar
  | nullTest
  | pointerToInteger
  | integerToPointer
  | pointerValue
  | nullPointer
  | discard
  | refused (card : String)
  deriving DecidableEq, Repr

/-- `CAST_TABLE`. -/
def castTable : List (String × Preservation × Provenance × Admission) := [
  ("IntegralCast", .possiblyLossy, .notApplicable, .scalar),
  ("IntegralToBoolean", .possiblyLossy, .notApplicable, .scalar),
  ("FloatingToBoolean", .possiblyLossy, .notApplicable, .scalar),
  ("IntegralToFloating", .possiblyLossy, .notApplicable, .scalar),
  ("FloatingToIntegral", .possiblyLossy, .notApplicable, .scalar),
  ("FloatingCast", .possiblyLossy, .notApplicable, .scalar),
  ("PointerToBoolean", .possiblyLossy, .lost, .nullTest),
  ("PointerToIntegral", .possiblyLossy, .lost, .pointerToInteger),
  ("IntegralToPointer", .possiblyLossy, .notApplicable, .integerToPointer),
  ("BitCast", .preserving, .kept, .pointerValue),
  ("NoOp", .preserving, .kept, .pointerValue),
  ("NullToPointer", .preserving, .notApplicable, .nullPointer),
  ("ToVoid", .preserving, .notApplicable, .discard),
  ("AtomicToNonAtomic", .preserving, .kept, .refused "CG19"),
  ("NonAtomicToAtomic", .preserving, .kept, .refused "CG19"),
  ("AddressSpaceConversion", .preserving, .kept, .refused "CG08"),
  ("ArrayToPointerDecay", .preserving, .kept, .refused "CG16")
]

theorem kinds_unique : (castTable.map (·.1)).Nodup := by decide

/-- The admission of a kind; an unlisted kind is refused. -/
def admissionOf (kind : String) : Admission :=
  match castTable.find? (·.1 == kind) with
  | some (_, _, _, admission) => admission
  | none => .refused "unlisted"

def Admission.isRefused : Admission → Bool
  | .refused _ => true
  | _ => false

/-- Only a pointer-to-pointer representation keeps provenance, and every row
that loses it is a pointer read into a non-pointer. -/
theorem provenance_rows :
    ∀ row ∈ castTable,
      (row.2.2.1 = .kept → row.2.2.2 = .pointerValue ∨ row.2.2.2.isRefused = true) ∧
      (row.2.2.1 = .lost →
        row.2.2.2 = .nullTest ∨ row.2.2.2 = .pointerToInteger) := by
  decide

/-! ## Values and custody -/

/-- A value: an integer, a pointer to an object at an address with the
identity the custody state tracks for it (`none`: untracked), the null
pointer, or no value. -/
inductive Val where
  | int (n : Int)
  | ptr (addr : Nat) (identity : Option Nat)
  | null
  | none
  deriving DecidableEq, Repr

/-- The abstract custody state: the live identities and those escaped. -/
structure Custody where
  live : List Nat
  escaped : List Nat
  deriving DecidableEq, Repr

inductive Outcome where
  | value (v : Val) (c : Custody)
  | refuse
  deriving DecidableEq, Repr

def identityOf : Val → Option Nat
  | .ptr _ identity => identity
  | _ => Option.none

/-- C11 6.3.1.2: a pointer's truth value is 0 exactly for the null pointer
(an object's address is never null). -/
def nullTest : Val → Int
  | .null => 0
  | .ptr _ _ => 1
  | _ => 0

/-- One cast of `v` read by admission `a` in custody state `c`. `transfer` is
the numeric conversion, never modelled here (CG20). -/
def apply (transfer : Int → Int) (a : Admission) (v : Val) (c : Custody) : Outcome :=
  match a, v with
  | .scalar, .int n => .value (.int (transfer n)) c
  | .nullTest, .ptr addr identity => .value (.int (nullTest (.ptr addr identity))) c
  | .nullTest, .null => .value (.int (nullTest .null)) c
  | .pointerToInteger, .ptr addr Option.none => .value (.int addr) c
  | .pointerToInteger, .ptr _ (some _) => .refuse
  | .pointerToInteger, .null => .value (.int 0) c
  | .integerToPointer, .int n => .value (if n = 0 then .null else .ptr n.toNat Option.none) c
  | .pointerValue, .ptr addr identity => .value (.ptr addr identity) c
  | .pointerValue, .null => .value .null c
  | .nullPointer, .int 0 => .value .null c
  | .discard, _ => .value .none c
  | _, _ => .refuse

/-- A representation cast has no custody effect. -/
theorem cast_has_no_custody_effect (transfer : Int → Int) (a : Admission) (v v' : Val)
    (c c' : Custody) (h : apply transfer a v c = .value v' c') : c' = c := by
  cases a <;> cases v <;> simp [apply] at h <;> (try split at h) <;> simp_all

/-- A pointer carrying an identity converted to an integer is an escape: it is
refused, never read as data. -/
theorem tracked_pointer_to_integer_refused (transfer : Int → Int) (addr i : Nat)
    (c : Custody) : apply transfer .pointerToInteger (.ptr addr (some i)) c = .refuse := rfl

/-- An untracked pointer converted to an integer is data: its address, and
the custody state unchanged. -/
theorem untracked_pointer_to_integer_is_data (transfer : Int → Int) (addr : Nat)
    (c : Custody) :
    apply transfer .pointerToInteger (.ptr addr Option.none) c = .value (.int addr) c := rfl

/-- An integer converted to a pointer carries no identity. -/
theorem integer_to_pointer_has_no_identity (transfer : Int → Int) (v v' : Val)
    (c c' : Custody) (h : apply transfer .integerToPointer v c = .value v' c') :
    identityOf v' = Option.none := by
  cases v <;> simp [apply] at h
  case int n =>
    obtain ⟨hv, _⟩ := h
    subst hv
    by_cases hn : n = 0 <;> simp [hn, identityOf]

/-- `PointerToBoolean` is the null test: 0 exactly for the null pointer. -/
theorem pointer_to_boolean_is_the_null_test (transfer : Int → Int) (v : Val)
    (c : Custody) (hp : v = .null ∨ ∃ addr identity, v = .ptr addr identity) :
    apply transfer .nullTest v c = .value (.int (nullTest v)) c ∧
      (nullTest v = 0 ↔ v = .null) := by
  rcases hp with h | ⟨addr, identity, h⟩ <;> subst h <;> simp [apply, nullTest]

/-- A pointer-to-pointer representation keeps the identity it converts. -/
theorem pointer_value_keeps_identity (transfer : Int → Int) (v v' : Val) (c c' : Custody)
    (h : apply transfer .pointerValue v c = .value v' c') : identityOf v' = identityOf v := by
  cases v <;> simp [apply] at h <;> obtain ⟨rfl, _⟩ := h <;> rfl

/-- A kind the table does not list is refused, whatever its operand. -/
theorem unlisted_cast_refused (transfer : Int → Int) (kind : String) (v : Val) (c : Custody)
    (h : castTable.find? (·.1 == kind) = Option.none) :
    apply transfer (admissionOf kind) v c = .refuse := by
  simp [admissionOf, h, apply]

/-- A listed but refused row refuses too. -/
theorem refused_row_refused (transfer : Int → Int) (card : String) (v : Val) (c : Custody) :
    apply transfer (.refused card) v c = .refuse := by
  cases v <;> rfl

/-! ## Type-query operands -/

inductive Ev where
  | read (place : Nat)
  | write (place : Nat)
  | call
  deriving DecidableEq, Repr

def Ev.isRead : Ev → Bool
  | .read _ => true
  | _ => false

inductive Trait where
  | sizeOf
  | alignOf
  deriving DecidableEq, Repr

/-- The operand class `query_operand` reads from Clang: a type or an
expression; whether its type is variably modified; for an expression, whether
it is a plain name. -/
inductive QueryClass where
  | unevaluated
  | evaluated
  | refused
  deriving DecidableEq, Repr

def classify (trait : Trait) (typeOperand variablyModified plainName : Bool) : QueryClass :=
  if !variablyModified then .unevaluated
  else match trait with
    | .alignOf => .refused
    | .sizeOf => if typeOperand || plainName then .evaluated else .refused

/-- C11 6.5.3.4p2: a query whose operand type is not variably modified is not
evaluated. -/
theorem constant_type_unevaluated (trait : Trait) (typeOperand plainName : Bool) :
    classify trait typeOperand false plainName = .unevaluated := rfl

/-- `_Alignof` never evaluates its operand. -/
theorem alignof_never_evaluated (typeOperand variablyModified plainName : Bool) :
    classify .alignOf typeOperand variablyModified plainName ≠ .evaluated := by
  cases variablyModified <;> simp [classify]

/-- The events a query contributes: none for an unevaluated operand; its size
expressions' events, once, when evaluated and all reads; otherwise a refusal. -/
def queryEvents (cls : QueryClass) (operand : List Ev) : Option (List Ev) :=
  match cls with
  | .unevaluated => some []
  | .evaluated => if operand.all Ev.isRead then some operand else Option.none
  | .refused => Option.none

/-- An unevaluated operand has no event, whatever it spells (`sizeof(*p)`,
`sizeof f()`). -/
theorem unevaluated_operand_has_no_events (operand : List Ev) :
    queryEvents .unevaluated operand = some [] := rfl

/-- An evaluated operand's events occur exactly once each. -/
theorem evaluated_operand_once (operand out : List Ev)
    (h : queryEvents .evaluated operand = some out) (e : Ev) :
    out.count e = operand.count e := by
  simp only [queryEvents] at h
  split at h
  · cases h; rfl
  · cases h

/-- An evaluated operand with an effect (a call or a store) is refused. -/
theorem evaluated_effect_refused (operand : List Ev) (e : Ev) (he : e ∈ operand)
    (hn : e.isRead = false) : queryEvents .evaluated operand = Option.none := by
  simp only [queryEvents]
  have : operand.all Ev.isRead = false := by
    rw [List.all_eq_false]
    exact ⟨e, he, by simp [hn]⟩
  simp [this]

/-! ## Consumed values -/

/-- A query's value as a consumer receives it: Clang's evaluated constant, or
the operand-class marker of `type_query.rs`/`sizeof_operand.rs`. -/
inductive QueryValue where
  | clang (n : Nat)
  | marker
  deriving DecidableEq, Repr

/-- A consumer that needs the number (a bound, a branch) takes Clang's constant
or refuses. -/
def consume : QueryValue → Option Nat
  | .clang n => some n
  | .marker => Option.none

theorem consumed_value_is_clangs (v : QueryValue) (n : Nat) (h : consume v = some n) :
    v = .clang n := by
  cases v <;> simp [consume] at h
  rw [h]

theorem marker_never_consumed : consume .marker = Option.none := rfl

/-- `constantVectors`: a query expression over `constantPrelude` and Clang's
evaluated value (x86-64 Linux, the platform the vectors test runs on); the
Rust test evaluates each with Clang's constant evaluator. -/
def constantPrelude : String :=
  "struct S { int a; char c; void *p; }; typedef unsigned short u16; enum E { E0, E1 = 5, E2 };"

def constantVectors : List (String × Nat) := [
  ("sizeof(int)", 4),
  ("sizeof(char)", 1),
  ("sizeof(u16)", 2),
  ("sizeof(void *)", 8),
  ("sizeof(struct S)", 16),
  ("_Alignof(struct S)", 8),
  ("sizeof(((struct S *)0)->c)", 1),
  ("E2", 6)
]

theorem constant_vectors_consumed :
    ∀ row ∈ constantVectors, consume (.clang row.2) = some row.2 := by
  decide

/-- `queryVectors`: a function body whose first type query has the class; the
Rust test reads each through Clang and `query_operand`. -/
def queryVectors : List (String × QueryClass) := [
  ("unsigned long subject(int *p) { return sizeof(*p); }", .unevaluated),
  ("unsigned long subject(int *p) { return sizeof(int); }", .unevaluated),
  ("int f(void); unsigned long subject(void) { return sizeof(f()); }", .unevaluated),
  ("unsigned long subject(int n) { return sizeof(int[n]); }", .evaluated),
  ("unsigned long subject(int n) { int a[n]; return sizeof a; }", .evaluated),
  ("unsigned long subject(int n) { typedef int v[n]; return sizeof(v); }", .evaluated),
  ("unsigned long subject(int n) { return _Alignof(int[n]); }", .refused),
  ("unsigned long subject(int n, int (*q)[n]) { return sizeof(*q); }", .refused)
]

/-! ## The type-size fold (`SIZEOF-TYPE-FOLD-NO-LEAN`) -/

/-- One token of a `sizeof ( … )` operand as `fold_type_sizes` reads it. -/
inductive FoldToken where
  | star
  | keyword
  | tag
  | typedefName
  | other
  deriving DecidableEq, Repr

def admittedToken : FoldToken → Bool
  | .other => false
  | _ => true

/-- `fold_type_sizes`' admission: a non-empty operand of admitted tokens. -/
def folds (operand : List FoldToken) : Bool :=
  !operand.isEmpty && operand.all admittedToken

/-- A name that is neither a typedef the reader read nor a tag never folds. -/
theorem non_typedef_non_tag_never_folds (operand : List FoldToken)
    (h : FoldToken.other ∈ operand) : folds operand = false := by
  simp only [folds]
  have : operand.all admittedToken = false := by
    rw [List.all_eq_false]
    exact ⟨.other, h, by decide⟩
  simp [this]

/-- `foldVectors`: an argument over the fixed reader of
`a_type_size_needs_a_typedef_or_a_tag_not_a_file_scope_object` (`struct rec`,
`typedef … rec_t`, file-scope `g`, local `i`) and whether it folds. -/
def foldVectors : List (String × Bool) := [
  ("sizeof ( rec_t ) * i", true),
  ("sizeof ( struct rec ) * i", true),
  ("sizeof ( unsigned long ) * i", true),
  ("sizeof ( struct rec * ) * i", true),
  ("sizeof ( * g ) * i", false),
  ("sizeof ( g ) * i", false),
  ("sizeof ( i ) * i", false),
  ("sizeof ( int [ 4 ] ) * i", false)
]

end QCore.CastRepresentation
