# W5 standalone first-party packaging

Five real executable packages now build independently of compiler/provider
internals under `providers/`: Libevent, ChunkIO, CMetrics, CTraces and
librdkafka. Each delivered crate links only the published schema crate and
serialization libraries. A separate conformance harness links the real
verifier host; the core workspace excludes the entire provider workspace.

This is the **Unknown-only packaging boundary**, not semantic analyzer
extraction. The existing first-party v1 registry adapters already returned
Unknown. Their exact response, diagnostic and no-certificate behavior is
preserved by these executables. The specialized source/model routes are
unchanged and retained. Evidence-v2 requests now receive an explicit
Unknown/no-witness response for every declared capability. No protocol
version, evidence format, certificate checker or Safe-admission rule changes.
Full W5 semantic extraction and aggregate Cycle 20D distribution/acceptance
remain open.

## Evidence

The process suite builds all five artifacts, compares their routing metadata
to the on-disk first-party manifests, and dispatches them through
`ProcessProvider` and `VerifierRegistry`. Its machine-readable parity rows are
archived here: 33 exact v1 comparisons across 11 capability routes (empty,
false and true assertion inventories), plus v2 Unknown results on all 11
routes using nonempty core task envelopes. No route produces a certificate.

Negative cases exercise plugin/request/source identities, protocol/evidence
versions, unsupported and nonconcrete selectors, capabilities, invalid task
identities, unknown fields, oversized/truncated/empty/multiple frames, and
installed package version/profile/digest/evidence-version substitution.
Appending a byte to an otherwise runnable Linux ELF binary triggers the
provider's own executable-digest check. These cases return no response or
cause the host to produce Unknown. The existing host v1 refusal tests remain
unchanged.

Two independent clean release builds use different target directories. All
five executable, manifest and deterministic archive hashes match. Build
records retain the Rust target/toolchain and dependency lock hash. Artifacts
are produced outside the repository; binary files are not committed.
`providers/package.py` recreates them and verifies each installed package.
The provider README documents versioned installation, upgrade, rollback and
offline invocation through the existing registry API. Default in-process
registry selection is unchanged.

A separate copied source tree containing the host and its dependency closure,
with no `providers/` directory or compiler/provider adapters, passes the host
suite. Dependency metadata checks reject any artifact crate dependency beyond
schemas and serialization. This does not claim an OS sandbox, new runtime
memory-limit enforcement, signatures or arbitrary source safety.

## Gates and scope

The archived standing root workspace tests, formatting, Clippy, both reevaluation
probes and Lean build pass on the original `6a66f99` baseline. The separate provider workspace tests, formatting
and Clippy also pass, as does package verification. Detailed counts and
artifact hashes are in `validation.json` and the archived logs. Rust debug
information and incremental builds were disabled to fit the shared build
budget; release package settings are fixed by the packaging tool.

No pinned Cycle 19 route is deleted or promoted. Existing-C analyzer source,
first-party semantic adapters, and the Lean model are unchanged by this
increment. The Lean gate verifies the existing firewall model; this package
does not claim a Rust/process/formal refinement theorem or completed semantic
provider extraction.

The provider suite, Clippy, package verification and two fresh clean builds were
rerun against `51bbfc0` and schemas 0.2.0; final package hashes in this report
refer to that run. An attempted separate root rebuild hit filesystem quota;
its partial output is not claimed as a passing gate. The combined-main gate
after merge covers the integrated schema/kernel changes. The provider-free
host snapshot remains evidence for the original baseline, not a fresh run.
