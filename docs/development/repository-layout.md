# Repository structure and entry points

For the current consolidated map, use [architecture and entry points](architecture.md).
The migrated section below preserves the earlier development-guide checkpoint.

[Development table of contents](../../DEVELOPMENT.md)

<!-- migrated-section:start -->
## Repository layout

| Path | Contents |
|---|---|
| [`compiler/`](../../compiler) | Rust library `qcore-compiler`: parser, checker, reference semantics, C backend, existing-C compatibility API, source driver and compatibility APIs |
| [`crates/qcore-verification/`](../../crates/qcore-verification) | Trusted explicit-registry source driver, generic model/source imports, obligation derivation, witness/source-effect replay and scope dispatch over core/frontend/contract/schemas/gateway/host |
| [`crates/qcore-core/`](../../crates/qcore-core) | Semantic kernel: quantitative types, ownership, errors, cleanup, CFG, concurrency, translation, library custody and chunk lifecycle models |
| [`crates/qcore-frontend/`](../../crates/qcore-frontend) | QCore language front end: lexer, parsers, quant-index resolution, elaboration |
| [`crates/qcore-contract/`](../../crates/qcore-contract) | Declarative `.qcore` contract loader and observation interpreter |
| [`crates/qcore-schemas/`](../../crates/qcore-schemas) | Published, versioned schema surface (serde types crossing the core/provider boundary; no analysis logic) |
| [`crates/qcore-verifier-host/`](../../crates/qcore-verifier-host) | Generic registry, source scope resolution, bounded process transport, evidence firewall and composition; no provider dependency |
| [`crates/qcorec/`](../../crates/qcorec) | CLI binary `quirin` (crate `qcorec`) over the compiler library |
| [`proof/`](../../proof) | Lean 4 executable core and safety proofs; per-cycle axiom audits |
| [`contract/`](../../contract/README.md) | Shared contracts and canonical provider corpus index |
| [`docs/`](..) | Specifications, cycle records, validation and audit reports |
| `third_party/libevent` | Submodule pinned to the audited Libevent revision |

Useful entry points: the source front end (`parse_source_program`,
`elaborate_source`), the integrated checker (`check_whole_program`), the file
compiler (`compile_files`), and the CLI binary `quirin`
(`check`, `emit`, `plugins list|inspect`).

<!-- migrated-section:end -->
