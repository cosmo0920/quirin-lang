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

namespace QCore.ChunkLifecycle

/-! Formal boundary for checked replacement, aggregate custody, and the bounded
ChunkIO teardown profile. The Rust checker supplies these Boolean
facts from its identity-indexed transition system; this module establishes the
properties exposed by an accepted certificate. -/

inductive ReplacementOutcome where
  | failed
  | succeeded
  deriving DecidableEq, Repr

structure Replacement where
  outcome : ReplacementOutcome
  oldLive : Bool
  newLive : Bool
  deriving DecidableEq, Repr

def Replacement.Valid (replacement : Replacement) : Prop :=
  (replacement.outcome = .failed ∧ replacement.oldLive = true ∧
    replacement.newLive = false) ∨
  (replacement.outcome = .succeeded ∧ replacement.oldLive = false ∧
    replacement.newLive = true)

instance (replacement : Replacement) : Decidable replacement.Valid := by
  unfold Replacement.Valid
  infer_instance

structure ReplacementCertificate where
  checked : Replacement
  deriving DecidableEq, Repr

def checkReplacement (replacement : Replacement) : Option ReplacementCertificate :=
  if replacement.Valid then some { checked := replacement } else none

private theorem checkReplacement_valid
    {replacement : Replacement} {certificate : ReplacementCertificate}
    (result : checkReplacement replacement = some certificate) : replacement.Valid := by
  unfold checkReplacement at result
  split at result
  · assumption
  · contradiction

/-- A failed realloc-like operation preserves the old identity and creates no
new live obligation. -/
theorem accepted_failed_replacement_preserves_old
    {replacement : Replacement} {certificate : ReplacementCertificate}
    (result : checkReplacement replacement = some certificate)
    (failed : replacement.outcome = .failed) :
    replacement.oldLive = true ∧ replacement.newLive = false := by
  have valid := checkReplacement_valid result
  rcases valid with valid | valid
  · exact valid.2
  · rw [failed] at valid
    cases valid.1

/-- A successful realloc-like operation invalidates the old identity exactly
when the replacement identity becomes live. -/
theorem accepted_successful_replacement_replaces_old
    {replacement : Replacement} {certificate : ReplacementCertificate}
    (result : checkReplacement replacement = some certificate)
    (succeeded : replacement.outcome = .succeeded) :
    replacement.oldLive = false ∧ replacement.newLive = true := by
  have valid := checkReplacement_valid result
  rcases valid with valid | valid
  · rw [succeeded] at valid
    cases valid.1
  · exact valid.2

structure AggregateCustody where
  ownerTerminal : Bool
  childTerminal : Bool
  childDestroyedIndependently : Bool
  deriving DecidableEq, Repr

def AggregateCustody.Valid (custody : AggregateCustody) : Prop :=
  (custody.ownerTerminal = true → custody.childTerminal = true) ∧
  custody.childDestroyedIndependently = false

instance (custody : AggregateCustody) : Decidable custody.Valid := by
  unfold AggregateCustody.Valid
  infer_instance

structure AggregateCertificate where
  checked : AggregateCustody
  deriving DecidableEq, Repr

def checkAggregate (custody : AggregateCustody) : Option AggregateCertificate :=
  if custody.Valid then some { checked := custody } else none

private theorem checkAggregate_valid
    {custody : AggregateCustody} {certificate : AggregateCertificate}
    (result : checkAggregate custody = some certificate) : custody.Valid := by
  unfold checkAggregate at result
  split at result
  · assumption
  · contradiction

theorem accepted_owner_teardown_discharges_stored_child
    {custody : AggregateCustody} {certificate : AggregateCertificate}
    (result : checkAggregate custody = some certificate)
    (owner : custody.ownerTerminal = true) : custody.childTerminal = true :=
  (checkAggregate_valid result).1 owner

theorem accepted_aggregate_rejects_independent_child_destruction
    {custody : AggregateCustody} {certificate : AggregateCertificate}
    (result : checkAggregate custody = some certificate) :
    custody.childDestroyedIndependently = false :=
  (checkAggregate_valid result).2

structure AggregateRemoval where
  ownerLive : Bool
  childContainedBefore : Bool
  childLiveAfter : Bool
  relationPresentAfter : Bool
  deriving DecidableEq, Repr

def AggregateRemoval.Valid (removal : AggregateRemoval) : Prop :=
  removal.ownerLive = true ∧
  removal.childContainedBefore = true ∧
  removal.childLiveAfter = true ∧
  removal.relationPresentAfter = false

instance (removal : AggregateRemoval) : Decidable removal.Valid := by
  unfold AggregateRemoval.Valid
  infer_instance

structure AggregateRemovalCertificate where
  checked : AggregateRemoval
  deriving DecidableEq, Repr

def checkAggregateRemoval
    (removal : AggregateRemoval) : Option AggregateRemovalCertificate :=
  if removal.Valid then some { checked := removal } else none

private theorem checkAggregateRemoval_valid
    {removal : AggregateRemoval}
    {certificate : AggregateRemovalCertificate}
    (result : checkAggregateRemoval removal = some certificate) : removal.Valid := by
  unfold checkAggregateRemoval at result
  split at result
  · assumption
  · contradiction

/-- Removing a child from an audited aggregate restores direct live custody
and removes exactly the parent/child containment relation. -/
theorem accepted_aggregate_removal_restores_child_custody
    {removal : AggregateRemoval}
    {certificate : AggregateRemovalCertificate}
    (result : checkAggregateRemoval removal = some certificate) :
    removal.childLiveAfter = true ∧
      removal.relationPresentAfter = false := by
  have valid := checkAggregateRemoval_valid result
  exact ⟨valid.2.2.1, valid.2.2.2⟩

abbrev ContextId := Nat
abbrev StreamId := Nat
abbrev ChunkId := Nat

structure Teardown where
  contextLive : Bool
  streamLive : Bool
  chunkLive : Bool
  backendObjectLive : Bool
  nestedBuffersLive : Bool
  fileMappingLive : Bool
  fileDescriptorLive : Bool
  filePathLive : Bool
  nameLive : Bool
  primaryListLinked : Bool
  residencyListLinked : Bool
  chunkCounter : Nat
  lockHeld : Bool
  busy : Bool
  outstandingTasks : Nat
  outstandingRoutes : Nat
  transactionActive : Bool
  contextIds : List ContextId
  streamIds : List StreamId
  chunkIds : List ChunkId
  failedOpenRolledBack : Bool
  nestedTeardownOrderRecorded : Bool
  deleteOnlyClassifiedPartial : Bool
  deriving DecidableEq, Repr

def Teardown.Valid (teardown : Teardown) : Prop :=
  teardown.contextLive = false ∧
  teardown.streamLive = false ∧
  teardown.chunkLive = false ∧
  teardown.backendObjectLive = false ∧
  teardown.nestedBuffersLive = false ∧
  teardown.fileMappingLive = false ∧
  teardown.fileDescriptorLive = false ∧
  teardown.filePathLive = false ∧
  teardown.nameLive = false ∧
  teardown.primaryListLinked = false ∧
  teardown.residencyListLinked = false ∧
  teardown.chunkCounter = 0 ∧
  teardown.lockHeld = false ∧
  teardown.busy = false ∧
  teardown.outstandingTasks = 0 ∧
  teardown.outstandingRoutes = 0 ∧
  teardown.transactionActive = false ∧
  teardown.contextIds.Nodup ∧
  teardown.streamIds.Nodup ∧
  teardown.chunkIds.Nodup ∧
  teardown.failedOpenRolledBack = true ∧
  teardown.nestedTeardownOrderRecorded = true ∧
  teardown.deleteOnlyClassifiedPartial = true

instance (teardown : Teardown) : Decidable teardown.Valid := by
  unfold Teardown.Valid
  infer_instance

structure TeardownCertificate where
  checked : Teardown
  deriving DecidableEq, Repr

def checkTeardown (teardown : Teardown) : Option TeardownCertificate :=
  if teardown.Valid then some { checked := teardown } else none

private theorem checkTeardown_valid
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) : teardown.Valid := by
  unfold checkTeardown at result
  split at result
  · assumption
  · contradiction

theorem accepted_chunk_teardown_discharges_nested_backend
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.backendObjectLive = false ∧
    teardown.nestedBuffersLive = false ∧
    teardown.fileMappingLive = false ∧
    teardown.fileDescriptorLive = false ∧
    teardown.filePathLive = false := by
  have valid := checkTeardown_valid result
  exact ⟨valid.2.2.2.1, valid.2.2.2.2.1, valid.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.1, valid.2.2.2.2.2.2.2.1⟩

theorem accepted_chunk_teardown_unlinks_and_balances_counter
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.primaryListLinked = false ∧
    teardown.residencyListLinked = false ∧
    teardown.chunkCounter = 0 := by
  have valid := checkTeardown_valid result
  exact ⟨valid.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.2.1⟩

theorem accepted_wrapper_teardown_has_no_outstanding_custody
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.lockHeld = false ∧ teardown.busy = false ∧
    teardown.outstandingTasks = 0 ∧ teardown.outstandingRoutes = 0 ∧
    teardown.transactionActive = false := by
  have valid := checkTeardown_valid result
  exact ⟨valid.2.2.2.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1⟩

theorem accepted_chunk_identities_are_unique
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.contextIds.Nodup ∧ teardown.streamIds.Nodup ∧
    teardown.chunkIds.Nodup := by
  have valid := checkTeardown_valid result
  exact ⟨valid.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1⟩

theorem accepted_failed_open_has_no_residual_obligation
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.failedOpenRolledBack = true :=
  (checkTeardown_valid result).2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1

theorem accepted_teardown_records_nested_order_and_partial_delete
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) :
    teardown.nestedTeardownOrderRecorded = true ∧
    teardown.deleteOnlyClassifiedPartial = true := by
  have valid := checkTeardown_valid result
  exact ⟨valid.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2⟩

theorem combined_chunk_safety
    {teardown : Teardown} {certificate : TeardownCertificate}
    (result : checkTeardown teardown = some certificate) : teardown.Valid :=
  checkTeardown_valid result

end QCore.ChunkLifecycle
