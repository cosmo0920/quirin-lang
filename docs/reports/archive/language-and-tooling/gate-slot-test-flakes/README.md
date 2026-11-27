# GATE-SLOT-TEST-FLAKES

Base `b069dbda8`. The gate-slot test modules failed intermittently at most
landings (a different test each time). Five failure modes were reported from
the integrator logs; each has a root cause below, with its evidence and fix.
One is a defect in `scripts/gate_slot.py` (its `status` reading, three
modes); two are races in the test harness. None is over-admission: no mode
let two jobs hold one slot.

Census of the integrator logs (`private/integrate-r*/**/*.log`, `FAIL:` /
`ERROR:` lines; one log can be a gate run or a rerun):

| Mode | Test | Logs |
|---|---|---|
| (a) | `test_gate_slot_lane.Status.test_status_shows_lane_holders_and_waiters` | 12 (r8, r17, r29, r30 ×2, r31, r42, r45, r51 ×2, r53 ×2) |
| (b) | `test_gate_slot_queue.Status.test_each_waiter_shows_its_queue_position` | 7 (r8, r31, r34, r38, r40, r46, r52) |
| (c) | `test_gate_slot_lane.Rule.test_a_lane_job_waits_until_its_declared_memory_is_available` | 3 (r29, r51, r54) |
| (d) | `test_gate_slot_queue.Stale.test_a_live_owner_without_a_lock_is_not_reclaimed` (ERROR) | 1 (r39) |
| (e) | `test_gate_slot_queue.Status.test_a_backing_off_drain_and_a_job_of_an_older_client_are_told_apart` | 1 (r38) |

## Modes and root causes

### (a), (b), (e): `status` counted a waiter's poll locks as slots (code defect)

`gate_status` read every `/proc/locks` entry on a gate file other than the
legacy lock as a slot the pid holds, and gave any job with a slot the role
`holder`; the short lane's `holders` were the `short`-class jobs with any lock
named `short/...`. But a waiter takes gate locks for a moment on every poll:

- `attempt` (audit) and `attempt_short` (lane) take their class's admission
  lock (`audit/admission`, `short/admission`) **shared** for the whole poll
  (memory reading, lane reserve, queue scan, slot probes), and an older
  client's poll does the same; `_acquire_build` takes `build/admission` shared;
- `_free_slots` (a queued waiter) and `lane_reserved_kib` (every audit and lane
  poll) probe free numbered slots by taking them **exclusive** and letting go.

A snapshot inside that window saw the waiter "holding" `short/admission`:
three holders of a two-slot lane (a), a queued audit waiter or an older
client's waiter reported as `holder` instead of `waiting for a slot` (b) and
`waiting for a slot (not queued ...)` (e). With polls every 0.02 s under CPU
load the window is a large fraction of the waiter's time. A shared
`build/admission` hold was also misread as a `rust-suite` closing the builds
(`builds.closed_by`, role `draining builds`); no test happened to snapshot it.

Not over-admission: the numbered slots are exclusive `flock`s, which the
kernel grants to one open file description at a time, and admission never
reads `status`; the extra pid in (a) is the waiter's shared admission lock.
The new tests below hold exactly those locks in a stand-in process, so the
snapshot always falls in the window; at the base they fail with the
integrator's message, `Lists differ: [p1, p2, p3] != [p1, p2]`, `'holder' !=
'waiting for a slot (not queued ...)'` and `builds.closed_by [pid] != []`
([log](new-status-tests-at-base.log)).

Fix (`gate_status`, new `slot_hold_is_poll`): a shared admission lock is a
poll (a `rust-suite` job closes admission exclusive); a numbered slot is held
when its holder note names the pid (every job notes each slot it takes, at
once), or when the pid has no ticket and the slot is of its own class (an
older client, or a job between its lock and its note); otherwise it is a
probe. Poll locks are listed under the job's new `polling` field, never in
`slots`, and the role, the lane's holders, `holders_of` and `builds.closed_by`
read `slots` only. `read_note` parses the note's first line: a note is one JSON
line rewritten by `pwrite` then cut to length, so a reader between the two saw
the new line plus the old tail and, under load, could exhaust its three
retries and read no note (which the new rule would have read as a probe).
Rule text: the Status bullets of [validation](../../../../agents/validation.md#gate-slots)
and the module docstring.

### (c): the fake meminfo was rewritten in place (harness race)

`set_memory` wrote the scratch meminfo with `Path.write_text` (truncate, then
write). The `two-gib` lane job polls it every 0.02 s; a read between the
truncate and the write found no `MemAvailable`, and `read_meminfo` refuses
that (fail-closed, correct), so the job ended with that refusal. Fix: both
`meminfo` helpers (`test_gate_slot_starvation.py`, `test_gate_slot.py`) write
`meminfo.new` and `os.replace` it, so a reader sees the old or the new file
whole. `/proc/meminfo` itself is generated per read and has no such window.

### (d): tearDown removed the scratch gate under a thread still polling (harness race)

`Queue.tearDown` set each job's `finish` event and joined the thread for at
most 20 s; `finish` only ends a job that was admitted. A job still waiting
(its acquire timeout is 120 s) kept polling, and recreating gate files
(`try_lock` makes the directory and opens with `O_CREAT`), while
`scratch.cleanup()` removed the tree: `OSError: [Errno 39] Directory not empty:
'.../gate/audit'`. Every test that ended with a waiter not admitted also
spent the full 20 s in tearDown (`Stale` took 20.4 s for that one test).
Fix: `Gate.cancel()` makes a wait in another thread give up at its next poll
(`WaitTimeout`, everything released and the ticket dropped; a drain's backoff
also ends); `Queue.tearDown` cancels every job, joins it, and fails, leaving
the scratch directory in place, if a thread is still alive, rather than
removing the tree under it. New test
`test_a_cancelled_waiter_ends_at_its_next_poll_and_leaves_nothing`. The
command line never cancels.

## New tests

`test_gate_slot_lane.Status`: `test_a_lane_waiter_inside_its_poll_is_a_waiter_not_a_third_holder`,
`test_a_probe_of_a_free_slot_is_not_a_hold`,
`test_an_older_clients_audit_waiter_inside_its_poll_is_not_queued_not_a_holder`,
`test_a_build_inside_its_admission_poll_does_not_close_the_builds`,
`test_which_locks_are_a_poll`; `test_gate_slot_queue.Order`:
`test_a_cancelled_waiter_ends_at_its_next_poll_and_leaves_nothing`. No
assertion was weakened, no retry or skip added.

## Stress demonstration

Harness: [stress.py](stress.py). It runs each module `--iterations` times
(the modules of one iteration concurrently, `unittest -v`) beside `--load`
CPU spinners it starts and stops by their captured pids, on a snapshot of
`scripts/` (`git archive`) so the worktree could change during a run: the base
`b069dbda8` for *before*, the fix commit `56a5e577d` for *after*. Host: 32
CPUs, shared with other sessions. Per-run logs and `*.out`:
[runs/stress-logs.tar.xz](runs/stress-logs.tar.xz); summaries `runs/*.summary.json`.

| Run | Modules | Iterations | Spinners | Wall |
|---|---|---|---|---|
| before | all five | 30 | 32 | 1,599 s |
| after | all five | 30 | 32 | 841 s |
| before-heavy | lane, queue | 30 | 48 | 1,602 s |
| after-heavy | lane, queue | 30 | 0 own, run beside before-heavy (its 48 spinners and tests) for all of its 508 s | 508 s |

Failures per test (count of runs failing it):

| Test | before | after | before-heavy | after-heavy |
|---|---|---|---|---|
| (a) `lane.Status.test_status_shows_lane_holders_and_waiters` | 1 | 0 | 3 | 0 |
| (b) `queue.Status.test_each_waiter_shows_its_queue_position` | 1 | 0 | 1 | 0 |
| (e) `queue.Status.test_a_backing_off_drain_and_a_job_of_an_older_client_are_told_apart` | 1 | 0 | 0 | 0 |
| (c) `lane.Rule.test_a_lane_job_waits_until_its_declared_memory_is_available` | 0 | 0 | 0 | 0 |
| (d) `queue.Stale.test_a_live_owner_without_a_lock_is_not_reclaimed` | 0 | 0 | 0 | 0 |
| every other gate-slot test | 0 | 0 | 0 | 0 |

The before run's `test_gate_slot.Arithmetic.test_parallel_rs_still_has_that_formula`
errored in all 30 runs and is excluded: it reads
`crates/qcore-audit/src/parallel.rs`, which the base snapshot (`scripts/` only)
lacked; the after snapshot includes that file and the test passed 30 of 30.
The after runs are also faster (841 s against 1,599 s) because a waiter no
longer holds its tearDown for 20 s.

Modes (c) and (d) did not reproduce in the 120 stress runs of their modules.
Both races are evident in the code and are shown in isolation instead:

- (c) [meminfo_race.py](meminfo_race.py) polls `read_meminfo` while a writer
  keeps changing the reading ([log](meminfo-race.log), 5 s each, under the
  stress load): the base helper had 330,736 of 506,634 reads refused with
  `has no MemAvailable field`, the fixed helper 0 of 440,951.
- (d) [teardown_race.py](teardown_race.py) runs the (d) test and records, when
  tearDown removes the scratch gate, which job threads still run
  ([log](teardown-race.log)): at the base `['waiter']` after 20.0 s of tearDown
  (the rmtree then races its polls; it lost once, at r39); fixed, none, after
  0.01 s.

For (a), (b), (e) the new status tests are the deterministic drive: they fail
at the base ([log](new-status-tests-at-base.log): 4 failures and 1 error, the
error being the new function's absence) and pass on the fix.

## Gate

This card changes Python gate tooling, its tests and the gate-slot rule text
only: no checker semantics, verdicts, providers or corpus figures. So no Lean
module, provider gate or corpus prediction applies.

Scoped card gate: `sh private/toolchain/env.sh python3 scripts/gate_scope.py
--base b069dbda8`, run on the report commit (code as at `56a5e577d`), exit 0.
Scope: `quirin-audit-plugin-measure`, because it names `scripts/gate_slot.py`
in a string. Steps: submodules 0; fmt 0; Clippy 0; Rust suite 0, with 57
passed and 0 failed over 8 units; Python 0, with 807 tests and 0 skipped. A
first attempt failed only at its submodules step, because `third_party/libevent`
was not yet initialised; every other step exited 0 (Python 807). That attempt
is retained and not counted. Logs are private to the worktree:
`private/gate-scoped-2.log` and `private/gate-scope/20261122T041334Z-3539088/`;
the first attempt is in `private/gate-scoped.log`.

## Follow-ons

- PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW is amended, not retired: its named
  row (`test_check_lean_audits`, wave 39) and its unnamed wave-34 row are not
  gate-slot tests. This card removes the gate-slot rows, the most frequent
  intermittent Python failures in the integrator logs, by fixing their causes.
- Registered GATE-SLOT-DRAIN-BLOCKERS-POLL-LOCKS: the drain's blocker
  naming (`Gate.audit_holders`) still reads `/proc/locks` without the poll
  rule. While a drain waits for the audits, audit admission is closed but the
  lane is open, so a lane waiter probing a free short slot at the moment of a
  give-up can be named as a short-slot blocker. The report is diagnostic only:
  the drain itself waits on the flocks.
- Probes still take free slots for a moment, so a real taker can lose one
  poll to a probe and retry. That costs liveness only, never exclusion, and
  the lane reserve counts a slot held by a probe at the largest budget (fail
  closed). Not changed.
