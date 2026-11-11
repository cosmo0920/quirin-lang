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

import QCore.BoundUntracked

/-! Audited companion to CG10-BOUND-UNTRACKED-ROOTS (gateway rule 11d). -/

#print axioms QCore.BoundUntracked.decision_vectors_hold
#print axioms QCore.BoundUntracked.the_premise_holds_exactly_on_an_untracked_unrefused_root
#print axioms QCore.BoundUntracked.an_ineligible_root_never_holds
#print axioms QCore.BoundUntracked.a_bound_root_never_holds
#print axioms QCore.BoundUntracked.an_address_taken_root_is_refused
#print axioms QCore.BoundUntracked.an_unlisted_root_is_refused
#print axioms QCore.BoundUntracked.the_never_bound_answer_is_unchanged
#print axioms QCore.BoundUntracked.an_ineligible_root_is_read_as_before
#print axioms QCore.BoundUntracked.a_tracked_root_keeps_the_tracked_reading
#print axioms QCore.BoundUntracked.a_record_object_root_is_read_as_before
#print axioms QCore.BoundUntracked.the_never_bound_reading_only_adds
#print axioms QCore.BoundUntracked.every_old_admission_stands
#print axioms QCore.BoundUntracked.under_the_premise_an_unbound_root_resolves
#print axioms QCore.BoundUntracked.under_the_premise_a_target_free_root_resolves
#print axioms QCore.BoundUntracked.a_refused_path_keeps_the_refusal
#print axioms QCore.BoundUntracked.a_rooted_place_keeps_the_refusal
#print axioms QCore.BoundUntracked.a_nullable_root_stays_refused
#print axioms QCore.BoundUntracked.an_unbound_root_resolves_only_under_the_premise
#print axioms QCore.BoundUntracked.every_never_bound_actual_stands
#print axioms QCore.BoundUntracked.an_address_actual_keeps_the_never_bound_premise
#print axioms QCore.BoundUntracked.an_ineligible_actual_is_refused
#print axioms QCore.BoundUntracked.a_deferred_actual_never_reads_a_tracked_root
#print axioms QCore.BoundUntracked.a_deferred_actual_is_read_under_the_premise
#print axioms QCore.BoundUntracked.the_refusal_is_monotone
#print axioms QCore.BoundUntracked.a_refusal_is_kept_on_every_later_path
#print axioms QCore.BoundUntracked.a_refused_path_never_holds
#print axioms QCore.BoundUntracked.a_borrowed_actual_preserves_the_premise
#print axioms QCore.BoundUntracked.a_custody_actual_ends_the_premise
#print axioms QCore.BoundUntracked.an_uncontracted_call_ends_the_premise
#print axioms QCore.BoundUntracked.a_none_result_satisfies_the_premise
#print axioms QCore.BoundUntracked.a_derived_result_of_an_untracked_base_satisfies_the_premise
#print axioms QCore.BoundUntracked.an_owned_result_keeps_the_tracked_reading
#print axioms QCore.BoundUntracked.the_join_keeps_a_refusal
#print axioms QCore.BoundUntracked.a_one_sided_binding_is_refused_after_the_join
#print axioms QCore.BoundUntracked.event_vectors_hold
#print axioms QCore.BoundUntracked.an_unbound_bound_root_is_admitted_on_an_unrefused_path
#print axioms QCore.BoundUntracked.the_same_root_is_refused_after_a_refusal
#print axioms QCore.BoundUntracked.the_same_root_is_refused_when_ineligible
