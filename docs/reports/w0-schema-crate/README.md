# W0: schema-surface and CLI extraction

Date: 2026-09-06. Status: W0 of the
[generalization plan](../../GENERALIZATION_PLAN.md) executed; the workspace
has three members and the published schema surface is a standalone crate.
No analysis logic moved, no verdict changed, and no fail-closed gate was
touched.

## What was extracted

`crates/qcore-schemas` (layer L0, serde types only, no workspace
dependencies) now holds, re-exported from `qcore-compiler` at the previous
paths for compatibility:

| Schema module | Moved from | Types |
| --- | --- | --- |
| `plugin` | `compiler/src/verifier_plugin.rs` | `PLUGIN_PROTOCOL_VERSION`, `PLUGIN_EVIDENCE_VERSION`, `DEFAULT_MAX_FRAME_BYTES`, `PluginId`, `VerifierCapability`, `PluginTrustClass`, `PluginEntrypoint`, `PluginLimits`, `SupportedTarget`, `PluginManifest`, `ManifestError`, `UnitIdentity`, `TargetProfile`, `FactPredicate`, `NormalizedFact`, `VerificationRequest`, `RequestError`, `ProposedVerdict`, `PluginDiagnostic`, `EvidenceClaim`, `Reproducibility`, `ProposedEvidence`, `VerificationResponse` |
| `contract` | `compiler/src/legacy_c.rs` | `ResourceApiKind`, `ResourceParameterMode`, `ResourceReturnMode`, `ResourceFunctionContract`, `ResourceContracts` |
| `mechanism` | `compiler/src/external_integration.rs` | `LibraryMechanismSummary` |
| `identity` | `compiler/src/external_source.rs` | `ExternalSourceBundle` |

`crates/qcorec` now owns the CLI binary (`compiler/src/main.rs` moved
verbatim) and the two tests that execute it as a process
(`core_cli_works_without_clang`,
`cli_checks_contracts_and_refuses_standalone_emit_without_overwriting_output`).

## What deliberately stayed in `qcore-compiler`

The trusted core side of the plugin protocol: `VerifierProvider`,
`BuiltInProvider`, `ProcessProvider`, `VerifierRegistry`, the certificate
firewall, `CoreCertificate`, `VerificationOutcome`, composition, the
first-party manifest literals, and the frame encode/decode helpers (they
carry `ProviderFailure`, a host-side error). The over-fitted
`AuditedLibrary`/`LibrarySummaryProfile` tables stay pending their W2
replacement by data profiles. The legacy-C analyzer, loader, and every
checker are unmoved.

## Behavior-affecting details, all reviewed

- `ResourceContracts::check_source_assumptions` entangled the data type with
  the front-end parser; per the W0 guidance the impl was split from the type:
  it is now `contract_loader::check_source_assumptions(&contracts)` with the
  same body and message. `ResourceContracts` gained a `functions()` accessor
  and `set_source_assumptions` became `pub` (it declares trusted assumptions;
  it was `pub(crate)` for the same caller before).
- Moved types gained `serde` derives (schema-surface rule: everything
  serializable). No previously existing wire format changed;
  `verifier_plugin` types kept their exact serde attributes.
- One included unit-test file (`tests/unit/external_source.rs`) imported
  `BTreeMap` implicitly through the module it tests; it now imports it
  directly.

## Guard

`compiler/tests/workspace_layering.rs` fails if `qcore-schemas` grows a
`path` dependency or any dependency on a workspace crate (plan invariant 7:
dependencies point downward only). The crate's README records the semver
compatibility policy (W0 task 3).

## Recorded decision

Pluggability is compile-time for now: providers become separate crates
depending on `qcore-schemas` only, running in-process behind the same
request/response types. The plan's eventual process boundary is deferred,
not abandoned; schema types remain process-portable, so the deferral costs
no schema rework.

## Standing gate

- `cargo test --workspace`: 408 passed, 0 failed (406 in `qcore-compiler`
  including the new layering guard, 2 relocated CLI tests in `qcorec`).
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `reevaluation_probe` exit 0; `reevaluation_followup` exit 0.
- `lake build`: 51 jobs, success (no Lean change was needed; no semantics
  moved).
- Contract corpus, submodule pin, and Cycle 20 plugin tests unchanged and
  green.
