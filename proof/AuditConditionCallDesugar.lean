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

import QCore.ConditionCallDesugar

/-! Audited Lean companion to card W44-BRANCH-CONDITION-CALLS.

The model states the desugaring of a call-bearing controlling expression into
calls and branch edges, its exact agreement with C's short-circuit evaluation
for every assignment of leaf values, the loop re-evaluation, and the admission
conjunction with each refusing premise. Faithful C token/type extraction and
correspondence between this rule and the compiled Rust are explicit review
premises, per standing constraint 28; the shipped Rust fixtures exercise that
correspondence on the bounded shapes.
-/

#print axioms QCore.ConditionCallDesugar.run_calls
#print axioms QCore.ConditionCallDesugar.run_assemble
#print axioms QCore.ConditionCallDesugar.the_desugaring_is_c_evaluation
#print axioms QCore.ConditionCallDesugar.and_evaluates_its_right_operand_only_when_the_left_holds
#print axioms QCore.ConditionCallDesugar.or_evaluates_its_right_operand_only_when_the_left_fails
#print axioms QCore.ConditionCallDesugar.choose_evaluates_exactly_the_selected_arm
#print axioms QCore.ConditionCallDesugar.not_swaps_only_the_exit
#print axioms QCore.ConditionCallDesugar.a_skipped_operand_applies_nothing
#print axioms QCore.ConditionCallDesugar.the_short_circuited_release_is_not_applied
#print axioms QCore.ConditionCallDesugar.the_evaluated_release_is_applied
#print axioms QCore.ConditionCallDesugar.the_loop_exit_is_the_false_value
#print axioms QCore.ConditionCallDesugar.every_visit_re_evaluates_the_condition
#print axioms QCore.ConditionCallDesugar.an_admitted_condition_has_an_admitted_shape
#print axioms QCore.ConditionCallDesugar.every_leaf_of_an_admitted_condition_is_ok
#print axioms QCore.ConditionCallDesugar.a_refused_shape_refuses
#print axioms QCore.ConditionCallDesugar.one_refused_leaf_refuses_the_condition
#print axioms QCore.ConditionCallDesugar.an_unmodeled_call_refuses
#print axioms QCore.ConditionCallDesugar.a_custody_result_refuses
#print axioms QCore.ConditionCallDesugar.a_callee_in_the_own_component_refuses
#print axioms QCore.ConditionCallDesugar.unsequenced_effects_refuse
#print axioms QCore.ConditionCallDesugar.a_non_neutral_residual_refuses
#print axioms QCore.ConditionCallDesugar.a_descriptor_read_as_data_refuses
#print axioms QCore.ConditionCallDesugar.a_leaf_with_every_premise_is_ok
#print axioms QCore.ConditionCallDesugar.identity_calls_commute
#print axioms QCore.ConditionCallDesugar.a_neutral_decision_keeps_both_edges
