# STABILITY-REEVAL #3 — the cadence's third firing

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


**Card:** `STABILITY-REEVAL` #3, wave 38. **Base:** `e69083a6` (`origin/main`
at branch time, confirmed equal by `git rev-parse`; over `0f6eb0a6`, the
wave-36 close). **Method:** `CLAUDE.md`'s four steps, over waves 36–37.
**Baselines:** `docs/reports/stability-reeval-1/README.md`,
`docs/reports/stability-reeval-2/README.md`.

This card is an **audit**. It weakens no gate, edits no frozen archive, changes
no verdict rule and lands no semantic change. It lands **one test** — a
tripwire that makes a by-name binding checked, shown red by mutation (§2.5) —
and **fourteen residuals**, registered with discharge conditions. Every claim
below is labelled **MEASURED** (a command was run or a line was read, and is
cited) or **INFERRED**; where a boundary is proved on one side and observed on
the other, this report says so in those words. Every number is from a command
this card ran; every log cited was listed before it was cited. Five read-only
sub-audits ran as delegated readers in this worktree with cargo forbidden while
the gate ran; each conclusion used here was re-verified against the cited line
by this card, or is attributed to the sub-audit in so many words with its
private log named.

The first act of this run was to commit a report skeleton (`6368cc7b`), on
firing #2's own lesson.

---

## 1. Tally over firing #2's 47 residuals

**10 discharged · 14 partial · 23 open** at the base, over the 47 entries
whose `recorded_by` starts `STABILITY-REEVAL` (19 from firing #1, 28 from
firing #2). A 48th, `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`, is **absent
from the matrix** because the wave-36 close retired it against its own
discharge — the `--check` content arm, still red by mutation at this base
(§2.6) — and is counted as discharged outside the 47. Each verdict is one
sub-audit line re-checked by this card at the cited location; the split is
**this card's**, not carried.

**Discharged (10):** `LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED` (15n now says
three of five), `GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION` (`AGENTS.md` repo-map
`proof/` row and constraint 28), `RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED`
(5b "conditional on `TrackedBasesAreBound`"), `LEDGER-CT-ROW-LEASE-ARM-STALE`
(CT row rewritten; `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` gone from
`AGENTS.md`), `PROVIDER-CONFORMANCE-UNIX-ONLY` (gate row: "a Unix figure"),
`LEDGER-BINARY-COUNT-CORRECTED-IN-ONE-FILE-ONLY`
(`wave33-close/binary-count-provenance.md` beside the frozen record),
`WAVE-33-RECORD-FUSED-PARAGRAPH` (blank line present),
`DANGLING-THEOREM-CITATION-AFTER-LEAN-SWEEP` (`lean-proof-integrity` names both
reports), `STABILITY-REEVAL-1-RESIDUALS-ORPHANED` (`CLAUDE.md`'s re-homing
rule landed as a before/after `--residuals` comparison — a wording variation,
not a softening — and both orphans are registered), `FB-PLAN-STALE-RED-ASSERTION`
(the plan restates the row as a measured environment boundary).

**Partial (14):** `CT-SHARED-FIXTURE-DISCLOSURE` (43-distinct-artifacts
sentence in `wave-34.md`; no ledger clause, `ct_manifest.json` still carries
`digest` zero times), `WINDOWS-COHERENCE-GATES-UNCARDED` (exonerated in
firing #1's own paragraph; no retrospective record, `wave34-close` still says
"a live instance"), `RETURN-POSITION-ESCAPE-UNREACHABLE` (boundary stated in
5b; tracker unchanged), `JPC-LEDGER-44-21-STALE` (row now 40 / 6-of-14 / 14;
the "which tree each figure was measured on" half absent),
`JPC-TRANSFERSOUND-UNNAMED-PREMISE` (`TransferSound` named in
`joined-provenance-call-frames` for the call-edge arm only; `AGENTS.md` still
silent), `FOREVER-FOR-EXIT-EDGE-UNOWNED` (5c narrowed to `while (1)` and the
route recorded; `a_forever_for_is_untouched_by_this_card` pins `for (;;)`
**without** a `break` — no test spells `for (;;)` with one, §2.1),
`GATE-EXIT-SWALLOWED-BY-PIPE` (both closes captured every exit to a file, and
`AGENTS.md`'s gate section mandates it; no runner script, no `pipefail`, and
`gate.json` still carries no cargo-test exit), `CT10-REASON-ROW` and
`MATRIX-UPDATED-STALE-AT-WAVE-27` (deferred with grounds at two closes; neither
taken nor declined), `JPC-PREMISE-CENSUS-BLIND-TO-CYCLICITY` (the call-frames
archive now carries `function` and `cyclic` per line — 10 cyclic / 4 acyclic
visits — but no report publishes the split in those words),
`RELEASE-SITE-PREMISES-PARTIALLY-INSTRUMENTED` (all seven disjuncts named in
`call-frames/scripts/instrument.py:363-375`; the four never-firing ones have no
unit test), `JPC-CENSUS-CONTRACT-SET-DEPENDENT` (confirmed measured by
`BASE-TYPE-READER-CENSUS`; populations 1–3 not re-indexed),
`LOADER-ABI-MODES-CORPUS-PROPERTY` (`W37-CORPUS-CONTRACTS-4` §3.3 states the
ground before measuring; no derivable-comparison census published),
`WAVE-36-CARDS-UNCHARTERED` (moot for wave 36; wave 38's reading list is in
the wave-37 close, no brief page exists — and §2.7's
`WAVE-38-QUEUE-NAMES-UNTRACKED` is the same shape one wave on).

**Open (23):** `LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED`,
`AUDITALL-TRIPWIRE-CONTROLS-UNCOMMITTED`,
`CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS`,
`LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`, `GATEWAY-CONSERVATIVE-UNATTACHED`,
`F02-DIGEST-PIN-DUPLICATED` (**escalated**, §1.2),
`LEAN-STATEMENT-SEMANTIC-REVIEW`, `RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`,
`JPC-OFF-CYCLE-THEOREM-ACYCLIC-ONLY` (`JoinedProvenanceCycles.lean:153-157`
unchanged since `0526d8d7`; the round-bound card touched the sibling module
only), `JPC-UNIQUENESS-ENTRY-AGREEMENT-UNSTATED`,
`DO-WHILE-ACYCLICITY-SCRIPT-PREMISE-STALE` (the wave-36 close recorded the
postponement; no note beside the script), `JPC-40-UNARCHIVED`,
`JPC-CONSUMPTION-ONLY-IN-EXIT-CUSTODY`, `RETURN-POSITION-PREMISE-CONTRACT-ROUTE`,
`RETURN-CALL-WHOLE-EXPRESSION-SINGLE-WITNESS`,
`RETURN-CALL-TESTS-CLANG-GUARDED-GREEN` (§2.7 measures the family at 750),
`RETURN-CALL-PIN-CONTROL-UNANCHORED`, `CORPUS-TOTALS-UNPINNED-BY-ANY-TEST`,
`INTEGRATOR-HEADLINE-COUNT-HAND-WRITTEN-UNCHECKED` (the close's `lean_audits`
key checks a *different* figure; the follow-on count is still hand-written and
checked by nothing — this card wrote it by hand too, from the generator's
printed `named followups 229`), `POINTER-EMPTY-TARGETS-INVARIANT-UNSTATED`,
`ABSTRACT-POINTER-DEFAULT-IS-THE-UNSOUND-TRIPLE`,
`SINCE-EMPTY-WINDOW-PROPERTY-REHOMED`, `CT-STAGE-TALLY-LITERAL-UNDISCLOSED`.

### 1.1 The pattern, restated with this firing's numbers

**MEASURED.** Of the 47, **18** needed a mechanism (a test, a control, a
script, a proof) and **29** needed a sentence. Mechanism: **0 discharged, 3
partial, 15 open**. Sentence: **10 discharged, 11 partial, 8 open**. Firing
#2's pattern holds and sharpens: across two waves and 47 entries, **not one
mechanism-needing residual was discharged**, and the three mechanism partials
(`GATE-EXIT-SWALLOWED-BY-PIPE`, `JPC-PREMISE-CENSUS-BLIND-TO-CYCLICITY`,
`RELEASE-SITE-PREMISES-PARTIALLY-INSTRUMENTED`) were side effects of cards
working next door, not of anyone reading the entry. One refinement the sub-audit
over entries 25–47 measured and this card confirms: the only two of *its*
discharges (`STABILITY-REEVAL-1-RESIDUALS-ORPHANED`,
`FB-PLAN-STALE-RED-ASSERTION`) are exactly the two items that were also on
firing #2's **integrator reconciliation list** and were verdicted `applied` by
the wave-36 sweep. Every residual that lived **only** in `named_followups` —
sentence or mechanism — received no discharge. The sweep discharges; the
tracker does not. That is the process finding of this firing, offered as
firing #2 offered its: a residual competes with the next wave's cards unless a
close is made to look at it.

### 1.2 `F02-DIGEST-PIN-DUPLICATED` — the predicted drift has happened, on Windows, invisibly

**MEASURED**, re-verified line by line after a sub-audit raised it.
`compiler/tests/f02_trace_relation.rs:96-97` pins `bfa572914cb96413` /
`9cbe67a70a420d52` — by its own comment the **fifth** recorded move, card
`WRITE-LEND-TASKS-EXERCISE`, 2026-09-18. `compiler/tests/windows_threads.rs:31,36`
pins `6df369991465940e` / `0da423be2f16b8bb` — the fourth-move values
(`write-lend-tasks/README.md:247`) — under the comment "Keep these pins paired
with `f02_trace_relation.rs`". Both files compute the same FNV-1a (offset
`0xcbf29ce484222325`, prime `0x100000001b3`; `f02_trace_relation.rs:101-108`,
`windows_threads.rs:48-53`) over `diagnostic_sink::instrument(module.c_source)`
of the same two programs; the Windows one normalises CRLF to LF first, so it
is the Linux function over the Linux bytes. Wherever emitted bytes are
platform-independent — which the pairing comment presumes and the Linux row
proves for the LF bytes — the Windows test
`windows_f02_renderings_satisfy_the_incoming_trace_relation_and_emission_pins`
asserts a digest the emitter no longer produces and is **red on every Windows
host since `fa48211a`**. The re-pinning card's §4 does not mention
`windows_threads.rs`; `rg` finds the four literals in exactly the two files
and two reports. The last edit of the Windows file was `37bc9004`, the
uncarded commit `WINDOWS-COHERENCE-GATES-UNCARDED` names — so the previous
repair of this exact pair was itself the uncarded edit firing #1 found. **Not
taken by this card:** re-pinning a Windows expectation without a Windows run
is `37bc9004`'s shape. Registered as `F02-WINDOWS-PIN-STALE-SINCE-WAVE-36`
with the two correct literals in its discharge; the original entry stays open,
its "hazard" now a live red row.

---

## 2. Step 1 — vacuous greenness in waves 36–37's fail-closed arms

### 2.1 `SCALAR-ROUTE-EXIT-EDGE` (`0b0b4ac1`) — the retained-exit arm is driven; the back edge is now a *named* instance of the shape #2 found the exit edge in

**MEASURED, reachability.** The rule has exactly one reader of the new fact
and one reporter:

* `loop_exit_edge` is **assigned** at three loop-header sites
  (`legacy_c.rs:5148` `while`, `:5186` `for` — `(!condition_always_true).then_some(1)`,
  so `for (;;)` records `None`, `:5230` `do`-`while`), read by the `Infeasible`
  arm at `:6527` (`PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge)`)
  and by `report_retained_loop_exit_edges` at `:6178-6185`, which is called
  **once**, at `:6576`, from the settled states. `rg` finds no other reader in
  the tree; `legacy_recursion.rs:432` and the three fixture closures in
  `legacy_joined_provenance.rs` (`:558`, `:603`, `:978`) only *initialise* it.
* The arm is driven end-to-end. `crates/qcore-c-gateway/tests/scalar_route_exit_edge.rs`
  (11 tests, 484 lines) asserts on the leak, the double release and the
  retained-edge refusal rather than on a proxy, and the card **ships both its
  harness and its log** — `docs/reports/scalar-route-exit-edge/scripts/mutate.py`
  and `mutations.log` (`find`, `ls`), the log naming the red tests per
  mutation: M1 (prune again) and M5 (protect the body edge instead) redden the
  same five, including `the_retained_edge_carries_the_state_the_header_had`
  and `a_retained_exit_edge_still_reports_a_double_release_after_the_loop`; M6
  (report inside the work list) reddens exactly
  `a_body_that_writes_the_deciding_scalar_reaches_the_unknown_arm`; the log
  ends `restored: sha256 EQUAL exit=0`. **The fixture coverage of the
  retained-exit arm is real.** This is the standard
  `LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED` asked for, met.

**MEASURED, the back edge.** `SCALAR-ROUTE-BACK-EDGE-UNDEFENDED` is registered
(`recorded_by: SCALAR-ROUTE-EXIT-EDGE`) with the `volatile` witness stated in
its `why` — `volatile int flag = 0; while (flag) { q_free(p); q_free(p); }`
measured `Safe`, a double release **inside** the body swallowed. The one test
touching the arm, `the_back_edge_arm_is_unchanged` (`:308-339`), pins the
**deletion** — `("int flag = 0;", false)` asserts `double_releases() >= 1` is
*false* for `do { q_free(p); } while (flag);` — i.e. it pins the admitting
behaviour on a non-`volatile` flag, where the deletion is exact. The `volatile`
witness itself is **in the report and the matrix only**: `rg volatile
crates/qcore-c-gateway/tests/` finds it only in `a_volatile_flag_loop_is_not_safe`,
which is the *exit* edge. So the answer to the brief's question is **yes: the
back edge is now exactly the shape firing #2 found the exit edge in** — a
value-route deletion of a CFG edge resting on the unwritten
no-asynchronous-modification premise (`GATEWAY-VOLATILE-PREMISE`, registered),
with a known false-`Safe` witness measured once and not pinned, and a corpus
reach of 0 of 201 that is a coverage fact (`SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES`)
— **with one difference that matters: it is named, registered, and its
discharge condition already asks for the pinning test.** No new residual:
naming it twice would be the orphaning shape §1.2 of firing #2 recorded. This
card deliberately does **not** add a test asserting the `volatile` false
`Safe` as a "measured boundary" pin — a pin in the admitting direction on a
known false `Safe` is a pinned expectation invariant 5 would then protect in
the wrong direction, and the residual's own discharge is the right home for
the test.

**MEASURED, the `for (;;)` neighbour.** `a_forever_for_is_untouched_by_this_card`
(`:377-396`) pins `for (;;) { }` with no `break` as `Safe` with zero retained
edges (exact: no exit exists), and `a_break_in_the_body_answers_as_before`
(`:428-449`) drives `break;`, a conditional `break` and `goto out;` — on a
`while (!flag)` header, **not** on `for (;;)`; `rg 'for \(;;\) \{[^}]*break'`
over the gateway tests finds nothing. Hence the partial verdict in §1.

**Proved vs observed, in those words.** *Proved:*
`QCore.ScalarRouteExitEdge.the_exit_edge_is_never_removed` and its
accusing-direction companions, of the abstract rule. *Observed:* that
`legacy_c.rs:6527` is the only site where a modelled value can remove a loop
header's exit edge (the card's census, run rather than argued, over the pinned
corpus), and the `volatile` premise itself.

### 2.2 `JOINED-PROVENANCE-ROUND-BOUND` (`797661f3`) — which side is proved, which observed, and what no test drives

**MEASURED, the arms.** `iterate` (`legacy_joined_provenance.rs:334`) returns
`Settled::Converged { rounds, solution }` or `Settled::Declined` (`:318-325`,
`:389`); `fixed_point` maps `Declined` to the vector it always returned
(`:461`). The **declining arm is driven directly** at `iterate` by four tests
that pass a deliberately short cap (`:647`, `:711`, `:769`, `:782`, each
asserting `Settled::Declined`), so the arm's fail-closed *mapping* is not
vacuous at the unit level. It is unreachable **through `derive`** by the
theorem — `JOINED-PROVENANCE-DECLINE-ARM-UNREACHABLE` (registered) records
that the one surviving mutation of seven is exactly a decline that publishes a
forged binding. That survivor is a genuine boundary, correctly named; nothing
is added to it here.

**Proved vs observed, in those words.**
*Proved:* `QCore.JoinedProvenanceRoundBound.a_cap_at_the_measure_settles` — a
cap at or above `nodes × (1 + sites + sites × names)` never declines — **of an
abstract sweep, under the hypothesis that every non-fixed sweep strictly
decreases the module's measure.** *Observed:* that the Rust transfer
(`Facts::step`'s four arms, `outgoing`, the null-edge exclusion) satisfies that
hypothesis. This is `JOINED-PROVENANCE-BOUND-DESCENT-PREMISE`, registered by
the card in those words ("stated of, not proved of, the Rust transfer"), and
this card confirms the classification: the card's own report records that the
*naive* measure was false at the arrival step (its §6), which is exactly what
makes the premise non-idle. The corpus evidence on the observed side is that
all 159 sweeping visits converged within their bounds (max 148 rounds against
a median bound of 378) — evidence about 159 graphs.

**MEASURED, the `Settled`-with-facts outcome.** One test drives it past the old
cap: `a_graph_needing_more_than_sixty_four_rounds_now_derives_its_fact`
(`:622-655`) builds `long_cyclic_cfg(120)`, asserts `rounds > 64`, asserts the
flat cap of 64 **declines** the same graph, and asserts `derive` publishes site
7's fact at the exit — both halves pinned. For the **four corpus functions**
that now derive (nine facts), **no test drives anything**: their facts are
published where nothing consumes them (`release_site` is the relation's only
consumer and none of the four reaches it, the card's §5), and the figure comes
from the off-by-default `provenance-rounds-census` feature that nothing builds
— `INDIRECT-CENSUS-FEATURE-UNBUILT`, already restated by the wave-37 close with
this card's sibling feature as its second instance. "Nine facts across four
functions" is a **measured-once** figure behind an unbuilt feature, not a tree
fact; recorded as the boundary, no new name.

**MEASURED, the missing archive.** The card's report directory contains
`README.md` and nothing else (`ls -la`: one file). Its seven-mutation harness
lives at `measure/joined-provenance-round-bound/mutations.py` — a gitignored
path — and no `mutations.log` is shipped, so the "six of seven caught" table
in its §7 is checkable by nobody after the merge. `DOCS-COMPACT-7` noticed the
same absence from the other side ("eleven of the twelve published an archive,
`joined-provenance-round-bound` having published none"). Registered in §2.7 as
part of `W36-W37-MUTATION-HARNESSES-UNSHIPPED`.

### 2.3 `W37-CORPUS-CONTRACTS-4` (`2caf7369`) — the withdrawal arm's first production firing, and what the leaf-neutrality claim rests on

**MEASURED** (sub-audit, re-verified at the cited lines). The arm is
`legacy_declared_abi.rs:577-579` — `compare` returns
`AbiRefusal::UnreadableDeclaration` when `project_parameter` (`:1012-1014`)
returns `None` for a parameter group containing `(`/`)`, the inline
`int (*cmp)(…)` declarator. **The transition:** the arm was fixture-driven
before this card — `crates/qcore-c-gateway/tests/loader_abi.rs:500-504` is the
row "an *inline* function-pointer parameter is a nested declarator, still not
read" (`void k_use(void (*p)(int), int n);` → `UnreadableDeclaration`), added
at `5f3c3cf7` (2026-09-10), eight days before the merge, inside
`the_admission_and_refusal_table_decides_row_by_row`. So
`LOADER-ABI-INLINE-FNPTR-DECLARATOR` records a move from *fixture-verified* to
*corpus-observed*, not from unverified to verified — and its eventual
discharge (reading the inline form) must flip that pinned row's outcome
**deliberately** (invariant 5), not around it.

**What "leaf-neutral, 11 same-position substitutions" rests on.** The card
archives raw artifacts, `scripts/attribution.py` and `logs/attribution.txt`,
and its §5.1 says the rows were "verified as a set equality, not by counting"
(`:437-440`). `attribution.py:88-95` classifies a substitution by
**per-position leaf-count equality**, and `census.py` lists rows; **neither
computes the position sets the sentence names.** The sub-audit recomputed them
from the archived artifacts (private `recompute_substitution.log`, `ls`'d):
the 9 BEFORE uncontracted-call positions are a subset of the 11 AFTER
withdrawal positions; the two extras (`rd_kafka_assignment_add` L20630,
`rd_kafka_share_assignment_add` L20567) carried `indirect memory access` before;
origins agree and per-position counts are equal at all 11. **The claim is
true; its archived support is prose plus a counting script.** Registered as
`W37-SUBSTITUTION-SET-EQUALITY-UNARCHIVED`.

### 2.4 `TASK-ROUTE-TSAN` (`3fb52b8c`) — the boundary between "cannot run" and "runs clean", stated precisely

**MEASURED** (sub-audit; the arms and both test bodies re-read by this card).
Two tests assert opposite-sounding things about one route and both are green:

| Test | File | Build | What it asserts |
|---|---|---|---|
| `thread_sanitizer_cannot_run_this_routes_programs_lease_or_not` (wave 36) | `compiler/tests/write_lend_tasks_exercise.rs:735` | **unbound** (`-fsanitize=thread -include track.h`, no pthread binding) | both programs die and both stderrs contain `SEGV` |
| `every_subject_dies_unbound_at_the_one_fault` (wave 37) | `compiler/tests/task_route_tsan.rs:237` | **unbound** | the `0x000000000018` fault, `races == 0`, five subjects |
| `every_subject_runs_clean_under_thread_sanitizer_when_bound` (wave 37) | `task_route_tsan.rs:262` | **bound** (`-include bind.h`: `#define thrd_create qcore_tsan_thread_create`, `thread_sanitizer.rs:104`) | `races == 0 && clean`, five subjects |

**The boundary.** Wave 36 asserts *under the published `thrd_create`, TSan
cannot run either program*; wave 37 asserts *under a pthread-substituted build,
all five subjects are race-free*, and separately re-asserts the unbound fault
at its address. They share no compilation, no C executable and no Rust binary.
Both are green because the unbound fault fires at `__tsan_func_entry` before
any QCore worker code runs — so the **unbound rows are evidence about the
toolchain**, and the **bound rows are evidence about a program whose thread
primitive has been substituted** (`TASK-ROUTE-TSAN-PTHREAD-BINDING`, the
card's own stated cost). Could both stay green while the route is broken? In
three bounded senses, yes: anything visible only on the published
`thrd_create` path (the bound row never executes it); any schedule or arm not
executed (the `Result::Err` arms of the flagship, a failed allocation, any
program outside the five subjects — `TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`, as
registered); and the bound rows read **no stdout** (`.stdout` occurs 0 times in
`task_route_tsan.rs`), so their correctness evidence is wave 36's separate ASan
row. **The "forced onto one byte" control is a committed test**
(`overlapping_writers_are_reported_which_is_what_makes_the_clean_row_mean_something`,
`:293`), and the archived `mutations.json` mutates *that test* — second-order
non-vacuity, the standard met.

Two arms of the wave-36 test are vacuous in the method's sense: `(Ok(_), Err(_)) => {}`
(`:767`) passes silently, and the `(Ok, Ok)` arm's `!stderr.contains("data race")`
cannot fail because `run_native` sets `TSAN_OPTIONS=halt_on_error=1` (`:655`)
and returns `Err` on any non-zero exit (`:660-667`), so a race never reaches it
(**MEASURED** text, **INFERRED** TSan semantics). Registered as
`WLT-EXERCISE-TSAN-TEST-SILENT-ARMS`. Two prose-versus-harness gaps in the
wave-37 card are registered under `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`: the
report's §4.1 says the join mutation makes TSan report "**two** races" while
the committed harness runs under `halt_on_error=1` (`thread_sanitizer.rs:183`)
and the test asserts `>= 1` (`:355`); and the CT46 row runs `fork_join_trace()`
(`crates/qcore-core/tests/unit/mutex_c_emit.rs:843`), a hand-built program
used only in that unit file, with nothing asserting its events equal the
lowering of `fixtures/ct/CT46/case.qcore` that `run_mutex_crossing` executes.
Nothing here is proved; the card's §8 says so.

### 2.5 `CT-LANE-5` (`e88d9917`) — replay order, and whether a proof leg can be moved without the harness noticing

**(a) Replay order — MEASURED** (sub-audit, lines re-read). Emitted C:
`task_c_emit.rs:784-791` — identity 0 logs immediately, a worker appends to
`context->events[context->event_count++]`; `qcore_task_collect` (`:898-912`)
replays them after `QCORE_TASK_JOIN`. Reference: `task_execution/reference.rs:353-359`
buffers child events per task, the child body runs **eagerly at spawn**
(`:846-851`), and `Join` (`:636-655`) extends the parent's events with the
child's buffer. So CT07's 17-line transcript is the parent's order **by
construction in both halves**. A byte-identical comparison *can* show custody
events and values, the rebasing law (owner bytes 0 and 2), one allocation by
address, and that both `thrd_create`s precede either join. It *cannot* show
the real-time order of the two workers' writes, whether the threads overlapped
at all — a runtime that ran worker 2 entirely after worker 1 would produce
identical stdout (INFERRED from the replay code) — or the absence of a race.
`CT07-TWO-THREAD-REPLAY-ORDER`'s `why` says exactly this; one staleness: its
discharge names `TASK-ROUTE-TSAN-SEGV` (retired) as the blocker, and the TSan
row that now exists runs the `std.logger` **twin** `DISJOINT_EXERCISE`, not
`fixtures/ct/CT07/case.qcore` (§2.4, `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`).

**(b) `CT-FIXTURE-EXTERNAL-READERS` — the direction is the reverse of the one
the entry states.** MEASURED. `ct_corpus.rs` derives a case's fixture directory
from its **id** (`fixture_root().join(&case.id)`, `:1846`), reads `case.qcore`
at run time (`:2106`), requires the manifest's `source_fixture` to
canonicalize-equal that path (`:2894-2905`), and requires every executed
stage's artifact to equal the planned one (`:3021-3036`). **A manifest
re-point to another directory is refused by the harness, not silently
accepted.** Each proof leg `include_str!`s the *same* `fixtures/ct/<ID>/case.qcore`
(`ct01_proof_leg.rs:46`, `ct37:45`, `ct03:51`) and pins the reference run
event for event plus a digest — so the only re-pointing a lane can perform is
overwriting the fixture, after which **both** readers see the new bytes and
the leg goes red unless the reference run is event-identical (in which case
green is correct). The entry's "would move a proof leg's artifact with nothing
in the CT harness saying so" is therefore not how it fails.

**The real gap is the other direction, and it was unchecked by anything.**
`PROOF_GATED_CASES` (`ct_corpus.rs:222-240`) and `RUNNER_REGISTRY`
(`:470-474`) credit the proof stage to a **string** `file::test`, compared as
strings against the manifest's `executor` (`:3010-3020`); `rg proof_leg
compiler/tests/ct_corpus.rs` returns only those six literals. Nothing read the
named file. A proof-leg test renamed, `#[ignore]`d, or re-pointed at another
case's fixture — CT12's is byte-identical to CT01's (the card's own §9) —
would have left the numerator reading 3 with the harness green. **This card
closes that gap with one test rather than registering it:**
`proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture`
(`ct_corpus.rs`, appended) reads each named file at run time and asserts the
`fn` exists, its attribute block carries `#[test]` and no `#[ignore]`, and the
file `include_str!`s the case's own fixture. **Shown red by mutation**,
`scripts/mutate_proof_leg_binding.py`, log `mutations.log` beside this
report: M1 rename the credited `fn` in `ct01_proof_leg.rs` → **RED**; M2
`#[ignore]` the credited test in `ct37_proof_leg.rs` → **RED**; M3 re-point
`ct03_proof_leg.rs`'s `include_str!` at CT01 → **RED**; `restored: sha256
EQUAL exit=0`, baseline and post-restore green. Since the tripwire reads the
leg files at run time, no mutation recompiled the harness. Clippy's first run
over this test tripped `manual_contains` (§5); repaired and re-run clean.
`CT-FIXTURE-EXTERNAL-READERS` stays registered — its *discharge* (an
`include_str!` inventory check) is right and still open for the five
non-proof readers — with an amendment to its `why` proposed in §6.

**Proved vs observed.** The leg tests bind the artifact to a Lean-proved
decision procedure — proved, for the reference run. The binding from the
manifest to the leg test was a name equality observed by nobody; it is now a
test.

### 2.6 Wave 36's arms

**`CARD-SCOPE-GUARD` — the refusal is tested; the pack-refs shape is not.**
MEASURED (sub-audit; its probe log `private/stability-reeval-3/sub-d/packrefs_probe.log`
`ls`'d and read by this card). `scripts/tests/test_main_guard_hooks.py` (27
tests) builds a throwaway repository with a real `.claude/worktrees/agent-x`
worktree and drives the versioned `.githooks/` with real git — incident,
token, ancestry, cwd, deletion, no-op, pre-push and installer arms — with an
11-mutant harness `scripts/tests/mutate_main_guard.py`; the sub-audit ran the
Python suite (`Ran 295 tests … OK`, exit 0). The refusal is exercised by
tests, not only by the integrator's log. **But `rg 'gc|maintenance|pack-refs'`
over the test file returns nothing**: `MAIN-GUARD-REFUSES-PACK-REFS` is
reproduced (by `DOCS-COMPACT-7` and again by the probe: exit 128
`refused-provenance` from an agent worktree with and without the token) and
pinned by no test. The probe found two things the entry does not carry: the
refusal fires **in the main checkout too** when the token is absent
(`refused-unauthorised`, exit 128), and with the token the guard log records
**two** `refs/heads/main` transactions per `pack-refs` — `0000→X` then
`X→0000`, the second byte-for-byte the deletion shape `test_deleting_main_is_refused`
protects — so the entry's proposed exemption of "old = null OID" alone is
incomplete. Registered as `MAIN-GUARD-PACK-REFS-UNPINNED`. Observed
throughout; no Lean, by design.

**`WRITE-LEND-TASKS-EXERCISE` — three legs, each driven; the mutation evidence
unshipped.** MEASURED (sub-audit, test names re-read): checker rule — eight
frontend tests in `write_lend_tasks_exercise.rs` (`:278-401`) and five IR tests
in `task_execution/tests.rs` (`:774-847`); emission —
`the_emitted_worker_writes_one_byte_at_its_leases_base` (`:527`),
`a_program_that_exercises_nothing_emits_no_byte_access` (`:563`); sanitized
native — `two_threads_write_disjoint_bytes_of_one_object_sanitized` (`:692`,
ASan/UBSan, stdout byte-compared, one allocation). The report's §8 table
records 11 mutations with M5d surviving by design — from `private/mutate.py`,
naming no catching test per row, unarchived (§2.7).

**`MUTEX-SHARE-CROSS-EMISSION` — the serial schedule, proved on one side.**
MEASURED: the amended `why` of `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`
matches the code — `mutex_c_emit.rs:820-836` emits `thrd_create` then
**immediately** `thrd_join` in the same block; the reference run says so at
`:384-389`; and `QCore.MutexShareCrossEmission.the_creator_performs_no_event_while_its_child_runs`
(`MutexShareCrossEmission.lean:423`) proves it of the model. *Proved:* the
checked order has no parent event inside the child segment. *Observed:* that
the emitted C realises it (one TSan run, on `fork_join_trace()`, §2.4).

**`JOINED-PROVENANCE-CALL-FRAMES` — the allow-list's local arm.** MEASURED
(sub-audit, lines re-read): the retain at `legacy_joined_provenance.rs:153-155`
receives `preserved = cfg.conditional_names` from `outgoing` (`:226`), which
re-applies the identical membership at `:227-229` — so deleting the retain
changes no `derive` result and only the direct `step` test at `:893` goes red,
exactly as `CALL-FRAMES-ALLOWLIST-LOCAL-ONLY` records. The spared arm is not
vacuous (restoring the pre-card `clear()` fails `:870` and `:969`). What no
test does is tie the allow-list's **construction** (`legacy_c.rs:1839-1844`,
called from `:1972-1973`) to the arm through `analyze_unrestricted_c11`: no
C-source fixture anywhere drives a local surviving an intervening contracted
call end to end. Registered as `CALL-FRAMES-NO-SOURCE-FIXTURE`.

**The wave-36 close's `--check` content arm — still red by mutation.**
MEASURED by the sub-audit and its logs read by this card
(`sub-d/mutate_check_arm.log`, exit 0; `sweep.md5.{before,mid,after}` all
`ec6d4ced…`): `docs/reports/wave36-close/scripts/mutate_check_arm.py` mutates a
**copy** and never writes the source; at this base **8 of 8 mutations RED**,
kill counts 3/1/3/1/5/2/1/1, identical to the wave-36 table. Incidentally the
arm is live on the tree: the wave-37 close's documented `--check` exits 1 at
this tip because `--since 0f6eb0a6` reaches this card's own skeleton and finds
`(pending)` where §6 now stands — the sweep finding the in-progress card, as
designed.

**The wave-37 close's `lean_audits` checked key — red by mutation, and
optional.** MEASURED on a private copy of `build_matrix.py` (original md5
unchanged): a gate with `lean_audits = 139` against 137 `proof/Audit*.lean` is
refused with the verbatim message naming `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`
and writes nothing; with the comparison mutated to `False` the 139 gate is
accepted and the matrix written, so `test_build_matrix_sections.py:289` is the
test that goes red. But `GATE_CHECKED` is populated only `if key in supplied`
(`:204`): a `gate.json` that **omits** the key passes without comment, as the
wave-35 and wave-36 gates do. Registered as `LEAN-AUDIT-GATE-KEY-OPTIONAL`.

### 2.7 New findings measured directly by this card

#### `MATRIX-RECORDED-BY-ABSENT-67` — a third of the tracker has no `recorded_by`, including 28 entries registered in waves 36–37

**MEASURED.** `named_followups` had 215 entries at the base; their key sets
are three: 147 with `(id, recorded_by, source, title, why)`, **67** with no
`recorded_by`, and one (`wave3-CT`) with no `source`. Of the 67, **39** are
pre-schema entries and **28** were registered in waves 36–37 by five cards:
`CARD-SCOPE-GUARD` (6), `JOINED-PROVENANCE-CALL-FRAMES` (2),
`WRITE-LEND-TASKS-EXERCISE` (5), `BASE-TYPE-READER-CENSUS` (5) and
`W37-CORPUS-CONTRACTS-4` (all 10 — `LOADER-ABI-INLINE-FNPTR-DECLARATOR` among
them). The wave-37 close asked cards to register in the matrix rather than in
reports alone, and the registrations arrived with a silent schema split; the
two wave-37 code arbiters sit on opposite sides of it. **Why it matters
here:** this card's brief defines its population as "entries whose
`recorded_by` starts with `STABILITY-REEVAL`", and the first script this card
wrote against the matrix crashed on a `KeyError` at the eighth entry it read —
a sweep keyed on `recorded_by` misses 67 entries without saying so. Nothing is
lost (every one carries `source`), but a tracker whose entries do not agree on
their keys cannot be queried by provenance. Every entry this card adds carries
all five keys. *Discharge:* a generator check that every entry carries the
five keys, shown red by mutation, after back-filling the 67 from their
`source` reports (an integrator edit, not this card's) — **or** a recorded
decision that `recorded_by` is optional and `source` is the provenance key.

#### `W36-W37-MUTATION-HARNESSES-UNSHIPPED` — four cards' non-vacuity claims rest on a gitignored path

**MEASURED**, by `ls` and `find` over every waves-36/37 report directory.
Three cards ship both harness and per-test log —
`joined-provenance-call-frames` (`scripts/mutations.py`, `logs/mutations.log`),
`scalar-route-exit-edge` (`scripts/mutate.py`, `mutations.log`),
`task-route-tsan` (`artifacts/mutate.sh`, `mutate2.sh`, `mutations.json`) —
the standard `LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED` set. Four do not:
**`joined-provenance-round-bound`** (`README.md` alone; harness at
`measure/…/mutations.py`, no log); **`ct-lane-5`** (§2.4 cites
"`measure/ct-lane-5/mutate.py`; results in `mutations.json`" — no such file
in the directory; the seven results *are* under the `mutations` key of
`ct_stage_evidence.json:236-237`, so the citation names the wrong file and the
harness is unshipped); **`write-lend-tasks-exercise`** (§8:
"`private/mutate.py` (worktree-private)"; `README.md` + `gate.json`);
**`mutex-share-cross-emission`** (§4.2 names no script; `README.md` +
`ct_stage_evidence.json`). Each states a non-vacuity count a later reader can
check only by re-deriving every mutation by hand — the situation firing #1
named for one card is now the majority case for the two waves' concurrency and
CT cards, while every gateway card ships its harness. *Discharge:* per card,
ship the harness and a per-test log beside the report (append-only) or restate
the count in the card's own record as measured-once-not-archived; and one line
in the card-report convention saying which.

#### `CLANG-GUARDED-GREEN-FAMILY-750` — #2's 14-row finding is a 750-guard family, and no `gate.json` records whether Clang was present

**MEASURED.** `rg -c 'if !clang_available\(\)' crates compiler --glob '*.rs'`
totals **750** early-return guards across **79** files, from
`publication_custody.rs` (36) down; `scalar_route_exit_edge.rs` contributes
its 11. `RETURN-CALL-TESTS-CLANG-GUARDED-GREEN` named the shape on one file of
14 rows; it is the crate convention. On a Clang-less host every one reports
`ok` having measured nothing, and **no close's `gate.json` records whether
Clang was present** when its workspace count was taken — the ten keys are
counts and exits. This card recorded `clang-version.txt` (Ubuntu clang 21.1.8,
`QCORE_CLANG` unset, exit 0) beside its own gate as the cheapest form of the
discharge. *Discharge:* a `clang` presence key in the close's `gate.json`,
checked by the generator the way `lean_audits` now is, **or** one shared
helper that reports Clang absence as `ignored` so the count moves visibly; and
the 14-row entry re-scoped to the family or retired into this one.

#### `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT` — one of two arms landed; partial, not retired

**MEASURED.** The entry's discharge reads: "give the close's `gate.json` a
`lean_audits` key **(and any other figure the row publishes)** so the row can
be checked against it, or derive the row from that file; failing either, drop
the attribution". The wave-37 close landed the `lean_audits` key, checked by
`build_matrix.py:236-238` against the files on disk. The merged-gate row also
publishes "233 result summaries over 231 test targets", which has no key, and
nothing checks the row's text against `gate.json` — the generator checks the
file against the disk, not the row against the file (§2.6). So the entry is
**partial**: the mechanism that would have caught the drift *for the Lean
figure* exists; the parenthetical and the row-level check do not. Not retired
by this card.

#### `LEGACY-CFG-NODE-FIXTURES-SPELL-EVERY-FIELD` — the mechanism behind the `797661f3` integration failure will recur

**MEASURED.** Three test fixtures in `legacy_joined_provenance.rs` (`:553`,
`:598`, `:973`) and one in `legacy_recursion.rs` (`:432`) build `LegacyCfgNode`
by spelling every field in a local closure; `LegacyCfgNode` has no constructor
and no `Default`. Every field a card adds to the node therefore breaks every
sibling card's fixtures at integration, with both branches green alone and no
textual conflict — the exact shape of §3.1, and it recurs for the next field.
*Discharge:* a `LegacyCfgNode::plain(successors)` (or `Default`) in the
gateway crate that the fixtures use; or a recorded decision that the
integration gate is the intended catch.

#### Step-4 findings registered here (measured in §4)

`F02-WINDOWS-PIN-STALE-SINCE-WAVE-36` (§1.2), `CT42-CELL-FALLBACK-ARM-DIRECTION`,
`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`, `WAVE-38-QUEUE-NAMES-UNTRACKED`,
`TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`, `WLT-EXERCISE-TSAN-TEST-SILENT-ARMS`,
`MAIN-GUARD-PACK-REFS-UNPINNED`, `LEAN-AUDIT-GATE-KEY-OPTIONAL`,
`CALL-FRAMES-NO-SOURCE-FIXTURE`, `W37-SUBSTITUTION-SET-EQUALITY-UNARCHIVED`.
All fourteen are in `docs/reviews/r0_3/acceptance_matrix.json` via
`scripts/add_followups.py`, regenerated with
`build_matrix.py --gate docs/reports/wave37-close/gate.json` (exit 0;
`named followups 229`, `lean audits 137`, `contract modules 88`).

---

## 3. Step 3 — semantic edits made without a card

### 3.1 `797661f3`, `loop_exit_edge: None,` in `long_cyclic_cfg` — verdict: **discharged, no residual**, on a stronger ground than the close gave

**MEASURED.** The close verdicted this "fully discharged by the message and
the integration gate; no residual name", on the ground that the field is a test
initializer. The brief asks the sharper question: does `None` make the
round-bound test measure a graph the scalar rule never sees, where the
fixture's *intent* — a cyclic CFG whose header has an exit edge — would want
`Some(_)`? Answer, from the code rather than the message:

1. **The module under test never reads the field.** `rg -n loop_exit_edge
   crates/qcore-c-gateway/src/legacy_joined_provenance.rs` returns exactly the
   three fixture literals (`:558`, `:603`, `:978`); `derive`, `iterate`,
   `acyclic_sites`, `round_bound` and `outgoing` read `successors` only (`:172`,
   `:177`, `:189`, `:206`, `:230`, `:342`, `:362`, `:401`, `:407`). So `None`
   and `Some(1)` produce **the same graph** for every function the test
   exercises; the value cannot move the measurement.
2. **The scalar rule never sees hand-built CFGs.** `report_retained_loop_exit_edges`
   has one caller, `legacy_c.rs:6576`, inside the custody worklist; the
   round-bound tests call `iterate`/`derive` directly on a `LegacyCfg` literal
   and enter no worklist. The rule's arm at `:6527` is unreachable from the
   fixture by construction, not by the field's value.
3. **Intent.** `long_cyclic_cfg` is `0 -> 1 -> 2 -> {1, 3}` plus a chain; node 2
   is a two-successor node standing for a loop header, and were it a real C
   `while` header the builder would record `Some(1)` at `:5148`. `None` is the
   value the sibling fixture `cyclic_cfg` carries (`:558`) and the value
   `for (;;)` carries; it says "no header recorded an exit edge here", which is
   faithful to a hand-built graph that went through no builder. It is not the
   value a C `while` would carry, and that is immaterial for the reason in (1).

**Verdict: discharged; no residual for the edit.** The close's verdict stands,
and the ground is now the measured one — the field is unread by the tested
module — rather than the categorical one. What the incident does show is the
fixture-spelling hazard registered above (`LEGACY-CFG-NODE-FIXTURES-SPELL-EVERY-FIELD`).

### 3.2 The merged-gate row's Lean figure amended per merge inside a sentence provenanced to the previous close — verdict: **the discharge that landed is half of the entry's own**

See §2.7, `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`. The integrator edits
themselves (`136 -> 137` folded in at `797661f3`; the row replaced by the
close's own measurement at `e69083a6`) are **clean at the base**: the row reads
137 attributed to the wave-37 close, whose `gate.json` says 137, and the
generator accepted 137 against 137 files on disk. No figure is currently
wrong. The entry stays open as partial; `LEAN-AUDIT-GATE-KEY-OPTIONAL` (§2.6)
is the way the drift comes back.

### 3.3 `--uncarded` over `0f6eb0a6..e69083a6`

**MEASURED**, exit 0 (`private/stability-reeval-3/uncarded-w37.exit`): 19
commits write no card report, and every one is a card's own code commit, the
close's own WIP (`91e064c7`) or `DOCS-COMPACT-7`'s two re-derivation commits.
**Zero integrator commits outside merges**, confirming the close's §4. The
integrator's every edit this wave lives inside a merge commit, which is
`SYNC-RULE-TOOLING-COMMIT-SCOPE`'s deferred question in the place the tool
cannot see; that decision stays deferred and is not this card's.

### 3.4 The two names that went quiet "for the wrong reason"

**MEASURED.** `W37-MPACK-FAMILY-FIRST` is cited inside
`REPORT-REGISTERED-CLAIM-UNCHECKED`'s `why` and registered nowhere; the close
classified it a corpus-layout convention. `CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`
was retired by the close and survives as a string in two other entries' `why`
texts. Both classifications are right, and both are the mechanism the wave-36
close first recorded: `--residuals` reads the whole tracker text, so a name
cited in a `why` is invisible to it. No new name — but it is the reason this
card's §1 and §4 check presence in `named_followups` by `id`, never by string,
and it is how §4 found that two of the queued cards' target names are
mentioned and not tracked.

---

## 4. Step 4 — what a queued card would do to today's claims

Each paragraph names the claim that is true today only because a construct is
unreachable, cites the line, and says what breaks first. The five were read
by a sub-audit; the load-bearing lines of each were re-read by this card and
are cited as such.

**`CT42-CELL-ROUTE` — the first thing to break is a mis-stated *admitting*
arm, not `Q664`.** MEASURED: `derive_socket_cells`
(`crates/qcore-core/src/socket_footprint.rs:148`) refuses first through
`check_socket_program` (`:149`), then pairs each `Accept` with the FIFO head of
a per-listener queue and hands one `DeclareDescriptorAlias` per socket to the
**unedited** T09 checker, where `decide_disjoint` refuses two descriptors of
one cell as `SharedProtocolCell`/`Q664` (`disjoint_footprint.rs:714-721`) and
returns `Ok((true, …))` — *proved disjoint* — for two distinct cells (`:722`).
The two arms "unreachable for an admitted program" (`:184` `continue`; `:211`
`None => acquire(entries, binding, None)`) give the binding a **fresh
singleton cell** (`cell.unwrap_or(descriptor)`, `:167`), which the checker
then judges *disjoint* from its real connector — the admitting direction. The
card's report says the opposite (`ct42-protocol-cell/README.md:128-132`, "the
direction that *refuses more*"). Harmless today only because `:149` refuses
first; load-bearing the moment source routes the deriver, since any
disagreement between its FIFO pairing and `std_socket`'s peer assignment
(`CT42-CELL-PENDING-PHASE`) reaches an arm that admits. Also load-bearing:
constraint 15i's `Q085` external-peer refusal becomes the *soundness ground*
of `DeclaredProtocolCellIdentity` on every derived-cell certificate
(`disjoint_footprint.rs:1145`; the ground stated at `socket_footprint.rs:57-66`),
so `CT42-CELL-DERIVED-PREMISE` must be discharged before a source certificate
carries a derived cell. *Conclusion:* make both fallback arms `Err` (or prove
pairing agreement) and take the T09 edit before routing; registered as
`CT42-CELL-FALLBACK-ARM-DIRECTION`.

**`AGGREGATE-OWNED-FIELD` — an IR card before it is a route card; `Q055` is
the gate it must knowingly lift.** MEASURED: every owner-addressing field of
the cycle-3 IR is a plain `String` — `Read(String)`, `Write(String)`,
`Drop(String)` (`ownership.rs:169-176`), `bindings: BTreeMap<String, Binding>`
(`:242`), `NormalPathLeak(Vec<String>)` (`:845`) — and
`check_ownership(&[OwnershipOp])` (`:972`) has no path type to receive. So
`p.buf` has nothing to be spelled into before any of 15k's four missing rules
is asked. The sentence of 15k that turns false first is "a record contributes
**no** `qcore_core::ownership` operation": it is an explicit no-op arm
(`std_buffer.rs:914-918`) plus the effect walk's "derives nothing" (`:695-703`,
whose own comment says an owned field "would make the first of those false").
`Q054` (`BufferError::Custody`) would then be reached for a resource the
checker was never told about, and `Q055`'s "a buffer at a field position" is
the fail-closed refusal that keeps 15k true today. INFERRED: 15k's emission
argument ("among admitted programs, reading or writing a byte *implies*
acquiring a buffer", 80 programs byte-identical) never enumerated an
acquisition through a record constructor. *Conclusion:* the owner vocabulary
breaks first; the card's brief should say so and name `Q055` as the deliberate
lift.

**`CT07-TWO-THREAD-PROOF-LEG` — the leg cannot be bound on the CT01/CT37/CT03
pattern at all.** MEASURED: each existing leg binds through
`AdmittedCausalOrder::derive` over the reference run
(`ct03_proof_leg.rs:381-384`, `ct01:220`, `ct37` the same); `derive` calls
`reference_trace` (`task_trace_relation.rs:852`), which returns
`Err(UnsupportedObservedEvent)` for `BufferLeased`/`BufferLeaseReturned`
(`:563-565`) **and** `BufferByteWritten`/`BufferByteRead` (`:551-553`); CT07's
fixture spawns `writer(lend buffer 0..2)` and `writer(lend buffer 2..4)`, each
writing a byte, so its reference run carries both kinds and `derive` refuses
it at the first lease. Constraint 15n as amended records that boundary as
**decided** — the alphabet does not grow; `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`
is absent from the matrix — so the F02 route to a CT07 leg is closed by
decision, not by pending work. The two theorems
(`WriteLendTasks.lean:209`, `WriteLendTasksExercise.lean:149`) are about the
**ledger's lease list**, not a trace; a binding must go through the checked
`ScopedTasks` module's lease set — a pattern with no precedent and no
`PROOF_GATED_CASES` entry of its kind. The residual's own discharge names the
unavailable pattern and calls the leg "unusually cheap". *Conclusion:* bindable
only through the ledger; the entry's discharge is mis-specified today;
registered as `CT07-PROOF-LEG-PATTERN-UNAVAILABLE`.

**`TSAN-ROWS-2` — forcing is cheap and byte-neutral on the task route and
impossible by construction on CT46; the card splits in two.** MEASURED: the
task runtime creates and joins through `#ifndef QCORE_TASK_CREATE / #define
QCORE_TASK_CREATE thrd_create` (`task_c_emit.rs:722-726`, used at `:885`,
`:902`), and the TSan harness already rides that seam by `-include`
(`thread_sanitizer.rs:104`, `:167`, "no emitted byte changes"). A
contention-forcing card can redefine `QCORE_TASK_CREATE` to park each worker
on a barrier until all are created — safe *because* worker events are
buffered and replayed (constraint 15a), so the strict stdout comparison stays
strict. The mutex backend calls `thrd_create` bare (`mutex_c_emit.rs:827`),
and CT46's crossing cannot be forced at all: the proved contiguity
(`the_creator_performs_no_event_while_its_child_runs`) means the creator is
*in* `thrd_join` while the child runs, so any hook that let it contend would
realise an order the checker never saw. The claim that becomes load-bearing
is `TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`' "evidence about the schedules that
executed": under a forced schedule a row's meaning changes from "one run,
whatever happened" to "this schedule, forced", compounding
`TASK-ROUTE-TSAN-PTHREAD-BINDING` with a second substitution in the same
header. *Conclusion:* a harness card for the task route and a recorded
decision for CT46 — the residual's two arms must not be merged into one card.

**`LOOPS-ON-COMPOSED-ROUTES` — the exactness premise is a top-level index
computation, and each refusal sits *before* it for a reason.** MEASURED:
`lend_windows` (`std_buffer.rs:965`) computes `last_use` as the last
**top-level** event naming each lend binding (`:972-991`, over
`Store | Load | Extent | Release | Lend` only) with
`through = last_use.get(binding).unwrap_or(index)` (`:1054`); a `Repeat` body
contributes nothing, so a loan used only inside a loop would close before the
loop opens and every owner write in the body would be admitted as "outside
every live loan" — 15h's admitting arm. That is why the `Repeat` refusal
(`:1926-1944`, `SLICE-PERMISSIONS-LOOP-WINDOW`) fires before `lend_windows` is
called (`:1945`); lifting it without rewriting `last_use` is an admitting
change. The string route is the same shape: `last_view_uses` (`std_string.rs:1057`)
by top-level index, `through … unwrap_or(index)` (`:1509`), the `Repeat`
refusal at `:1814-1843` naming the exact failure ("a store inside a body could
precede — in index order — a read that happens after it in time"). Per
residual: `STRING-CONCAT-LOOP-WINDOW` falsifies "the window runs to the last
statement reading the view"; `SLICE-PERMISSIONS-LOOP-WINDOW` falsifies the same
plus `crossed_lend_windows` (`:1069`) and the nested-acquire scan
(`:1962-1988`), both over top-level indices; `WRITE-LEND-RELEND` (`:1600-1607`,
`NotLendable`) is orthogonal to loops but shares "a loan is never released and
never re-lent" — a sub-lend is a second borrow node nested in the first's
body, which the one-pass walker cannot see. *Conclusion:* a `for` on either
route needs a window over iterations (a rewrite of `last_use`/`last_view_uses`
walking `Repeat` bodies under a runs-many-times semantics); the refusals are
correctly placed and must not be reordered. And a card sized from the tracker
finds **one** of its three names: `STRING-CONCAT-LOOP-WINDOW` and
`AGGREGATE-OWNED-FIELD` are absent from `named_followups` (mentioned only in
other entries' `why`), and `SLICE-PERMISSIONS-LOOP-WINDOW` is one of the nine
names `CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK` classifies as a **rule**;
registered as `WAVE-38-QUEUE-NAMES-UNTRACKED`.

---

## 5. Gate

Run **detached** (`setsid -f`, no `trap '' TERM`) by
`scripts/gate.sh` (shipped beside this report), at `6368cc7b` — the base
`e69083a6` plus this card's documentation-only skeleton — so the **cargo test**
step measured `e69083a6`'s code byte for byte. Every step's exit was written
to its own file under the worktree-private `private/stability-reeval-3/gate-base/`
(gitignored, never under `target/`), no pipe in any gate command; the exit
files are copied into `gate/` beside this report. `git submodule update
--init --recursive` ran first (exit 0, `third_party/libevent` at `d82464a2`).
Disk 310 GB free at start, 266 GB at end. Started 05:13:26Z, cargo test
finished 05:29:52Z (**16m26s**), gate finished 05:31:18Z (**17m52s** wall).
Clang presence recorded (`clang-version.txt`: Ubuntu clang 21.1.8, `QCORE_CLANG`
unset, exit 0) — the cheapest form of `CLANG-GUARDED-GREEN-FAMILY-750`'s
discharge, taken here.

| Step | Exit | Figure |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **0** | **4,637 passed / 0 failed / 8 ignored**, 233 summaries over 231 targets (222 `Running` + 9 doc-test sections; a loose `grep -c '^test result'` reads 235 for the two `result_` test names); `test result: FAILED` 0 times — **reproduces the wave-37 close's figure exactly**, as it must: no Rust moved on the base |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` (detached run) | **101** | one `manual_contains` error at `ct_corpus.rs:7812` — **in this card's own new test**, which had been appended after the cargo step's build phase and before clippy reached it; the base's code is clippy-clean by the wave-37 close's measurement, and nothing else in the log |
| the same, re-run after the one-line repair (`gate-post/clippy.exit`) | **0** | clean |
| `reevaluation_probe` | 0 | JSON emitted |
| `reevaluation_followup` | 0 | JSON emitted |
| `scripts/check_lean_audits.py` | 0 | **137 Lean audits** |
| `python3 -m unittest discover -s scripts/tests` (detached run, base matrix) | 0 | **`Ran 295 tests`, OK** |
| the same, re-run after registration and regeneration (`gate-post/python.exit`) | 0 | **`Ran 295 tests`, OK** |
| `cargo test -p qcore-compiler --test ct_corpus proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture -- --exact` | 0 | 1 passed |
| `scripts/mutate_proof_leg_binding.py` (→ `mutations.log`) | 0 | 3 of 3 mutations RED, `restored: sha256 EQUAL exit=0` |

The clippy row is reported as it happened rather than as it ended: the
detached run's clippy step is the one honest measurement of the tree with the
unrepaired test in it, and the repair is one line (`contains` for
`iter().any`). Constraint 16 rows, stated because a gate report must say which
it saw: all five named rows living in this workspace were green, and the
unnamed seventh `a_forced_override_never_weakens_an_execution` too — a
full-suite run on a machine carrying its own build and five sub-audits'
reads, **not** the quiet-host number `HOST-ROW-STARVATION-DESCRIPTION` asks
for. Provider **122 is carried**: nothing under `providers/` or
`crates/qcore-schemas/` is touched by this card; a Unix figure.

**This card changed one test and no other Rust**, so the workspace figure is
evidence that the base is green and that the added tripwire is green and
non-vacuous — not evidence about any code change. The matrix regeneration
with `--gate docs/reports/wave37-close/gate.json` accepted `lean_audits = 137`
against 137 files and printed `named followups 229`.

---

## 6. Integrator reconciliation list

1. **`F02-WINDOWS-PIN-STALE-SINCE-WAVE-36` — the one live red row this card
   found.** `compiler/tests/windows_threads.rs:31,36` should read
   `bfa572914cb96413` and `9cbe67a70a420d52` (the values
   `f02_trace_relation.rs:96-97` proves for the LF bytes). Land it in a card
   that records a Windows run of
   `windows_f02_renderings_satisfy_the_incoming_trace_relation_and_emission_pins`
   — or states in so many words that the native half was not run — and, in
   the same card, make the pair one shared constant or add a Linux-side test
   asserting the two files' literals agree (red by mutation), which is
   `F02-DIGEST-PIN-DUPLICATED`'s own discharge. Not done here: a Windows
   expectation re-pinned from Linux without a run is `37bc9004`'s shape.
2. **Amend `CT07-TWO-THREAD-PROOF-LEG`'s discharge** (registered by
   `CT-LANE-5`) in these words: replace "a proof-leg card on the
   CT01/CT37/CT03 pattern — a named host-unconditional test lowering CT07's
   own fixture and checking its artifact against the Lean-proved decision
   procedure" with "a proof-leg card binding CT07's checked `ScopedTasks`
   module's lease set to the Lean `Reachable` term of
   `QCore.WriteLendTasks` — a new binding pattern, since
   `AdmittedCausalOrder::derive` refuses this run at
   `task_trace_relation.rs:551,563` by constraint 15n's decided boundary —
   with its own non-vacuity method", and delete "unusually cheap". Or record
   `declined` with the ground.
3. **Amend `CT-FIXTURE-EXTERNAL-READERS`'s `why`** in these words: after "so
   re-pointing a case can silently re-point another card's tests", append
   "— measured by `STABILITY-REEVAL-3`: the harness refuses a manifest
   re-point (`ct_corpus.rs:2894-2905`, `:3021-3036`), so the silent direction
   is the reverse one — the proof stage credited to a `file::test` string
   nobody read — now checked by
   `proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture`".
   The discharge (an `include_str!` inventory check for the five non-proof
   readers) stands.
4. **Publish beside two frozen reports** (constraint 21, append-only):
   `docs/reports/ct42-protocol-cell/` — the fallback arms at
   `socket_footprint.rs:184,211` assign a singleton cell the checker judges
   *disjoint*, the admitting direction, unreachable only by `:149`
   (`CT42-CELL-FALLBACK-ARM-DIRECTION`); `docs/reports/task-route-tsan/` —
   §4.1's "two races" against the harness's `halt_on_error=1`, and "CT46's
   emitted crossing" being `fork_join_trace()` rather than the fixture's
   lowering (`TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`); and
   `docs/reports/ct-lane-5/` — §2.4's `mutations.json` is the `mutations` key
   of `ct_stage_evidence.json`.
5. **RETIREMENTS — named explicitly, as the brief requires. This card retires
   nothing.** In particular it does **not** retire
   `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT` (partial: the `(and any other figure
   the row publishes)` arm and the row-level check are unlanded, §2.7),
   `F02-DIGEST-PIN-DUPLICATED` (escalated, not discharged), or
   `CT-FIXTURE-EXTERNAL-READERS` (its discharge is right and open). Each was
   verified registered before this sentence was written.
6. **Decide `recorded_by`** (`MATRIX-RECORDED-BY-ABSENT-67`): back-fill the 67
   from their `source` reports and add the five-key generator check, or
   record that `source` is the provenance key. Either way, restate this
   cadence's population rule in the next brief on `source`.
7. **Mutation harnesses** (`W36-W37-MUTATION-HARNESSES-UNSHIPPED`): ask the
   wave-38 close to ship or restate for `joined-provenance-round-bound`,
   `ct-lane-5`, `write-lend-tasks-exercise` and `mutex-share-cross-emission`,
   and add one line to the card-report convention.
8. **Gate keys**: from the wave-38 close on, refuse a `--gate` file lacking
   `lean_audits` (`LEAN-AUDIT-GATE-KEY-OPTIONAL`) and add a `clang` presence
   key (`CLANG-GUARDED-GREEN-FAMILY-750`); this card's own `gate.json` carries
   `lean_audits` and records Clang in its note.
9. **Amend `MAIN-GUARD-REFUSES-PACK-REFS`'s discharge** to cover the second
   transaction (`X→0000`, the loose ref's deletion) and the main-checkout
   refusal without the token; then pin whichever behaviour is chosen
   (`MAIN-GUARD-PACK-REFS-UNPINNED`).
10. **Lines for the queued cards' briefs** (§4): `CT42-CELL-ROUTE` — make the
    fallback arms `Err` and take the T09 edit first; `AGGREGATE-OWNED-FIELD`
    — an IR card first, `Q055` the deliberate lift; `CT07-TWO-THREAD-PROOF-LEG`
    — ledger-side binding only; `TSAN-ROWS-2` — split into a `QCORE_TASK_CREATE`
    harness card and a recorded decision for CT46; `LOOPS-ON-COMPOSED-ROUTES`
    — rewrite `last_use`/`last_view_uses` over `Repeat` bodies, keep the
    refusals in front, and register or classify its three names first
    (`WAVE-38-QUEUE-NAMES-UNTRACKED`).
11. **Next firing.** `STABILITY-REEVAL` #4 is due after wave 39. On this
    firing's evidence the highest-value unexamined ground is the **Windows
    arm** — every Linux gate is blind to it and the one live red row this
    card found lives there — and the **tracker-only residuals** (§1.1): 18
    mechanism-needing entries, zero discharged in two waves; a close that
    reads them is the cheapest change to that number.
