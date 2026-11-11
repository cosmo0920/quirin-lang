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

import QCore.RetainingCallMintedName

/-! Audited Lean companion to card RETAINING-CALL-MINTED-NAME-ACTUALS (gateway rule 38b). -/

#print axioms QCore.RetainingCallMintedName.rule_38a_is_the_cell_bearing_case
#print axioms QCore.RetainingCallMintedName.every_withheld_borrowed_read_is_minted
#print axioms QCore.RetainingCallMintedName.a_function_map_callee_is_never_minted
#print axioms QCore.RetainingCallMintedName.an_uncontracted_callee_is_never_minted
#print axioms QCore.RetainingCallMintedName.a_custody_position_is_never_minted_without_cells
#print axioms QCore.RetainingCallMintedName.a_data_tail_is_never_minted_without_cells
#print axioms QCore.RetainingCallMintedName.a_release_is_never_minted_without_cells
#print axioms QCore.RetainingCallMintedName.off_a_withheld_declaration_the_flag_is_rule_38a_s
#print axioms QCore.RetainingCallMintedName.a_minted_entry_matches_its_minted_name_as_its_plain_name
#print axioms QCore.RetainingCallMintedName.the_minted_name_matches_only_its_own_callee
#print axioms QCore.RetainingCallMintedName.an_unminted_entry_never_matches_a_minted_name
#print axioms QCore.RetainingCallMintedName.the_plain_name_is_read_as_before
#print axioms QCore.RetainingCallMintedName.a_minted_match_is_reported_as_minted
#print axioms QCore.RetainingCallMintedName.a_plain_match_is_never_reported_as_minted
#print axioms QCore.RetainingCallMintedName.a_minted_match_is_a_plain_match
#print axioms QCore.RetainingCallMintedName.the_minted_name_never_admits_more
#print axioms QCore.RetainingCallMintedName.the_minted_name_admits_exactly_the_plain_set
#print axioms QCore.RetainingCallMintedName.a_refused_actual_is_admitted_under_no_name
#print axioms QCore.RetainingCallMintedName.a_minted_actual_at_an_owned_position_keeps_the_refusal
#print axioms QCore.RetainingCallMintedName.plain_actuals_need_no_position_check
#print axioms QCore.RetainingCallMintedName.an_admitted_probe_reads_every_minted_actual_borrowed
#print axioms QCore.RetainingCallMintedName.minted_admission_vectors_hold
#print axioms QCore.RetainingCallMintedName.the_brief_mk_event_add_actual_is_admitted_under_the_minted_name
#print axioms QCore.RetainingCallMintedName.a_withheld_release_is_not_admitted_under_the_minted_name
#print axioms QCore.RetainingCallMintedName.the_release_is_still_admitted_under_its_plain_name
#print axioms QCore.RetainingCallMintedName.another_callee_s_minted_name_admits_nothing
