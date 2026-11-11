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

import QCore.ScalarRouteExitEdge

/-! Card `SCALAR-ROUTE-EXIT-EDGE`: a loop header's exit edge is never removed.

Model of the `PointerRefinement::Infeasible` arm of the custody work list in
`crates/qcore-c-gateway/src/legacy_c.rs`, of the
`LegacyCfgNode::loop_exit_edge` fact that now gates it, and of the two premises
the arm rested on before this card.

`refine_pointer_null` is the only place in that file where a successor is not
propagated. For an `if` that is ordinary path sensitivity — both arms rejoin at
the same continuation. For a **loop header** the successors do not rejoin, so
deleting the exit edge leaves the function's exit node with no incoming state,
`report_leaks` never runs there, and `classify_diagnostics` returns `Safe` for
a body that leaks, double-releases or releases after escape *after* the loop.
Card `C-GATEWAY-CONSTANT-DO-WHILE` (wave 35) proved
`the_exit_edge_is_never_removed` of its own token rule and did not check this
neighbour, whose four probed rows all spelled the header positively and so
measured the **back** edge only.

The rule: `the_exit_edge_is_never_removed` is that same property, now true of
the value route.

The audit covers both directions, because *adding* an edge is the accusing
direction. Conservativity that nothing is lost: `propagation_only_grows` at the
edge level, `base_reach_implies_reach` at the destination level,
`no_diagnostic_is_withdrawn` at the diagnostic level, and
`the_rule_never_promotes_to_safe`, which is invariant 1 itself — a `Safe`
function after this card was `Safe` before it.

The boundary, so the rule is not wider than its finding:
`a_branch_is_unchanged` keeps every `if (p)` as precise as it was, and
`the_body_edge_is_unchanged` leaves the sibling back-edge arm exactly where it
stood (`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`, this card's named residual).

Non-vacuity, so the rule is not decoration: `the_old_rule_strands_the_exit` and
`the_new_rule_keeps_the_exit` are the two rules genuinely parting on one
header, with `the_witness_agrees_on_the_body_edge` showing they part on the
exit edge alone.

Why the arm was **restricted rather than proved**.
`the_abstract_state_does_not_decide_the_exit` exhibits two concrete situations
that agree on everything `refine_pointer_null` reads and disagree on whether
the exit is reachable; the witness is a `volatile` object, which C11 §5.1.2.3
lets be modified outside the program, so `while (!flag) { }` terminates in
well-defined C while the abstract state still calls `flag` null.
`the_second_arm_has_the_same_gap` says the same of the known-non-null arm, so
the finding is not about `int` bindings.
`retention_does_not_read_the_unchecked_premise` is the content of the repair:
this card's rule does not consult the premise it cannot check.

The second unchecked premise is `SCALAR-ROUTE-POISON-DEPENDENCE`.
`a_spared_write_leaves_the_fact_standing` and `no_write_leaves_the_fact_standing`
say the deciding fact survives a body write that constraint 10a's `Sparing`
list spares, and any body that does not write the name at all — so the old
rule's exactness needed `Sparing` to be complete for the names a loop header
reads, a role that constraint does not record.
`an_unspared_write_widens_the_fact` is the arm that made the old rule look safe
wherever it had been tested: real, but not the only one.
-/

#print axioms QCore.ScalarRouteExitEdge.the_exit_edge_is_never_removed
#print axioms QCore.ScalarRouteExitEdge.propagation_only_grows
#print axioms QCore.ScalarRouteExitEdge.a_branch_is_unchanged
#print axioms QCore.ScalarRouteExitEdge.the_body_edge_is_unchanged
#print axioms QCore.ScalarRouteExitEdge.base_reach_implies_reach
#print axioms QCore.ScalarRouteExitEdge.no_diagnostic_is_withdrawn
#print axioms QCore.ScalarRouteExitEdge.the_rule_never_promotes_to_safe
#print axioms QCore.ScalarRouteExitEdge.the_old_rule_strands_the_exit
#print axioms QCore.ScalarRouteExitEdge.the_new_rule_keeps_the_exit
#print axioms QCore.ScalarRouteExitEdge.the_witness_agrees_on_the_body_edge
#print axioms QCore.ScalarRouteExitEdge.the_abstract_state_does_not_decide_the_exit
#print axioms QCore.ScalarRouteExitEdge.the_second_arm_has_the_same_gap
#print axioms QCore.ScalarRouteExitEdge.retention_does_not_read_the_unchecked_premise
#print axioms QCore.ScalarRouteExitEdge.a_spared_write_leaves_the_fact_standing
#print axioms QCore.ScalarRouteExitEdge.an_unspared_write_widens_the_fact
#print axioms QCore.ScalarRouteExitEdge.no_write_leaves_the_fact_standing
