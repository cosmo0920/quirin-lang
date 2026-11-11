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

import QCore.OpCallbacks

/-! Audited companion to CG22-OP-CALLBACKS-AND-SERVED-QUEUES: op callbacks and served queues (gateway rules 12m-12o): a supplied object handed to the callback is taken by every admitted candidate, a type-switched release is the cautious join over its arms and an undeclared arm is never neutral, and a member's run-wide closed set decides only when the run's set is contained in the unit's. Card OP-CONSTRUCTOR-DECLARATIONS (rules 12o, 13w, 14e): a decided discriminant selects exactly its arm and an undecided or unstable one gives the cautious join; a constructor's registration followed by destroy invokes only the registered candidate, an op that maps to absent invokes none, and an unclassified store voids the state; `quant live #result: TAG.FIELD |-> #absent` and its two parts mean the same and are one canonical set. -/

#print axioms QCore.OpCallbacks.a_kept_supplied_object_is_refused
#print axioms QCore.OpCallbacks.an_admitted_decision_takes_or_is_identity
#print axioms QCore.OpCallbacks.a_taking_candidate_takes_the_object
#print axioms QCore.OpCallbacks.an_identity_candidate_receives_no_supplied_object
#print axioms QCore.OpCallbacks.an_admitted_hand_off_never_leaves_the_callee_holding
#print axioms QCore.OpCallbacks.supplied_classify_vectors_hold
#print axioms QCore.OpCallbacks.supplied_decision_vectors_hold
#print axioms QCore.OpCallbacks.an_undeclared_arm_is_unknown
#print axioms QCore.OpCallbacks.the_cautious_join_is_sound
#print axioms QCore.OpCallbacks.a_call_without_otherwise_is_refused
#print axioms QCore.OpCallbacks.an_undecided_link_is_refused
#print axioms QCore.OpCallbacks.arm_join_vectors_hold
#print axioms QCore.OpCallbacks.an_open_member_never_decides
#print axioms QCore.OpCallbacks.an_open_run_demotes
#print axioms QCore.OpCallbacks.a_kept_decision_covers_every_run_candidate
#print axioms QCore.OpCallbacks.an_unread_unit_keeps_the_member_open
#print axioms QCore.OpCallbacks.an_unread_unit_demotes
#print axioms QCore.OpCallbacks.a_read_unit_without_a_store_adds_nothing
#print axioms QCore.OpCallbacks.the_run_set_is_exactly_the_read_sets
#print axioms QCore.OpCallbacks.run_vectors_hold
#print axioms QCore.OpCallbacks.a_decided_discriminant_selects_exactly_its_arm
#print axioms QCore.OpCallbacks.a_decided_value_with_no_arm_is_unknown
#print axioms QCore.OpCallbacks.a_rewrite_within_one_arm_keeps_it
#print axioms QCore.OpCallbacks.the_decided_join_refines_the_cautious_join
#print axioms QCore.OpCallbacks.an_undecided_discriminant_gives_the_join
#print axioms QCore.OpCallbacks.an_unstable_discriminant_gives_the_join
#print axioms QCore.OpCallbacks.a_registration_then_destroy_invokes_only_the_registered_candidate
#print axioms QCore.OpCallbacks.an_absent_member_invokes_no_candidate
#print axioms QCore.OpCallbacks.an_unclassified_store_voids_the_absent_state
#print axioms QCore.OpCallbacks.the_shorthand_and_its_parts_mean_the_same
#print axioms QCore.OpCallbacks.the_shorthand_is_live_and_absent
#print axioms QCore.OpCallbacks.the_shorthand_entails_quant_live
#print axioms QCore.OpCallbacks.both_levels_are_one_canonical_set
#print axioms QCore.OpCallbacks.decided_join_vectors_hold
