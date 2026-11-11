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

/-! Owned-resource reservation and completion-capsule custody for scoped tasks.
The executable ledger additionally checks task/handle/scope identities and
freshness. These proofs concern checked custody transfer and active-task use,
not source import, permission shares, runtime creation or arbitrary threads. -/
namespace QCore.ScopedTasks
inductive Custodian where
  | task (id : Nat)
  | reservation (id : Nat)
  | capsule (id : Nat)
  deriving DecidableEq, Repr
inductive Phase where
  | active
  | completed
  | joined
  deriving DecidableEq, Repr
abbrev Custody := Nat → Option Custodian

def transfer (before : Custody) (resources : List Nat)
    (expected destination : Custodian) : Option Custody :=
  if resources.Nodup ∧ ∀ resource ∈ resources, before resource = some expected then
    some (fun resource => if resource ∈ resources then some destination else before resource)
  else none

def canUse (phases : Nat → Phase) (custody : Custody) (task resource : Nat) : Bool :=
  decide (phases task = .active ∧ custody resource = some (.task task))

theorem checked_transfer_had_original_custody
    {before after : Custody} {resources : List Nat} {expected destination : Custodian}
    (checked : transfer before resources expected destination = some after) :
    resources.Nodup ∧ ∀ resource ∈ resources, before resource = some expected := by
  unfold transfer at checked
  split at checked
  · assumption
  · contradiction

theorem checked_transfer_places_exact_resources
    {before after : Custody} {resources : List Nat} {expected destination : Custodian}
    (checked : transfer before resources expected destination = some after)
    (resource : Nat) :
    after resource = if resource ∈ resources then some destination else before resource := by
  unfold transfer at checked
  split at checked
  · cases checked
    rfl
  · contradiction

theorem transfer_preserves_live_identity_domain
    {before after : Custody} {resources : List Nat} {expected destination : Custodian}
    (checked : transfer before resources expected destination = some after)
    (resource : Nat) :
    (after resource).isSome = (before resource).isSome := by
  rw [checked_transfer_places_exact_resources checked resource]
  by_cases member : resource ∈ resources
  · rw [ite_eq_left member, (checked_transfer_had_original_custody checked).2 resource member]
    rfl
  · rw [ite_eq_right member]

theorem transfer_preserves_unselected_resource
    {before after : Custody} {resources : List Nat} {expected destination : Custodian}
    (checked : transfer before resources expected destination = some after)
    (resource : Nat) (outside : resource ∉ resources) :
    after resource = before resource := by
  simpa [outside] using checked_transfer_places_exact_resources checked resource

theorem rollback_restores_original_custody
    {before reserved : Custody} {resources : List Nat} {parent reservation : Nat}
    (checked : transfer before resources (.task parent) (.reservation reservation) = some reserved) :
    transfer reserved resources (.reservation reservation) (.task parent) = some before := by
  have original := checked_transfer_had_original_custody checked
  have held : ∀ resource ∈ resources, reserved resource = some (.reservation reservation) := by
    intro resource member
    simpa [member] using checked_transfer_places_exact_resources checked resource
  unfold transfer
  rw [ite_eq_left ⟨original.1, held⟩]
  congr 1
  funext resource
  by_cases member : resource ∈ resources
  · simp [member, original.2 resource member]
  · simp [member, transfer_preserves_unselected_resource checked resource member]

theorem reserved_resource_is_unavailable_to_parent
    {before reserved : Custody} {resources : List Nat} {parent reservation : Nat}
    (checked : transfer before resources (.task parent) (.reservation reservation) = some reserved)
    (phases : Nat → Phase) (resource : Nat) (member : resource ∈ resources) :
    canUse phases reserved parent resource = false := by
  have held := checked_transfer_places_exact_resources checked resource
  simp only [ite_eq_left member] at held
  simp [canUse, held]

theorem unjoined_capsule_is_unavailable_to_every_task
    (phases : Nat → Phase) (custody : Custody) (task resource child : Nat)
    (held : custody resource = some (.capsule child)) :
    canUse phases custody task resource = false := by
  simp [canUse, held]

theorem completed_task_cannot_execute_resource_use
    (phases : Nat → Phase) (custody : Custody) (task resource : Nat)
    (completed : phases task = .completed) :
    canUse phases custody task resource = false := by
  simp [canUse, completed]

theorem joined_task_cannot_execute_resource_use
    (phases : Nat → Phase) (custody : Custody) (task resource : Nat)
    (joined : phases task = .joined) :
    canUse phases custody task resource = false := by
  simp [canUse, joined]

theorem checked_join_returns_capsule_resources_to_creator
    {before after : Custody} {resources : List Nat} {child creator : Nat}
    (checked : transfer before resources (.capsule child) (.task creator) = some after)
    (resource : Nat) (member : resource ∈ resources) :
    before resource = some (.capsule child) ∧ after resource = some (.task creator) := by
  refine ⟨(checked_transfer_had_original_custody checked).2 resource member, ?_⟩
  simpa [member] using checked_transfer_places_exact_resources checked resource
end QCore.ScopedTasks
