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

import QCore.NeutralExpressionPosition

/-! Card W4-EXPR-POS: the typed neutral-expression-position admission.

Model of `crates/qcore-c-gateway/src/neutral_position.rs` and the `Parser`
method in `crates/qcore-c-gateway/src/legacy_c.rs` that consults it after the
existing lexical predicate. The Rust reader's extraction of declared types,
record members, the bound-name over-approximation and the contract environment
is a premise, as in `QCore.LegacyCAnalyzer`, `QCore.BorrowedActual` and
`QCore.MayAlias`; nothing here is a claim about arbitrary C memory safety.

* **F2 and F3, the operands** — a literal, an enumeration constant and a
  declared scalar are admitted at every depth; a pointer operand never is; a
  direct scalar-member read is admitted exactly when the record model carries
  the member and the body never binds the base, and refuses on either failure
  alone.
* **F1, the call** — an uncontracted callee, a non-`None` result, any
  non-`Borrowed` parameter and a mismatched arity each refuse; a neutral
  contract over admitted arguments is admitted; every argument of an admitted
  call is itself admitted; and a call with arguments exhausts the depth budget
  rather than being truncated.
* **The soundness sentence** — an admitted expression names no pointer operand
  anywhere and reads through no bound base anywhere. This is what makes
  replacing the position's `Unknown` with a `NoOp` skip no modeling.
* **Additivity** — a position the lexical predicate already accepted is decided
  by it, a refused expression keeps its `Unknown`, and an admitted one becomes
  a `NoOp`.
* **Non-vacuity** — the design note's §6 minimal validation case is admitted,
  and refuses without its contract, with a pointer actual, with a releasing
  callee and with an owned result; four nested neutral calls fit the budget and
  five do not; the field-read family decides both ways; and one unadmitted call
  refuses the whole expression. -/

-- F2 and F3: operands.
#print axioms QCore.NeutralExpressionPosition.a_literal_is_admitted
#print axioms QCore.NeutralExpressionPosition.an_enumeration_constant_is_admitted
#print axioms QCore.NeutralExpressionPosition.a_declared_scalar_is_admitted
#print axioms QCore.NeutralExpressionPosition.a_pointer_operand_is_never_admitted
#print axioms QCore.NeutralExpressionPosition.an_unbound_scalar_field_read_is_admitted
#print axioms QCore.NeutralExpressionPosition.a_bound_base_refuses_the_read
#print axioms QCore.NeutralExpressionPosition.a_non_scalar_field_refuses_the_read

-- F1: calls.
#print axioms QCore.NeutralExpressionPosition.an_uncontracted_call_is_never_admitted
#print axioms QCore.NeutralExpressionPosition.a_non_none_result_is_never_admitted
#print axioms QCore.NeutralExpressionPosition.a_non_borrowed_parameter_is_never_admitted
#print axioms QCore.NeutralExpressionPosition.a_mismatched_arity_is_never_admitted
#print axioms QCore.NeutralExpressionPosition.a_neutral_call_of_admitted_arguments_is_admitted
#print axioms QCore.NeutralExpressionPosition.every_argument_of_an_admitted_call_is_admitted
#print axioms
  QCore.NeutralExpressionPosition.a_call_exhausting_the_budget_refuses_a_nonempty_argument_list

-- The soundness sentence the permissive direction rests on.
#print axioms QCore.NeutralExpressionPosition.an_admitted_expression_names_no_pointer
#print axioms QCore.NeutralExpressionPosition.an_admitted_expression_reads_through_no_bound_base

-- The admission is additive and decides the position.
#print axioms
  QCore.NeutralExpressionPosition.the_lexical_predicate_still_decides_every_position_it_accepted
#print axioms QCore.NeutralExpressionPosition.a_refused_expression_keeps_its_unknown
#print axioms QCore.NeutralExpressionPosition.an_admitted_expression_becomes_a_no_op

-- Non-vacuity: the measured shapes, in both directions.
#print axioms QCore.NeutralExpressionPosition.the_digits10_return_expression_is_admitted
#print axioms
  QCore.NeutralExpressionPosition.the_digits10_return_expression_refuses_without_its_contract
#print axioms QCore.NeutralExpressionPosition.the_same_call_with_a_pointer_actual_refuses
#print axioms QCore.NeutralExpressionPosition.a_releasing_callee_refuses
#print axioms QCore.NeutralExpressionPosition.an_owned_result_refuses
#print axioms QCore.NeutralExpressionPosition.four_nested_neutral_calls_are_admitted
#print axioms QCore.NeutralExpressionPosition.five_nested_neutral_calls_refuse
#print axioms QCore.NeutralExpressionPosition.an_unbound_base_reads_its_scalar_member
#print axioms QCore.NeutralExpressionPosition.a_bound_base_never_reads_even_a_scalar_member
#print axioms QCore.NeutralExpressionPosition.an_unmodelled_member_refuses
#print axioms QCore.NeutralExpressionPosition.one_unadmitted_argument_refuses_the_whole_call
