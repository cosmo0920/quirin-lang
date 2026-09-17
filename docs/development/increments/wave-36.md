# Wave 36 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`WAVE-36-CLOSE` (2026-09-18).** Seven cards, **two arbiters**, base
`fa48211a`. All **89** integrator-addressed items given a verdict **measured
against the tree** — the target file grepped for the proposed text or its
effect, never inferred from a report's claim that it had landed — and that
distinction found **four unapplied items and one that could not land as
written**. Unapplied: `W36-CORPUS-CONTRACTS-3`'s **constraint 7** and
**constraint 8** amendments, `STABILITY-REEVAL` #2's **constraint 5c**
amendment, and **constraint 14's entire wave-36 entry**, which no card had
written — neither the six-of-seven "do not engage" list nor
`WRITE-LEND-TASKS-EXERCISE`'s `+0` censused row. All four are landed here, 5c
with two corrections *recorded* rather than made silently: the card's two line
numbers had moved (`legacy_c.rs:3312` and `:5161`, not `3277`/`5056`, the
mechanism verified in source before landing, since landing a stale number would
put a false statement into a standing constraint), and "those" → "that"
following the card's own deletion. The fifth, constraint 16, is **split**: the
proposed text says the host row "joins this list as its **sixth** row" and the
row has *been* the sixth since the wave-35 close, so that half is superseded and
the **measurement** lands beside the existing row — the row's first observation
at the *middle* of the load range (wave 35 measured ≈100 and ≈6.5) and the first
after the `poll(2)` repair, showing the repair narrows the failure window
without closing it. Constraint 16 is now recorded **known incomplete**:
`LOAD-SENSITIVE-ROW-UNNAMED` names a seventh test with the same signature in the
same crate. **`STABILITY-REEVAL` #2's `--check` content arm is adopted and built
stronger than proposed**: rather than parsing an `applied` verdict's prose
`where`, an entry carries a structured `verify` list of `{file, contains}` /
`{file, absent}` claims that the tool opens and reads, whitespace normalised on
both sides so a wrapped quotation matches and nothing else normalised so a
**softened** amendment is still a changed claim; from wave 36 an `applied`
verdict must carry `verify` or an `unverifiable` ground. 19 new tests (45 → 64),
each red by mutating the fixture, and **the arm itself was falsified by mutating
the implementation in eight places, all eight caught**. Run on this close's own
record it found **five** things — a false `absent` claim about constraint 15h
(whose superseded sentence is deliberately *retained* as a named reference), a
wrong evidence-pointer quotation, a capitalisation, a missing claim and the
close's own report needing a `no_section` reason — and the fixed record verifies
**112 content claims across 9 files**, 46 of 50 `applied` verdicts carrying
checkable text and 4 declared unverifiable with a ground. Verdicts: **50
applied, 31 noted, 4 superseded, 4 deferred, 0 declined**. `named_followups`
**171 → 187**: +18 for every residual the sweep found reaching no tracker (all
nine of `W36-CORPUS-CONTRACTS-3`'s §9 and all nine of
`C-GATEWAY-INDIRECT-CENSUS-2`'s, both cards having left them to the close
deliberately), +1 for the close's own `DECISION-ALSO-REGISTERED-AS-FOLLOWUP`
(two names are now carried by *both* the decision mechanism and the tracker, so
ending one will not end the other), and **−3 retirements, each named and each
checked to be a registered entry first**: `CARD-SCOPE-SELF-MERGE` and
`MAIN-GUARD-UNACTIVATED`, both discharged because the integrator ran the guard's
activation — the log carries the self-test's two refusals and every merge of the
wave as authorised — and `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`, discharged
by the arm above. The guard log is **read for the first time** (21 authorised
entries, 5 refusals), which is `MAIN-GUARD-LOG-UNCONSUMED`'s question answered
though not its automation; it also exposed a gap the installer's message does
not mention — **`git push` of `main` needs the token exactly as `git merge`
does**, two pushes refused before it was carried — now in `CLAUDE.md` beside the
`pkill` rule landed in `MUTEX-SHARE-CROSS-EMISSION`'s own words after that card
killed a **sibling** card's gate run, and beside the new close rule that a
retirement is named, checked, and swept after. The 44 residual-shaped names were
classified one by one (18 registered, 15 card names, 8 discharged with the
discharging card named, 2 standing rules, 1 a deferred decision) and the 26 seam
commits by eye (22 cards' own code, 3 integrator corrections, 1 the wave's
opening — **zero** uncarded work). Ten deferred-decision entries are re-listed
with grounds, six carried open from wave 33, two ended at wave 35 and recorded
so the 15-entry carry is accounted, and **two raised by this wave**:
`INDIRECT-CENSUS-FEATURE-UNBUILT` and `TASK-ROUTE-TSAN-SEGV`'s priority — the
latter behind a route-wide fact the close states plainly, that the **whole
checked concurrency route has no thread-sanitizer evidence and no TSan row
exists anywhere in this repository**. Gate at the close's own tip, with its one
code change *inside* the measurement: **4,611 passed / 0 failed / 8 ignored**
over 233 summaries and 229 executables, **135 Lean**, **293 Python**, both
probes 0, fmt/clippy clean, 22m31s, provider 122 carried (constraint 18 not
engaged). The Python run went red once, informatively:
`CommittedMatrixIsSelfConsistent` caught `171 != 187` after the hand edit, and
the repair was to regenerate rather than touch the count. **Nothing is
promoted**: no corpus count, unit status or function status moves at this close,
the contract corpus stands at 82, CT coverage at 46/50 with sanitized-native 10
and proof 3, and full language conformance stays **false**. `DOCS-COMPACT-7` is
due around wave 37 and `STABILITY-REEVAL` #3 after it; `main` moved to
`477b733d` (wave 37's opening) during this close, whose own commit message
agrees that wave 36's paragraph belongs here.

**`WRITE-LEND-TASKS-EXERCISE` (2026-09-18, wave 36).** Discharges the residual
of the same name, landing in one card the three things wave 31's sentence
demanded together: checker rule, emission and sanitized native comparison. A
worker body now **exercises** its range write lease —
`qcore_buffer_write(LEASE, INDEX, VALUE)` and `qcore_buffer_read(NAME, INDEX)`,
the **arity** selecting the form on the `borrow_utf8` precedent, so **+0 words,
+0 tokens, +0 `Q` codes** (censused at zero occurrences of either new arity
across 514 `.qcore`/`.qc` fixtures). The rebasing law is decided **in the IR and
split across the two places its halves are known**: the worker's body states the
exclusive bound it touches through its lease, because a body is checked without
knowing where its lease sits; the creator's spawn site, the only place the real
`[low, high)` exists, decides the fit. Those are proved *equivalent*
(`the_footprint_fit_is_exactly_the_rebasing_law`), which is what licenses
checking a summary once instead of every write at every creation. Exclusivity is
re-proved about the **writes themselves** rather than about the permissions
(`no_byte_is_written_by_two_parties`), and exhibited natively: **two threads
writing distinct bytes of one `int32_t` under ASan/UBSan/LSan, stdout
byte-identical to the reference run**, one allocation released once. Leg 4
**decides** `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`: the F02 observable alphabet
does **not** grow, because `AdmittedCausalOrder` is a *sequence* and two
disjoint leases' writes have no causal edge between them, so any order it could
carry would assert an ordering the run does not have — the reachability
argument, that a lease event always refuses first, is published beside it as a
measurement rather than as the reason. Three boundaries are measured rather than
claimed. **ThreadSanitizer cannot run any program this route emits** — wave 31's
own held-only program and the published lease-free `examples/threads/two_workers`
die at the identical address and pc, so the failure is the route's and not the
exercise's (`TASK-ROUTE-TSAN-SEGV`). A stray byte at `payload + 4` lands in
`struct qcore_buffer`'s **own tail padding**, which the allocation owns, so ASan
is silent — the measured reason the emitted helper carries its own extent guard
instead of delegating to a sanitizer, with the overflow named only at `+64`. And
a program mixing whole-payload and byte access is host-dependent
(`WRITE-LEND-EXERCISE-BYTE-ORDER`); the flagship is representation-independent by
C11 7.20.1.1 rather than by luck. Eleven mutations, ten caught and one surviving
**as recorded** — breaking the frontend's read-share arm leaves the IR refusing
with the same sentence, constraint 15h's "decided in the IR, not by a route rule"
made visible. The campaign found a real defect in the test set: three IR arms
were backstops **no test reached**, the
`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` shape again, now driven by five
IR-level tests. 16 card tests, 133 Lean audits, no Windows or Apple run claimed
(`WRITE-LEND-EXERCISE-WINDOWS-RUN`). CT07 is deliberately **not** remapped
(constraint 23); `CT07-TWO-THREAD-REMAP` names the lane work.
`docs/reports/write-lend-tasks-exercise/README.md`.

**`MUTEX-SHARE-CROSS-EMISSION` (2026-09-18)** discharges the residual of the same
name — wave 32's declared mutex crossing was *checked and never executed*,
because `mutex_c_emit::is_single_task_trace` gave no C to any trace carrying task
structure — and its interest is a mutation that **escaped**, and what had to
change because of it. The backend gains a second rendered shape,
`classify_trace`'s **fork/join**: non-nested `Spawn`/`Join` pairs whose children's
events are *contiguous* between them, which is exactly what the crossing lowers
to (measured: CT46's trace is `CreateMutex, Spawn, ⟨five child events⟩, Join,
⟨creator's own section⟩`). It renders with real `thrd_t` children over the
existing `mtx_t` emission, generations and protected allocations moved to file
scope so a child's own function can name them; the single-task rendering is
untouched and every previously emitted program is byte-identical. CT46's four
`unassessed` stages — reference execution, emission, native execution and the
sanitized run — move to `executed` on the case's **own** artifact, driven by
`run_mutex_crossing`, and CT46 joins `SANITIZED_GATED_CASES`: sanitized-native
**9 of 46 → 10 of 46**, the first movement of that numerator since wave 32, and
evidence about one case's artifact rather than conformance (constraint 23). The
three coupled edits `CT-CTWIN-LANE` §9 item 5 named landed in one change, as it
required. **The escape.** Five of six emitted-C mutations were caught, and three
were not: deleting the real `mtx_lock` while keeping its log line, moving the
real `mtx_unlock` away from its log line, and calling the child function
directly instead of spawning a thread each produced a **byte-identical** run,
exit 0, clean under ASan/UBSan/LSan. The reason is structural and worth stating
plainly: the native comparison compares the program's *logged* event sequence,
so it is evidence about that sequence and not about the C operations beneath it.
Two repairs followed rather than a caveat — `run_mutex_crossing` now asserts, on
the emitted text, that every logged mutex event is *immediately preceded* by its
own `mtx_lock`/`mtx_unlock` call (the adjacency one renderer writes, proved in
Lean as `a_logged_acquisition_is_preceded_by_a_real_one`), and the thread
boundary is pinned by `emitted_c_contains` plus a `main_excludes` check that the
creator's body carries none of the child's section. With those, the three
escapes go red. A fourth mutation — removing the `thrd_join` so the creator
races the child — aborts or hangs, never a silent pass. **What is deliberately
not claimed.** Contention, a data race, the C memory model. The checked trace is
a *total order* whose child segment is contiguous, so the faithful rendering
creates the child and immediately joins it: the creator performs no trace event
while the child runs. That is not a limitation worked around — a rendering that
let the creator run ahead would realise an order the checker never saw — and it
is **proved**, as
`QCore.MutexShareCrossEmission.the_creator_performs_no_event_while_its_child_runs`,
rather than promised in prose (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`). No
TSan row exists anywhere in this repository and none is added. The Lean is 30
theorems (`AuditMutexShareCrossEmission.lean`, axiom closure `propext` /
`Classical.choice` / `Quot.sound`): a refused trace renders nothing, refusals
agree between backend and interpreter, segment contiguity and its no-contention
corollary, the two renderings agree, and CT46's own shape decided. Ten hand-built
traces drive every way out of the shape in Rust, each refused by classification,
emission and reference execution at one index; all six classification arms were
shown non-vacuous by mutation, one of them only after a **second** attempt — the
"unmatched spawn" case was being caught by the contiguity arm first, so the
end-of-events arm needed a trace that simply ends inside an open segment. Two
pins moved deliberately and neither was deleted: a composed child module now
emits and runs (its old `c_source.is_none()` assertion carried the instruction to
do this in its own failure message), and the boundary it guarded is re-asserted
on a **guard-local** child, which is outside both shapes, reachable from source
and therefore not a vacuous refusal — pinned in the fixture too, as CT46's new
`guard_local.qcore` control. The card allocates **no `Q` code, reserves no word
and widens no condition**; `Q600`'s existing backend-refusal condition is only
*reached* from the new shape, recorded on the `SOCKET-EVENT-COMPOSE` precedent.
Evidence: `docs/reports/mutex-share-cross-emission/README.md`.

**`JOINED-PROVENANCE-CALL-FRAMES` (2026-09-18), the arbiter merging after
`W36-CORPUS-CONTRACTS-3` — the name is found, and the leaf still does not
move.** Wave 35 measured two independent blockers at all 14 sites reaching
`release_site`'s premise gate; this card was chartered against one of them, so
**zero corpus movement was the expected outcome and is what happened**: 0 leaves
added, 0 removed, 54,503 unmoved, 0 unit and 0 function status moves, with
AFTER's verdict-bearing artifacts **byte-identical** to BEFORE's, and BEFORE
reproducing `docs/reports/w36-corpus-contracts-3/artifacts/` byte for byte
(`diagnostics a7aa3b5f9ab193a0`, `chunk-contracts c4f438e4ed3dab3c`). Every
count stands as the declarative arbiter left it — leak **272**, `unknown_alias`
**4,308**, `unsupported` **49,922**, `summary_conflict` **1**, functions
**205 / 84 / 4,545**. What the card delivers is the **attribution** wave 35's
`JOINED-PROVENANCE-FACTS-NAME-NOTHING` asked for, taken from the analyzer's own
instrumentation rather than from source text. The 14 visits are **6 distinct
sites**, and the cause splits three ways: **`call_forget` at 3 sites (6 of 14
visits)** — constraint 10b's "calls forget prior relations", and the cause at
**every** site whose relation had ever named the released pointer; the fixed
point declining at the **iteration round bound** at 2 sites (6 visits), which
wave 35 did not separate and which is the new residual
`JOINED-PROVENANCE-ROUND-BOUND`; and `JOINED-PROVENANCE-CYCLE-SITES` at the
remaining 1 site (2 visits). Because calls were the cause wherever a name
existed, the designed rule landed: the `ContractCall` arm of `Facts::step` no
longer clears the binding map but keeps names in `cfg.conditional_names` —
pre-existing, `unique_local_names ∩ frame_protected − formals`, so
address-never-taken automatic locals a callee cannot reach under constraint
10a's own no-undefined-behaviour C object model — while forgetting the
destination and `Out` slots (already gone via `rebound_names`) and, newly, a
`Reallocated` parameter. The naming premise now passes at **6 of 14 visits (3 of
6 sites), from 0 of 14**. Nine Lean theorems in
`QCore.JoinedProvenanceCallFrames`, **all axiom-free**, state the rule, its
monotonicity in the refusing direction, that a written name is outside every
frame, and — as the conservativity statement in exact form — that the pre-card
blanket forget **is this rule at the empty frame**, with
`a_call_edge_is_transfer_sound` discharging precisely the hypothesis
`JoinedProvenanceCycles.fixed_point_is_sound` already carried, so wave 35's
theorems are reused unedited. 6 of 6 mutations caught; one mutation first
reported MISSED and was traced to a **non-unique anchor** mutating an unrelated
rule, fixed rather than recorded as a gap, and one tidy-up line was **deleted
rather than shipped unfalsifiable** once its four tests were found to be
asserting map shape instead of meaning. The card's own vacuity finding is
registered as `CALL-FRAMES-ALLOWLIST-LOCAL-ONLY`: the frame guard cannot fire
through `derive` today, because `outgoing` applies the same membership test
after every node, so it is driven by a direct `step` test and recorded as a
local enforcement of the premise rather than as verified in place. The
counterfactual for the standing decision is published: if
`JOINED-PROVENANCE-RELEASE-NULLABLE` were relaxed and nothing else changed,
exactly **one** of the 14 visits — `rd_kafka_jwt_b64_decode_payload` site 1,
name `payload` — would have every premise pass.
`AOM-DESTROY-JOINED-PROVENANCE` is untouched and unreachable by this rule by
construction, its binding being address-taken and therefore outside the frame.

**`C-GATEWAY-INDIRECT-CENSUS-2` (2026-09-18, wave 36).** A measurement card:
no analyzer rule changed, no verdict moved, no re-pin owned. It censuses the
corpus's largest refusal family, `indirect memory access is outside the resource
projection` — **18,555 leaves across 3,239 functions in 171 units** — from the
analyzer's own projection rather than from source text, which is the error that
stalled the abandoned wave-33 census (refusals are recorded at the preprocessed
line, constraint 12a, and wave 35 measured the source-vs-preprocessed gap at
~57x for macro idioms). Instrumentation sits behind the off-by-default
`indirect-census` cargo feature on `qcore-c-gateway`, enabled by no workspace
member; with the feature **on** and the census actively streaming 88,001
statement records, the pinned corpus audit still reproduces
`docs/reports/c-gateway-constant-do-while/artifacts/` **byte for byte** on both
verdict-bearing artifacts. The census explains **18,554 of 18,555 leaves
(99.99%)**, joins **171/171** units exactly, and independently reproduces the
integrator's wave-33 figure — **284 sole-blocked functions holding 833 leaves
(4.5%)**. Its result is **negative and reported as one**: ranking shapes by leaf
mass and by *decided functions* give nearly disjoint orders, the top eight shapes
by leaf mass (6,699 leaves, 36% of the family) decide **two** functions between
them, the best single shape decides **11**, the best pair **17**, and all 403
shapes admitted at once **45**. Constraint 5's premise — the base is a name this
function never binds to a resource identity — **fails for 95.7%** of the family
and 87.2% of the sole-blocked leaves, which bounds any arbiter that does not
propose to weaken it. Because 72.2% of leaves end at a member the declaration
reader will not classify, the card also instrumented `read_record_body` and found
it total-or-reject over the whole body: **173 of 909 record tags (19.0%)** are
refused entirely on one `{`, `(` or `:`, so a function-pointer member or bitfield
makes every sibling scalar unclassifiable — 2,713 leaves, measured ceiling **2
sole-blocked functions**. The reader's dominant answer, though, is that **78.5%
of unresolved members were never a question**: the *base*'s declared type was not
committed, a different reader nobody has measured
(`BASE-TYPE-READER-UNMEASURED`). Nine residuals are named, including
`CENSUS-TWO-PREMISE-READINGS` (the two implementations of constraint 5's premise
disagree on 16.0% of the family, and the constraint's own wording is the stricter
one) and `POISON-UNMASK-ON-ADMISSION` (any admission spares bindings from the
refusal poison and can unmask a stale scalar fact, constraint 10a's already
recorded hazard, so a wave-37 re-pin must attribute those moves separately).
Gate: **4,580 passed / 0 failed / 8 ignored** over 230 result summaries,
fmt/clippy clean (plus an extra clippy pass with the census feature **on**), both
probes exit 0, **132 Lean audits**, every exit captured to its own file — all
eight steps exit 0. One constraint-16 observation is recorded with **both**
facts: a separate run deliberately concurrent with two corpus audits failed
`a_forced_override_never_weakens_an_execution` with `Io(Broken pipe)`
(`qcore-verifier-host/tests/execution_quota.rs:777`) and the same test **passes**
in the isolated gate; that is constraint 16's signature and crate but **not** one
of its six named rows, and the card files it as `LOAD-SENSITIVE-ROW-UNNAMED`
rather than proposing a seventh, because one observation under a heavy
self-inflicted load carries no measured load figure. Report:
`docs/reports/c-gateway-indirect-census-2/README.md`.

`STABILITY-REEVAL` #2 (2026-09-18), the cadence's **second firing**, audited
waves 34–35 by `CLAUDE.md`'s four steps and closed with **31 registered
residuals and zero retirements** (`named_followups` 121 → **152**), the
zero deliberate: a retirement is what this firing found doing the damage. Over
firing #1's 27 residuals the tally is **15 discharged · 3 partial · 10 open**,
and the pattern is that every residual needing a **test, a control or a
mechanism** is open while almost every one needing a **sentence** is
discharged. Two findings are about the mechanisms the project relies on rather
than any card. `AGENTS.md`'s Cycle 19 raw corpus row carries "**44** corpus
functions" and "**21** sites" where card `JOINED-PROVENANCE-CYCLES` §11 said
"apply in these words" and the words were **40** and **14**; the 21 is the
**superseded `a300a39d` census** that card's own §2 exists to stop anyone
quoting, and the string 44 occurs nowhere in its report or logs
(`JPC-LEDGER-44-21-STALE`, measured independently by two sub-audits). And
wave 35's `reconciliation.json` records that very item **`applied`**, naming
the row as the place and asserting the 40 and the 14 "is present" — which they
are not. The close rule already demands an `applied` verdict say *where*, and
this one does; nothing checks what it **asserts** against what is **there**, and
a scan of all 103 wave-34/35 verdicts shows the defect invisible to any check
that does not read the named location (`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`).
Three Lean statements in one module are described by more than they prove:
`an_off_cycle_node_does_not_return`'s only hypothesis asserts **whole-graph
acyclicity**, so it is vacuous exactly where its card operates, while the
shipped `acyclic_sites` tests per-node off-cycle-ness inside a cyclic graph and
the property claimed ("runs at most once per call") is about execution, which
`Steps` cannot express; `fixed_point_is_sound` is described by the acyclicity
hypothesis it **lacks** rather than the `TransferSound` premise it **carries**,
a name that appears in no report, record or ledger; and `acyclic_solution_is_unique`
holds only among solutions agreeing at the entry. Constraint **5c** is false
about the shipped tree for `for (;;)`: `condition.is_empty()` has dropped that
header's **exit** edge since Cycle 19 (`490f5151`), with neither the Lean
statement nor the re-pin 5c says such a step needs — exact, not unsound, but the
neighbour is unowned (`FOREVER-FOR-EXIT-EDGE-UNOWNED`). The sentence 5c records
the integrator as having caught, because landing it "would have put a statement
contradicting 10b into this file", is landed verbatim in the committed script
5c sends readers to. Wave 35's own record shows a **gate exit swallowed by a
pipe** — a merge reached origin with the Python suite red — "the second time
this wave". Measured **clean** and reported as such: both `C-GATEWAY-RETURN-CALL-W4`
arms go red under mutation and both mutations produce a false `Safe`, so the
premises are load-bearing; all three unread `AGENTS.md` merge resolutions
dropped nothing; **zero** weakened tests across waves 34–35, with three pinned
expectations moved and all three disclosed in both report and commit message;
and an `AbstractPointer` with empty targets, `unknown` and `may_be_null` false
provably cannot reach `state.pointers`, by exhaustive enumeration of fourteen
write sites — though that invariant is asserted nowhere and the struct's derived
`Default` **is** the forbidden triple. Forward: the 64-round fixed-point bound
is driven by **no test**, counted by **no census**, and its decline returns the
**same value** as "nothing to derive", so the rate is unmeasurable without a code
change; `release_site` has **seven** refusal disjuncts and three are
instrumented; and both wave-36 arbiters are chartered against numbers that exist
only outside the tree. `docs/reports/stability-reeval-2/README.md`.

**`W36-CORPUS-CONTRACTS-3` (2026-09-18), wave 36's first arbiter by merge order (its brief said second; the order flipped when it finished first — integrator's settlement of `JOINED-PROVENANCE-CALL-FRAMES` §9.7) — six
declarations chosen by function-cover yield, and the two places the method
broke.** Declarative: six `.qcore` modules and six manifest rows, no crate
source touched, so **one audit binary served both stages** and the only
difference between them is which modules the manifest selects. The BEFORE
reproduces `docs/reports/joined-provenance-cycles/artifacts/` byte for byte in
both verdict-bearing artifacts — the card brief named
`c-gateway-constant-do-while` instead, which was **stale by one**, checked
rather than assumed (the two archives' diagnostics and chunk-contracts
decompress identically, so the cover is the same either way).
The cover was re-derived rather than carried: **193** projected functions are
blocked *only* by `call to uncontracted function` leaves, and **122 of them
have exactly one blocker**, which is what makes a single declaration able to
move a function. Selection was by sole-blocker count and deliberately not by
leaf mass — `fprintf` (1,138 leaves) and `snprintf` (1,044) are the sole
blocker of one function and none, measured. Landed, each read line-cited from
the pinned checkout before it was declared:
`cfl/sds_lifetime.qcore` (`cfl_sds_destroy`, 8 functions),
`librdkafka/kafka_mem.qcore` (`rd_kafka_mem_free`, 4),
`cmetrics/context_lifetime.qcore` (`cmt_destroy`, 3),
`librdkafka/cjson.qcore` (`cJSON_Delete`, 3),
`monkey/logging.qcore` (`mk_print`, 3, the card's one variadic tail) and
`ctraces/context_lifetime.qcore` (`ctr_destroy`, 2) — the corpus's **first
`ctraces` family**. The nullability markers are deliberately **not uniform**:
`nullable` for `cfl_sds_destroy` (first statement `if (!s) return;`) and
`cJSON_Delete` (`while (item != NULL)` precedes every dereference, and all
three call sites *reach* that null path), `nonnull` for `cmt_destroy` and
`ctr_destroy` (no guard, first executable statement dereferences the
parameter). Prediction committed **before the AFTER run finished**: 204 leaves
and 23 functions. Measured: **204 leaves exactly**, and **20** functions, not
23. The three-way split — **20 leaf-free** (all `Unknown → Safe`), **3
exchanged for another refusal**, **0 unmoved**, so no declaration failed to
decide its own leaf and **none of the six was withdrawn** by constraint 8's
gates. The two breakages are the card's real findings. The three `cJSON_Delete`
functions are `cJSON_Delete(cJSON_DetachItemFromArray(...))`; decided leaf,
then a *new* refusal — `nested call … inside a modeled call is outside the
resource projection` — that was unreachable while the outer call was
uncontracted (`CJSON-DELETE-NESTED-DETACH`), which makes a sole-blocker cover
an **upper bound** and never a forecast (`W36-COVER-HIDDEN-BLOCKER`). And
`rd_kafka_mem_free`'s own defining function moved **`Safe → Unknown`** on the
corpus's first **`summary_conflict`**: W4 derives `Released{nonnull}`, the
declaration says `nullable`, and the declaration is the faithful one —
`rd_free(NULL)` reaches `free(NULL)`, and declaring `nonnull` to silence the
conflict would assert a precondition the body does not have, which constraint 4
forbids. That is constraint 8's gate working (reported, never silently
resolved), in the conservative direction, at a net of +4/−1 for that
declaration (`KAFKA-MEM-FREE-SUMMARY-CONFLICT`). Corpus-wide: **217 leaves
removed, 128 added, net −89**; projected functions **186 → 205 Safe**, Unsafe
**84 unchanged**, Unknown 4,564 → 4,545; **zero unit status moves**; `leak`
**272 → 272, unmoved**, so constraint 7's pin needed no re-pin at all;
`unsupported` 50,106 → 49,922, `unknown_alias` 4,214 → 4,308 — every added leaf
in the accusing direction, and the rising-alias/falling-unsupported shape is
what modelling a release *does*, not a regression. `chunk-contracts.json` is
byte-identical between the two stages. Read and **deliberately not declared**,
with the readings published: `add` (3 functions) — three distinct definitions
with three signatures, the cmetrics one `static inline` with no header
declaration for constraint 8's ABI gate to agree with (`ADD-NAME-COLLISION`);
and `memcpy`/`memset` (9 functions, 130 leaves) — both return `void *` aliasing
their first parameter and v1 has no result mode that says so, a **vocabulary
gap** rather than a judgement that they are undeclarable
(`LIBC-MEM-RESULT-ALIAS`). No Lean module is added, on
`W31-CORPUS-CONTRACTS-2`'s recorded ground and stated rather than assumed: a
grown premise set is not a semantic change, and the vocabulary, loader and
checker are all unedited. Corpus 76 → **82** modules, manifest-selected 21 →
**27**, resource-v1 declarations 43 → **49**, every constraint-5 pin updated
deliberately (`qcore_contract_corpus.rs`, `contract_loader.rs`'s
`(21, 43, 145) → (27, 49, 145)` and its explicit old-table parity rows,
`prod_contracts.rs`, `neighbours.rs`). Gate
`submodule=0 test=101 fmt=0 clippy=0 probe=0 followup=0 lean=0 pytests=0` —
**4,579 passed, 1 failed, 8 ignored**, 132 Lean audits, both probes exit 0. The
one red row is the one the card brief flagged in advance,
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
and it failed on a **one-second wall-clock assertion**
(`standalone_host.rs:175`) over an 8 MiB pipe write and a process spawn against
a 40 ms declared timeout. Reported with load and repetitions as the brief
requires: it failed at load average **~40 on 16 cores** with sibling wave-36
cards building (`rust-lld` at 300%/150%/141%), and passed **12 of 12** re-run
in isolation at load 23.7–27.6. The card changes no crate source and
`qcore-verifier-host` never reads the contract manifest, so the row is offered
as constraint 16's **sixth** load-sensitive member — or as evidence that
`e3d3a2ce`'s repair is incomplete, the integrator's call
(`STANDALONE-HOST-CLOSED-INPUT-LOAD`).
`docs/reports/w36-corpus-contracts-3/README.md`.

**`CARD-SCOPE-GUARD` (2026-09-18)** answers `CARD-SCOPE-SELF-MERGE` with a
mechanism rather than another instruction, and its interest is how much of the
obvious design was **falsified by measurement**. The wave-33 defect was a *local*
ref update — `git -C <main checkout> merge` from an agent worktree — so a
`pre-push` hook never sees it and the git process's own directory is the main
checkout. The guard is therefore a **`reference-transaction`** hook (git ≥ 2.28;
host 2.53.0), which runs on every ref update from every worktree and aborts by
exiting non-zero in the `prepared` state, with `pre-push` beside it for the
remote door. The first design read the caller's exported `PWD`, reasoning that
`git -C` moves the process but not the variable; measured, **git rewrites `PWD`
for its own hooks**, and measured again, **a POSIX shell re-derives `PWD` from
`getcwd()` at startup**, so that arm could not fire on any input at all — it was
removed as vacuous rather than shipped green, with a test now pinning the
measurement so it is not re-added. What does survive `git -C` is **process
ancestry**: the card's own shell and session remain ancestors of that git
process, so the arm that catches the incident shape reads `/proc` upward for an
ancestor whose directory is a delegated worktree (Linux only, stated). A third
measurement: `update-ref -d refs/heads/main` reaches the hook as
`0000…→0000…`, and the first draft's `old == new` no-op filter therefore
**deleted `main` and reported success** — red before green. A fourth decided the
veto's scope: all 15 Agent-tool worktrees on this host are `agent-<hex>` while
the delegating session sits in a *named* one, so vetoing every
`.claude/worktrees/` path would have refused the integrator and left nobody able
to advance `main`. Two layers stand: a provenance veto no authorisation
overrides, and a token making refusal the default for everyone, with the
`CLAUDE_CODE_CHILD_SESSION` signal **calibrated at activation** against the
integrator's own environment instead of assumed. 27 tests drive it in throwaway
repositories, and **11 of 11 mutants die, no survivors**. Two things are not
claimed: the hooks are **inert** until the integrator runs
`python3 scripts/install_main_guard.py` once in the main checkout — a step a card
cannot take, which is why the residual is amended and kept open rather than
retired — and a hook is **bypassable by intent**, so this guards a card's mistake
and nothing more. `CLAUDE.md`'s worktree hygiene also gains the harness hazard
that surfaced mid-card: an isolation worktree with an unchanged tree is removed
even while its agent's background jobs run (`WORKTREE-AUTOREMOVAL-PRISTINE-TREE`).
No crate source, no `proof/`, no pinned artifact: the workspace is byte-identical
to `main`'s at `60958bf0`. `docs/reports/card-scope-guard/README.md`.

