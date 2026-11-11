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

import QCore.RkLogCallerFrame
#print axioms QCore.RkLogCallerFrame.mem_dedup
#print axioms QCore.RkLogCallerFrame.premised_covers_every_caller_event
#print axioms QCore.RkLogCallerFrame.proved_path_needs_no_premise
#print axioms QCore.RkLogCallerFrame.a_reply_path_is_infeasible
#print axioms QCore.RkLogCallerFrame.model_paths_derive
#print axioms QCore.RkLogCallerFrame.the_consumer_path_leaves_the_frame_after_the_local_enqueue
#print axioms QCore.RkLogCallerFrame.each_premise_is_needed
#print axioms QCore.RkLogCallerFrame.run_append
#print axioms QCore.RkLogCallerFrame.enq_restores_and_destroys_nothing
#print axioms QCore.RkLogCallerFrame.enq_links_only_the_fresh_op
#print axioms QCore.RkLogCallerFrame.a_tracked_object_is_untouched
#print axioms QCore.RkLogCallerFrame.the_disabled_reply_frees_only_the_fresh_objects
#print axioms QCore.RkLogCallerFrame.caller_frame_admission
#print axioms QCore.RkLogCallerFrame.an_interleaved_release_makes_the_forward_drop_last
#print axioms QCore.RkLogCallerFrame.without_the_interleaving_nothing_is_destroyed
#print axioms QCore.RkLogCallerFrame.an_event_callback_can_release_a_tracked_object
#print axioms QCore.RkLogCallerFrame.a_frame_hold_is_unknown
#print axioms QCore.RkLogCallerFrame.a_member_never_reads_a_member_declaration
#print axioms QCore.RkLogCallerFrame.a_held_member_row_is_never_safe
#print axioms QCore.RkLogCallerFrame.an_outside_caller_reads_the_declaration
#print axioms QCore.RkLogCallerFrame.the_hold_never_certifies
#print axioms QCore.RkLogCallerFrame.the_logger_rows_are_decided_by_their_bodies
