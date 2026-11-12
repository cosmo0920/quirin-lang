# Fluent Bit system coverage: review and prospective card queue

Review date: 2026-09-16. Status: proposed follow-on definitions, not dispatched
implementation or completed verification. Priority is relative within this
queue; it does not supersede existing soundness repairs, G1/G4 ordering, or the
integrator's wave selection. This queue extends Cycle 19C and W6 planning.

## Review result and evidence boundary

CT scenarios transfer by **property**, not by library name. Ownership, shared
identity, error cleanup, outstanding borrowers, and budget conservation are
useful beyond Libevent, librdkafka, ChunkIO, CMetrics and CTraces. They can guide
checks of backlog, routing, retries and engine coordination. They are neither
an exhaustive Fluent Bit requirements catalogue nor a production coverage claim.
An asynchronous Fluent Bit task/callback is not automatically a QCore scoped
task; the source-to-model relationship is an obligation of each implementation.
Libevent's event profile does not automatically cover Monkey's engine backend.

At review time, `python3 scripts/ct_stage_summary.py . --check` exited 0:
45/50 mapped, 3 case-bound proof rows (CT01, CT03, CT37), 18 native-execution
rows and 9 gated sanitized-native rows. These are separate evidence dimensions.
The five unmapped cases are CT10, CT24, CT41, CT42 and CT46. A production card
below does not map one of them merely by using the same concept. Even 50/50
mapping would not establish full language conformance or Fluent Bit correctness.

The acceptance matrix's generated CT notes still contain stale statements that
all proof stages are unassessed and only four cases have gated sanitizers;
its current derived rows disagree. Read the manifest and per-card records for
evidence. A later documentation repair must fix the generator's prose, not
hand-edit generated notes or historical reports.

The upstream `master` source inspected for this review is orientation only:
[backlog](https://github.com/fluent/fluent-bit/blob/master/plugins/in_storage_backlog/sb.c),
[engine](https://github.com/fluent/fluent-bit/blob/master/src/flb_engine.c),
[tasks](https://github.com/fluent/fluent-bit/blob/master/src/flb_task.c), and
[retry policy](https://docs.fluentbit.io/manual/administration/scheduling-and-retries).
It is not the pinned corpus and supplies no validated defect finding. The first
card must establish exact revision/build/source identity before using any path.

Existing constraints particularly relevant to this review:

- [Cycle 19C](../../cycles/CYCLE_19.md) already requires identity-indexed chunk
  lifecycle, route/task custody, accounting and independent failure paths.
- [Chunk residency declarations](../../../providers/qcore-provider-chunkio/contracts/chunkio/chunk_residency.qv)
  are trusted custody premises, not proofs of successful residency transitions
  or capacity enforcement. [Counter declarations](../../../providers/qcore-provider-chunkio/contracts/chunkio/counters.qv)
  require range and exclusivity premises; callers must establish them.
- [Busy-state contract](../../../providers/qcore-provider-chunkio/contracts/fluent_bit/busy_state.qv)
  distinguishes the wrapper busy flag from the underlying chunk lock.
- [CT24's withdrawn mapping](../../reports/ct-wire-lane-3/README.md#4-ct24--withdrawn-with-the-measurement)
  demonstrates why refusal for placement is not evidence about cleanup duties.
- [Cycle 18](../../cycles/CYCLE_18.md) does not treat readiness, priority or
  observed order as fairness or synchronization evidence. CT39 likewise refuses
  substituting concurrent work for wall-clock time.

## Priority queue and dispatch order

| Order | Card | Kind | Prerequisite | Intended decision | Status |
|---|---|---|---|---|---|
| P0 | FB-SYSTEM-CENSUS | Measurement | Pinned build available | Select reachable, attributable production slices and measure missing mechanisms | Done: [W45-FB-SYSTEM-SELECTION](../../reports/w45-fb-system-selection/README.md) (2026-09-26); residuals in `W45-FB-SYSTEM-SLICE-RESIDUALS` |
| P1.1 | FB-BACKLOG-CUSTODY | Semantic implementation | Census selects backlog slice | Preserve shared chunk identity and lifetime through queue/error transitions | |
| P1.2 | FB-BACKLOG-ACCOUNTING | Semantic implementation | Census plus agreed custody/identity interface | Establish per-transition byte/count accounting and the actual admission bound | |
| P1.3 | FB-RETRY-LIFETIME | Semantic implementation | Census selects dispatch/retry slice | Preserve task/chunk obligations through scheduling, completion and cancellation | First slice done: [W45-FB-RETRY-LIFETIME](../../reports/w45-fb-retry-lifetime/README.md) (2026-09-26; singleplex queue, OR-1 and OR-7 decided refuted); the rest of the retry route open |
| P2 | FB-ENGINE-PROGRESS | Measurement, then separately scoped implementation | Census; lifetime interfaces if traces share them | Distinguish bounded callback work from eventual service and throughput | Measurement done: [W45-FB-ENGINE-PROGRESS](../../reports/w45-fb-engine-progress/README.md) (2026-09-26; 121 loops at `046accf6`, E1–E3 observed-local with controls); implementation cut as [W45-FB-ENGINE-PROGRESS-MEASURE](../../reports/w45-fb-engine-progress-measure/README.md) (gateway rule 44: a checked counted-loop measure recorded beside the resource verdict, no verdict moved; slice run owed, `W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN`) |
| P3 | FB-SYSTEM-EXTENSIONS | Candidate inventory only | Findings from the preceding cards | Select one further subsystem per future card | |

P0 is the only immediately dispatchable card. P1 cards need the census's exact
functions, owned/planned paths, profiles and baseline attached to their briefs.
Do not promise one card can close a whole subsystem. If a missing generic
mechanism dominates a slice, split out that prerequisite and retain the
production slice as blocked with a named discharge condition. P1.1 precedes
P1.2 integration when both change shared identity semantics; P1.3 can be a
separate lane only after overlap is measured. P2's measurement can proceed
independently after P0. No wave numbers or agents are assigned by this document.

### Deferred contract-adoption continuation

[GUARDED-CONTRACT-ADOPTION-SUPPLEMENT](../../reports/guarded-contract-adoption/SUPPLEMENTAL_CARD.md) records the explicitly halted native acceptance work. It is deferred, not launched, and requires explicit human authorization before restart. Its canonical brief owns prerequisites, environment, frozen-input custody and acceptance; existing `PREMISE-FLOW-PRODUCTION-DISCHARGE` and the four original source residuals remain Open. This pointer does not change the production queue order or retire tracking.

### CT mechanisms available to later briefs

The original review figures above are a dated snapshot. Use [current status](../../STATUS.md)
for CT coverage and each linked report for its measured scope.

| Mechanism | Candidate production use | Obligation still owed |
|---|---|---|
| [CT24 cleanup/task composition](../../reports/ct24-cleanup-task-composition/README.md) | Backlog/retry destruction while a child retains a chunk share | Bind real queue transitions, owners, borrowers and failure arms to the custody model |
| [CT10 field footprints](../../reports/ct10-source-layout/README.md) | Concurrent writes to chunk or engine flags | Derive trustworthy target layout; the property-only successful-window check supplies no task-lifetime proof |
| [CT41 publication mapping](../../reports/ct41-publication-mapping/README.md) | Publishing initialized chunk/task state | Establish the declared invocation bounds and successful release/acquire observation; volatile is not synchronization |

These mechanisms supply no production verdict or throughput/fairness guarantee.
P0 remains first; its pinned source census decides whether a proposed transfer
fits the real system before a P1 implementation brief is dispatched.

## Card definitions

### FB-SYSTEM-CENSUS

**Question.** Which backlog/engine paths are already selected by the audit,
which are absent, and which checker gaps prevent a meaningful decision?

**Scope.** Inventory `plugins/in_storage_backlog/sb.c`, `src/flb_input_chunk.c`,
`src/flb_storage.c`, `src/flb_task.c`, `src/flb_engine.c`,
`src/flb_engine_dispatch.c`, and `src/flb_scheduler.c` where present at the pin.
Follow relevant output-worker/callback edges without silently extending scope.
Use the [persistent build inputs](../../CYCLE19_BUILD_INPUTS.md).

**Deliverables and acceptance.** Record the upstream revision, unmodified
compile database/task identities, selected functions and exclusions; identify
chunk/wrapper/task/retry identities, acquisition/publication/removal/destruction
sites, synchronization edges and failure outcomes. Publish a baseline with
Unknown reasons and denominator, distinguishing absent units from unsupported
paths in analyzed units. Select one bounded slice for each P1 card and predict
which obligations it could decide, not merely which refusal messages it could
replace. Record reuse of existing mechanisms and overlap with named follow-ups.
Zero measured reach is a valid reason to defer a proposed implementation.

**Boundary.** Measurement only: no semantic widening, contract additions,
incidental host fixes, verdict promotions or CT remapping. Its report may close
the census, not an implementation/proof card.

### FB-BACKLOG-CUSTODY

**Property.** Multiple output-queue wrappers may refer to one underlying chunk;
wrapper identity, delivery obligation and chunk ownership must remain distinct.
Every live reference resolves to the same live chunk generation. A destroy
requires discharge of all modeled references/borrowers and the applicable
delivery/drop policy, not just removal from one queue.

**Scope and evidence.** On the census-selected path, model queue insertion,
cross-queue removal, handoff into an input chunk/task, load failure and cleanup.
Document whether each edge borrows, transfers or retires an obligation. Supply
accepting controls for legitimate fan-out/removal and refusing mutations for
premature close, stale generation, double ownership and partial insertion
failure after an earlier insertion succeeded. Add audited Lean preservation and
bind the production import to the checked artifact; unsupported edges stay
Unknown. A synthetic queue alone does not complete the production leg.

**CT relationship and limits.** CT03–06, CT11, CT24–25, CT42 and CT50 are design
references, not new mappings. No exactly-once remote delivery or crash-recovery
claim follows from memory custody. Declared dropping policy must be explicit.

### FB-BACKLOG-ACCOUNTING

**Property.** Keep separate ledgers for unique physical resources (resident
bytes, mappings, descriptors), per-output delivery charges, and budget credits.
Shared chunks are counted according to each ledger's defined meaning. Every
successful or failed transition preserves the stated relation to concrete state;
checked arithmetic does not wrap or manufacture available capacity.

**Scope and evidence.** Define what each selected counter measures: content,
real file size, allocation or resident memory, including excluded overhead.
Derive the actual admission inequality from the pinned C. Determine whether a
limit is strict, allows chunk/batch overshoot, or is advisory; do not assume a
configuration name implies a hard bound. Exercise boundary sizes, shared chunks,
up/down failures, repeated removals and partial rollback. An admitting run and
mutations for missed decrement, duplicate increment and inconsistent size basis
must distinguish the intended invariant. Pair Rust decisions with audited Lean
accounting/bound theorems and source-bound evidence.

**CT relationship and limits.** CT35–38 guide sharing and peak-credit accounting;
CT39 prohibits a timing inference. A byte bound excludes RSS/OS cache bounds
unless explicitly modeled. A new ownership or budget discharge must satisfy
the fail-closed gates; arithmetic helper contracts alone are insufficient.

### FB-RETRY-LIFETIME

**Property.** Dispatch, completion, pending retry and cancellation carry explicit
task/chunk obligations. A failed scheduling operation cannot silently lose one;
an obsolete or duplicate completion cannot discharge a new generation's
obligation. Teardown cannot destroy callback state still reachable by admitted
in-flight work. Cancellation is not assumed to wait for callback completion.

**Scope and evidence.** Select one output route plus scheduler interaction,
with precise identities and transitions for success, retry, terminal failure,
schedule failure and shutdown. State the ordering/synchronization premises.
Exercise completion before observation, repeated reschedule, cancellation racing
with completion, failed retry allocation and teardown with outstanding work.
Include accepted retirement and refused stale/duplicate retirement controls,
audited Lean lifetime preservation, and production source/import validation.

**CT relationship and limits.** CT12–20, CT24–25, CT31, CT43–44 and CT50 inform
the tests. Do not replace asynchronous retry semantics with scoped join syntax
without proving the correspondence. No eventual network delivery, bounded retry
count or fairness claim without separate premises and policy checks.

### FB-ENGINE-PROGRESS

**Question.** Can a selected engine callback consume unbounded work without
yielding, or revisit unchanged state indefinitely after an error/retry outcome?

**First stage.** Inventory callback loops, batch limits, progress variables,
blocking calls, retry scheduling and yield/return points at the pin. Classify
each loop as bounded by a stated quantity, decreasing under explicit premises,
yielding, or unresolved. Include a zero-progress failure outcome and a valid
progressing control. Publish observations separately from universal arguments.

**Implementation entry condition.** Only after this measurement, cut one bounded
Rust/Lean card for a precise property: for example, at most N processed entries
per callback under specified call-cost assumptions, or a checked decreasing
measure. If remediation changes Fluent Bit scheduling, propose that upstream
change separately rather than silently changing the audited input.

**Limits.** Bounded memory does not imply progress; bounded work does not imply
latency; a callback returning does not imply fairness among outputs. Throughput
on few cores additionally needs workload/arrival assumptions, CPU and I/O costs,
scheduling behavior and measurements. Those are not supplied by CT mapping,
sanitizers or a successful native trace. Permanent output failure may prevent
drain forever without violating the selected safety property.

### FB-SYSTEM-EXTENSIONS

This is an ordered candidate inventory, not a dispatchable omnibus card:

1. Shutdown across inputs, outputs, timers and plugin state: outstanding
   callbacks/borrowers and partial-init cleanup; reuse FB-RETRY-LIFETIME first.
2. Worker queues and shared metadata: protected identity, stale refinements,
   lock order and handoff; CT26–34/48 are references, with unsupported mutex
   forms still refused and synchronization imported explicitly.
3. CMetrics/CTraces serialization and reset: borrowed payload lifetime,
   concurrent mutation and encoder-failure cleanup. Measure provider coverage
   before defining any new contract or inferring whole-library verification.
4. Record delivery and crash recovery: persist/ack/delete ordering, replay and
   duplicate/loss policy. Requires a new durability/protocol specification;
   neither chunk custody nor existing CT cases establish it.

Each selected candidate needs its own pinned slice, dependency list, accepting
and refusing controls, semantic/proof scope and production acceptance condition.

**Input pause/resume concurrency (candidate 5; bound, not implemented).**
[W47-TAIL-PAUSE-BINDINGS](../../reports/w47-tail-pause-bindings/README.md)
binds a design sketch of in_tail pause/resume contracts C1–C6 to Fluent Bit
`90978ac6` and measures its own slice (`tail-pause-selection.json`: 0 / 1 / 15
units, no concurrency verdict). The slice, its 60 bindings and its 9 negative
cases are the baseline for two cards:

1. `W47-LOOP-OWNER-CONTRACTS` (C1–C3): a derived thread-entry census, a
   loop-owner vocabulary for collectors and `mk_event_*`, and C2's "enqueue is
   not completion".
2. `W47-CONDVAR-JOIN-PUBLICATION` (C4–C5): condvar publication and
   join-before-release, which amends constraint 27. It follows card 1, whose
   thread inventory it needs.

C6, a request/acknowledgement API, waits for an upstream API
(`W47-TAIL-REQUEST-CONTROL-API`). Each card must reject its named negative
cases and freeze a prediction that names the corpus callers of
`flb_input_pause`/`resume` in `flb_input_chunk.c`.

## Implementation acceptance and wave integration

Every future brief must attach base/HEAD, owned and planned paths, applicable
AGENTS constraints, exact commands and report sections. Use generic core
mechanisms; keep Fluent Bit identities in adapters/configuration and production
fixtures. The verdict records artifact, property, profile, scope, bounds,
assumptions and evidence version. Missing source identity, synchronization or
call effects remains Unknown; v1 provider assertions cannot supply Safe.

Report model proof, artifact binding, production coverage and native observation
separately. Include controls that isolate the property (CT24's precedent), and
test whether failure arms are actually reached. A kernel theorem or a sanitizer
run alone is not evidence of whole-program correctness. Semantic changes need
matching audited Lean and the unchanged standing gate. Corpus-affecting cards
must reproduce the current baseline, attribute every verdict/leaf delta and
archive their own evidence; never rewrite a frozen report or absorb a leak move.

Keep production cases outside the CT namespace. Only a separate CT mapping lane
may update stage evidence for the case's own artifact. Follow acceptance-matrix
section ownership when registering implementation results; no count moves from
this planning review. At wave close, reconcile these card statuses and every
named residual with discharge conditions, then run the existing reconciliation
sweep. A measured blocker is a legitimate outcome, not permission to weaken a
property until its fixture passes.

## Validation of this planning edit

Documentation link checks, `git diff --check` and the CT stage consistency check
passed. The required standing gate ran with logs under the worktree-private
`private/fluent-bit-card-review/`: workspace tests exited 101 (4,543 passed,
1 failed, 8 ignored; 227 result summaries), fmt/clippy and both reevaluation
probes exited 0, and the Lean audit command exited 0 with 128 audits passed.

**FB-PLAN-GATE-HOST-TIMEOUT — RESOLVED AT THE WAVE-36 CLOSE; the paragraph
below is kept as the observation it was, and must not be read as a live
accusation** (`FB-PLAN-STALE-RED-ASSERTION`, raised by `STABILITY-REEVAL` #2,
which measured the test **green 4/4** while this text still asserted it red —
stale in the *accusing* direction, which is the direction that matters). What
was unknown here is now known, and neither half of the original finding
survives as written. The **cause** was a transport defect and not a host
condition: an 8 KiB pipe capacity against a 1 ms backoff quantum put a ceiling
on the 8 MiB-each-way arm, found and repaired with `poll(2)` by card
`VERIFIER-HOST-CONTROL-BUDGET` (wave 35), which changed
`crates/qcore-verifier-host/src/lib.rs` **only** and left the test file
untouched; the admitting control passes 38/38 after it. The **row** is
separately real and is now constraint 16's sixth load-sensitive row, sensitive
at the *top* of the range — measured failing 3 of 6 at load ≈100 (wave 35),
failing once in-gate at load ≈40 and passing 12/12 in isolation at load
23.7–27.6 (wave 36), and **passing** in the wave-36 close's own full-suite gate.
So it is an environment boundary with a measured mechanism, not an unresolved
validation finding, and the repair narrows the failure window without closing
it (`STANDALONE-HOST-CLOSED-INPUT-LOAD`). Verify it in isolation; never weaken
the 1-second bound. The original text, unedited:

> The unchanged
> `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
> test failed at `crates/qcore-verifier-host/tests/standalone_host.rs:212`: the
> large-response admitting control returned `Err(Timeout)` rather than its
> expected response under the 1,000 ms deadline. One isolated rerun also exited
> 101 at the same assertion. This is not classified as a flake or as the cause
> repaired by the earlier host card.

Original discharge condition, now met: a separately scoped host
investigation that establishes the cause on this environment and validates a
repair or records an evidenced environment boundary without weakening the
assertion/deadline. No host code changed here and no full-green gate is claimed.
