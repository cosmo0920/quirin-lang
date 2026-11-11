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

import QCore.FileClose

namespace QCore.FilesystemChunkClose

-- The backend phase has already run. Final list/name/chunk release consumes
-- one total counter obligation and preserves its exact residual OS outcomes.
structure State where
  chunkLive : Bool
  primaryLinked : Bool
  stateLinked : Bool
  nameLive : Bool
  total : Nat
  up : Nat
  backend : FileClose.State
  deletion : Option FileClose.DeletionOutcome
  deriving DecidableEq

def finish (s : State) : Option State :=
  if s.chunkLive && s.primaryLinked && s.stateLinked && s.nameLive &&
      !s.backend.backendLive && decide (0 < s.total) then
    some { s with
      chunkLive := false
      primaryLinked := false
      stateLinked := false
      nameLive := false
      total := s.total - 1 }
  else none

theorem finish_preserves_backend_and_deletion {before after : State}
    (h : finish before = some after) :
    after.backend = before.backend ∧ after.deletion = before.deletion := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    exact ⟨rfl, rfl⟩
  · contradiction

theorem finish_discharges_links_and_allocations {before after : State}
    (h : finish before = some after) :
    after.chunkLive = false ∧ after.primaryLinked = false ∧
      after.stateLinked = false ∧ after.nameLive = false := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    exact ⟨rfl, rfl, rfl, rfl⟩
  · contradiction

theorem finish_decrements_only_total {before after : State}
    (h : finish before = some after) :
    after.total = before.total - 1 ∧ after.up = before.up := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    exact ⟨rfl, rfl⟩
  · contradiction

theorem zero_total_rejects (s : State) : finish {s with total := 0} = none := by
  simp [finish]

theorem duplicate_chunk_release_rejects (s : State) :
    finish {s with chunkLive := false} = none := by simp [finish]

theorem unresolved_custody_survives_chunk_release {before after : State}
    (h : finish before = some after)
    (unresolved : before.backend.mapping = .unresolved ∨
      before.backend.descriptor = .unresolved) :
    FileClose.nestedDischarged after.backend = false := by
  rw [(finish_preserves_backend_and_deletion h).1]
  exact FileClose.unknown_custody_rejects_discharge unresolved

end QCore.FilesystemChunkClose
