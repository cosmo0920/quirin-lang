# W5 sequential semantic kernel consolidation

Baseline: main `6a66f99`. The provider-independent sequential kernel now resides physically in `crates/qcore-core`. Compiler module paths and crate-root reexports remain source compatible and resolve to the same Rust types as the core crate.

## Relocated implementation

Ten modules moved from `compiler/src`: `ownership`, `resource_flow`, `typed_errors`, `error_region`, `cleanup_lowering`, `cfg`, `c_emit`, `executable_model`, `advanced_control`, and `inference`. Their nine existing unit-test files moved to `crates/qcore-core/tests/unit` (`cfg` has no separate unit-test file). All 19 source/test files are byte-identical to the baseline, requiring no path normalization: 4448 implementation lines and 6274 combined implementation/test lines. `file-parity.json` archives baseline and resulting hashes. Internal crate-root imports retain their spelling through core's public reexports.

Core has no runtime dependencies and no workspace dependencies. Its only direct development dependency is `cc`, used by the existing generated-C compile/run tests; its dependency tree is archived separately for runtime/build and test edges. Core tests do not build or load verifier providers, frontend, contract, gateway, or audit crates. Compiler keeps its own `cc` development dependency for remaining native tests.

Whole-program checking, source driving, concurrency, verifier hosting/replay, library adapters and audit orchestration retain their existing owners. This completes this bounded sequential-kernel relocation, not every W5 decomposition item. No semantic definitions, contracts, policy tables, source-import routes, profiles or Lean definitions changed. In particular, r0.3's exact-versus-maximum `Flow` behavior is moved intact.

## Validation and claim boundary

`cargo test -p qcore-core` passes all 83 tests independently, including the moved C11 emission compile/execute checks. The workspace passes 550 tests. Its dependency-layering guard still forbids any workspace dependency in core, and a new compatibility regression checks that representative types from all ten compiler module paths have the same type identity as the core owners. Formatting, Clippy with warnings denied, both reevaluation probes, and `lake build` pass. The Libevent submodule was initialized at the pinned `d82464a277d0f42703702c4dfd9af6af38595a83` revision, and its pin test passes.

All gate commands use this worktree's own target cache with `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, and `CARGO_PROFILE_TEST_DEBUG=0` to bound build storage. `gate-results.json` records results, with logs in this directory. Lean audits check the unchanged semantic definitions and whole-project axiom closure; no semantic parity theorem is invented for a file move.

No full 185-translation-unit audit was rerun: no source-import/profile/contract path was impacted. The evidence here establishes exact implementation/test relocation, provider-free core execution, compatibility identity and workspace regression parity. It makes no new source verdict, coverage, native observation, or raw-unit promotion claim. Historical full-audit evidence remains unchanged.
