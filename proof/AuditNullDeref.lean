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

import QCore.NullDeref

/-! Audited companion to NULL-SAFETY-PROPERTY (gateway rules 14k-14p). -/

#print axioms QCore.NullDeref.join_sound_left
#print axioms QCore.NullDeref.join_sound_right
#print axioms QCore.NullDeref.a_discharged_check_is_of_a_non_null_value
#print axioms QCore.NullDeref.check_sound
#print axioms QCore.NullDeref.afterDereference_sound
#print axioms QCore.NullDeref.offset_sound
#print axioms QCore.NullDeref.walk_sound
#print axioms QCore.NullDeref.guard_sound
#print axioms QCore.NullDeref.nullness_kept_without_assignment
#print axioms QCore.NullDeref.moved_question_is_the_origins
#print axioms QCore.NullDeref.a_safe_function_discharged_every_check
#print axioms QCore.NullDeref.no_safe_on_an_undecided_pointer
#print axioms QCore.NullDeref.unsafe_only_for_a_reachable_violation
#print axioms QCore.NullDeref.caller_obligation_establishes_the_entry
#print axioms QCore.NullDeref.an_inferred_clause_is_necessary
#print axioms QCore.NullDeref.a_relied_on_reading_is_listed
#print axioms QCore.NullDeref.a_tracked_binding_is_never_premised
#print axioms QCore.NullDeref.a_known_or_widened_binding_is_never_premised
#print axioms QCore.NullDeref.combined_safe_needs_null_safe
#print axioms QCore.NullDeref.join_vectors_hold
#print axioms QCore.NullDeref.check_vectors_hold
#print axioms QCore.NullDeref.verdict_vectors_hold
#print axioms QCore.NullDeref.first_vectors_hold
#print axioms QCore.NullDeref.inferred_vectors_hold
#print axioms QCore.NullDeref.premised_vectors_hold
