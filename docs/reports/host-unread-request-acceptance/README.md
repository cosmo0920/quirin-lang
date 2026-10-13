# HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING: the host decides from consumption

Card of wave 48, base `e0b92ece6`, rebased onto `1f53f9fa0` (QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN); prediction frozen at `574c1093a` (`50edeb481` before the rebase)
([PREDICTION.md](PREDICTION.md)). It changes the verifier host's request
transport (`crates/qcore-verifier-host/src/lib.rs` `run_process`, the new
`src/request_consumption.rs`), adds `ProcessOutcome::RequestUnconsumed` and
`ProviderFailure::RequestUnconsumed`, a probe test binary
(`tests/request_consumption.rs`), three changed assertions in two existing
tests, one runner `EXCLUSIVE` entry, and the Lean module
`QCore.RequestConsumption`. No analyser, checker, contract or verdict code
changes.

## 1. The defect

`run_process` wrote the request frame into `Stdio::piped()` stdin and
dropped the write end once the frame was sent. It then accepted a successful
exit whatever the provider read. It refused only when a write met a broken
pipe (`VERIFIER-HOST-CLOSED-INPUT-DEADLINE`). So a provider that never read
its request was:

* **accepted** when the frame fit the pipe and the host's write landed before
  the exit;
* **refused** (`Io(Broken pipe)`) when the exit came first, or the frame was
  larger than the pipe.

Neither branch is a wrong Safe, because the proposal is still checked. But
the verdict depended on scheduling. §4 measures both outcomes on the base
binary for the same provider.

## 2. Census

Every executable the host runs through `run_process`, its single spawn site.
The search covered every test, example, fixture and provider crate; the
frozen summary is in [PREDICTION.md](PREDICTION.md) §2.

| Population | Count | Consumes the whole request before a successful exit? |
|---|---:|---|
| Rust providers on `quirin-provider-shared`'s runtime (five shipped packages, `sample_boolean_provider`, `configured_audit_provider`, the SDK example checker) | 8 | yes: `read_frame` reads the header, exactly `size` bytes, then requires end of file. On an installed-manifest mismatch it exits 2 **before** reading: an unsuccessful exit, unaffected |
| w3 fixture provider (`compiler/tests/fixtures/w3/provider`) | 1 | yes: header and exactly the payload |
| shell fixtures that read first (`READ_REQUEST` or `/bin/cat > /dev/null`) | 37 | yes |
| shell fixtures that do not read | 13 | no, and **every one is expected to fail**: 7 exit non-zero or never exit, 3 are killed at the deadline, and 3 exit 0 but are asserted `Timeout` (a descendant holds the output pipes) or `Io` |

**No real, intended provider legitimately ignores its input.** The
conservative default therefore stands, and no decision is owed to the user.
Spawns that bypass `run_process` are out of scope: `--verify-package` /
`--contract-manifest`, the conformance `raw()` frame tests, and subcommand
plugins, which use another protocol.

## 3. Mechanism

The host decides from **consumption**, observed in the request pipe itself
(`src/request_consumption.rs`).

* **The pipe is the host's.** It is created with `std::io::pipe`, and its
  read end becomes the child's stdin. The `Command` is dropped right after
  `spawn`, so the host holds no read end while it writes. A broken pipe
  still means that no reader is left.
* **The probe.** Once the whole frame is written, and before the write end
  is closed, the host takes a probe: a read end it never reads from.
  - On Linux, a second, independent open of the same pipe through
    `/proc/self/fd/<write end>` (`O_RDONLY | O_NONBLOCK | O_CLOEXEC`). Its
    flags belong to its own open file description, never to the child's
    stdin.
  - Elsewhere `/dev/fd` is a `dup`, so the probe is a read end kept from the
    spawn. The pipe cannot break there. That path has not been executed
    (`HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED`).
  - Holding a read end never delays the provider's end of file, which
    depends on writers only.
* **The decision.** It is taken once the direct child has exited and both
  output pipes are at end of file. It no longer waits for the write to
  finish or break.
  1. An unsuccessful exit gives `Terminated` (precedence unchanged).
  2. A broken pipe, a frame not wholly delivered, or `FIONREAD` > 0 on the
     probe gives `RequestUnconsumed`, spelled `request-unconsumed: …`, with
     the counts.
  3. A probe that could not be taken or read gives `Io` ("request
     consumption is unobservable"): fail-closed, never accepted.
  4. Otherwise the response is decoded as before.
* **A broken pipe at the deadline**, with the provider still alive, is
  `RequestUnconsumed` where it was `Io`. The two pinned assertions that a
  broken input is not a timeout keep that property on Linux.
* **A provider that reads its whole frame and exits before the host closes
  its end is accepted.** It can have read the whole frame only after the
  host wrote all of it, so `delivered = length`. It left nothing, so
  `FIONREAD` is 0 whether the probe was taken before or after its exit. The
  probe case `exact reader` reads exactly the frame length and never waits
  for end of file. §4 runs it with the host's close delayed by 30 ms.

On this host the probe was checked directly
([`scripts/reopen_probe.c`](scripts/reopen_probe.c); the unit test
`the_probe_counts_what_no_reader_took` pins the same facts). After a child that read nothing exited, the reopened end reports
`FIONREAD` 5 of 5 bytes, and still does after the writer closes. After a
child that read everything, it reports 0.

**The boundary of the claim.** Consumption is judged at that single moment.
Consider a descendant that inherited the request pipe, closed or redirected
both output pipes, and outlives the direct child. Its reads after the
decision are not credited. For that one shape the outcome can still depend
on timing (`HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`). No provider or
fixture in the census has that shape.

## 4. Order independence, measured

The probe binary `tests/request_consumption.rs` has nine cases. Five do not
consume their request:

* silent, small frame;
* silent, small frame, exiting 50 ms late, so the write lands first;
* `exec 0<&-`;
* a 4-byte `dd`;
* silent, 8 MiB frame.

Four consume it: `cat` to end of file and `head -c <frame length>`, each with
a small and an 8 MiB frame. Every non-consuming fixture writes the *valid*
response, so only its consumption distinguishes it.

The base binary (`d18d4cf8`) is the same file at the base's host. Its
refusing cases assert only "not `Completed`, exit 0"; the tip binary's
assert `RequestUnconsumed`. Both were run with EXECUTION-QUOTA-LOAD-
INDEPENDENCE's harness ([`scripts/loadbisect.py`](scripts/loadbisect.py),
[`scripts/campaign.sh`](scripts/campaign.sh)) under these conditions:

* idle;
* a host stall: the test process alone `SIGSTOP`ped for a random 0..N ms,
  while its children keep running;
* [`scripts/fifo_delay.c`](scripts/fifo_delay.c) preloaded into the measured
  binary only, delaying either the first write to each pipe writer or the
  close of one by 30 ms. The close delay also delays std's `spawn`. Before
  returning, `spawn` closes three pipe write ends in the parent (the child's
  stdout and stderr ends, and the exec-status pipe), so `spawn` returns about
  90 ms late. That moves the host's first write after the exit of even the
  50 ms-late provider;
* both delays together with a 20 ms stall.

Each cell counts runs in which that case **deviated** from its required
outcome: a non-consuming provider accepted, or a consuming one refused. The
per-run JSON is in [artifacts/campaign/](artifacts/campaign/).

| Binary | Condition | Runs | silent small | silent small, late | closed input | partial 4 B | silent large | readers (4 cases) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| base | idle | 20 | **20** | **20** | **18** | **20** | 0 | 0 |
| base | 20 ms host stall | 20 | **20** | **20** | **20** | **20** | 0 | 0 |
| base | 50 ms host stall | 20 | **20** | **20** | **19** | **20** | 0 | 0 |
| base | first write +30 ms | 20 | 0 | **20** | 0 | **20** | 0 | 0 |
| base | close +30 ms | 20 | 0 | 0 | 0 | **20** | 0 | 0 |
| base | both + 20 ms stall | 20 | 0 | 0 | 0 | **20** | 0 | 0 |
| tip | idle | 50 | 0 | 0 | 0 | 0 | 0 | 0 |
| tip | 20 ms host stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 |
| tip | 50 ms host stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 |
| tip | first write +30 ms | 50 | 0 | 0 | 0 | 0 | 0 | 0 |
| tip | close +30 ms | 50 | 0 | 0 | 0 | 0 | 0 | 0 |
| tip | both + 20 ms stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 |

**Base: 120 runs, 317 deviating case-runs. Tip: 300 runs, 0 deviations.**
Every tip run exited 0.

**Rebased tip.** After the rebase onto `1f53f9fa0`, the probe binary was
rebuilt (`1a8dd1e3`) and the tip half of the campaign was rerun under the
same six conditions, because `run_process` had met the other card's change
next to this one: **300 runs, 0 deviations**
([artifacts/campaign/SUMMARY.md](artifacts/campaign/SUMMARY.md), the
`rebased-*.json` files).

On the base, the same non-reading provider went both ways, and the
condition alone decided which:

* The silent small provider was accepted in every idle and stalled run, and
  refused in every run with the write or the spawn delayed.
* The closed-input provider was accepted in 18 of 20 idle runs and refused
  in the other two.
* The 4-byte reader was accepted everywhere: it blocks until the write
  arrives, reads 4 bytes and exits, so its pipe never breaks.
* The large silent provider was always refused, because a frame over the
  pipe capacity always broke the pipe.

On the tip, every non-consuming case is `RequestUnconsumed` and every
consuming case is `Completed`, under every condition. That includes the exact
reader with the host's close delayed, the case that exits before the host
closes its end.

## 5. Existing assertions

Three assertions changed. Before and after, all three are refusals.

| Test | Before | After (Linux) | After (other Unix) |
|---|---|---|---|
| `standalone_host` closed input, live at its 100 ms deadline | `Io(_)` | `RequestUnconsumed(_)` | `Timeout` (no broken pipe there) |
| `standalone_host` closed input, exit 0 with a valid response | `Io(_)` | `RequestUnconsumed(_)`, record outcome `RequestUnconsumed` | same |
| `cycle20_verifier_plugins` `vp04_vp07_vp08` closed input, live at deadline | `Io(_)` | `RequestUnconsumed(_)` | `Timeout` |

No assertion was weakened. Each new match is narrower than `Io(_)`, and the
"not a timeout" property of the two deadline arms holds on Linux. Every other
verifier-host, compiler, audit and provider-conformance test is unchanged
(§8). The probe binary joins the runner's `EXCLUSIVE` table. Its outcomes are
measured to be order-independent, but it still launches process providers
under a declared 20 s timeout, which the membership rule covers.

## 6. Lean

`QCore.RequestConsumption`, audited in `AuditRequestConsumption.lean`, models
`classify` and the decision branch. A `Run` is one schedule: bytes delivered,
bytes consumed, and whether a write broke. The pipe's three guarantees are
fields of the structure, not axioms:

* `consumed ≤ delivered`;
* `delivered ≤ length`;
* a write breaks only while part of the frame is unsent.

Theorems:

* `decide_is_a_function_of_consumption`: for a successful exit, the outcome
  is `decoded` exactly when `consumed = length`, and `requestUnconsumed`
  otherwise, for every schedule.
* `the_outcome_does_not_depend_on_the_schedule`: two runs with the same
  length and consumption get the same outcome.
* `a_non_consuming_provider_is_never_decoded`: whatever the exit, the
  probe's availability and the broken-pipe flag.
* `a_consuming_provider_is_decoded`.
* `a_broken_pipe_is_an_unconsumed_request`.
* `an_unsuccessful_exit_is_terminated`.
* `an_unobservable_consumption_is_refused`.
* `the_legacy_decision_depended_on_the_schedule`: two schedules of one
  non-reading provider. The legacy decision accepted one and refused the
  other; the new decision refuses both.

Not proved, and measured instead: that `FIONREAD` on the probe reports the
bytes no reader took (§3, unit test), and that the Rust `classify` transcribes
`decide` (pinned by `only_a_wholly_delivered_and_wholly_read_frame_is_consumed`
and `an_unobservable_consumption_refuses_and_never_accepts`).

## 7. Arbiter identity

The host change is production code, so all three routes were re-run, each
with the null property selected (`QCORE_NULL_PROPERTY`).

* **BEFORE:** the base release binary `c0a174a4…`, built with the base's
  `lib.rs`.
* **AFTER:** the pre-rebase tip's binary `48e1d29c…`, then the rebased
  tip's binary `a96bebb6…`. The integrator recorded main's arbiters at
  `1f53f9fa0` as byte-identical to `e0b92ece6`'s, so BEFORE stands for both
  comparisons ([artifacts/identity-rebased.txt](artifacts/identity-rebased.txt)).

Each corpus `run.json` records the binary it ran. Scripts:
[`scripts/arbiters.sh`](scripts/arbiters.sh),
[`scripts/measure-binary.sh`](scripts/measure-binary.sh),
[`scripts/identity.py`](scripts/identity.py) and
[`scripts/totals.py`](scripts/totals.py).

| Route | BEFORE = AFTER (custody) | Null property |
|---|---|---|
| corpus | 935 / 9 / 3,890, `bdf5f1c8…` (the brief's) | 1,113 / 1,435 / 2,355 / 18 Unsafe |
| system | 93 / 4 / 351, `e1e06489…` | 84 / 125 / 245 |
| slice | 52 / 1 / 347, 4 Pending, `348ecfba…` | 71 / 103 / 232 |

Every report, sidecar and null-property stream is **byte-identical**. The
corpus `audit.log` and `run.json`, and the invocation records, are identical
once the run directory name, the binary digest and the times are removed
([artifacts/identity.txt](artifacts/identity.txt),
[artifacts/totals.txt](artifacts/totals.txt)). The system and slice digests
differ from the brief's with equal totals, because those reports embed the
measuring worktree's paths, as earlier cards recorded. No verdict moved, so
no correction is named. No real provider is newly refused: the pinned audit
routes launch no process provider, and the census found none that would be
refused (§2).

## 8. Gate

Every code step ran on the rebased tip `42c821f86` with
[`scripts/gate.sh`](scripts/gate.sh) `g2`, one step at a time under the
shared heavy lock and with nothing running beside it. Exits are in
[gate/](gate/), logs in `private/hur/gate-g2/`, and the summary in
[gate.json](gate.json).

| Step | Exit | Count |
|---|---:|---|
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 488 units, 501 summaries; **7,956 passed, 0 failed**, 14 ignored; `private/hur/tests-g2/` |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **317** audits |
| `providers/package.py build`, then `verify qcore.libevent` | 0 / 0 | |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | 129 passed, 0 failed |
| `python3 -m unittest discover -s providers/tests` | 0 | 25 OK |
| `python3 -m unittest discover -s scripts/tests`, `build_docs_index.py --check` | see [gate/](gate/) | 08, 09 |
| `build_matrix.py . --gate gate.json`, then the Python suite and the index again (`finish-gate.sh`) | see [gate/](gate/) | 09a, 09b, 10b, 10c |

The test step passed on its first run, and no row was rerun in isolation.
The suite count is main's 7,943 plus 13: the nine probe tests and four unit
tests in `request_consumption`.

`providers/` is unchanged. The provider gate was still run because every
real provider package passes through the changed decision. Clang was recorded
in this worktree (`gate/clang-version.txt`).

Before the rebase, the same gate (`g1`, at `b5b245177` on base `e0b92ece6`)
was also green on every step: 7,952 passed / 0 failed, 317 audits, provider
gate 129 / 0. Its logs are in `private/hur/gate-g1/`.

## 9. Prediction against outcome

| Predicted | Outcome |
|---|---|
| census: no intended provider ignores its request | held (§2) |
| probe: five `RequestUnconsumed`, four `Completed`, 0 deviations in 50 runs per condition | held: 300 runs, 0 deviations, and again 300 / 0 on the rebased tip; base 317 deviating case-runs in 120 runs (§4) |
| three existing assertions move from `Io` to `RequestUnconsumed`, nothing else | held (§5) |
| corpus, system, slice and null property byte-identical | held (§7) |
| test count +9 plus unit tests | +13: 9 probe tests and 4 unit tests (main 7,943 → 7,956 after the rebase) |
| Lean 316 → 317 audits | held (§8) |
| not foreseen | the close shim also delays std's `spawn` (three pipe write ends closed in the parent), so it moves the write order too (§4) |

## 10. Follow-ups

* **Retired:** `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`. Its
  DISCHARGE asks for the host to decide from consumption, not write order,
  with Lean parity because the classification changed. That holds: §3, §4
  and §6.
* **Registered:**
  - `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`. A descendant that keeps the
    request pipe with both output pipes closed can still read after the
    decision (§3). It was found by reading; no provider has the shape.
  - `HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED`. The off-Linux probe and its
    `Timeout` assertions have never run.
  - `PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE`. The census found six
    test files outside the host crate whose shell fixtures are still written
    with `fs::write`, the ETXTBSY shape EXECUTION-QUOTA-LOAD-INDEPENDENCE
    repaired inside it.
* **Not taken:** `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN` ran as its own card
  and landed first (`1f53f9fa0`). This change does not touch
  `quota_descriptor` or the quota path; it touches only the request pipe and
  the decision in `run_process`. The rebase met one adjacent line: that
  card's `observed.spawned = true` right after `spawn`, beside this card's
  `drop(command)`. Both are kept, in that order.
