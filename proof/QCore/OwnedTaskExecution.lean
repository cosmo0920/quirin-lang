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

import QCore.ScopedTasks
import QCore.TaskExecution

/-! Owned-buffer extension of the finite leaf-task model. Resource identities
are distinct from i32 values and from source slot identities. The definitions
compose affine slot operations with checked custody transfers. The complete
resource inventory and checked worker-exit relation must correspond to the Rust
checker. Source lowering, actual heap pointers, C11 thread publication/join and
native allocation failure are not proved by this model. No loan or owned-error
payload is admitted here. -/
namespace QCore.OwnedTaskExecution
open ScopedTasks

inductive ValueType where
  | unit | i32 | ownedBuffer
  deriving Repr, DecidableEq
inductive Outcome where
  | unit
  | i32 (bits : Fin 4294967296)
  | ownedBuffer (resource : Nat)
  | error (tag : String)
  deriving Repr, DecidableEq

def outcomeTyped (expected : ValueType) (errors : List String) : Outcome → Prop
  | .unit => expected = .unit
  | .i32 _ => expected = .i32
  | .ownedBuffer _ => expected = .ownedBuffer
  | .error tag => tag ∈ errors

def payload : Outcome → List Nat
  | .ownedBuffer resource => [resource]
  | _ => []

inductive Slot where
  | vacant
  | scalar (value : TaskExecution.Scalar)
  | ownedBuffer (resource : Nat)
  | result (outcome : Outcome)
  | consumed
  deriving Repr, DecidableEq
abbrev Slots := Nat → Slot

def Slot.resource : Slot → Option Nat
  | .ownedBuffer resource => some resource
  | .result (.ownedBuffer resource) => some resource
  | _ => none

def putSlot (slots : Slots) (slot : Nat) (value : Slot) : Slots :=
  fun other => if other = slot then value else slots other

def slotsCorrespond (slots : Slots) (custody : Custody) (task : Nat) : Prop :=
  ∀ slot resource, (slots slot).resource = some resource → custody resource = some (.task task)

def affineSlots (slots : Slots) : Prop :=
  ∀ first second resource, (slots first).resource = some resource →
    (slots second).resource = some resource → first = second

def readBuffer (slots : Slots) (custody : Custody) (task slot : Nat) : Option Nat :=
  match slots slot with
  | .ownedBuffer resource =>
      if custody resource = some (.task task) then some resource else none
  | _ => none

theorem checked_read_requires_exact_slot_and_task_custody
    {slots : Slots} {custody : Custody} {task slot resource : Nat}
    (checked : readBuffer slots custody task slot = some resource) :
    slots slot = .ownedBuffer resource ∧ custody resource = some (.task task) := by
  unfold readBuffer at checked
  split at checked
  next ownedResource slotValue =>
    split at checked
    next held =>
      cases checked
      exact ⟨slotValue, held⟩
    next => contradiction
  next => contradiction

theorem owned_result_has_custody_but_is_not_yet_a_buffer_binding
    (slots : Slots) (custody : Custody) (task slot resource : Nat)
    (value : slots slot = .result (.ownedBuffer resource)) :
    (slots slot).resource = some resource ∧ readBuffer slots custody task slot = none := by
  simp [value, Slot.resource, readBuffer]

def extractOwnedResult (slots : Slots) (source destination : Nat) : Option Slots :=
  match slots source, slots destination with
  | .result (.ownedBuffer resource), .vacant =>
      some (putSlot (putSlot slots source .consumed) destination (.ownedBuffer resource))
  | _, _ => none

theorem checked_owned_extraction_preserves_exact_identity
    {before after : Slots} {source destination : Nat}
    (checked : extractOwnedResult before source destination = some after) :
    ∃ resource, before source = .result (.ownedBuffer resource) ∧
      before destination = .vacant ∧ source ≠ destination ∧
      after source = .consumed ∧ after destination = .ownedBuffer resource := by
  unfold extractOwnedResult at checked
  split at checked
  next resource sourceValue destinationValue =>
    have different : source ≠ destination := by
      intro same
      simp [same, destinationValue] at sourceValue
    cases checked
    exact ⟨resource, sourceValue, destinationValue, different,
      by simp [putSlot, different], by simp [putSlot]⟩
  next => contradiction

theorem owned_result_cannot_be_extracted_twice
    {before after : Slots} {source destination : Nat}
    (checked : extractOwnedResult before source destination = some after)
    (anotherDestination : Nat) :
    extractOwnedResult after source anotherDestination = none := by
  obtain ⟨_, _, _, _, consumed, _⟩ := checked_owned_extraction_preserves_exact_identity checked
  simp [extractOwnedResult, consumed]

theorem owned_extraction_preserves_slot_custody_correspondence
    {before after : Slots} {source destination task : Nat} {custody : Custody}
    (checked : extractOwnedResult before source destination = some after)
    (correspond : slotsCorrespond before custody task) : slotsCorrespond after custody task := by
  unfold extractOwnedResult at checked
  split at checked
  next resource sourceValue destinationValue =>
    have distinct : source ≠ destination := by
      intro same
      have incompatible := sourceValue
      rw [same, destinationValue] at incompatible
      contradiction
    cases checked
    intro slot identity held
    by_cases atDestination : slot = destination
    · subst slot
      simp [putSlot, Slot.resource] at held
      subst identity
      exact correspond source resource (by simp [sourceValue, Slot.resource])
    · by_cases atSource : slot = source
      · simp [putSlot, atSource, distinct, Slot.resource] at held
      · exact correspond slot identity (by simpa [putSlot, atDestination, atSource] using held)
  next => contradiction

theorem owned_extraction_preserves_affine_slots
    {before after : Slots} {source destination : Nat}
    (checked : extractOwnedResult before source destination = some after)
    (unique : affineSlots before) : affineSlots after := by
  unfold extractOwnedResult at checked
  split at checked
  next resource sourceValue destinationValue =>
    have distinct : source ≠ destination := by
      intro same
      have incompatible := sourceValue
      rw [same, destinationValue] at incompatible
      contradiction
    cases checked
    have sourceResource : (before source).resource = some resource := by
      simp [sourceValue, Slot.resource]
    intro first second identity firstHeld secondHeld
    by_cases firstDest : first = destination
    · subst first
      simp [putSlot, Slot.resource] at firstHeld
      subst identity
      by_cases secondDest : second = destination
      · exact secondDest.symm
      · by_cases secondSource : second = source
        · simp [putSlot, secondSource, distinct, Slot.resource] at secondHeld
        · have old : (before second).resource = some resource := by
            simpa [putSlot, secondDest, secondSource] using secondHeld
          exact False.elim (secondSource (unique second source resource old sourceResource))
    · by_cases firstSource : first = source
      · simp [putSlot, firstSource, distinct, Slot.resource] at firstHeld
      · have oldFirst : (before first).resource = some identity := by
          simpa [putSlot, firstDest, firstSource] using firstHeld
        by_cases secondDest : second = destination
        · subst second
          simp [putSlot, Slot.resource] at secondHeld
          subst identity
          exact False.elim (firstSource (unique first source resource oldFirst sourceResource))
        · by_cases secondSource : second = source
          · simp [putSlot, secondSource, distinct, Slot.resource] at secondHeld
          · apply unique first second identity oldFirst
            simpa [putSlot, secondDest, secondSource] using secondHeld
  next => contradiction

structure MoveCapture where
  slot : Nat
  resource : Nat
  deriving Repr, DecidableEq

def captureResources (captures : List MoveCapture) : List Nat := captures.map MoveCapture.resource

def consumeMoves (slots : Slots) (captures : List MoveCapture) : Slots :=
  fun slot => if slot ∈ captures.map MoveCapture.slot then .consumed else slots slot

def prepareMoves (slots : Slots) (custody : Custody) (parent reservation : Nat)
    (captures : List MoveCapture) : Option Custody :=
  if (captures.map MoveCapture.slot).Nodup ∧
      ∀ capture ∈ captures, slots capture.slot = .ownedBuffer capture.resource then
    transfer custody (captureResources captures) (.task parent) (.reservation reservation)
  else none

def commitMoves (slots : Slots) (reserved : Custody) (reservation child : Nat)
    (captures : List MoveCapture) : Option (Slots × Custody) := do
  let custody ← transfer reserved (captureResources captures) (.reservation reservation) (.task child)
  pure (consumeMoves slots captures, custody)

def rollbackMoves (slots : Slots) (reserved : Custody) (reservation parent : Nat)
    (captures : List MoveCapture) : Option (Slots × Custody) := do
  let custody ← transfer reserved (captureResources captures) (.reservation reservation) (.task parent)
  pure (slots, custody)

theorem prepared_moves_had_typed_parent_bindings
    {slots : Slots} {before reserved : Custody} {parent reservation : Nat}
    {captures : List MoveCapture}
    (checked : prepareMoves slots before parent reservation captures = some reserved) :
    (captures.map MoveCapture.slot).Nodup ∧
      ∀ capture ∈ captures, slots capture.slot = .ownedBuffer capture.resource := by
  unfold prepareMoves at checked
  split at checked
  · assumption
  · contradiction

theorem prepared_moves_have_unique_resources_and_parent_custody
    {slots : Slots} {before reserved : Custody} {parent reservation : Nat}
    {captures : List MoveCapture}
    (checked : prepareMoves slots before parent reservation captures = some reserved) :
    (captureResources captures).Nodup ∧
      ∀ resource ∈ captureResources captures, before resource = some (.task parent) := by
  unfold prepareMoves at checked
  split at checked
  · exact checked_transfer_had_original_custody checked
  · contradiction

theorem failed_creation_restores_exact_parent_slots_and_custody
    {slots : Slots} {before reserved : Custody} {parent reservation : Nat}
    {captures : List MoveCapture}
    (checked : prepareMoves slots before parent reservation captures = some reserved) :
    rollbackMoves slots reserved reservation parent captures = some (slots, before) := by
  unfold prepareMoves at checked
  split at checked
  · simp [rollbackMoves, rollback_restores_original_custody checked]
  · contradiction

theorem reservation_blocks_parent_buffer_access
    {slots : Slots} {before reserved : Custody} {parent reservation : Nat}
    {captures : List MoveCapture}
    (checked : prepareMoves slots before parent reservation captures = some reserved)
    (capture : MoveCapture) (member : capture ∈ captures) :
    readBuffer slots reserved parent capture.slot = none := by
  have binding := (prepared_moves_had_typed_parent_bindings checked).2 capture member
  unfold prepareMoves at checked
  split at checked
  · have held := checked_transfer_places_exact_resources checked capture.resource
    have selected : capture.resource ∈ captureResources captures := List.mem_map.mpr ⟨capture, member, rfl⟩
    simp only [ite_eq_left selected] at held
    simp [readBuffer, binding, held]
  · contradiction

theorem successful_move_consumes_every_selected_parent_slot
    {slots after : Slots} {reserved custody : Custody} {reservation child : Nat}
    {captures : List MoveCapture}
    (checked : commitMoves slots reserved reservation child captures = some (after, custody))
    (capture : MoveCapture) (member : capture ∈ captures) : after capture.slot = .consumed := by
  unfold commitMoves at checked
  cases transition : transfer reserved (captureResources captures) (.reservation reservation) (.task child) with
  | none => simp [transition] at checked
  | some next =>
    simp [transition] at checked
    obtain ⟨sameSlots, _⟩ := checked
    rw [← sameSlots]
    have selected : capture.slot ∈ captures.map MoveCapture.slot := List.mem_map.mpr ⟨capture, member, rfl⟩
    simp [consumeMoves, selected]

theorem moved_parent_binding_stays_unavailable_even_after_custody_returns
    (slots : Slots) (custody : Custody) (parent slot : Nat)
    (consumed : slots slot = .consumed) : readBuffer slots custody parent slot = none := by
  simp [readBuffer, consumed]

theorem move_commit_preserves_unselected_parent_slots
    (slots : Slots) (captures : List MoveCapture) (other : Nat)
    (outside : other ∉ captures.map MoveCapture.slot) :
    consumeMoves slots captures other = slots other := by
  simp [consumeMoves, outside]

theorem rollback_preserves_unselected_sibling_custody
    {slots afterSlots : Slots} {before after : Custody} {reservation parent : Nat}
    {captures : List MoveCapture}
    (checked : rollbackMoves slots before reservation parent captures = some (afterSlots, after))
    (resource : Nat) (outside : resource ∉ captureResources captures) : after resource = before resource := by
  unfold rollbackMoves at checked
  cases transition : transfer before (captureResources captures) (.reservation reservation) (.task parent) with
  | none => simp [transition] at checked
  | some next =>
    simp [transition] at checked
    obtain ⟨_, same⟩ := checked
    subst after
    exact transfer_preserves_unselected_resource transition resource outside

structure AllocationState where
  slots : Slots
  custody : Custody
  history : List Nat

/-- The false choice includes allocator failure: no generation is consumed. -/
def allocateBuffer (before : AllocationState) (parent slot identity : Nat)
    (succeeds : Bool) : Option (AllocationState × Option Nat) :=
  if before.slots slot = .vacant ∧ identity ∉ before.history ∧ before.custody identity = none then
    if succeeds then
      some ({
        slots := putSlot before.slots slot (.ownedBuffer identity)
        custody := (fun resource => if resource = identity then some (.task parent) else before.custody resource)
        history := before.history ++ [identity] }, some identity)
    else some (before, none)
  else none

theorem failed_allocation_preserves_complete_state
    {before after : AllocationState} {parent slot identity : Nat} {result : Option Nat}
    (checked : allocateBuffer before parent slot identity false = some (after, result)) :
    after = before ∧ result = none := by
  unfold allocateBuffer at checked
  split at checked
  · simpa using (Option.some.inj checked).symm
  · contradiction

theorem successful_allocation_requires_a_fresh_identity
    {before after : AllocationState} {parent slot identity : Nat}
    (checked : allocateBuffer before parent slot identity true = some (after, some identity)) :
    identity ∉ before.history ∧ before.custody identity = none ∧
      after.history = before.history ++ [identity] ∧
      after.slots slot = .ownedBuffer identity ∧ after.custody identity = some (.task parent) := by
  unfold allocateBuffer at checked
  split at checked
  next fresh =>
    cases checked
    exact ⟨fresh.2.1, fresh.2.2, rfl, by simp [putSlot], by simp⟩
  next => contradiction

def dropBuffer (before : AllocationState) (task slot : Nat) : Option AllocationState := do
  let resource ← readBuffer before.slots before.custody task slot
  pure {
    slots := putSlot before.slots slot .consumed
    custody := (fun other => if other = resource then none else before.custody other)
    history := before.history }

theorem buffer_drop_retains_generation_history
    {before after : AllocationState} {task slot : Nat}
    (checked : dropBuffer before task slot = some after) : after.history = before.history := by
  unfold dropBuffer at checked
  cases read : readBuffer before.slots before.custody task slot with
  | none => simp [read] at checked
  | some resource =>
    simp [read] at checked
    subst after
    rfl

theorem dropped_identity_cannot_be_reallocated
    {before after : AllocationState} {task slot identity : Nat}
    (checked : dropBuffer before task slot = some after)
    (used : identity ∈ before.history) (parent destination : Nat) (choice : Bool) :
    allocateBuffer after parent destination identity choice = none := by
  have retained := buffer_drop_retains_generation_history checked
  simp [allocateBuffer, retained, used]

/-- A checked exit relation is derived from the worker body, not guessed from
its return type. Inputs and drops are concrete identities after capture binding;
all inputs are accounted for by exactly a drop or the typed normal payload. -/
def workerExitChecked (inputs dropped : List Nat) (outcome : Outcome) : Prop :=
  inputs.Nodup ∧ dropped.Nodup ∧
    (∀ resource ∈ dropped, resource ∈ inputs) ∧
    (∀ resource ∈ payload outcome, resource ∈ inputs) ∧
    (∀ resource ∈ inputs, resource ∈ dropped ∨ resource ∈ payload outcome) ∧
    (∀ resource ∈ dropped, resource ∉ payload outcome)

theorem tag_only_worker_error_requires_every_owned_input_dropped
    {inputs dropped : List Nat} {tag : String}
    (checked : workerExitChecked inputs dropped (.error tag)) :
    ∀ resource ∈ inputs, resource ∈ dropped := by
  intro resource member
  simpa [payload] using checked.2.2.2.2.1 resource member

theorem owned_worker_return_is_an_original_undropped_input
    {inputs dropped : List Nat} {resource : Nat}
    (checked : workerExitChecked inputs dropped (.ownedBuffer resource)) :
    resource ∈ inputs ∧ resource ∉ dropped := by
  refine ⟨checked.2.2.2.1 resource (by simp [payload]), ?_⟩
  intro member
  exact checked.2.2.2.2.2 resource member (by simp [payload])

/-- `history` is the complete allocated-identity inventory. Its correspondence
to Rust's generation map is required; hidden resources are not assumed absent. -/
def remainingOwned (history : List Nat) (custody : Custody) (task : Nat) : List Nat :=
  history.filter (fun resource => custody resource == some (.task task))

def completeOwned (history : List Nat) (before : Custody) (child : Nat)
    (outcome : Outcome) : Option Custody :=
  if remainingOwned history before child = payload outcome then
    transfer before (payload outcome) (.task child) (.capsule child)
  else none

theorem completion_requires_exact_typed_resource_payload
    {history : List Nat} {before after : Custody} {child : Nat} {outcome : Outcome}
    (checked : completeOwned history before child outcome = some after) :
    remainingOwned history before child = payload outcome := by
  unfold completeOwned at checked
  split at checked
  · assumption
  · contradiction

theorem tag_only_error_cannot_hide_remaining_owned_resources
    {history : List Nat} {before after : Custody} {child : Nat} {tag : String}
    (checked : completeOwned history before child (.error tag) = some after) :
    remainingOwned history before child = [] :=
  completion_requires_exact_typed_resource_payload checked

theorem completed_owned_payload_is_inaccessible_to_every_task
    {history : List Nat} {before after : Custody} {child resource : Nat}
    (checked : completeOwned history before child (.ownedBuffer resource) = some after)
    (phases : Nat → Phase) (task : Nat) : canUse phases after task resource = false := by
  unfold completeOwned at checked
  split at checked
  · have held := checked_transfer_places_exact_resources checked resource
    simp [payload] at held
    exact unjoined_capsule_is_unavailable_to_every_task phases after task resource child held
  · contradiction

inductive Handle where
  | absent
  | running (creator child : Nat)
  | completed (creator child : Nat) (outcome : Outcome)
  | consumed
  deriving Repr, DecidableEq

/-- Join returns a Result binding; owned extraction is a later affine step.
A failed native join has no successful transition in this relation. -/
def joinOwned (slots : Slots) (custody : Custody) (creator destination : Nat)
    (handle : Handle) : Option (Slots × Custody × Handle) :=
  match handle with
  | .completed owner child outcome =>
      if creator = owner ∧ slots destination = .vacant then do
        let next ← transfer custody (payload outcome) (.capsule child) (.task creator)
        pure (putSlot slots destination (.result outcome), next, .consumed)
      else none
  | _ => none

theorem running_owned_task_cannot_be_collected
    (slots : Slots) (custody : Custody) (creator destination owner child : Nat) :
    joinOwned slots custody creator destination (.running owner child) = none := rfl

theorem only_creator_can_collect_owned_payload
    (slots : Slots) (custody : Custody) (creator destination owner child : Nat)
    (outcome : Outcome) (different : creator ≠ owner) :
    joinOwned slots custody creator destination (.completed owner child outcome) = none := by
  simp [joinOwned, different]

theorem checked_owned_join_binds_exact_result_and_consumes_handle
    {slots afterSlots : Slots} {custody afterCustody : Custody}
    {creator destination owner child : Nat} {outcome : Outcome} {afterHandle : Handle}
    (checked : joinOwned slots custody creator destination (.completed owner child outcome) =
      some (afterSlots, afterCustody, afterHandle)) :
    creator = owner ∧ afterSlots destination = .result outcome ∧ afterHandle = .consumed := by
  simp only [joinOwned] at checked
  split at checked
  next allowed =>
    cases transition : transfer custody (payload outcome) (.capsule child) (.task creator) with
    | none => simp [transition] at checked
    | some next =>
      simp [transition] at checked
      obtain ⟨sameSlots, _, sameHandle⟩ := checked
      exact ⟨allowed.1, by rw [← sameSlots]; simp [putSlot], sameHandle.symm⟩
  next => contradiction

theorem owned_join_cannot_be_repeated
    (slots : Slots) (custody : Custody) (creator destination : Nat) :
    joinOwned slots custody creator destination .consumed = none := rfl

def exitAllowed (slots : Slots) (liveSlots : List Nat) (history : List Nat)
    (custody : Custody) (task : Nat) (outcome : Outcome) : Prop :=
  (∀ slot ∈ liveSlots, (slots slot).resource = none) ∧
    remainingOwned history custody task = payload outcome

theorem checked_error_exit_has_no_owned_slot_or_resource
    {slots : Slots} {liveSlots history : List Nat} {custody : Custody}
    {task : Nat} {tag : String}
    (checked : exitAllowed slots liveSlots history custody task (.error tag)) :
    (∀ slot ∈ liveSlots, (slots slot).resource = none) ∧
      remainingOwned history custody task = [] := checked

theorem checked_unit_exit_has_no_owned_slot_or_resource
    {slots : Slots} {liveSlots history : List Nat} {custody : Custody} {task : Nat}
    (checked : exitAllowed slots liveSlots history custody task .unit) :
    (∀ slot ∈ liveSlots, (slots slot).resource = none) ∧
      remainingOwned history custody task = [] := checked

end QCore.OwnedTaskExecution

/-! Worker-local allocation extends the capture-only exit relation with the
complete successful local-allocation inventory. Allocation source recognition,
all-path CFG checking and the link to C execution remain explicit premises. -/
namespace QCore.OwnedTaskExecution

def workerAllocationExitChecked (inputs allocated dropped : List Nat) (outcome : Outcome) : Prop :=
  workerExitChecked (inputs ++ allocated) dropped outcome

theorem allocated_worker_return_is_an_input_or_local
    {inputs allocated dropped : List Nat} {resource : Nat}
    (checked : workerAllocationExitChecked inputs allocated dropped (.ownedBuffer resource)) :
    (resource ∈ inputs ∨ resource ∈ allocated) ∧ resource ∉ dropped := by
  have original := owned_worker_return_is_an_original_undropped_input checked
  exact ⟨List.mem_append.mp original.1, original.2⟩

theorem worker_error_drops_all_inputs_and_locals
    {inputs allocated dropped : List Nat} {tag : String}
    (checked : workerAllocationExitChecked inputs allocated dropped (.error tag)) :
    (∀ resource ∈ inputs, resource ∈ dropped) ∧ (∀ resource ∈ allocated, resource ∈ dropped) := by
  have all := tag_only_worker_error_requires_every_owned_input_dropped checked
  exact ⟨fun resource member => all resource (List.mem_append.mpr (Or.inl member)),
    fun resource member => all resource (List.mem_append.mpr (Or.inr member))⟩

theorem worker_exit_accounts_for_every_local
    {inputs allocated dropped : List Nat} {outcome : Outcome}
    (checked : workerAllocationExitChecked inputs allocated dropped outcome) :
    ∀ resource ∈ allocated, resource ∈ dropped ∨ resource ∈ payload outcome := by
  intro resource member
  exact checked.2.2.2.2.1 resource (List.mem_append.mpr (Or.inr member))

theorem fresh_worker_return_requires_all_inputs_dropped
    {inputs allocated dropped : List Nat} {resource : Nat}
    (checked : workerAllocationExitChecked inputs allocated dropped (.ownedBuffer resource))
    (fresh : resource ∉ inputs) : ∀ input ∈ inputs, input ∈ dropped := by
  intro input member
  have accounted := checked.2.2.2.2.1 input (List.mem_append.mpr (Or.inl member))
  rcases accounted with dropped | returned
  · exact dropped
  · have same : input = resource := by simpa [payload] using returned
    exact False.elim (fresh (same ▸ member))

theorem unreturned_local_must_be_dropped
    {inputs allocated dropped : List Nat} {resource localResource : Nat}
    (checked : workerAllocationExitChecked inputs allocated dropped (.ownedBuffer resource))
    (member : localResource ∈ allocated) (different : localResource ≠ resource) : localResource ∈ dropped := by
  rcases worker_exit_accounts_for_every_local checked localResource member with dropped | returned
  · exact dropped
  · exact False.elim (different (by simpa [payload] using returned))

theorem worker_local_cannot_alias_an_input
    {inputs allocated dropped : List Nat} {outcome : Outcome}
    (checked : workerAllocationExitChecked inputs allocated dropped outcome)
    {resource : Nat} (input : resource ∈ inputs) : resource ∉ allocated := by
  have separate := (List.nodup_append.mp checked.1).2.2
  exact fun member => separate resource input resource member rfl

/-- Low 32 bits are a nonzero task-local success ordinal. The high 32 bits are
its successful task identity. Root identifiers retain the old ordinal encoding.
No shared allocation counter or native worker scheduling order is needed. -/
def bufferIdentity (task ordinal : Nat) : Nat := task * 4294967296 + ordinal

def checkedBufferIdentity (task ordinal : Nat) : Option Nat :=
  if task < 4294967296 ∧ 0 < ordinal ∧ ordinal < 4294967296 then
    some (bufferIdentity task ordinal)
  else none

theorem root_buffer_identity_is_unchanged (ordinal : Nat) :
    bufferIdentity 0 ordinal = ordinal := by simp [bufferIdentity]

theorem buffer_identity_is_injective
    {firstTask secondTask firstOrdinal secondOrdinal : Nat}
    (firstBound : firstOrdinal < 4294967296) (secondBound : secondOrdinal < 4294967296)
    (same : bufferIdentity firstTask firstOrdinal = bufferIdentity secondTask secondOrdinal) :
    firstTask = secondTask ∧ firstOrdinal = secondOrdinal := by
  unfold bufferIdentity at same
  omega

theorem distinct_workers_have_disjoint_generations
    {firstTask secondTask firstOrdinal secondOrdinal : Nat}
    (firstBound : firstOrdinal < 4294967296) (secondBound : secondOrdinal < 4294967296)
    (different : firstTask ≠ secondTask) :
    bufferIdentity firstTask firstOrdinal ≠ bufferIdentity secondTask secondOrdinal := by
  intro same
  exact different (buffer_identity_is_injective firstBound secondBound same).1

theorem checked_buffer_identity_is_nonzero_and_fits_u64
    {task ordinal identity : Nat} (checked : checkedBufferIdentity task ordinal = some identity) :
    0 < identity ∧ identity < 18446744073709551616 := by
  unfold checkedBufferIdentity at checked
  split at checked
  next bounds =>
    cases checked
    unfold bufferIdentity
    omega
  next => contradiction

theorem checked_buffer_identity_requires_bounded_components
    {task ordinal identity : Nat} (checked : checkedBufferIdentity task ordinal = some identity) :
    task < 4294967296 ∧ 0 < ordinal ∧ ordinal < 4294967296 ∧ identity = bufferIdentity task ordinal := by
  unfold checkedBufferIdentity at checked
  split at checked
  next bounds => cases checked; exact ⟨bounds.1, bounds.2.1, bounds.2.2, rfl⟩
  next => contradiction

def allocateInNamespace (task ordinal : Nat) (succeeds : Bool) : Option (Option Nat × Nat) := do
  let identity ← checkedBufferIdentity task ordinal
  if succeeds then some (some identity, ordinal + 1) else some (none, ordinal)

theorem failed_worker_allocation_preserves_success_ordinal
    {task ordinal next : Nat} {result : Option Nat}
    (checked : allocateInNamespace task ordinal false = some (result, next)) :
    result = none ∧ next = ordinal := by
  unfold allocateInNamespace at checked
  cases valid : checkedBufferIdentity task ordinal <;> simp [valid] at checked
  exact ⟨checked.1.symm, checked.2.symm⟩

theorem successful_worker_allocation_advances_only_local_ordinal
    {task ordinal next identity : Nat}
    (checked : allocateInNamespace task ordinal true = some (some identity, next)) :
    identity = bufferIdentity task ordinal ∧ next = ordinal + 1 := by
  unfold allocateInNamespace at checked
  cases valid : checkedBufferIdentity task ordinal with
  | none => simp [valid] at checked
  | some value =>
    simp [valid] at checked
    have shape := checked_buffer_identity_requires_bounded_components valid
    exact ⟨checked.1.symm.trans shape.2.2.2, checked.2.symm⟩

theorem exhausted_namespace_cannot_allocate (task ordinal : Nat) (succeeds : Bool)
    (exhausted : task ≥ 4294967296 ∨ ordinal = 0 ∨ ordinal ≥ 4294967296) :
    allocateInNamespace task ordinal succeeds = none := by
  have invalid : ¬ (task < 4294967296 ∧ 0 < ordinal ∧ ordinal < 4294967296) := by omega
  simp [allocateInNamespace, checkedBufferIdentity, invalid]

end QCore.OwnedTaskExecution
