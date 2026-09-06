# Revision 0.3 implementation and verification ledger

The supplied review is adopted as the basis for revision 0.3, with current
implementation claims checked against the merged repository. The original
[review](supplied/REVIEW.md) describes archive SHA-256
`fd3e844ec975c74e47c7113037c334dd7a4f78fbb8a7e1543a5e59f58da5760b`, not current
main. Its original five files are archived byte-identically in `supplied/`; the user's ignored `unverified_specs` inputs are left in place.
The advertised archive, Rust probes, acceptance JSON, source inventory and
validation logs were not supplied; this integration does not claim to run or
reconstruct them. This ledger and source inventory are new local records.

## Current acceptance

`acceptance.json` is the machine-readable record. Requirements are not marked
repaired until implementation, relevant Rust regressions, Lean parity where
semantic rules change, and the standing gates have passed. Profile-qualified
results do not imply complete language conformance. The CT manifest remains
five mapped and 45 unmapped, with separate stages.

| Finding | Current acceptance | Evidence / remaining boundary |
|---|---|---|
| R01 | repaired model | [Report](../../../docs/reports/r0-3-concurrency-lifetimes/README.md); Live IDs are unique lifetimes in TrustedExplicitTrace. No arbitrary-source extraction or general callback lifetime proof. |
| R02 | repaired model | [Report](../../../docs/reports/r0-3-concurrency-lifetimes/README.md); Active task state is separate from retained clock history; source concurrency lowering remains unsupported. |
| R03 | explicit trust boundary | [Report](../../../docs/reports/r0-3-concurrency-lifetimes/README.md); Checked mutex-footprint invariants and reconstructed synchronization witnesses remain open; the model names the assumptions it consumes. |
| R04 | partially repaired | [Report](../../../docs/reports/r0-3-source-validation/README.md); Explicit i32/unit scalar compatibility is checked. Separate custody payload metadata preserves the reduced model; full owned-result/per-exit typing remains open. |
| R05 | repaired source | [Report](../../../docs/reports/r0-3-source-validation/README.md); Axis environment validation is unconditional; closed index/profile limits remain. |
| R06 | repaired model | [Report](../../../docs/reports/r03-loop-counts/README.md); Nonzero-net positive maximum counts reject. Exact counts remain supplied premises; identity alternatives and general source loops remain unsupported. |
| R07 | repaired consumers | [Report](../../../docs/reports/r0-3-source-validation/README.md); Full i32/i128 literal extrema are checked; arbitrary expression and result semantics are not added. |
| R08 | validated bounded consumers | [Report](../../../docs/reports/r03-contract-evidence/README.md); Resource loading, structural retention and observation replay remain separate; arbitrary contract theorem proving and source caller-precondition discharge remain open. |
| R09 | reconciled | [Report](../../../docs/reports/r03-integration/README.md); Current specs, commands, diagnostics, validation and obsolete navigation updated; historical reports retain their original provenance. |
| R10 | hardened bounded evidence | [Report](../../../docs/reports/r03-contract-evidence/README.md); Observation scope/module/bounds substitution rejects. Native provenance is declared trusted evidence; whole-program/source-model refinement remains open. |

## Changes since the reviewed archive

- W2 profile loading, table migration, Boolean-field routing and provider
  metadata discovery now carry declared assumptions and pinned parity evidence.
- W3 has core-derived inventories, bounded v2 observed-agreement replay and
  dispatch, and a separate provider host. Explicit false observation trust
  rejects; accepted observation evidence stays bounded to recorded runs.
  Lifecycle source derivation is still incomplete; v1 Safe refusal is unchanged.
- W4 adds validated direct Out/Stored/Reallocated and nullable returns, including
  the out-formal reassignment regression. Removed/recursion/general aliases and
  cost inference remain unsupported; gateway code is in its own crate.
- W5 has a configured provider-free audit runner and a second-codebase example;
  specialized production adapters and provider/kernel consolidation remain.
- W6 rejects ignored source function contracts with Q021 and resolves body-local
  guard/scope binders; this does not implement source concurrency lowering.
- W7 has local Libevent diagnostic codes, generic Lean names and Lean/axiom CI.
- The native reopen-failure evidence package is now present and separately
  validated. Fixed observations do not promote raw Cycle19 source units.

Execution records are indexed in the [workstream roadmap](../../ROADMAP_AFTER_CYCLE_9.md).
Current validation is recorded in [VALIDATION.md](../../VALIDATION.md): 549 Rust tests, 43 Python evidence-validator tests, both probes, format, Clippy, Lean build and 19 axiom audits passed. The full pinned audit preserves 185 unit results. These counts are different suites, not interchangeable conformance cases.
