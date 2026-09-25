# W44-CONTRACTS-ONLY-NO-BUILD: a contracts-only audit that needs no build settings

Card W44-CONTRACTS-ONLY-NO-BUILD, 2026-09-25, on base `634f9a74` ("Record the
w44-provider-resource-view integration and archive the combined baseline").
The user's definition reads: "Contract only should be meant as using all of
loaded contracts without build settings." W44-PROVIDER-RESOURCE-VIEW delivered
the first half, "all loaded contracts". This card delivers the second: a route
that audits C sources with the loaded contracts alone, with no compilation
database and no prepared build.

## Headline

* **`quirin-audit contracts`.** The simplest form points it at a checkout and
  names nothing else: `quirin-audit contracts --source-root ../fluent-bit`.
  - It walks `*.c`, or takes the files and directories you name.
  - It preprocesses each unit with only `-I`/`-D`/`--std` and three stated
    defaults: `gnu17`, the unit's own directory for quoted includes, and the
    host Clang's system headers.
  - It loads every crate's resource view by default.
  - It runs units with `--jobs` through the unchanged ordered fold, and has a
    JSON form.
* **Units that cannot be analysed.** A unit that does not preprocess or parse
  is `Unknown`, with a reason code and its first diagnostic. It is never
  dropped.
* **The publication rule.** Every verdict is relative to the recorded
  assumptions, and none is presented as equivalent to a build-configured
  verdict (§3). Three carriers enforce it:
  - each row's `assumptions`;
  - the report's `evidence` block;
  - the assumption record as each compile task's `source_profile` dependency,
    so a Safe certificate minted under assumed settings cannot be admitted for
    a configured task.
* **Lean.** `QCore.NoBuildScope` has 14 audited theorems. They include the
  witness that the favourable direction exists
  (`favourable_direction_exists`).
* **Fluent Bit, over the Cycle 19 libraries.** The runs cover 156 files: 165
  no-build units against 157 build-configured configurations.
  - With no flags, 6 units preprocess.
  - With the source tree's include directories, 52 units prepare in both
    runs, and every unit and function verdict equals the configured one:
    1,202 function pairs agree and 0 differ.
  - **No no-build verdict is more favourable than the configured one.**
  - Every difference is attributed (§4). The remaining Unknowns come from
    headers only a configure step generates, and from one per-file definition
    that only the build supplies.
* **Unchanged.** No analyzer, gateway, projection or contract text changed.
  The pinned arbiter is verdict-bearing identical to the
  W44-COLLECTION-ALLOCATION baseline (§6).

## 1. What a user runs

From the repository root:

```sh
cargo run --release -p qcore-audit -- contracts --source-root ../fluent-bit
```

With a report, include directories, a definition and 16 jobs:

```sh
cargo run --release -p qcore-audit -- contracts --source-root ../fluent-bit \
  lib/ctraces/src lib/chunkio/src \
  -I ../fluent-bit/lib/ctraces/include -I ../fluent-bit/lib/chunkio/include \
  -D NAME=1 --std gnu17 --report private/no-build/report.json --jobs 16
```

The JSON form is `quirin-audit contracts --config FILE`, with:

```json
{"schema_version": 1, "mode": "contracts-only", "build": "none",
 "source_root": "../fluent-bit", "sources": ["lib/ctraces/src"],
 "include_dirs": ["../fluent-bit/lib/ctraces/include"], "defines": [],
 "std": "gnu17", "load_path": "providers/contract-load-path.json",
 "report": "private/no-build/report.json", "jobs": 16}
```

`mode` and `build` are required and must be exactly these values. The
[crate README](../../../crates/qcore-audit/README.md#quirin-audit-contracts-contracts-only-without-build-settings)
is the user reference. The measurement used the JSON form
([bare.json](measurement/bare.json),
[src-includes.json](measurement/src-includes.json)).

## 2. Decisions

**A subcommand, not a `current` flag.** `quirin-audit current` is defined by
its inputs:

- a checkout `HEAD` whose bytes are verified against Git;
- a source profile's library groups;
- a compilation database's commands, with driver checks.

A no-build run has none of these, so a `current --no-build` would have to
refuse most of `current`'s flags. `contracts` reads naturally beside `current`
and `prepare`, and names what it keeps: the contracts. The contract choice is
factored out of `current_cli` (`choose_contracts`) unchanged. Both routes
resolve a selection, a manifest and the all-loaded default with the same code
and the same refusals.

**Selection.** Named paths are relative to the source root. A directory
contributes its regular `*.c` files in sorted order. The walk does not enter
dot-directories and does not follow symbolic links. The first selection of a
file wins. An empty directory, a path outside the root and an empty selection
all refuse. The walk rule, the skip counts and the duplicates are recorded in
the sidecar's `selection`.

**Preprocessing.** Each unit becomes one synthesized compile record:

```text
<host clang> -std=<std> -I<dir>... -D<def>... -c <file name>
```

It runs in the unit's own directory. The record is shell-quoted, then passed
through the unchanged `compiler_options` and gateway (`prepare_translation_unit`).
So the gateway's supported options still bound what a user can supply, and the
compile-task identity is the ordinary one. Include directories must exist,
because Clang ignores a misspelt one silently. Definitions must be
`IDENT[=VALUE]` with no `$`. Nothing is searched for or guessed. Headers are
read as they are on disk, tracked or not, and the assumption record says so
(§4.3).

**Unknown with a reason.** When `validation_error` is set, the route records
the unit as `Unknown` and does not project it. The row keeps the raw function
denominator. Its category is `reason_code: "environment-unavailable"`: a
configuration input was missing, which makes no claim about the source and can
never support a provisional Unsafe (§L21.1). The route's own code is in
`assumptions.reason`:

| Code | Meaning |
|---|---|
| `missing-include` | first error is `file not found` |
| `unresolved-configuration` | first error is an `#error` directive |
| `preprocessing-error` | any other preprocessing error |
| `syntax-error` | the `-fsyntax-only` stage failed |
| `unsupported-input-identity` | the gateway refused the input identity |
| `clang-unavailable` | Clang could not run |
| `unreadable-source`, `not-utf8`, `unsupported-setting` | the source or a setting could not be used |

The closed seven-category vocabulary is not extended. These units are not
gateway failures, so they do not fail the command. A real gateway failure
still does, as in `current`.

**Both forms.** The rule is `current`'s: a setting given by both forms must be
equal, and nothing is merged. `--jobs` is the only exception: flag, then
`jobs`, then `QCORE_AUDIT_JOBS`, then the host default. The report and sidecar
record whether the dialect is the default or declared. Which form declared it
is recorded only in `<report>.invocation.json`, so the report and sidecar are
byte-identical across forms (test). With no `--load-path` and no manifest,
`providers/contract-load-path.json` is read from the working directory
(`load_path_source: "default"`). If it is missing, the command refuses.

## 3. The publication rule

A no-build verdict is about the unit **as preprocessed under the recorded
assumptions**, and nothing else. Different macro and include settings select
different code, so a Safe under assumed flags says nothing about a build. The
rule adopted, stated in `docs/agents/source-identity.md` and in each row's
`verdict_scope`:

> Every verdict of `quirin-audit contracts`, `Safe` included, is relative to
> the recorded assumptions only, and is never presented as equivalent to,
> comparable with or a substitute for a build-configured verdict of the same
> file.

It is enforced, not only stated:

1. Every row carries `assumptions`: dialect, include directories, definitions,
   quote directory, `verdict_scope`, `outcome`, `reason` and
   `first_diagnostic`. The sidecar lists every file with the same fields and
   its raw SHA-256.
2. The report's `evidence` block states `evidence:
   "contracts-only-no-build"`, `build: "none"`, the assumed configuration and
   `equivalence_to_build_configured: "none: …"`.
3. The no-build "source profile" (selection plus assumed configuration) is
   each compile task's `source_profile` dependency, beside `build_settings`.
   Every Safe certificate's trust context digests `source_profile`
   (`check_audit_legacy_safety_certificate_for`). So a certificate minted under
   assumed settings can never be admitted for a configured compile task, or
   under other assumptions. A test pins that `-D` changes this digest.

**Lean** (`proof/QCore/NoBuildScope.lean`, audited by
`AuditNoBuildScope.lean`, 14 theorems):

- `publish_scope` and `publish_never_configured`: a published verdict's scope
  is its assumption set.
- `unprepared_unknown`, `reason_iff_unprepared`, `run_length` and
  `run_reasons`: a unit that does not prepare is Unknown with its reason, and
  none is dropped.
- `safe_relative`: Safe is exactly the analysis of the text these assumptions
  prepare.
- `noBuild_never_admitted_as_configured` and
  `noBuild_admitted_only_under_own_assumptions`: certificate admission binds
  the scope.
- `noBuild_not_comparable_with_configured`.
- `same_text_same_verdict` and `verdict_difference_needs_text_difference`:
  the attribution method in §4.
- `favourable_direction_exists` and `no_transfer_between_assumptions`: a
  witness world where one source is Safe under assumptions and provisional
  Unsafe under its build. So the scope is necessary, not decorative.

There are no placeholders and no project axioms. The Rust–Lean correspondence
is by review (constraint 28).

**Why not refuse Safe under assumed flags?** The measurement found no
favourable-direction case (§4.2). Refusing would withdraw 102 Safe functions
that equal their configured verdicts, and would not be more sound than scoping.
A scoped Safe claims only what `safe_relative` proves. The brief's legitimate
stop does not arise: this rule needs no user decision.

## 4. Measurement on Fluent Bit

**Inputs:**

- The checkout `/home/cosmo/Documents/GitHub/fluent-bit` at `864c3d97`, read
  only.
- The build-configured comparison: `quirin-audit prepare clang-build`, run
  into the session scratchpad outside the worktree (deleted after
  measuring). It is a Clang CMake build of the same commit.
- `QCORE_CLANG=/usr/bin/clang` (Ubuntu clang 21.1.8), 16 jobs, the final
  release binary ([digest](measurement/binary.sha256)).
- The harness: [`measure.sh`](scripts/harness/measure.sh),
  [`selection.py`](scripts/selection.py),
  [`attribute.sh`](scripts/harness/attribute.sh) and
  [`compare.py`](scripts/compare.py).
- Run `m2` is the evidence. Run `m1` gave identical tallies.

**The configured baseline** is `quirin-audit current --contracts-only` with
all loaded contracts over the profile's five library groups, without Monkey.
The no-build runs select the same libraries from the profile: `top_level`
roots as their files, `recursive` roots as directories, and `exact` as the
file. Three no-build runs:

- **bare**: no flags.
- **src-includes**: the `-I`/`-isystem` directories of the selected configured
  records that lie in the source tree, rebased onto the checkout, in
  first-seen order: 38 directories. This is what a user can name from the tree
  alone. The 17 build-tree directories are left out, because they do not exist
  without a build ([selection](measurement/selection.json)). No `-D` is given.
- **root**: the simplest form, the whole checkout with no flags.

### 4.1 Outcomes

| Run | Units | Prepared | Not prepared (reason) | Safe / prov. Unsafe / Unknown / data-only | Functions Safe | Wall |
|---|---:|---:|---|---|---:|---:|
| configured | 157 configurations (156 files) | 157 | — | 2 / 5 / 146 / 4 | 269 | 71 s |
| bare | 165 | 6 | 159 missing-include | 0 / 0 / 164 / 1 | 34 | 4 s |
| src-includes | 165 | 54 | 104 missing-include, 7 syntax-error | 2 / 1 / 158 / 4 | 102 | 9 s |
| root | 2,944 | 356 | 2,545 missing-include, 21 syntax-error, 20 unresolved-configuration, 2 not-utf8 | 14 / 17 / 2,903 / 10 | 726 | 355 s |

All runs exit 0 with 0 gateway failures ([runs.json](measurement/runs.json)).
The src-includes run at `--jobs 1` wrote a report and sidecar byte-identical
to the 16-job run (`cmp` exit 0 and 0).

### 4.2 Agreement and direction

On the 156 files both runs audit, [attribution-src-includes.json](measurement/attribution-src-includes.json)
and [attribution-bare.json](measurement/attribution-bare.json) place every unit
in exactly one category:

| Category | bare | src-includes |
|---|---:|---:|
| prepared, same preprocessed text, same verdict | 5 | 20 |
| prepared, different text, same verdict | 1 | 32 |
| prepared, different verdict | 0 | 0 |
| not prepared: missing header in the tracked source tree, not on the include path | 64 | 0 |
| not prepared: missing header only a configure step generates | 84 | 97 |
| not prepared: missing header present only as an untracked file | 2 | 0 |
| not prepared: syntax error | 0 | 7 |
| no-build only: not compiled by the configured build | 9 | 9 |

- **Function pairs.** On every unit that both runs prepared, all function
  statuses are equal: 361 of 361 pairs in bare, 1,202 of 1,202 in
  src-includes. Both configured Safe units (`cmt_time.c`, `ctr_version.c`) and
  the provisional Unsafe `ctr_random.c` agree.
- **The favourable direction:** 0 in every run. No unit or function is Safe
  under assumptions where the configured run is not Safe.
- **Different text, same verdicts.** The 32 different-text units read
  different code: each configured command carries 20 to 64 `-D`
  definitions, 76 distinct names across the selection (`CMT_HAVE_*`,
  `CTR_HAVE_*`, `CIO_HAVE_*`, `FLB_HAVE_*` and others), plus the generated
  headers. Their verdicts still agree. By
  `verdict_difference_needs_text_difference`, a text difference is necessary
  for a verdict difference, not sufficient; here it never produced one.
- **No configured counterpart.** In the root run, the 14 Safe units have no
  configured verdict to compare against. Each is not compiled by the
  configured build or lies outside its selection: WAMR samples and tests, two
  CMake probe files, `nghttp2_time.c`, `nghttp2_version.c`, `xsi_strerror.c`
  and a WASM filter example ([attribution-root.json.gz](measurement/attribution-root.json.gz),
  `safe_without_configured_counterpart`). They are published relative to the
  assumptions like every other verdict. Within the 156 shared files, the root
  run's results are identical to bare's, as the same assumptions require.

### 4.3 Attribution of every difference

- **Generated headers: 97 units, in both runs.** Configure produces these
  headers, so the no-build route never guesses them:
  - librdkafka's `src/rd.h` includes `../config.h`, which librdkafka's CMake
    step writes into the build tree: 83 units;
  - ChunkIO's `chunkio/cio_info.h`: 13;
  - Monkey's `mk_core/mk_core_info.h`, reached from `src/flb_input_chunk.c`: 1.
  Bare has 84 in this category: 83 plus one `cio_info.h`. The others stop
  earlier on a source-tree header.
- **Source-tree headers not on the path: 64 units, bare only.** For example,
  `cmetrics/cmetrics.h` (27), `ctraces/ctraces.h` (15) and ChunkIO headers
  (12). The src-includes run resolves every one of them.
- **Per-file build definition: 7 units, src-includes only.** cmetrics' CMake
  step defines `__CMT_FILENAME__` per file. Without it, `cmt_counter.c`,
  `cmt_exp_histogram.c`, `cmt_filter.c`, `cmt_gauge.c`, `cmt_histogram.c`,
  `cmt_summary.c` and `cmt_untyped.c` fail to parse
  (`use of undeclared identifier '__CMT_FILENAME__'`). This is `syntax-error`
  by the rule, `unresolved-configuration` in substance.
- **Untracked headers: 2 units in bare; read silently in src-includes.** An
  earlier in-tree configure left `include/fluent-bit/flb_info.h` and
  `lib/cmetrics/include/cmetrics/cmt_info.h` in the checkout. Both are
  git-ignored, and `cmt_info.h` records a different `CMT_SOURCE_DIR` from the
  clean clone's. In bare they are only missing. In src-includes, their
  directories are on the path, so the prepared cmetrics units and
  `flb_input_chunk.c` read these stale bytes. Verdicts still agree, but the
  record binds header bytes only through the preprocessed-text digest. So the
  assumption record now says headers are read as found, tracked or not. The
  residual is filed as `W44-NO-BUILD-HEADER-PROVENANCE`.
- **Not compiled by the configured build: 9 units.** These are
  `cio_file_win32.c`, `cio_sha1.c`, `win32/dirent.c`, `cmt_atomic_gcc.c`,
  `cmt_atomic_generic.c`, `cmt_atomic_msvc.c`, `cmt_decode_prometheus.c`,
  `rdkafka_sasl_win32.c` and `regexp.c`. All are Unknown in both no-build
  runs. The two `cmt_atomic_*` files prepare in src-includes and are Unknown.
- **The root run outside the selection: 2,788 units.** Of these, 1,727 are
  not compiled by the configured build, which includes the checkout's own
  `build/` tree and samples. The other 1,061 are compiled but outside the
  Cycle 19 groups.
  - Not prepared: 2,545 missing-include.
  - 20 `unresolved-configuration`: WAMR `#error` configuration guards.
  - 21 `syntax-error`: lwrb 7, nghttp2 5, WAMR 3, librdkafka 2, and one each in
    jansson, monkey, zstd and `src/flb_blob_db.c`.
  - 2 `not-utf8`, in onigmo.

## 5. Tests and Lean

**Rust.** `crates/qcore-audit/tests/no_build_cli.rs` has 7 tests; the brief's
list maps onto them:

- A unit that preprocesses with no settings is Safe. A missing include is
  Unknown with `missing-include`, its first diagnostic and
  `environment-unavailable`, and keeps its row and denominator. A hidden
  directory is not walked. `-D` absent leaves a leak (provisional Unsafe).
- `-I` turns the missing-include Unknown into an analysed Safe unit.
- `-D RELEASE_IT` changes the preprocessed text and the verdict (Unsafe →
  Safe), and changes the bound `source_profile` digest.
- CLI, JSON and both forms give byte-identical reports and sidecars.
  Conflicting `include_dirs`, `defines` or `std` refuse, and `--jobs` beats
  `jobs`. A wrong `mode` or `build` refuses, and so do `compile_database`,
  `packages` and `--compile-database`.
- An empty directory, a path outside the root and a missing include directory
  refuse. The default load path loads all views.
- Jobs 1 and 16 give identical reports, sidecars and console output apart
  from the timing line.
- The assumption record: the evidence block, every row's `assumptions` and
  quote directory, the `source_profile` binding, and the sidecar's
  per-file list.

`src/no_build.rs` adds 4 unit tests: parsing, settings refusals, diagnostic
classification, and quoting round-tripping through `shell_words`.

**Lean** is covered in §3.

## 6. Gates

See [gate.json](gate.json), harness [`gate.sh`](scripts/harness/gate.sh),
logs in the worktree-private `private/measure/gate/`.

GATE-TABLE

## 7. Files

* Code:
  * `crates/qcore-audit/src/no_build.rs` (new);
  * `main.rs` and `lib.rs`: the `contracts` subcommand;
  * `current_cli.rs`: `choose_contracts` factored out unchanged, and `anchor`
    and `one` made crate-visible;
  * `report.rs`: `record_assumptions`.
* Tests: `crates/qcore-audit/tests/no_build_cli.rs`.
* Lean: `proof/QCore/NoBuildScope.lean`, `proof/AuditNoBuildScope.lean`, and
  the `proof/QCore.lean` import.
* Docs:
  * `crates/qcore-audit/README.md`: the new section only;
  * `docs/agents/source-identity.md`: the scope rule;
  * `docs/development/architecture.md`;
  * the wave-44 increment paragraph;
  * `docs/reports/INDEX.md`;
  * the acceptance matrix;
  * this report, `followups.json`, `scripts/` and `measurement/`.

## 8. Follow-ups

* `W44-NO-BUILD-HEADER-PROVENANCE` (§4.3): a no-build verdict can rest on
  untracked or stale headers, and header bytes are bound only through the
  preprocessed text.
* `W44-NO-BUILD-GENERATED-CONFIGURATION` (§4.3): 97 of the 156 shared files
  stay Unknown without a configure step's generated headers, and 7 without a
  per-file build definition.
