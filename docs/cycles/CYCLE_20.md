# Cycle 20: Verifier Plugin Ecosystem

**Status: Active — 20A, the generic 20B firewall, and the initial 20E model are implemented**

Cycle 20 extracts library- and framework-specific verification from the core
compiler into a versioned verifier-plugin ecosystem. The initial providers are
Libevent, ChunkIO, librdkafka, CMetrics, and CTraces. Cycle 19 continues to own
the concrete semantics and pinned evidence from which those providers are
extracted; planning Cycle 20 does not weaken or postpone Cycle 19's safety
gates.

This is a separate milestone because it changes architecture, trust,
distribution, compatibility, and composition simultaneously. A Rust trait
alone is not a stable third-party plugin ABI, and an unchecked plugin verdict
must never be able to mint a QCore-C `Safe` certificate.

## Architectural boundary

The core owns:

- preprocessing and compilation-database normalization;
- source/build identity and digest checks;
- the versioned request, diagnostic, evidence, and certificate schemas;
- fail-closed result aggregation;
- independent certificate checking and Lean theorem bindings;
- registry policy, process isolation, resource limits, and deterministic
  execution; and
- cross-plugin composition.

A verifier plugin owns:

- its recognized API/profile table;
- library-specific contracts, state machines, and diagnostics;
- accepted source and build revisions;
- foreign callback, ownership, concurrency, and observation assumptions;
- conversion from normalized C facts into core-checkable evidence; and
- a manifest declaring every capability and trusted dependency.

Plugins may return `Safe`, `Unsafe`, or `Unknown`, but only core-checked
evidence can promote a plugin's proposed `Safe` result into a certificate.
Protocol errors, crashes, timeouts, version mismatches, undeclared
capabilities, and incomplete evidence are `Unknown`.

## Protocol and packaging

Use a versioned, process-isolated protocol rather than a native Rust dynamic
library ABI. Each package contains a manifest plus an executable provider. The
initial transport is deterministic length-delimited JSON over standard I/O;
the schema version is negotiated before any source or evidence is accepted.
Network access is disabled by default.

The manifest records:

- stable plugin ID, package version, and protocol versions;
- executable path and package digest;
- supported target libraries, revisions, dialects, platforms, and backends;
- capabilities such as resource contracts, event traces, chunk lifecycle,
  concurrency, or translation-unit projection;
- required foreign summaries and observation hooks;
- declared filesystem inputs and maximum time/memory/output budgets; and
- trust class, certificate checker, and Lean theorem family.

Requests carry normalized translation-unit/build identity, the requested
capability, target profile, bounded source facts, and explicit assumptions.
Responses carry a verdict, stable diagnostics, proposed evidence, coverage,
assumptions actually used, and reproducibility metadata.

## Ordered workstreams

### 20A: Core protocol and registry

Define canonical schemas, stable diagnostic namespaces, manifest validation,
plugin discovery, deterministic invocation, timeout/crash handling, and an
in-process adapter used by built-ins during migration. Add compatibility tests
for older/newer protocol and evidence versions.

### 20B: Certificate firewall and composition

Implement per-capability core checkers. A provider may construct evidence but
cannot bypass identity, lifecycle, resource, concurrency, or observation
checks. Define composition rules for multiple plugins claiming the same
translation unit, nested libraries, and shared resources. Conflicting claims
or incomparable assumptions are `Unknown`.

### 20C: Provider extraction

Extract providers in dependency order:

1. `qcore.libevent`, including profile classification and observation
   refinement;
2. `qcore.chunkio`, including nested teardown and delete-only post-states;
3. `qcore.cmetrics` and `qcore.ctraces`, sharing only core schema types rather
   than a mutable global contract list;
4. `qcore.librdkafka`, including refcount/queue/concurrency capabilities; and
5. a Fluent Bit composition profile that combines providers without absorbing
   their implementation.

Each extraction must reproduce the previous pinned verdict and coverage table
before the hard-coded path is removed.

### 20D: Tooling and distribution

Route CLI and audit examples through the registry. Add commands to list,
inspect, enable, disable, and verify installed providers and their permissions.
Package the first-party providers reproducibly and document installation,
upgrade, rollback, and offline audit procedures.

### 20E: Formal model

Model plugin evidence as an untrusted proposal checked by a trusted core
relation. Prove that registry dispatch and compatible composition preserve the
existing capability theorems, and that malformed, missing, or contradictory
plugin output cannot produce a `Safe` certificate.

## Acceptance gates

Cycle 20 is complete only when:

- protocol and manifest schemas are versioned, canonical, and covered by
  compatibility and malformed-input corpora;
- providers execute out of process with bounded resources and no implicit
  network access;
- every proposed `Safe` result is independently checked by the core;
- Libevent, ChunkIO, CMetrics, CTraces, and librdkafka have separate manifests,
  profile tables, tests, and stable diagnostic namespaces;
- the Fluent Bit composition profile reproduces Cycle 19's pinned source
  identity, verdicts, coverage, assumptions, and observation evidence;
- plugin absence, failure, timeout, incompatibility, or conflicting evidence
  produces `Unknown` rather than falling back to optimistic built-in behavior;
- the old monolithic Cycle 19 contract/audit routing is removed only after
  parity is recorded;
- the plugin composition Lean audit contains no placeholders or project
  axioms; and
- Rust tests, formatting, Clippy, Lean builds, protocol conformance tests, and
  reproducible provider-package audits all pass.

## Implementation record

The first Cycle 20 implementation adds `compiler/src/verifier_plugin.rs` with:

- canonical serde-backed protocol v1 and evidence v1 structures framed as a
  four-byte big-endian length followed by JSON;
- strict manifests, target/capability declarations, deterministic registry
  ordering, deterministic `verifier-plugin.json` package discovery,
  enable/disable policy, and a bounded process provider with cleared
  environment, timeout, and output limits;
- fail-closed handling for absence, disablement, version mismatch, process
  failure, malformed envelopes, and rejected evidence;
- a core certificate firewall that independently matches source/build
  identity, full fact coverage, claims, assumptions, provider version, package
  digest, and deterministic replay metadata;
- conflict-aware multi-provider composition; and
- separate first-party built-in adapters for `qcore.libevent`,
  `qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces`, and
  `qcore.librdkafka`. Built-ins traverse the same request, proposal, and core
  checking path and are not allowed to construct `CoreCertificate` values.

The revision 0.2 parser accepts declarations such as (configuration validation only):

```qcore
verify scope fluent_bit {
  plugin qcore.libevent version "0.1" profile linux_epoll
    capabilities { event_traces, resource_contracts };
  plugin qcore.chunkio profile default capabilities { chunk_lifecycle };
}
```

`resolve_source_scopes` validates these declarations against a registry.
`compile_files_with_registry` and `compile_files` reject resolved scopes with
`Q701` until core obligation generation and dispatch exist; unresolved scopes
retain their `Q700` diagnostics. The CLI
adds `qcorec plugins list` and `qcorec plugins inspect ID`. Cycle 20 tests are
kept outside source modules in `compiler/tests/cycle20_verifier_plugins.rs`.

`proof/QCore/VerifierPlugin.lean` models provider evidence as an untrusted
proposal and proves that checked safety has valid evidence, missing or invalid
evidence stays unknown, invalid envelopes stay unknown, and a missing/unknown
composition member prevents safety. `proof/AuditCycle20.lean` prints the axiom
surface.

The remaining acceptance work is provider-specific extraction/parity against
the Cycle 19 audits, reproducible standalone packages and full CLI lifecycle
tooling, OS-enforced memory/network isolation, and replacement of the old
monolithic audit routing. Those gates remain open rather than being weakened
by the generic plugin foundation.

### Initial in-process extraction

`event_driven.rs` and `chunk_source.rs` now live under
`compiler/src/verifier_plugins/libevent/` and
`compiler/src/verifier_plugins/chunkio/`, respectively. Each plugin module owns
its manifest and exposes a `provider()` adapter for explicit registry
registration. `first_party_registry()` registers those adapters. The Libevent
profile, state machine, source audit, and observation refinement, and the
ChunkIO source importer and revision pin, live behind these module boundaries.

Typed consumers should use `qcore_compiler::verifier_plugins::libevent` and
`qcore_compiler::verifier_plugins::chunkio`. The old `event_driven` and
`chunk_source` paths remain aliases to the same implementations and types.
External composition, contract checking, and audit examples use the plugin
paths. The existing semantic tests exercise the relocated implementations;
registry regression coverage checks both adapters for every declared capability.

This is an in-process extraction within the compiler crate. Core C11
preprocessing, chunk lifecycle checking, and contract certificate checking remain
in place. The typed APIs preserve their existing bounded results and private
summary construction; they do not create plugin `CoreCertificate` values or
route semantic inputs through protocol v1. The registry adapters still return
`Unknown` for Boolean-only requests. Standalone executable packaging, semantic
protocol integration, and pinned provider parity audits remain open gates.


### Superseding certificate containment, 2026-09-05

The [reevaluation repair](../REEVALUATION_SPEC.md#six-finding-repair-on-2026-09-05)
supersedes the assertion-based certificate results above. Protocol v1's Boolean
facts do not prove capability-specific properties: built-ins now return
`Unknown`, and the core refuses all v1 `Safe` proposals, including empty scopes.
Manifest capability declarations describe routing only. Semantic witnesses,
core-derived obligations, source integration, and provider parity remain required
before independent plugin safety certificates can be issued. The earlier test
results remain historical observations of the narrower assertion-replay model.


### Follow-up containment, 2026-09-05

The [follow-up repair](../REEVALUATION_FOLLOWUP.md#repair-and-containment-on-2026-09-05)
also downgrades every unchecked provider `Unsafe` proposal to `Unknown` with
`QP010`. Provider diagnostics remain available as allegations. There is no
protocol-v1 counterexample checker. The Lean model now proves that rejected
proposals remain unknown; its abstract positive-evidence interface is not an
implementation of the missing Rust semantic witness checker.

Process requests are encoded before spawn. On Unix, a cleanup guard owns a new
process group, kills it on all exits, and reaps the direct child asynchronously
when necessary. Nonblocking stdin/stdout/stderr share the invocation deadline;
there are no blocking pipe-worker joins. The manifest's `output_bytes` applies
to each request and response JSON payload, with four additional header bytes;
stderr has a separate 64 KiB cap. Unsupported non-Unix process execution returns
a protocol failure before spawn. This does not add OS memory/network isolation
or containment for descendants that deliberately leave the process group.

## Generalization dependency extension (2026-09-06)

The [generalization plan](../GENERALIZATION_PLAN.md), preserved as proposed,
connects this milestone to the remaining Cycle 19 work and later genericity
acceptance. The [roadmap](../ROADMAP_AFTER_CYCLE_9.md) records current status.
W1 supplies declarative resource contracts, W2 supplies versioned source-set
profiles, and W3 supplies core obligations and semantic-witness dispatch.
These feed W5's configured audit driver and the existing 20C extraction order;
W4 adds validated custody inference and W6 measures source-level CT coverage.
W7 tracks namespace, naming, roadmap and CI maintenance.

The Cycle 19 resource-audit contract loader is the first executed W1 increment.
It does not implement plugin evidence v2, close VP05, establish provider parity
or authorize removal of the existing source/profile routes. Review the W2
manifest schema and W3 witness schema before their broad implementation, and
archive each W5 provider comparison before deleting its predecessor.

## W3 bounded execution note (2026-09-07)

The earlier configuration-only description is superseded by
[`w3-evidence-v2`](../reports/w3-evidence-v2/README.md). Source scopes derive
capability obligations and dispatch through the registry. The checked positive
source path is observation agreement under an explicit trust assumption and
recorded-run bounds. A standalone provider links only the published schemas
and JSON. Lifecycle model replay is exercised, but source lifecycle effects
are deliberately incomplete pending a trusted importer. Empty inventories,
unsupported capabilities, mismatches, malicious witnesses and unchecked
violation allegations stay Unknown. Host relocation, W2 production-profile
composition and first-party semantic acceptance remain open.
