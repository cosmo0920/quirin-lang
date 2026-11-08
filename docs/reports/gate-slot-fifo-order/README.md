# GATE-SLOT-FIFO-ORDER

Round 26 (2026-11-08), base `1e522ca5d`. Launched by the integrator under the
user's standing instruction to keep cards running; it also covers
`MEASURE-SLOT-WAIT-EXCLUSION-CAP`. Neither id was registered; this card
registers and retires both through its
[integration-amendments.json](integration-amendments.json). Requirement R12 of
[gate-time reduction](../../plans/gate-time-reduction/README.md) (added by this
card from the integrator's record).

The incident (integrator's record, 2026-11-07/08): the audit slots were not
first come, first served. The measurement of SCALAR-STATEMENT-FORMS
(`private/measure/scalar-forms-20261107T072754Z`) waited 7.5 h, then 15.5 h,
while audits and `rust-suite` drains that asked hours later took the slots; it
then died with `plugin-timeout: exceeded the wall-time quota of 36000000 ms
(36000000 ms of declared slot waits excluded, limit 36000000 ms)`, because R7's
exclusion was capped at the quota (`slot_wait_limit_ms` equalled `--timeout-ms`).
GATE-SLOT-ADMISSION-STARVATION bounded drains but added no ordering.

Changed: `scripts/gate_slot.py`, the subcommand host
(`crates/qcore-audit/src/subcommand_plugins.rs`), the plugin SDK's `Quotas`
(`crates/quirin-audit-plugin`), the measure plugin's `timing` slot, their tests,
and the rule text in [validation](../../agents/validation.md#gate-slots). No
analyzer, contract or Lean change and nothing an audit reads, so no prediction
and no declared measurement.

## 1. Arrival order (acceptance 1)

**Mechanism.** A job that must wait takes a ticket in its family's queue
directory, `<gate dir>/<family>/queue/<seq>-<pid>.ticket`, after it holds the
legacy lock; no daemon, only files beside the existing flocks. Families: `audit`
for the `audit` and `rust-suite` classes (a suite takes every audit slot, so its
place among the audits is the point) and `lean`. The sequence number is read
from `.sequence` under a flock held for a few system calls (and is at least one
more than any ticket present, so a deleted counter cannot reorder); the ticket
is created already flocked by its owner (`O_CLOEXEC`: the flock lives exactly as
long as the process), carries pid, start time, class, admission budget and
state, and is renamed into place, so it is never visible unlocked. The owner
touches it on every poll.

**Admission rule.** A job may take a slot only if no job ahead of it (smaller
number) can take that slot. Each job ahead takes, in order, the lowest free slot
below its own count (`floor(MemAvailable / admission budget)`, at least one for
`audit`; both slots for `lean`); the job takes the lowest of those left below its
own count (`queued_slot`, a pure function). Consequences, each tested:

- the earlier of two equal waiters always wins, whoever polls first;
- a job ahead that cannot be admitted now (every slot its memory admits is busy)
  holds back nothing else, so memory never blocks the whole queue: with 14 GiB
  free, a 4 GiB audit behind an 11 GiB corpus audit that waits for slot 0 runs in
  slot 1 (`passed_over` in its receipt), and the corpus audit keeps slot 0 for
  itself when it frees; one slot is always counted, so a head is never
  unadmittable by memory alone;
- a `rust-suite` job ahead holds back every audit behind it, and a suite starts
  its drain only when no job is ahead of it, so it waits for the audits queued
  before it (the incident's drains had jumped them).

**A queue is not blocked for ever.** A ticket stops holding the others back when

| Case | Rule |
|---|---|
| owner gone (crash, `kill -9`) | its flock is free **and** its pid is gone, a zombie, or started at another time (a reused pid); whoever scans unlinks it. A live pid with a free lock is never reclaimed (it lapses, below); a held lock whose pid is not visible (another PID namespace) is never reclaimed |
| owner stopped or wedged | no poll for `QUIRIN_GATE_QUEUE_LAPSE` (default 10 min; `0`/`off` never): skipped, not removed, counted again at the next poll |
| drain gave up | the `rust-suite` ticket is `backoff`: it keeps its place and resumes there, the audits behind it run meanwhile (GATE-SLOT-ADMISSION-STARVATION's property, kept) |
| restart | a killed and re-run job asks again, behind the jobs that asked meanwhile; a cleaned `/tmp` starts a fresh queue |
| `QUIRIN_GATE_QUEUE=off` | no ticket: the job competes as an older client does (an escape hatch) |

A job that gives up (`--wait-timeout`, drain attempts) or is interrupted drops
its ticket. The receipt's `slot_record.queue` records family, ticket, ahead and
position at the request, the last position seen, tickets reclaimed, tickets
skipped, tickets passed over and when it left the queue; `limits` records
`queue`, `queue_lapse_seconds` and where it came from.

## 2. Transition: old and new clients on one host (acceptance 4)

Every old worktree keeps its own `gate_slot.py`; both versions take the same
slot flocks (`<class>/<n>`, `audit/admission`, the legacy lock) in the same
order and with the same modes. The ticket is advisory and adds **no lock any
client must take**:

- *No deadlock.* An old client never looks at tickets and a new client holds
  no gate lock while it waits for its turn except the shared legacy lock every
  waiter already held. A new suite at the head waits for the same slot locks
  an old suite would. Every ticket wait ends: its owners are served, reclaimed or
  skipped (§1).
- *No double admission.* A slot is an exclusive flock on the same file for both
  versions; the ticket only decides who tries. Tested: current and old waiters
  contending for one slot never overlap (intervals logged by the commands).
- *What is not guaranteed.* An old client can be admitted ahead of a ticketed
  job (it takes any free slot), and an old suite closes admission as before. Arrival
  order is guaranteed among queue-aware jobs; the guarantee grows as worktrees
  take this `gate_slot.py` (a worktree's rebase brings it). `status` lists an old
  waiter as "not queued: a gate_slot.py without arrival order".
- *Old `status` on a new gate.* It reads class directories' files only; the
  `queue` subdirectory is ignored.

Tests run the pre-card script, byte for byte
([fixture](../../../scripts/tests/fixtures/gate_slot_before_queue.py.txt),
`sha256 67a22457...`, git blob `14d1fe3e8`), against the current one in one
scratch gate.

## 3. Slot wait and the quota (acceptance 2)

`quirin-audit measure` runs each route under `gate_slot.py audit --wait-record`;
while every route in flight waits, the host does not count that time against
`--timeout-ms`. Before: the exclusion was capped at `--slot-wait-limit-ms`, whose
default was the quota itself, so a queue longer than the quota killed the run.
Now:

| Limit | Meaning | Default | Override |
|---|---|---|---|
| `--timeout-ms` | the analysis quota; slot waits are not counted | 30 min (unchanged) | host flag |
| `--slot-wait-limit-ms` | the most declared wait that is excluded | the overall wall bound (was: the quota); `0` counts every wait | host flag |
| `--wall-limit-ms` | the **overall wall bound**: the run is stopped at this elapsed time, analysis and declared waits together, with `plugin-wall-limit` | 7 days; never below the quota (a larger quota raises the default, a smaller explicit value is refused, exit 2) | host flag |

Seven days against the observed 7.5 h and 15.5 h waits (and the 10 h quota the
measurement ran under). The analysis quota still stops a plugin that
analyses too long (`plugin-timeout`); the bound stops a plugin that declares
waits for ever. Both limits are recorded in: the request's `quotas` (the plugin
sees what it runs under; both fields are optional in the SDK so a request from
an older host still parses), the host report's `run` (`wall_time_quota_ms`,
`slot_wait_limit_ms`, `wall_limit_ms` beside the existing times), the invocation
record, and the measure report's `timing.limits`. R7's records are unchanged
in meaning: `wall_time_ms`, `slot_wait_ms`, `analysis_time_ms`, the wait record
(`waiting`/`admitted`, `waited_seconds`) and the marker.

Tests: `crates/qcore-audit/tests/subcommand_plugins.rs` (a wait three times the
quota completes by default and the limits are in the request, run and invocation
records; the bound stops a plugin that declares waits for ever, also against a
huge slot-wait limit; the quota still stops a plugin that declares none; a bound
below the quota is refused; a larger quota raises the default; flag parsing) and
`crates/quirin-audit-plugin-measure/tests/slot_wait.rs` (a real held
`rust-suite` slot through the real host, now under the default limit, with the
measure report's `timing.limits`).

## 4. Status (acceptance 3)

`gate_slot.py status` shows, for each waiter, `queue position N of M (family)`,
the pids ahead of it, its state (`backoff`, `lapsed`), and `--json` lists every
queue under `queues`. A suite backing off holds no gate lock, so it is listed
from its ticket ("waiting for a slot (drain gave up; backing off, place kept)").
`status` reclaims nothing.

## 5. Tests (acceptance 4)

[scripts/tests/test_gate_slot_queue.py](../../../scripts/tests/test_gate_slot_queue.py),
39 tests, scratch gate directories, scratch legacy lock, fake meminfo; waiters are
threads of the test (own file descriptions, so `flock` conflicts as between
processes) or real `gate_slot.py` processes; fake owners are processes the test
started and stopped by captured pid; nothing orders events by a fixed sleep. The
live `/tmp/quirin-gate` is never touched. A mutation (queue off by default)
fails 7 of the 11 order and budget tests.

| Area | Tests |
|---|---|
| decision (`queued_slot`) | five pure cases: alone, each ahead takes the lowest, a blocked head holds nothing back, a suite holds everything, equal waiters |
| order | `lean` and `audit` waiters admitted in arrival order though the earliest polls slowest; a suite waits for the audits queued ahead and holds back those behind; a backing-off suite keeps its place while a late audit runs; a timed-out job leaves no ticket; a nested job takes none |
| mixed budgets | an 11 GiB audit waiting for slot 0 does not hold back a 4 GiB audit; a free slot a head could take is kept for it; memory below every budget still admits one slot; a stuck head is skipped after the lapse and counts again at its poll; a suite is not held by a ticket that stopped polling |
| crashed waiters, restarts | a dead pid's ticket is reclaimed and blocks nobody; a reused pid (other start time) does not keep a ticket alive; a live owner with a free lock is not reclaimed; a held lock of an invisible pid is not reclaimed; a real `kill -9` waiter's ticket is found stale and reclaimed by the next waiter; a restarted waiter goes to the back and the counter survives a deleted counter file; a cleaned gate directory starts at 1; a ticket abandoned mid-creation is swept |
| races | 24 simultaneous arrivals get 1..24; eight audits over two slots never exceed two |
| command line | the receipt's `queue` record and the stderr position line; the lapse and `QUIRIN_GATE_QUEUE=off` overrides and their `limits`; a malformed lapse is exit 2 |
| status | three real waiters show positions 1-3 of 3 with who is ahead, in text and `--json`; a backing-off suite and an old-client waiter are told apart; status reclaims nothing |
| older clients | the fixture is the old script (sha256); a current waiter is admitted when an old holder ends; an old waiter gets the slot a current holder frees; current and old waiters never hold one slot together; an old suite drains current audits and they wait for it; a current suite drains an old audit; a current drain that gives up reopens admission for an old audit |

`test_gate_slot.py` (30) and `test_gate_slot_starvation.py` (26) pass
unchanged; `test_holders_a_draining_suite_and_a_waiter` was not touched and
passed. Rust: `subcommand_plugins` 16 (one new), `qcore-audit` lib 169,
`quirin-audit-plugin` 7, the measure plugin's 27 + 2 + 10 + 1 + 5.

## 6. Replay of the incident

[scripts/replay_incident.py](scripts/replay_incident.py), record
[replay.json](replay.json): in a scratch gate with one audit slot, a holder, then
`M` (the measurement's audit), `A2`, a `rust-suite` job `R` and `A3` arrive in
that order, 0.5-1.5 s apart; five trials per client.

| Client | Orders the five jobs ran in |
|---|---|
| before the card | `R` first in all five (it closes admission and takes the slot ahead of three earlier waiters), then A3, A2, M / A3, M, A2 / A2, M, A3 / A2, A3, M / M, A2, A3: `M` last in two trials |
| this card | `M, A2, R, A3` in all five |

## 7. Gate

`python3 scripts/gate_scope.py --base 1e522ca5d --out <worktree>/private/gate-1`
on `8aeb7908a` ([gate.json](gate.json)), exit 0, 8 steps passed: scoped mode, six
Rust packages (`qcore-audit`, `qcore-compiler`, `qcorec`, `quirin-audit-plugin`,
`quirin-audit-plugin-libevent`, `quirin-audit-plugin-measure`) and one test
target (`qcore-verification::composition_bounds`); no Lean, no provider gate.

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| Clippy (six packages; `qcore-verification` target) | 0, 0 | `-D warnings` |
| Rust suite (`rust-suite` slot) | 0 | 206 units, 2,655 passed, 0 failed, 4 ignored; 6,131 s, of which 5,629 s waiting for the slot on the live gate |
| probes | 0, 0 | `reevaluation_probe`, `reevaluation_followup` |
| Python suite | 0 | 724 tests, 0 failures (includes the 39 new and the 56 existing slot tests) |

The suite step ran under this card's `gate_slot.py` on the live gate: ticket 1,
nobody ahead in the queue-aware family, admitted when the older clients in front
of it (which ignore the queue) released `rust-suite/0`; its receipt records the
`queue` block. Logs are private (`private/gate-1/`; console log outside the
worktree). `integration-amendments.json` passes
`scripts/apply_amendments.py --check` (exit 0).

## 8. Decisions and follow-ups

`GATE-SLOT-SHORT-BUILD-WAIT` (registered by FLUENT-BIT-LINK-PREPARE: a
3-minute prepare waited 5 h behind audits and drains) is **not covered and not
retired** by this card: it is not on this base's matrix, and arrival order
only bounds a short job's wait to the work queued ahead of it; it does not make
a 3-minute job fast. Covering it needs a different design (a lane for short
jobs, or a separate class that audits do not exclude), which is a user choice
about whether a short job may overtake. Leave it registered.

- `GATE-SLOT-QUEUE-OLD-CLIENT-RETIREMENT` (registered): when no worktree
  runs a pre-queue script, retire the fixture and the "not queued" wording.
- `GATE-SLOT-QUEUE-DEFAULTS-REVIEW` (registered): the lapse (10 min) and the
  seven-day bound were chosen from one incident, not from receipts.
- `PLUGIN-QUOTAS-OLD-SDK-REQUEST` (registered): `Quotas` keeps
  `deny_unknown_fields`, so an out-of-tree plugin built on the SDK before this
  card rejects a request that carries the two new fields; the in-tree plugins
  (measure, libevent) are built with the host.
- `SLOT-WAIT-DECLARED-BY-PLUGIN` and `GATE-SLOT-STATUS-UNLOCKED-WAITERS`
  (amended): the first is now bounded by the overall wall bound rather than
  quota + limit; the second is narrowed (a waiter holds a ticket from the legacy
  lock to admission; a timed legacy wait and a nested job still hold nothing).
