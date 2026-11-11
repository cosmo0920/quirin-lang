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

import QCore.TypedRootDeclarations

/-! Audited abstract companion to W47-TYPED-ROOT-DECLARATION-READER (gateway rule 10l). -/

#print axioms QCore.TypedRootDeclarations.bound_within_a_settled_set
#print axioms QCore.TypedRootDeclarations.seeded_of_check
#print axioms QCore.TypedRootDeclarations.settled_of_check
#print axioms QCore.TypedRootDeclarations.an_unlisted_name_is_never_bound
#print axioms QCore.TypedRootDeclarations.a_never_bound_root_has_no_binding
#print axioms QCore.TypedRootDeclarations.the_lexical_answer_stands
#print axioms QCore.TypedRootDeclarations.the_served_answer_implies_the_lexical_closure
#print axioms QCore.TypedRootDeclarations.the_served_answer_implies_the_ast
#print axioms QCore.TypedRootDeclarations.an_unanswered_function_serves_no_root
#print axioms QCore.TypedRootDeclarations.a_name_declared_more_than_once_serves_no_root
#print axioms QCore.TypedRootDeclarations.a_pointer_root_is_named_by_no_may_bind
#print axioms QCore.TypedRootDeclarations.a_pointer_root_is_never_bound
#print axioms QCore.TypedRootDeclarations.a_borrowed_actual_is_may_bind_not_bound
#print axioms QCore.TypedRootDeclarations.every_seed_is_may_bind
#print axioms QCore.TypedRootDeclarations.a_return_through_a_lexically_bound_root_refuses
#print axioms QCore.TypedRootDeclarations.a_return_through_an_ast_bound_root_refuses
#print axioms QCore.TypedRootDeclarations.a_served_return_hands_nothing_back
#print axioms QCore.TypedRootDeclarations.a_lexical_declared_local_stands
#print axioms QCore.TypedRootDeclarations.an_ast_declared_local_is_one_automatic_non_formal_declaration
#print axioms QCore.TypedRootDeclarations.a_shadowed_name_is_not_an_ast_declared_local
#print axioms QCore.TypedRootDeclarations.the_example_set_is_seeded_and_settled
#print axioms QCore.TypedRootDeclarations.an_untouched_root_is_never_bound
#print axioms QCore.TypedRootDeclarations.a_borrowed_root_is_never_bound
#print axioms QCore.TypedRootDeclarations.a_copy_of_a_call_result_is_bound
#print axioms QCore.TypedRootDeclarations.an_address_taken_root_is_bound
#print axioms QCore.TypedRootDeclarations.an_untouched_root_is_served
#print axioms QCore.TypedRootDeclarations.the_same_root_at_a_borrowed_actual_is_not_a_pointer_root
#print axioms QCore.TypedRootDeclarations.the_same_root_bound_lexically_is_not_served
