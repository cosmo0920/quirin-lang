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

namespace QCore.CallbackSource

/-! Symbol provenance for the bounded callback source-path projection.
The executable account erases C expressions to their symbol references and
producer type tags. It does not prove Clang/Rust import, C evaluation order,
integer arithmetic, memory bounds, external call contracts, or source/runtime
refinement. Both alternatives of each imported condition remain explicit. -/

structure Reference where
  operation : Nat
  typeTag : String
  deriving DecidableEq, Repr

structure Record where
  references : List Reference
  produces : Option String
  deriving DecidableEq, Repr

def referenceValid (prior : List (Option String)) (reference : Reference) : Bool :=
  decide (prior[reference.operation]? = some (some reference.typeTag))

def recordValid (prior : List (Option String)) (record : Record) : Bool :=
  record.references.all (referenceValid prior)

def appendRecord (prior : List (Option String)) (record : Record) :
    Option (List (Option String)) :=
  if recordValid prior record then some (prior ++ [record.produces]) else none

theorem accepted_record_has_typed_producers
    {prior next : List (Option String)} {record : Record}
    (accepted : appendRecord prior record = some next) :
    ∀ reference ∈ record.references,
      prior[reference.operation]? = some (some reference.typeTag) := by
  unfold appendRecord at accepted
  split at accepted
  · rename_i valid
    intro reference member
    have present := List.all_eq_true.mp valid reference member
    simpa [referenceValid] using present
  · contradiction

theorem accepted_record_rejects_forward_references
    {prior next : List (Option String)} {record : Record}
    (accepted : appendRecord prior record = some next) :
    ∀ reference ∈ record.references, reference.operation < prior.length := by
  intro reference member
  have producer := accepted_record_has_typed_producers accepted reference member
  obtain ⟨bounded, _⟩ := List.getElem?_eq_some_iff.mp producer
  exact bounded

theorem accepted_record_appends_exactly_its_producer
    {prior next : List (Option String)} {record : Record}
    (accepted : appendRecord prior record = some next) :
    next = prior ++ [record.produces] := by
  unfold appendRecord at accepted
  split at accepted
  · cases accepted
    rfl
  · contradiction

theorem wrong_type_reference_is_rejected
    (prior : List (Option String)) (reference : Reference)
    (mismatch : prior[reference.operation]? ≠ some (some reference.typeTag)) :
    referenceValid prior reference = false := by
  simp [referenceValid, mismatch]

-- A return/write/guard record does not produce a read or call result.
theorem nonproducing_record_cannot_supply_symbol (prior : List (Option String))
    (typeTag : String) :
    referenceValid (prior ++ [none]) { operation := prior.length, typeTag } = false := by
  simp [referenceValid]

def branchPaths (prior : List Bool) : List (List Bool) :=
  [prior ++ [true], prior ++ [false]]

theorem branch_preserves_taken_alternative (prior : List Bool) :
    prior ++ [true] ∈ branchPaths prior := by
  simp [branchPaths]

theorem branch_preserves_not_taken_alternative (prior : List Bool) :
    prior ++ [false] ∈ branchPaths prior := by
  simp [branchPaths]

/-! Abstract lexical storage boundary for array/record path projections.
The C layout and initialization semantics remain obligations of consumers. -/
def startStorage (live : List Nat) (slot : Nat) : Option (List Nat) :=
  if slot ∈ live then none else some (slot :: live)

def endStorage (live : List Nat) (slot : Nat) : List Nat :=
  live.filter (fun candidate => candidate != slot)

theorem accepted_storage_start_is_fresh
    {live next : List Nat} {slot : Nat}
    (accepted : startStorage live slot = some next) : slot ∉ live := by
  unfold startStorage at accepted
  split at accepted
  · contradiction
  · assumption

theorem accepted_storage_start_retains_existing
    {live next : List Nat} {slot : Nat}
    (accepted : startStorage live slot = some next) :
    ∀ prior ∈ live, prior ∈ next := by
  unfold startStorage at accepted
  split at accepted
  · contradiction
  · cases accepted
    intro prior member
    exact List.mem_cons_of_mem slot member

theorem storage_end_removes_identity (live : List Nat) (slot : Nat) :
    slot ∉ endStorage live slot := by
  simp [endStorage]

theorem storage_end_preserves_other_identity
    (live : List Nat) (slot other : Nat) (different : other ≠ slot)
    (present : other ∈ live) : other ∈ endStorage live slot := by
  simp [endStorage, different, present]

/-! Terminal control is retained separately from normal return. A literal-zero
macro executes its body once, and a nonreturning prefix never gains a suffix. -/
inductive ExitKind where
  | running
  | returned
  | terminated
  deriving DecidableEq, Repr

structure ControlPath where
  records : List Nat
  exitKind : ExitKind
  deriving DecidableEq, Repr

def appendRunning (path : ControlPath) (suffix : List Nat) : ControlPath :=
  match path.exitKind with
  | .running => { path with records := path.records ++ suffix }
  | _ => path

def finishMacro (path : ControlPath) (falseGuard : Nat) : ControlPath :=
  appendRunning path [falseGuard]

theorem terminated_path_has_no_following_effects
    (records suffix : List Nat) :
    appendRunning ⟨records, .terminated⟩ suffix = ⟨records, .terminated⟩ := rfl

theorem returned_path_has_no_following_effects
    (records suffix : List Nat) :
    appendRunning ⟨records, .returned⟩ suffix = ⟨records, .returned⟩ := rfl

theorem running_macro_retains_body_and_false_guard
    (records : List Nat) (falseGuard : Nat) :
    finishMacro ⟨records, .running⟩ falseGuard =
      ⟨records ++ [falseGuard], .running⟩ := rfl

theorem terminal_macro_does_not_test_condition
    (records : List Nat) (falseGuard : Nat) :
    finishMacro ⟨records, .terminated⟩ falseGuard = ⟨records, .terminated⟩ := rfl

/-! Finite observation account for structured read-only scan loops. Exhausted
fuel is Unknown, never evidence of a false loop header or safe termination.
Source integer arithmetic and memory bounds remain separate obligations. -/
inductive ScanOutcome where
  | returned (value : Int)
  | exited
  | unknown
  deriving DecidableEq, Repr

def scanPrefix (header hit : Nat → Bool) (result : Nat → Int) :
    Nat → Nat → ScanOutcome
  | 0, _ => .unknown
  | fuel + 1, index =>
      if header index then
        if hit index then .returned (result index)
        else scanPrefix header hit result fuel (index + 1)
      else .exited

theorem scan_without_fuel_stays_unknown
    (header hit : Nat → Bool) (result : Nat → Int) (index : Nat) :
    scanPrefix header hit result 0 index = .unknown := rfl

theorem false_scan_header_exits
    (header hit : Nat → Bool) (result : Nat → Int) (fuel index : Nat)
    (finished : header index = false) :
    scanPrefix header hit result (fuel + 1) index = .exited := by
  simp [scanPrefix, finished]

theorem scan_match_returns_before_next_iteration
    (header hit : Nat → Bool) (result : Nat → Int) (fuel index : Nat)
    (active : header index = true) (found : hit index = true) :
    scanPrefix header hit result (fuel + 1) index = .returned (result index) := by
  simp [scanPrefix, active, found]

theorem unmatched_scan_retains_next_iteration
    (header hit : Nat → Bool) (result : Nat → Int) (fuel index : Nat)
    (active : header index = true) (missing : hit index = false) :
    scanPrefix header hit result (fuel + 1) index =
      scanPrefix header hit result fuel (index + 1) := by
  simp [scanPrefix, active, missing]

end QCore.CallbackSource
