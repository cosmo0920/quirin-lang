import QCore.LoopSwitchCallEvaluation

/-! Audit for card `B03-LOOP-AND-SWITCH`.

These statements pin the exact evaluation frequencies of the modeled call,
the retained loop exit/back edge, the continue target, and the switch's
single evaluation. They deliberately make no extraction claim about the Rust
implementation; that correspondence remains the review-maintained premise
recorded by constraint 28.
-/

#print axioms QCore.LoopSwitchCallEvaluation.a_pre_test_loop_evaluates_before_entry_and_after_each_iteration
#print axioms QCore.LoopSwitchCallEvaluation.a_post_test_loop_evaluates_after_every_completed_iteration
#print axioms QCore.LoopSwitchCallEvaluation.a_switch_evaluates_exactly_once
#print axioms QCore.LoopSwitchCallEvaluation.every_visit_applies_the_modeled_effect
#print axioms QCore.LoopSwitchCallEvaluation.a_modeled_call_never_removes_the_loop_exit
#print axioms QCore.LoopSwitchCallEvaluation.a_modeled_call_keeps_the_loop_back_edge
#print axioms QCore.LoopSwitchCallEvaluation.continue_revisits_the_call
#print axioms QCore.LoopSwitchCallEvaluation.case_count_cannot_repeat_switch_evaluation
