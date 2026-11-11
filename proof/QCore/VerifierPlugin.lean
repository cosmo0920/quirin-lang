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

namespace QCore.VerifierPlugin

/-! Cycle 20's trust boundary. Provider output is only a proposal; the trusted
core checks protocol identity and the complete evidence relation before it can
produce `safe`. This is an abstract evidence interface, not a semantic witness
checker for Rust protocol v1 (which refuses every Safe proposal). Negative
proposals also remain unknown until a counterexample checker exists. -/

inductive Verdict where
  | safe
  | rejected
  | unknown
  deriving BEq, DecidableEq, Repr

structure Evidence where
  identityMatches : Bool
  coverageComplete : Bool
  claimsReplay : Bool
  assumptionsMatch : Bool
  deterministic : Bool
  deriving DecidableEq, Repr

def Evidence.Valid (evidence : Evidence) : Prop :=
  evidence.identityMatches = true ∧
  evidence.coverageComplete = true ∧
  evidence.claimsReplay = true ∧
  evidence.assumptionsMatch = true ∧
  evidence.deterministic = true

instance (evidence : Evidence) : Decidable evidence.Valid := by
  unfold Evidence.Valid
  infer_instance

structure Response where
  proposed : Verdict
  envelopeValid : Bool
  evidence : Option Evidence
  deriving DecidableEq, Repr

def check (response : Response) : Verdict :=
  if response.envelopeValid = true then
    match response.proposed with
    | .safe =>
      match response.evidence with
      | some evidence => if evidence.Valid then .safe else .unknown
      | none => .unknown
    | .rejected => .unknown
    | .unknown => .unknown
  else
    .unknown

/-- A checked safe result necessarily has evidence accepted by the core. -/
theorem checked_safe_has_valid_evidence
    (response : Response) (accepted : check response = .safe) :
    ∃ evidence, response.evidence = some evidence ∧ evidence.Valid := by
  rcases response with ⟨proposed, envelopeValid, evidence⟩
  cases proposed <;> cases envelopeValid <;> cases evidence <;>
    simp_all [check]

/-- A provider cannot mint safety merely by returning the word `safe`. -/
theorem safe_without_evidence_is_unknown
    (envelopeValid : Bool) :
    check { proposed := .safe, envelopeValid, evidence := none } = .unknown := by
  cases envelopeValid <;> rfl

/-- Malformed or mismatched response envelopes always fail closed. -/
theorem invalid_envelope_is_unknown
    (proposed : Verdict) (evidence : Option Evidence) :
    check { proposed, envelopeValid := false, evidence } = .unknown := by
  rfl

/-- Rejected evidence cannot be promoted to a core-safe result. -/
theorem invalid_evidence_is_unknown
    (evidence : Evidence) (invalid : ¬ evidence.Valid) :
    check { proposed := .safe, envelopeValid := true, evidence := some evidence } = .unknown := by
  simp [check, invalid]

/-- A provider allegation is not a checked violation. -/
theorem rejected_proposal_is_unknown
    (envelopeValid : Bool) (evidence : Option Evidence) :
    check { proposed := .rejected, envelopeValid, evidence } = .unknown := by
  cases envelopeValid <;> rfl

def compose : List Verdict → Verdict
  | [] => .unknown
  | .rejected :: _ => .rejected
  | .unknown :: _ => .unknown
  | .safe :: [] => .safe
  | .safe :: result :: results => compose (result :: results)

/-! ## Multi-provider composition (E03)

The `compose` fold above records only the verdict order. This section models
the merge relation the Rust `compose_verification` /
`compose_verification_with_inventory` pair implements: several minted
certificates over one unit, merged against the core's own required obligation
set. Certificates here are already core-checked — the firewall's own gates are
`check` above and the evidence-v2 replay model; what is proved here is that the
*merge* cannot cover an obligation nobody discharged, cannot count one
certificate twice, cannot mix identities, premises or bounds, and cannot
republish an unchecked violation. Obligation identifiers and premise names are
natural numbers: the Rust identifiers are core-computed strings, and this model
does not repeat their derivation. -/

namespace Composition

/-- A core certificate, reduced to what the merge inspects. -/
structure Certificate where
  unitId : Nat
  /-- The premise set the certificate publishes (name, value). -/
  premises : List (Nat × Bool)
  /-- Core-computed obligation identifiers this certificate discharges. -/
  coverage : List Nat
  /-- Obligation identifier to the bound it was discharged under. -/
  bounds : List (Nat × Nat)
  deriving DecidableEq, Repr

/-- One provider result. `alleged` is an unchecked violation claim: no evidence
version ships a counterexample checker, so it carries no certificate. -/
inductive Outcome where
  | safe : Certificate -> Outcome
  | alleged : Outcome
  | unknown : Outcome
  deriving DecidableEq, Repr

/-- Every outcome must be a checked `safe` carrying its certificate. -/
def collect : List Outcome -> Option (List Certificate)
  | [] => some []
  | .safe certificate :: rest => (collect rest).map (fun rest => certificate :: rest)
  | _ :: _ => none

def unitsAgree : List Certificate -> Bool
  | [] => true
  | head :: tail => tail.all (fun other => other.unitId == head.unitId)

/-- The published composite is one certificate, so a component premise set the
representative omits would be dropped from the published result. Retained as
the pre-E03-B1 relation; `composeInventory` no longer uses it, and
`uniform_premises_still_merge` pins that its accepting cases still merge. -/
def premisesAgree : List Certificate -> Bool
  | [] => true
  | head :: tail => tail.all (fun other => other.premises == head.premises)

/-! ### E03-B1: the published composite

Before this repair the merge published one component certificate as the
composite and refused any pair whose premise sets were not equal, because that
representative would otherwise have understated the assumptions the merge rests
on. The repair publishes the composite itself, so:

* `compositePremises` is the **union**: the merge holds only where every
  component's premises hold, and a larger premise set is a weaker claim. A name
  two components bind to different values is not a union but a contradiction,
  and `premisesConsistent` still refuses it.
* `compositeCoverage` is the **union**: each identifier in it was discharged by
  some component, and it is the set compared to the core's required set.
* `representative` exists only when a component *is* the merge. -/

/-- A premise name two certificates bind to different values. -/
def premisesConsistent (certificates : List Certificate) : Bool :=
  (certificates.flatMap Certificate.premises).all (fun left =>
    (certificates.flatMap Certificate.premises).all (fun right =>
      !(left.1 == right.1) || left.2 == right.2))

/-- The union of every component's premises: what the composite publishes. -/
def compositePremises (certificates : List Certificate) : List (Nat × Bool) :=
  certificates.flatMap Certificate.premises

/-- The published representative: a component certificate that is the whole
merge, which exists only when every component is that same certificate. -/
def representative : List Certificate -> Option Certificate
  | [] => none
  | head :: tail => if tail.all (fun other => other == head) then some head else none

def boundEntries (certificates : List Certificate) : List (Nat × Nat) :=
  certificates.flatMap Certificate.bounds

def boundsAgree (certificates : List Certificate) : Bool :=
  (boundEntries certificates).all (fun left =>
    (boundEntries certificates).all (fun right =>
      !(left.1 == right.1) || left.2 == right.2))

/-- The composite covers the union of its components' coverages: a set, so a
repeated certificate contributes one identifier and not two. -/
def covered (certificates : List Certificate) : List Nat :=
  certificates.flatMap Certificate.coverage

def coversExactly (certificates : List Certificate) (required : List Nat) : Bool :=
  decide (∀ id ∈ required, id ∈ covered certificates) &&
    decide (∀ id ∈ covered certificates, id ∈ required)

def composable (outcomes : List Outcome) (required : List Nat) : Bool :=
  match collect outcomes with
  | none => false
  | some certificates =>
      !certificates.isEmpty && !required.isEmpty &&
        unitsAgree certificates && premisesConsistent certificates &&
        boundsAgree certificates && coversExactly certificates required

def composeInventory (outcomes : List Outcome) (required : List Nat) : Verdict :=
  if composable outcomes required then .safe else .unknown

theorem composable_of_safe {outcomes : List Outcome} {required : List Nat}
    (accepted : composeInventory outcomes required = .safe) :
    composable outcomes required = true := by
  unfold composeInventory at accepted
  by_cases decision : composable outcomes required = true
  · exact decision
  · simp [decision] at accepted

/-- Non-vacuity: two providers over one unit, publishing the same premise set
with disjoint coverage and agreeing bounds, do merge over the exact required
set. Without this the refusal theorems below could hold of a relation that
accepts nothing. -/
theorem disjoint_certificates_merge :
    composeInventory
      [.safe ⟨1, [(7, true)], [10, 11], [(10, 1), (11, 1)]⟩,
        .safe ⟨1, [(7, true)], [12], [(12, 1)]⟩]
      [10, 11, 12] = .safe := by
  decide

/-- Non-vacuity for duplication: a repeated certificate contributes one
identifier, so it neither widens nor destroys its own exact coverage. -/
theorem duplicate_certificate_still_merges :
    composeInventory
      [.safe ⟨1, [(7, true)], [10], [(10, 1)]⟩, .safe ⟨1, [(7, true)], [10], [(10, 1)]⟩]
      [10] = .safe := by
  decide

/-- A merge never publishes a violation: the composite verdict is `safe` or
`unknown`, so an unchecked allegation can never be republished as checked. -/
theorem composition_is_never_a_violation (outcomes : List Outcome) (required : List Nat) :
    composeInventory outcomes required ≠ .rejected := by
  unfold composeInventory
  split <;> simp

theorem collect_alleged_is_none (before after : List Outcome) :
    collect (before ++ .alleged :: after) = none := by
  induction before with
  | nil => rfl
  | cons head tail ih => cases head <;> simp [collect, ih]

theorem collect_unknown_is_none (before after : List Outcome) :
    collect (before ++ .unknown :: after) = none := by
  induction before with
  | nil => rfl
  | cons head tail ih => cases head <;> simp [collect, ih]

/-- An unchecked allegation by one provider leaves the whole merge `unknown`;
it neither dominates the composite nor is discarded from it. -/
theorem alleged_component_is_unknown (before after : List Outcome) (required : List Nat) :
    composeInventory (before ++ .alleged :: after) required = .unknown := by
  simp [composeInventory, composable, collect_alleged_is_none]

/-- A missing or failed provider is an `unknown` component and blocks the merge. -/
theorem unknown_component_is_unknown (before after : List Outcome) (required : List Nat) :
    composeInventory (before ++ .unknown :: after) required = .unknown := by
  simp [composeInventory, composable, collect_unknown_is_none]

theorem no_outcomes_is_unknown (required : List Nat) :
    composeInventory [] required = .unknown := by
  simp [composeInventory, composable, collect]

/-- A derivation with no obligations is not promoted. -/
theorem empty_inventory_is_unknown (outcomes : List Outcome) :
    composeInventory outcomes [] = .unknown := by
  unfold composeInventory composable
  cases collect outcomes <;> simp

/-- Every required obligation is discharged by some component certificate. -/
theorem safe_covers_every_required_obligation
    {outcomes : List Outcome} {required : List Nat} {inventory : List Certificate}
    (accepted : composeInventory outcomes required = .safe)
    (gathered : collect outcomes = some inventory)
    (id : Nat) (needed : id ∈ required) :
    id ∈ covered inventory := by
  have valid := composable_of_safe accepted
  unfold composable at valid
  rw [gathered] at valid
  simp at valid
  have exact := valid.2
  unfold coversExactly at exact
  simp at exact
  exact exact.1 id needed

/-- An obligation no certificate discharges keeps the merge `unknown`. This is
also the duplication statement: submitting one certificate twice adds nothing,
because coverage is a set. -/
theorem uncovered_obligation_is_unknown
    {outcomes : List Outcome} {required : List Nat} {inventory : List Certificate}
    (gathered : collect outcomes = some inventory)
    (id : Nat) (needed : id ∈ required) (absent : id ∉ covered inventory) :
    composeInventory outcomes required = .unknown := by
  cases result : composeInventory outcomes required with
  | unknown => rfl
  | rejected => exact absurd result (composition_is_never_a_violation _ _)
  | safe =>
    exact absurd (safe_covers_every_required_obligation result gathered id needed) absent

/-- Duplicating a certificate does not widen its coverage. -/
theorem duplicate_certificate_adds_no_coverage (certificate : Certificate)
    (rest : List Certificate) (id : Nat) :
    id ∈ covered (certificate :: certificate :: rest) ↔ id ∈ covered (certificate :: rest) := by
  simp [covered]

/-- Consequently, two copies of one certificate cannot discharge an obligation
that certificate does not cover. -/
theorem duplicate_evidence_cannot_cover_a_missing_obligation
    (certificate : Certificate) (required : List Nat) (id : Nat)
    (needed : id ∈ required) (absent : id ∉ certificate.coverage) :
    composeInventory [.safe certificate, .safe certificate] required = .unknown := by
  refine uncovered_obligation_is_unknown (inventory := [certificate, certificate])
    (by simp [collect]) id needed ?_
  simpa [covered] using absent

/-- Certificates over different translation-unit identities do not merge. -/
theorem distinct_unit_identities_are_unknown
    (left right : Certificate) (rest : List Outcome) (required : List Nat)
    (distinct : right.unitId ≠ left.unitId) :
    composeInventory (.safe left :: .safe right :: rest) required = .unknown := by
  unfold composeInventory composable
  cases gathered : collect rest with
  | none => simp [collect, gathered]
  | some tail => simp [collect, gathered, unitsAgree, distinct]

/-- A premise name two certificates bind to different values is a
contradiction, not a union: the merge refuses it (`QP012`). This is the
soundness half of the premise rule and is unchanged by E03-B1. -/
theorem conflicting_premise_values_are_unknown
    (left right : Certificate) (rest : List Outcome) (required : List Nat)
    (name : Nat) (leftValue rightValue : Bool)
    (leftEntry : left.premises = [(name, leftValue)])
    (rightEntry : right.premises = [(name, rightValue)])
    (distinct : leftValue ≠ rightValue) :
    composeInventory (.safe left :: .safe right :: rest) required = .unknown := by
  unfold composeInventory composable
  cases gathered : collect rest with
  | none => simp [collect, gathered]
  | some tail =>
    simp [collect, gathered, premisesConsistent, leftEntry, rightEntry,
      distinct, Ne.symm distinct]

/-- Non-vacuity for E03-B1's completeness half: two components whose premise
sets differ without conflicting now merge. Before the repair this was refused,
because the published representative would have omitted premise `8`. -/
theorem unequal_premise_sets_merge :
    composeInventory
      [.safe ⟨1, [(7, true)], [10], [(10, 1)]⟩,
        .safe ⟨1, [(7, true), (8, false)], [11], [(11, 1)]⟩]
      [10, 11] = .safe := by
  decide

/-- The retained uniform case still merges, so the repair widened the relation
and did not move it. -/
theorem uniform_premises_still_merge :
    premisesAgree [⟨1, [(7, true)], [10], [(10, 1)]⟩, ⟨1, [(7, true)], [11], [(11, 1)]⟩] = true ∧
      composeInventory
        [.safe ⟨1, [(7, true)], [10], [(10, 1)]⟩, .safe ⟨1, [(7, true)], [11], [(11, 1)]⟩]
        [10, 11] = .safe := by
  exact ⟨by decide, by decide⟩

/-- Nothing a component declared is dropped from the published composite: the
premise union contains every component premise. This is what makes publishing
the composite honest where publishing a representative was not. -/
theorem composite_publishes_every_component_premise
    (certificates : List Certificate) (certificate : Certificate)
    (member : certificate ∈ certificates)
    (premise : Nat × Bool) (declared : premise ∈ certificate.premises) :
    premise ∈ compositePremises certificates := by
  simp only [compositePremises, List.mem_flatMap]
  exact ⟨certificate, member, declared⟩

/-- And nothing is invented: every published premise belongs to a component. -/
theorem composite_premises_come_from_components
    (certificates : List Certificate) (premise : Nat × Bool)
    (published : premise ∈ compositePremises certificates) :
    ∃ certificate ∈ certificates, premise ∈ certificate.premises := by
  simpa only [compositePremises, List.mem_flatMap] using published

/-- A merged `safe` publishes a premise set that binds each name once. -/
theorem safe_premises_are_consistent
    {outcomes : List Outcome} {required : List Nat} {inventory : List Certificate}
    (accepted : composeInventory outcomes required = .safe)
    (gathered : collect outcomes = some inventory)
    (left right : Nat × Bool)
    (leftPublished : left ∈ compositePremises inventory)
    (rightPublished : right ∈ compositePremises inventory)
    (sameName : left.1 = right.1) :
    left.2 = right.2 := by
  have valid := composable_of_safe accepted
  unfold composable at valid
  rw [gathered] at valid
  simp at valid
  have consistent : premisesConsistent inventory = true := valid.1.1.2
  unfold premisesConsistent at consistent
  simp only [compositePremises] at leftPublished rightPublished
  simp only [List.all_eq_true, Bool.or_eq_true, beq_iff_eq,
    Bool.not_eq_eq_eq_not] at consistent
  rcases consistent left leftPublished right rightPublished with mismatch | equal
  · exact absurd sameName (by simpa using mismatch)
  · simpa using equal

/-- The representative is published only when a component *is* the merge: with
two distinct components there is none, so no caller can read one component's
coverage, bounds or provenance as the composite's. -/
theorem representative_is_every_component
    (certificates : List Certificate) (certificate : Certificate)
    (published : representative certificates = some certificate)
    (other : Certificate) (member : other ∈ certificates) :
    other = certificate := by
  cases certificates with
  | nil => simp [representative] at published
  | cons head tail =>
    unfold representative at published
    by_cases uniform : tail.all (fun other => other == head) = true
    · simp only [uniform, ite_true, Option.some.injEq] at published
      cases published
      rcases List.mem_cons.mp member with rfl | inTail
      · rfl
      · simpa using (List.all_eq_true.mp uniform other inTail)
    · simp [uniform] at published

/-- Two distinct components have no representative at all. -/
theorem distinct_components_have_no_representative
    (left right : Certificate) (distinct : right ≠ left) :
    representative [left, right] = none := by
  simp [representative, distinct]

/-- One obligation discharged under two different bounds does not merge. -/
theorem incomparable_bounds_are_unknown
    (left right : Certificate) (rest : List Outcome) (required : List Nat)
    (id leftBound rightBound : Nat)
    (leftEntry : left.bounds = [(id, leftBound)])
    (rightEntry : right.bounds = [(id, rightBound)])
    (distinct : leftBound ≠ rightBound) :
    composeInventory (.safe left :: .safe right :: rest) required = .unknown := by
  unfold composeInventory composable
  cases gathered : collect rest with
  | none => simp [collect, gathered]
  | some tail =>
    simp [collect, gathered, boundsAgree, boundEntries, leftEntry, rightEntry,
      distinct, Ne.symm distinct]

/-! ## Unit identity and provider-indexed obligations (E03-B2, E03-B3)

Two smaller models, both about *identifiers* rather than about the merge.

`derive_task` used to fold the capability's semantic profile into the unit's
build digest, so two capabilities of one program certified different units and
every multi-capability merge refused with `QP015`. A unit identity names the
artifact, so it is now capability-independent; the capability separation is
carried by the obligation key, which contains it. Both directions matter: the
capabilities must agree on one artifact, and two artifacts must still differ.

`ObservationCoverageComplete` carries subject `Unit`, so two providers over one
unit derived one identifier for two different claims — "every hook *I* require
was checked". The identifier is now keyed additionally by the provider and the
hook denominator the replay decides it against. -/

/-- One derivation: an artifact, asked about under one capability. -/
structure Derivation where
  artifact : Nat
  capability : Nat
  deriving DecidableEq, Repr

/-- The unit identity, deliberately not a function of the capability. -/
def unitIdentity (derivation : Derivation) : Nat := derivation.artifact

/-- An obligation key still carries the capability, so identifiers of two
capabilities over one artifact stay distinct. -/
def obligationKey (derivation : Derivation) (base : Nat) : Nat × Nat :=
  (derivation.capability, base)

/-- E03-B2, completeness half: two capabilities over one artifact agree on the
unit identity, so the merge can represent them. -/
theorem capabilities_of_one_artifact_share_a_unit
    (artifact left right : Nat) :
    unitIdentity ⟨artifact, left⟩ = unitIdentity ⟨artifact, right⟩ := rfl

/-- E03-B2, soundness half: a real artifact difference still changes the unit
identity, so `distinct_unit_identities_are_unknown` still refuses it. -/
theorem distinct_artifacts_have_distinct_units
    (left right capability : Nat) (distinct : left ≠ right) :
    unitIdentity ⟨left, capability⟩ ≠ unitIdentity ⟨right, capability⟩ := distinct

/-- Nothing is lost by dropping the capability from the unit identity: the
obligation key still separates the capabilities. -/
theorem obligation_keys_stay_capability_distinct
    (artifact left right base : Nat) (distinct : left ≠ right) :
    obligationKey ⟨artifact, left⟩ base ≠ obligationKey ⟨artifact, right⟩ base := by
  simp [obligationKey, distinct]

/-- A provider-indexed obligation key: the provider and the denominator the
core decides the obligation against. -/
def coverageKey (provider denominator : Nat) : Nat × Nat := (provider, denominator)

/-- E03-B3: two providers derive two coverage obligations, so neither
certificate can discharge the other's. -/
theorem distinct_providers_have_distinct_coverage_keys
    (left right denominator : Nat) (distinct : left ≠ right) :
    coverageKey left denominator ≠ coverageKey right denominator := by
  simp [coverageKey, distinct]

/-- The denominator is keyed too: the same provider asked about a different
hook set is a different obligation. -/
theorem distinct_denominators_have_distinct_coverage_keys
    (provider left right : Nat) (distinct : left ≠ right) :
    coverageKey provider left ≠ coverageKey provider right := by
  simp [coverageKey, distinct]

/-- And the composed consequence: if one provider's certificate covers only its
own key, the other provider's key is uncovered and the merge stays `unknown` —
the soundness that used to rest on "every component must be `Safe`". -/
theorem one_providers_certificate_leaves_the_others_key_uncovered
    (unit left right : Nat) (premises : List (Nat × Bool))
    (bounds : List (Nat × Nat)) (distinct : left ≠ right) :
    composeInventory [.safe ⟨unit, premises, [left], bounds⟩] [left, right] = .unknown := by
  refine uncovered_obligation_is_unknown (inventory := [⟨unit, premises, [left], bounds⟩])
    (by simp [collect]) right (by simp) ?_
  simp [covered, Ne.symm distinct]

/-! ## Scope dispatch with an absent provider (E03-B4)

`dispatch_scope` skipped a provider the registry could not find *before*
deriving its obligations, so those obligations never entered the required set
and the composed verdict could read `safe` off the providers that did answer.
Absence is now a refusal in the composed verdict itself. -/

/-- The composed verdict of one scope. `absent` lists providers the registry
could not find, whose obligations therefore could not be derived at all. -/
def composeScope (absent : List Nat) (outcomes : List Outcome) (required : List Nat) : Verdict :=
  if absent.isEmpty then composeInventory outcomes required else .unknown

/-- A scope naming a provider that is not installed is `unknown`, whatever the
providers that did answer returned. -/
theorem absent_provider_scope_is_unknown
    (absent : List Nat) (missing : absent ≠ []) (outcomes : List Outcome)
    (required : List Nat) :
    composeScope absent outcomes required = .unknown := by
  unfold composeScope
  cases absent with
  | nil => exact absurd rfl missing
  | cons _ _ => simp

/-- Non-vacuity, and the control for the theorem above: with no absent provider
the scope is exactly the merge, and an honest scope still composes `safe`. -/
theorem present_scope_is_the_merge (outcomes : List Outcome) (required : List Nat) :
    composeScope [] outcomes required = composeInventory outcomes required := by
  simp [composeScope]

theorem honest_scope_still_composes :
    composeScope []
      [.safe ⟨1, [(7, true)], [10], [(10, 1)]⟩, .safe ⟨1, [(7, true)], [11], [(11, 1)]⟩]
      [10, 11] = .safe := by
  decide

/-- The same scope, with one provider missing, refuses. -/
theorem honest_scope_with_an_absent_provider_refuses :
    composeScope [42]
      [.safe ⟨1, [(7, true)], [10], [(10, 1)]⟩, .safe ⟨1, [(7, true)], [11], [(11, 1)]⟩]
      [10, 11] = .unknown := by
  decide

/-! ## E03-B-O1: deriving an absent provider's obligations

`dispatch_scope` used to skip a provider the registry could not find *before*
deriving anything, so its obligations were never stated. They are stated now
wherever the derivation reads nothing from the manifest, and the absence
becomes the refusal on them. Exactly one capability reads a manifest field
(the required-hook denominator of `ObservationCoverageComplete`), and for that
one the inventory is withheld rather than stated against a denominator the core
does not have. -/

/-- An obligation identifier, as a triple: capability, artifact, denominator.
`0` is the denominator of an obligation that has none. -/
abbrev Ident := Nat × Nat × Nat

/-- The measured predicate: the capability whose derivation reads the
provider's declared denominator. -/
def manifestDependent (capability : Nat) : Bool := capability == 0

/-- What the core states for one capability of one artifact, given the
provider's declared denominator when the provider is installed. -/
def derive (capability artifact : Nat) (denominator : Option Nat) : List Ident :=
  if manifestDependent capability then
    match denominator with
    | some d => [(capability, artifact, d)]
    | none => []
  else [(capability, artifact, 0)]

/-- E03-B-O1, the completeness half: for a capability whose derivation reads
no manifest field, the absent provider's inventory *is* the installed
provider's inventory. -/
theorem absent_derivation_states_the_installed_inventory
    (capability artifact denominator : Nat)
    (independent : manifestDependent capability = false) :
    derive capability artifact none = derive capability artifact (some denominator) := by
  simp [derive, independent]

/-- The soundness half: where the denominator is missing, nothing is stated —
an empty-denominator obligation would be a vacuous claim. -/
theorem absent_derivation_withholds_a_manifest_dependent_inventory
    (capability artifact : Nat) (dependent : manifestDependent capability = true) :
    derive capability artifact none = [] := by
  simp [derive, dependent]

/-- Non-vacuity for both halves: there is a capability of each kind, and the
withheld inventory is nonempty when the denominator is available. -/
theorem the_measured_capability_table_is_not_vacuous (artifact denominator : Nat) :
    manifestDependent 0 = true ∧ manifestDependent 1 = false ∧
      (derive 0 artifact (some denominator)).length = 1 ∧
      (derive 1 artifact none).length = 1 := by
  refine ⟨rfl, rfl, ?_, ?_⟩ <;> simp [derive, manifestDependent]

/-- And stating them can only refuse harder: an obligation the core states for
a provider that is not installed is discharged by nobody, so the merge stays
`unknown` however the rest of the scope answered. -/
theorem stated_absent_obligations_only_refuse
    {outcomes : List Outcome} {inventory : List Certificate} (required extra : List Nat)
    (gathered : collect outcomes = some inventory)
    (id : Nat) (stated : id ∈ extra) (unclaimed : id ∉ covered inventory) :
    composeInventory outcomes (required ++ extra) = .unknown :=
  uncovered_obligation_is_unknown gathered id (List.mem_append_right required stated) unclaimed

/-- The scope-level refusal is unchanged by the wider inventory: absence still
dominates, whatever was stated. -/
theorem absent_provider_scope_is_unknown_however_much_is_stated
    (absent : List Nat) (missing : absent ≠ []) (outcomes : List Outcome)
    (required extra : List Nat) :
    composeScope absent outcomes (required ++ extra) = .unknown :=
  absent_provider_scope_is_unknown absent missing outcomes _

/-! ## E03-B-O2: composition is associative under its own output

A composed outcome carried no component certificate, so feeding it back
refused (`QP014`): fail-closed, but silently non-associative. Composition now
takes a composite apart into the components it was built from, so every
grouping of one component sequence decides its conflicts over exactly the same
certificates. -/

/-- A composition input: a component certificate, or the composite an earlier
composition published. -/
inductive Component where
  | leaf : Certificate -> Component
  | merged : List Certificate -> Component
  deriving Repr

/-- Composing a composite is composing its parts. -/
def flatten : List Component -> List Certificate
  | [] => []
  | .leaf certificate :: rest => certificate :: flatten rest
  | .merged certificates :: rest => certificates ++ flatten rest

def composeComponents (components : List Component) (required : List Nat) : Verdict :=
  composeInventory ((flatten components).map Outcome.safe) required

theorem collect_safe (certificates : List Certificate) :
    collect (certificates.map Outcome.safe) = some certificates := by
  induction certificates with
  | nil => rfl
  | cons head tail ih => simp [collect, ih]

theorem flatten_leaves (certificates : List Certificate) :
    flatten (certificates.map Component.leaf) = certificates := by
  induction certificates with
  | nil => rfl
  | cons head tail ih => simp [flatten, ih]

/-- The grouping is invisible: both regroupings of three components reduce to
the same component sequence, so `composeComponents` decides every conflict over
the same certificates. -/
theorem regrouping_is_the_same_component_sequence (left middle right : Certificate) :
    flatten [.merged [left, middle], .leaf right] = [left, middle, right] ∧
      flatten [.leaf left, .merged [middle, right]] = [left, middle, right] ∧
      flatten [.leaf left, .leaf middle, .leaf right] = [left, middle, right] := by
  refine ⟨?_, ?_, ?_⟩ <;> simp [flatten]

/-- E03-B-O2: composition is associative under its own output. -/
theorem composition_is_associative (left middle right : Certificate) (required : List Nat) :
    composeComponents [.merged [left, middle], .leaf right] required =
        composeComponents [.leaf left, .leaf middle, .leaf right] required ∧
      composeComponents [.leaf left, .merged [middle, right]] required =
        composeComponents [.leaf left, .leaf middle, .leaf right] required := by
  refine ⟨?_, ?_⟩ <;> simp [composeComponents, flatten]

/-- Non-vacuity: a regrouped composition still merges. -/
theorem regrouped_components_merge :
    composeComponents
      [.merged [⟨1, [(7, true)], [10], [(10, 1)]⟩, ⟨1, [(7, true)], [11], [(11, 1)]⟩],
        .leaf ⟨1, [(7, true)], [12], [(12, 1)]⟩]
      [10, 11, 12] = .safe := by
  decide

/-- And a conflict between components of two different groups is refused
exactly as it is when they are composed flat: no grouping can hide it. -/
theorem regrouping_cannot_hide_a_conflict :
    composeComponents
      [.merged [⟨1, [(7, true)], [10], [(10, 1)]⟩, ⟨1, [(7, true)], [11], [(11, 1)]⟩],
        .leaf ⟨1, [(7, false)], [12], [(12, 1)]⟩]
      [10, 11, 12] = .unknown ∧
      composeComponents
        [.leaf ⟨1, [(7, true)], [10], [(10, 1)]⟩, .leaf ⟨1, [(7, true)], [11], [(11, 1)]⟩,
          .leaf ⟨1, [(7, false)], [12], [(12, 1)]⟩]
        [10, 11, 12] = .unknown := by
  exact ⟨by decide, by decide⟩

/-- Re-submitting one composite adds no coverage: the merge is over a set. -/
theorem re_submitting_a_composite_adds_no_coverage
    (left right : Certificate) (id : Nat) :
    id ∈ covered (flatten [.merged [left, right], .merged [left, right]]) ↔
      id ∈ covered (flatten [.merged [left, right]]) := by
  have duplicated : ∀ p q : Prop, (p ∨ q ∨ p ∨ q) ↔ (p ∨ q) := by
    intro p q
    constructor
    · rintro (h | h | h | h)
      · exact Or.inl h
      · exact Or.inr h
      · exact Or.inl h
      · exact Or.inr h
    · rintro (h | h)
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  simpa [flatten, covered] using duplicated (id ∈ left.coverage) (id ∈ right.coverage)

/-! ## E03-B-O3: the tightest-envelope intersection

E03-B recorded the intersection as argued rather than measured because no
derivation was believed to emit any of the three count kinds. A census
corrects half of that: three production derivations state `BoundedPaths`
(`crates/qcore-verification/tests/composition_bounds.rs` composes two of them),
while no derivation states a schedule or operation count, which a Rust tripwire
now pins. The algebra is proved here for all three kinds. -/

inductive Paths where
  | single : Paths
  | bounded : Nat -> Paths
  deriving DecidableEq, Repr

/-- `single` is the least path space any derivation claims, so it absorbs. -/
def tighterPaths : Paths -> Paths -> Paths
  | .single, _ => .single
  | _, .single => .single
  | .bounded left, .bounded right => .bounded (min left right)

/-- `none` is "not applicable" or "structural", never "unbounded": an explicit
count constrains the composite and an absent one does not relax it. -/
def tighterCount : Option Nat -> Option Nat -> Option Nat
  | some left, some right => some (min left right)
  | some value, none => some value
  | none, some value => some value
  | none, none => none

structure Bounds where
  paths : Paths
  schedules : Option Nat
  observedOnly : Bool
  operations : Option Nat
  deriving DecidableEq, Repr

def tighter (left right : Bounds) : Bounds :=
  { paths := tighterPaths left.paths right.paths,
    schedules := tighterCount left.schedules right.schedules,
    -- A composite containing one recorded-run component is a recorded-run
    -- claim: the disjunction, never the conjunction.
    observedOnly := left.observedOnly || right.observedOnly,
    operations := tighterCount left.operations right.operations }

def pathsAtMost : Paths -> Paths -> Bool
  | .single, _ => true
  | .bounded _, .single => false
  | .bounded left, .bounded right => left <= right

def countAtMost : Option Nat -> Option Nat -> Bool
  | _, none => true
  | none, some _ => false
  | some left, some right => left <= right

/-- "`left` claims no more than `right`": a narrower path space, counts at
least as small, and no weaker a recorded-run restriction — if `right` is
restricted to recorded runs then `left` must be too, or `left` would be
claiming static safety `right` never established. -/
def atMost (left right : Bounds) : Bool :=
  pathsAtMost left.paths right.paths && countAtMost left.schedules right.schedules &&
    (left.observedOnly || !right.observedOnly) &&
    countAtMost left.operations right.operations

theorem tighter_paths_is_a_lower_bound (left right : Paths) :
    pathsAtMost (tighterPaths left right) left = true ∧
      pathsAtMost (tighterPaths left right) right = true := by
  cases left <;> cases right <;>
    simp [tighterPaths, pathsAtMost, Nat.min_le_left, Nat.min_le_right]

theorem tighter_count_is_a_lower_bound (left right : Option Nat) :
    countAtMost (tighterCount left right) left = true ∧
      countAtMost (tighterCount left right) right = true := by
  cases left <;> cases right <;>
    simp [tighterCount, countAtMost, Nat.min_le_left, Nat.min_le_right]

/-- The published envelope claims no more than either component. -/
theorem the_envelope_is_a_lower_bound (left right : Bounds) :
    atMost (tighter left right) left = true ∧ atMost (tighter left right) right = true := by
  have paths := tighter_paths_is_a_lower_bound left.paths right.paths
  have schedules := tighter_count_is_a_lower_bound left.schedules right.schedules
  have operations := tighter_count_is_a_lower_bound left.operations right.operations
  refine ⟨?_, ?_⟩ <;>
    simp [atMost, tighter, paths.1, paths.2, schedules.1, schedules.2, operations.1,
      operations.2] <;>
    cases left.observedOnly <;> cases right.observedOnly <;> simp

theorem envelope_is_commutative (left right : Bounds) : tighter left right = tighter right left := by
  cases left with
  | mk leftPaths leftSchedules leftObserved leftOperations =>
    cases right with
    | mk rightPaths rightSchedules rightObserved rightOperations =>
      cases leftPaths <;> cases rightPaths <;>
        cases leftSchedules <;> cases rightSchedules <;>
        cases leftOperations <;> cases rightOperations <;>
        cases leftObserved <;> cases rightObserved <;>
        simp [tighter, tighterPaths, tighterCount, Nat.min_comm]

theorem envelope_is_idempotent (bounds : Bounds) : tighter bounds bounds = bounds := by
  cases bounds with
  | mk paths schedules observed operations =>
    cases paths <;> cases schedules <;> cases operations <;> cases observed <;>
      simp [tighter, tighterPaths, tighterCount]

/-- A single-path component absorbs any explicit path count. -/
theorem a_single_path_absorbs (paths : Paths) :
    tighterPaths .single paths = .single ∧ tighterPaths paths .single = .single := by
  cases paths <;> simp [tighterPaths]

/-- An absent count does not relax the composite. -/
theorem an_absent_count_does_not_relax (count : Nat) :
    tighterCount none (some count) = some count ∧
      tighterCount (some count) none = some count := by
  simp [tighterCount]

/-- One recorded-run component makes the whole composite a recorded-run claim,
so it can never be presented as static source safety. -/
theorem recorded_runs_are_contagious (left right : Bounds) :
    (tighter left right).observedOnly = (left.observedOnly || right.observedOnly) := rfl

/-- Non-vacuity, kind by kind. The path row is the measured one: the two real
Boolean-field derivations state 4 and 2 paths for one imported unit, and the
composite claims 2. -/
theorem every_count_kind_takes_the_minimum :
    tighterPaths (.bounded 4) (.bounded 2) = .bounded 2 ∧
      tighterCount (some 9) (some 4) = some 4 ∧
      tighterCount (some 64) (some 16) = some 16 := by
  refine ⟨?_, ?_, ?_⟩ <;> decide

theorem pathsAtMost_refl (paths : Paths) : pathsAtMost paths paths = true := by
  cases paths <;> simp [pathsAtMost]

theorem countAtMost_refl (count : Option Nat) : countAtMost count count = true := by
  cases count <;> simp [countAtMost]

theorem atMost_refl (bounds : Bounds) : atMost bounds bounds = true := by
  simp [atMost, pathsAtMost_refl, countAtMost_refl]

theorem pathsAtMost_trans {left middle right : Paths}
    (first : pathsAtMost left middle = true) (second : pathsAtMost middle right = true) :
    pathsAtMost left right = true := by
  cases left <;> cases middle <;> cases right <;> simp_all [pathsAtMost] <;> omega

theorem countAtMost_trans {left middle right : Option Nat}
    (first : countAtMost left middle = true) (second : countAtMost middle right = true) :
    countAtMost left right = true := by
  cases left <;> cases middle <;> cases right <;> simp_all [countAtMost] <;> omega

theorem atMost_trans {left middle right : Bounds}
    (first : atMost left middle = true) (second : atMost middle right = true) :
    atMost left right = true := by
  simp only [atMost, Bool.and_eq_true] at first second ⊢
  obtain ⟨⟨⟨paths₁, schedules₁⟩, observed₁⟩, operations₁⟩ := first
  obtain ⟨⟨⟨paths₂, schedules₂⟩, observed₂⟩, operations₂⟩ := second
  refine ⟨⟨⟨pathsAtMost_trans paths₁ paths₂, countAtMost_trans schedules₁ schedules₂⟩, ?_⟩,
    countAtMost_trans operations₁ operations₂⟩
  cases hleft : left.observedOnly <;> cases hmiddle : middle.observedOnly <;>
    cases hright : right.observedOnly <;> simp_all

/-- The composite's envelope, folded over its components in order. -/
def envelope : List Bounds -> Option Bounds
  | [] => none
  | head :: tail => some (tail.foldl tighter head)

theorem fold_is_a_lower_bound :
    ∀ (rest : List Bounds) (accumulated component : Bounds),
      component ∈ accumulated :: rest →
        atMost (rest.foldl tighter accumulated) component = true := by
  intro rest
  induction rest with
  | nil =>
    intro accumulated component member
    simp only [List.mem_singleton] at member
    subst member
    simpa using atMost_refl _
  | cons head tail ih =>
    intro accumulated component member
    have step : (head :: tail).foldl tighter accumulated
        = tail.foldl tighter (tighter accumulated head) := rfl
    rw [step]
    have folded : atMost (tail.foldl tighter (tighter accumulated head))
        (tighter accumulated head) = true :=
      ih (tighter accumulated head) (tighter accumulated head) (by simp)
    rcases List.mem_cons.mp member with rfl | inRest
    · exact atMost_trans folded (the_envelope_is_a_lower_bound component head).1
    · rcases List.mem_cons.mp inRest with rfl | inTail
      · exact atMost_trans folded (the_envelope_is_a_lower_bound accumulated component).2
      · exact ih (tighter accumulated head) component (List.mem_cons_of_mem _ inTail)

/-- However many components a composite has, its published envelope claims no
more than any one of them: the merge is only as strong as its weakest part. -/
theorem the_published_envelope_bounds_every_component
    (components : List Bounds) (published component : Bounds)
    (states : envelope components = some published) (member : component ∈ components) :
    atMost published component = true := by
  cases components with
  | nil => simp at member
  | cons head tail =>
    simp only [envelope, Option.some.injEq] at states
    subst states
    exact fold_is_a_lower_bound tail head component member

/-- A composite with no component states no envelope: `none` is "states none",
never "unbounded". -/
theorem an_empty_composition_states_no_envelope : envelope [] = none := rfl

end Composition

theorem missing_plugin_composition_is_unknown : compose [] = .unknown := by
  rfl

theorem unknown_plugin_prevents_safe (before after : List Verdict) :
    compose (before ++ [.unknown] ++ after) ≠ .safe := by
  induction before with
  | nil => simp [compose]
  | cons result results ih =>
    cases result with
    | safe =>
      cases results with
      | nil => simp [compose]
      | cons head tail => simpa [compose] using ih
    | rejected => simp [compose]
    | unknown => simp [compose]

/-! Final transport classification after direct-child exit and both output EOFs.
Deadline and output-limit decisions precede this boundary in the Rust host.
This models classification, not OS scheduling or pipe behavior. -/
namespace ProcessCompletion

inductive Outcome where
  | terminated
  | io
  | decode
  deriving DecidableEq, Repr

def classify (exitSucceeded inputFailed : Bool) : Outcome :=
  if !exitSucceeded then .terminated
  else if inputFailed then .io
  else .decode

theorem failed_exit_precedes_input_failure (inputFailed : Bool) :
    classify false inputFailed = .terminated := by
  simp [classify]

theorem partial_request_never_decodes (exitSucceeded : Bool) :
    classify exitSucceeded true ≠ .decode := by
  cases exitSucceeded <;> simp [classify]

theorem decoding_requires_success_and_complete_input
    (exitSucceeded inputFailed : Bool)
    (h : classify exitSucceeded inputFailed = .decode) :
    exitSucceeded = true ∧ inputFailed = false := by
  cases exitSucceeded <;> cases inputFailed <;> simp_all [classify]

end ProcessCompletion

end QCore.VerifierPlugin
