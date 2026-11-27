# HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING: prediction

Frozen before any implementation code. Base `e0b92ece6` (local `main`). Only
the report's scripts, the probe test file
`crates/qcore-verifier-host/tests/request_consumption.rs` (base-compatible
assertions) and this file exist at the freeze.

## 1. The defect, reproduced on the base

`ProcessProvider::run_process` (`crates/qcore-verifier-host/src/lib.rs`)
writes the frame nonblockingly into `Stdio::piped()` stdin and drops the
write end once `sent == frame.len()`. A successful exit is then accepted
whatever the provider read. If the provider has no reader left when the host
writes, `write` returns `EPIPE`, `input_failure` is set, and the successful
exit is refused (`Io("Broken pipe")`). So a provider that never reads is:

* accepted when the frame fits the pipe and is written before the exit;
* refused when the exit comes first, or the frame is larger than the pipe.

Base probe binary (`private/hur/bin-base/request_consumption`, built from the
probe test file at the base), one run each:

| Condition | silent small | silent small, late exit | closed input | partial read (4 B) | silent large | readers (4 cases) |
|---|---|---|---|---|---|---|
| idle | **accepted** | **accepted** | **accepted** | **accepted** | refused | accepted |
| first pipe write delayed 30 ms (`fifo_delay.so`) | refused | **accepted** | refused | **accepted** | refused | accepted |

The silent small and closed-input rows flip with the write order alone. That
is the ledger's defect.

## 2. Census (frozen before implementation)

Every executable the host runs through `run_process` (its single spawn
site), found by searching every test, example, fixture and provider crate:

* **Rust providers, 9 distinct.** The five shipped packages
  (`qcore-provider-{libevent,chunkio,cmetrics,ctraces,librdkafka}`),
  `sample_boolean_provider`, `configured_audit_provider` and the SDK's example
  checker all use `providers/quirin-provider-shared/src/runtime.rs`
  `read_frame`: the 4-byte header, exactly `size` bytes, then one more read
  that must return end of file. They consume the whole request before a
  successful exit. The w3 fixture (`compiler/tests/fixtures/w3/provider`)
  reads the header and exactly the payload: the whole frame, no end-of-file
  check. The runtime exits 2 **before** reading when its installed manifest
  does not match (`providers/conformance/tests/packages.rs:553`): an
  unsuccessful exit, unaffected.
* **Shell fixtures spawned, 50 bodies.** 37 read the whole request first
  (`READ_REQUEST` in `confined_process_execution`, `enforced_process_limits`,
  `execution_quota`; `/bin/cat > /dev/null` in `package_identity`,
  `standalone_host`, `cycle20_verifier_plugins`, `reevaluation_followup`,
  `user_contract_plugins`). 13 do not, and **every one is expected to
  fail**: seven exit non-zero or never exit, three are killed at their
  deadline, and three exit 0 without reading, asserted `Timeout` (a
  descendant holds the output pipes: `standalone_host.rs:171`,
  `cycle20_verifier_plugins.rs:763`) or `Io` (`standalone_host.rs:224`).
* **Accepted without a full read: none.** No real, intended provider
  legitimately ignores its input, so the conservative default stands and no
  user decision is owed.
* Out of scope (not `run_process`): `--verify-package` /
  `--contract-manifest` runs (`providers/package.py`, conformance
  `Command::output`), the conformance `raw()` frame tests, and subcommand
  plugins (a different protocol).

Predicted changes to existing assertions, all refusals before and after:
`standalone_host.rs:179` and `:224` and `cycle20_verifier_plugins.rs:786`
change from `Io(_)` to `RequestUnconsumed` on Linux. Nothing else moves.

## 3. Mechanism chosen

**The host decides from consumption, observed in the request pipe itself.**

* The request pipe is created by the host (`std::io::pipe`), its read end is
  the child's stdin, and the `Command` is dropped right after `spawn`, so the
  host holds no reader while writing. `EPIPE` therefore still means "no reader
  is left", exactly as today.
* **Linux:** once the whole frame is written, and before the write end is
  closed, the host opens a second, independent read end on the same pipe
  through `/proc/self/fd/<write end>` (`O_RDONLY | O_NONBLOCK | O_CLOEXEC`).
  It never reads from it. Opening it does not delay the provider's end of
  file, which depends on writers only. Measured on this host
  (`private/hur/probe/reopen.c`): after a child that read nothing exited, the
  reopened end reports `FIONREAD` 5 of 5 bytes; after a child that read
  everything, 0.
* **Other Unix:** no such reopen exists (`/dev/fd` is `dup` there), so the
  host keeps a duplicate of the read end from the spawn. `EPIPE` cannot occur
  there; a provider alive at its deadline is a `Timeout`.
* **Decision**, once the direct child has exited and both output pipes are at
  end of file:
  1. unsuccessful exit: `Terminated` (precedence unchanged);
  2. the frame not wholly written, or `EPIPE` seen, or `FIONREAD` on the probe
     > 0: refused, **`ProcessOutcome::RequestUnconsumed`** /
     `ProviderFailure::RequestUnconsumed`, spelled `request-unconsumed`;
  3. the probe could not be opened or queried: refused as `Io` (consumption
     unobservable, fail-closed);
  4. otherwise the response is decoded as today.
* **At the deadline** with a live child that broke the pipe (Linux): the
  refusal is `RequestUnconsumed`, as `Io` was before (the two pinned
  "broken input is not a timeout" arms keep that property under the new
  name).
* **A provider that reads its whole frame and exits before the host closes
  its end** is accepted: it can have read the whole frame only after the
  host wrote it all, so `sent == len`, and it left nothing, so `FIONREAD` is 0
  whether the probe was opened before or after its exit.

Bytes consumed is `delivered - unread`; the host's test
`delivered = len ∧ unread = 0` is equivalent to `consumed = len` for every
schedule, because `consumed ≤ delivered ≤ len`. That is the Lean claim.

## 4. Predicted outcomes

* Probe binary after the change: the five refusing cases give
  `RequestUnconsumed` and the four reading cases `Completed`, **in every run**
  under idle, host stalls of 20 and 50 ms, a 30 ms first-write delay and a
  30 ms close delay: 50 runs per condition, **0 deviations**. The base binary
  deviates under the same conditions (§1).
* Existing tests: `standalone_host` and `cycle20_verifier_plugins` closed-input
  arms change from `Io(_)` to `RequestUnconsumed` (the arm that asserts "not a
  timeout" keeps that); every other verifier-host, compiler, audit and
  provider-conformance test is unchanged, because every real provider and
  every fixture the census found reads its whole request (§2).
* No real provider is newly refused (census); if one is, it is a named
  correction, read by hand.
* Corpus, system, slice and null-property arbiters: **byte-identical** (the
  pinned audit routes do not launch process providers; the binary changes
  only in the verifier host).
* Test count: +9 (the probe binary) plus any added unit test.
* Lean: one new module and audit (`QCore.RequestConsumption`); 316 → 317
  audits.
