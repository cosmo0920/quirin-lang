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

import QCore.StorageDuration

/-! Audited companion to CG15-STORAGE-DURATION-AND-VOLATILE (gateway rule 11h). -/
#print axioms QCore.StorageDuration.a_static_object_outlives_every_frame
#print axioms QCore.StorageDuration.a_thread_local_object_outlives_every_frame
#print axioms QCore.StorageDuration.outlives_frame_iff
#print axioms QCore.StorageDuration.a_static_declaration_is_no_event
#print axioms QCore.StorageDuration.refused_is_sticky
#print axioms QCore.StorageDuration.run_refused_is_sticky
#print axioms QCore.StorageDuration.a_static_store_escapes_the_frame
#print axioms QCore.StorageDuration.a_static_store_is_refused_not_a_leak
#print axioms QCore.StorageDuration.a_static_read_has_no_identity
#print axioms QCore.StorageDuration.a_static_read_changes_no_custody
#print axioms QCore.StorageDuration.releasing_a_static_read_refuses
#print axioms QCore.StorageDuration.per_call_initialisation_misjudges_the_second_call
#print axioms QCore.StorageDuration.a_volatile_read_need_not_return_the_stored_identity
#print axioms QCore.StorageDuration.a_known_value_misjudges_a_volatile_branch
#print axioms QCore.StorageDuration.a_custody_decision_on_a_volatile_read_is_refused
#print axioms QCore.StorageDuration.a_neutral_volatile_branch_is_sound
#print axioms QCore.StorageDuration.trace_count
#print axioms QCore.StorageDuration.before_bounded
#print axioms QCore.StorageDuration.sequenced_accesses_are_ordered
#print axioms QCore.StorageDuration.unsequenced_accesses_are_never_ordered
#print axioms QCore.StorageDuration.forms_unique
#print axioms QCore.StorageDuration.unlisted_form_refused
#print axioms QCore.StorageDuration.atomic_access_is_cg19s
#print axioms QCore.StorageDuration.every_refused_row_is_a_custody_form
#print axioms QCore.StorageDuration.shape_vectors_follow_the_table
