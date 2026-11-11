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

import QCore.LoopLifetimeSummary
#print axioms QCore.LoopLifetimeSummary.a_may_join_keeps_every_left_registration
#print axioms QCore.LoopLifetimeSummary.a_may_join_keeps_every_right_registration
#print axioms QCore.LoopLifetimeSummary.a_must_ending_stands_on_both_paths
#print axioms QCore.LoopLifetimeSummary.a_destroy_ends_every_registration_at_its_holder
#print axioms QCore.LoopLifetimeSummary.a_destroy_keeps_other_holders
#print axioms QCore.LoopLifetimeSummary.the_fold_only_demotes
#print axioms QCore.LoopLifetimeSummary.an_open_summary_is_never_admitted
#print axioms QCore.LoopLifetimeSummary.an_undischarged_precondition_is_never_admitted
#print axioms QCore.LoopLifetimeSummary.a_foreign_function_formal_never_substitutes
#print axioms QCore.LoopLifetimeSummary.a_known_actual_preserves_its_own_root
#print axioms QCore.LoopLifetimeSummary.an_unresolved_actual_without_a_footprint_never_substitutes
#print axioms QCore.LoopLifetimeSummary.a_closed_footprint_keeps_an_unnameable_registration_root
#print axioms QCore.LoopLifetimeSummary.the_ordered_poisoning_boundary

#print axioms QCore.LoopLifetimeSummary.one_lifted_holder_never_ends_a_whole_footprint
#print axioms QCore.LoopLifetimeSummary.formal_and_result_roots_remain_possible_aliases

#print axioms QCore.LoopLifetimeSummary.shared_destroy_vectors_hold

#print axioms QCore.LoopLifetimeSummary.a_release_without_an_earlier_ending_refuses_every_incoming_record
#print axioms QCore.LoopLifetimeSummary.a_destroy_before_release_discharges_its_exact_holder
#print axioms QCore.LoopLifetimeSummary.a_later_destroy_does_not_retroactively_discharge_a_release

#print axioms QCore.LoopLifetimeSummary.a_lifted_prefix_never_discharges_a_release

#print axioms QCore.LoopLifetimeSummary.a_join_forwarded_on_two_present_arms_has_both_proofs

#print axioms QCore.LoopLifetimeSummary.every_retained_record_remains_owed_or_explicitly_forwarded
#print axioms QCore.LoopLifetimeSummary.forwarding_does_not_invent_or_remove_its_retained_record
#print axioms QCore.LoopLifetimeSummary.an_unproved_record_is_still_an_exit_obligation

#print axioms QCore.LoopLifetimeSummary.a_new_registration_invalidates_its_earlier_destroy

#print axioms QCore.LoopLifetimeSummary.an_unproved_holder_frame_cannot_discharge_an_incoming_record
