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

import Std

namespace QCore.Cycle18Composition

/-! Abstract rule model for the bounded Cycle 18 Monkey callback aggregate.

Digests and source identities are represented by positive naturals. Rust
derives those values from sealed summaries and raw evidence; this model proves
the join and refusal rules after derivation. It is not a C extraction, hash, or
runtime-attestation theorem. -/

structure CallbackIdentity where
  revision : Nat
  profile : Nat
  source : Nat
  paths : Nat
  deriving DecidableEq, Repr

structure Inventory where
  event : CallbackIdentity
  timeout : CallbackIdentity
  wait : CallbackIdentity
  deriving DecidableEq, Repr

def Inventory.valid (inventory : Inventory) : Bool :=
  inventory.event.revision != 0 &&
  inventory.event.revision == inventory.timeout.revision &&
  inventory.event.revision == inventory.wait.revision &&
  inventory.event.profile != 0 &&
  inventory.event.profile == inventory.timeout.profile &&
  inventory.event.profile == inventory.wait.profile &&
  inventory.event.source != 0 && inventory.timeout.source != 0 &&
  inventory.wait.source != 0 && inventory.event.paths == 4 &&
  inventory.timeout.paths == 3 && inventory.wait.paths == 1

def Inventory.pathCount (inventory : Inventory) : Nat :=
  inventory.event.paths + inventory.timeout.paths + inventory.wait.paths

structure EvidenceJoin where
  waitCallbackSource : Nat
  payloadCallbackSource : Nat
  timeoutCallbackSource : Nat
  observedWaitSummaryMap : Nat
  failureWaitSummaryMap : Nat
  observedWaitContractMap : Nat
  failureWaitContractMap : Nat
  waitRevision : Nat
  payloadRevision : Nat
  timeoutRevision : Nat
  waitProfile : Nat
  payloadProfile : Nat
  timeoutProfile : Nat
  waitBuild : Nat
  payloadBuild : Nat
  timeoutBuild : Nat
  waitMonkeySource : Nat
  failureMonkeySource : Nat
  timeoutMonkeySource : Nat
  failureMonkeyBuild : Nat
  payloadCheckedPaths : Nat
  waitCheckedPaths : Nat
  timeoutSourcePaths : Nat
  timeoutCheckedExecutions : Nat
  deriving DecidableEq, Repr

def evidenceMatches (inventory : Inventory) (evidence : EvidenceJoin) : Bool :=
  evidence.waitCallbackSource == inventory.wait.source &&
  evidence.payloadCallbackSource == inventory.event.source &&
  evidence.timeoutCallbackSource == inventory.timeout.source &&
  evidence.observedWaitSummaryMap != 0 &&
  evidence.observedWaitSummaryMap == evidence.failureWaitSummaryMap &&
  evidence.observedWaitContractMap != 0 &&
  evidence.observedWaitContractMap == evidence.failureWaitContractMap &&
  evidence.waitRevision == inventory.event.revision &&
  evidence.payloadRevision == inventory.event.revision &&
  evidence.timeoutRevision == inventory.event.revision &&
  evidence.waitProfile == inventory.event.profile &&
  evidence.payloadProfile == inventory.event.profile &&
  evidence.timeoutProfile == inventory.event.profile &&
  evidence.waitBuild != 0 && evidence.payloadBuild != 0 &&
  evidence.timeoutBuild != 0 && evidence.payloadCheckedPaths == 4 &&
  evidence.waitMonkeySource != 0 &&
  evidence.waitMonkeySource == evidence.failureMonkeySource &&
  evidence.waitMonkeySource == evidence.timeoutMonkeySource &&
  evidence.waitBuild == evidence.failureMonkeyBuild &&
  evidence.waitBuild == evidence.timeoutBuild &&
  evidence.waitCheckedPaths == 1 && evidence.timeoutSourcePaths == 3 &&
  evidence.timeoutCheckedExecutions == 1

structure AggregateCertificate where
  inventory : Inventory
  evidence : EvidenceJoin
  deriving DecidableEq, Repr

def check (inventory : Inventory) (evidence : EvidenceJoin) :
    Option AggregateCertificate :=
  if inventory.valid && evidenceMatches inventory evidence then
    some { inventory, evidence }
  else
    none

private theorem accepted_facts
    {inventory : Inventory} {evidence : EvidenceJoin}
    {certificate : AggregateCertificate}
    (result : check inventory evidence = some certificate) :
    inventory.valid = true ∧ evidenceMatches inventory evidence = true := by
  unfold check at result
  split at result
  · rename_i accepted
    simpa only [Bool.and_eq_true] using accepted
  · contradiction

theorem accepted_inventory_is_exact
    {inventory : Inventory} {evidence : EvidenceJoin}
    {certificate : AggregateCertificate}
    (result : check inventory evidence = some certificate) :
    inventory.event.paths = 4 ∧ inventory.timeout.paths = 3 ∧
      inventory.wait.paths = 1 := by
  have valid := (accepted_facts result).1
  simp only [Inventory.valid, Bool.and_eq_true, bne_iff_ne,
    beq_iff_eq] at valid
  omega

theorem accepted_inventory_has_eight_paths
    {inventory : Inventory} {evidence : EvidenceJoin}
    {certificate : AggregateCertificate}
    (result : check inventory evidence = some certificate) :
    inventory.pathCount = 8 := by
  rcases accepted_inventory_is_exact result with ⟨event, timeout, wait⟩
  simp [Inventory.pathCount, event, timeout, wait]

theorem accepted_evidence_joins_each_callback
    {inventory : Inventory} {evidence : EvidenceJoin}
    {certificate : AggregateCertificate}
    (result : check inventory evidence = some certificate) :
    evidence.waitCallbackSource = inventory.wait.source ∧
      evidence.payloadCallbackSource = inventory.event.source ∧
      evidence.timeoutCallbackSource = inventory.timeout.source := by
  have matched := (accepted_facts result).2
  simp only [evidenceMatches, Bool.and_eq_true, bne_iff_ne,
    beq_iff_eq] at matched
  omega

theorem accepted_failure_projection_matches_observed_wait
    {inventory : Inventory} {evidence : EvidenceJoin}
    {certificate : AggregateCertificate}
    (result : check inventory evidence = some certificate) :
    evidence.observedWaitSummaryMap = evidence.failureWaitSummaryMap ∧
      evidence.observedWaitContractMap = evidence.failureWaitContractMap ∧
      evidence.waitMonkeySource = evidence.failureMonkeySource ∧
      evidence.waitBuild = evidence.failureMonkeyBuild := by
  have matched := (accepted_facts result).2
  simp only [evidenceMatches, Bool.and_eq_true, bne_iff_ne,
    beq_iff_eq] at matched
  omega

theorem substituted_payload_source_rejects
    (inventory : Inventory) (evidence : EvidenceJoin)
    (different : evidence.payloadCallbackSource ≠ inventory.event.source) :
    check inventory evidence = none := by
  unfold check evidenceMatches
  simp [different]

inductive PublishedVerdict where
  | unknown
  deriving DecidableEq, Repr

def publish (_certificate : Option AggregateCertificate) : PublishedVerdict :=
  .unknown

/-- A complete bounded join still cannot promote the aggregate to Safe. -/
theorem aggregate_publication_remains_unknown
    (certificate : Option AggregateCertificate) :
    publish certificate = .unknown := rfl

structure CallerSize where
  sourceRevision : Nat
  profile : Nat
  size : Nat
  elementSize : Nat
  allocationBytes : Nat
  deriving DecidableEq, Repr

def callerSizeValid (expectedRevision expectedProfile : Nat)
    (caller : CallerSize) : Bool :=
  caller.sourceRevision == expectedRevision &&
  caller.profile == expectedProfile && caller.size > 0 &&
  caller.size ≤ 2147483647 && caller.elementSize > 0 &&
  caller.allocationBytes == caller.size * caller.elementSize

structure CallerSizeCertificate where
  checked : CallerSize
  deriving DecidableEq, Repr

def checkCallerSize (expectedRevision expectedProfile : Nat)
    (caller : CallerSize) : Option CallerSizeCertificate :=
  if callerSizeValid expectedRevision expectedProfile caller then
    some { checked := caller }
  else
    none

theorem accepted_caller_size_has_exact_extent
    {expectedRevision expectedProfile : Nat} {caller : CallerSize}
    {certificate : CallerSizeCertificate}
    (result : checkCallerSize expectedRevision expectedProfile caller = some certificate) :
    caller.size > 0 ∧ caller.elementSize > 0 ∧
      caller.allocationBytes = caller.size * caller.elementSize := by
  unfold checkCallerSize at result
  split at result
  · rename_i accepted
    simp only [callerSizeValid, Bool.and_eq_true, beq_iff_eq,
      decide_eq_true_eq] at accepted
    rcases accepted with ⟨⟨⟨⟨⟨_revision, _profile⟩, sizePositive⟩,
      _sizeBound⟩, elementPositive⟩, allocationExact⟩
    exact ⟨sizePositive, elementPositive, allocationExact⟩
  · contradiction

theorem zero_caller_size_rejects
    (expectedRevision expectedProfile : Nat) (caller : CallerSize)
    (zero : caller.size = 0) :
    checkCallerSize expectedRevision expectedProfile caller = none := by
  unfold checkCallerSize callerSizeValid
  simp [zero]

end QCore.Cycle18Composition
