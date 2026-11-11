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

import QCore.DescriptorCells

/-! Audited companion to MK-EVENT-DESCRIPTOR-QUANT (gateway rule 38, contract specification C04.6.2; user decision 2026-11-01, "I approve the changes of D1-D5."): a descriptor is an identity and a cell a name of it; a fresh identity is released once, a second release through any name refuses; the witness decides a creation's class and an untested one never discharges; an untracked cell decides nothing and a live or undecided cell refuses at an exit; the member-equality guard is decided only from live identities (distinct numbers under the hypothesis that two open descriptors never share one), an undecided guard refuses and changes no cell, and a guarded removal then keeps and poisons its record; removal and release commute; the shorthand and its canonical parts are one set; the Rust decision vectors hold. Card MK-EVENT-FIRST-MEMBER-CELL (C11 6.7.2.1p15): a pointer to a record whose first member is the event resolves to the event's own place, so the cell table's results are unchanged under it; every other record, a nested chain, an empty or incomplete record, a mix of types and any other type refuse; the Rust initial-member vectors hold. -/

#print axioms QCore.DescriptorCells.a_fresh_identity_is_owed_once
#print axioms QCore.DescriptorCells.a_stale_cell_never_releases_again
#print axioms QCore.DescriptorCells.the_same_alias_releases_once
#print axioms QCore.DescriptorCells.the_witness_decides_the_cell
#print axioms QCore.DescriptorCells.an_untested_witness_never_discharges
#print axioms QCore.DescriptorCells.an_untracked_cell_decides_nothing
#print axioms QCore.DescriptorCells.an_escaping_cell_refuses
#print axioms QCore.DescriptorCells.equal_identities_decide_eq
#print axioms QCore.DescriptorCells.distinct_live_descriptors_decide_ne
#print axioms QCore.DescriptorCells.an_undecided_guard_gives_the_join
#print axioms QCore.DescriptorCells.the_join_never_releases
#print axioms QCore.DescriptorCells.an_undecided_guarded_removal_keeps_and_poisons
#print axioms QCore.DescriptorCells.removal_and_cell_release_commute
#print axioms QCore.DescriptorCells.each_shorthand_and_its_parts_mean_the_same
#print axioms QCore.DescriptorCells.both_levels_are_one_canonical_set
#print axioms QCore.DescriptorCells.decision_vectors_hold
#print axioms QCore.DescriptorCells.the_initial_member_is_the_event_cell
#print axioms QCore.DescriptorCells.the_cell_table_is_unchanged_under_the_initial_member
#print axioms QCore.DescriptorCells.every_other_record_refuses
#print axioms QCore.DescriptorCells.a_nested_first_member_refuses
#print axioms QCore.DescriptorCells.a_non_event_first_member_refuses
#print axioms QCore.DescriptorCells.an_empty_or_incomplete_record_refuses
#print axioms QCore.DescriptorCells.an_undecided_or_other_type_refuses
#print axioms QCore.DescriptorCells.a_mixed_call_set_has_no_type
#print axioms QCore.DescriptorCells.a_refused_view_creates_nothing
#print axioms QCore.DescriptorCells.the_initial_member_is_at_the_record_address
#print axioms QCore.DescriptorCells.initial_member_vectors_hold
