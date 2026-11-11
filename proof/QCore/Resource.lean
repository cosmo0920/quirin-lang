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

import Init.Data.List.Perm

namespace QCore

abbrev ResourceId := Nat

/-- A reduced error-region state. `sealed` contains exactly the region-local
    obligations captured on the exceptional edge. -/
structure RegionState where
  outerOwned : List ResourceId
  sealed : List ResourceId
  released : List ResourceId
  deriving DecidableEq, Repr

namespace RegionState

/-- The concrete cleanup trace, in destructor execution order. -/
def cleanupTrace (state : RegionState) : List ResourceId := state.sealed.reverse

/-- No outer resource is owned by the region-local cleanup pack. -/
def OuterSeparated (state : RegionState) : Prop :=
  forall resource, resource ∈ state.outerOwned -> resource ∉ state.sealed

/-- Discharge preserves outer ownership and releases the sealed obligations in
    reverse acquisition order. -/
def discharge (state : RegionState) : RegionState :=
  { outerOwned := state.outerOwned
    sealed := []
    released := state.sealed.reverse ++ state.released }

theorem discharge_consumes_pack (state : RegionState) : state.discharge.sealed = [] := by
  rfl

theorem discharge_preserves_outer (state : RegionState) :
    state.discharge.outerOwned = state.outerOwned := by
  rfl

theorem discharge_order (state : RegionState) :
    state.discharge.released = state.sealed.reverse ++ state.released := by
  rfl

theorem sealed_resource_is_released (state : RegionState) (resource : ResourceId)
    (member : resource ∈ state.sealed) : resource ∈ state.discharge.released := by
  simp [discharge, member]

theorem cleanup_releases_no_outer (state : RegionState) (separated : state.OuterSeparated)
    (resource : ResourceId) (outer : resource ∈ state.outerOwned) :
    resource ∉ state.cleanupTrace := by
  intro inTrace
  have inSealed : resource ∈ state.sealed := by
    simpa [cleanupTrace] using inTrace
  exact separated resource outer inSealed

theorem cleanup_releases_each_at_most_once (state : RegionState)
    (unique : state.sealed.Nodup) : state.cleanupTrace.Nodup := by
  exact (List.reverse_perm state.sealed).nodup_iff.mpr unique

end RegionState
end QCore
