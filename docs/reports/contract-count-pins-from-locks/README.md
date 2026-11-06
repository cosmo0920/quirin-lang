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

**Addendum (found by acceptance 1, section 3).** One per-declaration pin, not
an aggregate count, also forced an edit outside `providers/` for every new
function-map declaration:

| # | Site | Literal | What it pins | Lock field | Changed |
|---|---|---|---|---|---:|
| 15 | `compiler/tests/contract_loader.rs`, `production_contracts_preserve_the_previous_resource_modes` | the `old` table (about 1,300 lines) | the function-map mode of every declaration in the manifest's environment; the test unwrapped `old.contract(name)`, so a new function-map declaration failed until its mode was written here | none before this card; now each `functions` declaration's `resource_mode` | most of the 78 commits touching the file |

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
- per `functions` declaration: `resource_mode`, the loaded
  `ResourceFunctionContract` serialized (its parameter modes, result and kinds;
  `null` for every other declaration). This is census row 15's pin as data.
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
| 15 (`old` modes) | every loaded function-map mode equals its lock's `resource_mode` (`function_modes`), the function map is exactly the locks', and every name `old` holds at `25508ccaf` is still compared with `old` as before |

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

Row 15 keeps the literal table: no entry is removed, and each still binds its
name. Only a declaration added after `25508ccaf` is pinned by its lock's
`resource_mode` alone, the same move CONTRACT-LOCKS made for per-declaration
clause counts (user decision 1 of 2026-10-24): the pin is a generated,
reviewed diff inside `providers/`, refused by `check` and by the per-file check
when it is not what the loader gives for the tree.

User decision (verbatim, 2026-11-06, on the integrator's question whether to
keep this move or revert `f336ef8ba`): "Keep it (rec.)". New function-map
modes are pinned in the locks.

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

## 3. Acceptance

**1. A declaration added to one provider changes no file outside
`providers/`.** On throwaway branches (never merged), two declarations were
appended to the already selected `quirin-provider-mpack` module
`mpack/writer.qv`: `qcore_scratch_count_probe_borrow` (a plain borrow, function
map) and `qcore_scratch_count_probe_retain` (`resource_retained_by(pointer,
global)`, retaining table). Then `quirin-audit pin --source-root <046accf6
checkout> --crate quirin-provider-mpack --function ...` and `python3
providers/contracts.py sync`.

- First run, `410d2e552` on `r19b-scratch-declaration-demo` (card tip
  `105f46326`): the scoped gate selected full mode; the Rust suite failed one
  unit of 611, `contract_loader::production_contracts_preserve_the_previous_resource_modes`
  (`old.contract(name).unwrap()` on the new borrow: census row 15). The run was
  stopped after the Python step; its Python failures were the two base
  failures below.
- Fix `f336ef8ba` (section 2, `resource_mode`).
- Second run, `83aab9fc9` on `r19b-scratch-declaration-demo-2` (card tip
  `f336ef8ba`): the diff is three files, all under `providers/`
  (`scratch-declaration/diffstat.txt`, `scratch.diff`): the module (+19
  lines), its `contract-inventory.lock` (two declarations, `declaration_count`
  34 -> 36, `clause_count` 122 -> 130, digest) and its `contracts.lock` (two
  pinned entries). The lock totals moved with no edit elsewhere: v1 638 ->
  640, functions 502 -> 503, retaining 115 -> 116. `python3 scripts/gate_scope.py
  --base f336ef8ba` (full mode: a `.qv` reaches the gateway's string-literal
  readers) ran every step: submodules, fmt, Clippy, the Rust suite (611 units;
  9,241 passed, 0 failed, 16 ignored), both reevaluation probes, the provider
  package build and both verifies, the provider workspace tests (133 passed,
  0 failed, 1 ignored), provider fmt and Clippy, all exit 0; the Python suite
  exit 1 with exactly the two base failures. `scratch-declaration/gate.json`
  is that run's record.

The two Python failures are the base's, not this card's: neither test reads a
file this card or the scratch changes. `test_build_matrix_sections`
expects the matrix's Lean audit count 408 to be 410 (the stale matrix
Lean-audit count the brief names as expected), and `test_gateway_registry`
finds `gateway_rule_registry.rs` stale because `gen_gateway_registry.py` orders
rule 38's Lean modules `WrittenSetDerivation` before
`CellProtocolReleaseWindowFrames` (a merge-order residue of base
`25508ccaf`'s two cards). Both fail identically on the card branch (section 4).

**2. A lock edited by hand still fails.**
`scripts/hand_edited_locks.py <checkout> hand-edited-locks` (logs and
`summary.json` in `hand-edited-locks/`) runs a green baseline, four hand edits
of `providers/quirin-provider-libc/contract-inventory.lock` written in the
lock's own byte form, each restored by rewriting its bytes with the round trip
asserted, then a green baseline again. Each edit runs the five Rust binaries
this card edits, `providers/contracts.py check` and the Python corpus suite.
See the table below (filled from `summary.json`).

| Run (lock edit) | Rust binaries (exit; failing tests) | `contracts.py check` | Python corpus |
|---|---|---|---|
| baseline before | 0 | 0 | 0 |
| `libc/threads.qv` `declaration_count` + 1 | 101; `every_contract_inventory_lock_equals_its_tree` | 1 | 0 |
| `pthread_setspecific` `retaining` -> `functions` (retaining 115 -> 114) | 101; eight tests in all four Rust files: every reader refuses a function-map declaration without a mode, and the per-file check fails | 1 | 1 (`test_repository_has_one_canonical_copy_per_pinned_module`: the reader refuses) |
| `libc/dirent.qv` removed from the resource view (69 -> 68) | 101; the same eight (the lock's view is not the load path's) | 1 | 1 (same) |
| first `libc/stdlib.qv` function-map mode loses a parameter | 101; `every_contract_inventory_lock_equals_its_tree`, `production_contracts_preserve_the_previous_resource_modes` | 1 | 0 |
| baseline after | 0 | 0 | 0 |

The harness ran at `f336ef8ba` plus this report's uncommitted text (no code
difference). An earlier run at `105f46326`, before `resource_mode`, gave the
same exits for the first three edits, except that the retaining edit then
failed `every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped`
on the retaining count itself (`left: 115, right: 114`) instead of a reader
refusal; its logs are replaced by this run's.

## 4. Card gate

`python3 scripts/gate_scope.py --base 25508ccaf --out <worktree>/private/gate`
on the card tip `8bed758f9` (`gate/gate.json`; logs under the worktree's
`private/gate/`). The diff reaches `qcore-contract` (its example) and the
providers, so the scoper selected full mode (22 packages), no Lean step, and
the provider gate.

| Step | Exit | Counts |
|---|---:|---|
| submodules, fmt, Clippy | 0 | |
| Rust suite (`run_test_binaries.py`) | 0 | 611 units, 652 summaries; 9,241 passed, 0 failed, 16 ignored, 135 filtered |
| reevaluation probe, followup | 0 | |
| Python suite | 1 | 578 tests, 2 failures: the base's two (section 3) |
| provider package build, verify-all, verify `qcore.libevent` | 0 | |
| provider workspace tests | 0 | 43 summaries; 133 passed, 0 failed, 1 ignored |
| provider fmt, Clippy | 0 | |

Wall time 5 h 35 min, almost all of it slot waits (the Rust suite step 2 h 31
min and the provider tests 3 h 00 min including their `rust-suite`/`audit`
waits behind other cards' gates and measurements; the Rust suite itself ran
18 min). The two Python failures are present without this card's diff: neither
test reads a path the diff touches.

## 5. Follow-ups and boundaries

- `CYCLE19-MANIFEST-FROM-VIEWS` (registered): a new resource-v1 *module*
  still edits `cycle19-resource-contracts.json`.
- `GATE-SCOPE-RUNTIME-READS-HEURISTIC` (observation appended): the lock
  readers compute each lock's path from `providers/contract-load-path.json`,
  so no string literal names a lock; an exact-reader scoper must declare
  them.
- Base defects seen by every gate here, not this card's: the matrix's stale
  Lean-audit count (408 vs 410) and `gateway_rule_registry.rs` out of
  generator order (rule 38's Lean list).
- Not done, by scope: the include_str! checks (R4), scoper changes, and the
  load-path crate counts (crates, not contracts).
