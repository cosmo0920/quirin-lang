# Wave 37 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.


**Wave 37 · `JOINED-PROVENANCE-ROUND-BOUND` (2026-09-18).** The
conditional-provenance fixed point's `FIXED_POINT_ROUNDS = 64` is replaced by
this graph's own **proved** bound, `nodes * (1 + sites + sites * names)`
(`QCore.JoinedProvenanceRoundBound`, 14 theorems; `a_cap_at_the_measure_settles`
is the operative half — a cap at or above the measure never declines). The card
started as a measurement: a decline and a convergence had returned the *same*
all-empty vector, so the decline rate was unobservable, and the smallest fix —
`iterate` returning a `Settled` outcome carrying its settling round, with
`Declined` mapped to precisely the vector `fixed_point` always returned — was
landed first and shown to reproduce the baseline archives byte for byte. Under
it, of **2,704** cyclic analysis visits over the pinned 185-unit corpus
(**1,273** distinct functions, correcting a 1,272 mis-transcription and not a
count), **2,545** never sweep at all for want of an off-cycle site; of the
**159** that do, 137 converged by round 59 and **22 declined**, every one of
them settling by round **148**. Nine of those 22, across four functions
including `flb_input_chunk_get_direct_routes`, publish facts on settling — so
by the card's own rule the bound had to move and the corpus had to be re-pinned.
It was, and **zero leaves moved**: leak **272**, `unknown_alias` **4,308**,
`unsupported` **49,922**, 54,503 leaves unmoved, zero unit and zero function
status moves, both verdict-bearing artifacts byte-identical to
`docs/reports/joined-provenance-call-frames/artifacts/`. The reason is the
finding: the relation has exactly one consumer, `release_site`, whose entire
reachable surface in this corpus is 14 visits over 6 sites — and none of the
four newly deriving functions is among them, so the new facts are published
where nothing asks for them. The two functions the wave-36 residual named need
**148** and **101** rounds and derive **nothing** on convergence, which
discharges that residual in both halves. The Lean measure had to be corrected
mid-card: counting only the names literally stored makes a site's *arrival*
increase the count, so absence must read as the top of that site's own lattice.
Six of seven mutations are caught; the survivor is recorded, the declining arm
being unreachable from `derive` unless the theorem is false. Retires
`JOINED-PROVENANCE-ROUND-BOUND`, `JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE`
and `JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED`; registers
`JOINED-PROVENANCE-BOUND-DESCENT-PREMISE`,
`JOINED-PROVENANCE-DECLINE-ARM-UNREACHABLE`,
`JOINED-PROVENANCE-BOUND-COST-UNMEASURED` and
`JOINED-PROVENANCE-BOUND-NOT-TIGHT` (named followups 171 → 172; Lean audits
135 → 136). `JOINED-PROVENANCE-RELEASE-NULLABLE` is **not** retired — it is an
untaken decision, re-measured unchanged on the final tree, exactly one gate
visit (`rd_kafka_jwt_b64_decode_payload`, site 1, `payload`) blocked by the
nullable premise alone. Evidence:
`docs/reports/joined-provenance-round-bound/README.md`.

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

**`BASE-TYPE-READER-CENSUS` (2026-09-18).** Wave 37's first measurement card
discharges `BASE-TYPE-READER-UNMEASURED`, the residual wave 36's indirect-access
census named as "the honest next census": 78.5% of that family's `Unresolved`
members carried no record tag, and nobody had measured the reader that failed to
commit the **base**'s declared type. Instrumentation only, behind the existing
off-by-default `indirect-census` feature, with both verdict-bearing artifacts of
a pinned Cycle 19 corpus audit reproducing
`docs/reports/joined-provenance-call-frames/artifacts/` **byte for byte with the
census actively streaming** — no analyzer rule changed, no verdict moved, no
re-pin owned, and the corpus baseline unmoved. The cause of every
`DeclaredType::Opaque` answer and every whole-function declaration refusal is
recorded **by the reader at the return that produces it** rather than re-derived,
because a second type reader would repeat card `C-GATEWAY-INDIRECT-CENSUS`'s
source-text mistake with a longer fuse; `readable_declarator` became
`unreadable_declarator_token`, of which it was the `is_none`, for the same
reason, with the refusal set unchanged. The card **reproduces** wave 36's figure
exactly — 10,514 of 13,396 tagless — and then **corrects its reading**: only
7,115 (67.7%) are base-type questions, the other 3,399 being chains whose base
the reader *did* commit and which lost the tag at a member or a subscript, so the
base-type reader's share of that row is **53.1%**, not 78.5%. The dominant cause
is a typedef that **defines** its aggregate (`typedef struct TAG { … } t;`),
3,220 leaves and 17.4% of the family, which `classify_specifiers` answers
`Opaque` for while the *forward* spelling resolves — one `match` arm. Its
discounted reach is **30** functions assuming the member then classifies, **12**
once it must actually classify, **17** with a per-member record-body reader
beside it, and constraint 5's premise holds for only 2 of the 12 — against 285
sole-blocked and 3,239 touched functions. The recommendation is therefore the
wave-36 card's answer reached independently one reader deeper: **no base-type
reader change survives the discount as a standalone wave-38 arbiter**, and the
leaf-mass/reach asymmetry reappears intact inside the base-type reader
(`FunctionRefused/LocalDeclaratorToken`, 2,147 leaves across 245 functions,
decides zero). Six residuals are **registered** in `named_followups`
(171 → 177), unlike wave 36's nine, and the family-size delta 18,555 → 18,549 is
attributed exactly to wave 36's own first arbiter `W36-CORPUS-CONTRACTS-3`,
confirming the registered residual `JPC-CENSUS-CONTRACT-SET-DEPENDENT` as
measured rather than hypothetical. `docs/reports/base-type-reader-census/`.

