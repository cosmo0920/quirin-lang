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

import QCore.PointerStore
import QCore.CheckedSelfInitCall

#print axioms QCore.CheckedSelfInitCall.a_missing_source_obligation_refuses
#print axioms QCore.CheckedSelfInitCall.admitted_call_preserves_every_field
#print axioms QCore.CheckedSelfInitCall.no_transfer_discharges_an_obligation
#print axioms QCore.CheckedSelfInitCall.default_entry_refuses
#print axioms QCore.CheckedSelfInitCall.a_tracked_frame_refuses
#print axioms QCore.CheckedSelfInitCall.refusal_remains_unknown
#print axioms QCore.CheckedSelfInitCall.finite_checked_calls_preserve_the_frame
#print axioms QCore.CheckedSelfInitCall.no_ordinary_summary
#print axioms QCore.CheckedSelfInitCall.no_callback_transport
#print axioms QCore.CheckedSelfInitCall.no_cross_unit_transport
#print axioms QCore.CheckedSelfInitCall.no_recursive_assumption
#print axioms QCore.CheckedSelfInitCall.self_stores_preserve_reference_lifetimes
#print axioms QCore.CheckedSelfInitCall.shared_flow_vectors
#print axioms QCore.CheckedSelfInitCall.shared_source_vectors

/-! Audited companion to CG11-POINTER-VALUED-STORES (gateway rule 10v). -/

#print axioms QCore.PointerStore.the_final_owner_decides_resolution
#print axioms QCore.PointerStore.decision_vectors_hold
#print axioms QCore.PointerStore.admitted_exactly_when
#print axioms QCore.PointerStore.example_c_is_refused
#print axioms QCore.PointerStore.an_owned_field_is_never_overwritten
#print axioms QCore.PointerStore.an_unresolved_owner_refuses
#print axioms QCore.PointerStore.another_root_is_another_object
#print axioms QCore.PointerStore.a_deeper_value_place_is_another_object
#print axioms QCore.PointerStore.a_direct_member_place_is_not_the_root_object
#print axioms QCore.PointerStore.frame_preserved
#print axioms QCore.PointerStore.no_obligation_lost_on_overwrite
#print axioms QCore.PointerStore.an_owning_field_overwrite_refuses
#print axioms QCore.PointerStore.a_tracked_old_value_refuses
#print axioms QCore.PointerStore.no_double_release_through_a_stored_alias
#print axioms QCore.PointerStore.the_member_release_shape_is_refused
#print axioms QCore.PointerStore.an_unlisted_destination_refuses
#print axioms QCore.PointerStore.a_never_bound_root_owns_untracked
#print axioms QCore.PointerStore.a_live_root_is_a_tracked_owner
