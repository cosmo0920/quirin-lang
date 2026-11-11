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

import QCore.MemberNonCUnits

/-! Audited companion to CALLBACK-MEMBER-NON-C-UNITS (gateway rule 12n, amended in place): an unread C++ unit whose preprocessed text is the whole unit opens only a member whose closure-graph tokens it names, or one with a graph token it was never checked against; every other unread unit opens every member. Under the reading's premise (a unit holding a store into the member names a graph token), a scoped unit that keeps the member's set holds no store into it and adds nothing to the run set, and a closed run set covers every designator any unit, read or unread, stores. -/

#print axioms QCore.MemberNonCUnits.the_recorded_reading_is_the_checked_reading
#print axioms QCore.MemberNonCUnits.an_undecided_unit_keeps_the_member_open
#print axioms QCore.MemberNonCUnits.a_named_graph_token_keeps_the_member_open
#print axioms QCore.MemberNonCUnits.an_unchecked_graph_token_keeps_the_member_open
#print axioms QCore.MemberNonCUnits.a_scoped_unit_naming_no_graph_token_holds_no_store
#print axioms QCore.MemberNonCUnits.a_scoped_unit_naming_no_graph_token_keeps_the_run_set
#print axioms QCore.MemberNonCUnits.a_closed_run_read_every_unit
#print axioms QCore.MemberNonCUnits.the_closed_run_set_covers_every_store
#print axioms QCore.MemberNonCUnits.scoped_vectors_hold
