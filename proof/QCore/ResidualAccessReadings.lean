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

/-!
# Four readings of residual indirect accesses (card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2, gateway rules 14a-14d)

The abstract rules implemented by
`crates/qcore-c-gateway/src/residual_access_reading.rs`,
`crates/qcore-c-gateway/src/legacy_residual_access.rs` and
`crates/qcore-c-gateway/src/scalar_element_address.rs`.

* **14a.** A scalar statement whose value holds exactly one call is lowered as
  the call, then the statement's own typed reads, only when the callee's
  contract has a `None` result and every position `Borrowed`. C leaves the
  order of the call and the other operands unspecified; the model proves that
  such a call changes no custody state, so a read's decision is the same on
  either side of it, and that a releasing position would not have that
  property (the counterexample is why the premise is required).
* **14b.** A pointer local given an interior pointer of a path whose every
  root is never bound binds exactly the target-free, possibly-null borrowed
  alias a pointer-valued path read binds, so a release through it discharges
  no tracked identity.
* **14c.** An address handed to a callee is read for a member (rule 10n(1))
  or for an element whose type is a scalar, and for nothing else.
* **14d.** An interior offset `P ± n` is read with a path operand (rule 14b's
  statement and an actual) or, at an actual only, a plain pointer variable the
  closure proves never bound.

## What this is not

A model of the classifier or of Clang. That the Rust's shape tests are these
decisions is bound by shared vectors (`DECISION_VECTORS`, `INTERIOR_VECTORS`,
`SCALAR_CALL_VECTORS`, `OFFSET_VECTORS`); faithful extraction is trust boundary 28.
-/

namespace QCore.ResidualAccessReadings

/-! ## 14a: a call that takes no custody commutes with every read -/

/-- A contract position, as far as custody is concerned. -/
inductive Mode where
  | borrowed
  | released
  | consumed
  | owned
  deriving DecidableEq, Repr

/-- An identity's liveness in the custody state. -/
inductive Life where
  | live
  | released
  deriving DecidableEq, Repr

/-- The custody state: each actual's identity (by position) and its life. -/
abbrev State := List Life

/-- The effect of a call on the identities its actuals name, position by
position: a releasing or consuming position ends that identity's life, a
`Borrowed` one leaves it. -/
def effect : List Mode → State → State
  | [], s => s
  | _ :: _, [] => []
  | m :: ms, l :: ls =>
      (match m with
        | .released | .consumed => .released
        | _ => l) :: effect ms ls

/-- A read through the identity at position `i`: it resolves only on a live
identity (a released one is a `use_after_release` candidate). -/
def readResolves (i : Nat) (s : State) : Bool := s.getD i .released == .live

/-- Every position is `Borrowed`. -/
def allBorrowed (ms : List Mode) : Bool := ms.all (· == .borrowed)

theorem a_borrowed_call_changes_no_custody (ms : List Mode) (s : State)
    (h : allBorrowed ms = true) : effect ms s = s := by
  induction ms generalizing s with
  | nil => cases s <;> rfl
  | cons m ms ih =>
      simp only [allBorrowed, List.all_cons, Bool.and_eq_true, beq_iff_eq] at h
      obtain ⟨hm, hms⟩ := h
      cases s with
      | nil => rfl
      | cons l ls =>
          subst hm
          simp only [effect]
          rw [ih ls hms]

theorem a_read_is_decided_alike_on_either_side_of_the_call (ms : List Mode) (s : State)
    (i : Nat) (h : allBorrowed ms = true) :
    readResolves i (effect ms s) = readResolves i s := by
  rw [a_borrowed_call_changes_no_custody ms s h]

/-- Why the premise is required: a releasing position decides the same read
differently after the call. -/
theorem a_releasing_call_does_not_commute :
    readResolves 0 (effect [.released] [.live]) ≠ readResolves 0 [.live] := by decide

/-- Rule 14a's decision: exactly one call, a `None` result, every position
`Borrowed` (`scalar_call_admitted`). -/
def scalarCallAdmitted (calls : Nat) (noneResult allBorrowedPositions : Bool) : Bool :=
  calls == 1 && noneResult && allBorrowedPositions

/-- The Rust's `SCALAR_CALL_VECTORS`. -/
def scalarCallVectors : List (Nat × Bool × Bool × Bool) :=
  [(1, true, true, true), (0, true, true, false), (2, true, true, false),
   (1, false, true, false), (1, true, false, false)]

theorem scalar_call_vectors_hold :
    scalarCallVectors.all (fun (c, n, b, a) => scalarCallAdmitted c n b == a) = true := by
  decide

theorem two_calls_are_never_read (n b : Bool) : scalarCallAdmitted 2 n b = false := by
  simp [scalarCallAdmitted]

theorem an_admitted_call_takes_no_custody (calls : Nat) (n : Bool) (ms : List Mode)
    (h : scalarCallAdmitted calls n (allBorrowed ms) = true) (s : State) :
    effect ms s = s := by
  simp only [scalarCallAdmitted, Bool.and_eq_true] at h
  exact a_borrowed_call_changes_no_custody ms s h.2

/-! ## 14b: the interior alias is the pointer read's alias -/

/-- A pointer binding: its tracked targets, whether it is unknown and whether
it may be null (`AbstractPointer`). -/
structure Binding where
  targets : List Nat
  unknown : Bool
  mayBeNull : Bool
  deriving DecidableEq, Repr

/-- What a pointer-valued path read binds its destination to
(`borrowed_destination`): target-free, unknown, possibly null. -/
def borrowedAlias : Binding := ⟨[], true, true⟩

/-- Rule 14b's value forms and the statement shapes it reads. -/
inductive Shape where
  | pointerLocal
  | memberPlace
  | scalarLocal
  deriving DecidableEq, Repr

inductive Form where
  | address
  | offset
  | call
  | cast
  deriving DecidableEq, Repr

/-- Rule 14b's decision (`interior_admitted`). -/
def interiorAdmitted : Shape → Form → Bool → Bool
  | .pointerLocal, .address, n => n
  | .pointerLocal, .offset, n => n
  | _, _, _ => false

/-- What rule 14b binds when it admits (else the gate's refusal stands). -/
def interiorBinding (shape : Shape) (form : Form) (neverBound : Bool) : Option Binding :=
  if interiorAdmitted shape form neverBound then some borrowedAlias else none

/-- The Rust's `INTERIOR_VECTORS`. -/
def interiorVectors : List (Shape × Form × Bool × Bool) :=
  [(.pointerLocal, .address, true, true), (.pointerLocal, .offset, true, true),
   (.pointerLocal, .address, false, false), (.pointerLocal, .offset, false, false),
   (.pointerLocal, .call, true, false), (.pointerLocal, .cast, true, false),
   (.memberPlace, .address, true, false), (.scalarLocal, .address, true, false)]

theorem interior_vectors_hold :
    interiorVectors.all (fun (s, f, n, a) => interiorAdmitted s f n == a) = true := by decide

theorem a_bound_root_is_never_read (s : Shape) (f : Form) : interiorAdmitted s f false = false := by
  cases s <;> cases f <;> rfl

theorem the_interior_alias_is_the_pointer_reads (s : Shape) (f : Form) (n : Bool) (b : Binding)
    (h : interiorBinding s f n = some b) : b = borrowedAlias := by
  unfold interiorBinding at h
  split at h
  · cases h; rfl
  · cases h

/-- A release through a binding discharges exactly its tracked targets. -/
def discharged (b : Binding) : List Nat := b.targets

theorem a_release_through_the_alias_discharges_nothing (s : Shape) (f : Form) (n : Bool)
    (b : Binding) (h : interiorBinding s f n = some b) : discharged b = [] := by
  rw [the_interior_alias_is_the_pointer_reads s f n b h]; rfl

theorem the_alias_is_never_known_non_null (s : Shape) (f : Form) (n : Bool) (b : Binding)
    (h : interiorBinding s f n = some b) : b.mayBeNull = true ∧ b.unknown = true := by
  rw [the_interior_alias_is_the_pointer_reads s f n b h]; exact ⟨rfl, rfl⟩

/-! ## 14c: a scalar element's address at a callee's actual -/

/-- The operand of the `&`, as far as the shape test reads it. -/
inductive Place where
  | member
  | element
  | paren
  | name
  | deref
  deriving DecidableEq, Repr

/-- Rule 10n(1) with rule 14c (`shape_admitted`). -/
def addressShapeAdmitted : Place → Bool → Bool
  | .member, _ => true
  | .element, scalar => scalar
  | _, _ => false

/-- The Rust's `DECISION_VECTORS`. -/
def addressVectors : List (Place × Bool × Bool) :=
  [(.member, true, true), (.member, false, true), (.element, true, true),
   (.element, false, false), (.paren, true, false), (.name, true, false),
   (.deref, true, false)]

theorem address_vectors_hold :
    addressVectors.all (fun (p, s, a) => addressShapeAdmitted p s == a) = true := by decide

theorem a_member_keeps_rule_10n (s : Bool) : addressShapeAdmitted .member s = true := rfl

theorem an_element_is_read_only_when_scalar (s : Bool)
    (h : addressShapeAdmitted .element s = true) : s = true := h

theorem nothing_else_is_read (p : Place) (s : Bool) (hm : p ≠ .member) (he : p ≠ .element) :
    addressShapeAdmitted p s = false := by
  cases p with
  | member => exact absurd rfl hm
  | element => exact absurd rfl he
  | paren => rfl
  | name => rfl
  | deref => rfl

/-! ## 14d: an interior offset at a callee's actual -/

/-- The pointer operand of `P ± n` (`OffsetOperand`). -/
inductive Operand where
  | path
  | name (neverBound : Bool)
  | other
  deriving DecidableEq, Repr

/-- Rules 14b and 14d (`offset_operand_admitted`): a path everywhere; a plain
pointer variable only at an actual and only never bound. -/
def offsetOperandAdmitted : Operand → Bool → Bool
  | .path, _ => true
  | .name n, actual => actual && n
  | .other, _ => false

/-- The Rust's `OFFSET_VECTORS`. -/
def offsetVectors : List (Operand × Bool × Bool) :=
  [(.path, false, true), (.path, true, true), (.name true, true, true),
   (.name true, false, false), (.name false, true, false), (.other, true, false),
   (.other, false, false)]

theorem offset_vectors_hold :
    offsetVectors.all (fun (o, a, r) => offsetOperandAdmitted o a == r) = true := by decide

theorem a_bound_name_is_never_read (actual : Bool) :
    offsetOperandAdmitted (.name false) actual = false := by
  cases actual <;> rfl

theorem a_statement_reads_only_a_path_operand (o : Operand)
    (h : offsetOperandAdmitted o false = true) : o = .path := by
  cases o with
  | path => rfl
  | name n => simp [offsetOperandAdmitted] at h
  | other => cases h

end QCore.ResidualAccessReadings
