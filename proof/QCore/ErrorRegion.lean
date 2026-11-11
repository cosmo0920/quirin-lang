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

namespace QCore.ErrorRegion

abbrev RegionName := Nat
abbrev ErrorName := Nat
abbrev ResourceId := Nat
abbrev Ledger := List ResourceId

/-- The region-local state retained while the region body executes. -/
structure RegionFrame where
  region : RegionName
  baseline : Ledger
  acquisitionLog : List ResourceId
  creditRemainder : Nat
  deriving DecidableEq

/-- An error edge seals one exact, path-specific snapshot of the frame. -/
structure CleanupPack where
  region : RegionName
  error : ErrorName
  baseline : Ledger
  actions : List ResourceId
  creditRemainder : Nat
  deriving DecidableEq

def sealPack (error : ErrorName) (frame : RegionFrame) : CleanupPack :=
  { region := frame.region
    error := error
    baseline := frame.baseline
    actions := frame.acquisitionLog
    creditRemainder := frame.creditRemainder }

theorem seal_is_path_specific (error : ErrorName) (frame : RegionFrame) :
    (sealPack error frame).actions = frame.acquisitionLog ∧
      (sealPack error frame).creditRemainder = frame.creditRemainder := by
  exact ⟨rfl, rfl⟩

/-- Opening changes sealed evidence into a linear capability.  The only other
    state is `consumed`, which deliberately contains no reusable pack. -/
inductive CleanupCapability where
  | opened (pack : CleanupPack)
  | consumed
  deriving DecidableEq

def quantOpen (requested : RegionName) (pack : CleanupPack) :
    Option CleanupCapability :=
  if requested = pack.region then some (.opened pack) else none

theorem quant_open_matching_region (pack : CleanupPack) :
    quantOpen pack.region pack = some (.opened pack) := by
  simp [quantOpen]

theorem quant_open_wrong_region (pack : CleanupPack) (requested : RegionName)
    (wrong : requested ≠ pack.region) : quantOpen requested pack = none := by
  simp [quantOpen, wrong]

structure DischargeResult where
  ledger : Ledger
  cleanupTrace : List ResourceId
  abandonedCredit : Nat
  capability : CleanupCapability
  deriving DecidableEq

/-- Discharge restores the baseline, emits reverse-acquisition (LIFO) cleanup,
    abandons exactly the sealed region-local credit, and consumes the pack. -/
def discharge : CleanupCapability → Option DischargeResult
  | .opened pack => some {
      ledger := pack.baseline
      cleanupTrace := pack.actions.reverse
      abandonedCredit := pack.creditRemainder
      capability := .consumed }
  | .consumed => none

theorem discharge_restores_region_baseline (pack : CleanupPack) :
    ∃ result, discharge (.opened pack) = some result ∧
      result.ledger = pack.baseline := by
  refine ⟨{
    ledger := pack.baseline
    cleanupTrace := pack.actions.reverse
    abandonedCredit := pack.creditRemainder
    capability := .consumed }, rfl, rfl⟩

theorem discharge_is_reverse_acquisition_order (pack : CleanupPack) :
    ∃ result, discharge (.opened pack) = some result ∧
      result.cleanupTrace = pack.actions.reverse := by
  refine ⟨{
    ledger := pack.baseline
    cleanupTrace := pack.actions.reverse
    abandonedCredit := pack.creditRemainder
    capability := .consumed }, rfl, rfl⟩

theorem discharge_releases_exactly_pack_resources (pack : CleanupPack)
    (resource : ResourceId) :
    resource ∈ pack.actions.reverse ↔ resource ∈ pack.actions := by
  simp

theorem discharge_abandons_exactly_region_credit (pack : CleanupPack) :
    ∃ result, discharge (.opened pack) = some result ∧
      result.abandonedCredit = pack.creditRemainder := by
  refine ⟨{
    ledger := pack.baseline
    cleanupTrace := pack.actions.reverse
    abandonedCredit := pack.creditRemainder
    capability := .consumed }, rfl, rfl⟩

theorem cleanup_capability_consumed_exactly_once (pack : CleanupPack) :
    ∃ result, discharge (.opened pack) = some result ∧
      result.capability = .consumed ∧ discharge result.capability = none := by
  refine ⟨{
    ledger := pack.baseline
    cleanupTrace := pack.actions.reverse
    abandonedCredit := pack.creditRemainder
    capability := .consumed }, rfl, rfl, rfl⟩

/-- A cleanup-free catch is legal precisely when neither live actions nor
    strict credit remain in the pack. -/
def CatchWithoutOpen (pack : CleanupPack) : Prop :=
  pack.actions = [] ∧ pack.creditRemainder = 0

theorem live_obligations_cannot_be_abandoned (pack : CleanupPack)
    (live : pack.actions ≠ []) : ¬CatchWithoutOpen pack := by
  intro accepted
  exact live accepted.1

theorem strict_credit_cannot_be_abandoned_on_normal_catch (pack : CleanupPack)
    (credit : pack.creditRemainder ≠ 0) : ¬CatchWithoutOpen pack := by
  intro accepted
  exact credit accepted.2

/-- Moving a resource to an error payload removes it from the cleanup pack. -/
def excludePayload (actions payload : List ResourceId) : List ResourceId :=
  actions.filter fun resource => !payload.contains resource

theorem payload_resource_is_excluded (actions payload : List ResourceId)
    (resource : ResourceId) (moved : resource ∈ payload) :
    resource ∉ excludePayload actions payload := by
  simp [excludePayload, moved]

theorem remaining_cleanup_resource_is_not_in_payload
    (actions payload : List ResourceId) (resource : ResourceId)
    (member : resource ∈ excludePayload actions payload) : resource ∉ payload := by
  simp only [excludePayload, List.mem_filter] at member
  simpa using member.2

/-- A failure after any initialized prefix seals exactly that prefix; resources
    never acquired on the path cannot enter its pack. -/
theorem partial_initialization_is_exact (region : RegionName) (error : ErrorName)
    (baseline initialized : Ledger) (credit : Nat) :
    (sealPack error {
      region := region
      baseline := baseline
      acquisitionLog := initialized
      creditRemainder := credit }).actions = initialized := by
  rfl

/-- Nested propagation discharges the inner pack before the outer pack, while
    preserving LIFO order independently inside each region. -/
def nestedCleanupOrder (inner outer : CleanupPack) : List ResourceId :=
  inner.actions.reverse ++ outer.actions.reverse

theorem nested_cleanup_order_is_deterministic (inner outer : CleanupPack) :
    nestedCleanupOrder inner outer =
      inner.actions.reverse ++ outer.actions.reverse := by
  rfl

end QCore.ErrorRegion
