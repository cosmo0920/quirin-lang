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

import QCore.RetainedBy

/-! Audited companion to CALLEE-RETENTION-CLASS1 stage (b): retained pointers (gateway rule 12t). -/

#print axioms QCore.RetainedBy.static_outlives_every_holder
#print axioms QCore.RetainedBy.foreign_holder_is_undecided
#print axioms QCore.RetainedBy.global_holder_of_a_non_static_is_undecided
#print axioms QCore.RetainedBy.a_caller_object_outlives_a_frame_holder
#print axioms QCore.RetainedBy.run_append
#print axioms QCore.RetainedBy.run_none
#print axioms QCore.RetainedBy.undecided_is_refused
#print axioms QCore.RetainedBy.release_refused_iff
#print axioms QCore.RetainedBy.a_safe_path_never_releases_a_held_identity
#print axioms QCore.RetainedBy.a_safe_path_leaves_no_tracked_holder_holding
#print axioms QCore.RetainedBy.reset_before_release_is_safe
#print axioms QCore.RetainedBy.release_before_reset_is_refused
#print axioms QCore.RetainedBy.a_holder_released_first_is_safe
#print axioms QCore.RetainedBy.a_holder_outliving_its_pointer_is_refused
#print axioms QCore.RetainedBy.a_holder_holding_at_the_exit_is_refused
#print axioms QCore.RetainedBy.a_frame_holder_dies_at_the_exit
#print axioms QCore.RetainedBy.global_holder_controls
#print axioms QCore.RetainedBy.an_unknown_holder_is_refused
#print axioms QCore.RetainedBy.decision_vectors_hold
#print axioms QCore.RetainedBy.trace_vectors_hold
