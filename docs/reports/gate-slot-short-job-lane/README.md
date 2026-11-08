# GATE-SLOT-SHORT-JOB-LANE

Round 30 (2026-11-08), base `1742f00cb` (GATE-SLOT-FIFO-ORDER's tip; not
landed). Discharges `GATE-SLOT-SHORT-BUILD-WAIT` (registered by
FLUENT-BIT-LINK-PREPARE, tip `a0ac2cae9`, not landed). The user's decision
(2026-11-08), answering "Should a short job, like a 3-minute prepare, be
allowed to jump ahead of long queued audits?": "Yes, add a fast lane." Requirement
R13 of [gate-time reduction](../../plans/gate-time-reduction/README.md), added by
this card. Registered and retired in its
[integration-amendments.json](integration-amendments.json) (the id
`GATE-SLOT-SHORT-JOB-LANE` was not registered before).

The incident: `prepare clang-build --link` (3 min 17 s, peak 832 MB) waited
5 h 8 min for an audit slot at a 4 GiB budget, behind running audits, queued
7 GiB audits and repeated `rust-suite` drains. GATE-SLOT-FIFO-ORDER's
[README §8](../gate-slot-fifo-order/README.md#8-decisions-and-follow-ups) says why
arrival order alone does not bound a short job's wait: it bounds it to the work
queued ahead of it, and a 3-minute job still waits for hours of audits.

Changed: `scripts/gate_slot.py`, [validation](../../agents/validation.md#gate-slots)
(rule text; the card-brief guidance in
[operations](../../agents/operations.md) does not say heavy builds go through the
audit class, so it is unchanged), R13 of the plan, one fixture and the new test
file. No analyzer, contract or Lean change and nothing an audit reads, so no
prediction and no declared measurement.

## 1. The lane rule

`gate_slot.py short [--memory-budget 1G|2G] [--max-wall D] -- <cmd>`.

| | |
|---|---|
| slots | `SHORT_SLOTS = 2`: `<gate dir>/short/0`, `short/1` |
| order | its own arrival-order queue (family `short`, the GATE-SLOT-FIFO-ORDER tickets); independent of the audit queue |
| audit locks | none taken: not behind queued audits, not shut out by a draining suite |
| legacy lock | shared, as every slot job (a `flock <legacy>` job still excludes the lane) |
| wall bound | `--max-wall`, default 30 min, at most 1 h; cannot be 0/off; `--deadline` is refused for the class; `QUIRIN_GATE_DEADLINE` is ignored for it |
| declared memory | `--memory-budget 1G` (default) or `2G`; 4G/5G/7G refused (exit 2: "not memory-light: use the audit class") |
| stop grace | at most 2 min (`--stop-grace`; default 60 s) |
| stall timeout | none by default (the wall bound is the guard; an I/O-quiet link must not be killed as a stall) |

**What qualifies** is what the bounds enforce, not a name: a CPU-bound,
memory-light step that finishes in minutes: `prepare` (the measured case),
single-purpose preparation, link and regeneration steps. A parallel `cargo build`
of the whole workspace was not measured and may sum past the hard stop below;
validation.md says to measure one before sending it to the lane.

**Wall bound: stopped, not only recorded.** Past `--max-wall` the monitor
SIGTERMs the command's process tree, SIGKILLs the survivors after the grace, and
the script exits 124; the receipt has `lane.wall_overrun: true`,
`lane.complete: false`, `stopped: deadline` and the existing `monitor.breach`.
Why stopped: the lane's promise to a draining suite is that a lane job ends
within its bound (§3); a recorded overrun would be an unbounded drain, the very
failure R11 removed for audits. Why that is fail-closed for a *half-written
tree*: the exit is 124 (non-zero), so a caller that checks the exit never reads a
stopped job as finished, and `lane.complete` is true only for an exit 0 that was
not stopped. The gate cannot know what the command had written and does not clean
it up; the rule asks the command to write its completion record last, and that
this holds for `prepare` and the other lane commands was not audited here
(follow-up `GATE-SLOT-LANE-PARTIAL-TREE`).

**Memory.** The declared budget is not applied to the command (unlike the audit
class's `QCORE_AUDIT_JOBS`); it is what admission counts. A tree whose summed
resident set exceeds it is warned and recorded (`monitor.rss_over_admission`,
`lane.memory_over_declared`) exactly as an audit's; it is not stopped for that,
because a six-job build's summed RSS double-counts shared pages and a stop at
the declared figure would kill a legitimate prepare (the incident recorded a peak of 832 MB; whether
that was the largest process or the tree's sum is not stated there). The hard stop is at
`SHORT_HARD_STOP_MIB = 4096`, the smallest audit budget: past that the job is an
audit-sized job in the lane; the tree is stopped like an overrun (`stopped:
memory`, exit 124). A sample is every 10 s, so a spike between samples can pass.

## 2. Memory admission: the lane against audits

Admission counts `MemAvailable` (the host's reading, which excludes memory
already in use). The lane adds `lane_reserved_kib`: for each lane slot held
(probed by a momentary lock), its declared budget less the resident set of the
job's last monitor sample (all of it before the first sample), floored at 0.

- **A lane job** is admitted when `MemAvailable - reserved >= its declaration`;
  otherwise it waits. A lane waiter that cannot be admitted by memory holds back
  nothing behind it (count 0 in `queued_slot`), so a 1 GiB job behind a 2 GiB
  job that does not fit runs (`passed_over`).
- **An audit** takes the same reserve from its reading before computing its
  slot count (`floor((MemAvailable - reserved) / budget)`, at least one) and for
  the waiters ahead of it; its receipt's `mem_available_kib` is that reduced
  value. So a lane job admitted a moment ago counts against audits before it has
  allocated, and not twice once its memory is in `MemAvailable`.
- A held lane slot with no readable note counts at 2 GiB (fail closed); a
  `rust-suite` job's own hold of the lane counts for nothing (the audits are shut
  out by its admission lock); a stale note left by a `kill -9` is not a hold (the
  probe locks the slot).

With 8 GiB available, two 4 GiB audit slots become one while a 2 GiB lane job has
not yet allocated (test).

## 3. Drains

A `rust-suite` job drains the audits exactly as before, **with the lane open**:
the lane never waits for what a drain waits for. Once every audit slot is held
the suite closes the lane (`short/admission`, exclusive) and takes `short/0` and
`short/1`, which waits for the lane jobs in flight. That wait is bounded by their
wall bound: a lane job ends within `--max-wall` (at most 1 h) plus one monitor
sample (10 s) plus the stop grace (at most 2 min), and `SHORT_DRAIN_BOUND` is that
plus 5 min of slack (a little over 1 h 7 min, normally minutes: the incident's
prepare takes 3 min). A wait past the bound means a wedged lane job (a stopped
`gate_slot.py`): the drain gives up as an audit drain does, naming the lane holder
in `blockers`, releasing everything (audits and lane reopen), backing off and
retrying (`lane_closed_at` in the attempt record). Test: a never-ending lane job
and a bound of 0.3 s; and a real lane job at `--max-wall 2s` that the suite waits
out.

**Decision, and the choice it makes.** The suite then *holds* both lane slots for
its whole run, so its load-sensitive rows (constraint 16: they never share the host
with an audit) do not share it with a CPU-bound lane job either, and a lane job
that arrives during the run waits for the suite to end (still ahead of every audit,
but up to the suite's ~2 h). The user's decision named queued audits and drains,
not a running suite; the fail-closed side keeps a measurement's validity, and the
alternative (the lane runs beside the suite, receipts marked) is
`GATE-SLOT-LANE-DURING-SUITE-RUN`, a user choice. A lane job started by the
suite's own command (`gate_slot.py short` nested under `rust-suite`) is covered by
the suite's slots (`COVERS`); nested under an audit it is refused (exit 2).

**Interaction with the FIFO tickets.** Lane jobs queue in family `short`; a suite
takes no lane ticket and holds the audit-family ticket only while it waits to
start its drain. The lane's order is arrival order among lane jobs (tickets, lapse,
reclaim of dead owners: the FIFO card's rules), and neither queue holds the other
back (test).

## 4. Older clients on one host (acceptance 2)

Tested against both older scripts byte for byte: `gate_slot_before_queue.py.txt`
(pre-FIFO) and the new `gate_slot_fifo_only.py.txt` (this base's
`scripts/gate_slot.py`, sha256 `1ea95433...`, git blob `440755066f7a`), each run
against the current one in one scratch gate.

- *No deadlock.* An older client has no `short` class (it exits 2 and creates no
  `short/` directory) and takes no lane lock. The current suite takes lane locks
  *after* the audit locks and a lane job takes no audit lock, so no cycle can form
  between a current suite, a lane job and an older audit/suite. A current suite
  waits for an older audit holder as before while the lane runs.
- *No double admission.* Each slot is an exclusive flock on one file that only
  current clients open for the lane; audit/suite/lean slots are as before. Tested:
  older and current audits and four lane jobs contending (intervals logged by the
  commands): an audit slot is never held twice, and never more than two lane jobs
  at once, both lane slots used.
- *Not guaranteed during the transition.* An older `rust-suite` does not close the
  lane, so a lane job may run beside it (its load-sensitive rows are guaranteed
  alone only once every worktree has this script); an older audit counts
  `MemAvailable` without the lane's reserve; a lane job is not held back by an
  older audit holder (it never was meant to be). Arrival order is, as before,
  guaranteed among queue-aware jobs only.

## 5. Status

`gate_slot.py status` shows lane jobs as class `short` with their queue position in
family `short`, a line `short lane: N of 2 slots held by pid ..., waiting: pid ...`,
`short_lane` (`slots`, `holders`, `waiters`) in `--json`, and a suite that has
drained the audits and waits for the lane as "draining (audits drained; holds
lane admission, waits for lane jobs)" with the lane holders as `blocked_by`.

## 6. Tests

[scripts/tests/test_gate_slot_lane.py](../../../scripts/tests/test_gate_slot_lane.py),
32 tests, scratch gate directories, scratch legacy lock, fake meminfo (the live
`/tmp/quirin-gate` is never touched); in-process waiters are threads, the rest real
`gate_slot.py` processes stopped by captured pid; nothing orders events by a fixed
sleep. A mutation (the lane taking the audit admission lock) fails five of them.

| Area | Tests |
|---|---|
| overtaking | a short job arriving after M, A2, a suite and A3 on one audit slot is admitted at once and leaves the audit queue as it was; a short job runs while a suite drains and audit admission is closed |
| cap and order | four jobs over two slots in arrival order; the lane and audit queues do not hold each other back |
| memory | the lane reserve cuts two 4 GiB audit slots to one; not counted twice once used; a lane job waits for its declared memory and a smaller later one passes it; a held slot with no note counts at 2 GiB; a suite's hold counts for nothing |
| drains | a suite waits for a lane job in flight, holds the lane for its run, and a later lane job runs after; a wedged lane job does not hold a drain past the bound (blockers named, everything reopened); a real lane job stopped at `--max-wall` while a real suite waits; nested under a suite is covered, under an audit refused |
| command line | defaults and receipt; every refusal (`--max-wall` 2h/0/off/61m/unitless, `--deadline`, 4G/7G, grace 10m, and `--max-wall` on other classes); largest accepted values; the environment's deadline does not lift the bound; overrun: exit 124, `wall_overrun`, `complete: false`, half-written file left and no completion record, slot free after; a failing command is not an overrun; the hard memory stop |
| older clients | both fixtures are the scripts as they were; neither has the class; a lane job runs beside an older audit holder and beside an older suite; a current suite waits for an older audit while the lane runs; no slot held twice among older and current audits and four lane jobs |
| crashes | `kill -9` of a lane holder frees its slot and its reserve (stale note is not a hold); a dead waiter's ticket is reclaimed; a stopped waiter is skipped after the lapse |
| status | two holders and a waiter in text and `--json`; a suite waiting for the lane |

## 7. Replay of the incident

[scripts/replay_incident.py](scripts/replay_incident.py), record
[replay.json](replay.json): a scratch gate with one audit slot (8 GiB), a holder
(4 s), then `M`, `A2` (audits), `R` (a rust-suite drain), `A3`, and the prepare
`P`, 0.3 s apart; five trials per client.

| Client | Orders the five queued jobs and `P` ran in | `P`'s wait |
|---|---|---|
| the queue-only script, `P` as `audit` (the incident) | `M, A2, R, A3, P` in 5 of 5: `P` last | 3.97-4.07 s, after the holder and every job ahead |
| this card, `P` as `short` | `P, M, A2, R, A3` in 5 of 5: `P` first, before the holder ended | 0.05-0.06 s; the queued jobs keep arrival order in 5 of 5 |

Compressed in time: the real wait was 5 h 8 min for a 3 min 17 s job.

## 8. Gate

`python3 scripts/gate_scope.py --base 1742f00cb --out <worktree>/private/gate-1` on
`d6a421f63` ([gate.json](gate.json)), exit 0, 6 steps passed (scoped mode; the
scoper chose `quirin-audit-plugin-measure` and `qcore-compiler::test::ct_corpus`
for a Python-and-docs change).

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| Clippy (`quirin-audit-plugin-measure`; `qcore-compiler` `ct_corpus`) | 0, 0 | `-D warnings` |
| Rust suite (`rust-suite` slot) | 0 | 8 units, 53 passed, 0 failed, 0 ignored; 12,185 s, of which 12,035 s waiting for the slot on the live gate (ticket 2; one drain, no give-up) |
| Python suite | 0 | 756 tests, 0 failures, 0 skipped (includes the 32 new) |

Beforehand the 69 existing slot tests (`test_gate_slot.py`,
`test_gate_slot_queue.py`) passed unchanged on the new script, and the 32 lane
tests pass alone (13 s). The suite step ran under this card's `gate_slot.py` on
the live gate beside older clients. Logs are private (`private/gate-1/`; console
log outside the worktree). `integration-amendments.json` passes
`scripts/apply_amendments.py --check` (exit 0) once `fluent-bit-link-prepare`'s
amendments are applied first, and not alone, because it retires
`GATE-SLOT-SHORT-BUILD-WAIT`, which only that card registers.

## 9. Decisions and follow-ups

- The wall bound stops the job; the declared memory is recorded and the hard stop
  is at 4 GiB (§1). The suite holds the lane during its run (§3).
- `GATE-SLOT-LANE-DEFAULTS-REVIEW` (registered): two slots, 30 min/1 h, 1G/2G,
  4 GiB and the suite's hour-long lane wait come from one measured case.
- `GATE-SLOT-LANE-DURING-SUITE-RUN` (registered): keep the lane closed during a
  suite's run, or open it with marked receipts: the user's choice.
- `GATE-SLOT-LANE-PARTIAL-TREE` (registered): audit that each command sent to the
  lane (`prepare` first) leaves nothing that reads as complete after a stop.
- `GATE-SLOT-QUEUE-OLD-CLIENT-RETIREMENT` (amended): the queue-only fixture is
  deleted with the other when no worktree runs a script without the lane.
- Not done, outside the scope: the measure plugin and `quirin-audit` still run
  their routes under `audit`; `prepare` and its callers were not edited to call the
  lane (the launching briefs choose it).
