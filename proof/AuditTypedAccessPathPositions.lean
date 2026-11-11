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

import QCore.TypedAccessPathPositions

/-! Audited abstract companion to W47-ACCESS-PATH-POSITIONS (gateway rule 10k). -/

#print axioms QCore.TypedAccessPathPositions.an_unlisted_position_refuses
#print axioms QCore.TypedAccessPathPositions.an_older_admission_is_never_replaced
#print axioms QCore.TypedAccessPathPositions.every_statement_refusal_holds_in_every_position
#print axioms QCore.TypedAccessPathPositions.a_union_path_refuses_in_every_position
#print axioms QCore.TypedAccessPathPositions.a_call_in_the_path_refuses_in_every_position
#print axioms QCore.TypedAccessPathPositions.a_cast_in_the_path_refuses_in_every_position
#print axioms QCore.TypedAccessPathPositions.a_connective_inside_one_read_refuses
#print axioms QCore.TypedAccessPathPositions.a_custody_position_refuses
#print axioms QCore.TypedAccessPathPositions.an_uncontracted_callee_refuses
#print axioms QCore.TypedAccessPathPositions.an_actual_beyond_the_declaration_refuses
#print axioms QCore.TypedAccessPathPositions.a_returned_pointer_from_a_tracked_root_refuses
#print axioms QCore.TypedAccessPathPositions.a_pointer_actual_from_a_tracked_root_refuses
#print axioms QCore.TypedAccessPathPositions.an_assignment_into_a_non_local_refuses
#print axioms QCore.TypedAccessPathPositions.an_unevaluated_and_operand_contributes_nothing
#print axioms QCore.TypedAccessPathPositions.an_unevaluated_or_operand_contributes_nothing
#print axioms QCore.TypedAccessPathPositions.an_unselected_arm_contributes_nothing
#print axioms QCore.TypedAccessPathPositions.an_and_guard_discharges_every_checked_hop
#print axioms QCore.TypedAccessPathPositions.an_or_guard_discharges_no_checked_hop
#print axioms QCore.TypedAccessPathPositions.the_or_hop_is_reached_where_the_guard_fails
#print axioms QCore.TypedAccessPathPositions.the_and_hop_is_reached_where_the_guard_holds
#print axioms QCore.TypedAccessPathPositions.an_operand_reads_before_either_edge
#print axioms QCore.TypedAccessPathPositions.a_read_position_preserves_the_frame
#print axioms QCore.TypedAccessPathPositions.a_typed_return_hands_nothing_back
#print axioms QCore.TypedAccessPathPositions.a_refused_return_stays_refused
#print axioms QCore.TypedAccessPathPositions.the_callee_contract_applies_to_the_callers_frame
#print axioms QCore.TypedAccessPathPositions.a_refused_actual_keeps_the_refusal
#print axioms QCore.TypedAccessPathPositions.a_typedef_spelling_alone_is_never_scalar
#print axioms QCore.TypedAccessPathPositions.an_aggregate_is_never_scalar
#print axioms QCore.TypedAccessPathPositions.a_union_is_never_scalar
#print axioms QCore.TypedAccessPathPositions.without_the_arm_an_enum_refuses
#print axioms QCore.TypedAccessPathPositions.the_enum_arm_only_adds_enumerations
#print axioms QCore.TypedAccessPathPositions.a_borrowed_scalar_actual_is_admitted
#print axioms QCore.TypedAccessPathPositions.the_same_actual_at_a_custody_position_is_refused
