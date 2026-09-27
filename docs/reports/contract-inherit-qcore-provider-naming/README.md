# CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: the executable provider crates on the contract load path

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING, 2026-09-25, on base `2d0b77e1`
(the CONTRACT-INHERIT-ADOPTION merge), with main `f8b41209` (the integrator's
matrix repair and adoption confirmation, docs only) merged in. It applies the
naming decision [CONTRACT-INHERIT-ADOPTION §6](../contract-inherit-adoption/README.md)
left to the user, converts the eight provider-owned allocator wrappers that
decision blocked, gives two real packages include pins, and exercises
inheritance from the derived-result parents that
[W43-LIBC-RESULT-ALIAS](../w43-libc-result-alias/README.md) added.

## Headline

* **Decision (user, binding): option B.** A contract crate's namespace is
  derived from **exactly two** package-name prefixes, `quirin-provider-` and
  `qcore-provider-`, with `-` read as `_`. There is no third prefix and no
  freely recorded namespace (option C rejected), and no crate is renamed.
  `qcore-provider-librdkafka` → `librdkafka`, `qcore-provider-libevent` →
  `libevent`, likewise `chunkio`, `cmetrics`, `ctraces`. A namespace names the
  crate that **owns** a contract: Monkey's `mk_*` contracts live in the libevent
  crate, so they are `libevent::mk_mem_alloc`. One remainder under both
  prefixes derives one namespace and is refused (`QI017`).
* **Load path: 6 → 11 crates.** The five executable crates record `NAMESPACE`
  and `PUBLISHED` (none published) in `src/main.rs`; `providers/contracts.py`
  reads them there, and excludes the conformance harness by name. Two new
  Cargo edges, `qcore-provider-{librdkafka,libevent}` → `quirin-provider-libc`,
  are pinned exactly.
* **Eight conversions, parity for each.** `rd_malloc`, `rd_calloc`,
  `rd_strdup`, `rd_strndup`, `rd_free`, `mk_mem_alloc`, `mk_mem_alloc_z`,
  `mk_mem_free` now inherit libc contracts. Every elaboration equals the
  hand-written contract token for token; the loaded records, ABI projections,
  premises and tails are equal; the Cycle 19 selection gives an equal
  environment. The census was re-run first and all eight still match.
* **First real include pins.** `qcore.librdkafka` pins
  `libc/memory.qcore` and `libc/string.qcore`, `qcore.libevent` pins
  `libc/memory.qcore`. `package.py build` installs them, `verify` accepts them,
  and the real-package mutation test refuses a changed and a missing included
  parent in both packages (6 refused mutations over 3 pins); the Rust `ContractBundle`
  does the same on the real layout.
* **Alias-result parents.** A child inherits `memcpy`, `strchr`, `strstr` and
  `gmtime_r` (base at index 1) under renamed parameters and loads to the
  parent's record, base index included. A reordered signature is refused
  (`QI010`); a parameter spelled like the predicate's second argument is
  captured (`QI012`). A same-typed **name** permutation is admitted by the
  positional rule; it is registered, not changed.
* **Arbiter: zero verdict-bearing change.** BEFORE and AFTER diagnostics are
  byte-identical to `docs/reports/w43-libc-result-alias/artifacts/`. The
  contracts sidecar moves only by the two selection entries and their eight
  chains; source-effects moves only in digests and digest-keyed bounds.
* **Lean.** `proof/QCore/ContractInherit.lean` §2 models the two prefixes:
  46 → 57 audited theorems, placeholder-free, no project axiom. The derivation
  is now also evaluated on all eleven real package names.
* **Follow-ups 299 → 298.** Retired `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`
  and `CONTRACT-INHERIT-ALIAS-RESULT-PARENTS`; registered
  `CONTRACT-INHERIT-PERMUTED-PARAMETER-NAMES`.

## 1. The naming rule

### 1.1 Derivation

`qcore_contract::contract_inclusion::derive_contract_namespace` strips the first
of `CONTRACT_CRATE_PREFIXES = ["quirin-provider-", "qcore-provider-"]` the name
carries, reads `-` as `_`, and requires an identifier (`[a-z_][a-z0-9_]*`).
The two prefixes exclude each other (Lean `prefixes_exclusive`), so the order
is never load-bearing. `ContractLoadPath::new` is otherwise unchanged: a name
with neither prefix, a recorded namespace that is not the derived one, and a
namespace two crates share are each `QI017`. The refusal text now names both
prefixes. `providers/contracts.py` has the same derivation
(`derive_namespace`) and refuses a second crate deriving an existing
namespace before any manifest is written.

| Package | Namespace | Contract-use edges |
|---|---|---|
| `qcore-provider-chunkio` | `chunkio` | `quirin-provider-shared` |
| `qcore-provider-cmetrics` | `cmetrics` | `quirin-provider-shared` |
| `qcore-provider-ctraces` | `ctraces` | `quirin-provider-shared` |
| `qcore-provider-libevent` | `libevent` | `quirin-provider-libc`, `quirin-provider-shared` |
| `qcore-provider-librdkafka` | `librdkafka` | `quirin-provider-libc`, `quirin-provider-shared` |
| six `quirin-provider-*` | unchanged | unchanged |

Every executable crate already depended on `quirin-provider-shared` (its
runtime), and that crate is on the load path, so the generator records the
edge. It grants nothing: `shared` publishes no module and re-exports none, and
visibility is not transitive.

The collision the user named, `quirin-provider-libc` beside a hypothetical
`qcore-provider-libc`, is refused in Rust (`qi017_…`, message "namespace `libc`
is also the namespace of crate …"), in Python
(`test_both_prefixes_derive_and_a_shared_namespace_is_refused`) and in Lean
(`same_remainder_same_namespace`, `shared_namespace_refused`).

### 1.2 Recording the constants in an executable crate

The executable crates have no library target, so the constants live in
`src/main.rs`, in the one spelling `contracts.py` parses
(`pub const NAMESPACE: &str = "...";`, `pub const PUBLISHED: &[&str] = &[...];`),
each marked `#[allow(dead_code)]` because the executable never reads them.
`contracts.py` picks the file from Cargo metadata: the library root if the
package has one, else its single binary root. Each crate's own test
`namespace_is_derived_and_published_modules_are_owned` checks the derivation
against `CARGO_PKG_NAME` and publication against its `contracts/` tree; a
library cannot link a binary, so the shared crate's
`load_path_manifest_matches_the_linked_crates` sets the five executable
records aside by name and pins that there are exactly five.

`qcore-provider-conformance` carries the prefix but is the test harness at
`providers/conformance`; it is excluded by name (`NON_CONTRACT_PACKAGES`), and
`contracts.py` refuses it if it ever grows a `contracts/` tree.

### 1.3 Dependency pins

`providers/conformance/tests/packages.rs`,
`artifact_dependencies_are_only_schemas_and_serialization`, is updated
deliberately: an artifact crate may now depend on a data contract crate
(never the shared bundle through that arm), and the exact edge set is pinned
to the two edges above, beside the unchanged data-crate pin
(`quirin-provider-fluent-bit` → `quirin-provider-libc`). Each new edge is
load-bearing: `every_include_names_a_published_libc_module` in both crates
links `quirin_provider_libc::PUBLISHED` and checks every directive.
`providers/Cargo.lock` gains exactly those two dependency lines.

## 2. The eight conversions

### 2.1 Census, re-run first

[`scripts/census.py`](scripts/census.py) is the adoption census with only the
admission column changed for option B. Run on an export of the base
([census-base.json](census-base.json)): 75 libc declarations, 231 provider
declarations, 46 clause-equal matches, 11 custody-bearing. All eight wrappers
still match their parent clause for clause under positional renaming; under
the new rule each is blocked only by the missing libc edge (`QI004`). The 13
new W43 derived-result libc declarations add no match: no provider contract
restates an alias- or interior-returning parent. After the card
([census.json](census.json)): 13 declarations inherit, 38 matches remain, 3
custody-bearing.

| Match | Disposition |
|---|---|
| `rd_malloc`, `rd_calloc`, `rd_strdup`, `rd_strndup`, `rd_free` ≡ `malloc`, `calloc`, `strdup`, `strndup`, `free` | **converted** |
| `mk_mem_alloc`, `mk_mem_alloc_z`, `mk_mem_free` ≡ `malloc`, `malloc`, `free` | **converted** |
| `cJSON_Delete` ≡ `free` | admitted by the rules now; **written out**: a recursive delete, not a wrapper |
| `flb_sds_destroy` ≡ `free` | admitted; written out (frees the header), unchanged |
| `cfl_sds_destroy` ≡ `free` | `QI004`; CFL's own release, unchanged |

### 2.2 The modules

`librdkafka/memory.qcore` includes `libc/memory.qcore` and
`libc/string.qcore`; `monkey/memory.qcore` includes `libc/memory.qcore`.
`rd_realloc` stays written out, because libc declares no `realloc`.
`mk_mem_alloc_z`'s body is `calloc(1, size)`, but it inherits `libc::malloc`:
the signatures must agree position by position, and the custody statement (a
borrowed size, a `nullable` owned result) is `malloc`'s exactly. The modules'
comments say so.

What the wrappers are, read from the Fluent Bit tree's copies: `rd.h` defines
`rd_malloc`/`rd_calloc`/`rd_strdup`/`rd_strndup` as the libc call followed by
`rd_assert` (with a `_strdup` arm on Windows and an `rd_malloc` + `memcpy` arm
without `HAVE_STRNDUP`), and `rd_free` as `free`; `mk_memory.h` defines
`mk_mem_alloc` as `malloc` and `mk_mem_alloc_z` as `calloc(1, n)` (NULL on
failure, `je_*` under `MALLOC_JEMALLOC`), and `mk_mem_free` as `free`. These
contracts were already trusted assumptions over those bodies before the card,
and were already selected; the conversion changes their written form, not
their meaning or their trust class.

### 2.3 Parity evidence

All in [`compiler/tests/contract_inherit_qcore_provider.rs`](../../../compiler/tests/contract_inherit_qcore_provider.rs),
against the two modules as they stood at `2d0b77e1`, archived byte for byte
under `compiler/tests/fixtures/contract_inherit_qcore_provider/`:

| Conversion | Tokens equal | Loaded record, ABI, premises, tails equal | Chain |
|---|---|---|---|
| `rd_malloc` | yes | yes | `quirin-provider-libc@0.1.0:libc/memory.qcore#malloc` |
| `rd_calloc` | yes | yes | `…libc/memory.qcore#calloc` |
| `rd_strdup` | yes | yes | `…libc/string.qcore#strdup` |
| `rd_strndup` | yes | yes | `…libc/string.qcore#strndup` |
| `rd_free` | yes | yes | `…libc/memory.qcore#free` |
| `mk_mem_alloc` | yes | yes | `…libc/memory.qcore#malloc` |
| `mk_mem_alloc_z` | yes | yes | `…libc/memory.qcore#malloc` |
| `mk_mem_free` | yes | yes | `…libc/memory.qcore#free` |

* `converted_wrappers_elaborate_to_the_hand_written_contracts`: declaration
  names and order equal, each declaration's token sequence equal (so only
  whitespace and comments differ), loaded functions, ABI projections, literal
  premises and variadic tails equal; each chain has one link whose digest is
  the libc module's text.
* `the_resource_manifest_selects_the_converted_modules_with_equal_meaning`:
  the Cycle 19 manifest, whose two entries moved from paths to
  `qcore-provider-libevent:monkey/memory.qcore` and
  `qcore-provider-librdkafka:librdkafka/memory.qcore`, loads, replays
  (`check_source_assumptions`) and publishes the chains; with the two
  hand-written modules in their place the functions and every rederived fact
  are equal.
* The corpus pins: `EXPECTED_CONTRACTS` counts elaborated clauses, so its eight
  rows are unchanged by construction; the written form is pinned in
  `EXPECTED_INHERITANCE` (5 → 13 rows). `scripts/tests/test_contract_corpus.py`
  pins the 13 libc references the placement check sees.

## 3. Packages with include pins

`python3 providers/contracts.py sync` computed the pins; `check` passes. The
inventories gain an `includes` object; nothing else in them changed:

| Package | Pins | `contracts.json` |
|---|---|---|
| `qcore.librdkafka` | `quirin-provider-libc:libc/memory.qcore` (`c7e477f4…`), `quirin-provider-libc:libc/string.qcore` (`ab20b8a8…`) | +`includes`, `memory.qcore` digest |
| `qcore.libevent` | `quirin-provider-libc:libc/memory.qcore` (`c7e477f4…`) | +`includes`, `memory.qcore` digest |
| `qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces` | none | byte-identical |

Each pin equals the load path's pin for that module, so a parent has one
identity everywhere. In the gate's package build (§6) `package.py build`
installed `includes/quirin-provider-libc/libc/{memory,string}.qcore` for
`qcore.librdkafka` and `includes/quirin-provider-libc/libc/memory.qcore` for
`qcore.libevent`, at their pinned digests, and `verify` exited 0 for all five
packages ([package-includes.json](package-includes.json)).

**Mutation controls.** `real_packages_preserve_unknown_and_reject_mutations`
now calls `included_parent_mutations` for each built package. For every pin of
a package that has pins, it appends a line to the installed parent (`verify`
must fail with "included parent changed"), deletes it (must fail with
"included parent set mismatch"), restores the original bytes, asserts the round
trip, and requires `verify` to pass again; it also pins that exactly
`qcore.libevent` and `qcore.librdkafka` carry pins. The isolated run printed
"6 changed or missing included parents refused in 2 packages"
(`measure/pisolated-nocapture/pisolated.log`, exit 0). `real_package_inventories_pin_their_included_parents`
drives the same two refusals through the Rust `ContractBundle` on a copy of
each real package layout.

Executable and archive digests move for the two packages, because each
executable embeds its `contracts.json`. No package identity is pinned by any
test, and no same-checkout base package build was run, so the executable
deltas of the other three (whose `main.rs` gained dead constants and tests
only) are not attributed here.

## 4. Derived-result parents (`CONTRACT-INHERIT-ALIAS-RESULT-PARENTS`)

W43-LIBC-RESULT-ALIAS loads both predicates to
`ResourceReturnMode::Derived { parameter, extent, nullable, const_dropped }`,
with the base **by index**, so the adoption report's condition 1 holds. The
tests build a load path from the real libc crate (its real texts; `time.qcore`
published in the test's copy only, for `gmtime_r`) and one application crate:

| Child | Parent | Loaded result, equal to the parent's |
|---|---|---|
| `app_copy(to, from, count)` | `memcpy(dest, src, n)` | `Derived { parameter: 0, Exact, nullable: false, const_dropped: false }` |
| `app_find(text, wanted)` | `strchr(s, c)` | `Derived { parameter: 0, Interior, nullable: true, const_dropped: true }` |
| `app_search(hay, pin)` | `strstr(haystack, needle)` | `Derived { parameter: 0, Interior, true, true }` |
| `app_gmtime(clock, out)` | `gmtime_r(timer, tp)` | `Derived { parameter: 1, Exact, nullable: true, const_dropped: false }` |

The elaborated text names the child's own base (`resource_return_alias(to,
nonnull)`, `resource_return_alias(out, nullable)`), and the chains replay.

* **Reordered parameters are refused.** Four children that move the base or
  another parameter (`memcpy` twice, `strchr`, `gmtime_r`) are each `QI010`:
  binding is by position, so a moved type is a signature disagreement.
* **Capture on the second argument.** A child parameter named `nonnull`
  (`memcpy`) or `nullable` (`strchr`) would be captured by the renaming and is
  `QI012`, naming the word.
* **What is not refused.** Two same-typed parameters with the parent's names
  swapped, `app_search(needle, haystack)`, are admitted: the child's **first**
  parameter is the base, whatever it is called, and the record is the
  parent's. That is C15's positional rule, and it is right for the declaration
  as written. But the brief's "a child that reorders parameters must be
  refused" holds only when a type moves; a C wrapper that really swapped two
  same-typed arguments would get a wrong trusted assumption, and nothing
  refuses a child name equal to a parent parameter at another position. This
  card does not change C15; it registers
  `CONTRACT-INHERIT-PERMUTED-PARAMETER-NAMES` (§8).

No real alias- or interior-returning wrapper is converted: the census finds no
hand-written provider contract that equals one.

## 5. Lean correspondence

`proof/QCore/ContractInherit.lean` §2 is rewritten for two prefixes, audited in
`proof/AuditContractInherit.lean` (46 → 57 `#print axioms`). The prefixes are
explicit character lists, tied to their spellings by `cratePrefix_spelling`
and `executablePrefix_spelling`, so the kernel evaluates the derivation.

| Rule | Rust / Python | Lean |
|---|---|---|
| exactly two prefixes; derivation strips the one the name carries | `derive_contract_namespace`, `contracts.py derive_namespace` | `cratePrefixes`, `stripPrefixes`, `stripPrefixes_sound`, `deriveNamespace_requires_prefix` (restated: one of the two) |
| the prefixes exclude each other | — (order irrelevant) | `prefixes_exclusive`, `isPrefixOf_append_self` |
| either prefix derives the remainder, `-` read as `_`; no `-` survives | same | `deriveNamespace_data`, `deriveNamespace_executable`, `deriveNamespace_has_no_dash` |
| a third prefix derives nothing | `QI017` | `deriveNamespace_other_prefix_none` |
| one remainder under both prefixes is one namespace, and a shared namespace fails the check | `ContractLoadPath::new` (`QI017`) | `same_remainder_same_namespace`, `shared_namespace_refused`, `namespace_names_one_crate` (unchanged) |
| the eleven real names derive the recorded namespaces; `libc` under both prefixes collides | `recorded_namespaces_equal_the_derived_ones` | `real_namespaces` (by `decide`) |

As constraint 28 states, the correspondence is maintained by review. The
identifier-shape refinement and the conformance exclusion are Rust/Python only.

## 6. Validation

Measured in this worktree (`agent-a5e7dd5583fb9ea29`) with
[`scripts/run_gate.py`](scripts/run_gate.py): every step's exit captured to a
file, never through a pipe (`CARGO_BUILD_JOBS=6`, `RUST_TEST_THREADS=2`, debug
info off). Logs are worktree-private under `measure/`. Clang was recorded in
this checkout ([clang-version.txt](clang-version.txt)).

The core and provider gates ran on `3044f204` ([gate.json](gate.json) is the
generator input). Later commits change the report, ledger, status, increment
record, index and matrix only.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | `measure/gate/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | `measure/gate/clang.log` |
| `cargo test --workspace --no-fail-fast` (first run) | **101** | 5,215 passed / **1 failed** / 11 ignored, 281 summaries | `measure/gate/test.log` |
| `cargo test --workspace --no-fail-fast` (rerun in full) | 0 | **5,216 passed / 0 failed / 11 ignored**, 281 summaries | `measure/gate-test2/test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **170** audits (`AuditContractInherit` 46 → 57 theorems) | `measure/gate/lean.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 failed / 0 ignored**, 39 summaries | `measure/gate/ptest.log` |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) | `measure/gate/pisolated.log` |
| the same, `--nocapture` | 0 | 1 passed; "6 changed or missing included parents refused in 2 packages" | `measure/pisolated-nocapture/pisolated.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/gate/pfmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/gate/pclippy.log` |
| `python3 providers/contracts.py check` | 0 | 72 provider-owned contracts; includes 0/0/0/1/2; load path current (11 crates) | `measure/gate/pcontracts.log` |
| `python3 providers/package.py build --output measure/gate/packages` | 0 | 5 packages | `measure/gate/pbuild.log` |
| `python3 providers/package.py verify …/<id>` (×5) | 0 ×5 | | `measure/gate/pverify-<id>.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | **13** tests (+1) | `measure/gate/ppython.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/contract-inherit-qcore-provider-naming/gate.json` | 0 | 298 follow-ups, 170 Lean audits, 108 contract modules | `measure/build-matrix-1.log` |
| `python3 -m unittest discover -s scripts/tests` (after the regeneration) | 0 | **330** tests | `measure/gate-final/python.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | index regenerated and matching | `measure/gate-final/docs-index.log` |

**The failed first run, disclosed.** The one failure was
`compiler/tests/std_socket_inet.rs`
`this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`, a
repository-wide `.qcore` census that pins the files its rule matches. It saw
`private/base-src/examples/sockets/inet_v{4,6}_echo.qcore`: the `git archive`
export of the base this card used to build the arbiter's BEFORE binary (§7),
kept under the ignored `private/` tree. The export was a measurement input,
not a code change; it was deleted after the BEFORE run, and the test step was
rerun **in full** in a fresh directory with exit 0. No assertion was changed.
Worktree-private scratch that contains repository files is visible to such
censuses; a later card should export elsewhere or delete it before its gate.

**Where the deltas come from** (against the base's integration gate,
`docs/reports/contract-inherit-adoption/integration/gate.json`: 5,208 over
280 summaries, 117 provider, 12 package Python):

* **Core Rust** +8, 280 → 281 summaries: the new
  `compiler/tests/contract_inherit_qcore_provider.rs` (8 tests). The edited
  tests in `contract_inherit.rs` and `contract_inherit_adoption.rs` keep their
  counts.
* **Provider Rust** +7: five `namespace_is_derived_and_published_modules_are_owned`
  and two `every_include_names_a_published_libc_module` in the executable
  crates.
* **Package Python** +1: `test_both_prefixes_derive_and_a_shared_namespace_is_refused`.
* **Load-sensitive rows.** No `P02-FLAKE` row failed. The isolated package run
  is the constraint-16 record, not a retry.

## 7. Corpus arbiter (constraint 7)

[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py),
each run with its own fresh release binary:

| Run | Tree | Binary | Exit | Record |
|---|---|---|---:|---|
| BEFORE, attempt 1 | export of `2d0b77e1` | `e123bcc0…52ee` | 1 | [before-attempt-1-failed-run.json](arbiter/before-attempt-1-failed-run.json) |
| BEFORE | export of `2d0b77e1`, libevent submodule linked | `e123bcc0…52ee` | 0 | [before-run.json](arbiter/before-run.json) |
| AFTER | `63c65b61` | `4b74f07d…9b1b` | 0 | [after-run.json](arbiter/after-run.json) |

The first BEFORE attempt ran from a `git archive` export, which carries no
submodule; the audit's Libevent pin check read the enclosing checkout's
revision and refused (`SourceRevisionMismatch`). It is retained and discarded
as evidence. The rerun linked the worktree's `third_party/libevent` (at the
pinned `d82464a2…`) into the export. Commits after `63c65b61` change tests,
formatting of tests, documentation and records only; no crate source the
audit binary links changed.

| Artifact | BEFORE vs W43 baseline | AFTER vs W43 baseline | AFTER vs BEFORE |
|---|---|---|---|
| diagnostics (verdict-bearing) | **byte-identical** (`98c44104…`) | **byte-identical** | **byte-identical** |
| chunk-contracts | byte-identical | byte-identical | byte-identical |
| source | byte-identical | byte-identical | byte-identical |
| contracts | differs (adoption's schema-2 moves, pre-existing) | differs | 2 source keys and the chains, below |
| source-effects | differs (checkout digests, pre-existing) | differs | 29 leaves, all digests, below |

Records: [before-vs-w43.txt](arbiter/before-vs-w43.txt),
[after-vs-w43.txt](arbiter/after-vs-w43.txt) (`baseline_identity.py
--root-alias`, exit 0, `verdict_bearing_identical: true` each),
[after-vs-before.json](arbiter/after-vs-before.json).

* **contracts** ([contracts-after-vs-before.json](arbiter/contracts-after-vs-before.json),
  [`scripts/contracts_parity.py`](scripts/contracts_parity.py), exit 0): every
  top-level key other than `manifest`, `sources` and `contract_inclusion` is
  equal; the manifest differs only by the two entries that became references,
  in place; of the sources, the two removed path keys hold exactly the
  hand-written fixtures and the two added reference keys hold their
  elaborations, and every other source is byte-equal; `contract_inclusion`
  keeps both BEFORE module records and adds the two converted modules with 5
  and 3 one-link chains ([after-contract-inclusion.json](arbiter/after-contract-inclusion.json)).
* **source-effects** ([source-effects-after-vs-before.json](arbiter/source-effects-after-vs-before.json),
  [`scripts/source_effects_leaves.py`](scripts/source_effects_leaves.py)):
  after the root alias, 29 leaves differ: 20 `build_digest` /
  `configuration_digest` / `task_digest` / `obligation_id` leaves, 3
  `contract_sources` digests, and 6 bounds keyed `resource_contracts:<digest>`
  whose key changed and whose value did not (checked pairwise). The
  environment digest moves because its source keys and provenance moved. None
  is verdict-bearing. (`arbiter_compare.py` counts 41 changed leaves because it
  does not pair renamed keys; the leaf script is the attribution.)

No new baseline is archived: nothing verdict-bearing moved, and
`docs/reports/w43-libc-result-alias/artifacts/` stays the baseline. A
concurrent card (W43-INDIRECT-ACCESS-GENERAL) will move that baseline; the
integrator re-arbitrates on the combined tree.

## 8. Ledger

* **Retired** ([retired-followups.json](retired-followups.json)):
  * `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`: a recorded user decision (B);
    the eight wrappers converted with the adoption card's parity test and an
    arbiter attribution (§2, §7); a real package built and verified with
    include pins (§3).
  * `CONTRACT-INHERIT-ALIAS-RESULT-PARENTS`: its discharge asked for a
    load-path test inheriting an alias and an interior parent with renamed
    parameters and equal loaded records, plus a capture control on the second
    argument (§4).
* **Registered** ([followups.json](followups.json)):
  `CONTRACT-INHERIT-PERMUTED-PARAMETER-NAMES` (§4).
* Follow-ups **299 → 298**. Still open and untouched:
  `CONTRACT-INHERIT-EXTENSION-CLAUSES`, `CONTRACT-INHERIT-NON-V1-CONSUMERS`,
  `CONTRACT-INHERIT-FLB-STRING-ASSUMPTIONS`.
* Increment record: `wave-41-continuation.md` was 36,333 bytes, over its cap,
  so this card opens `wave-41-continuation-2.md` at the record boundary and
  points the index at it.

## 9. Records and pins, each deliberate

* `crates/qcore-contract/tests/contract_inherit.rs`: the derivation table gains
  the five executable names; the refused list replaces `qcore-provider-chunkio`
  (now admitted) with names under other prefixes; the `QI017` control that used
  a `qcore-provider-` name now uses `other-provider-chunkio`, and two controls
  are added (a wrong recorded namespace under the second prefix, and the
  cross-prefix `libc` collision).
* `compiler/tests/contract_inherit_adoption.rs`: the load-path crate list
  (6 → 11), the inclusion-evidence keys (2 → 4), and the `QI017` manifest
  mutation (`qcore-provider-app` → `other-provider-app`).
* `compiler/tests/qcore_contract_corpus.rs`: `EXPECTED_INHERITANCE` 5 → 13
  rows. No module count or clause pin moved.
* `providers/conformance/tests/packages.rs`: the artifact dependency pin and
  the included-parent mutation controls (§1.3, §3).
* `providers/quirin-provider-shared/src/lib.rs`: the linked-crates check sets
  the five executable records aside and pins their names.
* `providers/tests/test_contract_load_path.py`: synthetic metadata carries
  targets, an executable crate, an other-prefix package and the conformance
  harness; one new test for both prefixes and the collision; the real-workspace
  test pins the pin counts per package.
* `scripts/tests/test_contract_corpus.py`: 13 libc references.

No assertion is weakened.

## 10. For the integrator

Files likely to conflict:

* **With main:** `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
  `docs/development/increments/README.md` (the current-record pointer),
  `docs/reports/INDEX.md`, `docs/agents/providers.md`,
  `docs/agents/diagnostics.md` (the `QI017` line), C15 and G05a.
* **With W43-INDIRECT-ACCESS-GENERAL:** the matrix and STATUS as above; if it
  edits a contract module under `providers/qcore-provider-{librdkafka,libevent}`
  or a published libc module, rerun `python3 providers/contracts.py sync`
  (the load-path pins and both packages' include pins follow the libc texts);
  `cycle19-resource-contracts.json` (two entries are references here); and the
  corpus pins in `compiler/tests/qcore_contract_corpus.rs`.
* A libc edit to `libc/memory.qcore` or `libc/string.qcore` now changes three
  inventories (`qcore.librdkafka`, `qcore.libevent` via their pins, and the
  load path); `contracts.py check` refuses a stale one.

## Reproduction

```sh
python3 providers/contracts.py sync
python3 docs/reports/contract-inherit-qcore-provider-naming/scripts/census.py <repo> <out.json>
python3 docs/reports/w43-return-expression/scripts/run_audit.py <out> <commit>
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py <out> docs/reports/w43-libc-result-alias/artifacts --root-alias <root>=<w43-root>
python3 docs/reports/contract-inherit-qcore-provider-naming/scripts/contracts_parity.py <before> <after> <out.json> --root-alias <root>=ROOT
python3 docs/reports/contract-inherit-qcore-provider-naming/scripts/source_effects_leaves.py <before> <after> <out.json> --root-alias <root>=ROOT
python3 docs/reports/contract-inherit-qcore-provider-naming/scripts/update_ledger.py   # once
python3 docs/reports/contract-inherit-qcore-provider-naming/scripts/run_gate.py measure/<run> <steps>
```
