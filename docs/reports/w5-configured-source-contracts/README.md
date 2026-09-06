# Configured source-contract dispatch

Recorded 2026-09-07 against main baseline `242bc98a259fbe704de00abfedd07f669c4f298d`.
The audit's bounded Boolean-field process route now loads package, model-library
and policy bindings from configuration through `qcore-audit`. A second C codebase
uses this same service and a real schemas-only child process. The production
run and the previous package-path alias reproduce all four baseline artifacts
byte-for-byte. This is a bounded W5 extraction, not full lifecycle composition.

## Implementation and authority

`crates/qcore-audit/src/source_contract.rs` exposes `SourceContractRegistry` and
`SourceContractPolicies`. The strict version-1 configuration anchors relative
paths at its directory, rejects duplicate libraries/packages and unknown fields,
and snapshots policy bytes. Limits are 1 MiB configuration, 256 packages,
4,096 bindings, 8 MiB per policy and 32 MiB total retained policy bytes.

Registry loading verifies executable packages, provider identities, the existing
Boolean-field checker and required protocol/evidence/capability declarations.
Dispatch chooses the binding using the sealed Clang import's model-library
identity and checks full manifest target support. It derives the existing opaque
source task, invokes the process and requires a checked certificate from trusted
replay before returning evaluation metadata. Configuration supplies selection,
not source authority. The importer, task representation, evaluator, firewall,
schema1.0.0 and production ChunkIO0.3.0 package are unchanged.

`contract/profiles/fluent-bit/source-contracts.json` supplies production bindings.
`QCORE_SOURCE_CONTRACTS` selects an explicit configuration. The existing
`QCORE_CHUNKIO_PACKAGE` alias selects a package with the default bindings and
preserves working-directory-relative path interpretation. Both together reject.
The four CLI checks in `cli-checks.json` reject missing configuration, an empty
alias, missing packages and conflicting selectors before source auditing.

The selected process route no longer contains provider-ID/library-policy
selection literals or a duplicate direct evaluation before dispatch. Trusted
witness checking remains mandatory. The no-selector direct comparison uses the
same policy data and does not require installed packages. Broader retained
specialized adapters and lifecycle routes have not been deleted.

## Independent process demonstration

`providers/conformance/tests/configured_source_contract.rs` imports the separate
`sample_widgets` C fixture with Clang. Its profile maps `sample_record.ready`
onto the existing logical Locked Boolean axis; `sample_ready` and `sample_invert`
exercise four input-state combinations each. `sample_boolean_provider` reuses
the generic statement executor and has only schemas/Serde normal dependencies;
the trusted service and import machinery are test dependencies.

Tests exercise checked results and policy snapshots; false postconditions reject
at trusted replay with QP009. Missing bindings/packages, model-library/revision
mismatches and installed manifest scope widening reject. Unsupported field
operations retain an explicit nonempty Unknown inventory. These process tests
are Unix-bound because the host requires process-group containment; Linux was
executed here. The fixture's declared revision is a profile premise, not evidence
of an actual Git checkout pin. It does not add arbitrary record semantics,
production caller reachability or broader architecture attestation.

## Production parity

`audit-parity.json` records exact commands, reused package manifest, input hashes,
artifact hashes and comparison lengths. The durable Monkey compile database and
source/build trees under `build/cycle19` were reused; no package was rebuilt.
New runs should use the persistent output recipe in
[the build guide](../../CYCLE19_BUILD_INPUTS.md).

| Evidence | Configured versus baseline | Alias versus configured |
|---|---|---|
| Source | byte-identical | byte-identical |
| Diagnostics | byte-identical | byte-identical |
| Chunk contracts | byte-identical | byte-identical |
| Source effects | byte-identical | byte-identical |

All eight real generated diff files decompress to empty. Counts remain 185 raw
units: **2 Safe / 42 provisional Unsafe / 137 Unknown**, plus four data-only
units. The source-effects sidecar retains **3 checked function contracts / 93
Unknown**. No raw unit is promoted and no provisional verdict is an upstream
bug claim. Archived gzip JSON files are the actual outputs from both full runs.

## Validation and remaining work

All standing checks passed: 722 workspace tests; format and all-target Clippy;
both reevaluation probes; Lean build and 20 axiom audit modules. The separate
provider workspace passed 17 tests, format and Clippy. Python evidence validation
passed 43 tests. Six new service configuration tests, three process integration
tests and five executor tests in the fixture binary account for the increases.
After adding the explicit Unix test bound, the three process tests and provider
format/Clippy were rerun successfully. No semantic definition changed, so the
existing Lean models and theorem surfaces apply unchanged. Logs, commands and
source identities are archived in `validation.json` and its referenced files.

General lifecycle source import, other specialized provider extraction,
provider-owned full source analysis and W5 consolidation remain open. Package
verification is a filesystem snapshot, not a signature, immutable execution
attestation or OS sandbox. CT coverage remains 7 mapped / 43 unmapped; Rust test
counts are not language-conformance counts.
