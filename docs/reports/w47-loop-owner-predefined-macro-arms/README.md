# W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: reserved macros from the recorded compiler

Card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS (wave 47, card-high), amending
gateway rule **10p**. Base: local `main` at `f269c231` (the worktree's HEAD
was already there; `main` later moved to `3ff0122d`, a records-only landing
the integrator merges). The frozen prediction is
[PREDICTION.md](PREDICTION.md), commit `a58078f5`, made after BEFORE was
measured and before any AFTER census, facts derivation, arbiter or Lean
change (a first draft of the Rust module existed, uncompiled and never run;
disclosed there).

Nothing here is a verified race, a verified bug, a verified fix, an `Unsafe`
claim or an upstream defect claim. Every old-vs-fixed statement has the form
*discharged / not discharged / unresolved at this path under these derived
premises*. The Fluent Bit implementation and test are unchanged.

## 1. Result

At the `90978ac6` export's configured reading, the fixed test's three sites
(`cb_tail_pause_test`, `tests/runtime/in_tail.c:2483`, `:2492`, `:2494`) are
**discharged at this path under the derived premises of §6** on
`(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)`. The two
Windows-only definitions that held them are dropped by decisions the recorded
compiler makes for their own compile commands:

- `lib/monkey/mk_core/mk_utils.c:41` (`localtime_r`): `#if defined
  (__linux__)` is decided true, so the group is read as its first arm and the
  `#elif defined (_WIN32)` arm is not read;
- `src/config_format/flb_cf_yaml.c:550` (`dirname`): `#ifndef _WIN32` is
  decided true (`_WIN32` undefined for both of the file's commands), so its
  `#else` is not read.

With both gone the list-head derivation's unmodelled-call blocker has no
function of the path's closure without a typed reading, the instance flag
derives no coroutine, and the coroutine route to the callback is dropped.

At `90978ac6~1` the old test (the negative control) stays **not discharged**:
all 8 of its sites keep their resolved test-thread context `T Either`. No
other site moves at either revision, the raw readings are byte-identical, and
the corpus, system slice and tail-pause slice are unchanged (§5).

## 2. Mechanism

**Derivation** (`loop_owner_predefined_macros.rs`, child of
`thread_entry_census.rs`, written by the new `predefined_macro_facts`
example). For every entry of the recorded compile database, in order, with
that entry's own first word as the compiler and its own flags (output,
`-c`, dependency outputs and the source file removed):

1. the compiler's identity: path, resolved path, SHA-256 of the resolved
   binary, `--version`; a compiler that cannot be resolved or run, or whose
   `--version` differs from the build record's `compiler_version`
   (`build-provenance.json`), fails every one of its commands;
2. `-dM -E -x LANG` over an empty unit: the macros defined at the start of a
   unit under the command's target and flags;
3. a probe unit, one `#ifdef NAME` per **candidate** (every reserved
   identifier, C11 7.1.3, that a census file's `#if`/`#elif`/`#ifdef`/
   `#ifndef` names): which are defined at the start of a unit. Every `-dM`
   name among the candidates must probe defined (a builtin such as
   `__FILE__` probes defined without being listed);
4. `-E -dD` of the command's own file: every `#define`/`#undef` outside the
   `<built-in>` and `<command line>` segments (system headers included),
   restricted to candidates, and every file its line markers name.

The language is the file's (`c`, `c++`, `assembler-with-cpp` for `.S`); a
`.s` file is assembled without the preprocessor and contributes nothing.

**The view** (`PredefinedMacros::derive`). The facts decide nothing unless
their database SHA-256 is the census database's, every command's file and
compiler equal the database's in order, and every compiler identified with
the recorded version. Then, for a reserved candidate name in a read file:

- a file with commands: **defined** when every command starts with it and no
  directive of those units `#undef`s it; **undefined** when none starts with
  it and none `#define`s it; a failed derivation of any of its commands
  leaves it undecided;
- a file without a command (a header, the test): the same over every
  command, only when some preprocessed unit read the file and no derivation
  failed;
- anything else undecided.

**The hook** (`MacroView::defined`). Only where the configured reading
itself is undecided: a decided *defined* stands unless a read file
`#undef`s the name, a decided *undefined* unless a read file or generated
header `#define`s it. The `FLB_` rules and the directive reader (`defined`,
`!`, `&&`, `||`, parentheses; anything else undecided, so `__GNUC__ >= 4`
keeps both arms) are unchanged. Each decision is reported per file
(`predefined_macros.decisions`), and `--list-functions` lists a reading's
functions and edges for attribution.

## 3. The derived facts

[`predefined_facts.py`](scripts/predefined_facts.py) ran both readings'
derivations (the fixed and the previous test as the extra unit, each
borrowing `src/flb_lib.c`'s command with the configured test header):

- 1,421 commands (1,402 C, 16 C++, 2 `.S`, 1 `.s`), 156 candidates, 2,506
  covered files; compilers `/usr/lib/llvm-21/bin/clang` and `clang++`, both
  resolving to `/usr/lib/llvm-21/bin/clang` (SHA-256 `412bbe8c…`),
  `--version` equal to the build record's (Ubuntu clang 21.1.8,
  `x86_64-pc-linux-gnu`);
- **34 derivations fail**, every one on `-E -dD` over a source the prepared
  build never generated (luajit's `lj_libdef.h` and `lj_vm.S`, the bison and
  flex outputs of the record accessor, stream processor and SQL processor,
  `src/flb_dump.c`, `src/fluent-bit.c`). Their files stay undecided, and so
  does every file without a command: at the export **no header and not the
  test** is decided;
- `_WIN32` probes undefined for every command and no preprocessed file
  `#define`s it; `__linux__` probes defined for every C command that
  derived (1,369).
- The two derivations are byte-identical (`9dd8bb73…`: neither test names a
  reserved macro in a condition, and an extra is keyed by its census path).

The first two attempts were discarded and retained under `private/pm/`: the
first compared `--version` byte for byte against the record, which stores it
without the last newline (every command failed closed: nothing decided); the
second printed each probe's name, which a builtin function-like macro
(`__has_attribute`) expands, so those probes failed. The derivation now
compares `--version` up to trailing white space (`same_version`) and prints a
probe's index, never its name.

## 4. Negative controls (fixtures first)

`crates/quirin-source-analysis/tests/loop_owner_predefined_macros.rs` (16
tests) and 6 tests appended to `loop_owner_instance_flow.rs`:

| Control | Outcome |
|---|---|
| a macro the compiler defines (`__linux__`) | its arm is read, the `#elif`/`#else` arms are not; under a set defining only `_WIN32`, only the Windows arm |
| no derived facts | every arm read, as before |
| `-D__QCORE_ONE__ -U__QCORE_ONE__`, `-U__clang__` (real Clang) | **undefined** in that unit; defined in the unit without `-U` |
| a compiler whose `--version` is not the recorded one (real Clang and synthetic) | every command fails; **nothing decided** |
| an unavailable compiler | nothing decided |
| facts of another database, or a command renamed | nothing decided |
| `#if __GNUC__ >= 4` and `defined (__GNUC__) && __GNUC__ >= 4` | **both arms** read |
| a read file's `#define _WIN32` | both arms read |
| a preprocessed header's `#define`/`#undef` (real Clang and synthetic) | the name stays undecided in that unit |
| a file compiled twice, one command defining the name | undecided there |
| a header no unit read (a plugin the build does not compile) | undecided |
| one failed derivation | its file and every header undecided |
| a non-reserved predefined name (`linux`), a reserved name no condition names | undecided |
| the fixed-test shape (escaped creator; `localtime_r`/`dirname` shapes under `#if defined (__linux__) … #elif defined (_WIN32)` and `#ifndef _WIN32 … #else`) undecided, under a set defining `_WIN32`, with a file `#define`ing `_WIN32`, or with the unit's derivation failed | **unresolved** ("have no typed reading") |
| the same shape, `__linux__` defined and `_WIN32` undefined | **discharged** on both owners |
| the old test's shape under that set | **not discharged** (`T`) |

## 5. Census, corpus, system and slice

**BEFORE** (base binaries `loop_owner_census` `3ebc9090…`,
`typed_dispatch_facts` `6653cf8b…`, facts regenerated) reproduced the
W47-SERVICE-PROPERTY-KEYS archive byte for byte: all four census readings
(`983c74b9…`, `4955a12d…`, `d05ebc0e…`, `a5edf1ed…`), `library.json` and
`fixed-test.json`; `previous-test.json` differs only in the generated copy's
path.

**AFTER** (release binaries `loop_owner_census` `4c3b4128…`,
`predefined_macro_facts` `7265a7fa…`, `typed_dispatch_facts` `9c3b87e1…`;
typed facts byte-identical to BEFORE but the previous copy's path;
[`census.py`](scripts/census.py) runs six readings):

| Reading | sites discharged / not / unresolved | edges | functions | seeds | JSON |
|---|---|---|---|---|---|
| `90978ac6` configured | 8 / 130 / 10 → **11 / 130 / 7** | 33,573 → 33,529 | 11,951 → 11,932 | 2,854 → 2,852 | `719a7d0d…` |
| `90978ac6` raw | 8 / 126 / 10, unchanged | 33,493 | | 2,902 | byte-identical (`983c74b9…`) |
| `~1` configured | 8 / 136 / 7, unchanged | 33,546 → 33,503 | 11,949 → 11,930 | 2,854 → 2,852 | `418a9251…` |
| `~1` raw | 8 / 132 / 7, unchanged | 33,466 | | 2,902 | byte-identical (`d05ebc0e…`) |
| both configured, without the facts (control) | unchanged | | | | byte-identical to BEFORE (`4955a12d…`, `a5edf1ed…`) |

The unstructured files are unchanged (4): the files the locator of
PREDICTION.md structured are headers, which no reading decides here.

**Attribution** ([`site_diff.py`](scripts/site_diff.py),
[`attribute.py`](scripts/attribute.py); archived in
[artifacts/census/](artifacts/census/)). Decisions were made in 69 files.

- **Sites:** only the three fixed-test sites move, unresolved → discharged,
  by the two decisions of §1 (the instance flag's reason now names the
  list-head instance, holder escape, P-CALLBACK-INSTANCE and the property
  keys, and no blocker).
- **Functions:** 19 are no longer read, each defined only in a decided-out
  arm of a file with a decision: `localtime_r`, `gettimeofday`
  (`mk_utils.c`), `strcasestr`, `strndup` (`mk_string.c`), `flb_timezone`
  (`flb_strptime.c`), `gmtime_r`/`localtime_r` of `mk_clock.c` and
  `gmtime_r`/`localtime_r`/`timegm` of `mk_server/mk_utils.c`, `dirname`
  and `read_glob_win` of `in_calyptia_fleet.c` and `prometheus_textfile.c`,
  `flb_cf_yaml.c::dirname`, `maxminddb.c::utf8_to_utf16` and the three
  `win32_*` functions of `flb_sosreport.c`. No removed function lies in a
  file without a decision.
- **Edges:** 52 (fixed) / 51 (previous) fewer: the calls into those
  functions (external names now, as on Linux), calls inside the dropped arms
  (`flb_pipe.c`'s `evutil_*` Windows pipe, `mk_fifo.c`'s
  `mk_win32_socketpair`, …), and at the fixed reading the coroutine edge
  `input_pre_cb_collect → cb_tail_pause_test` the underived instance flag
  kept. 8 edges are new, all from `main` of libmaxminddb's `mmdblookup.c`
  command-line tool, whose `#ifdef _WIN32 int wmain(…) { #else int main(…)
  { #endif` header is read as one `main` definition once the `wmain` arm is
  decided out; `main` is reached from no root.

No site moves Safe → non-Safe (the census publishes no `Safe`), and no
correction is named.

**Condvar/join and lifecycle census** (`w47-flb-lib-lifecycle`'s
`census.sh`, raw reading only): all four JSONs equal the archive once the
lifecycle module's worktree path is aliased
([`condvar_identity.py`](scripts/condvar_identity.py)). The old test's C5
status is unchanged.

**Arbiters** ([`arbiters.py`](../w47-loop-owner-typed-dispatch/scripts/arbiters.py),
`--jobs 8`; [`identity.py`](scripts/identity.py) against
`w47-flb-lib-lifecycle/integration/artifacts`, then compose3):

- **BEFORE** at the base, binary `fa14f1e1…`: the corpus
  `resource-diagnostics.json` is byte-identical to the archive (`ae17152b…`);
  the system and slice differ only in the checkout-root hashes
  (`resource_contract_sources`, `inventory_sha256`) and module paths; all
  three compose with 0 mismatches.
- **AFTER** at `8c6d1e3f`, binary `8d328e7b…`: every
  `resource-diagnostics*.json` sidecar of the corpus, system and slice is
  **byte-identical to BEFORE** ([`before_after.py`](scripts/before_after.py));
  only output paths, timings, the commit and binary hash (`run.json`,
  `audit.log`, `console.log`, `invocation.json`) differ, and the corpus
  parity stream is equal once sorted. Composed with the archive: corpus
  526 / 10 / 4,298, system 31 / 2 / 201, slice 24 / 1 / 244 / 1 Pending, 0
  mismatches each. Nothing moves: the census is on no audit route.

Logs: [artifacts/arbiters/](artifacts/arbiters/). No new baseline is
archived, since nothing moved.

## 6. Premises of the three discharged sites

Stated as: **discharged at `tests/runtime/in_tail.c:2483`, `:2492`, `:2494`
(the `90978ac6` export, configured reading) under these derived premises**,
never a verified fix. Derived at this path: the owners (`flb_lib_worker`,
`input_thread`), the coroutine hop and its resumers, the typed dispatch, the
list-head instance (one creation `flb_input(…, "tail", …)` at `:226`), the
holder escape, P-CALLBACK-INSTANCE, the property keys of `:218`, and the two
predefined-macro decisions of §1.

Stated, not proved (trust boundary 28), carried from rule 10p's cards:
P-CORO-HANDLE, P-PLUGIN-DIR, P-HEAD-INSTANCE, P-FRESH-CONTEXT,
P-REGISTERED-DESCRIPTORS, P-HOLDER-OPAQUE, P-INSTANCE-FIELDS-BY-MEMBER,
P-KEY-FOLD, P-INSTANCE, the immutable threaded flag, acutest's `TEST_CHECK`,
the lexical and typed readings' faithfulness (the recorded commands are the
program's; the test configured as the library's common set and typed with
its arguments), and the unmodelled premise that macro invocations on the
list-head region call no hazard (`W47-LOOP-OWNER-REGION-MACRO-CALLS`, which
this discharge now rests on). Added by this card:

- **P-RECORDED-COMPILER**: the compiler at the recorded path, with the
  recorded `--version`, behaves on the probe and the preprocessed units as it
  did in the build;
- **P-DD-COMPLETE**: Clang's `-dD` prints every `#define`/`#undef` it
  processes, and its line markers name every file a unit reads.

## 7. Lean

`proof/QCore/LoopOwner.lean`, audited in `proof/AuditLoopOwner.lean`: 66 →
81 theorems, no new audit module, no placeholders, no project axioms. A unit
starts from its probed set and applies its files' directives in order:

- **`a_predefined_macro_behaves_as_an_owned_macro`**: a decided name agrees
  with its definedness at every point of every unit of the file (with
  `definedAfter_without_undef`/`_without_define`);
- **`the_condition_reading_agrees_with_the_preprocessor`**: the three-valued
  reading of `defined`/`!`/`&&`/`||` agrees with the preprocessor's value
  whenever the names' decisions do (with `agrees_neg`, `agrees_conj`); with
  `the_taken_arm_is_read` the taken arm is kept;
- **`an_undecidable_comparison_keeps_the_arm`** (`keptArms_unknown_head`);
- **`a_compiler_mismatch_decides_nothing`** and
  **`a_compiler_mismatch_keeps_every_arm`** (`readCond_without_decisions`,
  `keptArms_all_unknown`);
- **`a_minus_u_undefines`**; **`a_file_directive_wins`**;
- **`the_old_test_path_is_never_discharged_whatever_the_arms`**.

## 8. Follow-ups

[`followups.json`](followups.json), applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): 566 → 568.

- **Retired:** `W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS` (its DISCHARGE clause
  is met: the reading decides each unit's predefined macros from the recorded
  compiler and flags, never guessing, measured at both revisions with the old
  test still not discharged).
- **Registered:**
  - `W47-PREDEFINED-FACTS-UNGENERATED-SOURCES`: 34 derivations fail on
    sources the prepared build never generated, so no header and not the
    test is decided;
  - `W47-CONFIGURED-READING-MULTI-COMMAND-OWNED`: the configured reading's
    `FLB_` rule intersects a file's commands' `-D` sets, so a name only some
    commands define is decided undefined: `FLB_SAMPLING_BENCH` in
    `plugins/processor_sampling/sampling_tail.c` (the benchmark target's
    command), 4 files with differing sets; a candidate soundness gap of an
    earlier card's reading, no site moves on it today;
  - `W47-PREDEFINED-VALUE-COMPARISONS`: 30 conditional directives of census
    files compare a reserved macro's value and stay undecided.
- **Amended:** `W47-LOOP-OWNER-COROUTINE-POST-YIELD`,
  `W47-LOOP-OWNER-INSTANCE-FLOW` and `W47-LOOP-OWNER-REGION-MACRO-CALLS` (the
  discharge now rests on it).

Sizes: [`residuals.py`](scripts/residuals.py).

## 9. Gate

GATE

**Files likely to conflict:**

- `docs/agents/gateway.md` (the 10p amendment), `docs/agents/README.md` (the
  10p row), `docs/STATUS.md`, `docs/development/increments/wave-47.md`,
  `docs/reports/INDEX.md`, `docs/reviews/r0_3/acceptance_matrix.json`;
- `crates/qcore-c-gateway/src/thread_entry_census.rs` (the child module,
  `BuildConfiguration::predefined`, `MacroView`,
  `CensusReading::predefined_decisions`);
- `crates/quirin-source-analysis/examples/loop_owner_census.rs`,
  `tests/loop_owner_instance_flow.rs`, `tests/loop_owner_coroutine.rs` (one
  field);
- `proof/QCore/LoopOwner.lean`, `proof/AuditLoopOwner.lean`.

No contract, provider, module-count or resource-view pin changes.

## Reproduce

```sh
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py OUT FACTS_BINARY GENERATED_DIR
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/predefined_facts.py OUT PREDEFINED_BINARY GENERATED_DIR
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/census.py OUT CENSUS_BINARY TYPED_FACTS PREDEFINED_FACTS
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/site_diff.py BEFORE.json AFTER.json
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/attribute.py OUT fixed|previous
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/arbiters.py OUT COMMIT SLICE_RUN_NAME
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/identity.py RUN OUT
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/condvar_identity.py RUN
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/residuals.py PREDEFINED_FACTS/fixed.json
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/gate.py LOGS rust|records
```
