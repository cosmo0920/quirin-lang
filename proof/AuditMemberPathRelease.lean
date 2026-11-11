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

import QCore.MemberPathRelease

/-! Audited companion to OP-CALLBACK-MEMBER-PATH-RELEASE (gateway rule 30, contract specification C04.19.6): a callback candidate's member-path state composed with the release's arms decides a linked object only when every reading takes it out of the caller's custody exactly once (no double release, no missed release) or never; a stale path and an unstated member never decide; one refusal refuses the join over the candidates; the set `{V, #kept}` is the join of its members; a guard complementing the hand-off's partitions the results. -/

#print axioms QCore.MemberPathRelease.the_set_is_the_join_of_its_members
#print axioms QCore.MemberPathRelease.gone_leaves_exactly_once
#print axioms QCore.MemberPathRelease.stays_never_leaves
#print axioms QCore.MemberPathRelease.a_stale_path_never_decides
#print axioms QCore.MemberPathRelease.an_unstated_member_never_decides
#print axioms QCore.MemberPathRelease.a_refusal_refuses_the_join
#print axioms QCore.MemberPathRelease.a_gone_join_is_gone_for_every_candidate
#print axioms QCore.MemberPathRelease.the_complement_partitions_the_results
#print axioms QCore.MemberPathRelease.compose_vectors_hold
