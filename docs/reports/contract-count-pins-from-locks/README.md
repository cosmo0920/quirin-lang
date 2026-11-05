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

## 2. Derivation

**Where the counts now live.** Lock schema 2 (`providers/contracts.py`
`INVENTORY_SCHEMA`, written by `sync` from `qcore-contract`'s example
`contract_inventory`, refused by `check` when it differs by a byte) adds two
fields, so every aggregate in the census is a sum of lock data:

- per `extern fn`: `resource_table`, the table of the resource-contract
  environment the declaration lands in when the v1 loader
  (`qcore_contract::contract_loader::load_resource_contract_sources`) loads its
  module alone: `functions`, `pending_review`, `relational` or `retaining`, or
  `null` when the loader refuses the module. A name in no table or in two is a
  probe error, never a guess.
- per module: `other_declarations`, the names of its declarations that are not
  `extern fn` (the 22 of census row 4: 14 `resource type`, 7 `error`, 1
  `quant axis`).

`sync` refuses a module whose resource-view membership (the provider probe's,
`qcore_audit::providers::probe_resource_views`) disagrees with its tables, so
the two loader probes must state one acceptance. The per-file pins (`sha256`,
`declaration_count`, `clause_count`, each declaration's `clauses`,
`inherits`, `premises`, `premise_bindings`) and the three views are unchanged,
and `compiler/tests/qcore_contract_corpus.rs`'s per-file check keeps its force:
it recomputes the whole lock from the tree, now including the two new fields
with its own loader call, and compares byte for byte.

**The reader.** One per language, the same fields and refusals:
`tests/support/contract_locks.rs` (`lock_totals`, `try_lock_totals`) for the
Rust tests and `scripts/contract_corpus.py` `lock_totals` for the Python
suite. Both read `providers/contract-load-path.json`, then each crate's lock,
and refuse (never sum) a lock of another schema or crate, a resource view
differing from the load path's or naming no locked file, an unknown table, or
a module whose view membership disagrees with its tables. Agreement is
mechanical: `tests/support/contract_locks_cases.json` holds one fixture (two
crates, the expected eleven totals) and eight refusals, and both
`contract_loader.rs::lock_totals_agree_with_the_shared_cases` and
`test_contract_corpus.py::test_lock_totals_agree_with_the_shared_cases` run it.

| Census row | Now asserted against |
|---|---|
| 1, 2, 12, 13 (`source_assumptions().len()`) | `resource_view_modules` (Σ resource views) |
| 8-11 (`provider_selection.rs`) | `resource_view_modules` |
| 3 (`modules.len()`) | `corpus_modules` (Σ locked files) |
| 4 (`(selected, v1, rejected)`) | `(resource_view_modules, v1_declarations, rejected_declarations)`: v1 = declarations with a table; rejected = declarations without one + `other_declarations` |
| 5, 6, 7 and the function map | `pending_review`, `relational`, `retaining`, `functions` (declarations per table); the existing sum identity against the test's own marker count stays |
| 14 (Python) | `corpus_modules`, `qcore_provider_modules`, `quirin_provider_modules` |

At `25508ccaf` the locks give exactly the former literals: 150 modules (92 +
58), 69 resource-view modules, 638 v1 and 370 rejected declarations, tables
502 / 11 / 10 / 115. So the per-module loads sum to the manifest-wide
environment's tables, which the test checks on every run.

**Same strength.** Each test still compares what it loads or walks (the
manifest's environment, the corpus walk, the marker count per declaration, the
provider selection) with a number it does not compute itself, so a loaded set
that disagrees with the locks fails. The locks are the reviewed record: the
number moves only with a lock diff inside `providers/`, which `contracts.py
check` and the per-file check refuse unless it is what `sync` writes from the
tree (hard invariant 5). The per-card history comments that accompanied the
literals are not repeated: they are in the five files at base `25508ccaf`, and
each lock diff now carries the same information as data.

**Boundaries.** A module mixing marked and unmarked declarations would fail
the derived v1/rejected pair (the lock gives none of its declarations a table,
while the test counts the marked ones), fail-closed, and needs a recorded
decision then; none exists at this base (v1 equals Σ `declaration_count` over
the views). Adding a new resource-v1 *module* still edits
`cycle19-resource-contracts.json`, outside `providers/`: that is the Cycle 19
selection, not a count pin (follow-up `CYCLE19-MANIFEST-FROM-VIEWS`). The
load-path crate counts (11 crates, 5 `qcore-provider-*`;
`provider_selection.rs`'s `crates.len() == 11`) stay literals: they count
crates, not contracts, and move only with a new crate's workspace edit.
