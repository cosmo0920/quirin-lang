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

namespace QCore.NestedCleanup

-- Post-child accounting: allocations can be released while exact per-child
-- residual identities or an up-counter obligation remain undischargeable.
structure State where
  parentsLive : Bool
  up : Nat
  residuals : List Nat
  deriving DecidableEq

def finish (entry consumed : Nat) (residuals : List Nat) : Option State :=
  if consumed ≤ entry then some ⟨false, entry - consumed, residuals⟩ else none

def discharged (s : State) : Bool := s.residuals.isEmpty && decide (s.up = 0)

theorem accepted_finish_preserves_residual_identities {entry consumed : Nat}
    {residuals : List Nat} {after : State} (h : finish entry consumed residuals = some after) :
    after.residuals = residuals := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem accepted_finish_balances_up_counter {entry consumed : Nat}
    {residuals : List Nat} {after : State} (h : finish entry consumed residuals = some after) :
    after.up + consumed = entry := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    simp_all
  · contradiction

theorem up_counter_underflow_rejected (entry consumed : Nat) (residuals : List Nat)
    (h : entry < consumed) : finish entry consumed residuals = none := by
  simp [finish, Nat.not_le.mpr h]

theorem accepted_discharge_has_no_child_or_counter_obligation (s : State)
    (h : discharged s = true) : s.residuals = [] ∧ s.up = 0 := by
  simpa [discharged] using h

theorem unresolved_child_blocks_discharge {entry consumed : Nat}
    {residuals : List Nat} {after : State} (h : finish entry consumed residuals = some after)
    (unresolved : residuals ≠ []) : discharged after = false := by
  have same := accepted_finish_preserves_residual_identities h
  simp [discharged, same, unresolved]

end QCore.NestedCleanup

namespace QCore.NestedCleanup

-- Unclassified backend custody keeps its stream/chunk identity and discriminant;
-- it is separate from modeled OS residuals and cannot imply OS absence.
structure OpaqueState where
  known : State
  unclassified : List (Nat × Nat × Int)
  deriving DecidableEq

def finishOpaque (entry consumed : Nat) (known : List Nat)
    (unclassified : List (Nat × Nat × Int)) : Option OpaqueState :=
  (finish entry consumed known).map fun state => ⟨state, unclassified⟩

def opaqueDischarged (s : OpaqueState) : Bool :=
  discharged s.known && s.unclassified.isEmpty

theorem opaque_finish_preserves_exact_inventory {entry consumed : Nat}
    {known : List Nat} {unclassified : List (Nat × Nat × Int)} {after : OpaqueState}
    (h : finishOpaque entry consumed known unclassified = some after) :
    after.unclassified = unclassified := by
  unfold finishOpaque at h
  cases hfinish : finish entry consumed known <;> simp [hfinish] at h
  cases h
  rfl

theorem opaque_residual_blocks_parent_discharge {entry consumed : Nat}
    {known : List Nat} {unclassified : List (Nat × Nat × Int)} {after : OpaqueState}
    (h : finishOpaque entry consumed known unclassified = some after)
    (unresolved : unclassified ≠ []) : opaqueDischarged after = false := by
  simp [opaqueDischarged, opaque_finish_preserves_exact_inventory h, unresolved]

theorem opaque_discharge_requires_both_inventories_empty (s : OpaqueState)
    (h : opaqueDischarged s = true) :
    s.known.residuals = [] ∧ s.known.up = 0 ∧ s.unclassified = [] := by
  have parts : discharged s.known = true ∧ s.unclassified = [] := by
    simpa [opaqueDischarged] using h
  exact ⟨(accepted_discharge_has_no_child_or_counter_obligation s.known parts.1).1,
    (accepted_discharge_has_no_child_or_counter_obligation s.known parts.1).2, parts.2⟩

end QCore.NestedCleanup
