# HOST-REQUEST-READ-BY-DETACHED-DESCENDANT: only the request pipe's last holder decides

Card of wave 48, base `b565f79ab`, rebased onto `5ee141946`
(PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE); prediction frozen at
`7d81fdbac` ([PREDICTION.md](PREDICTION.md)). Commits are named as they were
when measured; after the rebase, `7d81fdbac` is `40eb00a03`, `2a1414662` is
`1af047aa5` and `6c28b19d5` is `203cc034a`. It changes the verifier host's consumption
decision (`crates/qcore-verifier-host/src/request_consumption.rs`, the
decision branch and the deadline arm of `run_process` in `src/lib.rs`), adds
`ProviderFailure::RequestHeldByDescendant` and
`ProcessOutcome::RequestHeldByDescendant`, nine probe cases in
`tests/request_consumption.rs`, four unit tests, and extends the Lean module
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
mechanism was found that tells it from another process's.

* **First version (`2a1414662`).** As the brief first directed, `holders`
  reported the holders unobservable there, and every successful exit was
  refused as `Io`.
* **The user's decision ("keep previous macOS") replaced that.** Off Linux the
  host now decides **exactly as at the base**. `holders` returns
  `Holders::CountOnly`: while the host is still writing, an undelivered
  remainder is unconsumed; otherwise `FIONREAD` on the kept read end is the
  count, and a missing or unreadable probe still refuses as `Io`, with the
  base's text. The wait and the `RequestHeldByDescendant` refusal are
  Linux-only (`held` is never set off Linux).
* **The residual off Linux.** A descendant that reads on its own schedule is
  still accepted or refused by timing there, as at the base
  (`HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE`). That is the only shape the
  off-Linux decision leaves open.

Off Linux, every existing assertion holds again as at the base, including
`standalone_host`'s and `cycle20_verifier_plugins`' per-platform arms. The
probe binary's new cases assert what the off-Linux host decides:

* `RequestUnconsumed` when nothing reads the request;
* either outcome for a descendant that reads on its own schedule;
* the `setsid -f` case is Linux-only, because `setsid` is util-linux's.

None of this has been executed: there is no macOS host, and the macOS CI job
runs no verifier-host process row (`HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED`).

## 4. Order independence, measured

The probe binary `tests/request_consumption.rs` now has eighteen cases: the
earlier card's nine and this card's nine. In each new case a background
subshell holds the request pipe as its stdin, with stdout and stderr on
`/dev/null`, and the direct child answers with the valid response without
reading:

* **held silent**: the descendant holds the pipe and never reads (`sleep 30`,
  killed with the group after the decision);
* **held detached**: the same after `setsid -f`, outside the group (it ends
  on its own after 3 s);
* **held large**: the same with an 8 MiB frame, so the host is still writing;
* **held after reading**: the descendant reads to end of file, then keeps the
  pipe;
* **held briefly**: the descendant holds the pipe 50 ms and never reads;
* **descendant reads**, **reads late** (after 50 ms) and **reads large**
  (8 MiB): the descendant reads to end of file and exits;
* **background on `/dev/null`**: the direct child reads, and a background job
  with stdin from `/dev/null` and outputs closed outlives it.

The four "held" cases run under a 1,000 ms deadline; the others keep the
binary's 20 s. At the tip they must give `RequestHeldByDescendant` (exit 0,
refused no earlier than the deadline), `RequestUnconsumed`, `Completed`
(three readers) and `Completed` respectively.

The base binary `6c3cd4d7` is built from the base's host with the same probe
file in its base-compatible form (a refusal asserts only "not `Completed`,
exit 0"). The tip binary `dbc9d3f2` is built at `2a1414662`. Both ran under
the previous card's harness ([`scripts/campaign.sh`](scripts/campaign.sh),
[`scripts/loadbisect.py`](scripts/loadbisect.py),
[`scripts/fifo_delay.c`](scripts/fifo_delay.c)), each side under the shared
heavy lock: idle; a host stall (the test process alone `SIGSTOP`ped for a
random 0..N ms while its children run); the first write to each pipe writer
delayed 30 ms; the close of each pipe write end delayed 30 ms (this also delays
std's `spawn` by about 90 ms and each holder check's transient write end); and
both delays with a 20 ms stall. A cell counts the runs in which that case
deviated from its required tip outcome. The per-run JSON is in
[artifacts/campaign/](artifacts/campaign/).

| Binary | Condition | Runs | held silent | held detached | held large | held after reading | held briefly | descendant reads | reads late | reads large | background /dev/null | earlier nine |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| base `6c3cd4d7` | idle | 20 | 0 | 0 | 0 | 13 | 0 | 4 | 20 | 0 | 0 | 0 |
| base `6c3cd4d7` | 20 ms host stall | 20 | 0 | 0 | 0 | 17 | 0 | 0 | 20 | 0 | 0 | 0 |
| base `6c3cd4d7` | 50 ms host stall | 20 | 0 | 0 | 0 | 18 | 0 | 0 | 20 | 0 | 0 | 0 |
| base `6c3cd4d7` | first write +30 ms | 20 | 0 | 0 | 0 | 19 | 0 | 3 | 20 | 0 | 0 | 0 |
| base `6c3cd4d7` | close +30 ms | 20 | 0 | 0 | 0 | 20 | 0 | 0 | 0 | 0 | 0 | 0 |
| base `6c3cd4d7` | both delays + 20 ms stall | 20 | 0 | 0 | 0 | 20 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | idle | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | 20 ms host stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | 50 ms host stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | first write +30 ms | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | close +30 ms | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | both delays + 20 ms stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

**Base: 120 runs, 194 deviating case-runs. Tip: 300 runs, 0 deviations**,
every run exit 0, the earlier nine included.

On the base, the shape went both ways, and the condition decided which:

* **held after reading** was accepted in 13 to 20 of 20 runs per condition and
  refused in the rest: the descendant's read landed on either side of the
  decision;
* **descendant reads** was refused in 4 of 20 idle runs and 3 of 20 with the
  write delayed, and accepted otherwise;
* **reads late** was refused in every run without the close delay and
  accepted in every run with it (the delayed close moves the decision past the
  descendant's read);
* **reads large** was accepted in every base run: writing an 8 MiB response
  takes the direct child longer than the descendant needs to read 8 MiB, so
  the outputs reached end of file after the read. That is the base's timing,
  not a property of it;
* the always-held and never-reading cases were refused on the base too (as
  `RequestUnconsumed`), which their base-compatible assertion accepts.

On the tip every case gives its one required outcome under every condition.

**After the off-Linux change** (`6c28b19d5`, §3), the probe binary was rebuilt
(`8cbaabb0`), and a shorter confirmation run checked that the Linux path is
unchanged: 20 runs under each of the six conditions, **120 runs, 0
deviations** (the `after2-*.json` files and
[SUMMARY.md](artifacts/campaign/SUMMARY.md)). The change touches only
`cfg(not(target_os = "linux"))` code and the probe's off-Linux expectations.

## 5. Existing assertions

None moved on Linux. The nine earlier probe cases kept their exact outcomes
(§4), and the census found no fixture whose request pipe another process keeps,
so every other verifier-host, compiler, audit and provider-conformance test is
unchanged (§8). Off Linux the decision is the base's, so every existing
per-platform assertion holds there as before (§3). The probe binary was already
in the runner's `EXCLUSIVE` table.

## 6. Lean

`QCore.RequestConsumption` is extended in place; its audit,
`AuditRequestConsumption.lean`, now prints 17 theorems (8 before). A `Run`
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
* **Platform split.** Everything above is the Linux host: `observe` yields
  only `held` or `released`, and the wait theorems describe the Linux loop.
  The off-Linux decision gets its own `Holders` constructor, `countOnly`,
  which `decide` treats as the pre-card count. Two theorems state it:
  - `off_linux_decides_on_the_count_at_the_decision`: off Linux, the decision
    at instant `t` equals the pre-card `countDecide` at `t`, whoever holds the
    pipe;
  - `off_linux_a_late_reader_still_depends_on_the_schedule`: the residual, on
    the same late reader, refused at 0 and decoded at 2.

  `a_non_consuming_provider_is_never_decoded` also covers `countOnly`.

Not proved, and measured instead: that `FIONREAD` reports the bytes nobody
read, and that `POLLERR` on the reopened write end reports that no other reader
is left (§3); and that the Rust `classify` and `holders` transcribe `decide`
(their unit tests).

## 7. Arbiter identity

The host change is production code, so all three routes were re-run with the
null property selected (`QCORE_NULL_PROPERTY`) and no parity stream:
**BEFORE** the base release binary `d52529cf…` (`b565f79ab`), **AFTER** the
tip's `39a39d66…` (`2a1414662`). Each corpus `run.json` records the binary it
ran. Scripts: [`scripts/build-measure.sh`](scripts/build-measure.sh),
[`scripts/measure-binary.sh`](scripts/measure-binary.sh),
[`scripts/measure.sh`](scripts/measure.sh),
[`scripts/identity.py`](scripts/identity.py),
[`scripts/totals.py`](scripts/totals.py).

| Route | BEFORE = AFTER (custody) | Null property | Wall time BEFORE / AFTER |
|---|---|---|---|
| corpus | 935 / 9 / 3,890, `bdf5f1c8…` | 1,113 / 1,435 / 2,355 / 18 Unsafe | 487 s / 477 s |
| system | 93 / 4 / 351, `fba67e1e…` | 84 / 125 / 245 | 69 s / 69 s |
| slice | 52 / 1 / 347, 4 Pending, `a2e54b77…` | 71 / 103 / 232 | 36 s / 36 s |

Every report, sidecar and null-property stream is **byte-identical**. The
corpus `audit.log` and `run.json`, and the invocation records, are identical
once the run directory name, the binary digest and the times are removed
([artifacts/identity.txt](artifacts/identity.txt),
[artifacts/totals.txt](artifacts/totals.txt)). The totals equal the brief's;
the corpus digest is the brief's, and the system and slice digests differ from
it with equal totals because those reports embed the measuring worktree's
paths, as earlier cards recorded. No verdict moved, so no correction is named.
No real provider is newly refused: the pinned audit routes launch no process
provider, and the census found none with the shape (§2).

The arbiters were not re-run after the rebase. The two changes under the
rebased tip were each measured byte-identical against `b565f79ab`: this
card's in this section, and PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE's
(three behaviour-neutral clippy borrow removals, besides test code) in its own
report. The off-Linux change after these measurements changes only code
compiled off Linux, plus a `Holders` variant that the Linux build never
constructs, and the Linux decision path is identical.

## 8. Gate

**Gate g2, the record.** The card was rebased onto `5ee141946`
(PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE, which also carries the three
clippy 1.99 fixes). The whole gate then ran at `ece1dede0`, whose code is the
final tip's, with [`scripts/gate.sh`](scripts/gate.sh) `g2 all` under **one
exclusive hold** of the shared heavy lock (`HELD_LOCK=1`; no other gate or
build ran beside it). Exits are in [gate/](gate/), logs in
`private/hdd/gate-g2/`, and the summary in [gate.json](gate.json). Every step
ran under rustc and clippy 1.99.0.

| Step | Exit | Count |
|---|---:|---|
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 488 units, 501 summaries; **7,969 passed, 0 failed**, 14 ignored; `private/hdd/tests-g2/` |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **317** audits |
| `providers/package.py build`, then `verify qcore.libevent` | 0 / 0 | |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | 129 passed, 0 failed |
| `python3 -m unittest discover -s providers/tests` | 0 | 25 OK |
| `python3 -m unittest discover -s scripts/tests` | 0 | 365 OK |
| `python3 scripts/build_docs_index.py --check` | 0 | |
| `build_matrix.py . --gate gate.json`, then the Python suite and the docs index again ([`scripts/finish-gate.sh`](scripts/finish-gate.sh)) | see [gate/](gate/) | 09a, 09b, 10b, 10c |

The test step passed on its first run, and no row was rerun. The count is
main's 7,956 plus the card's thirteen tests: the nine probe cases and four
unit tests in `request_consumption`.

`providers/` is unchanged. The provider gate was still run, because every real
provider package passes through the changed decision. Clang was recorded in
this worktree (`gate/clang-version.txt`). The ledger edit keeps
`dimensions.named_followups` equal to the list length (764).

**Gate g1, superseded.** The first gate ran at `277ad447d` on base
`b565f79ab`, on the card's first version (off Linux every successful exit
refused as `Io`). Its exits are kept in [gate/g1/](gate/g1/), with logs in
`private/hdd/gate-g1/`. It gave 7,968 passed and 0 failed, 317 audits, a green
provider gate and 365 Python OK. It was incomplete: the host's stable
toolchain became rustc and clippy 1.99.0 at 02:48 (+0900) on 2026-10-14, while
the card ran, and its `needless_borrows_for_generic_args` lint failed clippy at
`crates/qcore-verification/src/source_imports.rs:186`, a line this card does
not touch. With only that lint allowed, clippy exited 0 over the whole
workspace. The fix landed with PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE.

## 9. Prediction against outcome

| Predicted | Outcome |
|---|---|
| census: no provider or fixture has the shape | held (§2) |
| tip: every new case its one outcome, 0 deviations in 50 runs per condition | held: 300 runs, 0 deviations (§4) |
| base: deviations in the "descendant reads" family and "held after reading" | held for "held after reading", "descendant reads" and "reads late"; **not** for "reads large", accepted in every base run (§4 explains the base's timing there) |
| base: the always-held silent cases refused there too, as `RequestUnconsumed` | held (base-compatible assertion; no deviation) |
| no existing Linux assertion moves | held (§5, §8) |
| off Linux every successful exit becomes `Io`; stale off-Linux expectations recorded, not edited | **superseded by the user's decision** ("keep previous macOS"): off Linux the host decides exactly as at the base, no off-Linux assertion is stale, and the detached-descendant shape stays timing-dependent there; never executed (§3, §10) |
| corpus, system, slice and null property byte-identical | held (§7) |
| tests: +9 probe tests plus unit tests | +13: 9 probe tests and 4 unit tests (§8) |
| Lean: module extended in place, 317 audits | held: 8 → 17 printed theorems, 317 audits (§6, §8) |
| ledger: entry retired, off-Linux entry amended | held, and the off-Linux residual registered as its own entry: 764 → 764 (§10) |

## 10. Follow-ups

* **Retired:** `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`. Its DISCHARGE asks
  the host to refuse or wait for an execution in which another process holds
  the request pipe at the decision, with a test that forces the shape. That
  holds on Linux (§3, §4): the host waits for any such process, not only
  members of the provider's group, and refuses one still holding the pipe at
  the deadline. Off Linux the residual is registered separately.
* **Registered:** `HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE`. Off Linux the
  other holders cannot be observed, and by the user's decision the host
  decides as at the base, so a descendant that keeps the request pipe with both
  outputs closed and reads on its own schedule still gets an outcome that
  depends on timing there (§3). DISCHARGE: an off-Linux holder check (for
  example a socket-pair request transport, whose peer close the host can
  observe), measured on a macOS host.
* **Amended:** `HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED`. Its text now says
  that the off-Linux arm is unchanged and that the probe binary's new
  off-Linux expectations are unexecuted too (§3).
* **Boundary, not registered:** a process with ptrace-level access to the
  host could open the host's own pipe descriptors through `/proc/<host>/fd`
  after the release. That is outside the provider model, which is not an OS
  sandbox (rule 26).
