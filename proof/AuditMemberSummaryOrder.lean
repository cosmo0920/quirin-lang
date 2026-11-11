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

import QCore.MemberSummaryOrder

/-! Audited companion to RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN (gateway rule 12n, amended): a decided member invocation's closed candidates are summarised before their caller, so a formal released through the call reads as a tracked local; the formal is never Safe where the local is not, and a candidate that could alias the formal keeps it non-Safe. -/

#print axioms QCore.MemberSummaryOrder.a_closed_member_candidate_precedes_its_caller
#print axioms QCore.MemberSummaryOrder.an_open_member_adds_no_edge
#print axioms QCore.MemberSummaryOrder.installed_classes_agree
#print axioms QCore.MemberSummaryOrder.an_uninstalled_candidate_refuses_at_summary_time
#print axioms QCore.MemberSummaryOrder.a_formal_reads_as_the_local_once_candidates_precede
#print axioms QCore.MemberSummaryOrder.the_formal_is_never_safe_where_the_local_is_not
#print axioms QCore.MemberSummaryOrder.a_candidate_that_could_alias_keeps_the_formal_unsafe
#print axioms QCore.MemberSummaryOrder.fixture_vectors_hold
