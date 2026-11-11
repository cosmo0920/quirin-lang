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

import QCore.CleanupOutcome

namespace QCore.FileClose

open CleanupOutcome
structure State where
  backendLive : Bool
  mapping : Custody
  descriptor : Custody
  deriving DecidableEq

def releaseBackend (state : State) : Option State :=
  if state.backendLive then some { state with backendLive := false } else none

def nestedDischarged (state : State) : Bool :=
  state.mapping == .absent && state.descriptor == .absent

theorem backend_release_preserves_mapping {before after : State}
    (h : releaseBackend before = some after) : after.mapping = before.mapping := by
  unfold releaseBackend at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem backend_release_preserves_descriptor {before after : State}
    (h : releaseBackend before = some after) : after.descriptor = before.descriptor := by
  unfold releaseBackend at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem unresolved_mapping_survives_backend_release (descriptor : Custody) :
    releaseBackend ⟨true, .unresolved, descriptor⟩ =
      some ⟨false, .unresolved, descriptor⟩ := rfl

theorem unknown_custody_rejects_discharge {state : State}
    (h : state.mapping = .unresolved ∨ state.descriptor = .unresolved) :
    nestedDischarged state = false := by
  rcases h with h | h <;> simp [nestedDischarged, h]

theorem accepted_nested_discharge_has_no_unresolved_resource {state : State}
    (h : nestedDischarged state = true) :
    state.mapping = .absent ∧ state.descriptor = .absent := by
  simpa [nestedDischarged] using h

theorem duplicate_backend_release_rejected (mapping descriptor : Custody) :
    releaseBackend ⟨false, mapping, descriptor⟩ = none := rfl

end QCore.FileClose

namespace QCore.FileClose

inductive DeletionOutcome where
  | noBackend | blocked | failed | succeeded
  deriving DecidableEq

def deletionAccepted (state : State) (outcome : DeletionOutcome) : Bool :=
  nestedDischarged state && outcome == .succeeded

theorem allocation_release_cannot_hide_failed_deletion (state : State) :
    deletionAccepted {state with backendLive := false} .failed = false := by
  simp [deletionAccepted]

theorem null_backend_is_not_successful_deletion (state : State) :
    deletionAccepted state .noBackend = false := by simp [deletionAccepted]

theorem accepted_deletion_has_no_nested_obligation {state : State} {outcome : DeletionOutcome}
    (h : deletionAccepted state outcome = true) :
    state.mapping = .absent ∧ state.descriptor = .absent ∧ outcome = .succeeded := by
  simp only [deletionAccepted, Bool.and_eq_true, beq_iff_eq] at h
  obtain ⟨resources, result⟩ := h
  obtain ⟨mapping, descriptor⟩ := accepted_nested_discharge_has_no_unresolved_resource resources
  exact ⟨mapping,descriptor,result⟩

end QCore.FileClose
