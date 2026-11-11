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

import QCore.ConstantDoWhile

/-! Card `C-GATEWAY-CONSTANT-DO-WHILE`: the constant `do … while (0)` exit.

Model of the `Statement::DoWhile` arm of the CFG builder in
`crates/qcore-c-gateway/src/legacy_c.rs` and of the token predicate
`condition_is_literal_zero` that keys it.

`do S while (0)` executes `S` exactly once (C11 §6.8.5.2), so the CFG without
a back edge is the construct's exact semantics and not an approximation of it.
The gateway built the back edge unconditionally, and on the pinned Fluent Bit
corpus the `do { … } while (0)` macro idiom is pervasive — the card's
preprocessed census measures it, source-level greps do not see it — so a
release inside one looked repeatable.

The audit covers both directions, because removing an edge is the **admitting**
direction that invariant 1 governs. Conservativity:
`removing_the_back_edge_only_removes_paths` and
`the_successor_list_never_grows` say the new CFG's paths and edges are
contained in the old one's, for every condition, so this change can withdraw a
refusal and can never manufacture one. Exactness:
`constant_do_while_is_its_body` is the `≡ S` statement as an equality of
admitted sets. Non-vacuity, so the rule is not decoration:
`the_back_edge_admits_a_second_execution` and
`the_constant_exit_refuses_the_second_execution` show the two models genuinely
part, and `the_repeated_release_is_a_path_the_c_does_not_have` names the parting
witness as the corpus's own `rd_kafka_jwt_b64_decode_payload` row — one release
under the C, two under the back edge.

`a_non_literal_condition_is_unchanged` and
`every_other_header_keeps_both_successors` are the boundary: the rule is keyed
on the single literal token `0` (standing constraint 5a — an operand-class
marker is never a computed value), so `while (x)` with `x` known zero,
`while (1 - 1)` and `while ((0))` all keep the CFG they had. The unclaimed
literal-zero spellings are the named residual
`CONSTANT-DO-WHILE-ZERO-SPELLINGS`.

`the_exit_edge_is_never_removed` is why this rule cannot strand a body or
truncate analysed coverage: it removes a path, never a destination.
-/

#print axioms QCore.ConstantDoWhile.constant_do_while_is_its_body
#print axioms QCore.ConstantDoWhile.constant_do_while_is_one_iteration
#print axioms QCore.ConstantDoWhile.removing_the_back_edge_only_removes_paths
#print axioms QCore.ConstantDoWhile.a_non_literal_condition_is_unchanged
#print axioms QCore.ConstantDoWhile.the_back_edge_admits_a_second_execution
#print axioms QCore.ConstantDoWhile.the_constant_exit_refuses_the_second_execution
#print axioms QCore.ConstantDoWhile.the_removed_paths_are_real
#print axioms QCore.ConstantDoWhile.the_back_edge_admits_two_releases
#print axioms QCore.ConstantDoWhile.the_constant_exit_admits_exactly_one_release
#print axioms QCore.ConstantDoWhile.the_repeated_release_is_a_path_the_c_does_not_have
#print axioms QCore.ConstantDoWhile.the_literal_zero_header_has_no_back_edge
#print axioms QCore.ConstantDoWhile.every_other_header_keeps_both_successors
#print axioms QCore.ConstantDoWhile.the_exit_edge_is_never_removed
#print axioms QCore.ConstantDoWhile.the_successor_list_never_grows
