# EXECUTION-QUOTA-LOAD-INDEPENDENCE: prediction (frozen before implementation)

Base: local `main` at `440c41b8b`. Frozen after the reproduction runs below and
before any test or source edit was written. Measurement binaries are private
copies under `private/eqli/` of this worktree; `bin-diag` is the base test file
with three panics extended to print the execution record (no other change), so
a `quota_observed` None names its cause.

## What the reproduction already shows

| Signature | Reproduced? | Load that reproduced it |
|---|---|---|
| `Spawn("Text file busy (os error 26)")` at `a_forced_override_never_weakens_an_execution` (:777) | yes, 1 of 30 runs | a release build running beside the binary (load1 6 to 17); 0 of 60 idle |
| `Io("Broken pipe (os error 32)")` | yes: storm control (:499) and `the_same_fixture_without_a_declared_quota_installs_none` (:531) | host stalls: the test process (only) stopped for 0–20 ms at a time, 3 of 10 runs |
| `quota_observed` None (:458, :584) | not yet | — |
| any row | no | pure CPU load: up to 256 spinners plus 16 fork loops (load1 ≈ 212), 18 runs, 0 failures; 12 concurrent copies of the binary, 0 failures; 8 `unshare -rn` loops, 0 failures |

So the rows are **not** CPU-load thresholds. They are races between threads of
one test process and between the host and its child, whose windows widen when
the test process is descheduled (page faults, I/O, memory pressure from builds),
not when the run queue is long.

## Mechanisms predicted

* **(a) RLIMIT_NPROC.** No `execution_quota` row declares `max_processes`; the
  per-execution quota is a cgroup v2 `pids` bound on the execution's tree.
  Predicted: 3,000 idle same-UID processes move no row. The runner's
  `EXCLUSIVE` reason for this binary ("RLIMIT_NPROC rows count every process of
  the real UID") is therefore wrong for this binary and will be corrected. No
  product defect is predicted in the quota mechanism.
* **(b) ETXTBSY.** `Fixture::script` / `install` write the executable with
  `fs::write` from a multi-threaded test process. `O_CLOEXEC` does not help:
  another test thread's spawn forks while the write descriptor is open, and the
  forked child holds the duplicate until its `exec` — after the confining
  `pre_exec` (cgroup join, Landlock, `unshare(CLONE_NEWUSER|CLONE_NEWNET)`), or
  for the host's own namespace probe, until `_exit`. Write-then-rename does not
  help either (same inode). Fix: the fixture writes its executable from a
  separate single-purpose process (`/bin/sh -c 'cat > "$1"; chmod 700 "$1"'`),
  so no write descriptor on an executable ever exists in the test process.
* **(c) EPIPE.** The fixtures never read their request. When the host is
  descheduled after `spawn` for longer than the fixture runs, its first write
  meets a closed pipe; the host then refuses a successful exit with
  `Io(Broken pipe)` (deliberate since `VERIFIER-HOST-CLOSED-INPUT-DEADLINE`:
  a successful exit cannot admit a partial request). Fix: every fixture reads
  its whole request to EOF first, with a shell builtin (no extra process, so no
  quota count moves), as a protocol-conforming provider does. The storm row's
  blanket acceptance of `ProcessOutcome::Io` is removed: with the request
  consumed first, `Io` is no longer an expected outcome there.
* **(d) `quota_observed` None.** By reading: every record with
  `quota_observed: None` under a declared quota is a pre-spawn refusal
  (`RefusedByProfile`) from `prepare_confinement` or `QuotaSession::create`,
  because `finish_quota` always observes an installed session. The trigger is
  not yet reproduced. The test will fail with the record's outcome and detail
  instead of a bare `expect`, and the bisection keeps the diagnostic binary
  under every load so a recurrence names its cause.

## Predicted outcome

* After the fix: **0 failures in 50 runs** of `execution_quota` under the host
  stall load that reproduced (c) (0–20 ms) and above it (0–50 ms), and 0 in the
  build-beside load that reproduced (b); the same for `confined_process_execution`
  and `enforced_process_limits`, whose fixtures share (b) and (c).
* The full suite with the corpus arbiter running concurrently: 0 failures.
* Test counts unchanged (no test added or removed): `execution_quota` 13.
* No production code changes are predicted, so the corpus, system and slice
  arbiters do not move and are not re-run for identity; if production
  verifier-host code does change, they are re-run and shown byte-identical.
* `EXCLUSIVE`: predicted to **keep** `execution_quota` (it launches process
  providers under a declared timeout, and the storm control compares task
  peaks of 3 s sleepers, a wall-clock-dependent quantity), with its reason
  corrected; the measurement decides.
