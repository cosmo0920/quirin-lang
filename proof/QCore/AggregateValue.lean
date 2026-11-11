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

import QCore.AggregateOwnedField

/-!
# Aggregate values and initializers (card CG14-AGGREGATE-VALUES-AND-INITIALIZERS, gateway rule 11f)

The abstract rule implemented by `crates/qcore-c-gateway/src/aggregate_value.rs`
(`decide_copy`, `Leaf::admitted`, `decide_literal`, `string_extent`) and its
route-G adapter `legacy_aggregate_value.rs`.

* **Copies (M08).** A whole-record copy is field-wise: every member of the
  destination takes the source member's value. A copied pointer is an *alias*
  of the source's value: the copy adds no owner (`copy_adds_no_owner`) and every
  identity the destination then holds the source held (`a_copy_only_aliases`).
  Releasing both copies of a record holding an owned pointer is a double
  release (`releasing_both_copies_is_a_double_release`), which is why a copy of
  a record with a pointer member is refused unless something states which copy
  owns: the owned-field model makes such a record uncopyable
  (`QCore.AggregateOwnedField.an_aggregate_is_never_copied`, joined here as
  `the_owned_field_model_agrees`). A custody-free record (only scalars) moves no
  identity at all (`a_custody_free_copy_moves_no_identity`); that is the one
  class `decideCopy` admits.
* **Initializers (M09).** Clang's semantic initializer lists every member and
  element; one the source does not name is the zero of its type, and a zero
  pointer is null: zero-fill yields no identity (`zero_fill_has_no_identity`).
  An initializer is admitted only when every leaf is of an admitted class, and
  then the local holds no identity (`an_admitted_initializer_holds_no_identity`);
  every other class may denote one (`an_unadmitted_leaf_may_carry_an_identity`).
* **Compound literals (M10).** A literal's storage lives from its block's entry
  to its exit. It is admitted only at a borrowed formal of a call inside the
  block, and a borrow ends when the call returns: the literal's lifetime bounds
  its borrows (`a_literal_borrow_ends_in_its_block`). A custody-taking position
  and every other position (a return, a store, a binding) refuse
  (`a_literal_never_escapes`).
* **Unlisted forms refuse** (`an_unlisted_form_refuses`,
  `leaf_admitted_exactly_when`).
* **String extents (M11)** are facts, not bounds: the terminator is stored
  exactly when the array has room past the literal's bytes
  (`the_terminator_fact`).

## What this is not

A model of the decisions and of the value semantics they rest on. Faithful Clang
extraction (semantic initializer lists, record members, types) is trust boundary
28, and that the Rust transfer computes these values is bound only at the
decisions: `copyVectors`, `leafVectors` and `literalVectors` are the Rust tables
row by row (`tests/unit/aggregate_value.rs`).
-/

namespace QCore.AggregateValue

/-! ## Classes and decisions (the Rust tables) -/

/-- The class of a type (`TypeClass`). -/
inductive Cls where
  | scalar
  | pointer
  | custodyFree
  | pointerBearing
  | unread
  deriving DecidableEq, Repr

/-- One side of a copy (`Side`): a typed place, an object of this frame, or
anything else. -/
inductive Side where
  | place
  | frame
  | other
  deriving DecidableEq, Repr

/-- The copy decision, `decide_copy`. -/
def decideCopy : Cls → Side → Side → String
  | .custodyFree, .other, _ => "copy-operand:destination"
  | .custodyFree, _, .other => "copy-operand:source"
  | .custodyFree, _, _ => "admit"
  | .pointerBearing, _, _ => "pointer-members"
  | _, _, _ => "unread-record"

/-- The class of one initializer leaf (`Leaf`). -/
inductive Leaf where
  | zero
  | scalar
  | stringArray
  | null
  | string
  | function
  | custodyFreeRecord
  | pointerBearingRecord
  | formalPointer
  | localPointer
  | pointerRead
  | address
  | call
  | compoundLiteral
  | other
  deriving DecidableEq, Repr

/-- The leaves that carry no identity, `Leaf::admitted`. -/
def Leaf.admitted : Leaf → Bool
  | .zero | .scalar | .stringArray | .null | .string | .function | .custodyFreeRecord => true
  | _ => false

/-- Where a compound literal is used (`LiteralUse`). -/
inductive LiteralUse where
  | borrowed
  | custodyTaking
  | elsewhere
  deriving DecidableEq, Repr

/-- The compound-literal decision, `decide_literal`. -/
def decideLiteral : Bool → Bool → LiteralUse → String
  | _, _, .elsewhere => "literal-position"
  | _, _, .custodyTaking => "literal-custody-position"
  | false, _, .borrowed => "leaf:other"
  | true, true, .borrowed => "literal-reads"
  | true, false, .borrowed => "admit"

def copyVectors : List (Cls × Side × Side × String) := [
  (.custodyFree, .place, .place, "admit"),
  (.custodyFree, .place, .place, "admit"),
  (.custodyFree, .frame, .place, "admit"),
  (.custodyFree, .frame, .place, "admit"),
  (.custodyFree, .place, .frame, "admit"),
  (.pointerBearing, .place, .place, "pointer-members"),
  (.pointerBearing, .frame, .place, "pointer-members"),
  (.unread, .place, .place, "unread-record"),
  (.custodyFree, .place, .other, "copy-operand:source"),
  (.custodyFree, .other, .place, "copy-operand:destination")
]

def leafVectors : List (Leaf × Bool) := [
  (.zero, true),
  (.scalar, true),
  (.stringArray, true),
  (.null, true),
  (.string, true),
  (.function, true),
  (.custodyFreeRecord, true),
  (.pointerBearingRecord, false),
  (.formalPointer, false),
  (.localPointer, false),
  (.pointerRead, false),
  (.address, false),
  (.call, false),
  (.compoundLiteral, false),
  (.other, false)
]

def literalVectors : List (Bool × Bool × LiteralUse × String) := [
  (true, false, .borrowed, "admit"),
  (true, true, .borrowed, "literal-reads"),
  (false, false, .borrowed, "leaf:other"),
  (true, false, .custodyTaking, "literal-custody-position"),
  (true, false, .elsewhere, "literal-position"),
  (false, true, .elsewhere, "literal-position")
]

theorem copy_vectors_hold :
    copyVectors.all (fun (c, d, s, r) => decideCopy c d s == r) = true := by
  decide

theorem leaf_vectors_hold :
    leafVectors.all (fun (l, a) => l.admitted == a) = true := by
  decide

theorem literal_vectors_hold :
    literalVectors.all (fun (f, r, u, d) => decideLiteral f r u == d) = true := by
  decide

/-- A copy is admitted exactly for a custody-free record between two typed
sides. -/
theorem copy_admitted_exactly_when (c : Cls) (d s : Side) :
    decideCopy c d s = "admit" ↔ (c = .custodyFree ∧ d ≠ .other ∧ s ≠ .other) := by
  cases c <;> cases d <;> cases s <;> decide

/-- A literal is admitted exactly at a borrowed formal, identity-free and
reading no memory. -/
theorem literal_admitted_exactly_when (f r : Bool) (u : LiteralUse) :
    decideLiteral f r u = "admit" ↔ (f = true ∧ r = false ∧ u = .borrowed) := by
  cases f <;> cases r <;> cases u <;> decide

/-- Every leaf class outside the listed seven refuses. -/
theorem leaf_admitted_exactly_when (l : Leaf) :
    l.admitted = true ↔
      l ∈ [.zero, .scalar, .stringArray, .null, .string, .function, .custodyFreeRecord] := by
  cases l <;> decide

/-! ## Values, copies and custody -/

/-- A member value: a scalar, or a pointer denoting a tracked identity or
none. -/
inductive Val where
  | scalar (n : Int)
  | ptr (id : Option Nat)
  deriving DecidableEq, Repr

def Val.identity : Val → Option Nat
  | .ptr i => i
  | .scalar _ => none

/-- A record value: its members, in order (nested records flattened, as
Clang's semantic initializer lists them). -/
abbrev Record := List Val

def identities (r : Record) : List Nat := r.filterMap Val.identity

/-- A store of record values by place, and the owner ledger: which place owns
each tracked identity. -/
structure Frame where
  store : String → Record
  ledger : List (Nat × String)

/-- The field-wise copy of place `src` into place `dst`: every member of the
destination takes the source member's value. The ledger is not touched. -/
def copyInto (φ : Frame) (dst src : String) : Frame :=
  { φ with store := fun p => if p = dst then φ.store src else φ.store p }

/-- A copy adds no owner: the ledger is the ledger before. -/
theorem copy_adds_no_owner (φ : Frame) (dst src : String) :
    (copyInto φ dst src).ledger = φ.ledger := rfl

/-- Every identity the destination holds after a copy, the source held: a
copied pointer is an alias of the source's value. -/
theorem a_copy_only_aliases (φ : Frame) (dst src : String) (i : Nat)
    (h : i ∈ identities ((copyInto φ dst src).store dst)) : i ∈ identities (φ.store src) := by
  simpa [copyInto] using h

/-- A record whose every member is a scalar. -/
def custodyFree (r : Record) : Prop := ∀ v ∈ r, ∃ n, v = .scalar n

theorem custody_free_has_no_identity (r : Record) (h : custodyFree r) : identities r = [] := by
  induction r with
  | nil => rfl
  | cons v rest ih =>
    obtain ⟨n, hn⟩ := h v (List.mem_cons_self ..)
    subst hn
    simp only [identities, List.filterMap_cons, Val.identity]
    exact ih (fun w hw => h w (List.mem_cons_of_mem _ hw))

/-- A custody-free copy moves no identity into its destination. -/
theorem a_custody_free_copy_moves_no_identity (φ : Frame) (dst src : String)
    (h : custodyFree (φ.store src)) : identities ((copyInto φ dst src).store dst) = [] := by
  simpa [copyInto] using custody_free_has_no_identity _ h

/-- Release one identity: it must be owned, and afterwards it is not. -/
def release (ledger : List (Nat × String)) (i : Nat) : Option (List (Nat × String)) :=
  if i ∈ ledger.map Prod.fst then some (ledger.filter (fun e => e.1 != i)) else none

/-- Releasing both copies of an owned pointer is a double release: the second
release finds no owner. This is the shape a pointer-bearing copy would admit,
so such a copy refuses unless a contract or the owned-field model says which
copy owns. -/
theorem releasing_both_copies_is_a_double_release (ledger : List (Nat × String)) (i : Nat)
    (l : List (Nat × String)) (h : release ledger i = some l) : release l i = none := by
  unfold release at h
  split at h
  · cases h
    unfold release
    simp
  · cases h

/-- The owned-field model agrees: a record declaring an owned field is not
copyable, and the class of such a record (it holds a pointer) is refused by
the copy decision in every position. -/
theorem the_owned_field_model_agrees (f : String) (r : Nat) (d s : Side) :
    QCore.AggregateOwnedField.copyable ⟨[⟨f, some r⟩]⟩ = false ∧
      decideCopy .pointerBearing d s ≠ "admit" := by
  refine ⟨QCore.AggregateOwnedField.an_aggregate_is_never_copied f r, ?_⟩
  cases d <;> cases s <;> decide

/-! ## Zero-fill and initializers -/

/-- The type of one flattened member or element. -/
inductive LeafTy where
  | scalar
  | pointer
  deriving DecidableEq, Repr

/-- The zero of a member's type: `0`, or the null pointer. -/
def zeroOf : LeafTy → Val
  | .scalar => .scalar 0
  | .pointer => .ptr none

/-- Zero-fill of a flattened type: every member and element its zero. -/
def zeroFill (t : List LeafTy) : Record := t.map zeroOf

/-- Zero-fill yields no identity: a zero pointer member is null. -/
theorem zero_fill_has_no_identity (t : List LeafTy) : identities (zeroFill t) = [] := by
  induction t with
  | nil => rfl
  | cons x rest ih =>
    cases x <;> simpa [zeroFill, identities, zeroOf, Val.identity] using ih

/-- The value one leaf of a class may denote, given the identity its runtime
value would carry: an admitted class carries none whatever the runtime holds
(a zero, a literal's static storage, a function, a scalar); every other class
may carry one. -/
def Leaf.denote : Leaf → Option Nat → Val
  | .zero, _ | .null, _ | .string, _ | .function, _ => .ptr none
  | .scalar, _ | .stringArray, _ | .custodyFreeRecord, _ => .scalar 0
  | _, i => .ptr i

theorem admitted_leaf_denotes_no_identity (l : Leaf) (i : Option Nat) (h : l.admitted = true) :
    (l.denote i).identity = none := by
  cases l <;> simp_all [Leaf.admitted, Leaf.denote, Val.identity]

/-- Every class the rule refuses may denote an identity: the refusal is
needed. -/
theorem an_unadmitted_leaf_may_carry_an_identity (l : Leaf) (h : l.admitted = false) :
    (l.denote (some 0)).identity = some 0 := by
  cases l <;> simp_all [Leaf.admitted, Leaf.denote, Val.identity]

/-- An initializer whose every leaf is admitted holds no identity. -/
theorem an_admitted_initializer_holds_no_identity (ls : List (Leaf × Option Nat))
    (h : ∀ p ∈ ls, p.1.admitted = true) :
    identities (ls.map fun p => p.1.denote p.2) = [] := by
  induction ls with
  | nil => rfl
  | cons p rest ih =>
    have hp := admitted_leaf_denotes_no_identity p.1 p.2 (h p (List.mem_cons_self ..))
    simp only [List.map_cons, identities, List.filterMap_cons, hp]
    exact ih (fun q hq => h q (List.mem_cons_of_mem _ hq))

/-! ## Compound-literal lifetime -/

/-- A literal's block: its storage lives from `entry` to `exit`. -/
structure Block where
  entry : Nat
  exit : Nat

/-- The interval a use of the literal at time `t` keeps its address: a borrow
lasts until the call returns (`t + 1`); any other use has no bound this rule
knows. -/
def borrowInterval : LiteralUse → Nat → Option (Nat × Nat)
  | .borrowed, t => some (t, t + 1)
  | _, _ => none

/-- The literal's lifetime bounds its borrows: an admitted use inside the
block ends inside it. -/
theorem a_literal_borrow_ends_in_its_block (b : Block) (t : Nat) (f r : Bool) (u : LiteralUse)
    (hin : b.entry ≤ t ∧ t < b.exit) (hadmit : decideLiteral f r u = "admit") :
    ∃ iv, borrowInterval u t = some iv ∧ b.entry ≤ iv.1 ∧ iv.2 ≤ b.exit := by
  have hu : u = .borrowed := ((literal_admitted_exactly_when f r u).mp hadmit).2.2
  subst hu
  exact ⟨(t, t + 1), rfl, hin.1, hin.2⟩

/-- A literal is never admitted where its address could outlive the block. -/
theorem a_literal_never_escapes (f r : Bool) :
    decideLiteral f r .elsewhere ≠ "admit" ∧ decideLiteral f r .custodyTaking ≠ "admit" := by
  cases f <;> cases r <;> decide

/-! ## Forms -/

/-- The statement forms the rule reads, and everything else. -/
inductive Form where
  | copy
  | initializer
  | compoundLiteral
  | stringInitializer
  | other (kind : String)
  deriving DecidableEq, Repr

def Form.listed : Form → Bool
  | .other _ => false
  | _ => true

/-- A form outside the four keeps its older refusal. -/
theorem an_unlisted_form_refuses (k : String) : (Form.other k).listed = false := rfl

/-! ## String extents (facts for CG16, no bound) -/

/-- The terminator is stored exactly when the array has room past the
literal's bytes (C11 6.7.9p14). -/
def terminated (arrayBytes literalBytes : Nat) : Bool := decide (literalBytes < arrayBytes)

theorem the_terminator_fact (a l : Nat) : terminated a l = true ↔ l < a := by
  simp [terminated]

end QCore.AggregateValue
