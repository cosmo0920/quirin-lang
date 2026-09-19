import QCore.BranchConditionCall

/-! Audited Lean companion to card B02-BRANCH-CONDITION.

The model states the evaluation-frequency boundary and the CFG ordering used by
the Rust implementation.  Faithful C token/type extraction and correspondence
between this rule and compiled Rust are explicit review premises, per standing
constraint 28; the shipped Rust fixtures and mutation harness exercise that
correspondence on the bounded shapes.
-/

#print axioms QCore.BranchConditionCall.an_admitted_candidate_has_exactly_one_evaluation
#print axioms QCore.BranchConditionCall.an_admitted_candidate_has_a_modeled_call
#print axioms QCore.BranchConditionCall.an_admitted_candidate_has_a_neutral_residual
#print axioms QCore.BranchConditionCall.short_circuit_evaluation_refuses
#print axioms QCore.BranchConditionCall.conditional_evaluation_refuses
#print axioms QCore.BranchConditionCall.a_type_operator_refuses
#print axioms QCore.BranchConditionCall.two_calls_refuse
#print axioms QCore.BranchConditionCall.an_unmodeled_call_refuses
#print axioms QCore.BranchConditionCall.a_non_neutral_residual_refuses
#print axioms
  QCore.BranchConditionCall.an_exactly_once_modeled_call_with_a_neutral_residual_is_admitted
#print axioms QCore.BranchConditionCall.both_unknown_result_arms_receive_the_post_call_state
#print axioms QCore.BranchConditionCall.the_transfer_occurs_before_either_successor
