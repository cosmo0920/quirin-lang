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

import QCore.ReturnExpressionFamilies

/-! Audited Lean companion to card W45-RETURN-EXPRESSION-FAMILIES: the
scalar-result read and the cast of a whole call in `return` position
(`crates/qcore-c-gateway/src/legacy_return_family.rs` and its hook in
`Parser::return_expression_instruction`). Abstract rule theorems; extraction,
the declaration readers and Rust correspondence remain constraint-28 premises. -/

#print axioms QCore.ReturnExpressionFamilies.an_earlier_admission_is_unchanged
#print axioms QCore.ReturnExpressionFamilies.the_rule_only_replaces_the_return_refusal
#print axioms QCore.ReturnExpressionFamilies.only_a_scalar_result_is_admitted
#print axioms QCore.ReturnExpressionFamilies.a_pointer_result_is_never_admitted
#print axioms QCore.ReturnExpressionFamilies.every_occurrence_of_an_admitted_read_is_admitted
#print axioms QCore.ReturnExpressionFamilies.an_admitted_read_mentions_no_descriptor
#print axioms QCore.ReturnExpressionFamilies.a_bound_name_is_an_unconditional_direct_root
#print axioms QCore.ReturnExpressionFamilies.every_bound_dereference_is_a_base
#print axioms QCore.ReturnExpressionFamilies.an_accusation_is_on_an_evaluated_dereference
#print axioms QCore.ReturnExpressionFamilies.a_scalar_read_returns_nothing
#print axioms QCore.ReturnExpressionFamilies.the_read_withdraws_no_leak
#print axioms QCore.ReturnExpressionFamilies.an_admitted_cast_call_rests_on_its_admission
#print axioms QCore.ReturnExpressionFamilies.a_refused_call_is_lowered_to_its_own_refusal
#print axioms QCore.ReturnExpressionFamilies.parentheses_alone_are_w4
#print axioms QCore.ReturnExpressionFamilies.targets_nonempty
#print axioms QCore.ReturnExpressionFamilies.an_owned_result_under_casts_is_a_pointer_conversion
#print axioms QCore.ReturnExpressionFamilies.an_integer_cast_of_an_owned_result_is_refused
#print axioms QCore.ReturnExpressionFamilies.a_reallocated_or_derived_result_under_a_cast_is_refused
#print axioms QCore.ReturnExpressionFamilies.an_admitted_cast_call_discharges_nothing_prior
#print axioms QCore.ReturnExpressionFamilies.the_evaluation_precedes_the_exit
#print axioms QCore.ReturnExpressionFamilies.a_declined_or_lowered_probe_restores_the_counter
#print axioms QCore.ReturnExpressionFamilies.the_scalar_read_is_not_vacuous
#print axioms QCore.ReturnExpressionFamilies.the_cast_call_is_not_vacuous
