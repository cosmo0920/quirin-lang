# Wave 37 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`DOCS-COMPACT-7` (2026-09-18).** The seventh firing of the three-file
compaction cadence, over `AGENTS.md`, `CLAUDE.md` and `README.md`, documentation
only, merged last by integrator instruction and **re-derived rather than
rebased**: a `git rebase` of its ten commits conflicted on exactly the regions
the wave rewrote, and resolving those as text would have been the union-merge the
hold existed to prevent, so the card reset to `2caf7369` and re-read every number
from its source there. **The standing constraints did not move**: 49 before and
49 after, same labels, same order, same group headings, and **47 of 49
byte-identical** — including all six this wave rewrote (5c, 7, 8, 10b, 14, 16),
each verified by hash, so constraint 14's "does not engage" enumeration is left
exactly as found. The two that changed are 23 (a self-disclaimed checkpoint whose
copied counts had gone stale a second time, removed in favour of the rule that
the ledger row is the authority) and 25 (a live corpus figure reading 76 against
**88**). `AGENTS.md` loses **9,175 bytes of narrative (−5.30%)** from six places
that had become changelogs — the headline paragraph's per-link prose, the *Cycle
19*, *CT* and *Contract corpus* rows' per-arbiter and per-lane chronicles, the
follow-on paragraph's per-merge chain, and the merged-gate row's three-wave
host-row story — each already a paraphrase of authorities the rows themselves
named, and each **moved verbatim** into this file's checkpoint section below so
the pass is not a deletion. Every current number, pin, rule and boundary is kept,
the arbiter chain kept in full at twelve links, and the *Contract corpus* row now
names its two pinned expectations, which it did not before. **Six corrections,
each re-read from its source.** The merged-gate row published **137** Lean audits
attributed to the wave-36 close, which measured **135** — its README gate table
and its own `gate.json` note both say so, and 137 is the count after this wave's
two new modules. That is **the same field wrong the same way in two consecutive
waves**: at this card's first base the row read 135 attributed to the wave-35
close, which measured 132. The mechanism is visible and is why it is registered
rather than absorbed — a close's `gate.json` carries nine keys and none is a Lean
count, so the one figure in the row with no machine-readable source in the record
it cites is exactly the one that has drifted twice (`GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`).
The follow-on paragraph opened "**213** entries at the wave-36 close" while
deriving a chain that ends at **187** — 213 is current, 187 was that close's, and
26 entries had been added since without the chain being updated. The
frozen-archive sentence still said "those seven" against a **twelve**-link chain,
and the corrected text is sharper than a re-count: **eleven** of the twelve
published an archive, `joined-provenance-round-bound` having published none. The
`Q-space` note still said "waves 32–34 … ran no condition extension either", a
universal its own `Q600` and `Q610`–`Q627` rows falsify — `MUTEX-SHARE-CROSS`
(wave 32) ran one and took the **tenth band** — so the tally is re-derived at
**fifteen firings on ten bands** and the quantifier narrowed to waves 33, 34 and
37, the same shape as `CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE` in the
neighbouring constraint about the same wave. The *CT* row said CT46 has "four
stages `unassessed` and none `executed`" where the manifest reads four
**`executed`** since wave 36, and said CT03's forward question "stays open" where
constraint 15n records it **decided**. And the *Cycle 19* row attached the
**leaf** split (183 removed, 15 added, net −168) to `unsupported`, which moves
−169. One correction the card had prepared it **no longer needed**: the wave-36
close landed `STABILITY-REEVAL` #2's constraint-5c `for (;;)` amendment first,
and with `legacy_c.rs:3312`/`:5161` — the same current lines this card had
independently re-measured when judging the proposed citations stale — so the two
agreed without contact and the card's version was dropped. `README.md` grew
**+1,208 bytes**, all correction: CT 45/50 → **46/50** and sanitized 9 → **10**,
the `std.buffer` row's "no worker operation exercises it yet" falsified in wave
36, the declared mutex crossing absent from the capability table entirely, that
crossing's own boundary added in constraint 15o's words (it witnesses a thread
boundary and says **nothing** about contention, a data race or the C memory
model, and no TSan row exists in this repository), the corpus baseline pointer
re-derived at this tip, and the main guard, which a human contributor could not
discover from `README.md` at all. `named_followups` **213 → 216**, with **zero
retirements**: two of the card's four first-pass registrations
(`W36-COVER-HIDDEN-BLOCKER`, `CJSON-DELETE-NESTED-DETACH`) had been registered by
the wave-36 close in the meantime and were **withdrawn rather than duplicated**,
and three were added — the tally residual above, the gate-row residual above, and
`MAIN-GUARD-REFUSES-PACK-REFS`, measured incidentally and reproduced
deterministically at both bases: `git pack-refs`, which git's automatic `gc`
runs, re-writes `refs/heads/main` with old = the null OID and new = the ref's
*existing* value, and the guard refuses that transaction though it advances
nothing, so a card sees the guard's banner attached to an operation that was
never a main update. Fail closed, no commit lost, documentation half landed in
`CLAUDE.md`, hook fix open. `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH` stays
deliberately unregistered (the wave-35 close dispositioned it "answered, not
registered"), and the sweep's other absent name,
`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`, is a **retired** entry still cited
in `CLAUDE.md` — precisely the case this wave's own new retirement rule predicts,
so finding it is the rule working. Theorem-shaped citations in the three files:
**79 candidates, 0 dangling** — all 18 `QCore.`-namespaced resolve into `proof/`
(137 audits, including this wave's `QCore.ScalarRouteExitEdge` and
`QCore.JoinedProvenanceRoundBound`), 39 bare Lean theorem names resolve, and the
remaining 22 are Rust functions, Rust test names, pinned-corpus C symbols and
record keys rather than claims about `proof/`. Two constraints were examined and
**deliberately not compacted**, recorded so the next firing need not rediscover
it: constraint 7's leak chain is the *attribution* of a pinned number across
every move it has made, which is the pin's evidence rather than narrative about
it, and constraint 14's keyword ledger is the census record that makes "censused
rather than assumed" checkable per card; constraint 16, which grew 2,686 bytes
this wave, is a live twice-revised measurement record and was left alone for the
same reason. `GATE-EXIT-SWALLOWED-BY-PIPE` was **not** promoted to a standing
rule — that would be writing a rule without a card — but its ground lived only in
the merged-gate row this pass compacts, so it is rehomed to the gate section with
its mechanical discharge stated as still open, beside the same treatment for
`WORKTREE-SUBMODULE-UNGATED`, whose doc half ("once per checkout" is false for a
delegated worktree) is landed. Gate at `2caf7369`, **all eight steps exit 0**:
**4,637 passed / 0 failed / 8 ignored** over 233 result summaries and 231 test
targets, **137** Lean audits, **293** Python, both probes 0, fmt and clippy
clean, provider 122 carried; all five of constraint 16's named rows in this
workspace green, and the unnamed seventh
`a_forced_override_never_weakens_an_execution` green too — under full-suite load
on a machine carrying its own build, which is **not** the quiet-host isolated
number `HOST-ROW-STARVATION-DESCRIPTION` asks for and is not offered as one.

**Wave 37 · `W37-CORPUS-CONTRACTS-4` (2026-09-18).** Wave 37's second arbiter, declarative:
six `.qcore` modules carrying **ten** declarations, six manifest rows, no crate
source at all, so one audit binary served both stages and the only difference
between them is which modules the manifest selects. BEFORE reproduces
`joined-provenance-call-frames`' `diagnostics.json` and `chunk-contracts.json`
byte for byte, which is what makes every movement below these ten premises and
nothing else. **8 functions `Unknown` → `Safe`** (205 → 213 Safe, 4,545 → 4,537
Unknown, provisional Unsafe 84 unmoved, **none lost**), **zero unit moves**;
`unsupported` 49,922 → **49,753** as 183 withdrawn and 15 added, of which
exactly **179 withdrawn name the ten declared callees — the ex-ante prediction,
met exactly**; `unknown_alias` 4,308 → 4,309; leak **272** and
`summary_conflict` **1** both unmoved. The added-leaf figure is the card's
strongest measurement and is not a raw count: **all 15 added rows are
same-position text substitutions and none is a genuine addition**, so no
position in the corpus refuses that did not refuse before. No `summary_conflict`
arose although six of the ten callees are defined in analysed units, and the
reason is structural rather than lucky — every declaration is the **neutral
element** (`N(m)`: every formal borrowed, result none), the weakest statement
the vocabulary can make, which a faithful summary of a body that releases
nothing cannot contradict; W36's conflict arose precisely because its
`rd_kafka_mem_free` declaration was *not* neutral. Two findings. First, the card
**predicted an exchange ex ante** and was exactly right: `pack_string` was
forecast, by reading `mpack_write_str(writer, str, cfl_sds_len(str))`, to
exchange its decided leaf for `nested call cfl_sds_len inside a modeled call`,
naming mechanism, function and inner callee — which turns
`W36-COVER-HIDDEN-BLOCKER` from a surprise into a prediction
(`NESTED-CALL-IN-ARGUMENT-POSITION`, now seen in a third library). Second, the
two remaining misses were **not** hidden blockers but a **withdrawal**:
`rd_kafka_topic_partition_list_sort`'s C declaration spells its comparator as an
**inline function-pointer parameter declarator**, which the ABI reader does not
parse — it resolves only the function-pointer *typedef* form card
`CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` taught it, which is why
`msgpack_packer_init`'s `fnptr` works — so constraint 8 withdrew the declaration
at every call site with `abi-unreadable-declaration`, **leaf-neutrally** (all 11
rows same-position substitutions, no function's leaf count changed). That is the
gate failing closed on a *reader* boundary rather than a disagreement, the first
such measurement from production text (`LOADER-ABI-INLINE-FNPTR-DECLARATOR`),
and the module was kept rather than reverted precisely because reverting it
would delete the only evidence for it. The card also measured **why** it lands
ten custody-neutral declarations and not one release, the exact inverse of its
predecessor: after W36 harvested the release-shaped callees, the sole-blocker
cover frontier is dominated by unit-internal `static` helpers that a name-keyed
declaration may not address at all, where W4 is the lever
(`W37-COVER-FRONTIER-IS-STATIC`). `add`, `memcpy`/`memset`, `snprintf`,
`sprintf`, `sds_cat_safe`, `rd_kafka_topic_partition_destroy0`,
`rd_kafka_destroy_app`, `XXH32`, `cmt_atomic_initialize` and three further
families were read and deliberately declined with their readings published,
including the refinement that `ADD-NAME-COLLISION` means **definitions that
disagree** and not merely several definitions — four identical vendored mpack
copies are safe to declare, `add`'s three divergent ones are not. Corpus 82 →
**88**, manifest-selected 27 → **33**, resource-v1 declarations 49 → **59**,
rejected unchanged at 145; constraint 5's pins updated deliberately in four
files. No Lean: a grown premise set is not a semantic change (invariant 6), the
`W31`/`W36` position taken on the same ground. Ten residuals **registered in the
acceptance matrix** rather than named in the report alone, which is the practice
wave 36's close asked for, taking `named_followups` **197 → 207** through
`build_matrix.py` (never by hand: the wave-36 close recorded that a hand-set count
leaves `dimensions.named_followups` stale and trips
`CommittedMatrixIsSelfConsistent`, constraint 25). The card was **rebased from
`477b733d` onto `797661f3`** and **both stages re-run there against a rebuilt
binary**, because two gateway code arbiters had merged beneath it —
`SCALAR-ROUTE-EXIT-EDGE` and `JOINED-PROVENANCE-ROUND-BOUND`, each claiming zero
movement. The re-run found **nothing moved anywhere**: BEFORE still reproduces
`a7aa3b5f…`/`c4f438e4…`, and AFTER is byte-identical to the archived AFTER in all
five artifacts, so the whole attribution stands unchanged **and both code cards'
zero-movement claims are independently confirmed from outside those cards**.
`docs/reports/w37-corpus-contracts-4/README.md`.

**`TASK-ROUTE-TSAN` (2026-09-18).** The checked concurrency routes had **no
ThreadSanitizer evidence at all** — wave 36's `WRITE-LEND-TASKS-EXERCISE`
measured TSan killing its lease exercise, wave 31's held-only program and the
published `examples/threads/two_workers` at the identical address `0x18` and
pc, concluded the fault was the route's, and registered
`TASK-ROUTE-TSAN-SEGV`. **It is not the route's.** ThreadSanitizer's runtime
ships an interceptor for `pthread_create` and **none for `thrd_create`**, so on
glibc 2.34+ a thread the emitted runtime creates reaches `start_thread`
unregistered, `cur_thread()` is null in it, and the first instrumented event
faults at `__tsan_func_entry+19` — `mov 0x18(%rdi),%rax`, symbolized under
`gdb`. The measurement that settles the attribution is a fifteen-line C file
containing **no QCore code**, which dies identically; its `pthread_create` twin
runs clean; and `nm` finds three `pthread_create` interceptor symbols and no
`thrd_create` counterpart beside a `thrd_create@GLIBC_2.34` import. Wave 36's
fourth row — a minimal `thrd_create` program running clean — was a **false
negative**: its thread body held no memory access and no call, so clang
instrumented it not at all and there was no `__tsan_func_entry` to fault. The
charter's own first hypothesis, a fixed-address mapping colliding with TSan's
reserved ranges, was **refuted** rather than assumed away: `0x18` is on the
zero page, and no `mmap` or `MAP_FIXED_NOREPLACE` appears in anything this
route emits. **Nothing in QCore changed and no emitted byte moved** — emitting
C11 `thrd_create` is deliberate, the route must also emit for MSVC — so the
workaround is a harness one: `tests/support/thread_sanitizer.rs` replaces
`thrd_create`/`thrd_join` at their **call sites** through `-include`, with the
pthread-backed bodies the shipped `c11_threads_compat.h` already compiles on
Apple hosts, one binding covering both emitters because the task runtime
reaches those names through its own documented `QCORE_TASK_CREATE` hook and the
mutex backend reaches them directly. The rows: all five task-route subjects
(the lease exercise and all four published `examples/threads` programs) die
unbound at the one fault and run **clean bound**, with zero races; CT46's
emitted crossing joins them in `mutex_c_emit.rs`. The clean row is **not
vacuous** — forcing the two workers onto one byte makes TSan name the race
inside `qcore_buffer_byte_write`, with both worker stacks and the heap block
`qcore_buffer_new` allocated — so wave 36's C11 3.14 claim that two threads
writing distinct bytes of one `int32_t` are not in a data race is now backed by
a race detector rather than by argument. All nine new arms were shown non-vacuous by
mutation, restored with mtime advanced — including a second, route-level one: 
replacing `shared_readers`' emitted `QCORE_TASK_JOIN` with a sleep, keeping
every other statement, makes TSan report two races, so the clean rows depend on
the join edge rather than on the detector seeing nothing. That mutation also
shows the instrument's character, since the sleep means the accesses never
physically overlap and TSan reports them anyway. What the rows cannot say
is stated rather than elided: they run a pthread-bound build
(`TASK-ROUTE-TSAN-PTHREAD-BINDING`), a clean detector run is evidence about the
schedules that executed and the failure arms are never taken
(`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`), CT46's checked trace still serialises
its child so `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION` is restated rather than
improved on, and every row is `cfg`-gated to Linux and contributes **no rows**
elsewhere rather than passing vacuously
(`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN`). One process finding is published rather
than smoothed over: `docs/reports/remaining-t04/README.md` §5.1 had stated the
**correct** diagnosis, and the remedy, waves earlier; it reached no tracker,
and wave 36 then contradicted it — `TSAN-DIAGNOSIS-WAS-LOST-THEN-CONTRADICTED`.
Gate at `dd07c480`: **4,620 passed / 0 failed / 8 ignored** over 232 result summaries, fmt and clippy clean, both probes exit 0, **135** Lean audits — all six steps 0. Run twice and both runs reported: the first exited 1 on `cargo fmt`, catching two rustfmt violations in this card's own new support module, which is the gate working; the repair is whitespace only and the whole gate was re-run rather than the one step. The test figure was identical across both runs, and no constraint-16 load-sensitive row flaked in either, despite another session's full gate sharing the machine at load ~33. `named_followups` **171 → 174** (+4 −1; `TASK-ROUTE-TSAN-SEGV` retired against
its own discharge condition, met word for word). No Lean: no semantic surface
moved — no checker rule, no model, no emitted byte.

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

## Checkpoints moved verbatim by `DOCS-COMPACT-7`

The blocks below were superseded **in place** by the wave-37 compaction
described above. They are reproduced **verbatim**, as checkpoints of their own
dates, so that the compaction is not a deletion. They are **not** a second
current rule authority: every current number and every standing rule lives in
[AGENTS.md](../../../AGENTS.md) and [CLAUDE.md](../../../CLAUDE.md), and where
one of these blocks disagrees with those files, those files are right and this
page records what was there before. Three of them carried a statement that was
false when it was superseded — the chain length, CT46's stage rows and the
follow-on count's own derivation — which is recorded in
`docs/reports/docs-compact-7/README.md` §4 rather than repaired here.

### 1. `AGENTS.md` headline paragraph — the arbiter chain, as it stood at `2caf7369`

```text
Headline numbers after `W37-CORPUS-CONTRACTS-4` (2026-09-18). The **corpus
baseline is `docs/reports/w37-corpus-contracts-4/artifacts/`** — wave 37's
second arbiter and the chain's second **declarative** link. Beneath it sit wave 37's
two **zero-movement** code arbiters, `joined-provenance-round-bound` (the 64-round
cap replaced by a proved per-graph bound; no archive published) over
`scalar-route-exit-edge` (a loop header's exit edge retained), each of whose AFTER is
byte-identical to `joined-provenance-call-frames/artifacts/` in both verdict-bearing
artifacts. That is why this card's BEFORE, re-run on the rebased tree above both of
them, still reproduces those same two artifacts byte for byte — reproducing all three
cards' AFTER at once, because the three are the same bytes — and it is the measurement
that shows the two code changes beneath moved nothing this corpus can see.
Beneath them, `joined-provenance-call-frames` is wave 36's
second merged arbiter, whose BEFORE reproduces `w36-corpus-contracts-3`'s
`diagnostics.json` and `chunk-contracts.json` byte for byte and which moved
**zero** leaves, zero functions and zero units. Beneath it, `w36-corpus-contracts-3`
is wave 36's first merged arbiter and the chain's first **declarative** link: it changed no crate
source at all, so **one audit binary served both its stages** and the only
difference between them is which modules the contract manifest selects. Its
BEFORE reproduces `joined-provenance-cycles`' `diagnostics.json` and
`chunk-contracts.json` byte for byte. **The chain is what makes each
delta attributable**, and it is one
rule applied at every link: each arbiter's BEFORE run reproduces its
predecessor's verdict-bearing artifacts (`diagnostics.json`,
`chunk-contracts.json`) **byte for byte**, which isolates that arbiter's own
movement from the gateway change beneath it. Since wave 33 the identity is
**asserted inside the archiving tool**, which refuses to write if it ever stops
holding, rather than merely logged. The links, newest first:
`w37-corpus-contracts-4` (wave 37, declarative) over
`joined-provenance-round-bound` (wave 37, zero movement) over
`scalar-route-exit-edge` (wave 37, zero movement) over
`joined-provenance-call-frames` (wave 36, the second of that wave's two arbiters by merge order) over `w36-corpus-contracts-3` (wave 36, the first, declarative) over
`joined-provenance-cycles` (wave 35, the second of that wave's two arbiters)
over `c-gateway-constant-do-while` (wave 35) over `c-gateway-return-call-w4`
(wave 34) over `c-gateway-return-position`
(wave 33) over `w31-corpus-contracts-2` (wave 31, the second of that wave's two
arbiters) over `c-gateway-wave` (wave 31) over `w4-recursive-summaries`
(wave 30). Each arbiter's leaf attribution and gate live in its own report,
which is authoritative for them. All earlier archives remain frozen
(constraint 21): those seven plus `wave23-c-operations`, `aom-destroy-siblings`,
`aom-binding-poison`, `gateway-c-location` and `w29-corpus-contracts`.
Diagnostics retain two coordinates: the presented original
`file:line` and the internal preprocessed line used for pin, leaf and
certificate identity (`GATEWAY-LOCATION-INTERNAL-CHANNELS`, constraint 12a).
```

### 2. `AGENTS.md` ledger row *Cycle 19 raw corpus (185 units)*, as it stood at `2caf7369`

```text
| Cycle 19 raw corpus (185 units) | **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** — **no unit status has moved at any arbiter from wave 31 through wave 36**. 4,834 projected functions: **213 Safe / 84 provisional Unsafe / 4,537 Unknown**. `flb_input_chunk_create` remains function-level **Safe**. Leak **272**, `unknown_alias` **4,309**, `double_release` **0**, `unsupported` **49,753**, `summary_conflict` **1** — all five read from the wave-37 baseline named above. The function and `unsupported` chain, each delta attributed leaf by leaf in its own arbiter's report §5 and narrated in that wave's increment record: `C-GATEWAY-WAVE` (wave 31) four scalar counter helpers Safe → Unknown plus one constant-loop abstraction exposing a provisional repeated release, five function moves; `W31-CORPUS-CONTRACTS-2` **zero** functions and zero units, `unsupported` 50,155 → 50,150 on the four uncontracted-call leaves naming `cio_chunk_get_content_size`/`flb_input_chunk_total_size` decided by two declared contracts, plus one argument-expression refusal retired at its chained-actual site, zero leaves added and two units changed; `C-GATEWAY-RETURN-POSITION` (wave 33) **33 functions Unknown → Safe**, none withdrawn and none made provisional Unsafe, `unsupported` 50,150 → 50,117 as **33 withdrawn return-position refusals and zero added**, the return position widened to admit a pointer-valued bounded typed access chain; `C-GATEWAY-RETURN-CALL-W4` (wave 34) **7 further functions Unknown → Safe** (179 → 186 Safe, 4,570 → 4,563 Unknown, provisional Unsafe 85 unmoved), `unsupported` 50,117 → **50,106** as **17 withdrawn and zero added** — 14 return-expression refusals plus 3 second-order `call to uncontracted function` leaves decided by summaries newly derived from the bodies that card promoted — the position widened again to admit a *modeled call* whose result mode is `None` or `Owned`, `Reallocated` refused by name as the arm that would discharge the replaced parameter's obligation at the exit. `C-GATEWAY-CONSTANT-DO-WHILE` (wave 35) **one function provisional Unsafe → Unknown** (85 → 84 Unsafe, 4,563 → 4,564 Unknown, Safe **186 unmoved**, so nothing is promoted), `unsupported`, `unknown_alias` and leak all **unmoved**, and `double_release` **1 → 0** as **one withdrawn leaf and zero added** — the corpus's only repeated-release row, `rd_kafka_jwt_b64_decode_payload` at internal L21340, **withdrawn** because `do S while (0)` runs `S` once (C11 §6.8.5.2) and the CFG's back edge was a path the C does not have; the function keeps 18 other leaves and so goes to `Unknown`, not `Safe`, and four of the five artifacts are byte-identical with BEFORE. `unknown_alias` moved 4,208 → 4,214 at wave 34, all six added rows `Unknown`-class and in the accusing direction; leak **272** (266 → 272 at `C-GATEWAY-WAVE`: six added, none withdrawn) is unmoved at every re-pin since, constraint 7 carrying that chain in full, and `double_release` stood at **1** from wave 31 until wave 35 withdrew it. `JOINED-PROVENANCE-CYCLES` (wave 35's second arbiter) then moved **zero** leaves, zero functions and zero units, so all four counts above stand as its first arbiter left them: the conditional-provenance relation was extended to cyclic CFGs by a greatest fixed point, is exercised on 40 corpus functions, and is blocked at `release_site`'s nullable-site and naming premises at every one of the 14 sites that reach them. `W36-CORPUS-CONTRACTS-3` (wave 36's second arbiter, and the chain's first **declarative** link) then moved **21 functions and zero units**: **20 Unknown → Safe** (186 → 205 Safe, 4,564 → 4,545 Unknown) as six declared contracts decided the last blocker of twenty projected functions, and **one Safe → Unknown** — `rd_kafka_mem_free`'s own defining function, on the corpus's first **`summary_conflict`** (0 → 1), W4 deriving `Released{nonnull}` where the faithful declaration says `nullable` because `rd_free(NULL)` reaches `free(NULL)`; the declaration governs every call site and declaring `nonnull` to silence it would assert a precondition the body does not have (constraint 4), so this is constraint 8's gate working in the conservative direction, not a defect. `unsupported` 50,106 → **49,922** as **217 withdrawn and 33 added**, of which exactly **204 withdrawn leaves name the six declared callees** — the card's own ex-ante prediction, met exactly — and `unknown_alias` 4,214 → **4,308**, every added row `Unknown`-class and in the accusing direction, which is what modelling a release *does*: once a release is real, a later mention of the released identity is unresolved rather than unexamined. Leak **272 unmoved**, so constraint 7's pin needed no re-pin, and `chunk-contracts.json` is byte-identical across that card's two stages. Three predicted function moves did **not** happen: the `cJSON_Delete` trio exchanged its decided leaf for a `nested call … inside a modeled call` refusal that was unreachable while the outer call was uncontracted, which makes a sole-blocker cover an **upper bound** and never a forecast (`W36-COVER-HIDDEN-BLOCKER`, `CJSON-DELETE-NESTED-DETACH`). Provisional findings remain subject to the reports' coverage and reachability boundaries; the production `input_chunk_get` residual stays Unknown `JOINED-PROVENANCE-CALL-FRAMES` then moved **zero** leaves, zero functions and zero units — its AFTER is byte-identical to its BEFORE — so all counts above stand as `W36-CORPUS-CONTRACTS-3` left them: conditional provenance is now preserved across a call the frame shows it cannot write, which takes `release_site`'s naming premise from 0 of 14 to **6 of 14**, while the nullable-site premise still blocks all 14. `SCALAR-ROUTE-EXIT-EDGE` (wave 37's first arbiter) then moved **zero** leaves, zero functions and zero units — its AFTER is byte-identical to its BEFORE and to the wave-36 baseline in both verdict-bearing artifacts — so all counts above stand: the pre-existing value route's deletion of a loop header's **exit** edge, which returns a false `Safe` for a leak, a double release or a release after escape placed after the loop and does so against well-defined C for a `volatile` flag, is refused at the structural level, and the corpus is measured to reach neither deciding arm at any of its **201** loop-header `BranchOnNull` nodes — 89.6% of those edges landing in the route's `untracked` early return, which is a coverage fact and not a safety one (`SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES`) `W37-CORPUS-CONTRACTS-4` (wave 37's second arbiter, and the chain's **second declarative** link) then moved **8 functions and zero units**, all **`Unknown` → `Safe`** (205 → 213 Safe, 4,545 → 4,537 Unknown, provisional Unsafe 84 unmoved) and **none lost**, as ten declared contracts decided the last blocker of eight projected functions. `unsupported` 49,922 → **49,753** as **183 withdrawn and 15 added**, of which exactly **179 withdrawn leaves name the ten declared callees** — the card's own ex-ante prediction, met exactly — and `unknown_alias` 4,308 → **4,309**. The added-leaf figure is the card's strongest measurement and is not a raw count: **all 15 added rows are same-position TEXT SUBSTITUTIONS and none is a genuine addition**, so no position in the corpus refuses that did not refuse before. Leak **272 unmoved** a second consecutive declarative arbiter, `summary_conflict` **1 unmoved** — no new conflict, because every one of the ten declarations is the **neutral element** (`N(m)`: every formal borrowed, result none), which a faithful summary of a body that releases nothing cannot contradict — and `chunk-contracts.json`, `source.json` and `source-effects.json` are all byte-identical across that card's two stages. Three of its eleven predicted function moves did not happen, and the card **predicted one of them ex ante**: `pack_string` exchanged its decided leaf for `nested call `cfl_sds_len` inside a modeled call`, forecast by reading the body — naming the mechanism, the function and the inner callee — which converts `W36-COVER-HIDDEN-BLOCKER` from a surprise into a prediction (`NESTED-CALL-IN-ARGUMENT-POSITION`). The other two were **not** hidden blockers but a **withdrawal**: `rd_kafka_topic_partition_list_sort`'s C declaration spells its comparator as an **inline function-pointer parameter declarator**, which the ABI reader does not parse — it resolves only the function-pointer *typedef* form card `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` taught it — so constraint 8 withdraws the declaration at every call site with `abi-unreadable-declaration`, **leaf-neutrally** (all 11 rows are same-position substitutions, no function's leaf count changed). That is the gate failing closed on a *reader* boundary rather than on a disagreement (`LOADER-ABI-INLINE-FNPTR-DECLARATOR`). The card also records that after wave 36's harvest the sole-blocker cover frontier is dominated by unit-internal `static` helpers, where W4 and not declaration is the lever (`W37-COVER-FRONTIER-IS-STATIC`), which is why it lands **ten custody-neutral declarations and not one release**, the exact inverse of its predecessor |
```

### 3. `AGENTS.md` ledger row *CT01–CT50 conformance*, as it stood at `2caf7369`

```text
| CT01–CT50 conformance | **46/50 mapped**, 4 unmapped; sanitized-native **10 of 46** (wave 32's `CT-COMPOSED-NATIVE` gave the `composed_epoch` route a runner that really compiles and runs an admitted case's own emitted C under the platform sanitizers and compares its stdout byte for byte with that module's own checked task half, moving CT22's and CT25's emission/native/sanitized rows off `unassessed`; CT23 stays out, being the route's refusal, and every arm of the runner was falsified before it was believed); **proof 3 of 46** — CT01 (wave 29, `CT01-PROOF-LEG`), CT37 (wave 30, `CT-PROOF-LEG-2`, chosen by a measured survey of all 42 mapped cases) and CT03 (wave 31, `CT03-PROOF-ALPHABET`, the first leg to need the F02 alphabet itself to grow — by the Buffer write, proved **conservative** so that a write-free trace relates exactly as before and no existing digest moved — while the *sibling* arm of that same alphabet decision, the two lease events still refused, was at the time **observed and not proved**, the occasioning case for `CLAUDE.md`'s `STABILITY-REEVAL` cadence; wave 32's `TRACE-ALPHABET-LEASE-BOUNDARY` **discharged** that residual and constraint 15n now carries the measured statement — the arm is **reachable and driven**, dropping is proved an *admitting* mistake rather than a lossy one, and the refusal's index is proved to be the first offending event's own, so this row no longer says "no test drives it" (the forward question, whether the observable alphabet should grow once a worker exercises its authority, stays open as `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`)): each case's scenario property proved in Lean of the checked artifact's reference run and its admitted renderings (CT37's completion strictly before the join in every admitted rendering), bound to the case by a named host-unconditional test through the F02 digest relation, native execution deliberately observation-only; the per-case gate is the pinned fail-closed `PROOF_GATED_CASES` table in `ct_corpus.rs`, and each flip's stage evidence supersedes card T00's frozen archive from its own record (constraint 21). Wave 28 (`CT-WIRE-LANE-2`) mapped CT07, CT08, CT09, CT21 and CT23 (CT07 stating the read-loan/retained-write boundary; CT09's layout leg an executed *observation*; CT23 on the `composed_epoch` route); wave 29 (`CT22-WIRE`) mapped **CT22** on `REGION-CREDIT-JOINS`' evidence with the wave-28 narrowed reason retired in its own record; wave 30 (`CT07-WRITE-REMAP`) upgraded **CT07's evidence** onto the write-lend scenario — two write authorities over disjoint ranges of one buffer, both held by one sequential entry's bindings, **not two threads** (`WRITE-LEND-TASKS` the residual) — superseding the wave-28 record's CT07 entry from its own record with **no count moved** (evidence quality, not coverage). Wave 31 (`CT-WIRE-LANE-3`) mapped **CT25** — *cleanup destroys owner after joins and loan expiry*, every word carried by an unedited model's checked fact — and **examined CT24 and withdrew it** with the measurement published: on the only route with an executed cleanup-destroy a region before the join refuses for its *placement* whatever its duties, so no control isolates the destroy, and mapping it would move the derived reasons map's keys against a matrix the lane may not regenerate (constraint 25, measured rather than assumed). Wave 32 (`CT-WIRE-LANE-4`) mapped **CT47 and CT44**: CT47 on the transferability judgment's refusal at the crossing construct — verified in source that the owned buffer is `MovedOnly` and *sendable*, so the refusal is for the **mode** and not a vacuous kind refusal, with all three contracts the kind does carry supplied as admitting controls — and CT44 after measuring both of card T00's archived blockers **obsolete** (the refusal names the borrowing task, and "environment moves before join" now has a source form), attribution separated in both directions. Each publishes the boundary its scenario's untranslatable words rest on (no pointer type; the storage is heap-bound, so "stack" has no referent). Wave 35 (`CT-CTWIN-LANE`) mapped **CT46** on card `MUTEX-SHARE-CROSS`'s composition, under a new `checked_mutex_crossing` verdict mode on the `mutex_guard` route — the corpus's first **admitted** case that deliberately stops at the checker: the module is accepted and the C backend renders nothing for it, `interpret_mutex_program` refusing the `Spawn` the crossing introduces, which is `MUTEX-SHARE-CROSS-EMISSION` measured rather than inherited and is why four stages read `unassessed` and none reads `executed`. The emission boundary is pinned in both directions, one control being the same route without the clause, which does render and does interpret. Two of CT46's own words are published as untranslatable — there is no wrapper type and no `Send`/`Sync` trait — both carried instead by the transferability judgment's `TransferKind::ProtectedResource`, asserted against `MUTEX_SHARE_CROSSING` itself. The same lane **re-measured the four remaining cases and mapped none**: CT41's obstruction is **structural** — clause (b) of the wave-9 attribution rule needs a control in which removing `volatile` moves the verdict, and `CTWIN-PUBLISH` §5's decision is that it moves nothing — with the scenario's own consumer shape now measured **`safe` with no diagnostic**, and the typed-source arm examined for the first time and found unable to tell `volatile` from `_Atomic` (`CT41-TYPED-SOURCE-UNDISCRIMINATING`); CT42 stays unmapped with **both** arms measured, the socket arm being **unreachable from source** (`CT42-CELL-UNROUTED`); CT10 stays unmapped on half a scenario; CT24 stays withdrawn. Wave 37 (`CT-LANE-5`) re-mapped **CT07** off the sequential write lend onto the two-thread range write lease — the scenario's word `independently` resting on two threads for the first time, with a sanitized native run and an allocation-identity witness behind it — superseding the wave-30 record from its own record with **no count moved**, and published the measurement for CT42, CT10 and CT41, all three still unmapped. A grown denominator is mapping, not conformance, and a grown proof numerator is evidence about one case's artifact (constraint 23) |
```

### 4. `AGENTS.md` ledger row *Contract corpus*, as it stood at `2caf7369`

```text
| Contract corpus | **88 modules** (73 → 74 at wave 29: `fluent_bit/input_chunk_sizes.qcore`; 74 → 76 at wave 31: `chunkio/chunk_sizes.qcore` and `fluent_bit/input_chunk_total_size.qcore`; 76 → 82 at wave 36 (`W36-CORPUS-CONTRACTS-3`), six declarations chosen by **function-cover yield** over the 193 baseline functions blocked only by uncontracted-call leaves — `cfl/sds_lifetime.qcore`, `cmetrics/context_lifetime.qcore`, `ctraces/context_lifetime.qcore` (the corpus's first `ctraces` family), `librdkafka/cjson.qcore`, `librdkafka/kafka_mem.qcore` and `monkey/logging.qcore`, the last carrying this corpus's second `resource_variadic_data_tail()` — each deliberate and each read line-cited from the pinned body before it was declared; 82 → 88 at wave 37 (`W37-CORPUS-CONTRACTS-4`), six modules carrying **ten** declarations chosen by the same function-cover yield over the 163 remaining such functions — `librdkafka/topic_partition.qcore`, `mpack/writer.qcore` (the corpus's first `mpack` family, deliberately **not** the existing `msgpack/`, which is a different library), `cmetrics/metric_values.qcore`, `cmetrics/mpack_utils.qcore`, `cfl/time.qcore` and `librdkafka/lz4_stream.qcore` — **not one of them a release**, every one the neutral element, each read line-cited from its own pinned body before it was declared; resource-v1 declarations 39 → 41 → 43 → 49 → **59**, manifest-selected modules 18 → 19 → 21 → 27 → **33**) |
```

### 5. `AGENTS.md` ledger row *Merged gate*, as it stood at `2caf7369`

```text
| Merged gate | **4,611 passed / 0 failed / 8 ignored, 229 test executables / 231 result summaries** · 122 provider (carried; **Unix** — see below) · **293 Python** · **137 Lean audits** · both probes exit 0 · fmt/clippy clean. Measured by the **wave-36 close** at its own integration tip (base `fa48211a`, the `WRITE-LEND-TASKS-EXERCISE` merge, plus the close's own commits — so the one code change the close makes, the reconciliation sweep's content-verifying `--check` arm, is *inside* this measurement rather than beside it), `git submodule update --init --recursive` run first and exit 0 (`WORKTREE-SUBMODULE-UNGATED`); one uninterrupted `cargo test --workspace --no-fail-fast` of 21m24s (a full rebuild; see the close's §11), **every step's exit captured to its own file with no pipe in the command** (`GATE-EXIT-SWALLOWED-BY-PIPE`). Per-command exits in `docs/reports/wave36-close/gate.json`, and each card's own gate in its own report. **A gate figure is a measurement on one machine, never a property**, and four qualifications travel with this one. First, constraint 16's rows, because that constraint requires a gate report to **state which it saw**: the sixth row `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` was **green here**, as were `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` and `memory_growth_beyond_the_declared_limit_is_bounded` — but this is a full-suite run on a machine carrying its own build, **not** the quiet-host isolated number `HOST-ROW-STARVATION-DESCRIPTION` still asks for, and it is not offered as one. Second, and new this wave, `a_forced_override_never_weakens_an_execution` was **also green here** and is the **unnamed seventh** row: card `C-GATEWAY-INDIRECT-CENSUS-2` measured it red under two concurrent corpus audits, so constraint 16's list is now recorded **known incomplete** rather than assumed complete (`LOAD-SENSITIVE-ROW-UNNAMED`). Third, the Python figure is the **re-run after a deliberate red**: editing `named_followups` by hand left `dimensions.named_followups` stale and `CommittedMatrixIsSelfConsistent` failed with `171 != 187`, which is `D02-DIMENSIONS-FOLLOWUP-COUNT`'s tripwire working; the repair was to regenerate through `build_matrix.py`, never to touch the count. Fourth, provider **122 is carried, not re-measured** (`git diff --name-only 60958bf0^ HEAD -- providers/ crates/qcore-schemas/` is empty and `qcore-schemas` is still 14.0.0, so constraint 18 is not engaged) **and is a Unix figure** (`PROVIDER-CONFORMANCE-UNIX-ONLY`). Prior figures, for the delta only: 4,580 / 0 / 8 at the wave-35 close, 4,559 at wave 34, 4,544 at wave 33, 4,534 at wave 32; the +31 is wholly the wave's own cards, this close adding no cargo test and all **19** of its new tests being Python |
```

### 6. `AGENTS.md` follow-on count paragraph, as it stood at `2caf7369`

```text
follow-ons in `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`,
**213** entries at the wave-36 close (171 → 189 → 190 → **187**: the **18**
residual-shaped names its sweep found reaching no tracker — all nine of
`W36-CORPUS-CONTRACTS-3`'s §9 and all nine of `C-GATEWAY-INDIRECT-CENSUS-2`'s,
both cards having left them to the close deliberately — plus the close's own
`DECISION-ALSO-REGISTERED-AS-FOLLOWUP`, less **three retirements, each named and
each checked to be a registered entry before being counted**:
`CARD-SCOPE-SELF-MERGE` and `MAIN-GUARD-UNACTIVATED`, both discharged by the
integrator activating the guard, and `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`,
discharged by the close's own `--check` content arm. Two further "retirements" in
the wave's own cards were **no-ops** — `JOINED-PROVENANCE-CALL-FRAMES` and
`WRITE-LEND-TASKS-EXERCISE` were never matrix entries — and are recorded as such
rather than counted).
The wave's own chain: 171 after WRITE-LEND-TASKS-EXERCISE (+7 −1), 165 after
MUTEX-SHARE-CROSS-EMISSION, 161 after JOINED-PROVENANCE-CALL-FRAMES (+3 −2),
159 after STABILITY-REEVAL #2 (+31), 128 after CARD-SCOPE-GUARD (+7), from
**121** at the wave-35 close, which itself registered **28** names — 27 of the
62 its widened sweep returned, plus `DOCS-COMPACT-4-TABLE-SYNC` — and retired
**10** as discharged by named cards; the section is *carried*,
`dimensions.named_followups` is
*derived*, and the two are checked against each other on the committed file, so
add entries through `docs/reports/remaining-d02/build_matrix.py` and never by
hand — constraint 25) plus each report's own follow-on list; per-card evidence in
```

### 7. `CLAUDE.md` compaction-cadence roster and `STABILITY-REEVAL` pointer, as they stood at `2caf7369`

```text
  (DOCS-COMPACT 2026-09-10, DOCS-COMPACT-2 2026-09-11, DOCS-COMPACT-3
  2026-09-12 after wave 26, DOCS-COMPACT-4 2026-09-13 after wave 29,
  DOCS-COMPACT-5 2026-09-13 after wave 31, DOCS-COMPACT-6 2026-09-17 in
  wave 34; next due around wave 36) re-reads every headline

  proof: a recurring **`STABILITY-REEVAL`** card (#1 ran in wave 34,
  `docs/reports/stability-reeval-1/README.md`; next due after wave 35)
```

