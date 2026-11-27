# W45-TSAN-ROWS-LOAD-SENSITIVE: why two TSan rows failed under load, and the fix

Card of wave 45, base `5e9630fe`. It is a test-harness change only:
`tests/support/thread_sanitizer.rs` and `compiler/tests/task_route_tsan.rs`.
No analyser, checker, emitter, verdict or Lean definition changes, so there is
no Lean parity obligation and the corpus arbiter was not run. No emitted byte
changes either: `the_binding_changes_no_emitted_byte` still passes.

## 1. What failed

Two integration gates in a row went red on one ThreadSanitizer "the race must
be reported" row of `task_route_tsan`. Each time it was a different row, and
the binary then passed 5 of 5 alone.

* W45-PARENTHESISED-CALLEE: `removing_the_join_edge_is_reported_under_every_schedule`,
  `shared_readers @ forced-together`. Both workers started and left. TSan
  printed only a **thread leak**, and no data race.
* W45-FB-SYSTEM-SELECTION: `overlapping_writers_are_reported_under_every_schedule`,
  `ct07-case @ forced-together`. Both workers started and left, and TSan
  printed **nothing at all**.

Wave 41 had registered the first shape as `TSAN-JOIN-EDGE-ROW-LOAD-SENSITIVE`,
without a mechanism.

## 2. Reproduction under controlled load

[`scripts/load.py`](scripts/load.py) runs the whole test binary `N` times with
`M` copies at once. The load is nothing but the copies themselves, on a 16-CPU
host with nothing installed. The `before` binary is built from the base tree.
The `after` binary is built from this card's harness commit. That was before
rustfmt re-wrapped one line and before the doc comments were added, so its code
is the same as the final tree's.

| concurrent copies `M` | runs `N` | before: red runs | after: red runs |
|---:|---:|---:|---:|
| 1 (alone) | 10 | **0** | **0** |
| 8 | 40 | **1** | **0** |
| 16 | 64 | **2** | **0** |
| 24 | 96 | **13** | **0** |
| 32 | 128 | **42** | **0** |

The per-row counts are in [`artifacts/load/`](artifacts/load/). Before the fix,
the red cells were:

* `overlapping_writers_…`, `ct07-case` and `disjoint-exercise`, under `forced-together`;
* `removing_the_join_edge_…`, `shared_readers` under both forced schedules and
  `ct07-case` under `forced-together`;
* at 32 copies, `a_join_deleted_is_caught_by_the_cohort_trigger`'s
  `forced-together` repair arm, 3 times.

In constraint 16's style: before the fix, the file was **measured failing at 8
or more concurrent copies and passing alone**, with a failure rate that rises
with load. After the fix it was **measured passing at every load up to 32
copies**.

## 3. Root cause

The file's own notes say correctly that TSan detects a missing happens-before
edge, not an observed interleaving. The instrument's verdict does not depend on
timing. What did depend on timing was the harness around it, in two ways.

### 3.1 The exit barrier wrote TSan trace on every poll

Under `forced-together`, a worker that has finished its body waits in
`qcore_forced_exit_barrier` until its partner's body has also finished. It
polled by taking and releasing the header's spinlock. The header is
`no_sanitize("thread")`, but clang still instruments **atomic operations and
the function entry/exit around them** in such a function. `objdump` of a row
program shows `__tsan_func_entry`, `__tsan_atomic8_exchange` and
`__tsan_atomic8_store` inside `qcore_forced_lock` and `qcore_forced_unlock`.

Each poll was therefore a release (a new TSan epoch) plus several trace events,
none of which TSan can deduplicate. TSan keeps a bounded per-thread trace
history. When a partner is descheduled, the waiting worker's polls recycle the
trace that holds its **own body's write**. The partner's write then races with
a shadow entry whose stack cannot be restored, and TSan v3 **drops such a report
without output**. That drop is inferred from the behaviour below, because the
runtime source is not on this host.

A mechanism experiment ([`scripts/mechanism.py`](scripts/mechanism.py) and
[`scripts/mechanism_sweep.py`](scripts/mechanism_sweep.py), table in
[`artifacts/mechanism-sweep.jsonl`](artifacts/mechanism-sweep.jsonl)) used the
pre-card programs captured with [`scripts/capture.patch`](scripts/capture.patch).
It delays worker 2's body by a spin that TSan cannot see (TSC reads in
uninstrumented code) and counts the reported races:

| overlap mutation, `ct07-case`, `forced-together` | 0 ms | 1 ms | 2 ms | 3 ms | 5 ms | 20 ms | 100 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| pre-card barrier, default `history_size` | 10/10 | 10/10 | 6/10 | 6/10 | **0/10** | 0/10 | 0/10 |
| pre-card barrier, `history_size=7` | | | | | 10/10 | 3/10 at 10 ms | 0/10 |
| barrier polling with a 1 ms `nanosleep` | | | | | 10/10 | 10/10 | 10/10 |
| barrier polling relaxed atomics, no lock | | | | | 10/10 | 10/10 | 10/10 |

With the pre-card barrier, a run missed the race once the barrier had polled
about **23,000–42,000** times. With relaxed polling, runs that polled
**170,000–253,000** times were all caught. A larger history only moves the
cliff (5 ms → 10–20 ms), which rules out any fix by an option value. Removing
the `start` print or wrapping the barrier in TSan's ignore annotations
(`__tsan_ignore_thread_*`, `AnnotateIgnoreSync*`) did not help: each caught 0 of
10. Under `forced-reverse` there is no exit barrier, and the same 20 ms delay on
either worker was caught 10 of 10. The join-edge-removal mutation under
`forced-together` follows the same pattern: 5/5 at 0 ms, 0/5 at 20 ms, and 5/5
at 20 ms with relaxed polling. This covers both subjects, and it is exactly the
failure seen at the W45-PARENTHESISED-CALLEE gate.

### 3.2 The join stand-in bet on 50 ms

`QCORE_TSAN_DROP_JOIN`, the stand-in the join-edge-removal mutation calls
instead of the join, slept 50 ms and assumed the worker had finished by then.
Under load that assumption fails. Here is what happens when a worker is still
parked or runnable after 50 ms:

* The parent frees the start record in `qcore_task_collect`.
* The worker then reads the freed record, and TSan reports a
  **`heap-use-after-free`**, not the data race the row counts.
* If the process ends first, TSan reports nothing.

On `shared_readers @ forced-reverse`, delaying either worker by 20 ms was
caught 4/4. Delaying it by 60 or 120 ms was caught 0/4.

### 3.3 Candidates ruled out

* **The gate did not time out.** It has no timeout, and every failing trace
  shows both workers released.
* **`halt_on_error` did not swallow a report.** No report was printed at all.
* **The thread-leak warning did not come first.** It is printed at exit only
  because no race had halted the run.
* **The harness process timeout did not fire.** `Command::output` has none.

## 4. The fix, deterministic and with no assertion weakened

* **Quiet exit barrier.** A relaxed counter `qcore_forced_changes` is bumped
  under the lock on every change to the cohort tables: a gate opening, a finish,
  or a seat given back. The barrier polls that counter with a relaxed load
  inline, which TSan deduplicates, and takes the lock only when the counter has
  moved. Lock round trips are now bounded by the cohort's own changes, not by
  how long a partner is descheduled. The final lock acquire is kept, so the
  barrier still ends with the same edge as before, and it adds none before any
  body.
* **Completion-driven stand-in.** The binding gives every created thread a
  completion flag, and `qcore_tsan_trampoline` sets it with a relaxed store when
  the start function returns. `qcore_tsan_unordered_join` spins on it with
  relaxed loads, inline. Relaxed atomics carry no happens-before edge in C11 or
  in TSan, so the removed edge stays removed. The worker's accesses now come
  before the parent's by construction rather than by a 50 ms bet. The forcing
  header's gate-opening variant opens the gate first and then waits the same
  way.
* **The gate-closed control does not hang.** A worker parked behind a gate that
  only its waiting creator could open never finishes. The stand-in asks
  `QCORE_TSAN_NEVER_RUNS(thread)` once before waiting: `0` in the binding, and
  `qcore_forced_never_runs` under forcing. That function reads without the
  lock, so it acquires nothing. The lower-case `qcore_tsan_drop_join` is now a
  macro, so that this hook is expanded at the call site. The join-deleted
  mutation still bypasses the gate-opening `QCORE_TSAN_DROP_JOIN`, and its
  `cohort: 0` control still releases no worker and reports nothing.
* **Load on demand.** `Row::hold` (`QCORE_TSAN_HOLD_ORDINAL` /
  `QCORE_TSAN_HOLD_MS`) holds the `N`-th created thread before its start
  function, using `thrd_sleep`, which TSan does not intercept. Two new rows
  hold one worker for 100 ms:
  * `a_held_partner_still_has_its_overlap_reported_under_forced_together`;
  * `a_held_worker_still_has_the_removed_join_edge_reported_under_every_schedule`.

  They make every assertion the ordinary rows make.
* **Header comment corrected.** The forcing header's comment claimed that every
  function was "compiled without TSan instrumentation" and that TSan observes
  only the program's own edges. It now states what `no_sanitize` leaves
  instrumented, and the rule that every wait must be quiet.

Every existing row keeps its assertion, including both mutation controls.
`a_join_deleted_is_caught_by_the_cohort_trigger` still catches its mutation
under both forced schedules and unforced, and its `cohort: 0` control still
reports 0 races with an empty schedule.

**Non-vacuity of the held rows.** [`scripts/mutate.py`](scripts/mutate.py) puts
back each pre-card wait, runs the held rows three times, restores the harness
by rewriting its bytes (round trip asserted), and runs a green baseline. Logs
and results are in [`artifacts/mutations/`](artifacts/mutations/).

| mutation | held rows red |
|---|---|
| pre-card exit barrier (lock and unlock on every poll) | **3/3**, both rows |
| pre-card 50 ms stand-in | **3/3**, the join row, `shared_readers @ unforced`, reported as `heap-use-after-free` |
| restored baseline | 0/1 |

**Margin.** The fixed binary with every row at `history_size=0`, the smallest
history, is green 5/5 alone and 64/64 at 16 copies
(`artifacts/load/after-history0-*.json`). That was a temporary build and was
not committed.

**Why not `EXCLUSIVE`.** The sensitivity was in the harness and is gone at
every measured load, including with the smallest history. Running the binary
alone would add wall time without protecting anything more. The held rows
reproduce the load on every run, so if the harness regresses it shows up
deterministically rather than as an occasional load failure. Constraint 16
records the measurement and this decision.

## 5. Validation

The standing gate was run by [`scripts/gate.sh`](scripts/gate.sh) on the
card's tree, with `CARGO_INCREMENTAL=0`, followed by `cargo clean --profile
dev`. Each step's exit was captured to a file, and no step was piped. The
results are recorded in [`gate.json`](gate.json).

| step | exit | result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | |
| `scripts/run_test_binaries.py`, run 1 | 0 | `test binaries: 297 units, 307 summaries; 5788 passed; 0 failed; 11 ignored; 0 measured; 0 filtered out` |
| `scripts/run_test_binaries.py`, run 2 | 0 | the same totals line |
| `scripts/run_test_binaries.py`, run 3 | 0 | the same totals line |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `scripts/check_lean_audits.py` | 0 | 195 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 358 tests OK |
| `scripts/build_docs_index.py --check` | 0 | |

In all three runs, `qcore-compiler::test::task_route_tsan` ran in the **parallel
phase** (`"exclusive": false` in each `summary.json`) and returned `20 passed; 0
failed; 1 ignored`. The two added rows raise the Rust total by 2. Clang is
`Ubuntu clang version 21.1.8`, recorded in this worktree
([`clang-version.txt`](clang-version.txt)). The provider workspace was not run,
because nothing under `providers/` changed.

## 6. Records and follow-ons

* **Constraint 16** (`docs/agents/providers.md`): one paragraph inside the
  preserved block, with the before and after loads. The rule's six enumerated
  rows are unchanged.
* **`docs/agents/validation.md`** is unchanged, because the runner's contract
  did not change.
* **Retired `TSAN-JOIN-EDGE-ROW-LOAD-SENSITIVE`.** Wave 41 registered it, with
  two proposed discharges: add the row to rule 16, or assert on the report set.
  This card took a third, the one the entry's own mechanism sentence pointed
  at. The cause was found in the harness and removed, and the load is now
  measured at 0 of 128 at 32 copies. The assertion still requires a data race.
* **Registered `W45-TSAN-QUIET-WAIT-ENFORCED-BEHAVIOURALLY`.** The quiet-wait
  rule is enforced by the two held rows at today's wait sites, not by a shape
  check. A new wait loop that releases on every poll would pass the header's
  shape test. The same drop mechanism also bounds what a clean row's zero
  means: it means "no race reported". The failing gates' clean rows were never
  red, and those subjects are race-free by the row design's other evidence, so
  no verdict moves.
