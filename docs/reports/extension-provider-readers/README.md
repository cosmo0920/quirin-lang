# EXT-01 provider package readers

Base: `64ddac3f60fe9b848dd789e895d67b809cce93e2`.

## Prediction, recorded before implementation

This card changes provider Python path readers and inventory validation only. Existing `.qcore` source, manifest, published-list, load-path, lock and package artifact bytes are not renamed or rewritten. The expected delta for all five existing provider package inventories, package digests, contract counts and checker verdicts is **zero**. Synthetic `.qv` contract-mode modules will be accepted at their literal paths and hashes; a `.qcore`/`.qv` pair with the same logical module stem will be rejected before search-order selection. Legacy `.qcore` modules remain readable and emit a visible path-specific warning stating detected contract mode and `.qv` recommendation. No pin is regenerated and no unsupported source form becomes Safe. If actual parser mode validation cannot be reached through this Python boundary, the report will name that integration dependency; it will not substitute a lexical guess.

## Implemented boundary

`providers/contracts.py` discovers `.qv` next to `.qcore`, reads exact `.qv` names from `PUBLISHED`, and keeps exact source bytes and SHA-256 paths in the load-path manifest and provider inventories. Its file census rejects two extensions for one relative stem. The specific ChunkIO `fluent_bit/busy_state.qcore` owner selector also admits that declaration's `.qv` spelling, without broadening the owner prefix. `providers/package.py` accepts `.qv` in contract paths and include pins, verifies exact installed files and hashes, and refuses dual stems in declared assets, installed assets and included parents. It does not rewrite or fall back from a requested path.

For `.qv`, the Python reader runs the frontend's batch `source_extension_modes` example, which uses `parse_source_module_mode` and refuses a missing, malformed or non-contract mode. The probe reports each file separately so malformed legacy input remains on its prior admission route and receives an `unknown mode` warning. The `check`, `build` and `verify` commands print `warning[legacy-source-extension]` with the actual file path, parser-derived mode and recommended suffix for legacy `.qcore`; no numeric diagnostic code was allocated. The probe is a header-intent guard, not a contract declaration validator. The existing Rust loader and verifier remain responsible for those checks.

## Identity and controlled exception

Comparison against the assigned base found **119 tracked provider contract assets and 16 tracked contract inventories/load-path/contract locks with zero changed paths**. The exact path and hash inventory is in `private/ext01/inventory-identity.json`. All existing `.qcore` assets, `PUBLISHED` strings, `contracts.json`, `contract-load-path.json` and `contracts.lock` remain byte-identical. Five built packages independently passed `package.py verify`; their pinned contract counts were ChunkIO 44, Cmetrics 4, Ctraces 1, libevent 16 and librdkafka 8 (73 total). Verification checks installed contract asset paths and hashes against each embedded inventory. The measured package output is `private/ext01/packages/packages.json`. It is a local artifact, not a baseline binary comparison or a checker-verdict claim.

The assigned base's separate `providers/Cargo.lock` was already missing the `quirin-source-analysis` dependency graph introduced by the source-analysis crate. The first two locked package-build attempts exited 1 when Cargo refused to update that lock. Offline full Cargo metadata showed only a new `quirin-source-analysis` package node and one dependency edge in `qcore-audit`; the correction changes no versions or checksums and does not touch contract pins. The base provider-lock SHA-256 was `3b2dc3ea3a4eb3c1ca50be797f225e12bc9f1cb5360b9ab6651f840b2d28d10b`; the corrected SHA-256 is `4a5cd746c84e35f48db182eb379391d74dad98d6a6fc27be90df7569570591b3`. Accordingly, the new `packages.json` provenance `lock_sha256` differs from the prediction. No equality claim is made for rebuilt executable or archive bytes. This is a dependency-graph repair required by `--locked`, not a repin of any source or contract identity.

## Validation and private logs

All commands ran from `/tmp/extension-provider-readers-qcore` with the initialized `third_party/libevent` submodule at `d82464a277d0f42703702c4dfd9af6af38595a83`. The private paths below are excluded from Git.

| Command | Exit / measured result | Evidence |
| --- | --- | --- |
| `CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/extension-provider-readers-target python3 -m unittest discover -s providers/tests -v` | 0; 24 tests | `private/ext01/python-providers.log` |
| `CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/tmp/extension-provider-readers-target python3 providers/package.py build --output private/ext01/packages --target-dir /tmp/extension-provider-readers-target/packaging` | 0; five packages built and internally verified | `private/ext01/package-build-resolved.log` |
| `python3 providers/package.py verify private/ext01/packages/NAME` for each of five package names | 0 each; five independent verifications | `private/ext01/verify-qcore.chunkio.log`, `private/ext01/verify-qcore.cmetrics.log`, `private/ext01/verify-qcore.ctraces.log`, `private/ext01/verify-qcore.libevent.log`, `private/ext01/verify-qcore.librdkafka.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | `private/ext01/provider-fmt.log` |
| `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/tmp/extension-provider-readers-target/packaging cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast -j 1` | 0; 43 suites, 129 tests passed, 0 failed | `private/ext01/provider-cargo-test-serial.log` |

The failed locked-build attempts are preserved in `private/ext01/package-build.log` and `private/ext01/package-build-fixed.log`; both failed on the pre-existing provider-lock dependency graph, before provider compilation. The first parallel provider Rust test attempt is preserved in `private/ext01/provider-cargo-test.log`: exit 101 during two `rust-lld` links before their test assertions ran. It is not counted as a pass. The serial retry passed. The parent integrated tree's provider gate is recorded separately below; no core-workspace full-suite claim is made by this card.

## Integrated provider gate

The parent integrated worktree at `/home/cosmo/.codex/worktrees/audit-checkpoint-batches/qcore-c` included this provider patch, the Rust contract readers and the shared parser helper. Only documentation changed during the gate. Provider formatting exited 0 (`private/ext01/integrated-provider-fmt.log`); provider workspace clippy with `--workspace --all-targets --locked --offline -j 2 -- -D warnings` exited 0 (`private/ext01/integrated-provider-clippy.log`). The complete locked offline provider workspace test command, serialized with `-j 1`, exited 0: **43 test summaries, 129 passed, 0 failed** (`private/ext01/integrated-provider-cargo-test.log`). The five-package integrated build/verify remains to be appended after the parent core timed-test gate releases the host.
