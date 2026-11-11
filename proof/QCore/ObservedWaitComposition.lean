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

import QCore.EventObservation
import QCore.EventDriven
import QCore.MonkeyWaitProjection

namespace QCore.ObservedWaitComposition

inductive ObserverFlag where
  | functionSections | dataSections | addressUndefinedSanitizers
  deriving DecidableEq, Repr

def spellObserverFlag : ObserverFlag → String
  | .functionSections => "-ffunction-sections"
  | .dataSections => "-fdata-sections"
  | .addressUndefinedSanitizers => "-fsanitize=address,undefined"

def parseObserverFlag (value : String) : Option ObserverFlag :=
  if value == "-ffunction-sections" then some .functionSections
  else if value == "-fdata-sections" then some .dataSections
  else if value == "-fsanitize=address,undefined" then some .addressUndefinedSanitizers
  else none

theorem observer_flags_round_trip (flag : ObserverFlag) :
    parseObserverFlag (spellObserverFlag flag) = some flag := by cases flag <;> decide

theorem unsupported_thread_sanitizer_refuses :
    parseObserverFlag "-fsanitize=thread" = none := by decide

def dependencyOutputsAdmitted (modes outputs : Nat) : Bool :=
  (modes == 0 && outputs == 0) || (modes == 1 && outputs == 1)

theorem unpaired_dependency_output_refuses :
    dependencyOutputsAdmitted 0 1 = false ∧
      dependencyOutputsAdmitted 1 0 = false := by decide

/-! Semantic model of the bounded Rust join. Native identities are retained
before fresh model names are assigned. Hash computation, C import, contract
parsing and native observation truth remain external boundaries; this module
does not prove a refinement from Rust or from C. -/

structure ModelEvent where
  id : Nat
  generation : Nat
  deriving DecidableEq, Repr

def renameEvent (native : EventObservation.Identity) : ModelEvent :=
  ⟨1, native.generation⟩

def lifecycle : List (EventDriven.Lifecycle × EventDriven.TransitionKind ×
    EventDriven.Lifecycle) := [
  (.idle, .assign, .idle),
  (.idle, .add, .pending),
  (.pending, .activate, .active),
  (.active, .callbackEnter, .running),
  (.running, .callbackExitOneShot, .idle),
  (.idle, .free, .freed)]

structure JoinAssumptions where
  threads : String
  observationTrust : String
  linkageTrust : String
  nativeRunManifestSha256Valid : Bool
  deriving DecidableEq, Repr

def requiredJoinAssumptions : JoinAssumptions := {
  threads := "pthreads"
  observationTrust := "producer_supplied_runtime_evidence"
  linkageTrust := "producer_supplied_selected_artifact_linkage"
  nativeRunManifestSha256Valid := true
}

def requiredKind : MonkeyWaitProjection.RequiredObservation → EventObservation.Kind
  | .baseCreateSuccess => .baseCreate
  | .eventCreateTimeoutNoDescriptorWithPriority => .eventNew
  | .eventAddSuccess => .eventAdd
  | .dispatchBegin => .dispatchBegin
  | .timeoutActivation => .activateTimeout
  | .callbackEnter => .callbackEnterTimeout
  | .callbackExit => .callbackExit
  | .dispatchEnd => .dispatchEnd
  | .eventFree => .eventFree
  | .argumentScopeEnd => .argumentEnd
  | .baseFree => .baseFree

theorem source_obligations_match_observation_sequence (context : EventObservation.Context) :
    MonkeyWaitProjection.requiredObservations.map requiredKind =
      (EventObservation.expected context).map EventObservation.Record.kind := rfl

theorem projected_lifecycle_is_legal :
    lifecycle.all (fun (before, kind, after) =>
      EventDriven.legalLifecycle before kind after) = true := by decide

theorem model_generation_preserves_native_generation
    (native : EventObservation.Identity) :
    (renameEvent native).generation = native.generation := rfl

-- Mapping a singleton identity domain to the fresh name is injective on that
-- domain. This does not authorize identifying two native pointers.
theorem fresh_name_is_injective_on_selected_domain
    (selected left right : EventObservation.Identity)
    (leftSelected : left ∈ [selected]) (rightSelected : right ∈ [selected]) :
    left = right := by
  simp only [List.mem_singleton] at leftSelected rightSelected
  exact leftSelected.trans rightSelected.symm

def project (trace : EventObservation.Trace) (context : EventObservation.Context)
    (assumptions : JoinAssumptions) :
    Option (ModelEvent × List (EventDriven.Lifecycle × EventDriven.TransitionKind ×
      EventDriven.Lifecycle)) :=
  if assumptions = requiredJoinAssumptions then
    match EventObservation.check trace context with
    | none => none
    | some _ => some (renameEvent context.event, lifecycle)
  else none

theorem accepted_replay_requires_exact_join_assumptions
    {trace : EventObservation.Trace} {context : EventObservation.Context}
    {assumptions : JoinAssumptions}
    {result : ModelEvent × List (EventDriven.Lifecycle × EventDriven.TransitionKind ×
      EventDriven.Lifecycle)}
    (accepted : project trace context assumptions = some result) :
    assumptions = requiredJoinAssumptions := by
  unfold project at accepted
  split at accepted
  · assumption
  · contradiction

theorem accepted_projection_retains_callback_identity
    {trace : EventObservation.Trace} {context : EventObservation.Context}
    {result : ModelEvent × List (EventDriven.Lifecycle × EventDriven.TransitionKind ×
      EventDriven.Lifecycle)}
    (accepted : project trace context requiredJoinAssumptions = some result) :
    ((trace.records.drop 5).head?).bind EventObservation.Record.event =
      some context.event := by
  unfold project at accepted
  cases checked : EventObservation.check trace context with
  | none => simp [checked] at accepted
  | some certificate =>
    exact EventObservation.accepted_callback_binds_expected_event_generation checked

def wrongThreads : JoinAssumptions :=
  { requiredJoinAssumptions with threads := "single" }

def invalidManifest : JoinAssumptions :=
  { requiredJoinAssumptions with nativeRunManifestSha256Valid := false }

theorem substituted_thread_assumption_refuses
    (trace : EventObservation.Trace) (context : EventObservation.Context) :
    project trace context wrongThreads = none := by
  simp [project, wrongThreads, requiredJoinAssumptions]

theorem invalid_manifest_identity_refuses
    (trace : EventObservation.Trace) (context : EventObservation.Context) :
    project trace context invalidManifest = none := by
  simp [project, invalidManifest, requiredJoinAssumptions]

/-! ### Declarative identity-policy extraction

The executable `.qcore` policy consumes primitive text and integer facts.  The
model below states that boundary separately from native trace replay: absent
facts refuse, exact literals remain load-bearing, and actual/expected identity
pairs are compared without accepting an aggregate authority Boolean. -/

structure TextIdentity where
  actual : Option String
  expected : String
  deriving DecidableEq, Repr

def textIdentityAdmitted (identity : TextIdentity) : Bool :=
  decide (identity.actual = some identity.expected)

theorem a_missing_identity_fact_refuses (expected : String) :
    textIdentityAdmitted ⟨none, expected⟩ = false := by
  simp [textIdentityAdmitted]

theorem an_unequal_identity_fact_refuses (actual expected : String)
    (different : actual ≠ expected) :
    textIdentityAdmitted ⟨some actual, expected⟩ = false := by
  simp [textIdentityAdmitted, different]

structure WaitIdentityPolicyFacts where
  semanticProfile : Option String
  threads : Option String
  observationTrust : Option String
  linkageTrust : Option String
  manifestDigestLength : Nat
  manifestInvalidLowerHexCount : Nat
  contracts : TextIdentity
  sourceRules : TextIdentity
  observationRules : TextIdentity
  observations : TextIdentity
  bindings : TextIdentity
  sourceRevision : TextIdentity
  sourceProfile : TextIdentity
  scope : TextIdentity
  boundEntryCount : Nat
  dispatcherBound : Nat
  dispatcherBoundDeclared : Nat
  eventBound : Nat
  eventBoundDeclared : Nat
  recordBound : Nat
  recordBoundDeclared : Nat
  runBound : Nat
  runBoundDeclared : Nat
  backend : TextIdentity
  callback : TextIdentity
  flagsActual : Nat
  flagsExpected : Nat
  sourceRequirementCount : Nat
  observedRecordCount : Nat
  deriving DecidableEq, Repr

def waitIdentityPolicyAdmitted (facts : WaitIdentityPolicyFacts) : Bool :=
  facts.semanticProfile == some "cycle18.timed_wait.single_dispatcher.v1" &&
  facts.threads == some "pthreads" &&
  facts.observationTrust == some "producer_supplied_runtime_evidence" &&
  facts.linkageTrust == some "producer_supplied_selected_artifact_linkage" &&
  facts.manifestDigestLength == 64 &&
  facts.manifestInvalidLowerHexCount == 0 &&
  textIdentityAdmitted facts.contracts &&
  textIdentityAdmitted facts.sourceRules &&
  textIdentityAdmitted facts.observationRules &&
  textIdentityAdmitted facts.observations &&
  textIdentityAdmitted facts.bindings &&
  textIdentityAdmitted facts.sourceRevision &&
  textIdentityAdmitted facts.sourceProfile &&
  textIdentityAdmitted facts.scope &&
  facts.boundEntryCount == 4 &&
  facts.dispatcherBound == facts.dispatcherBoundDeclared &&
  facts.eventBound == facts.eventBoundDeclared &&
  facts.recordBound == facts.recordBoundDeclared &&
  facts.runBound == facts.runBoundDeclared &&
  facts.dispatcherBound == 1 && facts.eventBound == 1 &&
  facts.recordBound == 11 && facts.runBound == 1 &&
  textIdentityAdmitted facts.backend &&
  textIdentityAdmitted facts.callback &&
  facts.flagsActual == facts.flagsExpected &&
  facts.sourceRequirementCount == 11 && facts.observedRecordCount == 11

theorem accepted_identity_policy_has_exact_record_bound
    (facts : WaitIdentityPolicyFacts)
    (accepted : waitIdentityPolicyAdmitted facts = true) :
    facts.recordBound = 11 := by
  by_cases exact : facts.recordBound = 11
  · assumption
  · simp [waitIdentityPolicyAdmitted, exact] at accepted

theorem a_substituted_policy_digest_refuses (facts : WaitIdentityPolicyFacts)
    (different : facts.observationRules.actual ≠ some facts.observationRules.expected) :
    waitIdentityPolicyAdmitted facts = false := by
  simp [waitIdentityPolicyAdmitted, textIdentityAdmitted, different]

theorem a_missing_callback_identity_refuses (facts : WaitIdentityPolicyFacts)
    (missing : facts.callback.actual = none) :
    waitIdentityPolicyAdmitted facts = false := by
  simp [waitIdentityPolicyAdmitted, textIdentityAdmitted, missing]

theorem a_mutated_record_clause_refuses_the_eleven_record_profile
    (facts : WaitIdentityPolicyFacts) (records : facts.recordBound = 11) :
    (facts.recordBound == 12) = false := by
  simp [records]

/-! The selected policy's byte identity is admitted before any policy clause
is executed. This relation is deliberately outside `waitIdentityPolicyAdmitted`:
deleting a policy's self-binding clause cannot authorize its own bytes. -/

def selectedPolicyIdentityAdmitted
    (contextDigest selectedPolicyDigest : String) : Bool :=
  decide (contextDigest = selectedPolicyDigest)

theorem deleting_the_self_binding_clause_cannot_bypass_stale_context
    (contextDigest selectedPolicyDigest : String)
    (stale : contextDigest ≠ selectedPolicyDigest) :
    selectedPolicyIdentityAdmitted contextDigest selectedPolicyDigest = false := by
  simp [selectedPolicyIdentityAdmitted, stale]

end QCore.ObservedWaitComposition
