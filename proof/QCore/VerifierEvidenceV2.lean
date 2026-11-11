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

namespace QCore.VerifierEvidenceV2

/-! Reduced executable W3 observed-contract replay. Required hook identities
and predicates belong to the core task. Provider records contain values only.
This model proves the nonempty denominator and precondition-counting boundary;
it neither authenticates runtime observations nor models the full Rust scalar
predicate parser, hashes, process transport or lifecycle source importer. -/

/-- The v2.0 replay checks one structural path and no claimed schedule/trace budget. -/
structure ReplayBounds where
  paths : Nat
  schedules : Option Nat
  maxOperations : Option Nat
  deriving DecidableEq, Repr

def observationBounds : ReplayBounds := ⟨1, none, none⟩

inductive ClaimScope where
  | observedRuns (runIds : List Nat)
  | functions (symbols : List Nat)
  deriving DecidableEq, Repr

/-- Natural-number module identities abstract the Rust module-path comparison. -/
structure ObservationClaim where
  scope : ClaimScope
  bounds : ReplayBounds
  declaredModule : Nat
  requiredModule : Nat
  deriving DecidableEq, Repr

def claimMatches (claim : ObservationClaim) : Bool :=
  decide (claim.scope = .observedRuns [] ∧ claim.bounds = observationBounds ∧
    claim.declaredModule = claim.requiredModule)

structure Task where
  complete : Bool
  requiredHooks : List Nat
  expectedOutput : Nat
  observationsTrusted : Bool
  observedRunsOnly : Bool
  bounds : ReplayBounds
  claims : List ObservationClaim
  deriving DecidableEq, Repr

structure Observation where
  hook : Nat
  input : Nat
  output : Nat
  deriving DecidableEq, Repr

def meetsPrecondition (record : Observation) : Bool := decide (record.input > 0)
def meetsPostcondition (task : Task) (record : Observation) : Bool :=
  decide (record.output = task.expectedOutput)

def counts (task : Task) (hook : Nat) (record : Observation) : Bool :=
  decide (record.hook = hook) && meetsPrecondition record && meetsPostcondition task record

def Accepts (task : Task) (records : List Observation) : Prop :=
  task.complete = true ∧ task.requiredHooks ≠ [] ∧
  task.requiredHooks.all (fun hook => records.any (counts task hook)) = true ∧
  records.all (fun record =>
    !(meetsPrecondition record) || meetsPostcondition task record) = true ∧
  (task.observationsTrusted = true ∧ task.observedRunsOnly = true) ∧
  task.bounds = observationBounds ∧ task.claims.all claimMatches = true

instance (task : Task) (records : List Observation) : Decidable (Accepts task records) := by
  unfold Accepts
  infer_instance

def replay (task : Task) (records : List Observation) : Bool :=
  decide (Accepts task records)

theorem accepted_has_complete_nonempty_inventory
    (task : Task) (records : List Observation) (accepted : replay task records = true) :
    task.complete = true ∧ task.requiredHooks ≠ [] := by
  have valid : Accepts task records := of_decide_eq_true accepted
  exact ⟨valid.1, valid.2.1⟩

theorem accepted_hook_has_recomputed_observation
    (task : Task) (records : List Observation) (accepted : replay task records = true)
    (hook : Nat) (required : hook ∈ task.requiredHooks) :
    ∃ record ∈ records, record.hook = hook ∧ record.input > 0 ∧
      record.output = task.expectedOutput := by
  have valid : Accepts task records := of_decide_eq_true accepted
  have found := List.all_eq_true.mp valid.2.2.1 hook required
  obtain ⟨record, member, counted⟩ := List.any_eq_true.mp found
  refine ⟨record, member, ?_⟩
  simpa [counts, meetsPrecondition, meetsPostcondition, and_assoc] using counted

theorem incomplete_inventory_rejects (task : Task) (records : List Observation)
    (incomplete : task.complete = false) : replay task records = false := by
  simp [replay, Accepts, incomplete]

theorem empty_inventory_rejects (task : Task) (records : List Observation)
    (empty : task.requiredHooks = []) : replay task records = false := by
  simp [replay, Accepts, empty]

theorem false_precondition_never_counts (task : Task) (hook : Nat)
    (record : Observation) (outside : record.input = 0) :
    counts task hook record = false := by
  simp [counts, meetsPrecondition, outside]

/-- Record the conditional trust requirement without overriding explicit refusal. -/
def applyObservationTrust (declared : Option Bool) (task : Task) : Task :=
  { task with
    observationsTrusted := declared.getD true
    observedRunsOnly := true
    complete := task.complete && declared.getD true }

theorem accepted_requires_observation_trust_and_bounds
    (task : Task) (records : List Observation) (accepted : replay task records = true) :
    task.observationsTrusted = true ∧ task.observedRunsOnly = true := by
  have valid : Accepts task records := of_decide_eq_true accepted
  exact valid.2.2.2.2.1

theorem explicit_refusal_is_preserved (task : Task) :
    (applyObservationTrust (some false) task).observationsTrusted = false := by
  rfl

theorem explicit_refusal_derivation_rejects (task : Task) (records : List Observation) :
    replay (applyObservationTrust (some false) task) records = false := by
  simp [applyObservationTrust, replay, Accepts]

theorem absent_flag_names_conditional_trust (task : Task) :
    (applyObservationTrust none task).observationsTrusted = true := by
  rfl

theorem accepted_claim_preserves_scope_bounds_and_module
    (task : Task) (records : List Observation) (accepted : replay task records = true)
    (claim : ObservationClaim) (member : claim ∈ task.claims) :
    claim.scope = .observedRuns [] ∧ claim.bounds = observationBounds ∧
      claim.declaredModule = claim.requiredModule := by
  have valid : Accepts task records := of_decide_eq_true accepted
  have checked := List.all_eq_true.mp valid.2.2.2.2.2.2 claim member
  exact of_decide_eq_true checked

theorem substituted_claim_rejects
    (task : Task) (records : List Observation) (claim : ObservationClaim)
    (member : claim ∈ task.claims) (mismatch : claimMatches claim = false) :
    replay task records = false := by
  cases result : replay task records with
  | false => rfl
  | true =>
    have checked := accepted_claim_preserves_scope_bounds_and_module task records result claim member
    have yes : claimMatches claim = true := by simpa [claimMatches] using checked
    simp [mismatch] at yes

end QCore.VerifierEvidenceV2
