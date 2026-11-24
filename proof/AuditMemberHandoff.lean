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

import QCore.MemberHandoff

/-! Audited companion to MEMBER-EFFECT-STORES, body half (gateway rule 38k). -/

#print axioms QCore.MemberHandoff.admitted_exactly_when
#print axioms QCore.MemberHandoff.a_refusal_negates_its_condition
#print axioms QCore.MemberHandoff.discharged_only_by_the_handoff
#print axioms QCore.MemberHandoff.a_refusal_is_the_gates
#print axioms QCore.MemberHandoff.a_used_local_keeps_the_refusal
#print axioms QCore.MemberHandoff.a_released_local_keeps_the_refusal
#print axioms QCore.MemberHandoff.a_second_store_keeps_the_refusal
#print axioms QCore.MemberHandoff.a_realloc_self_assignment_is_refused
#print axioms QCore.MemberHandoff.a_loop_is_refused
#print axioms QCore.MemberHandoff.a_rebound_formal_is_refused
#print axioms QCore.MemberHandoff.a_local_alias_of_the_formal_is_refused
#print axioms QCore.MemberHandoff.a_borrowed_use_after_is_refused
#print axioms QCore.MemberHandoff.a_row_lacking_the_premise_is_refused
#print axioms QCore.MemberHandoff.a_carried_row_lists_the_premise
#print axioms QCore.MemberHandoff.the_old_value_is_custody_free_or_premised
#print axioms QCore.MemberHandoff.no_summary_with_a_handoff
#print axioms QCore.MemberHandoff.a_callers_decision_is_unchanged
#print axioms QCore.MemberHandoff.decision_vectors_hold
#print axioms QCore.MemberHandoff.the_tables_are_prefixed
