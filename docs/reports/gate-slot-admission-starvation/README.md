# GATE-SLOT-ADMISSION-STARVATION

Round 25 (2026-11-06), base `13337d0af`. User decision (verbatim,
2026-11-06), on the integrator's question whether to register the gap:
"Register it (rec.)". Scope: requirement R11 of
[gate-time reduction](../../plans/gate-time-reduction/README.md) ("One audit
cannot starve the host"), tracked by `GATE-SLOT-ADMISSION-STARVATION`, whose
DISCHARGE items 1-4 are the acceptance.

The incident (integrator's record): a `quirin-audit current` experiment spun
10 h 18 min in `audit/3` with three threads at 100 % CPU, zero `rchar`/`wchar`,
zero major faults, RSS 14.4 GiB against a 7168 MiB admission; a card's
`rust-suite` job had closed `audit/admission` and waited 36,326 s for that
slot without bound, so a declared measurement, two checkpoint runs and a census
probe queued 8-14 h. The holders were found only by matching `/tmp/quirin-gate/*`
inodes against `/proc/locks`.

Only `scripts/gate_slot.py` changes (plus its tests and the rule text in
[validation](../../agents/validation.md#gate-slots)). No analyzer, contract,
Lean or measure-plugin change, and nothing an audit reads, so no prediction and
no declared measurement. The measure plugin needed no change (§5).

## 1. Deadlines (DISCHARGE 1)

While the command runs, its own `gate_slot.py` samples the command's process
tree (the child and its descendants by parent pid, never itself) every 10 s.

| Limit | Meaning | Default | Override |
|---|---|---|---|
| `--stall-timeout D` | stop when, for `D`, no tree member changed its `/proc/<pid>/io` counters (`rchar`, `wchar`, `syscr`, `syscw`, `read_bytes`, `write_bytes`; stdout/stderr output is a `write`), no member started or ended, and the `--progress-file` (if any) kept size and mtime | `audit` 2 h; `rust-suite`, `lean` none | flag, else `QUIRIN_GATE_STALL_TIMEOUT`; `0`/`off` disables |
| `--deadline D` | stop `D` after the command started (slot wait excluded) | none | flag, else `QUIRIN_GATE_DEADLINE` |
| `--stop-grace D` | SIGTERM to SIGKILL | 60 s | flag |

Durations need a unit (`90s`, `30m`, `2h`), so `60` is refused rather than
read as seconds or minutes; a malformed value is exit 2 before anything runs.
Each receipt states its `limits` and where each came from (flag, environment,
default).

**Why these defaults.** A wrong deadline kills a real measurement. The pinned
corpus route is the longest route: on WRITTEN-SET-DERIVATION's tip side it ran
1 h 56 min (`invocation.json` start 15:19:44, end 17:15:39 on 2026-11-04), with
clang started per unit, its log and its outputs written throughout. The
incident spun 10 h 18 min with none. Two hours in which no process of the tree
reads, writes, starts or ends is far outside a working run and still cuts that
loss by eight hours. There is no wall-clock default: a declared measurement
legitimately takes hours. Because the drain is bounded (§2), a runaway holds
only its own slot; other audits still run beside it, so the stall default can
be generous.

**Fail-safe readings.** I/O that cannot be read counts as progress (the
receipt counts `samples_unmeasured`, stderr says so once), so a tree that
cannot be measured is never stopped for stalling. A zombie's counters are
unreadable but it does no I/O, so it is not an unmeasured member (tested: an
unreaped zombie does not hide a stall). A sampling error never stops the
command. Without `/proc` (macOS) the stall and resident-set readings are off,
the script says so, and the wall-clock deadline still applies.

**On a breach** the job's own `gate_slot.py` prints and records (receipt
`monitor.breach`) the kind, the last progress, the seconds without it, the CPU
spent since, the tree's resident set and its processes; then SIGTERM to the
child (its own unreaped child, so the pid is its own) and to each descendant,
each re-identified by its `/proc` start time through a pidfd before it is
signalled, so a recycled pid is never hit; after the grace, SIGKILL to those
left, found again the same way. It exits 124 (as `timeout(1)`), with the
command's own status as `child_exit`. A process outside the tree (one the job
did not start, or one that left the tree) is never signalled, and no job stops
another: the drain reports holders, it never signals them.

## 2. Bounded drain (DISCHARGE 2)

A `rust-suite` job takes `rust-suite/0`, closes `audit/admission`, and waits
for each audit slot at most `--drain-timeout` (default 2 h, which covers one
corpus route started just before the drain; `QUIRIN_GATE_DRAIN_TIMEOUT`; `off`
is the old unbounded drain). When the bound passes it:

1. names each blocking holder: slot, pid (from `/proc/locks`, else the holder
   note), argv, when it asked and when it took the slot, its tree's CPU and I/O
   over the wait (sampled when admission closed and again now), its resident
   set against its admission budget;
2. reopens admission and releases every lock it took (`rust-suite/0`, the
   admission lock, the audit slots taken so far, the legacy lock);
3. backs off `--drain-backoff` (5 min), in which queued audits are admitted,
   and drains again.

`--drain-attempts N` (default 0, unlimited) makes the Nth give-up a failure:
exit 75 with the report, a receipt with `ran: false` and every attempt; the
command never runs. `--wait-timeout` still bounds the whole wait (exit 75,
holders named). The command runs only after a complete drain, so a give-up
can never pass a gate: a drain that keeps failing keeps the step waiting or
fails it.

R7 stays consistent: the `--wait-record` says `waiting` through every back-off
(the job only waited) and `admitted` with the whole wait once drained; a job
stopped by its deadline keeps `admitted` (its run was analysis time). The
measure plugin reads only `state` and `waited_seconds`, which keep their
meaning.

Trade-off, recorded as follow-up `GATE-SLOT-DRAIN-DEFAULTS-REVIEW`: before this
card a drain admitted nothing until it finished; now each back-off lets queued
audits start, so behind a stream of long audits a `rust-suite` job can wait
longer than the longest single audit. The receipts now carry the data
(`limits`, `slot_record.drain.attempts`) to review the defaults.

## 3. `gate_slot.py status` (DISCHARGE 3)

`python3 scripts/gate_slot.py status [--json] [--interval S]` reads
`/proc/locks`, keeps the entries whose (device, inode) is a slot file under the
gate directory or the legacy lock, and for each process gives:

- its role: `holder`; `draining`, with the audit slots it still waits for and
  their holders; `waiting for a slot` (a slot job holding only the legacy lock
  shared, which is how every audit waiter shows, since it polls with
  non-blocking `flock`); `queued in the kernel for` a lock (a `->` entry, e.g.
  an untimed wait for the legacy lock); `legacy holder`;
- class, argv, cwd (from the holder note, else `/proc/<pid>/cmdline`), when it
  asked, when it took its slot, wait and hold time (process start time for a
  waiter);
- its job tree's CPU and I/O over the interval (default 5 s) and resident set
  against its admission budget (`OVER` when above), and the job's own last
  sample (`live`: last progress, stall timeout, peak).

It reads the same `QUIRIN_GATE_DIR` / `QUIRIN_GATE_LEGACY_LOCK` as the jobs and
takes no lock. Jobs that hold no gate lock while waiting (a timed wait for the
legacy lock, a nested job inside its parent's slot) are not listed: follow-up
`GATE-SLOT-STATUS-UNLOCKED-WAITERS`.

## 4. Resident set above the admission budget, while running (DISCHARGE 4)

Each sample sums the tree's resident set (the host's view; the receipt's
existing `max_rss_kib` stays `wait4`'s largest single process). For an `audit`
job, the first sample above its admission budget prints the sum, the largest
member and how far into the run, then again at every further GiB; the receipt's
`monitor.rss_over_admission` keeps the first crossing, the samples over and the
peak, and the end-of-run line repeats it. Every class records
`monitor.peak_tree_rss_kib`. The primary slot file's holder note carries the
last sample (`live`), which `status` shows.

## 5. Tests and the incident replay

[scripts/tests/test_gate_slot_starvation.py](../../../scripts/tests/test_gate_slot_starvation.py),
26 tests, every one in a scratch gate directory with a scratch legacy lock and
a fake meminfo; child processes spin, sleep, write or allocate; every process
a test starts is stopped by its captured pid. The existing 30 tests of
`test_gate_slot.py` pass unchanged.

| Item | Tests |
|---|---|
| 1 | a tree with a SIGTERM-ignoring spinning grandchild and no I/O is stopped (exit 124, receipt kind `stall`, CPU since progress, both pids listed, SIGTERM then SIGKILL to the grandchild), a bystander process stays alive, the wait record stays `admitted`, the slot is free after; steady file writes, stdout output and a progress-file change are each progress; the wall-clock deadline; an unreaped zombie does not hide a stall; unreadable I/O never stalls; without `/proc` only the deadline applies; defaults, flag over variable over default, and refusals (unitless `60`, drain flags on other classes) run nothing |
| 2 | a drain behind a spinning holder in another process gives up, names it (slot, pid, argv, hold time, CPU over the wait above 0.2 s, `wchar` +0, admission 7168 MiB), reopens admission and frees every lock; a drain backs off and retries until the audit ends; an audit queued behind the drain is admitted in the back-off, before the suite; the command line exits 75 after `--drain-attempts 2`, names the holder, runs nothing, wait record `waiting`; `--wait-timeout` still bounds an unbounded drain |
| 3 | `/proc/locks` parsing (holders, `->` waiters, OFD and POSIX lines); a real holder, a draining suite blocked by it and an audit waiter, through the function and `status --json`; a kernel-queued `lean` waiter behind a `flock`-style legacy holder; resident set against a 1 MiB admission note marked `OVER`; an empty gate |
| 4 | a child holding 64 MiB under a 16 MiB admission is reported while it runs (it exits 0 only after the report releases it); the slot note's `live` reading during a run, cleared at release |

The incident replay ([scripts/replay_incident.py](scripts/replay_incident.py),
record [replay.json](replay.json)) shrinks the incident to seconds in a scratch
gate with two audit slots: a spinning audit with `--stall-timeout 4s`, a
`rust-suite` job with `--drain-timeout 2s --drain-backoff 1.5s`, and a queued
audit. `status` during the drain shows the holder (CPU +0.5 s, I/O +0 over
0.5 s), the draining suite blocked by `audit/0` and the waiter. The suite gave
up after 2.1 s, named the spinner (CPU 2.02 s, every I/O counter +0 over the
2.0 s wait) and reopened admission; the queued audit ran in the back-off and
ended at 3.6 s; the spinner was stopped for its stall (exit 124, child exit
143) at 4.9 s; the suite drained on its second attempt and ran (exit 0) at
5.2 s. Before this card the suite would have waited for the spinner without
bound and the queued audit behind both.

The measure plugin (`crates/quirin-audit-plugin-measure`, CORPUS-GROUP-CHECKPOINTS'
files) needed no change: it runs every route as `gate_slot.py audit` and reads
the wait record's `state` and `waited_seconds`, both unchanged; its routes get
the class defaults (2 h stall, no wall-clock deadline). It clears the
environment and passes only the gate location variables, so a measurement
cannot override the defaults today: follow-up
`GATE-SLOT-DEADLINE-OVERRIDE-THROUGH-MEASURE`.

## 6. Gate

`python3 scripts/gate_scope.py --base 13337d0af --out <worktree>/private/gate`
on `9c8c34dae` ([gate.json](gate.json)): scoped mode, one package
(`quirin-audit-plugin-measure`, which names `scripts/gate_slot.py`); no Lean,
no provider gate. The gate's own `rust-suite` step ran under this card's
`gate_slot.py` on the live gate: it waited 856 s for `rust-suite/0`, closed
admission, drained in 662 s on its first attempt (within the 2 h bound) and
ran 106 s; its receipt records the default `limits` and 10 monitor samples.

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| Clippy | 0 | |
| Rust suite (`rust-suite` slot) | 0 | 7 units, 45 passed, 0 failed, 0 ignored (includes `slot_wait.rs`, the measure plugin's real-slot test); 1,625 s with 1,518 s of slot wait |
| Python suite | 0 | 651 tests, 0 failures |

`integration-amendments.json` passes `scripts/apply_amendments.py --check`
once the integrator's R11 records (`b0ca1c98b`,
`docs/plans/gate-time-reduction/r11-amendments.json`, which registers
`GATE-SLOT-ADMISSION-STARVATION`) are applied; at the base alone it refuses
the retirement of the not-yet-registered id, as it must.

## 7. Follow-ups

- `GATE-SLOT-DRAIN-DEFAULTS-REVIEW`: review the 2 h stall, 2 h drain and 5 min
  back-off against the receipts the slots now write, including how long
  `rust-suite` jobs wait behind back-to-back audits.
- `GATE-SLOT-DEADLINE-OVERRIDE-THROUGH-MEASURE`: let a declared measurement
  pass `QUIRIN_GATE_STALL_TIMEOUT` / `QUIRIN_GATE_DEADLINE` to its routes.
- `GATE-SLOT-STATUS-UNLOCKED-WAITERS`: list waiters that hold no gate lock.
- `GATE-SLOT-DEADLINES-WITHOUT-PROC`: stall and resident-set readings, status
  and blocker deltas need `/proc`; on macOS only the wall-clock deadline works,
  and the existing receipt reads `ru_maxrss` as KiB where macOS reports bytes.
