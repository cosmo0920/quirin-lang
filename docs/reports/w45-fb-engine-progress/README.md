# W45-FB-ENGINE-PROGRESS: do the engine callbacks make progress?

Card W45-FB-ENGINE-PROGRESS, 2026-09-26, base `37e1b82d` ("Record the
w45-refuted-contract-withdrawal integration gate"), Fluent Bit
`046accf6aded7906f66a1baf0202da349e521736`. This is the first stage of the
plan's P2 card [FB-ENGINE-PROGRESS](../../plans/fluent-bit-system-coverage/README.md#fb-engine-progress),
which is measurement only:

> Can a selected engine callback consume unbounded work without yielding, or
> revisit unchanged state indefinitely after an error/retry outcome?

The card changes no analyzer, contract, provider or Lean code, and no Quirin
verdict moves. **Quirin proves resource safety, not progress.** This card adds
no progress verdict and claims none.

Source reading, the Quirin audit, native observation and universal argument are
kept apart below. Every native result is a *local observation under an injected
premise*. None is an upstream defect claim, a confirmed bug or a production
failure rate, and nothing was sent upstream.

## Headline

* **Census.** The eight system-selection files contain 121 loops in 72
  functions, and every loop is bound to file:line at `046accf6`
  ([census.json](census/census.json), schema
  `w45-fb-engine-progress/census/v1`). Macro loops are placed at the macro's
  use.
  * **115 bounded** by a stated quantity;
  * **2 decreasing** under an explicit premise;
  * **1 yielding** (the engine's event loop);
  * **3 unresolved**: `cb_queue_chunks` (sb.c:611), the ring-buffer drain
    (flb_input_chunk.c:3746) and `random_uniform` (flb_scheduler.c:106).

  The census also has one recursion cycle (decreasing) and five
  cross-invocation revisit sites.
* **The top candidate.** The storage-backlog collector `cb_queue_chunks` has
  a zero-progress arm, and it runs on the engine thread. `sb.c` and ChunkIO
  are byte-identical between `864c3d97` and `046accf6`, so the observation's
  lines are unchanged: the loop at 611-612, the call at 633, the arm at
  649-650 and the return at 724.
* **Observations at `046accf6`**, on an unmodified Debug build under gdb:
  * **E1**: a persistent `CIO_ERROR` reproduces the earlier no-yield result.
  * **E2**: *one* transient `fstat` failure is enough. It leaves the chunk's
    descriptor open, every later bring-up fails, and the collector does not
    return (12,000 failed bring-ups in 6.8 s).
  * In E1 and E2 the engine dispatched **0** flush timers. The two controls
    dispatched 9 each.
  * **Controls**: C1 (valid) and C2 (a transient `open` failure, which leaves
    no descriptor) both drain the queues and yield.
  * **E3**, the retry revisit: while a retried chunk cannot be brought up,
    the retry is rescheduled 11 times with `attempts` stuck at 1 despite
    `retry_limit 3`. Control C3 spends attempts 1, 2, 3 and drops the chunk.
* **Quirin's view.** The resource audit gives no loop a progress meaning. Of
  the 121 loops:
  * 112 are in functions that are Unknown;
  * 9 are in functions that are Safe *for resources*, including
    `sched_timer_coro_get_id`'s two nested loops.
  * Four loops carry a loop-condition refusal.
* **Implementation: not cut.** 29 loops have a local counted-loop shape that
  a checked measure could state. That is a new analyzer rule with a Lean
  theorem, so it is registered as `W45-FB-ENGINE-PROGRESS-MEASURE`, with its
  exact property, slice and controls (§6).
* **Slice.** units 0 / 2 / 13 and functions 28 / 2 / 204 / 3, identical to
  `main`'s archived slice with **0 differing function rows**
  ([comparison](artifacts/slice-vs-w45-refuted-contract-withdrawal.txt)).

## 1. Build and evidence binding

The slice and the loop inventory use one prepared attempt:

* **Preparation.** `scripts/prepare-attempt.sh attempt-1 BINARY`, which is
  `quirin-audit prepare` with `--output
  /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-engine-progress/attempt-1`.
  The binary is `quirin-audit` built at `37e1b82d`, sha256 `d7377944…`.
* **The clone.** It is a `--no-hardlinks` clone of the user's checkout,
  detached at the pin. The checkout's HEAD was confirmed as `046accf6`, and
  the checkout itself was only read.
* **The Debug build.** It is built out of tree from that clone by
  `scripts/native-build.sh`: Clang 21.1.8, CMake 4.2.3, Ninja 1.13.2,
  `-DCMAKE_BUILD_TYPE=Debug`, with the backlog card's feature switches.
  Binary sha256 `7e535c9e…`; `--version` reports Git commit `046accf6…`.
* **The clean check.** `prepare` was then re-run over the same output (exit
  0). It refuses a clone whose HEAD differs from the pin or that has tracked
  changes. Untracked files are not checked: the build is out of tree.
* **The slice ran twice.** The first run (`slice-37e1b82d`) used compile
  database `f2e3d8c2…`. The re-check's configure step rewrote the database
  (`8e473753…`), so the slice was run again (`slice-37e1b82d-final`). The
  function rows are identical in the two runs and against `main`'s archive.
  The census and the published [invocation](artifacts/slice-invocation.json)
  use the second run, on the same database as the loop inventory.
* **Worktree git guard.** This card's worktree guard refuses the card's own
  `git` commands on paths under `private/`. The clone, the status check and
  the revision check were therefore all done by `prepare`, never by a `git`
  command issued here.

[native/evidence.json](native/evidence.json) (`scripts/bind_native.py`)
records, for every run:

* the configure command, the binary hash and the fixture;
* the command, watchdog exit and PID;
* the gdb script's and configuration's hashes;
* the full log's hash;
* the `[wd]` summary and wall-clock stamps;
* counts of Fluent Bit's own log lines that corroborate the gdb counters.

The full logs stay private. Every run had a `timeout -k 5` watchdog and exited
0 through gdb's own cap. No process was left behind, and only captured PIDs
were used.

**The fixture** (`scripts/make-fixture.sh`, the backlog card's): a `dummy`
input with filesystem storage and two `tcp` outputs to closed ports. It gives
13 chunks of 36,864 bytes, each shared by two output backlog queues. Every
backlog run restores the pristine copy and refuses to start unless the
restored chunks' SHA-256 listing matches.

Seven earlier runs used earlier revisions of the same scripts and reached the
same outcomes. They are listed under `superseded_runs` and are not the
evidence. The differences were:

* the collector breakpoint was set by bare name, which also matched
  `in_emitter`'s `cb_queue_chunks`, though no emitter was configured;
* the caps were 400 and 2,000 observations, which is too short against the
  1 s flush timer;
* there was no flush counter.

## 2. Census

**Method.** The mechanical inventory and the hand classification are
published separately, and the census joins them.

* `scripts/loop_inventory.py` runs each file's own compile record with
  `-fsyntax-only -Xclang -ast-dump=json`. It lists every `for`, `while` and
  `do` in every function defined in the file ([loop-inventory.json](census/loop-inventory.json)).
  For each loop it records the exits inside it and the calls in its body.
  It also finds the slice's direct-call recursion cycles.
* `scripts/classification.py` is the hand classification from source
  reading. It gives each loop a role, entry, class, bound or measure,
  premises, progress variable, blocking calls, retry scheduling and
  expressibility.
* `scripts/census.py` joins the two with the slice report. It refuses to
  publish if any loop is unclassified or any row names no loop, so the join
  is fail-closed in both directions.

**Premises** are named once in `census.json`:

* PL: the list is finite and well formed, and callees do not insert into it;
* PC: the instance lists are fixed while the engine runs;
* PF: the saved successor survives the body's callees;
* PW: at most one wrapper per chunk per queue;
* PR: no concurrent ring-buffer producer;
* PU: `cio_chunk_up_force` returns neither `CIO_ERROR` nor `CIO_RETRY`;
* PE: the exponent is non-negative;
* PRAND: 1 ≤ range ≤ `RAND_MAX`;
* PK: the pinned macro constants;
* PN: the counter and its bound are not written in the body.

| Role | Bounded | Decreasing | Yielding | Unresolved |
|---|---|---|---|---|
| engine loop and its per-round cleanup | 3 | | 1 | |
| engine callbacks (flush timer, collectors, scheduler timers and requests, output events) | 58 | 1 | | 3 |
| ingestion (input append, release-space eviction) | 17 | | | |
| shutdown (STOP, grace timer) | 8 | | | |
| start-up, configuration, exit | 27 | 1 | | |
| no caller found | 2 | | | |

**Batch limits.** Only the engine loop's inner batch has a per-call work
limit: at most `FLB_ENGINE_LOOP_MAX_ITER` = 10 events per round
(flb_engine.c:1454). The output-event reader has one too: at most
`FLB_ENGINE_OUTPUT_EVENT_BATCH_SIZE` = 1 status per read (flb_engine.c:909).

Every other callback loop is bounded by a list length or a count fixed at
entry, with no batch limit. For example, one ingestion's eviction search is
bounded by outputs × inputs × 2 passes × chunks (flb_input_chunk.c:1143, 568,
569). The bound on events per round counts events, not the work each event's
handler does.

**The rows that are not simply bounded:**

| Id | Site | Class | Measure or premise |
|---|---|---|---|
| EP-L012 | `cb_queue_chunks`, sb.c:611-722 | unresolved | Decreasing in the total queue length only under **PU** (below). The `CIO_ERROR`/`CIO_RETRY` arm (649-650) changes no state. Runs on the engine thread. |
| EP-L047 | `flb_input_chunk_ring_buffer_collector`, flb_input_chunk.c:3746-3780 | unresolved | `while (1)` that exits when the ring buffer is empty or the input pauses. Bounded by capacity only under **PR**. Runs on the engine thread with no batch limit. |
| EP-L096 | `random_uniform`, flb_scheduler.c:106-108 | unresolved | Rejection sampling: it ends with probability 1 under **PRAND**, with no deterministic bound. range = 0 divides by zero at :102. |
| EP-L099 | `ipow`, flb_scheduler.c:237-247 | decreasing | `exp >>= 1`, at most 32 iterations under **PE**. A negative `exp` stays at −1. |
| EP-L045 | `flb_input_chunk_ring_buffer_cleanup`, flb_input_chunk.c:3726-3731 | decreasing | One entry per read under PR (input destroy). |
| EP-L088 | `flb_engine_start`, flb_engine.c:1450-1661 | yielding | Blocks in `mk_event_wait` each round and leaves only by return on shutdown. |
| EP-R1 | `flb_output_task_flush` → `flb_output_task_queue_flush_one` → `flb_output_task_singleplex_flush_next` (flb_output.c) | decreasing | The pending queue length. Each re-entry follows a failed flush and consumes one pending entry. |

**Ranking.** `cb_queue_chunks` ranks first, for three reasons:

* it runs on the engine thread, with no batch limit;
* its failure arm is a genuine zero-progress arm, not a workload premise;
* its trigger is a return code of a file operation.

The ring-buffer drain ranks second: its unbounded case needs a workload (a
producer that keeps pace), not a failure. `random_uniform` ranks third: its
premise is a configuration ordering.

**Revisits across invocations.** In these sites the callback returns, but the
next invocation meets the same state:

| Id | Site | Revisit, and what releases it |
|---|---|---|
| EP-V1 | flb_engine_dispatch.c:114-131 | A retry whose chunk cannot be brought up is rescheduled without spending an attempt. **Observed (E3).** |
| EP-V2 | flb_engine_dispatch.c:101-111 | A retry is deferred without spending an attempt while `flush_in_flight` is at its limit. Released by in-flight completions. |
| EP-V3 | flb_engine_dispatch.c:396-421 | A chunk with no buffer or no tag is skipped on every flush. |
| EP-V4 | flb_scheduler.c:193-228 | A request that cannot get a timer is re-queued every frame. |
| EP-V5 | sb.c:602-607 | The collector returns at once while the queued total is at `mem_limit`. Released only if the outputs drain. |

## 3. Native observations

These are separate from the arguments in §4. Every run is at `046accf6`, on
the unmodified binary, and each line is bound in
[evidence.json](native/evidence.json). "Yields" counts returns at both of
`cb_queue_chunks`' return points (sb.c:606 and sb.c:724). "Flushes" counts
`flb_engine_flush` calls after the collector's first entry.

| Run | Injection (gdb script) | Enter / yields | Bring-ups (failed) | Queued | Flushes | Wall time |
|---|---|---|---|---|---|---|
| **C1** valid control | none ([c1](native/gdb/c1-valid-control.gdb)) | 10 / 10 | 13 (0) | 13 | 9 | 9.0 s |
| **E1** persistent `CIO_ERROR` | every collector-originated `cio_chunk_up_force` returns −1 ([e1](native/gdb/e1-persistent-error.gdb)) | 1 / **0** | 12,000 (12,000) | 0 | **0** | 5.1 s |
| **E2** one transient `fstat` failure | once: `cio_file_update_size` takes its failure arm (`fs_size = 0`, `CIO_ERROR`, cio_file.c:912-916) ([e2](native/gdb/e2-oneshot-fstat.gdb)) | 1 / **0** | 12,000 (12,000; 11,999 at "file descriptor already exists") | 0 | **0** | 6.8 s |
| **C2** one transient `open` failure | once: `cio_file_native_open` returns −1 with the descriptor still −1 ([c2](native/gdb/c2-oneshot-open.gdb)) | 10 / 10 | 14 (1) | 13 | 9 | 9.0 s |

Fluent Bit's own log agrees with the gdb counters:

* **C1 and C2** each log 13 `queueing` lines.
* **E2** logs 11,999 `file descriptor already exists` lines.
* **E1 and E2** log no `queueing` line.

In E2 the first bring-up of the head chunk failed at the injected `fstat`
(descriptor 20 was already open). That chunk heads both queues, and every
later bring-up of it failed at `cio_file.c:827-830`.

| Run | Injection | Retry dispatches | Attempts seen | Reschedules | Dropped | Time span |
|---|---|---|---|---|---|---|
| **C3** retry control | none ([c3](native/gdb/c3-retry-control.gdb)) | 3 | 1, 2, 3 | 0 | **1** ("cannot be retried") | 5.0 s from the first retry to the drop |
| **E3** chunk cannot be brought up | `ret = -1` after `flb_input_chunk_set_up` in `flb_engine_dispatch_retry` (flb_engine_dispatch.c:115), every dispatch ([e3](native/gdb/e3-retry-setup-error.gdb)) | 12 (cap) | 1 throughout | 11 | **0** | 26 s |

The E3 fixture is [retry.conf](native/conf/retry.conf): one record, filesystem
storage, one closed-port `tcp` output, `retry_limit 3`, `scheduler.base 1` and
`scheduler.cap 2`. Its −1 is the value `cio_chunk_up` returns when
`storage.max_chunks_up` is reached (cio_file.c:837-841). The source comment at
flb_engine_dispatch.c:116-123 describes the reschedule as intended.

## 4. Arguments (source reading, universal under stated premises)

**`cb_queue_chunks` under PU.** Consider a pass over the queues in which some
queue is non-empty. On that queue's head, every arm except
`CIO_ERROR`/`CIO_RETRY` calls `sb_remove_chunk_from_segregated_backlogs`:

* the corrupted arm (635-647);
* the bad-metadata arm (663-671);
* the empty-chunk arm (677-684);
* the map-failure arm (691-703);
* success (712-716).

Each of these removes the head wrapper. So a pass removes at least one wrapper
or finds every queue empty. The total queue length is a measure, and under PU
the outer loop runs at most that length plus 1 times. `total < mem_limit`
gives a second exit.

Without PU, no measure exists, because the failing arm writes nothing. E1
shows the premise can fail. E2 shows it fails *persistently* after a single
transient failure, by this chain:

1. `_cio_file_up` opens the file (cio_file.c:845).
2. On a `cio_file_update_size` failure it returns `CIO_ERROR` without closing
   the descriptor (853-856).
3. The chunk is then "not up" (mapped is false) but open, so every later
   `up_force` returns at the open-descriptor check (827-830).

C2 isolates the stickiness. An `open` failure leaves no descriptor, and the
next attempt succeeds.

The mmap-failure arm of the same function reaches a different state, map =
`MAP_FAILED`. That state is recorded separately as a source-reading candidate
(`W45-FB-CIO-MAP-FAILED-STATE`) and was not exercised.

**Consequence on the engine.** The storage-backlog collector is a time
collector of a non-threaded input (sb.c:769). It runs inside the engine loop's
inner batch (flb_engine.c:1454-1456 → `flb_input_collector_fd`). While it does
not return, no other engine event is handled, and E1 and E2 dispatched no
flush timer. This is a statement about this collector under PU's failure. It
says nothing about latency or fairness when the collector does return.

**Retry revisit (EP-V1).** The −1 arm of `flb_engine_dispatch_retry`
(114-131) calls `flb_task_retry_reschedule`, which reuses `retry->attempts`.
Attempts are spent in only two places:

* `flb_task_retry_create` (flb_task.c:356), reached only after a flush;
* the no-buffer arm of the same function (flb_engine_dispatch.c:146-156),
  reached only after the chunk is up.

So under a persistent "cannot bring up" premise, `retry_limit` never ends
the retry.

## 5. Quirin's view

For each loop, [census.json](census/census.json) records the resource audit's
own view of the enclosing function, from this card's slice run: its status in
the static and shared variants (identical in every pair), its leaf counts, top
reasons, the leaves inside the loop, and any loop-condition refusal. Its
`progress_verdict` field is `null` by construction.

* **112 loops are in Unknown functions** (static variant). Their reasons are
  the resource refusals already attributed by the slice cards, mainly
  indirect accesses, uncontracted callees, unresolved identities and branch
  conditions.
* **The four unresolved and decreasing callback rows** are all in Unknown
  functions:
  * `cb_queue_chunks`: 26 leaves, including "unmodeled call, aggregate or
    indirect access in loop condition" at sb.c:611, and its checked
    specification is not established;
  * the ring-buffer collector: 9 leaves;
  * `random_uniform`: 3 leaves;
  * `ipow`: 2 leaves.
* **Four loops carry a loop-condition refusal**: sb.c:611,
  flb_input_chunk.c:3726, flb_task.c:325 and flb_engine.c:909. The last is
  the output-event reader, whose bound is the constant 1.
* **9 loops in 7 functions are in Safe functions**: `dlq_stream_total_size`,
  `sched_timer_coro_get_id` (two nested loops), `sched_timer_coro_get_by_id`,
  `flb_sched_timer_coro_get`, and four `do { } while (0)` macro bodies in
  three start-up functions. Safe there is a resource verdict; for the five
  list traversals it holds relative to the list-iteration premises of
  gateway 5l. It **says nothing about termination**.
* **The recursion cycle EP-R1** runs through `flb_output_task_flush`, which
  is provisional Unsafe on its refuted OR-7 declaration
  ([W45-REFUTED-CONTRACT-WITHDRAWAL](../w45-refuted-contract-withdrawal/README.md)).
  That is a resource candidate, not a progress one, and not an upstream
  defect claim.

**Expressibility**, per loop in `census.json`'s `expressible_as`:

* **29 loops could carry a checked measure local to the loop**:
  * 3 counted against a constant (the two batch limits and the two-pass
    eviction);
  * 11 against a local, parameter or field not written in the loop;
  * 10 against a `uint16_t` route count read from chunk metadata, at most
    65,535;
  * 1 right-shift measure (`ipow`), which needs `exp ≥ 0` as a
    precondition contract;
  * 4 `do { } while (0)` bodies.
* **87 are list-length bounds.** They need a contract premise that the list
  is finite and that the body's callees do not extend it. That is PL, which
  cannot be derived locally. The list-iteration rules of gateway 5l already
  carry that premise for resource purposes.
* **4 have no measure without an unestablished premise**: EP-L012, EP-L047,
  EP-L096, and EP-L045 outside its lifecycle premise.
* **1 is the service loop.**

## 6. Implementation decision

The plan's entry condition is "one precise, checkable property". The census
shows several (§5), but each needs a **new analyzer rule with a Lean theorem**:
Quirin's existing-C gateway has no loop measure today (constraint 10b checks
loops for resources, not termination). A provider contract cannot state a loop
bound either: the resource-v1 vocabulary has no bound clause.

Cutting that rule here would widen the card from measurement into a new
semantic mechanism. So it is registered instead as
**`W45-FB-ENGINE-PROGRESS-MEASURE`**, with this property:

* **The loop shape.** A `for` loop whose counter is a local integer set in
  the init clause and tested `counter < B`, where B is one of:
  * a literal or macro constant;
  * an unwritten, non-address-taken local or formal;
  * a `uint16_t` value.

  The counter is advanced by exactly one `counter++` and is not otherwise
  written or address-taken.
* **What is recorded.** The bound B − c0 as a checked measure, with a Lean
  theorem that the measure is non-negative and strictly decreasing. It is a
  bound fact beside the resource verdict: no verdict moves, and no progress
  verdict is produced.
* **Accepting controls**: `handle_output_events` (≤ 1),
  `flb_input_chunk_get_direct_routes`:2286 (≤ 65,535) and
  `flb_task_create`:652.
* **Refusing controls**, each Unknown: `cb_queue_chunks` (no counter), `ipow`
  without the precondition, and `random_uniform`.

That property bounds work per call. It does not bound latency, and it would
not decide EP-L012, whose missing premise is a ChunkIO return-code fact.

## 7. Limits

* **Bounded memory does not imply progress.** The backlog queues stay bounded
  in E1 and E2 while nothing is queued.
* **Bounded work does not imply latency.** A bounded loop whose bound is a
  list length can still run long when the list is long. Only two loops have
  a per-call batch limit.
* **A callback returning does not imply fairness.** In E3 the retry callback
  returns every time and still revisits the same state. The engine loop's
  batch of 10 events is not a fairness guarantee among inputs or outputs.
* **A permanent output failure may prevent drain without violating safety.**
  C1's and C2's outputs never accept a byte (closed ports); the queues drain
  into tasks and retries, not to a destination. EP-V5 releases only if the
  outputs drain.
* **The observations are local.** Each result holds on one host, for one
  build, under an injected premise. No production hang, hang rate or
  upstream defect is claimed. Filing upstream is the user's decision.
* **No verdict moved.** No `Unknown` became `Safe` or `Unsafe` because of a
  native observation.

## 8. Validation

Gate commands, exits and counts are in [gate.json](gate.json). The corpus
arbiter was not run: no analyzer, contract or provider code changed, so it
cannot move. The slice was run (§ Headline, [invocation](artifacts/slice-invocation.json)).

## Follow-ups

This card registered eight entries ([followups.json](followups.json), and the
acceptance matrix):

* `W45-FB-ENGINE-PROGRESS-MEASURE`, the implementation card (§6);
* `W45-FB-BACKLOG-TRANSIENT-STICKY-ERROR` (E2, observed-local);
* `W45-FB-RETRY-UNSPENT-RESCHEDULE` (E3, observed-local);
* `W45-FB-RING-BUFFER-DRAIN-UNBOUNDED`;
* `W45-FB-SCHED-RANDOM-RANGE`;
* `W45-FB-ENGINE-REVISITS-UNEXERCISED` (EP-V2 to EP-V4);
* two out-of-scope source-reading candidates found while reading:
  * `W45-FB-CIO-MAP-FAILED-STATE`;
  * `W45-FB-BACKLOG-EXIT-WRAPPERS`, where `sb_destroy_backlogs` unlinks a
    queue before destroying its wrappers.

The card closes no entry. The earlier `FB-AUDIT-03-ZERO-PROGRESS` stays as its
card recorded it, at `864c3d97`. E1 is its re-observation at `046accf6`.
