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

namespace QCore.ParentTeardown

-- Children have already run. Their residual obligations survive allocation
-- release regardless of whether the parent API exposes a failure result.
structure State where
  parentLive : Bool
  fieldsLive : List Bool
  residualChildren : List Nat
  deriving DecidableEq

def finish (s : State) : Option State :=
  if s.parentLive then
    some { s with parentLive := false, fieldsLive := s.fieldsLive.map (fun _ => false) }
  else none

def nestedDischarged (s : State) : Bool := s.residualChildren.isEmpty

theorem parent_free_preserves_child_obligations {before after : State}
    (h : finish before = some after) : after.residualChildren = before.residualChildren := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem parent_free_cannot_establish_nested_discharge {before after : State}
    (h : finish before = some after) : nestedDischarged after = nestedDischarged before := by
  unfold nestedDischarged
  rw [parent_free_preserves_child_obligations h]

theorem accepted_parent_free_releases_parent_and_fields {before after : State}
    (h : finish before = some after) :
    after.parentLive = false ∧ ∀ field ∈ after.fieldsLive, field = false := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    simp
  · contradiction

theorem duplicate_parent_free_rejected (s : State) :
    finish { s with parentLive := false } = none := by simp [finish]

end QCore.ParentTeardown
