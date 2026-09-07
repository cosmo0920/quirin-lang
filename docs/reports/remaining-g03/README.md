# G03: configuration-only addition of a second source set

Date: 2026-09-07. Workstreams W2/W5, integration. This records a bounded
demonstration that a *new* pinned source set can be audited through the
existing configured route with **no change to any analysis code**, plus the
two generalization gaps that attempt surfaced. It certifies no library: every
verdict below is a bounded resource projection under declared contracts,
`Unknown` stays in the denominator, and a provisional `Unsafe` is never
presented as an upstream defect. No raw unit is promoted.

## Chosen source set and pin

**`chunkio-core`** — ChunkIO, the embedded chunk I/O library vendored in the
pinned Fluent Bit checkout. It is a real, small C library, distinct from
CMetrics, with its own source root, its own compiled translation units and its
own build commands.

| Property | Value |
|---|---|
| Source set | `chunkio-core` |
| Pin | `fluent-bit` `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` |
| Clean paths | `lib/chunkio` |
| Checkout | `build/cycle19/fluent-bit-clang-source` (independent pinned clone) |
| Source root | `lib/chunkio/src`, `top_level` unit selection |
| Audited files | 13 digest-pinned `.c` files |
| Compile database | `build/cycle19/fluent-bit-clang-production/compile_commands.json` — CMake's **unmodified** output |
| Resource contracts | the existing `contract/cycle19-resource-contracts.json` (5 modules) |
| Provider binding | none (`provider: null`); no source-contract route configured |

The compile database is the same unmodified Clang GNU17 CMake database that
`scripts/prepare_cmetrics_clang_audit.py` already produces. The new source set
selects a different library and a different source root out of it. Its
ChunkIO commands carry exactly the four frontend flags the gateway transports
(`-Wall`, `-g`, `-fPIC`, `-Wa,--noexecstack`), so no option projection had to
change.

## Configuration files (the entire change to the audit route)

Two new data files, both under `contract/profiles/chunkio-core/`:

- `profile.json` — schema v1 source-set manifest: pin, clean path, source
  root/selection, the 13 audited-file digests, empty witness/mechanism/
  disposition/build/evidence inventories, `boolean_field_models: []`,
  `provider: null`, and `resource_contract_manifest`
  `"../../cycle19-resource-contracts.json"`.
- `audit.json` — schema v1 audit configuration: profile directory, checkout,
  compile database, `libraries: ["chunkio"]`, report path.

No new contract module was needed: the existing corpus manifest already
declares the libc/Monkey/librdkafka/Fluent Bit/ChunkIO allocation and
open/close contracts this set calls.

## Reproduce

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-unit
python3 -B scripts/prepare_cmetrics_clang_audit.py --fluent-bit ../fluent-bit
cargo run --quiet -p qcore-audit -- contract/profiles/chunkio-core/audit.json
```

## Audit output

```
13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown / 0 data-only / 0 gateway failures
```

Keyed per-unit result (`audit/stdout.log`, `audit/chunkio-core-audit.json.gz`,
`audit/chunkio-core-audit.assumptions.json`):

| Unit | Verdict | Projected / source functions | Diagnostics |
|---|---|---|---|
| `lib/chunkio/src/chunkio.c` | provisional Unsafe | 13/13 | leak=3, unknown_alias=4, unsupported=96 |
| `lib/chunkio/src/cio_chunk.c` | Unknown | 30/30 | unknown_alias=12, unsupported=159 |
| `lib/chunkio/src/cio_error.c` | Unknown | 4/4 | unsupported=6 |
| `lib/chunkio/src/cio_file.c` | provisional Unsafe | 32/32 | leak=1, unknown_alias=22, unsupported=306 |
| `lib/chunkio/src/cio_file_unix.c` | Unknown | 15/15 | unknown_alias=8, unsupported=103 |
| `lib/chunkio/src/cio_log.c` | Unknown | 2/3 | unknown_alias=1, unsupported=11 |
| `lib/chunkio/src/cio_memfs.c` | Unknown | 5/5 | unknown_alias=3, unsupported=42 |
| `lib/chunkio/src/cio_meta.c` | Unknown | 4/4 | unsupported=40 |
| `lib/chunkio/src/cio_os.c` | Unknown | 2/3 | unknown_alias=1, unsupported=9 |
| `lib/chunkio/src/cio_scan.c` | Unknown | 3/3 | unknown_alias=3, unsupported=39 |
| `lib/chunkio/src/cio_stats.c` | Unknown | 2/2 | unsupported=23 |
| `lib/chunkio/src/cio_stream.c` | Unknown | 8/8 | unknown_alias=13, unsupported=58 |
| `lib/chunkio/src/cio_utils.c` | Unknown | 3/4 | unknown_alias=4, unsupported=28 |

Per-function status across the set: 1 `Safe`, 2 `Unsafe`, 120 `Unknown`.
Three functions are not projected at all and are listed under `not_projected`
in the report rather than dropped. This is an Unknown-heavy result and that is
the expected, valid outcome: it is a withdrawal of unsupported claims, not a
finding about ChunkIO.

**No raw unit here is promoted.** These units also appear in the pinned
Cycle 19 corpus under a different profile and orchestration; this run is an
independent configured projection, not a re-measurement of that corpus.

## Fail-closed checks

`failclosed.py` copies the checked-in profile into a scratch tree, mutates
exactly one declared input, and requires that the audit reject and write no
report. Output in `failclosed.log`:

| Case | Result | Message |
|---|---|---|
| control, unmutated | ACCEPTED | `13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown / 0 data-only / 0 gateway failures` |
| changed source pin | REJECTED | ``pin `fluent-bit`: expected 000…001, found 66910c10a4d7eafa810b84229cf2dcf7b0f26f97`` |
| changed audited-file digest | REJECTED | `DigestMismatch { path: "lib/chunkio/src/chunkio.c", … }` |
| unsupported compiler flag (`-Wextra`) | REJECTED | ``compiler option `-Wextra` cannot be preserved by the bounded gateway`` |
| absent provider package | REJECTED | `source-contract package: No such file or directory` |

Unknown source forms are not silent: every unmodeled call, indirect access or
unresolved alias appears as a keyed diagnostic on its function, and the unit's
`reason` states the unresolved counts (see the table above and the report).

These are also pinned as hermetic regressions in
`crates/qcore-audit/tests/second_source_set.rs` (5 tests, all passing), over a
synthetic pinned repository with the same nested-root shape:

- `a_nested_root_source_set_publishes_keyed_verdicts_from_configuration_alone`
- `an_unknown_source_form_is_reported_with_a_reason_rather_than_omitted`
- `a_changed_source_pin_rejects_before_any_unit_is_published`
- `an_absent_provider_package_rejects_the_configured_source_contract_route`
- `a_compiler_option_the_gateway_cannot_preserve_rejects_the_database`

## Keyed comparison against the existing bounded routes

`compare.py` writes `comparison.json`: per-unit verdicts/reasons/coverage/
diagnostic categories, per-function status, report keys and assumption keys
for three routes — the new `chunkio-core` set, the pinned `cmetrics-counter`
route and the archived `w5-configured-audit` synthetic repository.

| | chunkio-core | cmetrics-counter | w5 synthetic demo |
|---|---|---|---|
| Units | 13 | 1 | 3 |
| Verdicts | 2 Unsafe / 11 Unknown | 1 Unsafe | 1 Safe / 1 Unsafe / 1 Unknown |
| Function status | 1 Safe / 2 Unsafe / 120 Unknown | 1 Unsafe / 6 Unknown | 1 Safe / 1 Unsafe / 1 Unknown |
| Report keys | identical set | identical set **+ `source_contracts`** | identical set |
| Assumption keys | identical set | identical set | identical set |
| `scope` string | "no observation or provider composition; Unsafe is provisional" | "separate configured source-contract evidence is embedded … Unsafe is provisional" | same as chunkio-core |
| Pin recorded | `pin.fluent-bit=66910c10…` | `pin.fluent-bit=66910c10…` | `pin.sample=78a0ea53…` |
| Declared contracts | the 5 corpus modules, verbatim | the same 5 modules, verbatim | one 2-declaration demo module |

The only structural difference is the `source_contracts` block, present
exactly when a source-contract route is configured; `chunkio-core` binds no
provider, so it correctly has none rather than an empty or optimistic one. All
seven report keys and all seven assumption keys are otherwise shared, and both
Fluent-Bit-pinned routes record byte-identical declared contract text. The new
set therefore behaves like the existing routes on every keyed dimension the
report exposes, differing only where its configuration differs.

## What code had to change

**None.** The diff to `crates/` and `compiler/` contains only new test files:

- `crates/qcore-audit/tests/second_source_set.rs` (new test)
- `crates/qcore-audit/tests/fixtures/alloc_release.qcore` (new test fixture)

No production Rust file, no function-name table and no closed enum was
touched. The `chunkio-core` audit runs from `profile.json` + `audit.json` plus
already-present contract data. Verified by construction — the complete working
tree before commit was:

```
?? contract/profiles/chunkio-core/
?? crates/qcore-audit/tests/fixtures/alloc_release.qcore
?? crates/qcore-audit/tests/second_source_set.rs
?? docs/reports/remaining-g03/
```

No pinned expectation was updated: the contract corpus stays at 59 modules
(the new set reuses the existing manifest), the submodule revision pin is
unchanged, and no diagnostic was downgraded.

## Findings: two generalization gaps (not repaired here)

The first candidate for the second source set was the repository's own pinned
**Libevent** submodule (`third_party/libevent`, `d82464a277d0f42703702c4dfd9af6af38595a83`,
release-2.2.2-alpha-11). It is a genuinely separate pinned checkout, so it is
the stronger test. It could **not** be expressed as configuration. The
attempted profile, contract module, audit configuration and build script are
archived unchanged under `libevent-core/`, together with the real CMake build
provenance and a sample database entry.

**F1 — a source set whose C sources live at the checkout root cannot be
selected.** `LibraryProfile::source_roots[].root` is validated by
`is_relative_without_traversal`, which admits only `Component::Normal`
segments, so `"."` is rejected — even for `UnitSelection::Exact`, where the
loader and `select_profile_units` never read `root` at all. Libevent compiles
`log.c`, `select.c`, `watch.c`, … directly from its repository root, so the
whole set is unrepresentable. Reproduce:

```sh
cargo run --quiet -p qcore-audit -- docs/reports/remaining-g03/libevent-core/audit.json
# audit rejected: inconsistent profile manifest:
#   library `libevent-core` source root `.` must be relative
```

This is a schema/validation gap, not a fail-closed protection: it does not
prevent an unsound result, it prevents a sound one. Repairing it means
deciding what "checkout root" means in `SourceRootSelection` — a schema
decision with a `PROFILE_MANIFEST_VERSION` consequence — so it was **not**
implemented here under the config-only constraint.

**F2 — a second real CMake C project's unmodified database is not consumable.**
`compile_db::compiler_options` accepts four frontend flags
(`CFrontendFlag::{AllWarnings, DebugInformation, PositionIndependentCode,
AssemblerNoExecStack}`) and rejects everything else. Libevent's own
`CMakeLists.txt` appends 18 further options unconditionally under GNUC
(`-Wextra`, `-Wstrict-prototypes`, `-Wdeclaration-after-statement`,
`-fno-strict-aliasing`, `-fvisibility=hidden`, `-O3`, …), so every one of its
55 database entries rejects. Libevent also emits several entries per file for
different targets, which `select_profile_units` correctly rejects as
conflicting commands. Extending the accepted set is a new feature (each flag
needs a preservation argument through preprocessing/syntax/typed AST), not
data plumbing, so it was **not** implemented here. `chunkio-core` clears this
bar only because Fluent Bit's build happens to use exactly the four
transported flags.

Neither gap was worked around by weakening a gate. `libevent-core/` is kept as
reproducible evidence, deliberately **not** installed under
`contract/profiles/`, because it does not currently load.

## Checks

The canonical CMetrics audit was run before and after this change from the
same prepared build. Both runs report `1 checked function contract(s) /
6 Unknown` and `1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown`, and both
artifacts are byte-identical:

```
99137e51e1edd1cec7ea28c65c4cb781de169190ec3e82d6aa622eda6c82b511  cmetrics-audit.json
827af29d5ccb9f0197d972b5f07518acee0783331781507c412466e49f50ca5a  cmetrics-audit.assumptions.json
```

The full standing per-change gate is `gate.sh`; its combined output, with a
recorded exit code for every section, is `gate.log`. All twelve sections exit 0:

| Section | Exit | Result |
|---|---|---|
| `cargo test --workspace` | 0 | **824 passed, 0 failed** (819 before + the 5 new tests) |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `reevaluation_probe` | 0 | — |
| `reevaluation_followup` | 0 | — |
| provider tests (`providers`) | 0 | **45 passed** |
| Python trace tests | 0 | **43 passed** (`Ran 43 tests … OK`) |
| Lean audits (`check_lean_audits.py`) | 0 | **20 audits** |
| `lake build` | 0 | — |
| `chunkio-core` audit | 0 | 13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown |
| `cmetrics-counter` audit | 0 | 1 checked / 6 Unknown; 1 unit provisional Unsafe |
| G03 fail-closed mutations | 0 | control accepted, all four mutations rejected |

Submodules were initialized with
`git -c protocol.file.allow=always submodule update --init --recursive`.

## Open scope

This demonstrates that a second pinned source set with an already-supported
layout and an already-supported flag set is pure configuration. It does not
show that an arbitrary C project is: F1 and F2 are the two concrete barriers,
and provider-backed source-contract evidence for a new set still requires a
provider that can execute its model. Full lifecycle source import, specialized
provider extraction and registry-based composition remain open.
