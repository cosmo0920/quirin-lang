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

import QCore.RetentionPath

/-! Audited companion to FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (gateway rule 12t, contract specification C04.14; USER DECISIONS 2026-10-16, "Tracked holder path" and "R3: quant state") and RETENTION-UNMATCHED-REMOVAL-POISONS (USER DECISION 2026-10-17, "Yes, small fix card now"): a record held by a resolved holder path ends only by a removal of exactly that registration or its loop's destroy, so a holder that would outlive the held object is never `Safe`; an unresolved path never decides; a removal that does not match a record exactly poisons every record it may name, and a poisoned record ends only by its loop's destroy (the old sequence "unmatched removal, matched removal, release" no longer decides); a release while any record (an interior included) holds the identity refuses. -/

#print axioms QCore.RetentionPath.run_append
#print axioms QCore.RetentionPath.an_unresolved_path_never_decides
#print axioms QCore.RetentionPath.a_removal_drops_exactly_its_unpoisoned_record
#print axioms QCore.RetentionPath.an_unmatched_removal_poisons_what_it_may_name
#print axioms QCore.RetentionPath.poisonIf_object
#print axioms QCore.RetentionPath.poisonIf_of_poisoned
#print axioms QCore.RetentionPath.a_matched_removal_ends_an_unpoisoned_record
#print axioms QCore.RetentionPath.step_keeps_a_poisoned_record
#print axioms QCore.RetentionPath.a_poisoned_record_ends_only_by_its_loop_destroy
#print axioms QCore.RetentionPath.a_poisoned_record_without_its_destroy_is_never_safe
#print axioms QCore.RetentionPath.the_old_sequence_no_longer_decides
#print axioms QCore.RetentionPath.step_keeps_a_record
#print axioms QCore.RetentionPath.a_path_record_ends_only_by_its_own_removal_or_its_destroy
#print axioms QCore.RetentionPath.a_path_record_without_its_removal_or_destroy_is_never_safe
#print axioms QCore.RetentionPath.a_destroy_ends_every_record_at_its_loop
#print axioms QCore.RetentionPath.release_refused_iff
#print axioms QCore.RetentionPath.a_safe_path_never_releases_a_held_identity
#print axioms QCore.RetentionPath.a_safe_path_leaves_no_outliving_holder_holding
#print axioms QCore.RetentionPath.path_decision_vectors_hold
#print axioms QCore.RetentionPath.path_trace_vectors_hold
