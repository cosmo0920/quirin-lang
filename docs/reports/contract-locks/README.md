# CONTRACT-LOCKS: contract pins as generated lock files

Refactoring round 1, pluggable-cards seam A, first bullet; user decision 1 of
2026-10-24 ([plan](../../plans/pluggable-cards/README.md#6-decisions)). Card
base `6c4855745`; merged with main `c3d8ad3d4` before the final gate. No
analyzer, contract text, verdict or Lean change, so no corpus identity run is
due.

## What changed

- **`contract-inventory.lock`** in the root of each of the eleven load-path
  crates (the five `qcore-provider-*` and six `quirin-provider-*` crates that
  own `contracts/`). Canonical JSON: sorted keys, two-space indent, ASCII,
  final newline; `files` is keyed by logical path; `declarations`, `inherits`
  and `premise_bindings` keep source order, which the old tables pinned. Per
  module: `sha256` of the written bytes; `declarations` (every `extern fn`
  with the clause count of its **elaborated** contract, as the old
  `EXPECTED_CONTRACTS` counted) and the totals `declaration_count` /
  `clause_count`; `inherits` (`contract inherit` sites as written, as
  `EXPECTED_INHERITANCE` pinned them); `premises` (what
  `contract_loader::module_premises` reads, function to names and origins);
  `premise_bindings` (the `// qcore-premise-binding-manifest-v1` pins). Per
  crate: `resource_view`, `bounds_view` and `outer_view` as
  `contract-load-path.json` records them. 187,749 bytes in all on the merged tree.
- **Generation.** `providers/contracts.py sync` writes the locks and `check`
  compares them byte for byte, refusing a stale, missing or symbolic-link lock,
  a lock in a directory that is no load-path crate, and a probe that does not
  describe exactly the crate's contract files; a refusal names the modules and
  fields that differ. The parse-derived facts come from a new probe,
  `cargo run -p qcore-contract --example contract_inventory LOAD_PATH.json`
  (`crates/qcore-contract/examples/contract_inventory.rs`), which parses,
  elaborates and reads premises with the loaders' own code; the digests are
  computed in Python, independently. The `contracts.lock`,
  `bounds-contracts.lock` and `outer-contracts.lock` pin locks are untouched;
  `check` still verifies them through `quirin-audit pin --verify-contracts`.
- **Readers.** `compiler/tests/qcore_contract_corpus.rs` loses
  `EXPECTED_CONTRACTS` (818 rows on the merged base, 809 at the card base), `EXPECTED_INHERITANCE` (11), the module
  count literal (140; 138 at the card base) and the `librdkafka/null_deref.qv` premised count (71);
  main's 2,841 lines become 630. A new test,
  `every_contract_inventory_lock_equals_its_tree`, recomputes each lock from
  the tracked tree with the test's own reading (digest via
  `qcore_schemas::digest::sha256_hex`, elaborated parse, inclusion scan,
  `module_premises`, binding-pin scan, views from the load path) and requires
  value and byte equality. The four existing tests keep every semantic
  assertion (parse, round trip, compile, retired-directive refusal, the twelve
  required predicates, the three exit selectors, the closesocket arm, the named
  premise assertions) and take their expected values from the locks; the
  premised-count assertion becomes equality of the whole premise manifest,
  names and origins in order. `providers/tests/test_contract_load_path.py`
  stops asserting 66 and 17: the real-workspace test compares all three loader
  views with every lock and runs the lock check; a new synthetic test covers
  `sync`, `check` and every refusal with a stand-in probe.
- **Rules and pointers.** Rule 18 gains an amendment
  ([rules/18.md](../../agents/rules/18.md), regenerated into
  [providers](../../agents/providers.md#constraint-18)); hard invariant 5 in
  `AGENTS.md`, `providers/README.md` and the architecture map's corpus row now
  name the locks.

The per-row provenance comments of the retired tables (card, source line,
reason for each count) are not carried into the locks; they remain in
`compiler/tests/qcore_contract_corpus.rs` at `6c4855745` and in each card's
report.

## Identity: every old pin, before and after

Before = the literal in a revision's tests; after = this branch's committed
locks. [`compare_pins.py`](compare_pins.py) parses the literals with `git show
REV:...` and reads the locks (`python3 docs/reports/contract-locks/compare_pins.py
. REV`). The merged base is main `c3d8ad3d4`: the CMetrics landing
`d5d041264` added literal rows (`cfl_list_init`, six `cmt_map_*`, two
`cmt_opts_*`) and moved the module count 138 → 140, and AUDIT-CANONICAL-PATHS
changed no contract or corpus test. The merges keep this branch's readers and
regenerate the cmetrics and cfl locks with `sync`. Outputs:
[`compare_pins-c3d8ad3d4.txt`](compare_pins-c3d8ad3d4.txt) and, at the card
base, [`compare_pins.txt`](compare_pins.txt).

| Count the old literal asserted | Merged base `c3d8ad3d4`: literal | locks | Card base `6c4855745`: literal | locks |
|---|---|---|---|---|
| module count (`sources.len()`) | 140 | 140 | 138 | 138 |
| `EXPECTED_CONTRACTS` declarations | 818 | 818 | 809 | 809 |
| `EXPECTED_CONTRACTS` clauses, summed | 7,721 | 7,721 | 7,607 | 7,607 |
| `EXPECTED_CONTRACTS` modules with a declaration | 140 | 140 | 138 | 138 |
| `EXPECTED_CONTRACTS` rows (module, name, clauses), in order | 818 rows | equal, row for row | 809 rows | equal, row for row |
| per-module declaration and clause totals | 140 modules | 0 mismatches | 138 modules | 0 mismatches |
| `EXPECTED_INHERITANCE` edges | 11 | 11 | 11 | 11 |
| `EXPECTED_INHERITANCE` rows (module, child, parent), in order | 11 rows | equal, row for row | 11 rows | equal, row for row |
| `librdkafka/null_deref.qv` premised declarations | 71 | 71 | 71 | 71 |
| resource-view modules, summed (Python) | 66 | 66 | 66 | 66 |
| `quirin-provider-libc` resource view (Python) | 17 | 17 | 17 | 17 |

Per crate on the merged tree (files / declarations / clauses): chunkio
46 / 67 / 709, cmetrics 7 / 39 / 208, ctraces 1 / 19 / 88, libevent
19 / 130 / 792, librdkafka 13 / 265 / 4,356, cfl 5 / 19 / 96, fluent-bit
26 / 111 / 656, libc 17 / 132 / 707, mpack 1 / 9 / 34, msgpack 1 / 1 / 7,
shared 4 / 26 / 68.

## Strength

[`mutations.py`](mutations.py) applies six mutations one at a time, runs the
corpus test binary and `providers/contracts.py check` on each, restores the
bytes (round trip asserted) and ends with a green baseline; logs and
`summary.json` are in [`mutations/`](mutations/). All six are killed by both
commands, and the restored tree passes both (measured once on the pre-merge tree of `d17306815`; the merges change no reader):

| Mutation | Corpus test | `contracts.py check` |
|---|---|---|
| comment appended to `cfl/time.qv` (digest only) | exit 101 (tree-equals-lock) | exit 1 (`sha256`) |
| one `ensures` dropped from `msgpack/packer.qv` | exit 101 | exit 1 (the resource view changes first) |
| lock: `cfl_list_size` clauses 2 → 3 | exit 101 (two tests) | exit 1 (`declarations`) |
| lock: `flb_free` parent `libc::free` → `libc::malloc` | exit 101 (two tests) | exit 1 (`inherits`) |
| lock: a premise name in `queue_destructor.qv` | exit 101 (two tests) | exit 1 (`premises`) |
| lock: msgpack's resource view emptied | exit 101 | exit 1 (`resource_view`) |

The comment-only mutation is a strengthening: the old literals let a contract
file change without any test edit as long as no count, name or edge moved.

## The matrix

`docs/reports/remaining-d02/build_matrix.py` derives `corpus.contract_modules`
from the corpus roots on disk (`scripts/contract_corpus.py`, 140 on the merged tree, by the same walk)
and `contract_clause_pin` / `contract_modules_per_directory` from the B00
ledger, which recorded the table at the B00 checkpoint (59 modules, 100
entries) and is frozen evidence (constraint 21). None reads the retired
literals, so nothing moves and the generator is not changed. Its constant
`contract_modules_pinned_by: compiler/tests/qcore_contract_corpus.rs` stays
true: that test still pins the corpus, through the locks.

## Boundaries

- The lock is data the tests read; it verifies no contract. A wrong
  declaration with a regenerated lock passes, as a wrong declaration with an
  edited literal did; the review of the lock diff is the control.
- `providers/tests/` is still run by no gate step
  (`PROVIDER-PYTHON-TESTS-UNGATED`, now 26 tests). The standing gate's Rust
  test carries the tree-equals-lock check, byte form included.
- The corpus test reaches the contracts and locks through
  `contract/corpus.json` (a data-derived path). A diff of one contract file
  and its lock still selects `qcore-compiler` in the scoped gate, through
  other compiler tests' `providers` string literals
  (`gate_scope.py --plan --paths providers/quirin-provider-cfl/contracts/cfl/time.qv
  providers/quirin-provider-cfl/contract-inventory.lock`, measured on this
  tip), not because the scoper sees this reader.
- `compiler/tests/contract_view_census.rs`'s `EXPECTED_MULTI_MODULE` is a
  registry guard for multi-module declarations, not a count pin, and is
  unchanged.
- Card CONTRACT-SUPERSESSION extends the `quirin-audit pin` locks; this card
  does not.

## Gate

**Final scoped card gate**: `python3 scripts/gate_scope.py --base c3d8ad3d4`
on `7d7841603`, the merge of main. Mode **full**, because the diff reaches
`qcore-contract`, with the provider gate and no Lean, since nothing under
`proof/` changed. Files: [`gate.json`](gate.json), [console](gate-console.txt);
logs are in the worktree's `private/gate-scope/20261025T074258Z-1282415/`.

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 checked out |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py` under the `rust-suite` slot) | 0 | 553 units, 566 summaries; 8,672 passed; 0 failed; 15 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 -m unittest discover -s scripts/tests` | **1** | 445 tests, 1 failure (below) |
| `providers/package.py build` (five packages) | 0 | chunkio, cmetrics, ctraces, libevent, librdkafka |
| verify every built package, then `qcore.libevent` | 0, 0 | 5 + 1 |
| provider `cargo test --workspace --locked --offline` | 0 | 43 summaries; 133 passed; 0 failed; 1 ignored |
| provider fmt, provider Clippy | 0, 0 | |

The one failure is main's, not this card's.
`CommittedMatrixIsSelfConsistent.test_each_derived_dimension_matches_the_section_it_counts`
failed with `830 != 832`: main `c3d8ad3d4` committed two ledger entries without
regenerating the matrix's dimensions. This branch changes neither the matrix
nor `scripts/`. Main's next commit, `710021bd7`, regenerates the matrix and
touches only records. Merge `99a664a9b` brings it in. On that tip the Python
suite exits 0 with 445 tests ([log](python-final.log)). No other gate step reads
the files that merge changes, so no other step was rerun.

The Rust suite and provider-test wall times (2,745 s and 2,959 s) are mostly
waits for the shared `rust-suite` and `audit` slots that other cards held.

**Earlier gate, before main moved**: the same command with `--base 6c4855745`
on `9b6cfe14f`. All 13 steps exited 0: Rust suite 550 units, 8,646 passed,
0 failed, 15 ignored; Python 445; provider test 133 passed
([`gate-9b6cfe14f.json`](gate-9b6cfe14f.json),
[console](gate-9b6cfe14f-console.txt)). A second attempt with `--base
d5d041264` was stopped before its Rust suite ran, because local main had moved
again.

Beside the gate, on the merged tree:
- `python3 providers/contracts.py check`: exit 0; 18 pin locks verified, 11
  inventory locks, 140 files, 818 declarations, 7,721 clauses, 86 provider
  inventory contracts.
- `python3 scripts/gen_rule_docs.py --check`: exit 0 (213 rule files).
- `python3 -m unittest discover -s providers/tests`: exit 0, 26 tests.

Before the merges:
- [`mutations.py`](mutations.py): 6 of 6 mutations killed, baseline green.
