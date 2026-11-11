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

import QCore.ConditionAcquisition

/-! Audited companion to CG04-CONDITION-ACQUISITION (gateway rule 10x). -/

#print axioms QCore.ConditionAcquisition.use_vectors_hold
#print axioms QCore.ConditionAcquisition.the_split_use_is_accepted
#print axioms QCore.ConditionAcquisition.an_unsplit_custody_result_keeps_its_refusal
#print axioms QCore.ConditionAcquisition.a_recursive_callee_stays_refused
#print axioms QCore.ConditionAcquisition.carrier_vectors_hold
#print axioms QCore.ConditionAcquisition.a_custody_split_tests_its_own_carrier
#print axioms QCore.ConditionAcquisition.exactly_one_edge_carries_the_obligation
#print axioms QCore.ConditionAcquisition.the_ok_edge_carries_the_obligation
#print axioms QCore.ConditionAcquisition.the_failed_edge_carries_nothing
#print axioms QCore.ConditionAcquisition.every_outcome_reaches_one_edge
#print axioms QCore.ConditionAcquisition.the_obligation_follows_the_outcome
#print axioms QCore.ConditionAcquisition.placement_vectors_hold
#print axioms QCore.ConditionAcquisition.with_generations_every_split_is_placed
#print axioms QCore.ConditionAcquisition.off_every_cycle_a_site_has_one_generation
#print axioms QCore.ConditionAcquisition.a_merged_site_would_hide_a_live_generation
#print axioms QCore.ConditionAcquisition.on_the_failed_edge_the_old_identity_is_live
#print axioms QCore.ConditionAcquisition.on_the_ok_edge_only_the_new_identity_is_live
#print axioms QCore.ConditionAcquisition.exactly_one_identity_is_live
#print axioms QCore.ConditionAcquisition.evaluated_calls_are_held
#print axioms QCore.ConditionAcquisition.the_acquisition_is_applied_as_c_evaluates_it
#print axioms QCore.ConditionAcquisition.an_acquisition_leaf_applies_its_call_once
#print axioms QCore.ConditionAcquisition.an_unevaluated_operand_acquires_nothing
#print axioms QCore.ConditionAcquisition.the_right_operand_acquires_only_when_the_left_holds
#print axioms QCore.ConditionAcquisition.the_short_circuited_acquisition_is_not_applied
#print axioms QCore.ConditionAcquisition.leaf_vectors_hold
#print axioms QCore.ConditionAcquisition.only_listed_leaves_are_admitted
#print axioms QCore.ConditionAcquisition.a_call_in_a_type_operand_is_refused
#print axioms QCore.ConditionAcquisition.an_increment_beside_a_call_is_refused
#print axioms QCore.ConditionAcquisition.a_type_operand_applies_no_call
#print axioms QCore.ConditionAcquisition.an_admitted_leaf_applies_every_call
#print axioms QCore.ConditionAcquisition.a_call_in_a_type_operand_is_held_but_not_evaluated
