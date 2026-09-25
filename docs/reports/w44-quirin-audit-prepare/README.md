# W44-QUIRIN-AUDIT-PREPARE: `quirin-audit prepare` replaces the Python preparation scripts

Card W44-QUIRIN-AUDIT-PREPARE, 2026-09-25, on base `fc6a0f14` ("Record the
w44-quirin-audit-providers integration gate").

## Headline

* **New subcommand.** `quirin-audit prepare` prepares the Fluent Bit audit
  inputs in Rust (`crates/qcore-audit/src/prepare.rs`). It has two steps, each
  a line-by-line port of one retired script:
  * `prepare clang-build` replaces `scripts/prepare_cmetrics_clang_audit.py`;
  * `prepare monkey` replaces `scripts/prepare_cycle19_monkey.py`.

  Each step keeps its script's flags, defaults, refusals and output files.
  `prepare` with no step runs both into `build/cycle19/clang/<commit>/`, which
  is the layout `quirin-audit current` reads. `--run` then chains into
  `quirin-audit current`, passing `--jobs N` and any arguments after `--`.
* **Parity.** Both implementations prepared the same local Fluent Bit revision
  (`864c3d97`), once into different output roots and once into the same path,
  one after the other. In the same-path pass all 17,535 compared files were
  compared:
  * 17,528 are byte-identical. These include both compile databases, both
    `CMakeCache.txt` files, `cmetrics-audit.json.config`,
    `build-provenance.json`, the Monkey marker, `libcmetrics.a` and every
    object file.
  * The other 7 are CMake and make logs. They differ only in CMake's random
    try-compile names, step timings and `-j2` output interleaving.
  * The `quirin-audit current` runs over the two preparations wrote six
    report files, all byte-identical: 3 Safe, 10 provisional Unsafe,
    169 Unknown and 4 data-only over 186 configurations.
* **Scripts retired.** Both scripts are deleted and every live reference is
  repointed. `scripts/tests/test_retired_scripts.py` now guards all three
  retired preparation scripts. Frozen `docs/reports/` and increment text is
  unchanged.
* **No analyzer, verdict or Lean change.** This card changes tooling only.
  Preparation is a deterministic invocation of external programs, and no
  verdict depends on it: `quirin-audit current` checks its inputs itself (the
  Clang driver identity, source bytes against `HEAD`). So there is nothing new
  to model in Lean.

## 1. What a user runs

To prepare and audit in one command:

```sh
cargo run --release -p qcore-audit -- prepare --fluent-bit ../fluent-bit \
  --clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++ \
  --run --jobs 16
```

The command runs from the repository root, or takes `--repository DIR`. It
prepares `build/cycle19/clang/<HEAD commit>/`. `--revision REV` selects
another local commit, and `--output DIR` another location. The command writes
`prepare.json` there, then runs:

```sh
QCORE_CLANG=<--clang, resolved> quirin-audit current \
  --checkout <out>/fluent-bit-clang-source \
  --profile-directory <repo>/contract/profiles/fluent-bit \
  --compile-database <out>/fluent-bit-clang-production/compile_commands.json \
  --monkey-database <out>/monkey/monkey-build/compile_commands.json \
  --monkey-source-report <out>/reports/monkey-source.json \
  --report <out>/reports/resource-diagnostics.json --jobs 16 [ARGS after --]
```

With no contract arguments, the profile's declared manifest
(`cycle19-resource-contracts.json`) selects the contracts. Arguments after
`--` are passed through, for example provider selections or
`--contracts-only`. An argument that repeats one the chain already sets is
refused by `current` ("given twice").

The two-command form is `prepare` without `--run`, then the `current` command
shown in `crates/qcore-audit/README.md`. The individual steps remain
available, with the scripts' defaults:

* `prepare clang-build` writes to `build/cycle19/revisions/<commit>` by
  default.
* `prepare monkey --fluent-bit DIR` writes to `build/cycle19` by default.

`docs/CYCLE19_BUILD_INPUTS.md` now names these steps.

**Fail-closed arms.** The subcommand keeps every refusal the scripts had:

* an output inside the upstream checkout;
* a C compiler that is missing or not Clang;
* a reused clone at another revision, with tracked changes, or with borrowed
  object alternates;
* a CMake cache from another source or compiler;
* a build that changed tracked files;
* not exactly one `cmt_counter.c` entry;
* local changes under `lib/monkey`;
* an unmanaged non-empty Monkey output;
* a changed managed identity or source copy, or unexpected extra files;
* a Monkey database without the unique Libevent `mk_event.c` entry, or with
  missing files.

It adds three refusals:

* a directory that is not the QCore-C repository;
* `--jobs` or `--` without `--run`;
* with `--run`, a `QCORE_CLANG` naming a compiler other than the prepared C
  compiler. This is checked before any clone or build, because otherwise the
  audit would reject every unit only after the build.

`crates/qcore-audit/tests/prepare_cli.rs` drives these arms and runs the
Monkey step end to end on a fixture checkout, with stand-in `clang`, `clang++`
and `cmake` programs. The test covers the reuse path and a changed copy.

## 2. What is reproduced exactly

The port runs the same programs with the same argument vectors. `git`,
`cmake` and `clang --version` are run as processes, with the inherited
environment:

* `git rev-parse --verify --end-of-options REV^{commit}`;
* `git clone --no-hardlinks --no-checkout` and `checkout --detach`;
* the two `status --porcelain` checks and `ls-files -z`;
* both CMake configure vectors, with `-DCMAKE_C_FLAGS:STRING=-std=gnu17` and
  the explicit `CMAKE_C_COMPILER` and `CMAKE_CXX_COMPILER`;
* `cmake --build … --target cmetrics-static -j2`.

Python helper semantics are reproduced where they reach an output byte:

* **`shutil.which`.** A name with a separator is taken as given, otherwise the
  `PATH` join spelling is used. The spelling reaches the CMake command and the
  database.
* **`Path.resolve()`.** Components that exist are resolved, and the rest are
  applied lexically.
* **Text mode.** Output is decoded as strict UTF-8 with universal newlines,
  and `str.strip`.
* **`logged`.** One pipe carries both stdout and stderr, and it is written
  even when the command fails.
* **`shutil.copy2`.** Contents, mode and times are copied. In pass 1, 795
  copied Monkey files were compared with their sources in the clone. The
  mtimes and modes match on both sides, except for the same one file on each
  side.
* **CMake cache.** Parsing matches Python's `dict`: a key keeps its first
  position and takes its last value.

The records are Python's `json.dumps(indent=2)`
(`crates/qcore-audit/src/prepare_json.rs`): insertion order, `ensure_ascii`
escapes and inline empty containers. The workspace `serde_json` has no
`preserve_order`, so the port adds its own ordered value type, with no new
dependency. A non-integer number is refused rather than printed in a notation
that might differ from Python's.

Three deliberate differences reach no prepared byte:

* The repository root is `--repository` or the working directory. The
  scripts used their own location.
* A captured command's stdin is closed. The scripts' captured commands
  inherited it, and none of them reads stdin.
* The executable test is mode bits, where Python uses `os.access`.

**Provenance.** The step records are unchanged:

* `build-provenance.json` records the configure command, including
  `-DCMAKE_CXX_COMPILER`.
* The Monkey marker records the CMake command.

`prepare.json`, new, records the following
(example: `parity/3-same-path-prepare.json`):

* the argv and working directory, the repository, upstream and revision;
* both compilers, each with its requested name, `which` path, resolved path
  and `--version`, the C++ compiler included, which `invocation.json` leaves
  to preparation;
* the CMake version;
* each step's command, record paths and SHA-256 digests;
* the chained `current` argv.

## 3. Parity evidence

Harness: `scripts/harness/` (`run_prep.sh`, `run_audit.sh`, `run_same.sh` and
the `compare*.sh` drivers). Comparators: `scripts/compare_preparations.py` and
`scripts/compare_audits.py`.

**Inputs.**

* Fluent Bit `/home/cosmo/Documents/GitHub/fluent-bit` at `HEAD`
  `864c3d97f3589515285a19396ed8dfc85cda08ae` (branch
  `cosmo0920-mitigate-out_chronicle-failures-on-testing`), read only.
* `--clang /usr/lib/llvm-21/bin/clang --clangxx /usr/lib/llvm-21/bin/clang++`,
  Ubuntu clang 21.1.8.
* CMake 4.2.3 with the Unix Makefiles generator.
* The scripts from base `fc6a0f14`.
* This card's `quirin-audit` (debug build for the first pass, release build
  afterwards).
* Outputs in session scratch under `/tmp`, outside the worktree, deleted after
  measurement. The key files and hashes are kept in the worktree-private
  `private/prep/kept/`.

Each comparator lists every regular file of both trees, except `.git`. It
compares bytes, then applies the one path normalisation (replace the left
output root with the right one). A file that still differs gets a named
second-pass explanation, or none. The clones are compared by `HEAD` and
`status --porcelain --untracked-files=all`.

| Pass | What ran | Files | Identical | After path normalisation | Differ (explained / unexplained) |
|---|---|---:|---:|---:|---:|
| 1 | Fresh preparations into `…/py/…` and `…/rs/…` | 17,529 | 15,472 | 2,046 | 11 / 0 |
| 2 | Both rerun over their own outputs (reuse path; Rust with `--run --jobs 16`), then `current` on the Python side | 17,540 | 15,471 | 2,051 | 18 / 2 (report digests) |
| 3 | Python scripts, then `prepare --run`, into the **same** path, each moved aside after its audit | 17,535 | **17,528** | 0 | 7 / 0 |

Each pass has one file on the Rust side only: `prepare.json`. In every pass
the clones are at the same `HEAD`, with an empty status. Records:
`parity/1-different-paths-fresh.json`,
`parity/2-different-paths-rerun-and-audit.json`, `parity/2-audit-reports.json`,
`parity/3-same-path.json` and `parity/3-same-path-audit.json`.

**The brief's named files.** Pass 3 needs no normalisation. Pass 1 needs only
the output-path normalisation:

| File | Pass 3 | Pass 1 | SHA-256 (pass 3) |
|---|---|---|---|
| `fluent-bit-clang-production/compile_commands.json` | identical | after path normalisation | `ecb612c46f145579…` |
| `fluent-bit-clang-production/cmetrics-audit.json.config` | identical | after path normalisation | `c3a80bdf75f77905…` |
| `fluent-bit-clang-production/CMakeCache.txt` | identical | after path normalisation | `7c7dd0f381985f5d…` |
| `fluent-bit-clang-production/build-provenance.json` | identical | digests (below) | `6bc6d1a5e204fc36…` |
| `fluent-bit-clang-production/library/libcmetrics.a` | identical | after path normalisation | `f7723939331f5451…` |
| `monkey/monkey-build/compile_commands.json` | identical | after path normalisation | `fcd5a2af49b65c42…` |
| `monkey/monkey-build/CMakeCache.txt` | identical | after path normalisation | `9fab8f7aa5861bfc…` |
| `monkey/qcore-monkey-inputs.json` | identical | digests (below) | `d5f5e7e343d6cbac…` |

**Every remaining difference, with its explanation.** Each explanation is
checked by the comparator, not asserted:

1. **Output path** (passes 1 and 2, 2,046 or 2,051 files). Replacing the left
   root with the right one makes these files byte-identical. They include the
   compile databases, CMake caches, the configuration, Makefiles and object
   files.
2. **Digests of path-bearing files** (passes 1 and 2).
   * `build-provenance.json` differs in `compile_database_sha256`,
     `cmake_cache_sha256` and `static_library_sha256`.
   * The Monkey marker differs in `compile_database_sha256`.

   Each digest equals the SHA-256 of the left file after path normalisation.
   With those digests substituted, the records are byte-identical.
3. **Bison's include guard** (`lib/cmetrics/cmt_decode_prometheus_parser.h`,
   passes 1 and 2) spells the output path in upper case.
4. **`CMakeRuleHashes.txt`** (passes 1 and 2) lists the same rules, but each
   MD5 digests rule text that contains the output path.
5. **CMake per-run artefacts** (all passes). These are
   `CMakeConfigureLog.yaml`, `CMakeError.log`, `CMakeOutput.log` and the
   `configure.log` files. They differ only in random try-compile names
   (`cmTC_*`, `TryCompile-*`) and step timings. The same names and timings
   differ between any two CMake runs.
6. **`-j2` make output** (all passes).
   * `native-build.log` has the same lines in another order, apart from
     progress percentages. In pass 3, bison and flex wrote into the shared
     pipe mid-line, so the same bytes are interleaved at another point.
   * In pass 2, the rebuild's `compiler_depend.make` files have the same lines
     in another order.
7. **Pass-2 audit reports.** In `resource-diagnostics.json`, exactly 147
   `compile_task_identity.preprocessed_source_sha256` values and 186
   `inventory_sha256` values differ, and no other field. Those digests cover
   text that contains the output path. `resource-diagnostics.invocation.json`
   differs in the two database digests of item 2. Pass 3 removes the path,
   and both files are then byte-identical.

**Audit reports.**

* **Pass 3.** All six report files are byte-identical, including the
  invocation record: `resource-diagnostics.json` (19,948,357 bytes),
  `.contracts.json`, `.chunk-contracts.json`, `.source-effects.json`,
  `.invocation.json` and `monkey-source.json`.
* **Pass 2.** Four of the six are byte-identical without normalisation. The
  other two are item 7.
* **Console.** Apart from the `preparation:` timing line and the three paths
  that `prepare` prints first, the console output is identical.
* **Summary.** Every run prints `185 source file(s), 186 configuration(s):
  3 Safe / 10 provisional Unsafe / 169 Unknown / 4 data-only / 0 gateway
  failures`.

## 4. Retirement and references

Deleted: `scripts/prepare_cmetrics_clang_audit.py` and
`scripts/prepare_cycle19_monkey.py`. At the user's firm instruction during the
card, no script was kept. No live consumer needed one: the only invocations
were the documentation recipes below and the messages.

Repointed:

* **Rust messages.**
  * `compile_db.rs`: the GCC-driver refusal. The `reason_codes.rs` and
    `report.rs` tests are updated with it. Classification is unchanged,
    because it keys on the message's prefix.
  * `current_cli.rs`: the scope of `invocation.json`.
  * `compiler/examples/support/cycle19_compatibility.rs`: the empty Monkey
    database message.
* **Documentation.**
  * The `crates/qcore-audit/README.md` sequence and its CMetrics sections.
  * `docs/development/architecture.md`.
  * Both recipes in `docs/CYCLE19_BUILD_INPUTS.md`. Its claim that the Monkey
    setup "uses the declared Fluent Bit profile pin" is corrected: the setup
    records the checkout's `HEAD`, as the script did.
  * The history paragraph in `docs/GENERALIZATION_PLAN.md`.
* **The guard.** `scripts/tests/test_retired_scripts.py` lists both new
  names. The `docs/agents` pages had no reference.

`qcore-audit` gains no dependency and stays free of compiler and provider
crates.

## 5. Gate

See `gate.json` (measured on this card's final code; commands in
`scripts/harness/gate.sh`, logs in the worktree-private `private/gate/`).

## 6. Follow-ons

* `scripts/prepare_libevent_core_audit.py` remains Python. Porting it as a
  `prepare` step was out of this card's scope.
* The Persistent CMetrics section of `docs/CYCLE19_BUILD_INPUTS.md` has a
  pre-existing mismatch:
  * It runs `quirin-audit contract/profiles/cmetrics-counter/audit.json`
    after preparation.
  * That checked-in configuration reads
    `build/cycle19/fluent-bit-clang-{source,production}`.
  * The preparation step, like the script before it, writes to
    `build/cycle19/revisions/<commit>` by default, and prints the path of a
    configuration generated for that location.

  The recipe works only with `--output build/cycle19`, or with the generated
  configuration. This card preserved the behaviour, and the recipe should say
  which to use.
