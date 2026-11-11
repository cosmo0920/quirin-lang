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

import QCore.IndirectChainAccess

/-! Audited Lean companion to card W44-INDIRECT-CHAIN-ACCESS. -/

#print axioms QCore.IndirectChainAccess.the_root_premise_is_left_to_the_flow
#print axioms QCore.IndirectChainAccess.a_never_bound_scalar_chain_is_an_o04_chain
#print axioms QCore.IndirectChainAccess.a_bad_link_refuses
#print axioms QCore.IndirectChainAccess.a_non_continuable_intermediate_refuses
#print axioms QCore.IndirectChainAccess.a_single_selection_is_not_this_rule
#print axioms QCore.IndirectChainAccess.a_chain_beyond_the_bound_refuses
#print axioms QCore.IndirectChainAccess.lastIsScalar_of_pointer_last
#print axioms QCore.IndirectChainAccess.a_pointer_final_member_refuses
#print axioms QCore.IndirectChainAccess.a_continued_access_refuses
#print axioms QCore.IndirectChainAccess.a_chain_changes_no_binding
#print axioms QCore.IndirectChainAccess.field_links_are_unchanged
#print axioms QCore.IndirectChainAccess.the_statuses_and_markers_are_unchanged
#print axioms QCore.IndirectChainAccess.with_no_chain_the_access_is_w43s
#print axioms QCore.IndirectChainAccess.admitted_chains_add_no_diagnostic
#print axioms QCore.IndirectChainAccess.a_refused_chain_is_never_safe
#print axioms QCore.IndirectChainAccess.an_unbound_never_bound_root_is_untracked
#print axioms QCore.IndirectChainAccess.an_unbound_root_the_body_may_bind_refuses
#print axioms QCore.IndirectChainAccess.a_nullable_root_refuses
#print axioms QCore.IndirectChainAccess.a_target_free_root_the_body_may_bind_refuses
#print axioms QCore.IndirectChainAccess.a_record_object_root_the_body_may_bind_refuses
#print axioms QCore.IndirectChainAccess.a_tracked_root_is_one_live_identity
#print axioms QCore.IndirectChainAccess.a_tracked_root_is_never_accused
#print axioms QCore.IndirectChainAccess.an_untracked_owner_walks_untracked
#print axioms QCore.IndirectChainAccess.every_link_of_an_untracked_root_resolves
#print axioms QCore.IndirectChainAccess.a_tracked_owner_past_the_first_link_refuses
#print axioms QCore.IndirectChainAccess.a_tracked_owner_needs_a_bound_place
#print axioms QCore.IndirectChainAccess.a_nullable_place_refuses
#print axioms QCore.IndirectChainAccess.a_tracked_link_is_the_stored_live_child
#print axioms QCore.IndirectChainAccess.a_drifted_link_refuses
#print axioms QCore.IndirectChainAccess.a_released_child_refuses
#print axioms QCore.IndirectChainAccess.an_unbound_formal_chain_of_depth_four_is_admitted
#print axioms QCore.IndirectChainAccess.a_linked_live_child_is_admitted
#print axioms QCore.IndirectChainAccess.a_linked_released_child_is_refused
#print axioms QCore.IndirectChainAccess.a_tracked_owner_one_link_deeper_is_refused
#print axioms QCore.IndirectChainAccess.an_unbound_member_place_of_a_tracked_root_is_refused
#print axioms QCore.IndirectChainAccess.a_full_scalar_chain_is_admitted
