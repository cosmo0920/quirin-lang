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
seven mapped and 43 unmapped, with separate stages and Linux-only native evidence for CT13/CT15.

| Finding | Current acceptance | Evidence / remaining boundary |
|---|---|---|
| R01 | repaired model | [Report](../../../docs/reports/r0-3-concurrency-lifetimes/README.md); Live IDs are unique lifetimes in TrustedExplicitTrace. No arbitrary-source extraction or general callback lifetime proof. |
| R02 | repaired model | [Report](../../../docs/reports/r0-3-concurrency-lifetimes/README.md); Active task state is separate from retained clock history. The [scoped-copy source path](../../reports/w6-native-source-tasks/README.md) now checks explicit task results and joins; [Result recovery](../../reports/w6-task-recovery/README.md) preserves prior siblings through failed creation; general authority/loan lowering remains open. |
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
  the out-formal reassignment regression. Direct nullable Removed wrappers and
  context-specific recursion now validate; general aliases/recursive fixed points
  and cost inference remain open. The [custody transfer repairs](../../reports/w4-nullable-removal/README.md)
  reject ten reproduced false-Safe patterns while retaining unresolved effects;
  gateway code is in its own crate.
- W5 has a configured provider-free audit runner and sealed-import process
  dispatch with a second-codebase demonstration and exact four-artifact parity;
  general lifecycle import, specialized adapters and consolidation remain.
- W6 rejects ignored source function contracts with Q021 and resolves body-local
  guard/scope binders. The later scoped-copy source path now executes Linux C11
  leaf workers. CT15 maps join failure and CT19 uses real sibling-failure source;
  explicit Result recovery now maps CT13 with native storage checks.
- W7 has local Libevent diagnostic codes, exact provider diagnostic attribution,
  generic Lean names and Lean/axiom CI.
- The native reopen-failure evidence package is now present and separately
  validated. Fixed observations do not promote raw Cycle19 source units.

Execution records are indexed in the [workstream roadmap](../../ROADMAP_AFTER_CYCLE_9.md).
Current validation is recorded in [VALIDATION.md](../../VALIDATION.md): 737 Rust tests, 43 Python evidence-validator tests, both probes, format, Clippy, Lean build and 20 axiom audits passed. Nineteen separate provider tests also pass. The latest full pinned audit comparison is archived in the [configured frame-helper report](../../reports/w5-configured-frame-helpers/README.md). These counts are different suites, not interchangeable conformance cases.
