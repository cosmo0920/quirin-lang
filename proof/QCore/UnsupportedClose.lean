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

namespace QCore.UnsupportedClose

def otherKind (memory filesystem kind : Int) : Prop :=
  kind ≠ memory ∧ kind ≠ filesystem

theorem symbolic_class_skips_both_backend_guards {memory filesystem kind : Int}
    (h : otherKind memory filesystem kind) :
    decide (kind = memory) = false ∧ decide (kind = filesystem) = false := by
  exact ⟨by simp [h.1], by simp [h.2]⟩

theorem zero_belongs_unless_explicitly_excluded (memory filesystem : Int) :
    otherKind memory filesystem 0 ↔ memory ≠ 0 ∧ filesystem ≠ 0 := by
  simp only [otherKind, ne_eq, eq_comm]

-- Abstract partial post-state of the non-memory/non-filesystem branch.
-- Source interpretation, native provenance and reachability remain premises.
structure State where
  chunkLive : Bool
  primaryLinked : Bool
  stateLinked : Bool
  nameLive : Bool
  total : Nat
  up : Nat
  backendObligations : List Nat
  backendClassified : Bool
  deriving DecidableEq

def finish (s : State) : Option State :=
  if s.chunkLive && s.primaryLinked && s.stateLinked &&
      !s.backendClassified && decide (0 < s.total) then
    some { s with
      chunkLive := false
      primaryLinked := false
      stateLinked := false
      nameLive := false
      total := s.total - 1 }
  else none

def discharged (s : State) : Bool :=
  s.backendClassified && s.backendObligations.isEmpty

theorem finish_preserves_backend_obligations {before after : State}
    (h : finish before = some after) :
    after.backendObligations = before.backendObligations := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem finish_decrements_only_total {before after : State}
    (h : finish before = some after) :
    after.total = before.total - 1 ∧ after.up = before.up := by
  unfold finish at h
  split at h
  · cases Option.some.inj h
    exact ⟨rfl, rfl⟩
  · contradiction

theorem unclassified_backend_cannot_discharge (s : State)
    (h : s.backendClassified = false) : discharged s = false := by
  simp [discharged, h]

theorem finish_cannot_establish_discharge {before after : State}
    (h : finish before = some after) : discharged after = false := by
  unfold finish at h
  split at h
  · rename_i guard
    cases Option.some.inj h
    simp_all [discharged, Bool.and_eq_true]
  · contradiction

theorem zero_total_rejects (s : State) : finish {s with total := 0} = none := by
  simp [finish]

theorem duplicate_release_rejects (s : State) :
    finish {s with chunkLive := false} = none := by simp [finish]

end QCore.UnsupportedClose
