# W3 bounded evidence-v2 replay and scope dispatch (2026-09-07)

This increment implements the core obligation/task wire records, a distinct
v2 request/response schema within unchanged length-delimited protocol-v1
framing, semantic witness replay, and source `verify scope` dispatch from the
approved [W3 design](../../design/W3_OBLIGATIONS_EVIDENCE_V2.md).
It is a bounded increment, **not aggregate W3 acceptance**.

## Checked behavior

- The core derives the obligation denominator, source/contract digests, target,
  semantic profile, bounds and assumptions. Certificates retain the complete
  task and replayed witness. Request identity, target, assumptions and obligations
  must match the core task; digest echo alone cannot rebind these fields.
- Every v1 Safe refusal is preserved. V2 Safe requires an implemented core
  replay, a nonempty complete inventory, exact coverage and a matching envelope.
  Unchecked Unsafe stays Unknown (`QP010`). Unknown effects and unsupported
  capabilities cannot be discharged by provider assertions.
- Observation replay evaluates core-pinned `requires` and `ensures normal`
  predicates. Missing variables, false postconditions, missing/renamed hooks,
  duplicate run IDs, mismatched profiles and ambiguous contract names reject.
  False preconditions do not contribute coverage. The current required-hook
  schema is a set: its minimum is one checked observation per required hook.
- A successful observation certificate means **agreement for the recorded
  runs**, under `observations_trusted: true` and `observed_runs_only: true`.
  The provider supplies runtime values and native-run provenance; replay does
  not independently authenticate that a binary produced them. Run identities
  and bindings are retained in the certificate witness; the task's initially
  empty observed-run scope does not mean all runs.
- Lifecycle replay checks every model site exactly once, operation/identity
  fidelity, source ordering and the core chunk transition checker. Tests reject
  omissions, duplicates, phantom sites, reordering, retargeting and unreleased
  model state. **Source lifecycle derivation always carries an incomplete
  marker**: spelling `cio_*` does not prove API identity, fallibility, backend,
  residency or delete parameters. These model-replay tests do not certify real
  ChunkIO source. Exact statement grammar also diagnoses nested calls, unknown
  calls, complex arguments, rebinding and early exits; later cleanup cannot be
  counted after return.
- Composition requires the full nonempty obligation union and rejects missing
  coverage, incompatible bounds and conflicting assumptions.

## Source and profile boundary

`compiler/src/verify_dispatch.rs` currently constructs a `source` revision,
`qcore` dialect target from the selected profile and manifest. The conformance
provider deliberately declares that target. Pinned production C-library
profiles that do not match fail QP005/Q701; no revision, ABI or source identity
is inferred from their library name. W2 production-profile import, C gateway
obligation import, event/concurrency derivation, complete lifecycle importing
and first-party provider promotion remain open. The v1 fallback is dispatched
when a nonempty inventory exists; it still cannot establish safety.

The source compiler retains its Cycle 12 executable subset after successful
scope verification. Dispatch is now real (VP05's noninvocation gap is closed
for the explicit conformance target), but it does not add general structured
source execution or enforce source function contract clauses.

## Independent ABI and adversarial evidence

`compiler/tests/fixtures/w3/provider/` is a standalone Rust binary with its own
workspace and lockfile. Its dependencies are **qcore-schemas and serde_json
only**. `w3_evidence_v2` builds it offline from its manifest, loads it through
`ProcessProvider::from_package`, checks the exact framed request and verifies
both an accepted recorded observation and rejection of a forged postcondition.
The fixture's values and provenance are synthetic conformance data, not Cycle
19 native evidence. Separate mutations cover 14 malicious response variants,
request/task rebinding, missing replay and incomplete derivation.

Observation i128 values use canonical decimal strings on the wire. This avoids
lossy JSON consumer numbers and serde's internally tagged enum i128 limitation.
Tests round-trip both signed extremes and reject numerical JSON values, floats,
out-of-range strings, leading zeros, explicit plus and negative zero.

## Lean and diagnostics

`QCore.VerifierEvidenceV2` is a reduced executable observation model: required
hook identities and predicates come from the core task; records supply values.
Its theorems establish a complete nonempty denominator and an actual matching
observation satisfying the recomputed pre/postconditions for every required
hook. False preconditions never count. This is not a proof of the full Rust
predicate parser, hashing, transport or source import. `AuditCycle20.lean`
prints the theorem axiom surfaces without placeholders or project axioms.

W7 diagnostic collision repair moves Libevent Q700–Q712 to
`qcore.libevent.L00`–`L12` and reserves Q700–Q719 for core scope diagnostics.
The spec changelog and provider module retain the full mapping; archived audit
records are not rewritten.

## Remaining architecture work

The verifier host is still in `qcore-compiler`; `qcore-verifier-host` extraction
is deliberately a separate follow-up. Existing library routes are retained.
No pinned corpus unit is promoted, no parity gate is deleted, and no first-party
provider is advertised as supporting v2 merely because the schema exists.

## Validation

All standing commands returned exit 0: submodule initialization, workspace tests
(**458 passed**), formatting, clippy with warnings denied, both reevaluation
examples and `lake build`. The standalone schemas-only provider is built by the
workspace integration test. `lake env lean AuditCycle20.lean` also returned 0;
its output contains only Lean standard axioms (`propext`, `Quot.sound`) where
needed, and no project axioms. See [gate-results.json](gate-results.json) and
the adjacent logs. There are no ignored or failed tests in this run.
