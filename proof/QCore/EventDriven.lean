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

namespace QCore.EventDriven

/-! Cycle 18's formal boundary for the bounded Libevent profile.  The Rust
checker constructs the same lifecycle and aggregate evidence: generations and
close owners are unique, callback entry is authorized, one dispatcher gives a
single same-base callback slot, and cross-context conflicts require an
explicit ordering edge. -/

abbrev BaseId := Nat
abbrev EventId := Nat
abbrev Generation := Nat
abbrev ArgumentId := Nat
abbrev DescriptorId := Nat

structure EventKey where
  identity : EventId
  generation : Generation
  deriving DecidableEq, Repr

inductive Lifecycle where
  | idle
  | pending
  | active
  | running
  | selfTeardown
  | finalizing
  | finalized
  | freed
  deriving DecidableEq, Repr

inductive TransitionKind where
  | assign
  | add
  | activate
  | callbackEnter
  | callbackExitOneShot
  | callbackExitPersistent
  | deleteBlocking
  | deleteNonblocking
  | deleteCurrentCallback
  | freeCurrentCallback
  | callbackExitSelfTeardown
  | finalize
  | free
  deriving DecidableEq, Repr

def legalLifecycle : Lifecycle → TransitionKind → Lifecycle → Bool
  | .idle, .assign, .idle => true
  | .idle, .add, .pending => true
  | .pending, .add, .pending => true
  | .idle, .activate, .active => true
  | .pending, .activate, .active => true
  | .active, .activate, .active => true
  | .running, .activate, .running => true
  | .active, .callbackEnter, .running => true
  | .running, .callbackExitOneShot, .idle => true
  | .running, .callbackExitPersistent, .pending => true
  | .idle, .deleteBlocking, .idle => true
  | .pending, .deleteBlocking, .idle => true
  | .active, .deleteBlocking, .idle => true
  | .idle, .deleteNonblocking, .idle => true
  | .pending, .deleteNonblocking, .idle => true
  | .active, .deleteNonblocking, .idle => true
  | .running, .deleteNonblocking, .running => true
  | .running, .deleteCurrentCallback, .running => true
  | .running, .freeCurrentCallback, .selfTeardown => true
  | .selfTeardown, .callbackExitSelfTeardown, .freed => true
  | .idle, .finalize, .finalized => true
  | .finalizing, .finalize, .finalized => true
  | .idle, .free, .freed => true
  | .finalized, .free, .freed => true
  | _, _, _ => false

structure Transition where
  before : Lifecycle
  kind : TransitionKind
  after : Lifecycle
  currentGeneration : Bool
  baseLive : Bool
  deriving DecidableEq, Repr

def Transition.Valid (transition : Transition) : Prop :=
  legalLifecycle transition.before transition.kind transition.after = true ∧
  transition.currentGeneration = true ∧
  transition.baseLive = true

instance (transition : Transition) : Decidable transition.Valid := by
  unfold Transition.Valid
  infer_instance

structure TransitionCertificate where
  checked : Transition
  deriving DecidableEq, Repr

def checkTransition (transition : Transition) : Option TransitionCertificate :=
  if transition.Valid then some { checked := transition } else none

private theorem checkTransition_valid
    {transition : Transition} {certificate : TransitionCertificate}
    (result : checkTransition transition = some certificate) : transition.Valid := by
  unfold checkTransition at result
  split at result
  · assumption
  · contradiction

/-- Every accepted lifecycle step belongs to the modeled transition relation. -/
theorem accepted_transition_preserves_lifecycle
    {transition : Transition} {certificate : TransitionCertificate}
    (result : checkTransition transition = some certificate) :
    legalLifecycle transition.before transition.kind transition.after = true :=
  (checkTransition_valid result).1

/-- Accepted steps cannot use a stale generation or a freed base. -/
theorem accepted_transition_is_generation_safe
    {transition : Transition} {certificate : TransitionCertificate}
    (result : checkTransition transition = some certificate) :
    transition.currentGeneration = true ∧ transition.baseLive = true :=
  ⟨(checkTransition_valid result).2.1, (checkTransition_valid result).2.2⟩

inductive BlockingReason where
  | io
  | timeout
  | signal
  | explicitLoopBlock
  deriving DecidableEq, Repr

structure Execution where
  lifecyclePreserved : Bool
  callbackArgumentLive : Bool
  callbackAuthorityDeclared : Bool
  sameBaseSingleDispatcher : Bool
  crossContextConflictsOrdered : Bool
  descriptorOwners : List DescriptorId
  eventKeys : List EventKey
  persistentCustodyPreserved : Bool
  deferredCustodyPreserved : Bool
  teardownQuiescent : Bool
  cleanupExactlyOnce : Bool
  blockingReason : Option BlockingReason
  progressClaimQualified : Bool
  observedTrace : List Nat
  allowedTrace : List Nat
  deriving DecidableEq, Repr

def Execution.Valid (execution : Execution) : Prop :=
  execution.lifecyclePreserved = true ∧
  execution.callbackArgumentLive = true ∧
  execution.callbackAuthorityDeclared = true ∧
  execution.sameBaseSingleDispatcher = true ∧
  execution.crossContextConflictsOrdered = true ∧
  execution.descriptorOwners.Nodup ∧
  execution.eventKeys.Nodup ∧
  execution.persistentCustodyPreserved = true ∧
  execution.deferredCustodyPreserved = true ∧
  execution.teardownQuiescent = true ∧
  execution.cleanupExactlyOnce = true ∧
  execution.progressClaimQualified = true ∧
  (execution.progressClaimQualified = true → execution.blockingReason.isSome = true) ∧
  execution.observedTrace = execution.allowedTrace

instance (execution : Execution) : Decidable execution.Valid := by
  unfold Execution.Valid
  infer_instance

structure Certificate where
  checked : Execution
  deriving DecidableEq, Repr

def check (execution : Execution) : Option Certificate :=
  if execution.Valid then some { checked := execution } else none

private theorem check_valid {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) : execution.Valid := by
  unfold check at result
  split at result
  · assumption
  · contradiction

theorem accepted_lifecycles_are_well_formed
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.lifecyclePreserved = true :=
  (check_valid result).1

theorem accepted_callback_has_live_declared_authority
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.callbackArgumentLive = true ∧
    execution.callbackAuthorityDeclared = true :=
  ⟨(check_valid result).2.1, (check_valid result).2.2.1⟩

theorem accepted_same_base_callbacks_are_exclusive
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.sameBaseSingleDispatcher = true :=
  (check_valid result).2.2.2.1

theorem accepted_cross_context_conflicts_are_ordered
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.crossContextConflictsOrdered = true :=
  (check_valid result).2.2.2.2.1

theorem accepted_close_authority_and_generations_are_unique
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.descriptorOwners.Nodup ∧ execution.eventKeys.Nodup :=
  ⟨(check_valid result).2.2.2.2.2.1,
    (check_valid result).2.2.2.2.2.2.1⟩

theorem accepted_persistent_and_deferred_callbacks_preserve_custody
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.persistentCustodyPreserved = true ∧
    execution.deferredCustodyPreserved = true :=
  ⟨(check_valid result).2.2.2.2.2.2.2.1,
    (check_valid result).2.2.2.2.2.2.2.2.1⟩

theorem accepted_teardown_is_quiescent
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.teardownQuiescent = true :=
  (check_valid result).2.2.2.2.2.2.2.2.2.1

theorem accepted_cleanup_discharges_owned_resources_once
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.cleanupExactlyOnce = true :=
  (check_valid result).2.2.2.2.2.2.2.2.2.2.1

theorem accepted_progress_claim_is_qualified_by_blocking
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.blockingReason.isSome = true := by
  have valid := check_valid result
  exact valid.2.2.2.2.2.2.2.2.2.2.2.2.1 valid.2.2.2.2.2.2.2.2.2.2.2.1

theorem accepted_observation_refines_event_trace
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) :
    execution.observedTrace = execution.allowedTrace :=
  (check_valid result).2.2.2.2.2.2.2.2.2.2.2.2.2

theorem combined_cycle18_safety
    {execution : Execution} {certificate : Certificate}
    (result : check execution = some certificate) : execution.Valid :=
  check_valid result

end QCore.EventDriven
