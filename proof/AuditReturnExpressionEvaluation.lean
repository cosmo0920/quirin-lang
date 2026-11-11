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

import QCore.ReturnExpressionEvaluation

-- Card B01-RETURN-EXPRESSION. Rust model:
-- `Parser::return_expression_instruction` in
-- `crates/qcore-c-gateway/src/legacy_c.rs`; behavioural pins:
-- `crates/qcore-c-gateway/tests/return_position_modeled_call.rs`.

#print axioms QCore.ReturnExpressionEvaluation.an_admitted_call_is_unconditional
#print axioms QCore.ReturnExpressionEvaluation.an_admitted_call_has_no_synthetic_collision
#print axioms QCore.ReturnExpressionEvaluation.a_conditional_call_stays_refused
#print axioms QCore.ReturnExpressionEvaluation.a_collision_stays_refused
#print axioms QCore.ReturnExpressionEvaluation.a_local_summary_stays_refused
#print axioms QCore.ReturnExpressionEvaluation.an_admitted_return_is_exactly_the_call_effect
#print axioms QCore.ReturnExpressionEvaluation.a_preserved_obligation_remains_visible
#print axioms QCore.ReturnExpressionEvaluation.the_positive_family_is_admitted
#print axioms QCore.ReturnExpressionEvaluation.a_declared_release_is_applied
#print axioms QCore.ReturnExpressionEvaluation.an_admitted_opaque_read_changes_no_custody
#print axioms QCore.ReturnExpressionEvaluation.an_unvalidated_alias_read_stays_refused
#print axioms QCore.ReturnExpressionEvaluation.a_tracked_base_stays_refused
#print axioms QCore.ReturnExpressionEvaluation.a_conditional_opaque_result_stays_refused
