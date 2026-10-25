# CONTRACT-LOCKS: contract pins as generated lock files

Refactoring round 1, pluggable-cards seam A, first bullet; user decision 1 of
2026-10-24 ([plan](../../plans/pluggable-cards/README.md#6-decisions)). Card
base `6c4855745`; merged with origin/main `d5d041264` before the final gate
(see the merged-base paragraph below). No analyzer, contract text, verdict or Lean change,
so no corpus identity run is due.

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
  `contract-load-path.json` records them. 186,421 bytes in all.
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
  `EXPECTED_CONTRACTS` (809 rows), `EXPECTED_INHERITANCE` (11), the module
  count literal (138) and the `librdkafka/null_deref.qv` premised count (71);
  2,829 lines become 621. A new test,
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

Before = the literal in the base revision's tests; after = the committed locks
([`compare_pins.py`](compare_pins.py), run as `python3 docs/reports/contract-locks/compare_pins.py . 6c4855745`, which parses the base literals with
`git show 6c4855745:...` and reads the locks).

| Count the old literal asserted | Before (literal) | After (locks) |
|---|---|---|
| module count (`sources.len()`) | 138 | 138 |
| `EXPECTED_CONTRACTS` declarations | 809 | 809 |
| `EXPECTED_CONTRACTS` clauses, summed | 7,607 | 7,607 |
| `EXPECTED_CONTRACTS` modules with a declaration | 138 | 138 |
| `EXPECTED_CONTRACTS` rows (module, name, clauses), in order | 809 rows | equal, row for row |
| per-module declaration and clause totals | 138 modules | 0 mismatches |
| `EXPECTED_INHERITANCE` edges | 11 | 11 |
| `EXPECTED_INHERITANCE` rows (module, child, parent), in order | 11 rows | equal, row for row |
| `librdkafka/null_deref.qv` premised declarations | 71 | 71 |
| resource-view modules, summed (Python) | 66 | 66 |
| `quirin-provider-libc` resource view (Python) | 17 | 17 |

**Merged base.** origin/main moved to `d5d041264` during the gate: the CMetrics
landing added literal rows (`cfl_list_init`, six `cmt_map_*`, two
`cmt_opts_*`) and moved the module count 138 → 140. Merge `d3e911777` keeps
this branch's readers, regenerates the cmetrics and cfl locks with `sync`,
and reproduces every one of main's literals
([`compare_pins.py`](compare_pins.py) `. d5d041264`, output in
[`compare_pins-d5d041264.txt`](compare_pins-d5d041264.txt)): 140 modules,
818 declarations, 7,721 clauses (row for row, in order), 140 modules with a
declaration, 0 per-module mismatches, 11 inheritance edges (row for row),
71 premised, 66 resource-view modules, 17 in libc.

Per crate at `6c4855745` (files / declarations / clauses): chunkio 46 / 67 / 709, cmetrics
5 / 31 / 112, ctraces 1 / 19 / 88, libevent 19 / 130 / 792, librdkafka
13 / 265 / 4,356, cfl 5 / 18 / 78, fluent-bit 26 / 111 / 656, libc
17 / 132 / 707, mpack 1 / 9 / 34, msgpack 1 / 1 / 7, shared 4 / 26 / 68.

## Strength

[`mutations.py`](mutations.py) applies six mutations one at a time, runs the
corpus test binary and `providers/contracts.py check` on each, restores the
bytes (round trip asserted) and ends with a green baseline; logs and
`summary.json` are in [`mutations/`](mutations/). All six are killed by both
commands, and the restored tree passes both:

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
from the corpus roots on disk (`scripts/contract_corpus.py`, unchanged at 138)
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

Scoped card gate, `python3 scripts/gate_scope.py --base 6c4855745`, on
`9b6cfe14f`: mode **full**, because the diff reaches `qcore-contract`, with the
provider gate and no Lean, since nothing under `proof/` changed. 13 steps,
all exit 0 ([`gate.json`](gate.json), [console](gate-console.txt); logs in the
worktree's `private/gate-scope/20261025T025407Z-254444/`):

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 checked out |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py` under the `rust-suite` slot) | 0 | 550 units, 563 summaries; 8,646 passed; 0 failed; 15 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 -m unittest discover -s scripts/tests` | 0 | 445 tests |
| `providers/package.py build` (five packages) | 0 | chunkio, cmetrics, ctraces, libevent, librdkafka |
| verify every built package, then `qcore.libevent` | 0, 0 | 5 + 1 |
| provider `cargo test --workspace --locked --offline` | 0 | 43 summaries; 133 passed; 0 failed; 1 ignored |
| provider fmt, provider Clippy | 0, 0 | |

The Rust suite and provider-test wall times (9,399 s and 7,091 s) are mostly
waits for the shared `rust-suite` and `audit` slots that other cards held. The
suite's own run phase took 491 s.

Beside the gate, on the same tree:
- `python3 providers/contracts.py check`: exit 0; 18 pin locks verified, 11
  inventory locks, 138 files, 809 declarations, 7,607 clauses, 84 provider
  inventory contracts.
- `python3 -m unittest discover -s providers/tests`: exit 0, 26 tests.
- `python3 scripts/gen_rule_docs.py --check`: exit 0 (211 rule files).
- [`mutations.py`](mutations.py): 6 of 6 mutations killed, baseline green.
