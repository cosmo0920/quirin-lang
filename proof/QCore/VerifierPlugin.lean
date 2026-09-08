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
    · simp only [uniform, if_true, Option.some.injEq] at published
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

end QCore.VerifierPlugin
