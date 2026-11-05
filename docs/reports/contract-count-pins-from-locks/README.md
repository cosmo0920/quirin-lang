# CONTRACT-COUNT-PINS-FROM-LOCKS

Round 19, card B (2026-11-05), base `25508ccaf`. Scope: requirement R3 of
[gate-time reduction](../../plans/gate-time-reduction/README.md) and its
tracker `CONTRACT-COUNT-PINS-OUTSIDE-LOCKS`. User request (verbatim,
2026-11-05): "We need to record the requirements of saving gating time
tasks.", then "Launch three cards of gate-time reductions."

## 1. Census (before any change, at `25508ccaf`)

Every aggregate contract count asserted as a literal in a Rust or Python test.
"Changed" counts non-merge commits since 2026-10-01 whose diff adds a new value
for that literal, measured at `25508ccaf` by this directory's
`python3 scripts/pin_history.py <checkout> 2026-10-01` and, per component of
row 4, `scripts/tuple_history.py` (same arguments).

| # | Site | Literal | What it counts | Lock or manifest field stating the same quantity | Changed |
|---|---|---|---|---|---:|
| 1 | `compiler/tests/contract_loader.rs:101` (`production_contracts_preserve_the_previous_resource_modes`) | `source_assumptions().len() == 69` | modules the Cycle 19 manifest (`cycle19-resource-contracts.json`) selects, as loaded (one source assumption per module) | Σ `resource_view` over the locks (= each crate's `resource_view` in `providers/contract-load-path.json`); the manifest is that union, module for module (`provider_selection.rs`, all-loaded test) | 9 |
| 2 | `compiler/tests/contract_loader.rs:1737` (`certificates_replay_exact_assumptions_and_reject_table_mutation`) | `== 69` | the same set, replayed in a Safe certificate | as 1 | (with 1) |
| 3 | `compiler/tests/contract_loader.rs:2027` (`every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped`) | `modules.len() == 150` | tracked `.qcore`/`.qv` files under the `contract/corpus.json` roots | Σ `len(files)` over the locks (`qcore_contract_corpus.rs` already asserts the corpus is the union of the lock files) | 22 |
| 4 | same test, `(selected, v1_declarations, rejected_declarations)` | `(69, 638, 370)` | modules every declaration of which carries `resource_contract_v1`; marked declarations; unmarked declarations of every kind | selected: Σ `resource_view`; v1: Σ `declaration_count` over resource-view modules (638); rejected: Σ `declaration_count` outside the views (348) **plus 22 declarations that are not `extern fn` (14 `resource type`, 7 `error`, 1 `quant axis`), which no lock states** | 58 (selected 10, v1 39, rejected 18) |
| 5 | same test | `pending_review().len() == 11` | v1 declarations held for human review (C06.1) | **none** | 0 |
| 6 | same test | `relational().len() == 10` | v1 return-code dispositions (C04.13) | **none** | 2 |
| 7 | same test | `retaining().len() == 115` | v1 declarations withheld into the retaining table (C04.14-C04.19) | **none** | 20 |
| 8 | `crates/qcore-audit/tests/provider_selection.rs:651` | `entries.len() == 69` | Cycle 19 manifest entries, lowered to `crate:module` | Σ `resource_view` | 10 (rows 8-11 together) |
| 9 | `provider_selection.rs:685` | `from_selection.source_assumptions().len() == 69` | the provider selection's loaded modules | Σ `resource_view` | (with 8) |
| 10 | `provider_selection.rs:814` | `total == 69` | Σ of the load path's per-crate resource views | Σ `resource_view` (the locks restate the load path's views) | (with 8) |
| 11 | `provider_selection.rs:858` | `entries(manifest).1.len() == 69` | Cycle 19 manifest entries | Σ `resource_view` | (with 8) |
| 12 | `crates/qcore-c-gateway/tests/neighbours.rs:1032` | `source_assumptions().len() == 69` | as 1 | Σ `resource_view` | 9 |
| 13 | `crates/qcore-c-gateway/tests/prod_contracts.rs:940` | `source_assumptions().len() == 69` | as 1 | Σ `resource_view` | 9 |
| 14 | `scripts/tests/test_contract_corpus.py:84-86` | `150`, `92`, `58` | corpus modules; those under `qcore-provider-*` and `quirin-provider-*` roots | Σ `len(files)` over all locks, over `qcore-provider-*` locks, over `quirin-provider-*` locks | 22 |

83 distinct commits changed at least one of these literals since 2026-10-01;
78 touched `contract_loader.rs`. Not aggregate contract counts, so out of the
census: the load path's crate counts (11 crates, 5 `qcore-provider-*`) in
`providers/tests/test_contract_load_path.py`, which move only when a crate is
added, and the per-file and per-declaration pins of the locks themselves.

Three quantities (rows 4's non-function part, 5, 6, 7) are stated by no lock.
Rows 7 and 4 change often (20 and 18 times), so the card moves them into the
lock (section 2) rather than leaving literals.
