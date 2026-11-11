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

import QCore.VariadicNested

/-! Card O04-TRACE: declared variadic data tails and one level of nested calls.

Both fail-closed directions are audited. Every premise of the nested-call
whitelist refuses on its own; a nested result that carries custody and a nested
parameter that carries custody are each refused, the second quantified over the
whole mode vocabulary so a later widening cannot slip a mode past it; and a
nested call beyond the enclosing declaration's named vector is refused unless
that declaration carries a tail, which is the point at which the card's two
halves must compose.

The composition theorem is audited in both orders together with its converse:
an admitted nested call's declared effects are the identity, so inner-then-outer
and outer-then-inner agree with the enclosing call alone and C's unspecified
argument evaluation order cannot be observed — and a contract failing either
neutrality premise is exhibited as *not* neutral, so those premises are
load-bearing rather than decorative.

For the tail: the declaration alone admits nothing, an actual that may hold a
tracked identity is refused *under* the declaration, and an actual beyond the
named vector with no declared tail is refused rather than dropped — the
fail-closed arity repair. Non-vacuity is audited in both directions, with each
premise separately flipped on the same witness. -/

-- Every premise of the nested-call whitelist refuses on its own, N9 (the
-- fail-open this card found and closed) included.
#print axioms QCore.VariadicNested.unread_declarations_never_admit_a_nested_call
#print axioms QCore.VariadicNested.a_two_level_nested_call_is_never_admitted
#print axioms QCore.VariadicNested.a_partial_nested_expression_is_never_admitted
#print axioms QCore.VariadicNested.a_call_inside_the_nested_arguments_is_never_admitted
#print axioms QCore.VariadicNested.an_uncontracted_nested_callee_is_never_admitted
#print axioms QCore.VariadicNested.an_uncontracted_outer_callee_never_admits_a_nested_call
#print axioms QCore.VariadicNested.a_nested_arity_mismatch_is_never_admitted
#print axioms QCore.VariadicNested.an_undenotable_inner_actual_is_never_admitted

-- N6 and N7: the neutrality premises, in the refusing direction.
#print axioms QCore.VariadicNested.an_owned_nested_result_is_never_admitted
#print axioms QCore.VariadicNested.a_reallocating_nested_result_is_never_admitted
#print axioms QCore.VariadicNested.a_custody_bearing_nested_parameter_is_never_admitted
#print axioms QCore.VariadicNested.an_admitted_nested_contract_is_neutral

-- N8: the slot the nested result occupies, where the two halves compose.
#print axioms QCore.VariadicNested.a_custody_slot_never_admits_a_nested_call
#print axioms QCore.VariadicNested.a_nested_call_beyond_the_named_vector_needs_a_declared_tail
#print axioms QCore.VariadicNested.an_admitted_nested_slot_is_data

-- The composition theorem, both orders, and the fold it rests on.
#print axioms QCore.VariadicNested.borrowed_fold_is_the_identity
#print axioms QCore.VariadicNested.an_admitted_nested_call_is_neutral
#print axioms QCore.VariadicNested.compose_inner_first
#print axioms QCore.VariadicNested.compose_outer_first
#print axioms QCore.VariadicNested.evaluation_order_is_unobservable
#print axioms QCore.VariadicNested.admitted_sibling_calls_are_the_identity
#print axioms QCore.VariadicNested.sibling_evaluation_order_is_unobservable

-- The converse: each neutrality premise is load-bearing, exhibited.
#print axioms QCore.VariadicNested.an_owned_result_is_not_neutral
#print axioms QCore.VariadicNested.a_releasing_parameter_is_not_neutral
#print axioms QCore.VariadicNested.an_owned_parameter_is_not_neutral

-- The variadic data tail, including the fail-closed arity repair.
#print axioms QCore.VariadicNested.an_excess_actual_without_a_declared_tail_is_refused
#print axioms QCore.VariadicNested.unread_declarations_never_admit_a_tail
#print axioms QCore.VariadicNested.a_tail_actual_that_may_hold_an_identity_is_never_admitted
#print axioms QCore.VariadicNested.a_tail_actual_taking_an_address_is_never_admitted
#print axioms QCore.VariadicNested.a_tail_actual_carrying_an_unadmitted_call_is_never_admitted
#print axioms QCore.VariadicNested.an_empty_tail_actual_is_never_admitted
#print axioms QCore.VariadicNested.every_admitted_tail_actual_is_data
#print axioms QCore.VariadicNested.the_declaration_alone_admits_nothing

-- Non-vacuity, in both directions.
#print axioms QCore.VariadicNested.a_nested_call_in_a_declared_tail_is_admitted
#print axioms QCore.VariadicNested.a_nested_call_at_a_borrowed_parameter_is_admitted
#print axioms QCore.VariadicNested.a_nested_call_at_a_custody_parameter_is_refused
#print axioms QCore.VariadicNested.the_nested_premises_are_separately_load_bearing
#print axioms QCore.VariadicNested.the_nested_contract_premises_are_separately_load_bearing
#print axioms QCore.VariadicNested.a_data_tail_is_admitted
#print axioms QCore.VariadicNested.the_tail_premises_are_separately_load_bearing
#print axioms QCore.VariadicNested.an_empty_excess_still_needs_the_declaration
#print axioms QCore.VariadicNested.the_witness_nested_contract_is_neutral
