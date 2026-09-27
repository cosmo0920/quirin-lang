# W45-TEST-HYGIENE: tree-walking tests read tracked files only

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Card W45-TEST-HYGIENE, 2026-09-26, base `37e1b82d`. Rule: [validation constraint 29](../../agents/validation.md#constraint-29).

## Why

On 2026-09-26 a card left a full export of the repository under
`private/w45-fb/…/main-2f9131a8` in the user's main checkout.
`std_socket_inet`'s `this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`
walks the checkout for `.qcore` files, found the copy's
`examples/sockets/*.qcore` and failed on a correct tree. It skipped `target/`,
`.git` and nested checkouts, but nothing stopped it reading ignored `private/`.
The fragility was general. Any test that discovers inputs by walking the
working tree answers for ignored and untracked content. That content can fail
a correct tree, or let a stale copy satisfy a check.

## Mechanism

- **Rust:** `tests/support/tracked_files.rs`, included with `#[path]` where the
  repository's shared test support already lives. `tracked_files(dir)` runs
  `git -C dir ls-files -z --cached -- .` (with `GIT_DIR`, `GIT_WORK_TREE` and
  `GIT_INDEX_FILE` removed) and returns `dir/<path>` for each regular file or
  symlink, sorted. It adds no dependency.
- **Python:** `scripts/tests/tracked_files.py`, with `tracked_files` and
  `tracked_relative`, is the twin.

The index is the source, so a file added with `git add` is listed. Submodule
gitlinks are skipped (no site needs submodule content), and so is a tracked
path deleted from the working tree, which the old walks also missed.
`--recurse-submodules` is used nowhere.

**Fail-closed.** The helper panics (Rust) or raises `TrackedFilesUnavailable`,
an `AssertionError` (Python), with the reason and the card name in three
cases:

- `git` cannot run;
- the path is not in a checkout (`git ls-files` fails, as it does on an
  exported tarball);
- nothing is tracked under the path.

It never walks the filesystem instead. No test is documented as runnable
outside git, so no separate bounded walk exists.

**Contexts.** Every CI job uses `actions/checkout`, which gives a git
checkout even at depth 1. The Lean-only job runs only
`scripts.tests.test_check_lean_audits`, which uses no helper. The standing
gate runs in a worktree. `providers/package.py build` and `verify` build and
check packages but run no tests. Nothing in the standing or provider gate
runs these tests outside a checkout.

`scripts/contract_corpus.py` is the Python twin of the Rust corpus support. Its
`sources`, `libc_placement_violations` and `libc_inheritance` now take an
optional `tracked` list, which replaces the walk. The default is still the
filesystem walk. That default keeps `build_matrix.py`, the frozen report
scripts and the temporary-tree tests unchanged. `test_contract_corpus` passes
`tracked` on the real tree.

## Census

Scope: `compiler/tests` (including `support/` and `unit/`), `crates/*/tests`,
`providers/*/tests`, `providers/tests`, `scripts/tests` and `tests/support`.
In-source `#[cfg(test)]` modules that walk the tree are included too, which
covers eleven provider crates and one gateway module. Primitives searched:
`read_dir`, `WalkDir`, `os.walk`, `glob`/`rglob`, `iterdir`, `listdir`,
`scandir` and `find`. "Changes the outcome?" means whether an untracked or
ignored file could change the result before this card. The "Before" column
gives the result with the decoys below.

**Converted (C1–C14).**

| # | Site | Walks, and why | Changes the outcome? | Before |
|---|---|---|---|---|
| C1 | `compiler/tests/std_socket_inet.rs` (`this_card_reserves_no_word…`) | the whole repository, `.qcore`/`.qc`: constraint 14's census of address-host claimants | yes (the incident) | 1 test fails, both decoy kinds |
| C2 | `compiler/tests/support/contract_corpus.rs` `sources()` | the 11 corpus roots, `.qcore`: the corpus that `qcore_contract_corpus`, `contract_loader`, `contract_view_census`, `contract_inherit_adoption` and `contract_inherit_parity` read | yes: the corpus-size pin (115 against 114) and parity (110 against 111) | see set B |
| C3 | `compiler/tests/ct_corpus.rs` `fixture_files` (`ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`) | all of `compiler/tests/fixtures`: byte-identity groups with a CT fixture | yes: an undisclosed copy | 1 test fails |
| C4 | `compiler/tests/proof_leg_digests.rs` `lean_pins`, `rust_pins` | `proof/QCore/*.lean` and `compiler/tests/*.rs`, non-recursive: digest pins, cross-checked | yes: an unpinned case or a pin without its twin | 3 tests fail |
| C5 | `compiler/tests/examples_suite.rs` `family_directories`, `published_programs` | `examples/`: the family and program index | yes: a family without its page | 2 tests fail |
| C6 | `crates/qcorec/tests/examples_cli.rs` (`every_published_driver_command_line…`) | `examples/*/README.md` | yes: a directory with no page panics | 1 test fails |
| C7 | `crates/qcore-verification/tests/composition_bounds.rs` `production_sources` | `crates/*/src`, `compiler/src`: E03-B-O3's vacuity tripwire | yes: a stray `schedules_explored: Some` | 1 test fails |
| C8 | `crates/qcore-c-gateway/src/lib.rs` `census_sink_inventory` (feature `indirect-census`, CI) | `src/*.rs`: census sinks force one job | yes: an unlisted sink variable | 1 test fails |
| C9 | `providers/conformance/tests/packages.rs` (`only_template_carrying_crates_are_packaged`) | `providers/*/provider-template.json` | yes: an extra crate directory | 1 test fails |
| C10 | `compiler/tests/first_party_discovery.rs` `Fixture::new` | copies `contract/profiles` to a scratch directory for production discovery | yes in principle: the copy took untracked files. Any decoy that would change the verdict also changes production discovery of the real root (U13), so no control separates them | passes |
| C11 | `compiler/tests/contract_inherit_qcore_provider.rs` `ScratchPackage::new` | copies `providers/<crate>/contracts` into a scratch package | yes in principle. Its tests also load the real load path, which refuses a contract-root decoy (QI014) both before and after | see set B |
| C12 | `index_is_exactly_the_contract_tree` in `quirin-provider-{libc,shared,cfl,mpack,msgpack,fluent-bit}` and `contract_tree` in `qcore-provider-{chunkio,cmetrics,ctraces,libevent,librdkafka}` | the crate's `contracts/`: the hand-written index equals the tree | yes for the six index tests. The five binaries use the tree only for `PUBLISHED ⊆ owned`, which an extra file cannot break | libc index test fails (set B) |
| C13 | `scripts/tests/test_retired_scripts.py` `live_files` | the whole repository, text files: no live file names a retired script | yes. The old walk excluded `private/`, `build/`, `target/`, `.claude/` and others by name, but not unignored new directories | 1 test fails |
| C14 | `scripts/tests/test_contract_corpus.py` | `contract_corpus.sources`, `libc_placement_violations` and `libc_inheritance` on the real tree, and an `rglob` of the libc crate | yes: the module count and the libc-name check | 2 fail untracked, 1 fails ignored |

**Unchanged (U1–U14).** `test_tracked_files.py` pins these sites.

| # | Site | Walks | Why it stays |
|---|---|---|---|
| U1 | `compiler/tests/ct_corpus.rs` ~3023 | one CT case's fixture directory: inventory equals the executed inputs | a declared fixture directory; an extra file failing it is the check's purpose |
| U2 | `compiler/tests/ct_corpus.rs` `sanitized_heap_run` | `*.h` in a case's fixture directory, copied to a scratch directory | declared fixture directory |
| U3 | `compiler/tests/support/ct_atomic_publication.rs` | CT41's fixture directory: inventory | declared fixture directory |
| U4 | `compiler/tests/unit/pinned_lifecycle.rs` `copy_tree` | the vendored fixture tree, copied to be tampered with | declared fixture directory; production `load_vendored_units` itself refuses unlisted files |
| U5 | `crates/qcore-audit/tests/user_contract_plugins.rs` (2) | two fixture directories copied to scratch, then the scratch copy | fixture directories and the test's own scratch directory |
| U6 | `crates/qcore-contract/tests/contract_inherit.rs` | `tests/fixtures/contract_inherit/<crate>/contracts` | declared fixture directory |
| U7 | `providers/quirin-provider-plugin/tests/sdk.rs` `copy_tree` | the example-crate fixture, copied | declared fixture directory |
| U8 | `scripts/tests/test_w43_chunk_source_profile.py` | the vendored fixture tree, compared with its manifest | declared fixture directory (U4's Python twin) |
| U9 | `compiler/tests/std_event.rs` | the system temporary directory (Windows) | not the repository |
| U10 | `crates/qcore-core/tests/unit/capacity_boundary.rs` | a scratch directory it created | not the repository |
| U11 | `scripts/tests/test_check_lean_audits.py` | a scratch proof directory | not the repository |
| U12 | `scripts/tests/test_run_test_binaries.py` | a scratch capture directory | not the repository |
| U13 | `compiler/tests/composed_five_providers.rs` | profile directories under `contract/profiles` | mirrors production `load_first_party_manifests`, which the test calls on the same root first; a decoy directory fails that call in both forms |
| U14 | `scripts/tests/test_build_matrix_sections.py` (4) | `proof/Audit*.lean`, provider templates, contract roots | the expected side of `build_matrix.py`'s own identical reads; the equality cannot change under decoys, and converting one side would make a decoy fail it |

A fixture-directory inventory (U1, U3, U8) still reads untracked files there.
An extra file fails it, which is its purpose. A required file that exists only
untracked would satisfy a local run and fail in CI, as an `include_str!` of
an uncommitted file would.

## Controls

`scripts/decoys.py` places decoys and records their exact paths in
`private/w45-test-hygiene/decoys-SET.json`. It removes exactly those paths
and refuses to run outside `.claude/worktrees/`. `scripts/run_controls.py`
runs the converted tests one command each, with every exit captured to a file.
The decoy sets:

- **`a-untracked`:** 9 files in new directories, one per converted walk kind.
- **`a-ignored`:** the same kinds, beside a new `.gitignore` or under
  `/private/`. This set includes the incident's shape: a copy of
  `inet_v6_echo.qcore` under `private/w45-th-decoys/export/examples/sockets/`.
- **`b-untracked`, `b-ignored`:** one `.qcore` each in the libc and chunkio
  contract roots, which production loaders also read.

`git status --porcelain --ignored` showed every placed decoy as `??` or `!!`.
All decoys were removed by exact path, and the gate ran on a tree without them.

**Before (base code, scratch placement, nothing committed).**

- Set A: 9 commands fail under each of `a-untracked` and `a-ignored`: C1, C3,
  C4, C5, C6, C7, C8, C9 and C13. Every failure message names its decoy.
- Set B: the test-side failures are C2's corpus-size pin, `contract_inherit_parity`'s
  count, C12's libc index test (untracked set) and C14 (2 failures untracked,
  1 ignored).
- Logs: `private/w45-test-hygiene/before/`.

**After.**

- With no decoys, every converted test passes (`after/clean`).
- With all 26 set-A decoy files placed together, every converted test passes: 26
  commands, all exit 0 (`after/a-all`).
- With set B placed, every test-side failure is gone: `contract_inherit_parity`,
  every provider crate and `test_contract_corpus` pass. The **21** failures that
  remain, in `qcore_contract_corpus` (3), `contract_loader` (4),
  `contract_view_census` (2), `contract_inherit_adoption` (4) and
  `contract_inherit_qcore_provider` (8), all fail on the same error:
  `QI014 … owned modules differ from the contract directory: unlisted [decoy]`.
  That is the production load-path reader (`load_path_manifest::qcore_files`)
  refusing an unlisted module, fail-closed and by design. It was already
  refusing before this card. It is registered as a residual.

**Index.** Staging two decoys with `git add` made the converted `std_socket_inet`
and `test_retired_scripts` fail, naming them (`after/staged`). The decoys
were then unstaged with `git rm --cached`.

**Helper self-tests.**

- `compiler/tests/tracked_inputs.rs` (4 tests) and
  `scripts/tests/test_tracked_files.py` (4 tests) check four things in a
  scratch git repository:
  - tracked and index-only files are listed;
  - untracked, ignored and deleted files are not;
  - a non-checkout directory fails closed;
  - an all-ignored directory fails closed.
- `test_tracked_files.py`'s fifth test is rule 29's tripwire: the remaining
  walk primitives in test files must equal the U1–U14 table.
- `test_contract_corpus.py` adds a test for the `tracked` parameter.

## Prediction and outcome

The outcome matches [PREDICTION.md](PREDICTION.md) on every point:

- No analysis changed, so no corpus or slice figure moved, and the arbiter was
  not run (brief).
- Every existing test kept its name and pinned expectation. Per binary, the
  after counts equal the before passed-plus-failed counts.
- No pin counted untracked files on a clean checkout, so none changed.
- The totals grow only by the helper's own tests: one Rust test binary with 4
  tests, and 6 Python tests.
- No Lean: no semantic module changed.

## Gate

Run at `d433f18d` plus the docs of this report, logs and exits in the
worktree-private `private/w45-test-hygiene/gate/`, `CARGO_INCREMENTAL=0`.

| Step | Exit | Count |
|---|---|---|
| `git submodule update --init --recursive` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 313 units, 323 summaries: 5,957 passed, 0 failed, 11 ignored (`private/test-binaries/20260926T144629Z-460224/`) |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 101, then 0 | an unused `Path` import in `composition_bounds.rs`; fixed and rerun (`04-clippy-rerun`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 206 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 364 tests (358 + 6 new) |
| provider `cargo test` / fmt / clippy | 0 / 0 / 0 | 129 passed, 0 failed, 1 ignored |
| `providers/contracts.py check` | 0 | |
| `python3 -m unittest discover -s providers/tests` | 0 | 15 tests |
| `providers/package.py build` + `verify` of the five packages | 0 + 5 × 0 | the gate script's glob also passed the five `.tar.gz` archives to `verify`, which takes a directory (exit 1, "missing provider manifest"); an invocation error, not a result |
| `clang --version` | 0 | Ubuntu clang 21.1.8 |
| CI-only `cargo test -p qcore-c-gateway --features indirect-census --lib census_sink_inventory` | 0 | 1 passed (`after/clean-gateway`) |

## Follow-up

`W45-PRODUCTION-WALKS-READ-WORKING-TREE` covers nine production or validation
readers that tests exercise on the real tree and that read the working tree:

- `qcore_contract::load_path_manifest` `qcore_files` (QI014);
- `compiler/src/first_party.rs` profile discovery;
- `scripts/contract_corpus.py`'s default walk, which `build_matrix.py` uses;
- `build_matrix.py`'s `proof/Audit*.lean` and template globs;
- `scripts/reconciliation_sweep.py`'s report, rule and increment scans;
- `scripts/build_docs_index.py`;
- `scripts/check_lean_audits.py`'s `Audit*.lean` glob;
- `providers/contracts.py`;
- `providers/package.py`'s template glob and contract copy.
