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

import QCore.BranchConditionCall

/-! Audited Lean companion to card B02-BRANCH-CONDITION.

The model states the evaluation-frequency boundary and the CFG ordering used by
the Rust implementation.  Faithful C token/type extraction and correspondence
between this rule and compiled Rust are explicit review premises, per standing
constraint 28; the shipped Rust fixtures and mutation harness exercise that
correspondence on the bounded shapes.
-/

#print axioms QCore.BranchConditionCall.an_admitted_candidate_has_exactly_one_evaluation
#print axioms QCore.BranchConditionCall.an_admitted_candidate_is_externally_declared
#print axioms QCore.BranchConditionCall.an_admitted_candidate_has_a_modeled_call
#print axioms QCore.BranchConditionCall.an_admitted_candidate_has_a_neutral_residual
#print axioms QCore.BranchConditionCall.short_circuit_evaluation_refuses
#print axioms QCore.BranchConditionCall.conditional_evaluation_refuses
#print axioms QCore.BranchConditionCall.a_type_operator_refuses
#print axioms QCore.BranchConditionCall.two_calls_refuse
#print axioms QCore.BranchConditionCall.a_locally_defined_call_refuses
#print axioms QCore.BranchConditionCall.an_unmodeled_call_refuses
#print axioms QCore.BranchConditionCall.a_non_neutral_residual_refuses
#print axioms
  QCore.BranchConditionCall.an_exactly_once_modeled_call_with_a_neutral_residual_is_admitted
#print axioms QCore.BranchConditionCall.both_unknown_result_arms_receive_the_post_call_state
#print axioms QCore.BranchConditionCall.the_transfer_occurs_before_either_successor
