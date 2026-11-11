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

import QCore.ReachGatePosition

/-! Card REACH-GATE: the INDIRECT half of the joint expression-position gate.

Model of family **F4** of `crates/qcore-c-gateway/src/neutral_position.rs` — the
bounded typed access chain admitted inside the six expression positions — and of
the role split `crates/qcore-c-gateway/src/legacy_c.rs` threads into
`Parser::supported_expression`. The Rust reader's extraction of declared types,
record members, the bound-name over-approximation and the contract environment
is a premise, as in `QCore.LegacyCAnalyzer`, `QCore.BorrowedActual`,
`QCore.MayAlias` and `QCore.NeutralExpressionPosition`; nothing here is a claim
about arbitrary C memory safety.

* **F4, the chain** — a scalar leaf is admitted in either role; a pointer leaf,
  record-typed or not, is admitted *exactly* where the position discards the
  value; a base this body may bind refuses at any depth
  (`REACH-GATE-BOUND-BASE`, card O04-READ-BOUND's subject, whose liveness
  premise an expression position cannot supply); a base whose declared type the
  reader would not commit to, a leaf it would not commit to and an empty
  selection list each refuse; a link that is not a fully typed record pointer
  refuses; the walk reaches the leaf exactly at the bound and a chain longer
  than the bound yields no leaf at all, so the bound is a refusal and never a
  truncation.
* **F1, the call** — carried over from card W4-EXPR-POS and restated for this
  card's step, which additionally decides every argument **without** the pointer
  leaf: an uncontracted callee, a non-`None` result, any non-`Borrowed`
  parameter and a mismatched arity each refuse; a neutral contract over admitted
  arguments is admitted; every argument of an admitted call is itself admitted
  without the pointer leaf; and a call with arguments exhausts the depth budget
  rather than being truncated.
* **The refined soundness sentence** — an admitted expression reads through no
  bound base and names no bare pointer operand anywhere; an expression admitted
  without the pointer leaf produces no pointer value anywhere, from which a
  `return` expression and every argument of an admitted call keep card
  W4-EXPR-POS's sentence verbatim; and an assignment refuses every position
  where the pointer leaf is in play, so an admitted pointer value can never be
  stored.
* **Additivity, and the refusing direction** — a position the lexical predicate
  or card W4-EXPR-POS's families already accepted is decided by them; a refused
  expression keeps its `Unknown` and an admitted one becomes a `NoOp`; and
  switching the pointer leaf off only ever withdraws admissions.
* **Non-vacuity, in both directions** — the `z_*` fixture family measured in
  `crates/qcore-c-gateway/tests/reach_gate_positions.rs`, projected: the
  three-selection scalar chain is admitted in either role; the pointer chain
  splits on the role; the same chain refuses on a bound base; four selections
  are admitted and five refuse; an unmodelled leaf refuses; a scalar chain is
  admitted inside a call argument and a pointer chain is not; an assignment
  refuses the pointer chain; and a bare pointer operand refuses beside an
  admitted call. -/

-- F4: the bounded typed access chain.
#print axioms QCore.ReachGatePosition.a_scalar_leaf_is_admitted_in_either_role
#print axioms
  QCore.ReachGatePosition.a_pointer_leaf_is_admitted_exactly_where_the_value_is_discarded
#print axioms
  QCore.ReachGatePosition.a_record_pointer_leaf_is_admitted_exactly_where_the_value_is_discarded
#print axioms QCore.ReachGatePosition.a_bound_base_refuses_every_chain
#print axioms QCore.ReachGatePosition.a_base_without_a_declared_record_type_refuses
#print axioms QCore.ReachGatePosition.an_opaque_leaf_refuses_in_either_role
#print axioms QCore.ReachGatePosition.an_empty_selection_list_refuses
#print axioms QCore.ReachGatePosition.a_chain_through_a_non_record_pointer_link_refuses
#print axioms QCore.ReachGatePosition.a_chain_at_the_bound_is_walked
#print axioms QCore.ReachGatePosition.a_chain_longer_than_the_bound_refuses
#print axioms QCore.ReachGatePosition.a_five_selection_chain_refuses_at_the_bound
#print axioms QCore.ReachGatePosition.a_chain_admitted_without_the_pointer_leaf_has_a_scalar_leaf
#print axioms QCore.ReachGatePosition.a_chain_admitted_without_the_pointer_leaf_is_admitted_with_it

-- F1: calls, restated for this card's step.
#print axioms QCore.ReachGatePosition.an_uncontracted_call_is_never_admitted
#print axioms QCore.ReachGatePosition.a_non_none_result_is_never_admitted
#print axioms QCore.ReachGatePosition.a_non_borrowed_parameter_is_never_admitted
#print axioms QCore.ReachGatePosition.a_mismatched_arity_is_never_admitted
#print axioms QCore.ReachGatePosition.a_neutral_call_of_admitted_arguments_is_admitted
#print axioms
  QCore.ReachGatePosition.every_argument_of_an_admitted_call_is_admitted_without_the_pointer_leaf
#print axioms QCore.ReachGatePosition.a_call_exhausting_the_budget_refuses_a_nonempty_argument_list

-- The refined soundness sentence.
#print axioms QCore.ReachGatePosition.an_admitted_expression_reads_through_no_bound_base
#print axioms QCore.ReachGatePosition.an_admitted_expression_names_no_bare_pointer
#print axioms
  QCore.ReachGatePosition.an_expression_admitted_without_the_pointer_leaf_names_no_pointer_value
#print axioms QCore.ReachGatePosition.a_returned_expression_names_no_pointer_value
#print axioms QCore.ReachGatePosition.an_argument_of_an_admitted_call_names_no_pointer_value
#print axioms QCore.ReachGatePosition.an_assignment_refuses_every_pointer_leaf_position

-- Additivity and the refusing direction.
#print axioms QCore.ReachGatePosition.the_earlier_families_still_decide_every_position_they_accepted
#print axioms QCore.ReachGatePosition.a_refused_expression_keeps_its_unknown
#print axioms QCore.ReachGatePosition.an_admitted_expression_becomes_a_no_op
#print axioms QCore.ReachGatePosition.refusing_the_pointer_leaf_only_refuses

-- Non-vacuity: the measured shapes, in both directions.
#print axioms QCore.ReachGatePosition.the_measured_scalar_chain_is_admitted_in_either_role
#print axioms QCore.ReachGatePosition.the_measured_pointer_chain_splits_on_the_role
#print axioms QCore.ReachGatePosition.the_measured_chain_refuses_on_a_bound_base
#print axioms QCore.ReachGatePosition.a_five_selection_chain_refuses
#print axioms QCore.ReachGatePosition.a_four_selection_chain_is_admitted
#print axioms QCore.ReachGatePosition.an_opaque_leaf_refuses_the_measured_chain
#print axioms QCore.ReachGatePosition.a_scalar_chain_is_admitted_inside_a_call_argument
#print axioms QCore.ReachGatePosition.a_pointer_chain_refuses_inside_a_call_argument
#print axioms QCore.ReachGatePosition.an_assignment_refuses_the_measured_pointer_chain
#print axioms QCore.ReachGatePosition.a_bare_pointer_operand_refuses_beside_an_admitted_chain
