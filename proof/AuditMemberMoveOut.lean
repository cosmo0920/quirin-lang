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

import QCore.MemberMoveOut

/-! Audited companion to OUT-SLOT-MEMBER-THEN-FRESH (gateway rule 96, rule
74l's mode (b), contract specification C04.19.4's owned member). -/

#print axioms QCore.MemberMoveOut.the_nonnull_reading_is_unchanged
#print axioms QCore.MemberMoveOut.a_nullable_result_needs_an_owned_member
#print axioms QCore.MemberMoveOut.liveness_still_needs_a_nonnull_result
#print axioms QCore.MemberMoveOut.no_other_result_carries_a_state
#print axioms QCore.MemberMoveOut.an_undecided_discriminant_states_no_owned_member
#print axioms QCore.MemberMoveOut.another_value_states_no_owned_member
#print axioms QCore.MemberMoveOut.move_out_vectors_hold
#print axioms QCore.MemberMoveOut.a_fresh_object_only_from_a_stated_member
#print axioms QCore.MemberMoveOut.an_alias_only_of_a_link_held_alone
#print axioms QCore.MemberMoveOut.a_partial_link_reads_the_fallback
#print axioms QCore.MemberMoveOut.a_second_move_out_takes_nothing
#print axioms QCore.MemberMoveOut.one_holder_after_the_move
#print axioms QCore.MemberMoveOut.an_uncleared_member_is_held_twice
#print axioms QCore.MemberMoveOut.slot_mode_vectors_hold
#print axioms QCore.MemberMoveOut.a_null_run_refusal_publishes_nothing
#print axioms QCore.MemberMoveOut.a_null_only_mode_delivers_only_into_a_known_null_slot
#print axioms QCore.MemberMoveOut.a_held_object_is_refused
#print axioms QCore.MemberMoveOut.the_null_only_reading_is_the_effect
#print axioms QCore.MemberMoveOut.an_unseeded_partial_store_reads_nothing
