# Provider-owned Quirin contracts

Base: `10aafcf8`. Status: implemented and validated at the recorded boundaries.

The user prioritized repository structure before the remaining Cycle 18
production acceptance work. The five provider crates now own 72 Quirin contract
modules under their `contracts/` directories: ChunkIO 44 (including Fluent Bit's
`busy_state`), Libevent/Monkey 15, librdkafka 8, CMetrics 4 and CTraces 1.
Their `contracts.json` files enumerate package-relative paths and SHA-256 values.
Shared language, support-library and application policies without a provider owner remain in the shared corpus.

## Ownership and compatibility

`providers/contracts.py` is the ownership map and synchronization entry point.
`sync` copies canonical provider sources to the retained top-level corpus and
regenerates inventories; `check` writes nothing and refuses missing, stale,
changed or unowned files. Orphaned compatibility contracts require deliberate
removal and corpus pin updates; synchronization never deletes them silently.
Symlink inventories cannot redirect synchronization writes.

The [byte comparison](contract-parity.json) measures every canonical contract
against the base and its retained `contract/qcore/` snapshot: all 72 are exact.
These snapshots preserve the existing in-process routes, pinned resource
manifest, source paths and core build independence from the optional providers
workspace. There are still 96 logical corpus modules, not 168. Ownership moved;
contract predicates, declarations, clause counts and Lean semantics did not.
Existing Rust and audited Lean verification models remain applicable unchanged.

A GPT-5.6 Sol agent at medium reasoning implements package distribution and
verification in an isolated worktree. The integrator owns the canonical assets,
compatibility checks, documentation, review, complete gate and publication.

## Acceptance boundaries

Packaging contracts does not grant a provider trust or additional capabilities.
Contracts remain declared premises; the independent checker retains its existing
obligations and refusal behavior. This structural change closes none of the
production callback, cancellation, event-generation or foreign-host acceptance
gaps. The ten proposed Priority 1/2 task candidates remain the next semantic
workstreams after this packaging priority.

Cargo's own `package --list --allow-dirty --offline --manifest-path …` was run
for each artifact crate. All five exits were 0 and every owned contract plus
its inventory was included; [the counts](cargo-package-lists.json) are
44/4/1/15/8. This checks the source-package inventory, not crates.io publication
or an independently buildable Cargo archive (these crates remain unpublished
and use the existing workspace/shared-source layout).

## Integrated validation

The [gate](gate.json), [core command record](gate-facts.json) and
[provider gate record](provider-gate-summary.json) record **5,034 core Rust passed, 0 failed, 11 ignored; 164 Lean audits; 324 root Python tests; 122 provider Rust tests**.
The core run measured `5ff36cfdfdf313ee69539b832e0742a8d913d54f`; subsequent integration
changes only the separate provider workspace, ownership tests and documentation.
Core compiled sources, shared contract bytes, workspace manifests, resource
selection and Lean are unchanged from the base. Provider tests measure their
own source commit and the final root Python suite follows matrix regeneration.

| Command | Exit | Private log |
|---|---|---|
| `git submodule update --init --recursive` | 0 | `private/provider-contract-packaging/core-gate/submodules.log` |
| `clang --version` | 0 | `private/provider-contract-packaging/core-gate/clang-version.log` |
| `cargo test --workspace --no-fail-fast` | 0 | `private/provider-contract-packaging/core-gate/cargo-test.log` |
| `cargo fmt --all -- --check` | 0 | `private/provider-contract-packaging/core-gate/cargo-fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `private/provider-contract-packaging/core-gate/cargo-clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `private/provider-contract-packaging/core-gate/reevaluation-probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `private/provider-contract-packaging/core-gate/reevaluation-followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0 | `private/provider-contract-packaging/core-gate/lean-audits.log` |
| `cargo test -p qcore-compiler --test cycle20_verifier_plugins vp04_vp07_vp08_process_lifecycle_and_payload_budgets -- --exact --test-threads=1` | 0 | `private/provider-contract-packaging/core-gate/isolated-process-lifecycle.log` |
| `cargo test -p qcore-verifier-host --test enforced_process_limits a_declared_process_limit_stops_the_fixture_from_forking -- --exact --test-threads=1` | 0 | `private/provider-contract-packaging/core-gate/isolated-process-limit.log` |
| `cargo test -p qcore-verifier-host --test enforced_process_limits memory_growth_beyond_the_declared_limit_is_bounded -- --exact --test-threads=1` | 0 | `private/provider-contract-packaging/core-gate/isolated-memory-limit.log` |
| `cargo test -p qcore-verifier-host --test confined_process_execution a_confined_fixture_cannot_write_outside_the_admitted_paths -- --exact --test-threads=1` | 0 | `private/provider-contract-packaging/core-gate/isolated-confined-write.log` |
| `cargo test -p qcore-verifier-host --test standalone_host closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request -- --exact --test-threads=1` | 0 | `private/provider-contract-packaging/core-gate/isolated-closed-input.log` |
| `python3 -m unittest discover -s scripts/tests` (after matrix regeneration) | 0 | `private/provider-contract-packaging/python-final.log` |

Ownership tests cover source/snapshot drift, missing or stale inventories,
orphan preservation, inventory symlink refusal, repository-path aliases and
unowned policies. Package build/verify runs cover all five providers, exact
asset identity, archive extraction and repeated deterministic package output.
The CMetrics semantic conformance test consumes its policy from the installed
package. Its expected verification result and the protocol remain unchanged.

No contract declaration/clause pin changed. The 96 logical modules, 33 selected
resource modules, 59 resource declarations and 281 registered follow-ups remain.
The [reconciliation record](reconciliation.json) records this close's decisions;
the [post-integration residual scan](residual-sweep.json) records 61 historical
names outside the tracker. The 281 named follow-ups are unchanged from the base;
no entry was retired and no historical residual is discharged by this scan.

The integrated checkout also built and verified all five packages;
[integrated artifacts and comparison](integrated-packages.json) record exit 0.
All 72 contract files and the five inventories match the delegated build.
Executables and their corresponding plugin-manifest digests differ between
checkouts, so no cross-checkout executable/archive byte identity is claimed.
The delegated same-checkout repeated-build comparison remains its narrower
measurement.

The provider lane's [report](../provider-contract-bundle/README.md) preserves
the initial Python member-order assertion failure and the sandbox confinement
failure (121 passed/1 failed, exit 101). The corrected Python tests and complete
native-host suite passed. No assertion or confinement rule was weakened.
Its sixteen private log files were copied and hash-verified under
`private/provider-contract-packaging/agent-evidence/`; the backup inventory is
`private/provider-contract-packaging/agent-evidence-backup.json`.

Merge `e44e9a1c` preserves the implementation history. Two add/add conflicts
from the shared asset checkpoint were resolved by keeping the integrator's
reviewed `contracts.py` and ownership tests: the symlink guard applies at and
below the contract boundary, permitting legitimate repository-path aliases.
Both ownership and packaging tests passed again in the integrated checkout.

The [cleanup record](cleanup.json) confirms the completed worktree was clean,
its tip was integrated, its agent reported no jobs and no process references
were observed before removal. Sixteen log files (54,948 bytes) were preserved
with verified hashes. The package outputs remain in the root-private directory;
the active external worktree remains untouched. Worktree removal and merged
agent-branch deletion both exited 0.

Reconciliation exited 0, verifying all ten content claims for four applied
items. Its log is `private/provider-contract-packaging/reconciliation-final.log`.
