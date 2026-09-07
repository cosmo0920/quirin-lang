# G01 — compile-task identity and dependency contract — 2026-09-07

Work card G01 (W2/W3, verify-or-extend). Scope: the identity and dependency
contract of one accepted C import configuration in the configured audit, its
gateway transport, and the evidence keyed on it.

This is a bounded identity/dependency contract for the *supported* configuration
route. It is not build hermeticity, not compiler equivalence, and it promotes no
raw Cycle 19 unit.

## 1. What was already bound (verified, not re-implemented)

The evidence layer already carried a strong import identity.
`BooleanSourceIdentity` (`crates/qcore-c-gateway/src/boolean_source.rs`) binds:

| Component | Field |
|---|---|
| Selected source bytes | `raw_source_digest` |
| Effective preprocessing input | `preprocessed_source_digest` |
| Dialect, legacy include paths, legacy definitions, analyzed function scope, process working directory, compiler description, and — when nonempty — the ordered typed `CPreprocessorOption` sequence including include classes and the four admitted frontend flags | `configuration_digest` (+ `dialect`) |
| Contract rule text | `source_contract_digest` |
| Loaded profile | `profile_digest` |
| Selected model library | `model_library` |
| Declared upstream revision | `source_revision` |
| Clang program, version and default target | `compiler` |
| Recursive dependency closure of imported frame helpers | `helpers` |

`qcore-verification`'s `BooleanFieldSourceTask::derive`
(`crates/qcore-verification/src/source_effect.rs`) hashes that whole identity
into `UnitIdentity::build_digest` and into `contract_sources`, so the evidence-v2
`task_digest` retires whenever any of it changes, and
`BooleanFieldSourceTask::check` refuses a witness whose `task_digest` or summary
does not match.

Existing tests that already pinned parts of this contract, confirmed still
passing and left unchanged:

- `compiler/tests/unit/chunk_source.rs::checked_source_identity_binds_inputs_configuration_profile_and_policy`
- `compiler/tests/unit/chunk_source.rs::checked_helper_build_identity_survives_source_import_and_raw_record_parity`
- `crates/qcore-verification/tests/source_effect.rs::actual_source_build_and_contract_snapshots_separate_tasks`
- `crates/qcore-verification/tests/source_effect.rs::ordered_preprocessing_options_bind_source_tasks_even_when_output_is_identical`
- `crates/qcore-audit/tests/compile_options.rs` (eight tests: include classes and
  order, quote-only source lookup, macro order, command structure, admitted
  frontend flags, required Clang driver)
- `crates/qcore-audit/tests/configured_cli.rs::absent_selections_contracts_and_unsupported_flags_fail_closed`,
  whose `conflict` case already rejected a second compile record with a
  different `-std=`.

Background: `docs/reports/w2-preprocessor-transport/README.md` and
`docs/reports/w2-clang-cmake-build/README.md`.

## 2. The gap this card closed

The **resource-projection** side of the configured audit recorded no compile-task
identity at all. `qcore-audit`'s `report.assumptions.json` bound the selected
`.c` bytes (`source_sha256`), the compilation database, the profile and the
contract manifest — but nothing about the *effective preprocessing input*.

A used header, a generated configuration header, or any header on the search
path could therefore change the analyzed code and flip a unit's verdict while
every recorded input stayed byte-identical. Anything keyed on that record would
be reused across two different analyses. The new CLI regression
`a_used_header_change_invalidates_the_recorded_compile_task_identity` reproduces
exactly that: the same `src/unit.c`, the same database, `Safe` then `Unknown`.

## 3. What was newly bound

### `qcore_c_gateway::ClangIdentity` (`crates/qcore-c-gateway/src/clang.rs`)

The previously private compiler description is now a public, reusable
`ClangIdentity` with `capture()`, `program()`, `version()` and `target()`.
`boolean_source` uses it under its former name; field names, order and
serialization are unchanged, so every existing identity digest is unchanged.

### `CompileTaskIdentity` (`crates/qcore-audit/src/compile_db.rs`)

`compile_task_identity(entry, unit, input, compiler, dependencies)` binds, in
one serialized record with a `digest()`:

| Acceptance component | Bound as |
|---|---|
| Compiler identity/version | `compiler.program`, `compiler.version` |
| Target | `compiler.target` |
| Dialect | `dialect` (`c11` / `gnu99` / `gnu11` / `gnu17`) |
| Working directory | `directory` (the compilation record's own directory) |
| Ordered arguments | `arguments` — the complete decoded vector, plus `declared_driver` |
| Main source path | `file` (absolute, resolved against `directory`) |
| Effective preprocessing input | `preprocessed_source_sha256`, or `preprocessing_error` when Clang rejected it |
| Selected source bytes | `raw_source_sha256` |
| Transported configuration | `preprocessor_options` (ordered, typed), `include_paths`, `definitions`, `function_scope` |
| Dependency closure — contracts and profiles | `dependencies` map |

`run_config` fills `dependencies` with `compile_database`,
`resource_contract_manifest`, `source_profile`, and — when source contracts are
configured — `source_contract_configuration` and `source_contract_rules`. Each
audited unit's identity and `identity_sha256` are written to
`report.assumptions.json` under `compile_tasks`, with
`compile_task_identity_version: 1`.

The identity is built from the same `C11TranslationUnit` the projection
analyzes, and the entry has already passed `compiler_options`, which rejects any
argument the gateway cannot transport. The recorded argument vector is therefore
complete rather than a filtered projection.

### Ambiguity rule (`same_configuration`, `select_unique_configuration`)

One configuration is one `(working directory, complete command)` pair.
`select_profile_units` now uses the shared `same_configuration` predicate, and
`select_unique_configuration` rejects both "no command" and "more than one
distinct configuration", naming the count. Repeats of one identical record are
one task and are audited once.

## 4. Tests added

`crates/qcore-audit/tests/compile_task_identity.rs` (new, 6 tests):

- `identity_binds_compiler_target_dialect_directory_ordered_arguments_and_effective_input`
- `a_changed_used_header_invalidates_the_identity_with_an_unchanged_main_source`
- `include_order_and_admitted_macro_options_invalidate_a_byte_identical_analysis`
  — include-order swap, an unread `-D`, `-D`/`-U` order and a dialect change all
  change the identity while the preprocessed digest is unchanged
- `contract_profile_and_database_dependency_changes_invalidate_the_identity`
- `a_rejected_preprocessing_run_keeps_an_attributable_identity`
- `duplicate_compile_records_for_one_file_are_kept_distinct_and_never_chosen_between`

`crates/qcore-audit/tests/configured_cli.rs` (2 added):

- `a_used_header_change_invalidates_the_recorded_compile_task_identity`
- `repeated_and_conflicting_compile_records_for_one_file_are_distinguished`
  — one repeated record yields one unit and one task; a conflicting `-D` set and
  a conflicting `-o` both reject with `conflicting compilation commands` and no
  report is written

`crates/qcore-verification/tests/source_effect.rs` (1 added):

- `a_changed_header_expansion_retires_the_prior_task_and_its_witness` — identical
  checked paths and identical source bytes, different effective preprocessing
  input; the task digest changes and each task refuses the other's witness while
  still discharging its own

`crates/qcore-c-gateway/tests/boolean_frame_helpers.rs` (1 added):

- `the_shared_clang_identity_keeps_its_recorded_serialized_shape` — pins the
  field set and emission order of the compiler description, which is embedded in
  every published `import_identity` and therefore in the pinned
  `chunk-contracts` and `source-effects` artifacts

## 5. Measured ambiguity in the pinned Cycle 19 inputs

`compile-database-ambiguity.json` is a read-only census of the persisted
databases. It is a finding, not a change:

| Database | Records | Files | Files with >1 configuration | Max configurations for one file |
|---|---|---|---|---|
| `fluent-bit-clang-production` | 1,415 | 1,244 | 155 | 17 |
| `monkey-build` | 142 | 61 | 27 | 4 |

Inside the audited source roots: `lib/cmetrics/src`, `lib/ctraces/src`,
`lib/librdkafka-2.15.0/src` and `lib/chunkio/src` are unambiguous, but

- `src/flb_input_chunk.c` (the `fluent_bit` `Exact` selection) has **two**
  configurations, a static build and a shared build differing by
  `-Dfluent_bit_shared_EXPORTS` — a preprocessor definition;
- all **27** `mk_core/deps/libevent/*.c` units of the Monkey/forced-Libevent leg
  have **four** configurations each, differing by `event_shared_EXPORTS`,
  `event_core_shared_EXPORTS`, `event_extra_shared_EXPORTS` and their include
  sets.

The **generic configured route rejects** all of these (§3, tested in §4). The
**pinned Cycle 19 compatibility adapter does not**: `select_exact_unit` takes
the first matching record and `select_recursive_units` calls
`dedup_by(file)`, so 28 audited units are analyzed under one arbitrarily chosen
configuration out of two or four. Repairing that changes which units the pinned
185-unit audit analyzes and therefore its four pinned artifacts, which is a
deliberate pin update outside this card's bounds. It is recorded here as an
open exclusion, not as a repaired defect. `compiler/examples/support/cycle19_compatibility.rs`
is unmodified by this card.

## 6. Explicit exclusions

1. **Individual headers are not enumerated.** The dependency closure of used
   headers is bound *by effect*, through `preprocessed_source_sha256`. Because
   preprocessing uses `-E -P`, line markers are stripped: a different header
   file with identical relevant contents is the same task, and a header edit
   that leaves the token stream unchanged does not change the identity. No
   `-MD`-style dependency scan is performed.
2. **The declared driver is recorded, never checked for equivalence.** The
   identity describes the Clang that actually ran. `compiler_options` demands
   the configured Clang executable only when explicit frontend build flags are
   present; compiler substitution and GCC/Clang equivalence stay out of scope.
3. **The pinned compatibility adapter's silent configuration choice** (§5).
4. **`compiler_options_compatibility`** keeps its historical projection, which
   ignores unmodeled flags; identities derived through it inherit that bound.
5. **Identity is provenance, not attestation.** The compiler executable, its
   environment and the filesystem are trusted and must stay stable while an
   identity is in use. Nothing here proves a checkout, a build or an object file.
6. **Textually different spellings of one command are distinct configurations.**
   This over-rejects rather than under-rejects.
7. **Cost.** `run_config` now preprocesses each unit once more to bind the
   effective input. This route audits configured selections (7 CMetrics units,
   the second-codebase fixtures), not the 185-unit compatibility corpus.

## 7. Gate results

Exact counts and exit codes are in `validation.json`; command output is in
`logs/`.

| Gate | Result |
|---|---|
| `cargo test --workspace` | 829 passed, 0 failed (baseline 819 + 10 new) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| `lake build` (proof, `~/.elan/bin`) | exit 0, 62 jobs |
| `scripts/check_lean_audits.py` | exit 0, 20 Lean audits |
| `python3 -B -m unittest discover -s scripts/tests` | 43 tests, OK |
| providers workspace `cargo test` | 45 passed, 0 failed |
| providers `cargo fmt` / `cargo clippy` | exit 0 / exit 0 |
| `providers/package.py build` | exit 0, five packages |
| Configured CMetrics audit | **1 checked function contract / 6 Unknown**, raw unit provisional `Unsafe` — unchanged |

No Lean definition or theorem changed; no new Lean theorem was needed, because
this card adds provenance and selection rules, not semantics.

### Pinned 185-unit artifacts

Byte parity of the four pinned artifacts **could not be re-measured in this
environment**: the upstream Fluent Bit checkout at
`/home/cosmo/Documents/GitHub/fluent-bit` has moved off the audited pin
(`66910c10…`, found `d411765e…`), and the audit's pin gate correctly refuses to
run (`logs/cycle19-audit.log`). This is an environment condition, not a
consequence of the change. The persisted `fluent-bit-clang-source` clone is at
the pin but has no `build/compile_commands.json` of its own.

What the change can and cannot touch there:

- `compiler/examples/support/cycle19_compatibility.rs`, `project_unit`,
  `AuditReport`, `compiler_options_compatibility`, `compile_entries` and
  `shell_words` are unmodified.
- `select_profile_units` and `run_config` are used only by the configured
  `qcore-audit` driver, not by the compatibility example.
- The only shared code touched is the compiler description, moved verbatim from
  a private `boolean_source::CompilerIdentity` to the public
  `clang::ClangIdentity` with the same derives, field names and field order.
  Its serialized shape is now pinned by
  `the_shared_clang_identity_keeps_its_recorded_serialized_shape`, and the
  existing identity tests in `compiler/tests/unit/chunk_source.rs` and
  `crates/qcore-c-gateway/tests/boolean_frame_helpers.rs` pass unchanged.

The CMetrics audit — which exercises the same gateway import, identity and
process replay path end to end against the persisted pinned Clang CMake build —
reproduces its recorded result exactly.

## 8. Status language

No verdict changes. No raw Cycle 19 unit is promoted; Safe/Unsafe/Unknown and
"unpromoted" keep their AGENTS.md meanings. Provisional `Unsafe` remains
provisional. This card adds identity and fail-closed ambiguity handling; it
proves nothing new about the analyzed C.
