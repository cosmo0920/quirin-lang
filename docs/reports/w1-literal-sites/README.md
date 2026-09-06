# W1: literal-site migration

Date: 2026-09-06. Status: the W1 literal-site migration of the
[generalization plan](../../GENERALIZATION_PLAN.md) is executed. This is a
representation migration only: no contract changed meaning, no verdict moved,
no pinned expectation was edited, and no fail-closed gate was touched. The
remaining W1 item is the richer observation/lifecycle clause vocabulary.

## What moved

Every leftover Rust-literal contract table (`ResourceContracts::new`,
`insert(_, ResourceApiKind::_)`, `insert_contract` with
`ResourceFunctionContract` literals) outside the loader's own mapping tests
now loads `.qcore` fixture modules through the total-or-reject v1
`contract_loader`:

| Site | Previous literals | Now loads |
| --- | --- | --- |
| `compiler/tests/unit/legacy_c.rs` | `q_alloc`/`q_free`/`q_transfer` base table plus seven per-test `insert_contract` extensions (rich modes, nullable, realloc, container store/remove, internal `make`) | `compiler/tests/fixtures/contracts/q_base.qcore` plus `q_rich_modes`, `q_nullable`, `q_realloc`, `q_container_store`, `q_container_remove`, `q_internal_make` |
| `compiler/tests/unit/research_extensions.rs` | `q_alloc`/`q_free` for the Cycle 17 aggregate certificate | `q_base.qcore` |
| `compiler/examples/reevaluation_followup.rs` | `q_alloc`/`q_free` for the LC01/control probes | `q_base.qcore` |
| `compiler/examples/support/legacy_audit_report.rs` (tests) | `alloc` Acquire | `alloc_acquire.qcore` |

The fixtures live under `compiler/tests/fixtures/contracts/`, not in
`contract/qcore/`: they are synthetic test APIs (`q_*`, `alloc`, `make`),
not production library contracts, so the pinned corpus counts in
`compiler/tests/qcore_contract_corpus.rs` and `contract/qcore/README.md`
are deliberately untouched. Fixture text is embedded with `include_str!` and
fed to `load_resource_contract_sources`, so the loaded environments carry
their exact source assumptions and the certificate recheck
(`check_legacy_safety_certificate` → `check_source_assumptions`) replays
them in the migrated certificate tests.

Mode fidelity: `Acquire` → `resource_return_owned(nonnull)`, `Release` →
`resource_released(p, nonnull)`, `Escape` → `resource_consumed(p, nonnull)`,
and the rich-mode literals map clause-for-clause onto the v1 vocabulary
(`resource_borrowed/owned/consumed/released/out/stored/removed`,
`resource_return_none/owned/reallocated`). The loaded
`ResourceFunctionContract` values are identical to the previous literals;
the loader's round-trip test (`contract_loader.rs::
production_contracts_preserve_the_previous_resource_modes`) pins that
mapping for the production table.

## What stayed literal, and why

- `compiler/tests/contract_loader.rs` — the loader's own unit tests
  construct literals to test the clause→mode mapping itself (including the
  deliberate certificate-mutation probe). Migrating these would make the
  tests circular.
- `crates/qcore-schemas/src/contract.rs` — the data model and the
  `From<ResourceApiKind>` compatibility mapping (definitions, not tables).
- `crates/qcore-contract/src/contract_loader.rs` — the loader's own
  construction of its output.
- `ResourceContracts::default()` sites
  (`compiler/tests/toolchain_compatibility.rs`,
  `compiler/examples/cycle17_legacy_audit.rs`) — deliberately empty
  environments; there is no table to migrate.

Nothing was blocked on the v1 vocabulary: every migrated clause was
expressible. The richer clause vocabulary (error sentinels, conditional
effects, quantitative clauses) remains the open W1 item, but no existing
literal site needed it.

## Parity

Verdict/diagnostic parity is pinned by the migrated tests themselves: every
`Safe`/`Unsafe`/`Unknown` assertion and diagnostic-kind expectation is
byte-identical to before the migration, and all pass unchanged. The
reevaluation follow-up probe reports `failed_expectations: 0` with the same
LC01/control outcomes.

## Standing gate

- `git submodule update --init --recursive`: exit 0.
- `cargo test --workspace`: exit 0 — 408 passed, 0 failed
  (compiler lib 323 + integration 45; qcore-contract 4; qcore-core 15;
  qcore-frontend 19; qcorec CLI 2). The migrated
  `legacy_audit_report` example tests (2) pass under
  `cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit`.
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --all-targets -- -D warnings`: exit 0.
- `cargo run -p qcore-compiler --example reevaluation_probe`: exit 0.
- `cargo run -p qcore-compiler --example reevaluation_followup`: exit 0.
- `cd proof && lake build`: exit 0 (51 jobs; no semantic change; Lean
  untouched).
- Contract corpus counts and the Libevent submodule pin are unchanged and
  green.
