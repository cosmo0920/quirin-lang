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

import QCore.CallInstantiation

/-! Audited companion to CG03-CALL-SUMMARY-INSTANTIATION (gateway rule 10u). -/

#print axioms QCore.CallInstantiation.acceptance_vectors_hold
#print axioms QCore.CallInstantiation.acceptance_is_position_independent
#print axioms QCore.CallInstantiation.the_return_operand_takes_what_the_condition_leaf_takes
#print axioms QCore.CallInstantiation.every_b01_admission_stands
#print axioms QCore.CallInstantiation.the_widening_is_a_validated_callee_or_a_derived_result
#print axioms QCore.CallInstantiation.a_recursive_callee_is_refused_in_every_use
#print axioms QCore.CallInstantiation.a_custody_result_is_refused_where_it_is_tested
#print axioms QCore.CallInstantiation.a_passed_value_is_not_a_reallocation
#print axioms QCore.CallInstantiation.every_b04_admission_stands
#print axioms QCore.CallInstantiation.instantiation_is_one_summary_and_one_substitution
#print axioms QCore.CallInstantiation.instantiation_is_deterministic
#print axioms QCore.CallInstantiation.an_admitted_substitution_refuses_no_formal
#print axioms QCore.CallInstantiation.one_refused_formal_rejects_the_call
#print axioms QCore.CallInstantiation.a_custody_formal_takes_only_an_identity
#print axioms QCore.CallInstantiation.an_unlisted_actual_is_refused
#print axioms QCore.CallInstantiation.an_identity_substitution_is_the_objects_value
#print axioms QCore.CallInstantiation.an_address_substitution_is_the_slot
#print axioms QCore.CallInstantiation.data_evaluates_no_pointer_object
#print axioms QCore.CallInstantiation.a_place_cg10_refuses_stays_refused
#print axioms QCore.CallInstantiation.a_place_is_only_observed
#print axioms QCore.CallInstantiation.applications_count_their_calls
#print axioms QCore.CallInstantiation.a_distinct_call_occurs_once
#print axioms QCore.CallInstantiation.each_returned_call_is_applied_once
#print axioms QCore.CallInstantiation.the_return_operand_escapes_nothing
#print axioms QCore.CallInstantiation.the_actual_events_precede_the_call
#print axioms QCore.CallInstantiation.an_effectful_actual_is_scalar_data
#print axioms QCore.CallInstantiation.substitution_vectors_hold
