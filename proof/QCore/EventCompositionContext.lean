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

namespace QCore.EventCompositionContext

/-! Caller-held identity binding for observed Cycle 18 composition evidence.

The identities below abstract canonical strings and SHA-256 values as positive
natural numbers. The Rust boundary checks concrete syntax and hashes raw
payload bytes. This model proves the equality and refusal relation after those
representations have been derived; it is not a hash theorem or runtime
attestation. -/

structure ClaimIdentity where
  artifact : Nat
  property : Nat
  semanticProfile : Nat
  scope : List Nat
  bounds : List (Nat × Nat)
  assumptions : List (Nat × Nat)
  evidenceVersion : Nat
  deriving DecidableEq, Repr

structure SourceIdentity where
  compileTask : Nat
  fluentBitRevision : Nat
  monkeySource : Nat
  monkeyProfile : Nat
  monkeyBuild : Nat
  deriving DecidableEq, Repr

structure LibeventIdentity where
  revision : Nat
  source : Nat
  profile : Nat
  build : Nat
  deriving DecidableEq, Repr

structure PolicyIdentity where
  contracts : Nat
  sourceRules : Nat
  observationRules : Nat
  deriving DecidableEq, Repr

structure ObserverIdentity where
  binary : Nat
  harness : Nat
  linkedMonkey : Nat
  linkedLibevent : Nat
  declaredProducerTrust : Bool
  deriving DecidableEq, Repr

structure Context where
  claim : ClaimIdentity
  source : SourceIdentity
  libevent : LibeventIdentity
  policy : PolicyIdentity
  observer : ObserverIdentity
  observations : Nat
  bindings : Nat
  deriving DecidableEq, Repr

structure PayloadIdentity where
  observations : Nat
  bindings : Nat
  deriving DecidableEq, Repr

def positivePairs (values : List (Nat × Nat)) : Bool :=
  values.all fun value => value.1 != 0 && value.2 != 0

def Context.complete (context : Context) : Bool :=
  context.claim.artifact != 0 &&
  context.claim.property != 0 &&
  context.claim.semanticProfile != 0 &&
  !context.claim.scope.isEmpty && context.claim.scope.all (· != 0) &&
  !context.claim.bounds.isEmpty && positivePairs context.claim.bounds &&
  !context.claim.assumptions.isEmpty && positivePairs context.claim.assumptions &&
  context.claim.evidenceVersion != 0 &&
  context.source.compileTask != 0 &&
  context.source.fluentBitRevision != 0 &&
  context.source.monkeySource != 0 &&
  context.source.monkeyProfile != 0 &&
  context.source.monkeyBuild != 0 &&
  context.libevent.revision != 0 &&
  context.libevent.source != 0 &&
  context.libevent.profile != 0 &&
  context.libevent.build != 0 &&
  context.policy.contracts != 0 &&
  context.policy.sourceRules != 0 &&
  context.policy.observationRules != 0 &&
  context.observer.binary != 0 &&
  context.observer.harness != 0 &&
  context.observer.linkedMonkey != 0 &&
  context.observer.linkedLibevent != 0 &&
  context.observer.declaredProducerTrust &&
  context.observations != 0 && context.bindings != 0

def payloadMatches (context : Context) (payload : PayloadIdentity) : Bool :=
  context.observations == payload.observations &&
  context.bindings == payload.bindings

structure ValidatedContext where
  checked : Context
  deriving DecidableEq, Repr

def check (presented expected : Context) (payload : PayloadIdentity) :
    Option ValidatedContext :=
  if presented.complete && expected.complete &&
      decide (presented = expected) && payloadMatches presented payload then
    some { checked := presented }
  else
    none

private theorem check_facts
    {presented expected : Context} {payload : PayloadIdentity}
    {validated : ValidatedContext}
    (result : check presented expected payload = some validated) :
    presented.complete = true ∧ expected.complete = true ∧
      presented = expected ∧ payloadMatches presented payload = true := by
  unfold check at result
  split at result
  · rename_i accepted
    simp only [Bool.and_eq_true, decide_eq_true_eq] at accepted
    exact ⟨accepted.1.1.1, accepted.1.1.2, accepted.1.2, accepted.2⟩
  · contradiction

theorem accepted_context_is_complete
    {presented expected : Context} {payload : PayloadIdentity}
    {validated : ValidatedContext}
    (result : check presented expected payload = some validated) :
    presented.complete = true ∧ expected.complete = true :=
  ⟨(check_facts result).1, (check_facts result).2.1⟩

theorem accepted_context_equals_caller_expected
    {presented expected : Context} {payload : PayloadIdentity}
    {validated : ValidatedContext}
    (result : check presented expected payload = some validated) :
    presented = expected :=
  (check_facts result).2.2.1

theorem accepted_context_rederives_payload
    {presented expected : Context} {payload : PayloadIdentity}
    {validated : ValidatedContext}
    (result : check presented expected payload = some validated) :
    presented.observations = payload.observations ∧
      presented.bindings = payload.bindings := by
  have matched := (check_facts result).2.2.2
  simp [payloadMatches, Bool.and_eq_true] at matched
  exact matched

theorem unequal_context_rejects
    (presented expected : Context) (payload : PayloadIdentity)
    (different : presented ≠ expected) :
    check presented expected payload = none := by
  unfold check
  simp [different]

theorem scope_substitution_rejects
    (presented expected : Context) (payload : PayloadIdentity)
    (different : presented.claim.scope ≠ expected.claim.scope) :
    check presented expected payload = none := by
  apply unequal_context_rejects presented expected payload
  intro equal
  exact different (congrArg (fun context => context.claim.scope) equal)

theorem observation_payload_substitution_rejects
    (presented expected : Context) (payload : PayloadIdentity)
    (different : presented.observations ≠ payload.observations) :
    check presented expected payload = none := by
  unfold check payloadMatches
  simp [different]

theorem binding_payload_substitution_rejects
    (presented expected : Context) (payload : PayloadIdentity)
    (different : presented.bindings ≠ payload.bindings) :
    check presented expected payload = none := by
  unfold check payloadMatches
  simp [different]

inductive PublishedStatus where
  | unknown
  deriving DecidableEq, Repr

def statusAfterContextValidation (_validated : ValidatedContext) : PublishedStatus :=
  .unknown

/-- Context equality alone cannot publish `Safe` or composed callback status. -/
theorem context_validation_remains_unknown (validated : ValidatedContext) :
    statusAfterContextValidation validated = .unknown := rfl

end QCore.EventCompositionContext
