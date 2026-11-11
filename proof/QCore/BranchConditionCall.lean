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

/-! # One modeled call in a branch condition (card B02-BRANCH-CONDITION)

This is the abstract rule implemented by
`legacy_condition_call.rs` and `Parser::condition_call_instruction`.  The C
reader and the Rust-to-Lean correspondence remain review premises: this module
proves properties of the rule, not extraction of arbitrary C.

An admitted condition has one externally declared contracted call that C
evaluates on every visit, and a residual expression already admitted by the
existing neutral-position rule.  The external-declaration premise prevents a
local recursive component from reading its provisional neutral assumption back
as admission evidence.  The call's existing transfer runs exactly once before
the unknown Boolean result forks to both successors.  Shapes whose evaluation
is conditional, suppressed by a type operator, contains more than one call, or
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
  /-- The contract was established outside this translation unit, rather than
      provisionally assumed while validating this local recursive component. -/
  externallyDeclared : Bool
  callModeled : Bool
  residualNeutral : Bool
  deriving DecidableEq, Repr

/-- Only one externally declared, unconditionally evaluated modeled call with
a neutral residual is admitted. -/
def admits (candidate : Candidate) : Bool :=
  candidate.frequency == Frequency.exactlyOnce &&
    candidate.externallyDeclared && candidate.callModeled && candidate.residualNeutral

/-- The two branch successors after the existing call transfer.  The branch
result is deliberately unknown, so both arms receive the same post-call state. -/
def successors {State : Type} (transfer : State → State) (state : State) : List State :=
  [transfer state, transfer state]

theorem an_admitted_candidate_has_exactly_one_evaluation
    (candidate : Candidate) (h : admits candidate = true) :
    candidate.frequency = Frequency.exactlyOnce := by
  simp [admits] at h
  exact h.1.1.1

theorem an_admitted_candidate_is_externally_declared
    (candidate : Candidate) (h : admits candidate = true) :
    candidate.externallyDeclared = true := by
  simp [admits] at h
  exact h.1.1.2

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

theorem short_circuit_evaluation_refuses (external callModeled residualNeutral : Bool) :
    admits ⟨Frequency.shortCircuit, external, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem conditional_evaluation_refuses (external callModeled residualNeutral : Bool) :
    admits ⟨Frequency.conditional, external, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem a_type_operator_refuses (external callModeled residualNeutral : Bool) :
    admits ⟨Frequency.typeOperator, external, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem two_calls_refuse (external callModeled residualNeutral : Bool) :
    admits ⟨Frequency.multipleCalls, external, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem a_locally_defined_call_refuses
    (frequency : Frequency) (callModeled residualNeutral : Bool) :
    admits ⟨frequency, false, callModeled, residualNeutral⟩ = false := by
  simp [admits]

theorem an_unmodeled_call_refuses
    (frequency : Frequency) (external residualNeutral : Bool) :
    admits ⟨frequency, external, false, residualNeutral⟩ = false := by
  simp [admits]

theorem a_non_neutral_residual_refuses
    (frequency : Frequency) (external callModeled : Bool) :
    admits ⟨frequency, external, callModeled, false⟩ = false := by
  simp [admits]

theorem an_exactly_once_modeled_call_with_a_neutral_residual_is_admitted :
    admits ⟨Frequency.exactlyOnce, true, true, true⟩ = true := by
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
