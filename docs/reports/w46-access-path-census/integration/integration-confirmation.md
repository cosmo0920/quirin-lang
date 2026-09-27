# Integration confirmation (measurement card, no corpus movement)

The integrator re-ran this card on 2026-09-27, on merge commit `fa9823d9`
(`integrate/w46-census`, card tip `1cb85a0b` merged onto main `37c59235`).
Main's two commits ahead of the card's own base (`5c9d05a9` and `37c59235`)
are docs-only w46-contract-gap-expansion records; the report index was
regenerated in the merge. This card is a measurement card (MOVES=no): the
census stream (`access_path_census.rs` and its two hooks) runs only behind
the `indirect-census` feature and changes no analyzer, contract or Lean
source, so the corpus is required to stay verdict-bearing identical.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. Full figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| `git submodule update --init --recursive` | 0 | — |
| core `cargo test --workspace --no-fail-fast` | 0 | **6,021 passed / 0 failed / 11 ignored** over 334 strict `^test result: ` summaries |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `reevaluation_probe` example | 0 | — |
| `reevaluation_followup` example | 0 | — |
| `check_lean_audits.py` | 0 | **210** Lean audits (`Passed 210 Lean audits`) |
| `python3 -m unittest discover -s scripts/tests` | 0 | **364** tests, OK |
| `build_docs_index.py --check` | 0 | clean |
| `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings` | 0 | clean |
| `cargo test -p qcore-c-gateway --features indirect-census --lib` | 0 | **598 passed / 0 failed / 0 ignored** over 1 strict summary |
| `cargo clean --profile dev` | 0 | — |
| provider workspace `cargo test --locked --offline --no-fail-fast` | 0 | **129 passed / 0 failed / 1 ignored** over 43 strict summaries |
| release build, `cycle19_fluent_bit_libraries_audit` | 0 | binary at `fa9823d9` |
| arbiter run (`run_audit.py`) | 0 | see below |
| identity step 32 (`baseline_identity.py`) against `docs/reports/w46-external-callee-contracts/integration/artifacts` | 0 | `verdict_bearing_identical: true` |

No load-sensitive row needed a rerun; every step passed on the first run.

## Identity

`docs/reports/w46-access-path-census/integration/identity-vs-external-callee-contracts-baseline.json`
records the comparison: `diagnostics`, `chunk-contracts` and `source`
(`monkey-source.json`) are byte-identical; `contracts` is identical after the
recorded checkout-root alias; `source-effects` differs only in
checkout-bound digests, exactly as at the card's own baseline check reported
in [the card's report](../gate.json). `verdict_bearing_identical: true`.

Arbiter figures, unmoved against the `w46-external-callee-contracts` baseline:

| | baseline | this integration |
|---|---|---|
| functions (Safe / provisional Unsafe / Unknown) | 405 / 9 / 4,420 | 405 / 9 / 4,420 |
| units (Safe / provisional Unsafe / Unknown / data-only) | 5 / 8 / 168 / 4 | 5 / 8 / 168 / 4 |
| leak | 34 | 34 |
| unknown_alias | 3,572 | 3,572 |
| unsupported | 41,239 | 41,239 |
| summary_conflict | 1 | 1 |
| Pending | 0 | 0 |

No baseline archive is written here: MOVES=no for this card, and the identity
step confirms the corpus did not move, so `docs/reports/w46-external-callee-contracts/integration/artifacts`
remains the current baseline.
