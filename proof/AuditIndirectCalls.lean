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

import QCore.IndirectCalls

/-! Audited companion to CG22-INDIRECT-CALLS-AND-CALLBACKS (gateway rule 11p). -/

#print axioms QCore.IndirectCalls.an_open_set_is_unknown
#print axioms QCore.IndirectCalls.an_empty_set_is_unknown
#print axioms QCore.IndirectCalls.join_over_ge_each
#print axioms QCore.IndirectCalls.safe_needs_a_cover
#print axioms QCore.IndirectCalls.safe_means_every_candidate_safe
#print axioms QCore.IndirectCalls.join_over_covers_sound
#print axioms QCore.IndirectCalls.a_covering_safe_decision_is_safe
#print axioms QCore.IndirectCalls.an_open_set_can_miss_a_violation
#print axioms QCore.IndirectCalls.a_local_holds_one_of_its_definitions
#print axioms QCore.IndirectCalls.invocation_after_release_is_never_safe
#print axioms QCore.IndirectCalls.registration_after_teardown_is_never_safe
#print axioms QCore.IndirectCalls.an_escaped_registration_is_unknown
#print axioms QCore.IndirectCalls.the_protocol_is_safe
#print axioms QCore.IndirectCalls.join_vectors_hold
#print axioms QCore.IndirectCalls.join_vectors_count
#print axioms QCore.IndirectCalls.callback_vectors_hold
#print axioms QCore.IndirectCalls.callback_vectors_count
#print axioms QCore.IndirectCalls.an_inline_declarator_reads_as_the_typedef
#print axioms QCore.IndirectCalls.an_inline_declarator_projects_only_fnptr
#print axioms QCore.IndirectCalls.a_deeper_inline_declarator_stays_unread
#print axioms QCore.IndirectCalls.a_qualified_inline_declarator_stays_unread
