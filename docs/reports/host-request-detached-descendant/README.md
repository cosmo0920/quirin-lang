# HOST-REQUEST-READ-BY-DETACHED-DESCENDANT: only the request pipe's last holder decides

Card of wave 48, base `b565f79ab`; prediction frozen at `7d81fdbac`
([PREDICTION.md](PREDICTION.md)). It changes the verifier host's consumption
decision (`crates/qcore-verifier-host/src/request_consumption.rs`, the
decision branch and the deadline arm of `run_process` in `src/lib.rs`), adds
`ProviderFailure::RequestHeldByDescendant` and
`ProcessOutcome::RequestHeldByDescendant`, nine probe cases in
`tests/request_consumption.rs`, three unit tests, and extends the Lean module
`QCore.RequestConsumption`. No analyser, checker, contract or verdict code
changes.

## 1. The defect

Card HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING made the host decide a
successful exit from consumption: once the direct child has exited and both
output pipes are at end of file, `FIONREAD` on a probe counts what nobody read.
A descendant that inherited the request pipe as its stdin, with both outputs
on `/dev/null`, can still read at that moment. Reads that land before the
decision were credited and later ones were not, so the same provider was
accepted or refused by timing. Neither branch was a wrong Safe (the response
is still checked), but the outcome was nondeterministic. §4 measures it on the
base binary.

## 2. Census

Re-checked over every provider, plugin, package fixture and test that the host
runs through `run_process` ([PREDICTION.md](PREDICTION.md) §2,
[`scripts/census.py`](scripts/census.py)):

| Population | Has the shape? |
|---|---|
| Rust providers (five shipped packages, `sample_boolean_provider`, `configured_audit_provider`, the SDK example checker, the w3 fixture provider) | no: none spawns a process |
| shell fixtures (32 `#!/bin/sh` sources, 11 background-job lines at the base) | no: a non-interactive `sh` gives an asynchronous list `/dev/null` as stdin unless the list redirects it (measured on this host: dash). Each job holds the output pipes (so the decision already waits for it), is waited for, or is explicitly `</dev/null` |

**No real, intended provider has the shape**, so no provider is newly refused
and no decision is owed to the user.

## 3. Mechanism

**The host decides only once it is the request pipe's last holder, waiting at
most until the declared deadline** (`request_consumption::holders`).

* **While the host still writes** it holds no read end (since the previous
  card), so a write that does not break proves that another process reads.
  The decision waits for the write to finish or break; a broken pipe stays
  final (`RequestUnconsumed`), because no reader can appear after it.
* **Once the frame is written** the host holds only its probe. To ask whether
  anyone else holds a read end, it opens a write end through
  `/proc/self/fd/<probe>` (`O_WRONLY | O_NONBLOCK`; for a pipe this never waits
  for a partner), closes the probe and polls the write end with no events.
  `POLLERR` is set exactly when the pipe has no reader. Then no reader can
  appear any more, and `FIONREAD` on that write end is the final unread count
  (on a pipe it counts the same bytes through either end). If a reader
  remains, the host reopens the probe through the write end and closes the
  write end, so a reader waiting for end of file still gets it, and checks
  again after its backoff quantum (1 ms).
* **At the deadline**, a successful exit whose pipe is still held is refused as
  `ProcessOutcome::RequestHeldByDescendant`, spelled
  `request-held-by-descendant: …`. It is distinct from `RequestUnconsumed` (the
  count was never final) and from `WallClockTimeout` (the provider itself had
  finished).
* **Inability to decide refuses.** A failed open, poll or `FIONREAD` is `Io`
  ("request consumption is unobservable"), never acceptance.
* **Nothing changes where no other process holds the pipe.** The first check
  then finds `POLLERR` and decides at once, exactly as before.

Measured directly before any host code ([`scripts/holders_probe.py`](scripts/holders_probe.py),
`private/hdd/holders_probe.out`, exit 0): a live holder is seen; a released
pipe reports 0 of 5 or 5 of 5 unread bytes; a reader blocked waiting for end
of file still gets it and exits after a check. The unit tests
`the_holder_check_sees_another_reader_until_it_is_gone` and
`a_released_pipe_reports_what_no_reader_took` pin the same facts through the
host's own functions.

**Why a bounded wait rather than an immediate refusal.** The brief's default
was to refuse whenever another process holds the pipe at the decision. That
makes the outcome a function of the holders at that moment, but those holders
are themselves timing. A descendant that reads the whole request and exits
would be accepted when its exit lands before the decision and refused after
it. Lean's `an_immediate_decision_would_still_depend_on_the_schedule` shows
this on a two-instant example, and §4's base rows show the count-only half on
real processes. Waiting until the last other holder is gone makes the outcome
the final consumption's, which no later schedule can change. Only a holder that
outlives the deadline is refused, and the deadline already bounds every other
wait of the host (a descendant that holds the output pipes defers the decision
the same way). The wait is strictly better on the card's own criterion. Its
cost is that a provider whose descendant keeps the pipe takes its whole
deadline before the refusal.

**Off Linux** `/dev/fd` is a `dup`: the probe is the read end itself, and no
mechanism was found that tells it from another process's. `holders` therefore
reports the holders unobservable there, and **every successful exit is refused
as `Io`** (fail-closed, as the brief directs). This has never been executed (no
macOS host); the macOS CI job runs no verifier-host process row. The probe
binary asserts the off-Linux outcome per platform. Two `standalone_host` arms
and every other test that expects a process provider to complete are stale off
Linux and were not edited: nothing executes them there, and the parallel card
owns `standalone_host.rs`. Both points are recorded in the ledger (§9).

## 4. Order independence, measured

HDD_CAMPAIGN

## 5. Existing assertions

None moved on Linux. The nine earlier probe cases kept their exact outcomes
(§4), and the census found no fixture whose request pipe another process keeps,
so every other verifier-host, compiler, audit and provider-conformance test is
unchanged (§8). The probe binary was already in the runner's `EXCLUSIVE`
table.

## 6. Lean

`QCore.RequestConsumption` is extended in place; its audit,
`AuditRequestConsumption.lean`, now prints 15 theorems (8 before). A `Run`
gains `held`, whether another process held the pipe at the decision, with the
pipe fact that a broken pipe has no reader as a field, not an axiom. The
observation carries `Holders` (`released unread`, `held`, `unobservable`), and
`decide` refuses a held pipe as `heldByDescendant`.

* `decide_is_a_function_of_consumption_and_holders`: for a successful exit,
  the outcome is `heldByDescendant` when held, otherwise `decoded` exactly
  when `consumed = length`, for every schedule. The earlier
  `decide_is_a_function_of_consumption` is its corollary for `held = false`.
* `the_outcome_does_not_depend_on_the_schedule`: equal length, consumption
  and holders give equal outcomes.
* `a_held_request_is_never_decoded`, `a_non_consuming_provider_is_never_decoded`
  (now also over a held pipe), `a_consuming_provider_is_decoded` (once no
  other process holds it), and the earlier four, restated.
* The wait over time. An `Execution` has a `release` instant after which
  consumption is constant (`settled`, a field). `waitDecide` decides at the
  host's first check after the release if it falls within the deadline, and
  refuses as held otherwise:
  - `the_wait_decides_from_the_final_consumption`;
  - `the_wait_does_not_depend_on_when_the_host_checks`;
  - `a_holder_past_the_deadline_is_refused`;
  - `the_wait_decodes_only_after_the_release`;
  - `an_immediate_decision_would_still_depend_on_the_schedule`: for a
    descendant that reads and exits at instant 2, the count alone gives
    unconsumed at 0 and decoded at 2, refusing a held pipe gives
    held-by-descendant at 0 and decoded at 2, and the wait decodes at every
    check after the release.

Not proved, and measured instead: that `FIONREAD` reports the bytes nobody
read, and that `POLLERR` on the reopened write end reports that no other reader
is left (§3); and that the Rust `classify` and `holders` transcribe `decide`
(their unit tests).

## 7. Arbiter identity

HDD_IDENTITY

## 8. Gate

HDD_GATE

## 9. Prediction against outcome

HDD_PREDICTION

## 10. Follow-ups

* **Retired:** `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`. Its DISCHARGE asks
  the host to refuse or wait for an execution in which another process holds
  the request pipe at the decision, with a test that forces the shape. That
  holds (§3, §4): the host waits for any such process, not only members of the
  provider's group, and refuses one still holding the pipe at the deadline.
* **Registered:** `HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE`. Off Linux
  every successful process-provider exit is now refused as `Io`, because the
  other holders cannot be observed there (§3). DISCHARGE: an off-Linux
  observation (for example a request transport whose peer close the host can
  see), measured on macOS, or a recorded user decision that process providers
  are Linux-only.
* **Amended:** `HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED`. Its text now names
  the off-Linux holder arm and the off-Linux expectations that are stale
  (§3).
* **Boundary, not registered:** a process with ptrace-level access to the
  host could open the host's own pipe descriptors through `/proc/<host>/fd`
  after the release. That is outside the provider model, which is not an OS
  sandbox (rule 26).
