# W5 whole-program checking in frontend

Baseline: main `1ea51e3`. `whole_program.rs` and its unit-test file now reside physically in `crates/qcore-frontend`. The 705-line implementation and 377-line test file are byte-identical to the baseline; `file-parity.json` records both hashes without normalization.

The module already consumes frontend symbols/elaboration and core semantics only. Crate-private core aliases preserve its existing imports, with a test-only `Effect` alias for the unchanged tests. Frontend's sole dependency remains `qcore-core`; it gains no compiler, contract, schema, gateway, provider or host dependency. `frontend-dependencies.log` records the complete normal/build/test dependency tree.

The compiler's `whole_program` module and its root reexports continue to expose the same types and function. The new architecture regression checks type identity and both qualified module/root `check_whole_program` paths against the frontend function signature. The source driver and C emission orchestration remain in compiler. No semantic, contract, source-profile or Lean definition changed.

The standalone frontend suite passes 35 tests. All 552 workspace tests, formatting, Clippy with warnings denied, both reevaluation probes, `lake build`, and all 19 Lean audits pass. Libevent was initialized at pinned `d82464a277d0f42703702c4dfd9af6af38595a83`; the workspace pin test passes. Commands use this worktree's own target cache with `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, and `CARGO_PROFILE_TEST_DEBUG=0`. Gate results and logs are archived here.

No full 185-TU C audit was rerun: no existing-C import route or source-profile data changed. This evidence establishes exact relocation and API/workspace parity, not new source-verdict coverage or source/native/proof refinement. Pending chunk/schema/provider changes are outside this isolated commit.

## Read-only follow-on assessment

The next core candidate is the generic portion of `compiler/src/research_extensions.rs`: loop evidence/summary/error types and checking; `Ledger`, `LedgerMorphism`, `CategoryError`; loop/native ledger converters; and `CategoricalCertificate`/its checker. These use standard collections plus already-owned core flow/native types. A dedicated `qcore-core::resource_accounting` module can preserve all item bodies and retain compatibility through compiler reexports.

`Cycle17Input`, `Cycle17Certificate`, `ResearchExtension`, `Cycle17Error`, and `close_cycle17` must stay in compiler orchestration: they combine core work with `LegacySafetyCertificate` and the gateway's rechecker. Five generic loop/category tests and their quantity helper can move to core, while the single aggregate test remains in compiler with its Clang and contract-loader fixture. Preserve exact-versus-maximum rules and the loop-converter's independent certificate recheck. Native ledger conversion currently interprets its supplied execution record; aggregate validation obtains that record by running the native backend. Relocation should not turn that interpretation into a claim of independent source or native-provenance verification.

No research-extension code or tests were modified in this commit.
