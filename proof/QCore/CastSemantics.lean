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

import QCore.CastRepresentation

/-! # One formal cast semantics (CAST-FORMALIZATION, rule 12h)

Card CAST-FORMALIZATION refines rule 11b's kind table
(`QCore.CastRepresentation.castTable`) into one **conversion class** per
C11 §6.3 / §6.5.4 conversion, decided from Clang's cast kind and the operand's
and result's type classes (`cast_semantics::classify`), and states, once, what
each class does to an identity and to an access (`cast_semantics::CONVERSION_TABLE`).
Every route-G position consults this one table.

Stated here:
* **Classification** (`classify`), bound to the Rust classifier by
  `classifyVectors` (the Rust test renders each of its vectors' type classes in
  this syntax and finds the row here, in order).
* **Identity.** The view and round-trip rows (`byteView`, `typedView`,
  `punnedView`, `qualifier`) keep the operand's identity, and the round trip
  through `void *` or a character pointer yields the same pointer
  (C11 6.3.2.3p1, p7). A pointer converted to an integer loses its provenance:
  the integer never carries an identity, and a tracked pointer's conversion is
  an escape, refused. An integer converted to a pointer never yields an
  identity. A null pointer constant has none.
* **Access.** An access through a converted pointer is admitted with no
  premise only at the same type or through a character type (6.5p7); a typed
  view of `void *`/character storage is admitted only under
  `P-EFFECTIVE-TYPE`; a punned view (an incompatible effective type) and a
  function/object conversion are refused, never certified.
* **Effect only.** `(void) e` keeps `e`'s events and has no value; no
  conversion adds, drops or reorders an event.
* **Agreement** with rule 11b: every kind a class refines is a `castTable` row
  (or a decay owned elsewhere), with the provenance flag the class's identity
  effect implies.
* **W45-SR5** (`W45-SR5-SCALAR-READ-STAR-THROUGH-CAST`): a cast is transparent
  to the dereference count, so `*(T *)q->f` makes two dereferences and is
  never rule 5j's one; `sr5Vectors` binds the token reader.

Premises stated, not proved: faithful Clang extraction of cast kinds and type
spellings (trust boundary 28), C11 6.3.2.3p7 (a correctly aligned conversion,
`P-ALIGNED-POINTER-CONVERSION`), and `P-EFFECTIVE-TYPE` where an admission
names it. -/
namespace QCore.CastSemantics

/-! ## Type classes and the classifier -/

inductive Pointee where
  | void
  | character (spelling : String)
  | function
  | object (key : String)
  deriving DecidableEq, Repr

inductive TypeClass where
  | scalar
  | void
  /-- A pointer, its pointee, and whether the pointee is `_Atomic`. -/
  | pointer (pointee : Pointee) (atomic : Bool)
  | other
  deriving DecidableEq, Repr

inductive ConvClass where
  | scalar
  | nullTest
  | pointerToInteger
  | integerToPointer
  | nullPointer
  | byteView
  | typedView
  | punnedView
  | qualifier
  | functionPointer
  | functionObject
  | discard
  | atomic
  | addressSpace
  | arrayDecay
  | functionDecay
  | unlisted
  deriving DecidableEq, Repr

def TypeClass.isPointer : TypeClass → Bool
  | .pointer _ _ => true
  | _ => false

def scalarKinds : List String :=
  ["IntegralCast", "IntegralToBoolean", "FloatingToBoolean", "IntegralToFloating",
   "FloatingToIntegral", "FloatingCast", "NoOp", "BooleanToSignedIntegral"]

/-- The pointer-to-pointer refinement of `BitCast`/`NoOp`. -/
def pointerPair : Pointee → Pointee → ConvClass
  | .function, .function => .functionPointer
  | .function, _ => .functionObject
  | _, .function => .functionObject
  | a, b =>
    if a = b then .qualifier
    else match a, b with
      | _, .void => .byteView
      | _, .character _ => .byteView
      | .void, .object _ => .typedView
      | .character _, .object _ => .typedView
      | _, _ => .punnedView

/-- `cast_semantics::classify_classes`. -/
def classify (kind : String) (src dst : TypeClass) : ConvClass :=
  if kind = "PointerToBoolean" ∧ src.isPointer then .nullTest
  else if kind = "PointerToIntegral" ∧ src.isPointer then .pointerToInteger
  else if kind = "IntegralToPointer" ∧ dst.isPointer then .integerToPointer
  else if kind = "NullToPointer" ∧ dst.isPointer then .nullPointer
  else if kind = "ToVoid" then .discard
  else if kind = "AtomicToNonAtomic" ∨ kind = "NonAtomicToAtomic" then .atomic
  else if kind = "AddressSpaceConversion" then .addressSpace
  else if kind = "ArrayToPointerDecay" then .arrayDecay
  else if kind = "FunctionToPointerDecay" then .functionDecay
  else if kind = "BitCast" ∨ kind = "NoOp" then
    match src, dst with
    | .scalar, .scalar => if kind = "NoOp" then .scalar else .unlisted
    | .pointer a atomicA, .pointer b atomicB =>
      if atomicA || atomicB then .atomic else pointerPair a b
    | _, _ => .unlisted
  else if kind ∈ scalarKinds ∧ src = .scalar ∧ dst = .scalar then .scalar
  else .unlisted

/-- A function pointer never converts to an object pointer, nor back. -/
theorem function_object_refused (kind : String) (k : String) (a b : Bool)
    (hk : kind = "BitCast" ∨ kind = "NoOp") :
    classify kind (.pointer .function a) (.pointer (.object k) b) ≠ .typedView ∧
      classify kind (.pointer (.object k) a) (.pointer .function b) ≠ .typedView ∧
      classify kind (.pointer .void a) (.pointer .function b) ≠ .qualifier := by
  rcases hk with rfl | rfl <;> cases a <;> cases b <;>
    simp [classify, pointerPair, TypeClass.isPointer]

/-- An object pointer converted to `void *` or a character pointer is a byte
view; the way back is a typed view; between two other object types, a pun. -/
theorem view_rows (k k' : String) (hk : k ≠ k') :
    classify "BitCast" (.pointer (.object k) false) (.pointer .void false) = .byteView ∧
      classify "BitCast" (.pointer .void false) (.pointer (.object k) false) = .typedView ∧
      classify "BitCast" (.pointer (.object k) false) (.pointer (.object k') false)
        = .punnedView := by
  refine ⟨?_, ?_, ?_⟩ <;> simp [classify, pointerPair, TypeClass.isPointer, hk]

/-! ## The table -/

inductive Identity where
  | data
  | preserved
  | lost
  | absent
  | noValue
  | refused
  deriving DecidableEq, Repr

inductive Access where
  | notPointer
  | same
  | character
  | effectiveType
  | punned
  | untracked
  | null
  | callee
  | refused
  deriving DecidableEq, Repr

/-- `CONVERSION_TABLE`: class, the Clang kinds it refines, identity, access. -/
def conversionTable : List (ConvClass × List String × Identity × Access) := [
  (.scalar, ["IntegralCast", "IntegralToBoolean", "FloatingToBoolean", "IntegralToFloating", "FloatingToIntegral", "FloatingCast", "NoOp"], .data, .notPointer),
  (.nullTest, ["PointerToBoolean"], .lost, .notPointer),
  (.pointerToInteger, ["PointerToIntegral"], .lost, .notPointer),
  (.integerToPointer, ["IntegralToPointer"], .absent, .untracked),
  (.nullPointer, ["NullToPointer"], .absent, .null),
  (.byteView, ["BitCast", "NoOp"], .preserved, .character),
  (.typedView, ["BitCast", "NoOp"], .preserved, .effectiveType),
  (.punnedView, ["BitCast"], .preserved, .punned),
  (.qualifier, ["NoOp", "BitCast"], .preserved, .same),
  (.functionPointer, ["BitCast", "NoOp"], .preserved, .callee),
  (.functionObject, ["BitCast"], .refused, .refused),
  (.discard, ["ToVoid"], .noValue, .notPointer),
  (.atomic, ["AtomicToNonAtomic", "NonAtomicToAtomic"], .refused, .refused),
  (.addressSpace, ["AddressSpaceConversion"], .refused, .refused),
  (.arrayDecay, ["ArrayToPointerDecay"], .refused, .refused),
  (.functionDecay, ["FunctionToPointerDecay"], .preserved, .callee),
  (.unlisted, [], .refused, .refused)
]

theorem classes_unique : (conversionTable.map (·.1)).Nodup := by decide

def identityOfClass (c : ConvClass) : Identity :=
  match conversionTable.find? (·.1 == c) with
  | some (_, _, identity, _) => identity
  | none => .refused

def accessOfClass (c : ConvClass) : Access :=
  match conversionTable.find? (·.1 == c) with
  | some (_, _, _, access) => access
  | none => .refused

/-- Agreement with rule 11b's kind table: every kind a class refines is a
`castTable` row (or the function decay, CG22's), a class keeping an identity
refines only kept-provenance rows (but a scalar `NoOp`), a class losing one
only lost-provenance rows, and a class with no identity only rows with none. -/
def provenanceOf (kind : String) : Option CastRepresentation.Provenance :=
  (CastRepresentation.castTable.find? (·.1 == kind)).map (·.2.2.1)

def agreesRow : ConvClass × List String × Identity × Access → Bool
  | (c, kinds, identity, _) =>
    kinds.all fun kind =>
      kind == "FunctionToPointerDecay" ||
        match provenanceOf kind, identity with
        | some .kept, .preserved => true
        | some _, .preserved => c == .functionPointer
        | some .lost, .lost => true
        | some .notApplicable, .absent => true
        | some .notApplicable, .noValue => true
        | some .notApplicable, .data => true
        | some .kept, .data => kind == "NoOp"
        | some _, .refused => true
        | _, _ => false

theorem agrees : conversionTable.all agreesRow = true := by decide

/-! ## Values: identity and provenance -/

/-- A value: an integer (never an identity), a pointer at an address with the
identity it carries (`none`: untracked) and its pointee class, the null
pointer, or no value. -/
inductive Val where
  | int (n : Int)
  | ptr (addr : Nat) (identity : Option Nat) (pointee : Pointee)
  | null
  | none
  deriving DecidableEq, Repr

def identityOf : Val → Option Nat
  | .ptr _ identity _ => identity
  | _ => Option.none

inductive Outcome where
  | value (v : Val)
  | refuse
  deriving DecidableEq, Repr

def Outcome.identity : Outcome → Option Nat
  | .value v => identityOf v
  | .refuse => Option.none

/-- One conversion of `v` to a result whose pointee class is `target`
(ignored for a non-pointer result). `transfer` is the numeric conversion,
never modelled here (CG20). -/
def convert (transfer : Int → Int) (c : ConvClass) (target : Pointee) (v : Val) : Outcome :=
  match c, v with
  | .scalar, .int n => .value (.int (transfer n))
  | .nullTest, .ptr _ _ _ => .value (.int 1)
  | .nullTest, .null => .value (.int 0)
  | .pointerToInteger, .ptr addr Option.none _ => .value (.int addr)
  | .pointerToInteger, .ptr _ (some _) _ => .refuse
  | .pointerToInteger, .null => .value (.int 0)
  | .integerToPointer, .int n => .value (if n = 0 then .null else .ptr n.toNat Option.none target)
  | .nullPointer, _ => .value .null
  | .byteView, .ptr addr identity _ => .value (.ptr addr identity target)
  | .typedView, .ptr addr identity _ => .value (.ptr addr identity target)
  | .punnedView, .ptr addr identity _ => .value (.ptr addr identity target)
  | .qualifier, .ptr addr identity _ => .value (.ptr addr identity target)
  | .byteView, .null => .value .null
  | .typedView, .null => .value .null
  | .punnedView, .null => .value .null
  | .qualifier, .null => .value .null
  | .discard, _ => .value .none
  | _, _ => .refuse

/-- The view and round-trip rows keep the identity they convert. -/
theorem view_preserves_identity (transfer : Int → Int) (c : ConvClass) (target : Pointee)
    (v v' : Val) (hc : c = .byteView ∨ c = .typedView ∨ c = .punnedView ∨ c = .qualifier)
    (h : convert transfer c target v = .value v') : identityOf v' = identityOf v := by
  rcases hc with rfl | rfl | rfl | rfl <;> cases v <;> simp [convert] at h <;>
    subst h <;> rfl

/-- The round trip through `void *` (or a character pointer) and back to the
original pointee yields the same pointer (C11 6.3.2.3p1, p7). -/
theorem round_trip_same_pointer (transfer : Int → Int) (addr : Nat) (identity : Option Nat)
    (original via : Pointee) :
    (match convert transfer .byteView via (.ptr addr identity original) with
      | .value v => convert transfer .typedView original v
      | .refuse => .refuse) = .value (.ptr addr identity original) := rfl

/-- A pointer converted to an integer loses its provenance: whatever the
outcome, it carries no identity, and a tracked pointer's conversion refuses. -/
theorem pointer_to_integer_loses_provenance (transfer : Int → Int) (target : Pointee) (v : Val) :
    (convert transfer .pointerToInteger target v).identity = Option.none := by
  cases v with
  | ptr addr identity p => cases identity <;> rfl
  | _ => rfl

theorem tracked_pointer_to_integer_refused (transfer : Int → Int) (target : Pointee)
    (addr i : Nat) (p : Pointee) :
    convert transfer .pointerToInteger target (.ptr addr (some i) p) = .refuse := rfl

/-- An integer converted to a pointer never yields an identity. -/
theorem integer_to_pointer_no_identity (transfer : Int → Int) (target : Pointee) (v : Val) :
    (convert transfer .integerToPointer target v).identity = Option.none := by
  cases v with
  | int n => by_cases hn : n = 0 <;> simp [convert, hn, Outcome.identity, identityOf]
  | _ => rfl

/-- A null pointer constant has no identity. -/
theorem null_pointer_no_identity (transfer : Int → Int) (target : Pointee) (v : Val) :
    (convert transfer .nullPointer target v).identity = Option.none := by
  cases v <;> rfl

/-- A function/object conversion, and every refused class, refuses. -/
theorem refused_classes_refuse (transfer : Int → Int) (target : Pointee) (v : Val)
    (c : ConvClass)
    (hc : c = .functionObject ∨ c = .atomic ∨ c = .addressSpace ∨ c = .arrayDecay ∨
      c = .unlisted) :
    convert transfer c target v = .refuse := by
  rcases hc with rfl | rfl | rfl | rfl | rfl <;> cases v <;> rfl

/-- Each refused class's table identity is `refused`. -/
theorem refused_rows (c : ConvClass)
    (hc : c = .functionObject ∨ c = .atomic ∨ c = .addressSpace ∨ c = .arrayDecay ∨
      c = .unlisted) :
    identityOfClass c = .refused := by
  rcases hc with rfl | rfl | rfl | rfl | rfl <;> decide

/-! ## Access through a converted pointer -/

inductive AccessOutcome where
  | admitted
  | admittedUnderEffectiveType
  | refused
  deriving DecidableEq, Repr

/-- An lvalue access through a conversion's result. -/
def accessDecision : Access → AccessOutcome
  | .same => .admitted
  | .character => .admitted
  | .effectiveType => .admittedUnderEffectiveType
  | _ => .refused

/-- An access is certified with no premise only at the same type or through a
character type (C11 6.5p7). -/
theorem admitted_access_without_premise (c : ConvClass)
    (h : accessDecision (accessOfClass c) = .admitted) :
    accessOfClass c = .same ∨ accessOfClass c = .character := by
  revert h
  cases c <;> decide

/-- A view is admitted under `P-EFFECTIVE-TYPE` and only a typed view is. -/
theorem view_access_needs_the_premise (c : ConvClass)
    (h : accessDecision (accessOfClass c) = .admittedUnderEffectiveType) :
    c = .typedView := by
  revert h
  cases c <;> decide

/-- A punned view (an incompatible effective type) is refused, never
certified, with or without the premise. -/
theorem punned_access_refused : accessDecision (accessOfClass .punnedView) = .refused := by
  decide

/-! ## Effect only -/

/-- An elaborated part: its events, in order, and its value. -/
structure Part where
  events : List Nat
  value : Val
  deriving DecidableEq, Repr

/-- A conversion of a part: the operand's events, unchanged, and the
converted value; `none` where the conversion refuses. -/
def castPart (transfer : Int → Int) (c : ConvClass) (target : Pointee) (p : Part) : Option Part :=
  match convert transfer c target p.value with
  | .value v => some { events := p.events, value := v }
  | .refuse => Option.none

/-- No conversion adds, drops or reorders an event. -/
theorem cast_keeps_events (transfer : Int → Int) (c : ConvClass) (target : Pointee)
    (p q : Part) (h : castPart transfer c target p = some q) : q.events = p.events := by
  simp only [castPart] at h
  split at h
  · cases h; rfl
  · cases h

/-- `(void) e` is `e` evaluated for its effect only: its events and no value. -/
theorem discard_is_effect_only (transfer : Int → Int) (target : Pointee) (p : Part) :
    castPart transfer .discard target p = some { events := p.events, value := .none } := by
  cases p with
  | mk events value => cases value <;> rfl

/-! ## Shared classification vectors -/

/-- `CLASSIFY_VECTORS`: kind, operand class, result class, class. The Rust test
renders each of its vectors' type classes in this syntax and reads the rows. -/
def classifyVectors : List (String × TypeClass × TypeClass × ConvClass) := [
  ("IntegralCast", .scalar, .scalar, .scalar),
  ("NoOp", .scalar, .scalar, .scalar),
  ("FloatingToIntegral", .scalar, .scalar, .scalar),
  ("IntegralToBoolean", .scalar, .scalar, .scalar),
  ("PointerToBoolean", .pointer (.character "char") false, .scalar, .nullTest),
  ("PointerToIntegral", .pointer .void false, .scalar, .pointerToInteger),
  ("IntegralToPointer", .scalar, .pointer (.character "char") false, .integerToPointer),
  ("NullToPointer", .scalar, .pointer .void false, .nullPointer),
  ("BitCast", .pointer (.object "struct S") false, .pointer .void false, .byteView),
  ("BitCast", .pointer (.object "struct S") false, .pointer (.character "char") false, .byteView),
  ("BitCast", .pointer (.object "struct S") false, .pointer (.character "unsigned char") false, .byteView),
  ("BitCast", .pointer (.object "struct S") false, .pointer (.character "unsigned char") false, .byteView),
  ("BitCast", .pointer .void false, .pointer (.object "struct S") false, .typedView),
  ("BitCast", .pointer (.character "char") false, .pointer (.object "struct S") false, .typedView),
  ("BitCast", .pointer (.character "unsigned char") false, .pointer (.object "int") false, .typedView),
  ("BitCast", .pointer (.object "struct A") false, .pointer (.object "struct B") false, .punnedView),
  ("BitCast", .pointer (.object "int") false, .pointer (.object "long") false, .punnedView),
  ("BitCast", .pointer (.object "void *") false, .pointer (.object "char *") false, .punnedView),
  ("NoOp", .pointer (.object "struct S") false, .pointer (.object "struct S") false, .qualifier),
  ("NoOp", .pointer (.character "char") false, .pointer (.character "char") false, .qualifier),
  ("BitCast", .pointer (.object "struct S") false, .pointer (.object "struct S") false, .qualifier),
  ("BitCast", .pointer .function false, .pointer .function false, .functionPointer),
  ("BitCast", .pointer .function false, .pointer .void false, .functionObject),
  ("BitCast", .pointer .void false, .pointer .function false, .functionObject),
  ("ToVoid", .scalar, .void, .discard),
  ("ArrayToPointerDecay", .other, .pointer (.character "char") false, .arrayDecay),
  ("BitCast", .pointer (.object "_Atomic(int)") true, .pointer (.object "int") false, .atomic),
  ("Dependent", .scalar, .scalar, .unlisted)
]

theorem classify_vectors_hold :
    classifyVectors.all (fun (k, s, d, c) => classify k s d == c) = true := by decide

/-! ## W45-SR5: the dereference count through a cast -/

/-- An lvalue/value spelling as rule 5j reads a `return` operand. -/
inductive Expr where
  | name (n : String)
  | member (base : Expr) (field : String) (arrow : Bool)
  | deref (e : Expr)
  | index (base : Expr)
  | cast (type : String) (e : Expr)
  deriving DecidableEq, Repr

/-- The dereferences C performs evaluating the expression: each `*`, `->` and
subscript is one; a member `.` and a cast are none. -/
def derefs : Expr → Nat
  | .name _ => 0
  | .member e _ true => derefs e + 1
  | .member e _ false => derefs e
  | .deref e => derefs e + 1
  | .index e => derefs e + 1
  | .cast _ e => derefs e

/-- A cast is transparent to the count. -/
theorem cast_transparent (t : String) (e : Expr) : derefs (.cast t e) = derefs e := rfl

/-- Rule 5j admits a may-bind root only as the root of exactly one
dereference. -/
def oneDereference (e : Expr) : Bool := derefs e == 1

/-- `*(T *)q->f` dereferences twice: rule 5j refuses it. -/
theorem star_through_cast_is_two (t q f : String) :
    derefs (.deref (.cast t (.member (.name q) f true))) = 2 ∧
      oneDereference (.deref (.cast t (.member (.name q) f true))) = false := by
  constructor <;> rfl

/-- `sr5Vectors`: a `return` operand's tokens (the Rust reader's spelling, `q`
the may-bind name), its expression, and the dereference count. The Rust test
asserts rule 5j admits each exactly when the count is one. -/
def sr5Vectors : List (String × Expr × Nat) := [
  ("q - > f", .member (.name "q") "f" true, 1),
  ("* ( int * ) q - > f", .deref (.cast "int *" (.member (.name "q") "f" true)), 2),
  ("* ( int * ) q [ 0 ]", .deref (.cast "int *" (.index (.name "q"))), 2),
  ("* ( int * ) ( q - > f )", .deref (.cast "int *" (.member (.name "q") "f" true)), 2),
  ("( long ) q - > f", .cast "long" (.member (.name "q") "f" true), 1),
  ("* * ( int * * ) q - > f", .deref (.deref (.cast "int **" (.member (.name "q") "f" true))), 3)
]

theorem sr5_vectors_hold : sr5Vectors.all (fun (_, e, n) => derefs e == n) = true := by decide

end QCore.CastSemantics
