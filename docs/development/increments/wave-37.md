# Wave 37 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

## SCALAR-ROUTE-EXIT-EDGE (2026-09-18, wave 37's first arbiter)

`STABILITY-REEVAL-2` asked whether the pre-existing value route in
`refine_pointer_null` — original Cycle-19 code carried by no card, recorded as
`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED` — could delete a loop header's **exit**
edge, the wave-35 pin having probed only positively spelled headers and so only
the back edge. It can, and the consequence is a false `Safe`: with
`int flag = 0;` and a live acquisition after the loop, `while (!flag) { }`,
`while (flag == 0) { }`, the same in `do`-`while` and `for`, and the
known-non-null second arm `while (p)` all returned **`Safe` with no diagnostic**
where the loop-free control is `Unsafe` with a leak — because a deleted exit
edge leaves the function's exit node with no incoming state, so `report_leaks`
never runs and *every* downstream diagnostic is suppressed, a real double
release after the loop included. The card **restricts** rather than defends, and
the deciding measurement is that a defence is impossible as stated: for a
`volatile` flag the loop terminates in well-defined C (C11 §5.1.2.3) and the
route's must-fact is simply false, while a second premise — that every body
write reaches the header as a forget, opaque value or poison, which is
constraint 10a's `Sparing` completeness in a role 10a does not record — would
have been needed even then. `LegacyCfgNode` now carries `loop_exit_edge`, a
static fact recorded where each header's successors are assigned, the
`Infeasible` arm retains that edge carrying the unrefined state, and the refusal
is reported from the **settled** states by `report_retained_loop_exit_edges`
rather than inside the work list, because the arm is reached on intermediate
iterates of a header whose fixed point later widens the binding to `unknown`
(constraint 6's schedule hazard, handled as `require_live_borrowed_actuals`
handles its own must-fact). The scope is structural and deliberately narrow: an
`if`'s two arms rejoin, so pruning one orphans nothing, and the corpus has
**154** such prunings whose precision a blanket restriction would have
destroyed; the sibling **back** edge is left exactly where it stood
(`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`), deleting a body being what
`C-GATEWAY-CONSTANT-DO-WHILE` proved for a literal token; and `for (;;)` carries
no exit successor for the rule to reach (`FOREVER-FOR-EXIT-EDGE-UNOWNED`,
untouched). The re-pin moved **nothing**: BEFORE reproduced
`docs/reports/joined-provenance-call-frames/artifacts/` byte for byte in both
verdict-bearing artifacts, AFTER reproduces BEFORE, the three-way leaf split is
**0 added / 0 removed / 54,503 unmoved** with zero unit and zero function moves,
and leak **272**, `unknown_alias` **4,308**, `unsupported` **49,922**,
`summary_conflict` **1** and `double_release` **0** all stand. The census says
why, and it is a coverage fact rather than a safety one: of **201** loop-header
`BranchOnNull` nodes, **zero** reach either deciding arm — 369 of 412
loop-header edge rows (**89.6%**) land in the route's `untracked` early return,
because the reduced importer has no scalar type environment, while 37 rows do
spell the inverted polarity. A card that binds integer loop conditions makes the
hazard reachable in one step (`SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES`), which is
why the rule was landed now. Proved as
`QCore.ScalarRouteExitEdge.the_exit_edge_is_never_removed`, with
`propagation_only_grows`, `base_reach_implies_reach`, `no_diagnostic_is_withdrawn`
and `the_rule_never_promotes_to_safe` for the accusing direction,
`a_branch_is_unchanged` and `the_body_edge_is_unchanged` for the boundary, and
`the_abstract_state_does_not_decide_the_exit` carrying the `volatile` witness;
eleven end-to-end tests assert on the leak and the exit rather than on the
`double_releases()` proxy that let the finding survive a wave, and six mutations
each turn a distinct set of them red. `CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`
is **retired**, its "more interesting outcome" clause being what happened: the
two routes do disagree. `docs/reports/scalar-route-exit-edge/README.md`.

