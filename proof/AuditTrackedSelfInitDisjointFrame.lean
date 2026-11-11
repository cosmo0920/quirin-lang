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

import QCore.TrackedSelfInitDisjointFrame

#print axioms QCore.TrackedSelfInitDisjointFrame.protected_slot_shares_no_write_byte
#print axioms QCore.TrackedSelfInitDisjointFrame.an_enclosing_member_protects_against_each_subwrite
#print axioms QCore.TrackedSelfInitDisjointFrame.a_separate_write_keeps_each_slot_byte
#print axioms QCore.TrackedSelfInitDisjointFrame.finite_separate_writes_keep_each_slot_byte
#print axioms QCore.TrackedSelfInitDisjointFrame.the_complete_ledger_is_preserved
#print axioms QCore.TrackedSelfInitDisjointFrame.all_field_links_are_preserved
#print axioms QCore.TrackedSelfInitDisjointFrame.all_partial_links_are_preserved
#print axioms QCore.TrackedSelfInitDisjointFrame.all_nullable_facts_are_preserved
#print axioms QCore.TrackedSelfInitDisjointFrame.plain_and_derived_bindings_survive
#print axioms QCore.TrackedSelfInitDisjointFrame.surviving_member_facts_have_slot_protection
#print axioms QCore.TrackedSelfInitDisjointFrame.accepted_links_have_the_actual_owner
#print axioms QCore.TrackedSelfInitDisjointFrame.accepted_partial_links_are_still_real_links
#print axioms QCore.TrackedSelfInitDisjointFrame.unsupported_custody_is_never_erased_to_admit
#print axioms QCore.TrackedSelfInitDisjointFrame.missing_source_calls_refuse
#print axioms QCore.TrackedSelfInitDisjointFrame.pointer_rebinding_refuses
#print axioms QCore.TrackedSelfInitDisjointFrame.missing_layout_refuses
#print axioms QCore.TrackedSelfInitDisjointFrame.refusal_is_unknown
#print axioms QCore.TrackedSelfInitDisjointFrame.finite_composition_preserves_every_obligation
#print axioms QCore.TrackedSelfInitDisjointFrame.no_ordinary_summary
#print axioms QCore.TrackedSelfInitDisjointFrame.no_callback_permission
#print axioms QCore.TrackedSelfInitDisjointFrame.no_cross_unit_export
#print axioms QCore.TrackedSelfInitDisjointFrame.shared_flow_vectors
#print axioms QCore.TrackedSelfInitDisjointFrame.shared_layout_vectors
