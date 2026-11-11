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

import QCore.NullDerefGrade

/-! Audited companion to NULL-DEREF-QTT-GROUNDING (contract specification C09.3,
language specification L07, grammar G05d; USER DECISION 2026-10-11). -/

#print axioms QCore.NullDerefGrade.pathGrade_exact
#print axioms QCore.NullDerefGrade.pathGrade_valid
#print axioms QCore.NullDerefGrade.pathGrade_lower_pos_iff
#print axioms QCore.NullDerefGrade.relevant_accepts_path_iff
#print axioms QCore.NullDerefGrade.firstOf_is_firstAfter
#print axioms QCore.NullDerefGrade.firstOf_dereferenced_iff
#print axioms QCore.NullDerefGrade.bodyGrade_none_iff
#print axioms QCore.NullDerefGrade.bodyGrade_valid
#print axioms QCore.NullDerefGrade.bodyGrade_lower_pos_iff
#print axioms QCore.NullDerefGrade.inferred_iff
#print axioms QCore.NullDerefGrade.relevant_accepts_body_iff_inferred
#print axioms QCore.NullDerefGrade.necessary_iff_relevant
#print axioms QCore.NullDerefGrade.relevant_body_needs_the_identity
#print axioms QCore.NullDerefGrade.relevant_admits_only_an_existing_identity
#print axioms QCore.NullDerefGrade.refines_relevant_iff
#print axioms QCore.NullDerefGrade.relevant_accepts_iff
#print axioms QCore.NullDerefGrade.requiresIdentity_iff
#print axioms QCore.NullDerefGrade.zero_lower_imposes_nothing
#print axioms QCore.NullDerefGrade.many_imposes_nothing
#print axioms QCore.NullDerefGrade.a_path_without_a_first_dereference_infers_no_clause
#print axioms QCore.NullDerefGrade.a_clause_position_demands_the_identity
#print axioms QCore.NullDerefGrade.an_unannotated_position_demands_nothing
#print axioms QCore.NullDerefGrade.nonNull_kept
#print axioms QCore.NullDerefGrade.requiresIdentity_add
#print axioms QCore.NullDerefGrade.moved_question_is_sequential_addition
#print axioms QCore.NullDerefGrade.requiresIdentity_join
#print axioms QCore.NullDerefGrade.established_join
#print axioms QCore.NullDerefGrade.relevant_admits_join_iff
#print axioms QCore.NullDerefGrade.inferred_append
#print axioms QCore.NullDerefGrade.join_assoc
#print axioms QCore.NullDerefGrade.bodyGrade_append
#print axioms QCore.NullDerefGrade.branch_join_matches
#print axioms QCore.NullDerefGrade.caller_obligation_is_grade_admission
#print axioms QCore.NullDerefGrade.entry_is_an_admitted_identity
#print axioms QCore.NullDerefGrade.admission_establishes_the_entry
#print axioms QCore.NullDerefGrade.stronger_than_necessary_is_still_sound
#print axioms QCore.NullDerefGrade.firstGrade_requires_iff
#print axioms QCore.NullDerefGrade.grade_vectors_hold
