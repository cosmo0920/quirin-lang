namespace QCore.VerifierPlugin

/-! Cycle 20's trust boundary. Provider output is only a proposal; the trusted
core checks protocol identity and the complete evidence relation before it can
produce `safe`. -/

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
    | .rejected => .rejected
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

def compose : List Verdict → Verdict
  | [] => .unknown
  | .rejected :: _ => .rejected
  | .unknown :: _ => .unknown
  | .safe :: [] => .safe
  | .safe :: result :: results => compose (result :: results)

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
