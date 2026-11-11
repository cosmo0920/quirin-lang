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

import QCore.LexicalClosureNestedAssignment

/-! Audited abstract companion to W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (gateway rule 10o). -/

#print axioms QCore.LexicalClosureNestedAssignment.bound_mono
#print axioms QCore.LexicalClosureNestedAssignment.the_old_view_is_contained
#print axioms QCore.LexicalClosureNestedAssignment.the_superset_reading_keeps_every_old_binding
#print axioms QCore.LexicalClosureNestedAssignment.a_full_set_is_an_old_set
#print axioms QCore.LexicalClosureNestedAssignment.an_unlisted_name_is_never_bound_in_the_full_model
#print axioms QCore.LexicalClosureNestedAssignment.the_old_set_is_settled_for_the_old_view
#print axioms QCore.LexicalClosureNestedAssignment.the_nested_copy_is_bound
#print axioms QCore.LexicalClosureNestedAssignment.the_old_set_misses_a_binding
#print axioms QCore.LexicalClosureNestedAssignment.the_full_set_is_settled
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_root_is_outside_the_lexical_set
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_root_is_outside_the_ast_set
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_root_has_no_binding
#print axioms QCore.LexicalClosureNestedAssignment.where_the_ast_refuses_the_lexical_answer_stands
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_root_is_lexically_never_bound
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_root_is_not_ast_bound
#print axioms QCore.LexicalClosureNestedAssignment.an_ast_bound_root_is_not_served
#print axioms QCore.LexicalClosureNestedAssignment.where_the_ast_refuses_the_function_the_lexical_answer_is_served
#print axioms QCore.LexicalClosureNestedAssignment.an_unreadable_function_keeps_rule_10l
#print axioms QCore.LexicalClosureNestedAssignment.the_join_implies_rule_10l
#print axioms QCore.LexicalClosureNestedAssignment.the_joined_pointer_root_implies_rule_10l
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_pointer_root_is_named_by_no_may_bind
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_pointer_root_is_never_bound
#print axioms QCore.LexicalClosureNestedAssignment.a_return_through_an_ast_bound_root_refuses_everywhere
#print axioms QCore.LexicalClosureNestedAssignment.a_return_through_a_lexically_bound_root_refuses_everywhere
#print axioms QCore.LexicalClosureNestedAssignment.a_pointer_actual_through_an_ast_bound_root_refuses
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_return_hands_nothing_back
#print axioms QCore.LexicalClosureNestedAssignment.a_mixed_path_through_a_joined_bound_root_refuses
#print axioms QCore.LexicalClosureNestedAssignment.an_untouched_root_is_still_served
#print axioms QCore.LexicalClosureNestedAssignment.the_same_root_bound_by_the_ast_is_not_served
#print axioms QCore.LexicalClosureNestedAssignment.the_same_root_named_by_the_ast_may_bind_is_no_pointer_root
#print axioms QCore.LexicalClosureNestedAssignment.an_ast_refused_function_serves_the_lexical_answer
#print axioms QCore.LexicalClosureNestedAssignment.a_joined_root_in_the_gap_example
