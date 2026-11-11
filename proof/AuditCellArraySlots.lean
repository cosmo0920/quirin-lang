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

import QCore.CellArraySlots

/-! Audited Lean companion to card MK-EVENT-CELL-ARRAY-SLOTS (gateway rule 38a). -/

#print axioms QCore.CellArraySlots.an_admitted_element_is_in_bounds
#print axioms QCore.CellArraySlots.an_admitted_element_is_at_a_borrowed_position
#print axioms QCore.CellArraySlots.an_admitted_element_is_at_a_cell_bearing_withheld_declaration
#print axioms QCore.CellArraySlots.a_custody_position_never_admits
#print axioms QCore.CellArraySlots.a_data_tail_never_admits
#print axioms QCore.CellArraySlots.a_variable_index_never_admits
#print axioms QCore.CellArraySlots.a_negative_index_never_admits
#print axioms QCore.CellArraySlots.a_non_canonical_index_never_admits
#print axioms QCore.CellArraySlots.an_out_of_bounds_index_never_admits
#print axioms QCore.CellArraySlots.a_pointer_member_never_admits
#print axioms QCore.CellArraySlots.a_non_integer_element_never_admits
#print axioms QCore.CellArraySlots.a_declaration_without_cells_never_admits
#print axioms QCore.CellArraySlots.a_function_map_callee_never_admits
#print axioms QCore.CellArraySlots.an_untyped_base_never_admits
#print axioms QCore.CellArraySlots.an_admitted_element_reads_as_the_member_read
#print axioms QCore.CellArraySlots.an_admitted_element_is_a_candidate
#print axioms QCore.CellArraySlots.a_refused_element_is_never_a_candidate
#print axioms QCore.CellArraySlots.an_admitted_element_obligates_its_base
#print axioms QCore.CellArraySlots.a_released_base_of_an_admitted_element_refuses_and_is_accused
#print axioms QCore.CellArraySlots.a_statement_of_one_refused_element_keeps_the_refusal
#print axioms QCore.CellArraySlots.a_minted_name_matches_only_its_own_callee
#print axioms QCore.CellArraySlots.without_cells_the_minted_name_never_matches
#print axioms QCore.CellArraySlots.the_plain_name_is_unchanged
#print axioms QCore.CellArraySlots.the_switch_off_refuses_every_cell_call
#print axioms QCore.CellArraySlots.a_refused_element_never_reaches_rule_38
#print axioms QCore.CellArraySlots.an_admitted_element_reaches_rule_38
#print axioms QCore.CellArraySlots.the_switch_never_widens
#print axioms QCore.CellArraySlots.the_brief_slot_is_admitted
#print axioms QCore.CellArraySlots.the_second_slot_is_admitted
#print axioms QCore.CellArraySlots.one_past_the_end_is_refused
#print axioms QCore.CellArraySlots.at_mk_event_add_it_is_refused
#print axioms QCore.CellArraySlots.the_brief_creation_lowers_to_the_reads_then_the_call
#print axioms QCore.CellArraySlots.the_brief_creation_adds_no_diagnostic_over_a_live_base
