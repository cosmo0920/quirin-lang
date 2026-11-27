# W3 generic verifier-host extraction (2026-09-07)

`qcore-verifier-host` now owns the registry, package discovery, source scope
resolution, process transport, v1/v2 envelope firewall, private core certificate
construction and certificate composition. It builds and tests with zero provider
crates and without `qcore-compiler`, `qcore-contract` or `qcore-c-gateway`.

Its only workspace dependencies are `qcore-schemas` and `qcore-frontend`.
The latter preserves the existing `resolve_source_scopes` method and implements
the permitted L2-to-L1 dependency. External dependencies are serde, serde_json
and Unix libc. The compiler's libc dependency moved with process transport.
The workspace dependency guard includes the new crate and compiler-to-host edge.

`compiler/src/verifier_plugin.rs` remains a compatibility facade, so callers
retain the same public types and methods. First-party package construction
remains on the compiler side. Host code contains no first-party registration,
provider implementation or library-specific identifier.

The `WitnessChecker` trait belongs to the host; capability-specific trusted
replay and source obligation derivation remain in `qcore-compiler` temporarily.
The host invokes that trusted callback and never imports the compiler, avoiding
a dependency cycle. This is a mechanical extraction, not full kernel/W5
consolidation. Production source/profile import remains open, source lifecycle
derivations remain incomplete, and the W3 observed-agreement trust/bounds
limitations continue to apply.

## Parity

[relocation-parity.json](relocation-parity.json) records the move from `11daf01`.
After removing the contiguous first-party helper block (retained by the
compiler facade) and substituting the frontend crate import, the generic host
source is **byte-identical** to its previous implementation. The existing
private composition regression moved with it. No firewall, process deadline,
output cap, v1 refusal, v2 witness relation or verdict rule changed.

The host-only tests additionally exercise an empty registry and an untrusted
assertion-echo provider with empty and nonempty coverage. Both remain Unknown
without importing a first-party package. The full workspace suite retains the
schemas-only standalone Rust provider build/run and malicious-v2 process test.
No hardcoded audit route was removed and no pinned unit is promoted.

## Validation

The standalone host suite passes (3 tests). The full standing gate passes:
submodule initialization, **504 workspace tests**, formatting, clippy with
warnings denied, both reevaluation examples and `lake build` all return exit 0.
The workspace gate includes actual schemas-only Rust provider compilation and
process conformance. See [gate-results.json](gate-results.json) and adjacent logs.
There are no semantic changes requiring new Lean definitions; the existing
Lean model and audits remain the corresponding semantic reference.
