# HOST-REQUEST-READ-BY-DETACHED-DESCENDANT: prediction

Frozen before any implementation code. Base `b565f79ab` (local `main`). At
the freeze only this file, the report's scripts and the probe cases added to
`crates/qcore-verifier-host/tests/request_consumption.rs` (in a
base-compatible form: a refusal asserts only "not `Completed`, exit 0") exist.

## 1. The defect

`ProcessProvider::run_process` takes its consumption decision once the direct
child has exited and both output pipes are at end of file
(`crates/qcore-verifier-host/src/request_consumption.rs`, card
`HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`). A descendant that inherited
the request pipe as its stdin, and put both outputs elsewhere, can still be
reading at that moment. Its reads that land before the decision are credited
and its later reads are not, so the outcome of one provider depends on where
the decision falls among the descendant's reads.

Shapes, and what the base does with them:

| Case | Direct child | Descendant (outputs on `/dev/null`) | Base |
|---|---|---|---|
| held, silent | answers, never reads | holds stdin, never reads (`sleep 30`) | `RequestUnconsumed` |
| held, detached | same | same, after `setsid -f` (outside the group) | `RequestUnconsumed` |
| held, large frame | same | same, 8 MiB frame | `RequestUnconsumed` (frame undelivered) |
| held after reading | same | reads to end of file, then holds (`cat; sleep 30`) | **either**: accepted if the read lands before the decision |
| held briefly | same | holds 50 ms, never reads | `RequestUnconsumed` |
| descendant reads | same | reads to end of file and exits | **either** |
| descendant reads late | same | waits 50 ms, then reads to end of file | **either**, mostly refused |
| descendant reads, large | same | reads 8 MiB to end of file | **either**, mostly refused |
| background on `/dev/null` | reads to end of file | `sleep 30 </dev/null` | accepted |

## 2. Census (frozen before implementation)

Re-checked over every provider, plugin, package fixture and test the host
runs through `run_process`:

* **Rust providers** (the five shipped packages, `sample_boolean_provider`,
  `configured_audit_provider`, the SDK example checker, the w3 fixture
  provider): none spawns a process (`Command::new`, `fork` and `posix_spawn`
  appear in no provider crate or fixture provider). No descendant exists.
* **Shell fixtures:** every fixture is `#!/bin/sh` (32 sources). Eleven lines
  start a background job at the base (`scripts/census.py`; it also lists this
  card's two new probe lines once they exist). None
  redirects the job's stdin from the request: a non-interactive `sh` (dash
  here) gives an asynchronous list `/dev/null` as stdin unless the list
  redirects it (POSIX; measured on this host: the background job's fd 0 is
  `/dev/null`). Each is one of:
  - holds the output pipes, so the decision already waits for its exit
    (`standalone_host.rs:171`, `cycle20_verifier_plugins.rs:764`,
    `enforced_process_limits.rs:360,381`, `execution_quota.rs:441,545,577`);
  - waited for by the direct child (`reevaluation_followup.rs:92`);
  - explicitly `</dev/null` with outputs closed, an accepted control
    (`cycle20_verifier_plugins.rs:835`);
  - a subcommand plugin, another protocol (`subcommand_plugins.rs:249`).
* **Result: no provider or fixture has the shape.** No real, intended
  provider is refused by the change, and no decision is owed to the user.

## 3. Mechanism chosen

**The decision waits until the host is the request pipe's last holder,
bounded by the existing deadline.**

* **Linux, frame wholly written.** The host keeps its probe (a read end it
  never reads). To ask whether any other process holds a read end, it opens a
  write end through `/proc/self/fd/<probe>` (`O_WRONLY | O_NONBLOCK |
  O_CLOEXEC`; opening a pipe this way never waits), closes the probe and polls
  the write end: `POLLERR` is set exactly when the pipe has no reader left.
  `FIONREAD` on that write end then counts the bytes nobody read, and no
  reader can appear any more, so the count is final. If a reader remains, the
  probe is reopened through the write end and the write end closed again, so
  a reader waiting for end of file is not held up; the host checks again after
  its backoff quantum. Measured on this host before any host code
  (`scripts/holders_probe.py`): a live holder is seen, a released pipe gives
  its unread count (0 or 5 of 5), and a reader blocked waiting for end of file
  still gets it and exits.
* **Linux, frame not wholly written.** The host still holds the write end,
  and holds no read end while writing. A write that does not break proves a
  reader exists, so the decision is deferred and the write goes on: it either
  completes (then the case above), or breaks (`RequestUnconsumed`, final:
  nobody can read any more).
* **At the deadline,** a successful exit whose request pipe is still held is
  refused as `ProcessOutcome::RequestHeldByDescendant`
  (`ProviderFailure::RequestHeldByDescendant`, `request-held-by-descendant:
  …`), distinct from `RequestUnconsumed` and from `WallClockTimeout`.
* **Inability to decide refuses.** A failed open, poll or `FIONREAD` is `Io`
  ("request holders are unobservable"), never acceptance.
* **Off Linux** no independent reopen exists (`/dev/fd` is `dup` there, so the
  probe is the read end itself), and no other mechanism observes the other
  holders: every successful exit is refused as `Io` (fail-closed). That path
  is unexecuted (no macOS host); the consequence is recorded with
  `HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED`.

**Why a bounded wait rather than an immediate refusal.** An immediate
refusal makes the outcome a function of the holders at the decision, but the
holders at the decision are themselves scheduling: a descendant that reads
the whole request and exits ("descendant reads") would be accepted when its
exit lands first and refused otherwise. Waiting until the last other holder
is gone makes the outcome a function of the final consumption, which no later
schedule can change; only a holder that outlives the deadline is refused, and
the deadline is the bound every other host wait already has. The cost is that
a provider whose descendant keeps the request pipe takes its whole deadline.

## 4. Predicted outcomes

* **Probe binary at the tip,** 50 runs per condition (idle, host stall 20 and
  50 ms, first write +30 ms, close +30 ms, both + 20 ms stall), **0
  deviations**:
  - held silent / detached / large / after reading:
    `RequestHeldByDescendant`, exit 0;
  - held briefly: `RequestUnconsumed`;
  - descendant reads / reads late / reads large, background on `/dev/null`:
    `Completed`;
  - the nine earlier cases unchanged.
* **Probe binary at the base** (20 runs per condition): deviations in the
  "descendant reads" family and "held after reading" under at least one
  condition (the read lands on both sides of the decision); the always-held
  silent cases are refused there too, as `RequestUnconsumed`.
* **Existing assertions on Linux: none moves.** The census found no fixture
  with the shape, and every case without another holder decides exactly as
  before, with no added wait.
* **Off Linux** (unexecuted): every successful exit becomes `Io`. The
  existing per-platform assertions in `standalone_host.rs` and
  `cycle20_verifier_plugins.rs` that expect acceptance or
  `RequestUnconsumed` off Linux are then stale; they are not executed
  anywhere and are recorded, not edited (the parallel card owns
  `standalone_host.rs`).
* **Arbiters:** corpus, system and slice, and the null property,
  **byte-identical** (the pinned audit routes launch no process provider).
* **Tests:** +9 probe tests, plus unit tests for the holder check and the
  decision.
* **Lean:** `QCore.RequestConsumption` extended in place (its audit file
  too): **317 audits, unchanged**.
* **Ledger:** `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT` retired (764 →
  763), unless a residual needs registering; the off-Linux entry amended.
