# Wave 37 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`CT-LANE-5` (2026-09-18).** Wave 37's CT mapping lane, base `0f6eb0a6`. **One
case re-mapped, three re-measured and left unmapped, one not attempted, and no
count moved.** CT07's scenario is *two disjoint array halves mutated
independently*, and for two waves the word **independently** was carried by a
program with one thread in it: wave 28 mapped two disjoint *read* loans, wave 30
(`CT07-WRITE-REMAP`) two *write* authorities — and said so itself, "**NOT two
threads**; the task-crossing half is residual `WRITE-LEND-TASKS`". That
residual's successor landed in wave 31 and became exercisable in wave 36, whose
`WRITE-LEND-TASKS-EXERCISE` deliberately left the row alone and named the work as
its own follow-on item 7 with a discharge condition. This card discharges it:
CT07 moves `sequential` → `task`, `sequential_sanitized` →
`native_heap_scenarios`, becoming **CT03's write twin** — where CT03 is two
readers over one allocation, CT07 is two `thrd_create`'d writers over disjoint
byte ranges of one allocation, each exercising its own range write lease in its
own worker body, both leases outstanding before either byte is written, the owner
reading both bytes back after the joins, one allocation matched **by address**,
17 observation lines compared line for line, clean under ASan/UBSan/LSan.
**Mapped stays 46/50, sanitized-native 10/46, proof 3/46**; CT07 was mapped
before and after, so this is evidence quality and not coverage (constraint 23),
and the wave-30 record is superseded **from this card's own record** with its
bytes untouched (constraint 21). The remap is a **strict superset** rather than a
swap: wave 30's sequential program is retained as an accept control and its three
reject twins as rejection controls, which took two `serde`-defaulted harness
fields — `accept_controls`, because the heap mode had nowhere to put an accept
control, and `diagnostic_sequence`, because one retained control refuses in *two*
layers (`[Q054, Q053]`) and a head-only match would have let either fall silent.
Seven mutations, **seven caught**, so the row is green about something. The three
cases left unmapped are each published with their measurement rather than
inherited: **CT42** fails on both arms independently — the C twin needs a contract
loader change that cascades into `qcore-schemas` (a descriptor's failure value is
`-1`, which `ResourceReturnMode` cannot express at all), `qcore-core` and the
gateway, which this lane may not touch; and the socket cell has the model, the
Lean and `Q664` but **zero source routing**, so a row there would name stages the
harness cannot execute. **CT10**'s sub-byte half exists in exactly one
model-level module with zero frontend callers, whose own doc says the bit range
"never participates in the disjointness decision" — and the tempting byte
encoding was measured **unsound in both directions**, not merely coarse
(`CT10-BYTE-ENCODING-UNSOUND`). **CT41** is not re-attempted, there being no new
capability; **CT24** is not attempted, constraint 25 making a third withdrawal
actively hazardous. Two boundaries are published rather than papered over: there
is **no ThreadSanitizer evidence** for this case or this route
(`TASK-ROUTE-TSAN-SEGV`), so "two threads, distinct memory locations, no race" is
carried by C11 3.14 plus the sanitizer and allocation evidence and not by a race
detector; and the byte-identical trace **does not witness interleaving**, because
worker events are buffered privately and replayed by the parent
(`CT07-TWO-THREAD-REPLAY-ORDER`). The card also hit and measured a structural
hazard: eight CT fixture directories are read by `include_str!` from **outside**
`ct_corpus.rs` — three of them the proof-gated cases — so re-pointing a case can
silently re-point another card's tests, which is exactly what happened to
`write_lend.rs` here and was caught by a stale doc comment rather than by a
mechanism (`CT-FIXTURE-EXTERNAL-READERS`). Gate at the card tip: **4,611 passed /
0 failed / 8 ignored** over 231 summaries, fmt and clippy clean, both probes exit
0, **135 Lean audits**, `ct_stage_summary --check` exit 0 with no drift; the
first gate run was **killed rather than reported** because it predated the
`write_lend.rs` fix. `named_followups` 187 → **191**. +0 reserved words, +0
tokens, +0 `Q` codes, and no C gateway source change.


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

