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

import QCore.TypedPlace

/-! Audited companion to CG10-COMMON-TYPED-PLACE (gateway rule 10t). -/

#print axioms QCore.TypedPlace.every_unlisted_form_refuses
#print axioms QCore.TypedPlace.a_listed_place_reads_its_steps
#print axioms QCore.TypedPlace.decision_vectors_hold
#print axioms QCore.TypedPlace.a_record_or_array_root_is_refused
#print axioms QCore.TypedPlace.a_deeper_place_is_refused
#print axioms QCore.TypedPlace.a_refused_decision_has_no_obligation
#print axioms QCore.TypedPlace.an_unbound_root_is_refused
#print axioms QCore.TypedPlace.a_target_free_root_is_refused
#print axioms QCore.TypedPlace.a_root_that_is_not_live_is_refused
#print axioms QCore.TypedPlace.a_nullable_root_is_refused
#print axioms QCore.TypedPlace.a_live_root_discharges_a_root_only_place
#print axioms QCore.TypedPlace.a_live_linked_member_discharges_a_direct_member_place
#print axioms QCore.TypedPlace.a_tracked_second_link_is_refused
#print axioms QCore.TypedPlace.every_w46_admission_stands
#print axioms QCore.TypedPlace.the_untracked_case_is_w46s
#print axioms QCore.TypedPlace.a_pointer_store_stays_refused
#print axioms QCore.TypedPlace.a_refused_tracked_decision_keeps_the_refusal
#print axioms QCore.TypedPlace.any_unlisted_form_stays_refused
#print axioms QCore.TypedPlace.every_hop_discharged_defines_the_access
#print axioms QCore.TypedPlace.a_refused_hop_is_never_safe
#print axioms QCore.TypedPlace.a_nonpointer_access_preserves_the_frame
#print axioms QCore.TypedPlace.a_pointer_read_is_a_borrow_without_custody
#print axioms QCore.TypedPlace.a_borrowed_read_with_a_frame_refuses
#print axioms QCore.TypedPlace.the_m01_control_is_admitted
#print axioms QCore.TypedPlace.the_route_refuses_an_unbound_formal
