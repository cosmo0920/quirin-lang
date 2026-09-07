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
representative omits would be dropped from the published result. -/
def premisesAgree : List Certificate -> Bool
  | [] => true
  | head :: tail => tail.all (fun other => other.premises == head.premises)

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
        unitsAgree certificates && premisesAgree certificates &&
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

/-- Certificates publishing different premise sets do not merge, whether the
difference is a conflicting value or a premise one of them omits. -/
theorem distinct_premise_sets_are_unknown
    (left right : Certificate) (rest : List Outcome) (required : List Nat)
    (distinct : right.premises ≠ left.premises) :
    composeInventory (.safe left :: .safe right :: rest) required = .unknown := by
  unfold composeInventory composable
  cases gathered : collect rest with
  | none => simp [collect, gathered]
  | some tail => simp [collect, gathered, premisesAgree, distinct]

/-- A merged `safe` publishes one premise set that every component declared. -/
theorem safe_premises_are_uniform
    {outcomes : List Outcome} {required : List Nat}
    {head : Certificate} {tail : List Certificate}
    (accepted : composeInventory outcomes required = .safe)
    (gathered : collect outcomes = some (head :: tail))
    (other : Certificate) (member : other ∈ tail) :
    other.premises = head.premises := by
  have valid := composable_of_safe accepted
  unfold composable at valid
  rw [gathered] at valid
  simp at valid
  have agree : premisesAgree (head :: tail) = true := valid.1.1.2
  simp [premisesAgree] at agree
  exact agree other member

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
