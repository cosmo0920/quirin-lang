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

import QCore.VoidViewReadback

/-! Audited companion to CG12-VOID-VIEW-READBACK (gateway rule 11j). -/

-- The view: shared vectors, exactly one admitted form, every other refused.
#print axioms QCore.VoidViewReadback.view_vectors_hold
#print axioms QCore.VoidViewReadback.view_vectors_count
#print axioms QCore.VoidViewReadback.a_view_is_exactly_a_bit_cast_from_void_to_an_object_pointer
#print axioms QCore.VoidViewReadback.a_cast_between_object_pointers_is_never_a_view
#print axioms QCore.VoidViewReadback.a_non_object_target_is_never_a_view
#print axioms QCore.VoidViewReadback.position_vectors_hold
#print axioms QCore.VoidViewReadback.a_view_over_a_link_is_refused
#print axioms QCore.VoidViewReadback.a_view_over_a_tracked_identity_is_refused
#print axioms QCore.VoidViewReadback.a_view_over_an_untracked_root_adds_no_premise

-- The readback's static premises: a type-mismatched view is refused.
#print axioms QCore.VoidViewReadback.decision_vectors_hold
#print axioms QCore.VoidViewReadback.decision_vectors_count
#print axioms QCore.VoidViewReadback.identity_exactly_when
#print axioms QCore.VoidViewReadback.a_type_mismatched_view_is_refused
#print axioms QCore.VoidViewReadback.an_escaping_owner_gives_no_identity
#print axioms QCore.VoidViewReadback.an_escaping_member_gives_no_identity
#print axioms QCore.VoidViewReadback.an_unlisted_owner_keeps_the_old_reading
#print axioms QCore.VoidViewReadback.the_flow_binds_only_an_admitted_resolved_readback

-- The flow's event model: the stored identity, or none.
#print axioms QCore.VoidViewReadback.event_vectors_hold
#print axioms QCore.VoidViewReadback.event_vector_outcomes
#print axioms QCore.VoidViewReadback.run_append
#print axioms QCore.VoidViewReadback.a_readback_returns_the_stored_identity
#print axioms QCore.VoidViewReadback.an_intervening_unknown_store_yields_no_identity
#print axioms QCore.VoidViewReadback.a_partial_link_yields_no_identity
#print axioms QCore.VoidViewReadback.a_released_child_is_never_read_back
#print axioms QCore.VoidViewReadback.the_readback_is_the_last_store
#print axioms QCore.VoidViewReadback.lastStore_is_stored
#print axioms QCore.VoidViewReadback.a_readback_returns_only_a_stored_identity
#print axioms QCore.VoidViewReadback.the_flagship_readback_yields_its_identity
#print axioms QCore.VoidViewReadback.one_poison_flips_the_flagship
