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
Bounded direct calls over extent-carrying byte resources. This models the
decisions implemented by `qcore_core::resource_calls`: affine owned arguments,
whole-resource read/write loans, call-site minimum extents, decreasing call
ranks, transitive alloc/mut effects, and the zero-live-owner premise on an
allocation failure exit. Frontend extraction, C emission, and Rust/Lean
correspondence remain review premises rather than extraction theorems.
-/
namespace QCore.ResourceCalls

inductive Mode where
  | owned | readLoan | writeLoan | scalar
  deriving DecidableEq, Repr

inductive Access where | read | write deriving DecidableEq, Repr

def permits : Mode → Access → Bool
  | .owned, _ => true
  | .readLoan, .read => true
  | .readLoan, .write => false
  | .writeLoan, _ => true
  | .scalar, _ => false

theorem read_loan_permits_read : permits .readLoan .read = true := by decide
theorem read_loan_refuses_write : permits .readLoan .write = false := by decide
theorem write_loan_permits_write : permits .writeLoan .write = true := by decide
theorem scalar_never_permits_resource_access (access : Access) :
    permits .scalar access = false := by cases access <;> decide

/-- One resource actual may be repeated only when every occurrence is a read loan. -/
def aliasesCompatible (modes : List Mode) : Bool :=
  modes.all (· == .readLoan) || modes.length ≤ 1

theorem duplicate_reads_are_compatible :
    aliasesCompatible [.readLoan, .readLoan] = true := by decide
theorem owned_and_read_alias_refuses :
    aliasesCompatible [.owned, .readLoan] = false := by decide
theorem write_and_read_alias_refuses :
    aliasesCompatible [.writeLoan, .readLoan] = false := by decide

theorem compatible_repeated_aliases_are_reads (modes : List Mode)
    (compatible : aliasesCompatible modes = true) (repeated : 1 < modes.length) :
    modes.all (· == .readLoan) = true := by
  simp only [aliasesCompatible, Bool.or_eq_true] at compatible
  rcases compatible with allReads | atMostOne
  · exact allReads
  · have : modes.length ≤ 1 := of_decide_eq_true atMostOne
    omega

inductive OwnedSlot where | live | moved | dropped deriving DecidableEq, Repr

def moveOwned : OwnedSlot → Option OwnedSlot
  | .live => some .moved
  | .moved | .dropped => none

def dropOwned : OwnedSlot → Option OwnedSlot
  | .live => some .dropped
  | .moved | .dropped => none

theorem one_move_consumes_the_slot : moveOwned .live = some .moved := by decide
theorem a_second_move_refuses : moveOwned .moved = none := by decide
theorem drop_after_move_refuses : dropOwned .moved = none := by decide

/-- Borrowed parameters can be accessed, but cannot manufacture owned returns. -/
def canReturnOwned : Mode → Bool
  | .owned => true
  | _ => false

theorem read_borrow_cannot_escape_as_owned : canReturnOwned .readLoan = false := by decide
theorem write_borrow_cannot_escape_as_owned : canReturnOwned .writeLoan = false := by decide

/-- A helper records the largest `index + 1` obligation for each parameter. -/
def extentAdmitted (minimum actual : Nat) : Bool := decide (minimum ≤ actual)

theorem admitted_extent_discharges_the_minimum (minimum actual : Nat)
    (h : extentAdmitted minimum actual = true) : minimum ≤ actual := by
  simpa [extentAdmitted] using h

theorem too_short_actual_refuses (minimum actual : Nat) (h : actual < minimum) :
    extentAdmitted minimum actual = false := by
  simp [extentAdmitted, Nat.not_le.mpr h]

def propagatedMinimum (callerMinimum calleeMinimum : Nat) : Nat :=
  max callerMinimum calleeMinimum

theorem propagated_extent_discharges_callee_access
    (callerMinimum calleeMinimum actual index : Nat)
    (admitted : extentAdmitted (propagatedMinimum callerMinimum calleeMinimum) actual = true)
    (access : index < calleeMinimum) : index < actual := by
  have propagated : propagatedMinimum callerMinimum calleeMinimum ≤ actual :=
    admitted_extent_discharges_the_minimum _ _ admitted
  have included : calleeMinimum ≤ propagatedMinimum callerMinimum calleeMinimum := by
    simpa [propagatedMinimum] using Nat.le_max_right callerMinimum calleeMinimum
  omega

/-- Ranks are derived from the graph; every edge must decrease. -/
def callsAdmitted (rank : Nat → Nat) (edges : List (Nat × Nat)) : Prop :=
  ∀ edge ∈ edges, rank edge.2 < rank edge.1

theorem self_call_refuses (rank : Nat → Nat) (edges : List (Nat × Nat)) (f : Nat)
    (edge : (f, f) ∈ edges) : ¬ callsAdmitted rank edges := by
  intro admitted
  have decreases : rank f < rank f := admitted (f, f) edge
  omega

theorem mutual_call_refuses (rank : Nat → Nat) (edges : List (Nat × Nat)) (a b : Nat)
    (forward : (a, b) ∈ edges) (backward : (b, a) ∈ edges) :
    ¬ callsAdmitted rank edges := by
  intro admitted
  have first : rank b < rank a := admitted (a, b) forward
  have second : rank a < rank b := admitted (b, a) backward
  omega

structure Effects where
  alloc : Bool
  mutation : Bool
  deriving DecidableEq, Repr

def Effects.union (left right : Effects) : Effects :=
  ⟨left.alloc || right.alloc, left.mutation || right.mutation⟩

def Effects.admits (declared required : Effects) : Bool :=
  (!required.alloc || declared.alloc) && (!required.mutation || declared.mutation)

theorem callee_mutation_is_transitive (caller callee : Effects)
    (h : callee.mutation = true) : (caller.union callee).mutation = true := by
  simp [Effects.union, h]

theorem explicit_empty_refuses_indirect_mutation (callee : Effects)
    (h : callee.mutation = true) :
    Effects.admits (Effects.mk false false)
      ((Effects.mk false false).union callee) = false := by
  simp [Effects.admits, Effects.union, h]

/-- With no payload error row in v1, allocation failure may exit only at zero custody. -/
def allocationFailureAdmitted (liveOwners : Nat) : Bool := decide (liveOwners = 0)

theorem zero_owner_failure_exit_is_clean : allocationFailureAdmitted 0 = true := by decide

theorem a_live_owner_refuses_allocation_failure (liveOwners : Nat) (h : 0 < liveOwners) :
    allocationFailureAdmitted liveOwners = false := by
  simp [allocationFailureAdmitted, Nat.ne_of_gt h]

end QCore.ResourceCalls
