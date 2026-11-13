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

import QCore.ListLengthBound

/-! Audited companion to W45-FB-LIST-FINITENESS-PREMISE (a list-length bound under a declared list-finiteness premise, gateway rule 48). -/

#print axioms QCore.ListLengthBound.a_link_preserving_frame_fixes_the_links
#print axioms QCore.ListLengthBound.cursor_is_the_walk
#print axioms QCore.ListLengthBound.safe_cursor_is_the_plain_cursor
#print axioms QCore.ListLengthBound.a_prefix_of_a_run_is_a_run
#print axioms QCore.ListLengthBound.the_loop_exits_after_the_length
#print axioms QCore.ListLengthBound.the_full_run_has_the_length
#print axioms QCore.ListLengthBound.run_length_bounded
#print axioms QCore.ListLengthBound.safe_run_length_bounded
#print axioms QCore.ListLengthBound.measure_positive_while_running
#print axioms QCore.ListLengthBound.measure_decreases_by_one
#print axioms QCore.ListLengthBound.appending_cursor
#print axioms QCore.ListLengthBound.walk_appending
#print axioms QCore.ListLengthBound.every_inserted_state_is_well_formed
#print axioms QCore.ListLengthBound.an_inserting_body_never_exits
#print axioms QCore.ListLengthBound.an_ill_formed_list_never_exits
#print axioms QCore.ListLengthBound.the_ill_formed_list_has_no_length
#print axioms QCore.ListLengthBound.an_admitted_shape_meets_the_premises
#print axioms QCore.ListLengthBound.an_undeclared_head_is_refused
#print axioms QCore.ListLengthBound.a_frame_writing_links_is_refused
#print axioms QCore.ListLengthBound.an_undefined_callee_is_refused
#print axioms QCore.ListLengthBound.a_non_traversal_is_refused
#print axioms QCore.ListLengthBound.recording_a_fact_keeps_the_verdict
#print axioms QCore.ListLengthBound.recording_facts_keeps_the_verdict
#print axioms QCore.ListLengthBound.decide_vectors_hold
