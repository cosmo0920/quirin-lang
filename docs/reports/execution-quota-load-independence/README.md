# EXECUTION-QUOTA-LOAD-INDEPENDENCE: why `execution_quota` failed under load, and the fixes

Card of wave 48, base `440c41b8b`, prediction frozen at `a40b65826`
([PREDICTION.md](PREDICTION.md)). It changes the verifier host's
per-execution cgroup name (`crates/qcore-verifier-host/src/process_quota.rs`),
four verifier-host test files and a shared fixture helper, the runner's
`EXCLUSIVE` reasons, and `QCore.ProcessQuotaPlan` (three theorems). No
analyser, checker, contract or verdict code changes.

## 1. What failed

`LOAD-SENSITIVE-ROW-UNNAMED` recorded five gate failures of
`qcore-verifier-host::test::execution_quota`, each green 13/13 when the binary
was rerun alone:

| Row | Signature |
|---|---|
| `a_forced_override_never_weakens_an_execution` (:777) | `Io("Broken pipe")`, later `Spawn("Text file busy (os error 26)")` |
| `the_same_fixture_without_a_declared_quota_installs_none` | `Io("Broken pipe")` |
| `a_declared_quota_bounds_a_fork_storm_in_this_execution_only` (:458) | `quota_observed` None |
| `the_quota_binds_without_depending_on_the_accounts_process_count` (:584) | `quota_observed` None |

The binary was already in the runner's `EXCLUSIVE` table, with the reason
"RLIMIT_NPROC rows count every process of the real UID". That isolates it only
from the runner's own other binaries, never from other cards on the host.

## 2. Measurement

[`scripts/loadbisect.py`](scripts/loadbisect.py) starts a bounded background
load under the invoking UID, runs one test binary N times from the crate root
with its default test threads, and records each run's exit, the 1-minute load
average, the UID's task count and every failure's panic. Each load process is
self-bounded by a deadline and is killed by its captured PID when the level
ends. No load process was left alive after any run. The load dimensions are:

* idle same-UID `sleep` processes, which raise the count that `RLIMIT_NPROC`
  sees;
* CPU spinners;
* fork+exec loops;
* concurrent copies of the binary;
* `unshare -rn` loops;
* **host stall**: the measured test process alone is `SIGSTOP`ped for a random
  0..N ms, then runs for 0..N ms. Its children keep running;
* **forked-child stall**: each child of the test process that has forked but
  not yet `exec`ed is stopped for N ms.

[`scripts/amplify.c`](scripts/amplify.c) is an `LD_PRELOAD` shim, compiled
into `private/` and loaded into the measured binary only. It widens one window
per mechanism. `CLOSE_US` delays the close of a write descriptor on
`provider.sh`. `REALTIME_NS` rounds `CLOCK_REALTIME`, which models a coarse
wall clock.

`bin-diag` is the base test file with three panics extended to print the
execution record, so a `quota_observed` None names its cause. It has no other
change. Every set, with per-test signatures, is in
[artifacts/load/SUMMARY.md](artifacts/load/SUMMARY.md); the per-run JSON is
beside it. The campaign scripts are `scripts/campaign{1,2,3}.sh` and
`scripts/attribution-probe.sh`. The host has 16 CPUs and about 2,200 tasks
under the UID at rest.

### 2.1 Load does not flip these rows

On the base binary, CPU load did not move any row:

* up to 256 spinners plus 16 fork loops (load1 up to 212): 33 runs, 0 failures;
* 4 or 12 concurrent copies, 8 `unshare -rn` loops, or all of them mixed with
  32 spinners: 24 runs, 0 failures;
* idle: 60 runs, 0 failures;
* 1,000, 3,000 or 5,000 extra same-UID processes (UID tasks 3,209 to 7,228):
  100 runs, 0 failures.

The natural reproductions were:

* `Spawn("Text file busy")` at :777, once in 30 runs, with a release build
  running beside the binary;
* one `quota_observed` None in 10 runs at 3,000 sleepers. The diagnostic named
  the cause: `could not create …/app.slice/qcore-exec-2911641-1791799458013850267:
  File exists (os error 17)`. The next 25 runs at 3,000 sleepers were clean,
  so the account count was not the trigger.

**There is no load threshold to bisect.** These rows are races. Load changes
their probability through descheduling and through slow fork-to-`exec`
windows. The run-queue length does not decide them. Each race has its own
dimension below, and the before/after comparison is made on that dimension.

### 2.2 Per mechanism, base against fixed

| Mechanism | Load (dimension) | Base (diagnostic) binary | Fixed binary |
|---|---|---|---|
| (c) EPIPE | host stall 2 / 5 / 10 / 20 / 50 ms | 0/10, 0/10, **2/20**, **3/10**, **2/10** | 20 ms **0/50**, 50 ms **0/50** |
| (b) ETXTBSY | close delay 20 ms; plus forked-child stall 5 / 20 / 50 ms | **6/20**; **10/20**, **3/20**, **9/20** | close delay + child stall 50 ms **0/50** |
| fork window alone | forked-child stall 20 ms, no shim | 0/30 | **0/50** |
| (d) EEXIST | `CLOCK_REALTIME` rounded to 1 µs / 1 ms | **2/20**, **18/20** | 1 ms **0/50** |

In constraint 16's style, the base binary was measured failing under:

* host stalls of 10 ms and above (EPIPE);
* the fork-window amplifier, at every child-stall setting (ETXTBSY);
* a 1 µs or coarser realtime clock (EEXIST).

It passed with no amplifier, at every CPU, process-count and copy load
measured. The fixed binary was measured passing under all of these:
**250 runs, 0 failures**.

## 3. Mechanisms and fixes

All four signatures come from one test process that forks concurrently from
its test threads, each fork with a slow `pre_exec`. The slow steps are the
cgroup join, Landlock, `unshare(CLONE_NEWUSER|CLONE_NEWNET)`, and the host's
namespace probe, which forks and `_exit`s without `exec`. A forked child holds
a duplicate of every descriptor that was open at the moment of the fork, until
its own `exec`.

* **(a) RLIMIT_NPROC: not a mechanism of this binary.** No `execution_quota`
  row declares `max_processes`. The quota is a cgroup v2 `pids` bound on the
  execution's own tree. Its count did not move at 7,228 UID tasks (§2.1).
  The host's quota mechanism does not depend on the account's count, so there
  is no product defect of that kind. The runner's `EXCLUSIVE` reason was
  wrong for this binary and is corrected.
* **(b) ETXTBSY.** `Fixture::script` and `install` wrote the executable with
  `fs::write`. If another test thread forks while that descriptor is open, its
  child keeps a writable duplicate until that child `exec`s, and an `execve` of
  the script in that window fails with `ETXTBSY`. `O_CLOEXEC` (which Rust
  already uses) only closes the duplicate at `exec`. Writing to a temporary file
  and renaming would not help either, because the inode is the same.
  - **Fix:** [`tests/support/process_fixture.rs`](../../../crates/qcore-verifier-host/tests/support/process_fixture.rs)
    `write_executable` writes through `/bin/sh -c 'cat > "$1"; chmod 700 "$1"'`.
    The only write descriptor then belongs to a single-threaded `cat` that
    forks nothing and has exited before the call returns. An existing file
    keeps its inode, which `package_identity`'s in-place rewrite asserts.
  - **A fifth disguise.** Under the amplifier, `the_quota_binds_…` failed at
    `peak >= 3` with `peak=Some(1)`. The base test discarded the run's result
    (`let _ = provider.verify(…)`). A child that joins the cgroup in `pre_exec`
    and then fails `execve` gives exactly that peak of 1.
* **(c) EPIPE.** The fixtures never read their request. The host refuses a
  successful exit after a broken request pipe. That is deliberate since
  `VERIFIER-HOST-CLOSED-INPUT-DEADLINE`: a successful exit cannot admit a
  partial request. So whether a row failed depended only on whether the
  fixture finished before the host's first write. There are two ways to make
  the host late:
  - It is descheduled (host stall).
  - Its `spawn()` returns late. std's `spawn` reads its exec-status pipe to
    EOF, and that pipe's write end is duplicated into every child forked
    concurrently by another thread, so `spawn` returns only after every such
    sibling has `exec`ed. The attribution probe shows this. The forked-child
    stall alone, which never stops the measured test's own child, failed the
    base confined binary **10 of 15** runs with EPIPE only. The close delay
    alone failed it 7 of 15 with ETXTBSY only.
  - **Fix:** every fixture's first command is `READ_REQUEST`, a builtin
    `while read -r line; do :; done` that consumes the request to EOF. It
    starts no process, so no quota or `RLIMIT_NPROC` count a row asserts
    moves. The host closes the pipe only after writing the whole frame, so a
    fixture can no longer exit before that write.
  - **EPIPE is no longer accepted blanket.** The storm row's accepted outcomes
    drop `ProcessOutcome::Io`. The per-tree row now asserts `Completed`, with
    the host's detail, instead of discarding the result.
* **(d) `quota_observed` None is EEXIST, a product defect.** Every record with
  a declared quota and no observation is a pre-spawn refusal, because
  `finish_quota` observes every installed session. The refusal was
  `QuotaSession::create`'s `mkdir` of `qcore-exec-{pid}-{nanos}`.
  - **Cause:** two executions of one host process can read the same
    nanosecond. Concurrent sessions are released together from the same
    cached kernel probes (`OnceLock`) and run identical instructions up to
    `SystemTime::now()`, so the second `mkdir` met `EEXIST` and that execution
    was refused with Unknown.
  - **Impact:** it is fail-closed, never a wrong Safe. But a host verifying
    concurrently (the arbiter's `--jobs`) could refuse an execution because of
    scheduling.
  - **Fix:** `process_quota::linux::session_name(pid, sequence, nanos)` adds a
    per-process `AtomicU64` sequence number. A unit test
    (`per_execution_cgroup_names_never_collide_within_one_process`) pins that
    64 sequence numbers with one pid and one nanosecond give 64 names, and that
    a name decodes back to its triple.
  - **Diagnostics:** the test's `observed_quota` panics with the record's
    outcome and detail instead of a bare `expect`.

## 4. Sibling rows

Constraint 16's other verifier-host rows share mechanisms (b) and (c). They
use the same helper, and `package_identity` uses `write_executable` only,
because its fixtures already read their request.

| Binary | Base: host stall 20 / 50 ms | Base: close delay + child stall 50 ms | Fixed: stall 20 / 50 ms; amplifier |
|---|---|---|---|
| `confined_process_execution` | **2/20** EPIPE, 0/20 | **15/20** EPIPE | **0/50, 0/50; 0/50** |
| `enforced_process_limits` | **1/20** EPIPE, 0/20 | **15/20** ETXTBSY (all seven spawning rows, including `memory_growth_…` and the RLIMIT_NPROC row) | **0/50, 0/50; 0/50** |
| `package_identity` | 0/20, 0/20 | 0/20 | 0/50, 0/50; 0/50 |

`package_identity` had one historical observation:
`a_removed_executable_is_refused_at_the_next_spawn` failed with
`Spawn("Text file busy")` in a wave-47 gate
(`implementation-wave-47-681db5/private/test-binaries/20260929T121351Z-1547079`).
The amplifier did not reach its short-lived forks here. Its fix rests on the
mechanism, not on a measured flip.

The RLIMIT_NPROC row (`a_declared_process_limit_stops_the_fixture_from_forking`)
gave the following results on the base binary:

* 3,000 extra same-UID processes (5,217 to 5,230 UID tasks): 0/15;
* 64 spinners plus 8 fork loops: 0/15 (the one failure at that level was an
  EPIPE in `a_well_behaved_provider_…`);
* its only failures were ETXTBSY under the amplifier.

Its historical `Io(Broken pipe)` (`P02-FLAKE`) is mechanism (c). The fixed
binary passed 0/15 at both levels.

## 5. Runner

`execution_quota` **stays** in `EXCLUSIVE`, with a corrected reason. It still
launches process providers under a declared 30 s timeout, and its storm
control compares task peaks of 3 s sleepers, which is a wall-clock quantity.
Both fall under the table's membership rule. The two siblings stay for the
same reason, and their reasons now say the `Io(Broken pipe)` and ETXTBSY
shapes were repaired. `EXCLUSIVE` was never what protected these rows from
other cards, so the fixes do not depend on it.

## 6. Proof under concurrency

The gate's test step was run concurrently with the AFTER corpus arbiter. This
is the condition the integrator kept seeing: other work on the host during
the suite.

* [`scripts/gate.sh`](scripts/gate.sh) built the suite, then launched the
  arbiter (`--jobs 4`, under the heavy lock).
* Once the arbiter was analysing, it ran `scripts/run_test_binaries.py`,
  deliberately outside the lock.
* The epoch stamps are in `private/eqli/gate-g1/overlap.txt`:
  - arbiter running at +1.0 s;
  - suite from +1.0 s to +332.4 s;
  - arbiter end at +618.5 s.
* So the whole suite, its `EXCLUSIVE` phase included, ran inside the
  arbiter's run. The arbiter took 618 s against 452 s alone.

The result: **487 units, 500 summaries, 7,939 passed, 0 failed, 14 ignored,
exit 0, with no rerun of anything**. Every verifier-host unit was green:
`execution_quota` 13, `confined_process_execution` 12,
`enforced_process_limits` 12, `package_identity` 10, the lib 31.

## 7. Arbiter identity

`process_quota.rs` is production code, so all three routes were re-run.
BEFORE was the base release binary (`72f150ee…`, built at `440c41b8b`). AFTER
was this tip's binary (`4c1ba16a…`). Each run's `run.json` `binary_sha256`
matches its binary.

| Route | BEFORE | AFTER |
|---|---|---|
| corpus | 935 / 9 / 3,890, `bdf5f1c8…` (the brief's) | **byte-identical** (run under the concurrent suite) |
| system | 93 / 4 / 351, `be478b33…` | **byte-identical** |
| slice | 52 / 1 / 347, 4 Pending, `9bc3b74e…` | **byte-identical** |

Every report and sidecar is byte-identical. The corpus `audit.log` and
`run.json`, and the invocation records, are identical once the run directory
name, the binary digest and the times are removed. The typed-path parity
streams hold the same rows: 689,660 / 142,456 / 51,389, compared as a multiset
with Clang node addresses normalised, since their order follows the
`--jobs` workers. See [artifacts/identity.txt](artifacts/identity.txt) and
[`scripts/identity.py`](scripts/identity.py). The system and slice digests
differ from the brief's (`f596d64b…`, `ee98ad3d…`) with equal totals,
because those reports embed the measuring worktree's paths, as
GAPO-RETURN-CORRECTNESS recorded. No verdict moved, so no correction is named.

## 8. Lean

`QCore.ProcessQuotaPlan` gains `SessionKey` and three theorems, audited in
`AuditProcessQuotaPlan.lean`. The clock is an arbitrary function: nothing
assumes it is monotone or that two readings differ.

* `session_keys_are_injective_in_the_sequence`: one process's session keys are
  injective in the sequence number.
* `the_sessions_of_one_process_have_pairwise_distinct_keys`: the first `count`
  draws have pairwise distinct keys.
* `a_repeated_clock_reading_collides_only_the_legacy_key`: two draws with equal
  clock readings had equal legacy `(pid, clock)` keys and have distinct keys
  now.

The decimal spelling is pinned on the Rust side.

## 9. Gate

Every code step ran at `81f5925fe` with [`scripts/gate.sh`](scripts/gate.sh)
`g1`. Exits are in [gate/](gate/), logs in `private/eqli/gate-g1/`, and the
summary in [gate.json](gate.json).

| Step | Exit | Count |
|---|---:|---|
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` (concurrent with the corpus arbiter, §6) | 0 | 487 units, 500 summaries; **7,939 passed, 0 failed**, 14 ignored; `private/eqli/tests-g1/` |
| corpus arbiter beside it | 0 | byte-identical (§7) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **316** audits |
| `python3 -m unittest discover -s scripts/tests` | PYEXIT | PYCOUNT |
| `python3 scripts/build_docs_index.py --check` | INDEXEXIT | |

The test step passed on its first run. No row was rerun in isolation.
The suite count is main's 7,938 plus the cgroup-name unit test.

There is no provider gate, because `providers/` is unchanged. Clang:
`clang --version` was recorded in this worktree (`gate/clang-version.txt`).

## 10. Prediction against outcome

| Predicted | Outcome |
|---|---|
| (a) no RLIMIT_NPROC row; 3,000 sleepers move nothing | held (0/25 at 3,000 and at 5,000) |
| (b) fork-inherited write descriptor; fixed by an out-of-process writer | held |
| (c) fixtures not reading their request; fixed by `READ_REQUEST` | held; the host's lateness has a second cause the prediction did not name (`spawn` waits for concurrently forked siblings' `exec`) |
| (d) a pre-spawn refusal, cause unknown | **cause found: EEXIST on the cgroup name, a product defect** |
| no production change, arbiters not re-run | **wrong**: `process_quota.rs` changed, so all three routes were re-run (§7) |
| test counts unchanged | `qcore-verifier-host` unit tests +1 (30 → 31, the name pin); integration counts unchanged |
| `EXCLUSIVE` keeps `execution_quota` with a corrected reason | held |

## 11. Follow-ups

* Retired:
  - `LOAD-SENSITIVE-ROW-UNNAMED`. Every observed signature is reproduced,
    attributed to a mechanism and repaired, with measured before/after figures
    (§2.2) and the constraint-16 entry amended.
* Amended:
  - `P02-FLAKE`, with the repaired shapes.
* Registered:
  - `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`. `quota_descriptor` maps a failed
    descriptor `dup` to "no join", so the child runs outside its cgroup while
    the record names an installed quota. This is fail-open evidence, found by
    reading, and it is out of this card's scope.
  - `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`. A provider that never reads
    its request is refused or accepted depending on write order. The card
    repaired the fixtures, not the host's rule.
