import QCore.LoopSwitchCallEvaluation

/-! Audit for card `B03-LOOP-AND-SWITCH`.

These statements pin the exact evaluation frequencies of the modeled call,
the retained loop exit/back edge, the continue target, and the switch's
single evaluation. They deliberately make no extraction claim about the Rust
implementation; that correspondence remains the review-maintained premise
recorded by constraint 28.
-/

#print axioms QCore.LoopSwitchCallEvaluation.pre_test_normal_exit_has_one_final_evaluation
#print axioms QCore.LoopSwitchCallEvaluation.pre_test_abrupt_exit_has_no_final_evaluation
#print axioms QCore.LoopSwitchCallEvaluation.post_test_normal_exit_evaluates_after_every_body
#print axioms QCore.LoopSwitchCallEvaluation.post_test_abrupt_exit_skips_the_last_evaluation
#print axioms QCore.LoopSwitchCallEvaluation.every_condition_visit_applies_the_modeled_effect
#print axioms QCore.LoopSwitchCallEvaluation.a_modeled_call_never_removes_the_loop_exit
#print axioms QCore.LoopSwitchCallEvaluation.a_modeled_call_keeps_the_loop_body_edge
#print axioms QCore.LoopSwitchCallEvaluation.every_continue_route_revisits_the_call
#print axioms QCore.LoopSwitchCallEvaluation.a_for_continue_runs_increment_before_the_call
#print axioms QCore.LoopSwitchCallEvaluation.switch_dispatch_applies_the_effect_once
