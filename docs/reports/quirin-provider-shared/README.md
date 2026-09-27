# QUIRIN-PROVIDER-SHARED: the providers' shared assets become real crates

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Card QUIRIN-PROVIDER-SHARED, 2026-09-23, on base `df60bc78`. This is a
parity-preserving structural refactor. The providers' shared contract data
(`providers/shared/contracts/`) and their `#[path]`-included Rust
(`providers/common/`) move into six library crates in the `providers/`
workspace. No contract byte, verdict, capability or provider behaviour changes.

The brief was amended mid-card, at the user's direction via the integrator:
the shared contracts are split per library rather than kept in one catch-all
crate. The branch history shows only the final layout, because the
intermediate single-crate commits were rewritten on this card's own branch
before any gate measurement.

## Headline

* **Six crates, one pure-move commit.** `7fb59054` renames 50 files and changes
  no content (`git diff -M` reports 0 insertions and 0 deletions). The
  wiring commit `aebd76a5` then adds the manifests and updates every
  consumer. All 24 shared `.qcore` sources, all 13 projection fixtures and three
  of the eight Rust modules are byte-identical at the tip. The other five Rust
  modules and the four test side files differ only in a deleted `#[path]` line,
  re-pointed `include_str!` paths, one doc-comment sentence and two deleted
  `#[allow(dead_code)]` lines (see [moved files](#moved-file-identity)).
* **`#[path]` attributes, 36 before and 15 after.** All 17 includes from
  provider and conformance binaries into `providers/common` are gone, and so
  are the 4 side-file test includes. The 15 includes into
  `crates/qcore-verification/tests/fixtures/` remain on purpose and are
  registered as `PROVIDER-CONFORMANCE-CORE-FIXTURE-PATHS`. The brief's
  integrator figures were 18 / 4 / 13; the measured figures are 17 / 4 / 15,
  and the total is 36 either way.
* **Arbiter.** On the pinned 185-unit corpus, the verdict-bearing diagnostics
  are **byte-identical** to the W43 baseline. chunk-contracts and source are
  also byte-identical. source-effects is byte-identical to this worktree's own
  pre-move run, so its W43 difference is the pre-existing checkout-path digest
  difference the integrator already itemised. The contracts artifact differs
  **by path only**: 18 moved source keys hold identical texts, and after
  rewriting the new crate paths back, it equals the pre-move artifact exactly.
* **Packages.** All five packages build and verify. Their executable, manifest
  and archive digests change, while their contract inventories and embedded
  declarations are identical. Each change is attributed to the crate boundary
  ([package identities](#package-identities)). No identity is re-pinned.
* **Follow-ups 282 → 283.**

## Crate map

The providers workspace has twelve members, up from six. The new crates are
`publish = false` libraries. None of them is an executable package, and
`package.py` still packages exactly the five `qcore-provider-*` crates.

| Crate (lib) | Owns | Rust API | Cargo dependants |
|---|---|---|---|
| `quirin-provider-libc` (`quirin_provider_libc`) | `contracts/libc/`: `errno`, `memory`, `pthread_mutex` (3). The **only** home for libc contracts. It has no dependency and is usable without the bundle. | `CONTRACTS`, `contract(path)` | `quirin-provider-shared` |
| `quirin-provider-cfl` (`quirin_provider_cfl`) | `contracts/cfl/`: `arena_lifetime`, `intrusive_list`, `sds_lifetime`, `time` (4) | same | `quirin-provider-shared` |
| `quirin-provider-mpack` (`quirin_provider_mpack`) | `contracts/mpack/writer` (1) | same | `quirin-provider-shared` |
| `quirin-provider-msgpack` (`quirin_provider_msgpack`) | `contracts/msgpack/packer` (1) | same | `quirin-provider-shared` |
| `quirin-provider-fluent-bit` (`quirin_provider_fluent_bit`) | `contracts/fluent_bit/` (12: hash-table publication, the six input-chunk modules, logging, memory, resource memory, routes mask, time) | same | `quirin-provider-shared` |
| `quirin-provider-shared` (`quirin_provider_shared`) | `contracts/language/quantified_indices`, `contracts/library/{custody_evidence,source_projection}` (3). Rust modules `runtime`, `source_effect`, `counter_effect`, `teardown_effect`, `call_use_effect`, `cleanup_replay_effect`, `composition_effect`, `reactor_callbacks`. `fixtures/` (13). | its own `CONTRACTS`; re-exports `libc`, `cfl`, `mpack`, `msgpack`, `fluent_bit`; `CONTRACT_SETS`, `shared_contracts()`, `shared_contract(path)` | the five `qcore-provider-*` crates and `qcore-provider-conformance` |

Each contract crate's `CONTRACTS` index is `(logical path, source text)` pairs
built with `include_str!`. A unit test pins the index to the crate's own
`contracts/` tree, byte for byte, so a module added on disk without an index
entry fails the test and cannot be silently omitted. That matters for the
coming libc contract card. The bundle test requires the 24 names to be
disjoint and each crate's names to stay under its own library directory, so a
libc module cannot sit in the Fluent Bit set.

Consumers, from [crate-map.json](crate-map.json), which
[`scripts/crate_map.py`](scripts/crate_map.py) regenerates. Every contract
module is also selected through its crate's `contracts/` root in
`contract/corpus.json`.

* **libc.** The resource manifest (`cycle19-resource-contracts.json`) selects
  all three modules. `compiler/tests/contract_loader.rs` and
  `docs/GENERALIZATION_PLAN.md` name `libc/memory`.
* **cfl.** The resource manifest selects `sds_lifetime` and `time`, and the
  Emacs mode test reads `intrusive_list`. `arena_lifetime` is reached through
  the corpus only.
* **mpack, msgpack.** The resource manifest selects one module from each.
* **fluent-bit.** The resource manifest selects 11 of the 12 modules. The C
  gateway tests `hash_contracts`, `neighbours` and `prod_contracts` embed
  modules with `include_str!`. `compiler/tests/std_string.rs` and three
  frontend comments name modules. `memory` is read by
  `scripts/cycle19_native_observations.py` and two docs.
* **shared (contracts).** `library/source_projection` is embedded by the C
  gateway (`typed_source.rs`) and one gateway test, the Cycle 19 compatibility
  example, five `qcore-verification` tests and three of its fixtures, a compiler test, two
  conformance tests, the CMetrics counter profile and
  `scripts/prepare_cmetrics_clang_audit.py`. `language/quantified_indices` is
  named by a compiler test and a doc. `library/custody_evidence` is reached
  through the corpus only.
* **shared (Rust).** Every provider `main` and the two conformance fixture
  binaries use `runtime`. `source_effect` is used by `chunkio`, `cmetrics` and
  both conformance binaries. `counter_effect`, `teardown_effect`,
  `call_use_effect`, `cleanup_replay_effect` and `composition_effect` are used
  by `chunkio`, and `reactor_callbacks` by `libevent`.
* **shared (fixtures).** The four side-file unit test modules and eight
  conformance tests read the fixtures. `crates/qcore-verification/tests/emit_cleanup_replay_fixtures.rs`,
  the opt-in scratch generator, writes three of them.

**Why providers depend on `quirin-provider-shared` rather than narrower
crates.** Every provider needs `runtime`, and every evaluator lives in the
shared crate, so no narrower crate would serve a binary. Linking the whole
library does not change what a binary contains or does. Each `main` names
exactly the evaluators it chained before, in the same order. The linker keeps
only what is referenced: the per-module symbol table of every executable is
the same before and after (ctraces and librdkafka define only `runtime`,
libevent only `runtime` and `reactor_callbacks`, cmetrics `runtime` and
`source_effect`, and chunkio the seven it calls), and no shared contract text
is embedded in any executable. Cargo features were considered and not used.
One `package.py` invocation builds all five packages, so feature unification
would compile the union anyway, and the capability surface is already fixed by
the call graph, which the symbol check measures.

**Layering.** No core crate depends on any of the six. The core pin
`compiler/tests/workspace_layering.rs` now refuses a `quirin-provider*` name as
well as `qcore-provider*`, which strengthens it. It already refused any
`providers/` path. Core consumers still reach shared contracts only as file
data (`include_str!` or runtime paths), as the precedent
[canonical-contract-locations](../canonical-contract-locations/README.md)
established. The provider-side pin (`conformance/tests/packages.rs`,
`artifact_dependencies_are_only_schemas_and_serialization`) now admits
`quirin-provider-shared` as an artifact's only first-party dependency. It holds
that crate to schemas, serialization and its five sibling crates, requires the
five contract crates to have **no** dependencies, and counts six shared crates.

## Structure before and after

| | Before (`df60bc78`) | After (`aebd76a5`) |
|---|---|---|
| Shared contracts | `providers/shared/contracts/` (plain directory, 24) | five per-library crates (21) + `quirin-provider-shared` (3) |
| Shared Rust | `providers/common/*.rs`, compiled into each binary separately | `quirin-provider-shared/src/`, one library |
| Side-file tests | `common/<m>_tests.rs` via `#[path]` | `src/<m>/tests.rs`, ordinary `mod tests;` |
| Fixtures | `providers/common/fixtures/` | `quirin-provider-shared/fixtures/` |
| Providers workspace members | 6 | 12 |
| `contract/corpus.json` roots | 6 | 11 (logical names unchanged) |
| `providers/shared/`, `providers/common/` | present | removed |

`#[path]` attributes under `providers/`, measured by `grep -rn '#\[path' providers --include='*.rs'`:

| Group | Before | After |
|---|---:|---:|
| Provider binaries → `providers/common` (chunkio 7, cmetrics 2, ctraces 1, libevent 2, librdkafka 1) | 13 | 0 |
| Conformance fixture binaries → `providers/common` (`configured_audit_provider` 2, `sample_boolean_provider` 2) | 4 | 0 |
| `common/<m>.rs` → `<m>_tests.rs` | 4 | 0 |
| `conformance/tests/*.rs` → `crates/qcore-verification/tests/fixtures/` (13 files, 11 modules) | 15 | 15 |
| **Total** | **36** | **15** |

## Moved-file identity

[moved-files.json](moved-files.json) lists all 50 renames with the base blob,
the SHA-256 of its bytes and the SHA-256 at the tip. 40 are byte-identical at
the tip: the 24 `.qcore` modules, 13 fixtures, `source_effect.rs`,
`counter_effect.rs` and `reactor_callbacks.rs`. The other 10 carry only these
edits:

| File (new path under `providers/quirin-provider-shared/`) | Edit |
|---|---|
| `src/{teardown,call_use,cleanup_replay,composition}_effect.rs` | the `#[path = "<m>_tests.rs"]` line deleted (`#[cfg(test)] mod tests;` kept) |
| `src/<m>/tests.rs` (4) | `include_str!("fixtures/…")` → `"../../fixtures/…"`; `"../qcore-provider-chunkio/provider-template.json"` → `"../../../…"`; rustfmt re-wrapped one line |
| `src/runtime.rs` | module doc "Included as source by each artifact crate" → "Linked by each artifact crate from `quirin-provider-shared`"; two `#[allow(dead_code)]` deleted (the functions are `pub` in a library) |
| `README.md` | rewritten to describe the crate |

All 96 logical contract modules keep their names and bytes: their
`contract_corpus.sources()` digests equal the base's, and 24 physical paths
changed. `compiler/tests/qcore_contract_corpus.rs` is not edited, and its
module, declaration and clause pins pass unchanged. **No pin changed.** The
only count assertion that moved is `scripts/tests/test_contract_corpus.py`.
It identified the 24 shared modules by a `shared` path component, and now
identifies them by the `quirin-provider-` crate prefix, still requiring 24. It
also gains an assertion that every `libc/` module is in `quirin-provider-libc`.

## Consumer updates

| Consumer | Change |
|---|---|
| `contract/corpus.json` | shared root replaced by six crate roots |
| `cycle19-resource-contracts.json` | 18 paths re-pointed; selection text otherwise unchanged (33 modules) |
| `crates/qcore-c-gateway/src/typed_source.rs` and gateway tests (5 files) | `include_str!` paths |
| `crates/qcore-verification/tests` (5 tests, 3 fixtures, the fixture emitter) | `include_str!` and fixture-directory paths |
| `compiler/examples/support/cycle19_compatibility.rs`, compiler tests (4) | paths |
| `crates/qcore-frontend` (3 comments), `contract/profiles/cmetrics-counter/audit.json`, `editors/emacs/qcore-mode-tests.el` | paths |
| `providers/*/Cargo.toml`, `providers/Cargo.toml`, `providers/Cargo.lock` | new members and workspace dependencies; lock change is additive (6 packages, 6 dependency lines) |
| `providers/*/src/main.rs`, `conformance/src/bin/*_provider.rs` | `#[path] mod` → `use quirin_provider_shared::{…}` |
| `providers/conformance/tests/*.rs` (10) | fixture and contract paths |
| `providers/conformance/tests/packages.rs`, `compiler/tests/workspace_layering.rs` | dependency pins as described above |
| `providers/contracts.py` | comment only (ownership map unchanged; the check still reports 72) |
| `scripts/cycle19_native_observations.py`, `scripts/prepare_cmetrics_clang_audit.py` | paths |
| `docs/reports/corpus-repin/diff_artifacts.py` (the live constraint-22 itemiser) | `contract_module` also maps `providers/quirin-provider-*/contracts/` into its comparison namespace; the old `shared` spelling is kept so diffs across this move still compare; test extended |
| Live docs | `AGENTS.md` invariant 5, `docs/agents/providers.md` constraint 18 (paths only), `providers/README.md`, `contract/README.md`, `docs/STATUS.md`, `docs/development/architecture.md`, `docs/GENERALIZATION_PLAN.md`, `docs/QCORE_MODULE_MODES.md`, `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`, `docs/QCORE_SPEC_r0_3.md`, `docs/cycles/CYCLE_19.md`, four design notes |

Frozen reports and increment records are unedited, and so are their
historical path mentions. No Lean file references these paths, so Lean is
untouched.

## Arbiter

`docs/reports/w43-return-expression/scripts/run_audit.py` ran twice in this
worktree with fresh release builds of `cycle19_fluent_bit_libraries_audit`. The
BEFORE run was on base `df60bc78` ([before-run.json](before-run.json), exit 0)
and the AFTER run on `aebd76a5` ([after-run.json](after-run.json), exit 0).
`baseline_identity.py … --root-alias <this worktree>=<W43 worktree>` against
`docs/reports/w43-return-expression/artifacts/`, with AFTER also given
`--reference` BEFORE, exited 0 both times:

| Artifact | BEFORE vs W43 | AFTER vs W43 | AFTER vs BEFORE |
|---|---|---|---|
| diagnostics (verdict-bearing) | byte-identical | **byte-identical** | byte-identical |
| chunk-contracts | byte-identical | byte-identical | byte-identical |
| source | byte-identical | byte-identical | byte-identical |
| source-effects | differs (checkout path) | differs (checkout path) | **byte-identical** |
| contracts | identical after root alias | differs | differs by path only |

* **source-effects.** `json_leaf_diff.py` finds 41 differing leaves against W43
  in both BEFORE and AFTER. That is the same count and the same key names the
  integrator attributed to checkout-path digests in
  [integration-confirmation](../w43-return-expression/integration-confirmation.md).
  AFTER vs BEFORE has 0 differing leaves, so this card moves none of them.
* **contracts.** AFTER vs BEFORE has 37 differing leaves: 18 removed and 18
  added source keys, plus the embedded `manifest` text.
  [`scripts/contracts_parity.py`](scripts/contracts_parity.py) rewrites
  `providers/quirin-provider-*/contracts/` back to `providers/shared/contracts/`
  and gets a document **equal** to BEFORE. The 18 moved texts are identical,
  and the constraint-22 itemiser reports 0 added, 0 removed, 0 changed logical
  modules and no key collision ([contracts-parity.json](contracts-parity.json)).

Identity records: [before-identity.json](before-identity.json),
[after-identity.json](after-identity.json). No function or unit status moves.
No count changes.

## Package identities

`python3 providers/package.py build` ran once at base and once at the wiring
tree, in this worktree with the same default target directory, Rust 1.98.0 and
exit 0 both times ([packages-before.json](packages-before.json),
[packages-after.json](packages-after.json)).
[`scripts/package_identity.py`](scripts/package_identity.py) checks each
package from its bytes and exits 0 ([package-identity.json](package-identity.json)).

| Package | Executable before → after | Declarations | Shared modules with symbols (same both sides) |
|---|---|---|---|
| `qcore.chunkio` | `faf3541a…` → `6d7fcecb…` | identical | 7 (all but `reactor_callbacks`) |
| `qcore.cmetrics` | `b9ff0c7a…` → `706fdd99…` | identical | `runtime`, `source_effect` |
| `qcore.ctraces` | `21c34f98…` → `93dbc6a2…` | identical | `runtime` |
| `qcore.libevent` | `d19249fd…` → `90676403…` | identical | `runtime`, `reactor_callbacks` |
| `qcore.librdkafka` | `ad0d2e7e…` → `56e87a78…` | identical | `runtime` |

For every package, the script asserts the following:

* `contracts.json` and every `contracts/` asset are identical, so
  `contracts_manifest_sha256` is unchanged.
* `verifier-plugin.json` differs in `package_digest` only.
* Both embedded envelopes (manifest template and contract inventory) are
  byte-identical.
* The set of shared modules with defined symbols is the same, and so is the
  set of moved-module source paths spelled in panic locations: before as
  `<crate>/src/../../common/<m>.rs`, after as
  `quirin-provider-shared/src/<m>.rs`.
* None of the 24 shared contract texts is embedded.

Attribution follows from these checks.

* **Executable digests** change because the same code is now compiled in a
  library crate. Symbol names carry `quirin_provider_shared` instead of the
  binary's crate, and the panic-location paths change spelling. Codegen-unit
  partitioning and cross-crate inlining also move which helpers are separate
  symbols. Crate-local functions differ by at most one per binary: cmetrics
  inlines one `source_effect` closure it kept before; ctraces and librdkafka
  now keep `runtime::run_with_contracts` out of line; libevent keeps its
  `propose`. About 830–850 generic instantiations of std and serde code are
  placed differently. These layout effects are disclosed, not asserted.
* **`manifest_sha256`** changes only through the recomputed `package_digest`.
  **`archive_sha256`** changes because the archive contains the executable and
  manifest.
* **`lock_sha256`** changes from `3217bd0e…` to `95cc9b61…` because of the
  additive `Cargo.lock` entries for the six new crates.

Behavioural parity is tested, not inferred from the digests. The package
conformance tests and the other 16 conformance targets pass (109/0/0), and
`package.py verify` exits 0 for all five packages. Separately, the base build
here does not reproduce the identities recorded in
[task-context-contract-bundle/package-parity.json](../task-context-contract-bundle/package-parity.json),
even though no provider source changed between that record and `df60bc78`.
That record came from a different checkout, and every comparison above is
same-checkout. This card does not claim cross-checkout reproducibility.

## Validation

Measured in this worktree (`agent-a982e29dfe882f8ec`). The core gate ran on
`aebd76a5`, and later commits change documents, the ledger and report scripts
only. Every step's exit was captured to a file, never piped, with
[`run_gate.py`](../w43-return-expression/scripts/run_gate.py)
(`CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=2`). Logs are worktree-private under
`measure/`.

| Command | Exit | Result | Log |
|---|---|---|---|
| `git submodule update --init --recursive` | 0 | | `measure/gate/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt)) | `measure/gate/clang.log` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,117 passed / 0 failed / 12 ignored**, 272 summaries | `measure/gate/test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **166** audits | `measure/gate/lean.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final matrix regeneration) | 0 | **317** tests | `measure/python-final.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 failed / 0 ignored**, 39 summaries | `measure/provider-gate/test.log` |
| `cargo test --manifest-path providers/Cargo.toml -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations --locked --offline -- --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) | `measure/provider-gate/isolated-packages.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/provider-gate/fmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/provider-gate/clippy.log` |
| `python3 providers/contracts.py check` | 0 | 72 provider-owned | `measure/provider-gate/contracts-check.log` |
| `python3 providers/package.py build --output measure/after/packages` | 0 | 5 packages | `measure/after/package-build.log` |
| `python3 providers/package.py verify measure/after/packages/<id>` (×5) | 0 ×5 | | `measure/provider-gate/verify-<id>.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 5 tests | `measure/provider-gate/package-python.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/quirin-provider-shared/gate.json` | 0 | 283 follow-ups | `measure/build-matrix.log` |
| `python3 scripts/build_docs_index.py` | 0 | index regenerated | |

**Core counts are unchanged.** 5,117 / 0 / 12, 166 and 317 equal the
integrator's combined-tree figures at `e469b8be`. This card moves core tests'
paths only.

**The provider count drops from 123 to 109, and no test is lost.** Before,
each capability evaluator's unit tests ran once in every binary that
`#[path]`-included the module: `source_effect`'s 7 tests ran 4 times (chunkio,
cmetrics and the two conformance fixture binaries). The 59 distinct evaluator
tests, totalling 7 + 2 + 7 + 6 + 9 + 25 + 3, made 80 executions. They now run
once, in `quirin-provider-shared`. The card adds 7 contract-index tests, one
per contract crate plus the bundle test. The integration tests are unchanged
at 43, and 43 + 59 + 7 = 109. The 123 is derived as 43 + 80 and matches the
carried figure. It was not remeasured on the base.

No `P02-FLAKE` row failed in the full runs. The isolated package-mutation run
above is the constraint-16 record, not a retry.

## Remaining work

* **`PROVIDER-CONFORMANCE-CORE-FIXTURE-PATHS`** (registered): the 15 core
  fixture includes, with the proposed core-workspace test-support crate as
  their home.
* **Not registered.** No consumer reads the new `CONTRACTS` APIs yet. Core
  crates may not depend on provider crates, so core consumers keep selecting
  shared modules as file data. The APIs serve provider-workspace consumers,
  such as the coming libc contract card, which adds modules under
  `quirin-provider-libc/contracts/libc/` and the matching `CONTRACTS` entries.
