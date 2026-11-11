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

import QCore.CandidateComposition

/-! Audited companion to HANDOFF-CANDIDATE-OWN-CALLBACKS (gateway rule 13u amended, with 12m and 12n): a callback candidate whose declaration carries its own member invocations is contracted exactly when its own reading admits and each own invocation decides; the contracted set is the least fixed point, so a cycle (a self-cycle, two candidates) never supports itself, and the depth-first search the Rust runs agrees with it on the shared vectors; a variable written only by an exact copy of a member calls a value the member held (or a formal's own value), so the member's closed set covers the call, and any other write may call another value. -/

#print axioms QCore.CandidateComposition.a_composed_candidate_is_contracted_iff_each_own_invocation_decides
#print axioms QCore.CandidateComposition.an_undecided_own_invocation_refuses
#print axioms QCore.CandidateComposition.a_cycle_never_supports_itself
#print axioms QCore.CandidateComposition.a_self_cycle_never_stands
#print axioms QCore.CandidateComposition.a_two_candidate_cycle_never_stands
#print axioms QCore.CandidateComposition.compose_vectors_hold
#print axioms QCore.CandidateComposition.exec_held
#print axioms QCore.CandidateComposition.an_exact_copy_calls_a_member_value
#print axioms QCore.CandidateComposition.a_copied_local_calls_a_member_value
#print axioms QCore.CandidateComposition.an_exact_copy_stays_in_the_closed_set
#print axioms QCore.CandidateComposition.an_inexact_copy_may_call_another_value
#print axioms QCore.CandidateComposition.copy_vectors_hold
#print axioms QCore.CandidateComposition.an_admitted_copy_is_the_only_write
