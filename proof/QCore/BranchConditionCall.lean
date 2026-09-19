/-! # One modeled call in a branch condition (card B02-BRANCH-CONDITION)

This is the abstract rule implemented by
`legacy_condition_call.rs` and `Parser::condition_call_instruction`.  The C
reader and the Rust-to-Lean correspondence remain review premises: this module
proves properties of the rule, not extraction of arbitrary C.

An admitted condition has one contracted call that C evaluates on every visit,
and a residual expression already admitted by the existing neutral-position
rule.  The call's existing transfer runs exactly once before the unknown
Boolean result forks to both successors.  Shapes whose evaluation is
conditional, suppressed by a type operator, contains more than one call, or
whose residual is not neutral stay refused.
-/

namespace QCore.BranchConditionCall

/-- The evaluation-frequency classes distinguished by the lexical boundary. -/
inductive Frequency
  | exactlyOnce
  | shortCircuit
  | conditional
  | typeOperator
  | multipleCalls
  deriving DecidableEq, Repr

/-- Facts handed to this rule after ordinary call and residual classification. -/
structure Candidate where
  frequency : Frequency
  callModeled : Bool
  residualNeutral : Bool
  deriving DecidableEq, Repr

/-- Only one unconditionally evaluated modeled call with a neutral residual is
admitted. -/
def admits (candidate : Candidate) : Bool :=
  candidate.frequency == Frequency.exactlyOnce &&
    candidate.callModeled && candidate.residualNeutral

/-- The two branch successors after the existing call transfer.  The branch
result is deliberately unknown, so both arms receive the same post-call state. -/
def successors {State : Type} (transfer : State → State) (state : State) : List State :=
  [transfer state, transfer state]

theorem an_admitted_candidate_has_exactly_one_evaluation
    (candidate : Candidate) (h : admits candidate = true) :
    candidate.frequency = Frequency.exactlyOnce := by
  simp [admits] at h
  exact h.1.1

theorem an_admitted_candidate_has_a_modeled_call
    (candidate : Candidate) (h : admits candidate = true) :
    candidate.callModeled = true := by
  simp [admits] at h
  exact h.1.2

theorem an_admitted_candidate_has_a_neutral_residual
    (candidate : Candidate) (h : admits candidate = true) :
    candidate.residualNeutral = true := by
  simp [admits] at h
  exact h.2

theorem short_circuit_evaluation_refuses (callModeled residualNeutral : Bool) :
    admits ⟨Frequency.shortCircuit, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem conditional_evaluation_refuses (callModeled residualNeutral : Bool) :
    admits ⟨Frequency.conditional, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem a_type_operator_refuses (callModeled residualNeutral : Bool) :
    admits ⟨Frequency.typeOperator, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem two_calls_refuse (callModeled residualNeutral : Bool) :
    admits ⟨Frequency.multipleCalls, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem an_unmodeled_call_refuses (frequency : Frequency) (residualNeutral : Bool) :
    admits ⟨frequency, false, residualNeutral⟩ = false := by
  simp [admits]

theorem a_non_neutral_residual_refuses (frequency : Frequency) (callModeled : Bool) :
    admits ⟨frequency, callModeled, false⟩ = false := by
  simp [admits]

theorem an_exactly_once_modeled_call_with_a_neutral_residual_is_admitted :
    admits ⟨Frequency.exactlyOnce, true, true⟩ = true := by
  decide

theorem both_unknown_result_arms_receive_the_post_call_state
    {State : Type} (transfer : State → State) (state : State) :
    successors transfer state = List.replicate 2 (transfer state) := by
  simp [successors]

theorem the_transfer_occurs_before_either_successor
    {State : Type} (transfer : State → State) (state thenState elseState : State)
    (h : successors transfer state = [thenState, elseState]) :
    thenState = transfer state ∧ elseState = transfer state := by
  simp [successors] at h
  exact ⟨h.1.symm, h.2.symm⟩

end QCore.BranchConditionCall
