# CORPUS-REPIN — re-measurement and deliberate re-pin of the four 185-unit artifacts

> **Superseded as the current baseline, 2026-09-09:** card O04-ARMS re-measured
> the corpus through this same pinned route before and after its analyzer
> change; the *before* run reproduces `artifacts/` here byte-identically for
> `diagnostics.json`/`chunk-contracts.json`, and the *after* run — headline
> counts unchanged, no verdict moves, diagnostics moved in 4 units in the
> fail-closed direction — is the current baseline at
> `docs/reports/remaining-o04-arms/artifacts/`, with the complete diffs
> archived beside it. This directory remains the wave-6 measurement record.

**Date:** 2026-09-08. **Base:** `a63fbae` (*Update spec §L21.1 prose: the
reason-code vocabulary is implemented*). **Card type:** measurement, run
deliberately as a **pin-moving** card under hard invariants 3 (parity before
deletion) and 5 (pinned expectations are intentional).

The four archived 185-unit Cycle 19 artifacts had not been re-measured since
`w5-cmetrics-unit-provider`. Five landed changes had accumulated against them —
C02 (typed scalar-field access classification, in shared `legacy_c.rs`), G02
(declared-input transport), O04, C03-NFR (neutral-frame recursion) and
REASON-CODES (a new `reason_code` field on report units) — and the archived
number (42 provisional `Unsafe`) disagreed with two later measurements (43).

**Headline, pinned route, 185 units: 2 Safe / 42 provisional Unsafe /
137 Unknown / 4 data-only.** Identical to the archived pin. **No unit changed
verdict in either direction and no raw unit is promoted.** Twenty-three
*functions* move `Unknown` → `Safe`; that is C02's precision gain, is recorded
in §5 with its discharging rule, and is a function-level result, not a unit
promotion.

**42 vs 43 is resolved and is not an analysis disagreement** (§4): the two
numbers count different things. The pinned route emits one row per source file
(185); the current route emits one row per accepted compile configuration
(186), and `src/flb_input_chunk.c` has two configurations, both provisional
`Unsafe`. The set of distinct provisional-`Unsafe` **files** is 42 under both
routes.

---

## 1. What was run

| | |
|---|---|
| Fluent Bit pin | `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` |
| Upstream checkout | `/home/cosmo/Documents/GitHub/fluent-bit` — **read only**; its HEAD (`864c3d97f`, the user's own branch) was never moved, reset or checked out |
| Independent clone | `build/cycle19/clang/66910c10…/fluent-bit-clang-source` (fresh `git clone --no-hardlinks`, detached at the pin) |
| Compiler | `Ubuntu clang version 21.1.8 (6ubuntu1)`, `/usr/bin/clang`, target `x86_64-pc-linux-gnu` |
| Libevent submodule | `d82464a277d0f42703702c4dfd9af6af38595a83` (initialised for this run; see §7.1) |
| Monkey | forced-Libevent preparation at the pin, 142 compile entries |
| Provider packages | rebuilt for this run into `build/cycle19/provider-packages-repin` |
| Host | Linux 7.0.0-30-generic x86_64, CMake 4.2.3, rustc 1.98.0, Python 3.14.4 |

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 --clang clang --clangxx clang++
python3 -B providers/package.py build --output build/cycle19/provider-packages-repin

# (a) pinned compatibility route — the shape of the four archived artifacts
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-repin/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <reports>/monkey-source.json <reports>/resource-diagnostics.json

# (b) current route — the 186-configuration inventory, for §4
QCORE_CLANG=/usr/bin/clang \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --compile-database <build>/compile_commands.json <clone> <monkey>/compile_commands.json \
  <reports>/monkey-source.json <reports>/resource-diagnostics.json
```

Both exit 0 with zero gateway failures. Full provenance, digests and the exact
argument vectors are in [`provenance.json`](provenance.json); the logs are under
[`logs/`](logs/).

### 1.1 One deliberate input substitution, and its cost

The archived run consumed the developer checkout's **own ad-hoc GCC build
database** (`/usr/bin/cc`, configured with `FLB_DEV`, `FLB_TESTS_INTERNAL`,
`FLB_TESTS_RUNTIME`, `FLB_IN_EBPF`, `FLB_AWS`, `FLB_WAMRC`,
`FLB_PREFER_SYSTEM_LIBS`, `FLB_SIMD`, …). That checkout has moved off the pin,
and the historical configuration is recorded nowhere in this repository, so it
is **not reproducible**. This card therefore uses the reproducible
explicit-Clang CMake build that `scripts/prepare_cycle19_clang_audit.py`
prepares at the pin, and feeds the **same database to both routes**, so the two
routes differ only in route logic.

The pinned compatibility route reads `<root>/build/compile_commands.json`, so
the prepared database was copied there verbatim. No gate was bypassed, no
compile command was rewritten, and the pinned route's own historical option
projection (`compiler_options_compatibility`, `CSourceInput::HistoricalStdin`)
is untouched — it reads only `-I`, `-isystem`, `-D` and `-std=` and never the
compiler program.

Every consequence of the substitution is itemised and bounded:

| Consequence | Extent | Semantic effect |
|---|---|---|
| CMetrics selects its atomic module by compiler ID, so `cmt_atomic_gcc.c` is replaced by `cmt_atomic_clang.c` | 1 unit of 185 | none: same four functions, same per-function statuses (`Safe`, `Unknown`×3), same `unsupported: 3`, same reason text |
| `flb_input_chunk.c` preprocessed lines shift by a constant `−6` | 1 unit, 907/907 diagnostics matched | none: every diagnostic matches 1:1 after the offset |
| `monkey-source.json`'s `compile_entry` records `/usr/bin/clang` instead of `/usr/bin/cc` | 1 envelope field | none |

The 159 other units keep **identical preprocessed line numbers**, which is the
direct evidence that the database swap did not perturb them.

---

## 2. The re-pinned artifacts

The new canonical set is [`artifacts/`](artifacts/), with per-file digests in
[`artifacts/manifest.json`](artifacts/manifest.json):

| Archived stem | Produced by | Uncompressed bytes | Baseline (`w5-cmetrics-unit-provider`) | Byte-identical |
|---|---|---|---|---|
| `diagnostics.json` | `resource-diagnostics.json` | 10,741,560 | 11,006,128 | no |
| `source.json` | `monkey-source.json` | 1,665,474 | 1,663,331 | no |
| `chunk-contracts.json` | `resource-diagnostics.chunk-contracts.json` | 4,141,225 | 4,141,225 | no |
| `source-effects.json` | `resource-diagnostics.source-effects.json` | 114,479 | 114,434 | no |
| `contracts.json` | `resource-diagnostics.contracts.json` | 8,168 | not archived before | — |
| `current-route-diagnostics.json` | route (b), 186 configurations | 16,222,549 | — | — |

**The predecessor copies are not overwritten.** Each report under
`docs/reports/` is the evidence of its own run; `w4-nullable-removal` and
`w5-cmetrics-unit-provider` keep theirs, exactly as `w5-cmetrics-unit-provider`
kept `w4-nullable-removal`'s. Re-pinning means the *newest baseline pointer*
moves: a future audit-parity record should name
`docs/reports/corpus-repin/artifacts/<stem>.json.gz`, the way
`docs/reports/w2-preprocessor-transport/audit-parity.json` names the
`w5-cmetrics-unit-provider` copies today.

### 2.1 Reproducibility check inside this card

The pinned route was run twice on identical inputs, once with the in-process
`Direct` source-contract configuration and once with the process package
selected by `QCORE_CHUNKIO_PACKAGE`. `resource-diagnostics.json`,
`resource-diagnostics.chunk-contracts.json`,
`resource-diagnostics.contracts.json` and `monkey-source.json` are
**byte-identical** between the two; the package run additionally emits
`resource-diagnostics.source-effects.json`, which only the process route
produces (`process_mode` in the compatibility adapter). The re-pinned set is
the package run.

---

## 3. Per-artifact diff

Machine-readable: [`diff.json.gz`](diff.json.gz) (29 MB uncompressed, every
changed leaf verbatim), produced by
[`diff_artifacts.py`](diff_artifacts.py); the causal roll-up is
[`attribution.json`](attribution.json) from
[`attribute_changes.py`](attribute_changes.py); the compact roll-up is
[`summary.json`](summary.json).

### 3.1 `diagnostics.json` — 185 units

| | |
|---|---|
| Status histogram | `{Safe 2, Unsafe 42, Unknown 137, data_only 4}` → **identical** |
| Unit-level verdict changes | **0** |
| Units added / removed | 1 / 1 (`cmt_atomic_clang.c` for `cmt_atomic_gcc.c`, §1.1) |
| Units with any field change | 172 of 184 common |
| Envelope keys changed | `diagnostic_counts`, `reason_code_meaning` (new) |
| Total diagnostics | `unsupported` 49,867 → 48,318 (−1,549); `unknown_alias` 4,199 → 4,166 (−33); `leak` 271 → **271 (unchanged)** |

Changed-field signatures over the 172 units:

| Fields that changed | Units |
|---|---|
| `diagnostic_counts`, `functions`, `reason`, `reason_code` | 108 |
| `diagnostic_counts`, `functions` | 35 |
| `reason_code` only | 26 |
| `functions`, `reason_code` | 2 |
| `functions` only | 1 |

No unit's `status`, `library`, `file`, `source_functions` or `not_projected`
changed.

### 3.2 `chunk-contracts.json` — 96 results, 21 backend-teardown records

Three rows changed, each in exactly **one leaf**:
`summary.profile_assumptions.profile_digest`, from
`7722bebd110c623870b6499ab21d4a53464ce9c0249b2ec04ab4957f0b1c358e` to
`5507e9f605522ac8ffbe8cc6dbe541f096169d531a3a6276c50344bf8a05f035`.
The `fluent-bit` source-set profile has been edited since the archived run; the
pin it declares (`66910c10…`) is unchanged. No status, path, write, dependency
or contract text moved. The 21 backend-teardown records are unchanged.

### 3.3 `source-effects.json` — 96 results

`{checked_function_contract 3, unknown 93}` → **identical**. The 93 `unknown`
rows are byte-identical. The three checked rows move in `task`, `witness`,
`evidence`, `bounds` and `import_identity`, all traceable to two facts:

* `import_identity.compiler.program` is `/usr/bin/clang` rather than `clang`,
  because the prepared route exports an absolute `QCORE_CLANG`. This changes
  `configuration_digest`, and with it the task digest and the obligation
  identifiers under `bounds` — exactly the dependency REASON-CODES §7 note 2
  predicted would be the only mover of these digests.
* the provider package was rebuilt for this run.

Neither is an analysis change: the same three functions are checked over the
same four Boolean-state combinations under the same eight declared assumptions.

### 3.4 `source.json` (Monkey callback source) — 3 callbacks, 15 wrappers

* envelope `compile_entry`: `/usr/bin/cc …` → `/usr/bin/clang …` (§1.1);
* all 18 rows: `profile_assumptions.profile_digest` moved, as in §3.2;
* 4 wrappers additionally changed `paths`, in **2 leaves each and only there**:
  a `char[93]` string literal holding the absolute path of the prepared Monkey
  source becomes `char[189]` holding this worktree's path. This is the embedded
  source-path provenance of the prepared tree, not analysis. A future
  preparation at another path changes exactly these four strings.

---

## 4. The 42-vs-43 resolution — named down to the unit

The archived artifacts record **42** provisional `Unsafe`; the explicit-Clang
rerun at `9f0fa573` and C03-NFR's re-measurement at this same pin both recorded
**43**. Neither number is wrong and neither is an analysis change.

* The **pinned compatibility route** (`run`) selects units from the compilation
  database and *deduplicates by source file*: 185 rows for 185 files.
* The **current route** (`run_current`) records one row per accepted **compile
  configuration**: 186 rows for the same 185 files.
* The extra configuration belongs to **`src/flb_input_chunk.c`**
  (`fluent-bit-chunk/flb_input_chunk.c`), which appears twice in the
  compilation database with two distinct commands. Both rows are provisional
  `Unsafe`, so it contributes **two** `Unsafe` rows to the current route and one
  to the pinned route: 43 = 42 + 1.

Measured, in this card, on the same corpus and the same compile database
(`summary.json` → `current_route`, and `artifacts/current-route-diagnostics.json.gz`):

| Route | Rows | Safe | provisional Unsafe | Unknown | data-only |
|---|---|---|---|---|---|
| pinned (`--pinned-evidence`) | 185 files | 2 | **42** | 137 | 4 |
| current (`--compile-database`) | 186 configurations | 2 | **43** | 137 | 4 |

The set of distinct provisional-`Unsafe` **files** is the same 42 in both. The
cause is the recorded compile-configuration ambiguity that card G01's read-only
census already documented ("the pinned Cycle 19 compatibility adapter still
chooses silently between two configurations for `src/flb_input_chunk.c`"); this
card measures its arithmetic consequence and does not repair it. Whether the
adapter *should* keep choosing silently is still open and still G01's.

The candidate causes named in the card brief are excluded by measurement:
C02 changes no unit verdict (§5), G02 cannot reach this route (its transport is
`HistoricalStdin` here, and 159 units keep identical preprocessed line numbers),
and the explicit-Clang command admission changes one unit's *identity*
(`cmt_atomic_*`) without changing any verdict.

---

## 5. Twenty-three functions move `Unknown` → `Safe` — flagged for the integrator

**No unit is promoted** and no *unit* moved to `Safe`. At function granularity,
23 functions move `Unknown` → `Safe` and **none moves in the other direction**:

| Library / unit | Functions |
|---|---|
| `chunkio/chunkio.c` | `cio_set_max_chunks_up` |
| `chunkio/cio_error.c` | `cio_error_reset` |
| `cmetrics/cmt_counter.c` | `cmt_counter_allow_reset` |
| `cmetrics/cmt_encode_prometheus.c` | `prom_fmt_init` |
| `ctraces/ctr_encode_opentelemetry.c` | `otel_span_set_dropped_attributes_count`, `otel_span_set_dropped_events_count`, `otel_span_set_end_time`, `otel_span_set_start_time` |
| `ctraces/ctr_link.c` | `ctr_link_set_dropped_attr_count`, `ctr_link_set_flags` |
| `ctraces/ctr_resource.c` | `ctr_resource_set_dropped_attr_count` |
| `ctraces/ctr_span.c` | `ctr_span_end_ts`, `ctr_span_event_set_dropped_attributes_count`, `ctr_span_kind_set`, `ctr_span_set_dropped_attributes_count`, `ctr_span_set_dropped_events_count`, `ctr_span_set_dropped_link_count`, `ctr_span_set_dropped_links_count`, `ctr_span_set_flags` |
| `librdkafka/lz4.c` | `LZ4_decoderRingBufferSize` |
| `librdkafka/rdkafka_error.c` | `rd_kafka_error_set_fatal`, `rd_kafka_error_set_retriable`, `rd_kafka_error_set_txn_requires_abort` |

**The discharging rule, named exactly.** Card C02's *typed direct scalar-field
access classification*
(`docs/reports/remaining-c02/README.md`; `crates/qcore-c-gateway/src/legacy_c.rs`):
a statement is a resource-projection no-op only under the full conjunctive
whitelist — no call, no `&`, `*`, `[`, `]`; every access a direct
`base -> field` or `base . field` on a declared single-star record pointer or
record value; the record a `struct` defined exactly once whose every member
declaration was read; the field's declared type non-pointer, non-array,
arithmetic or enumeration, without `volatile` or `_Atomic`; the access does not
continue; every other identifier an enumeration constant or a declared scalar
variable; and the base a name this function never binds to a resource identity.
C02's own report predicted exactly this corpus effect and named
`cmt_counter_allow_reset` as the CMetrics instance; the other 22 are the same
rule reaching the rest of the corpus for the first time.

**The arithmetic that supports the attribution** (`attribution.json`):

| Line-independent diagnostic delta | Count |
|---|---|
| Removed, total | 1,639 |
| — of which the three C02-target descriptions (`unsupported assignment expression` 776, `indirect memory access is outside the resource projection` 468, `unrecognized assignment destination` 294) | **1,538 (93.8 %)** |
| — of which `call to uncontracted function X` where `X` is one of the 23 newly-`Safe` functions (`prom_fmt_init` 5, `ctr_span_end_ts` 2, `rd_kafka_error_set_fatal`/`_retriable`/`_txn_requires_abort` 1 each, `ctr_span_kind_set` 1) | 11 |
| — of which `unknown_alias`, mostly enumeration-constant identities inside statements C02 now classifies | 45 |
| — of which `leak` | 45, exactly balanced by 45 added: the description embeds an acquisition line number, and the total `leak` count is **unchanged at 271** |
| Added, total | 57 (45 `leak` line-text, 12 `unknown_alias`) |

The 12 added `unknown_alias` rows are the honest other direction of the same
rule: with the surrounding statement no longer refused wholesale, an
unresolved identity that the refusal used to mask becomes visible. No verdict
moved because of them.

**Nothing else moved.** C03-NFR's route still has measured corpus reach 0 (it
reported byte-identical artifacts at this pin), O04 changes no verdict, and G02
cannot reach the pinned route.

---

## 6. Reason-code prediction versus actual

`docs/reports/remaining-reason-codes/predict_185.py` applied
`ReasonCategory::classify` to the archived bytes and predicted what the re-pin
would carry. Reproduced by [`compare_prediction.py`](compare_prediction.py)
(log: `logs/prediction-comparison.log.gz`):

| Artifact | Rows | Predicted distribution | Classifier over re-measured reasons | `reason_code` actually carried |
|---|---|---|---|---|
| `diagnostics.json` | 185 | `unsupported-syntax` 136, `null` 48, `incomplete-obligation-coverage` 1 | **identical** | **identical** |
| `source-effects.json` | 96 | `unsupported-syntax` 91, `null` 3, `incomplete-obligation-coverage` 2 | **identical** | **absent — no field** |
| `chunk-contracts.json` | 96 | `unsupported-syntax` 91, `null` 3, `incomplete-obligation-coverage` 2 | **identical** | absent — as predicted |
| `source.json` | 0 reason-bearing rows | unchanged | unchanged | n/a |

**Category match rate: 377 / 377 rows (100 %) across all four artifacts.** The
single `incomplete-obligation-coverage` unit is the predicted one:
`librdkafka/tinycthread.c`, ``C function `_cnd_timedwait_win32` was not found``.
No re-measured row classifies to `UNCLASSIFIED`, no `residual-custody` row
appeared (no unit has a zero unsupported count), and no `Unknown` unit carries a
`null` code.

**One divergence, in the prediction's carrier column, not in its numbers.**
REASON-CODES §7 lists `source-effects.json`'s carrier as
`results[].reason_code` (96 rows). The re-measured artifact carries **no**
`reason_code` field there. The cause: carrier 2 is
`crates/qcore-audit/src/report.rs`, which serialises the audit report's
`units[]` (that is `diagnostics.json`) and `source_contracts.units[].functions[]`;
`source-effects.json` and `chunk-contracts.json` are assembled ad hoc by
`compiler/examples/support/cycle19_compatibility.rs`
(`"kind": "provider_source_effect"` literals), which REASON-CODES did not
touch. So `source-effects.json` belongs with `chunk-contracts.json` in the
"third carrier, not implemented" row — follow-on **REASON-CODES-B3** covers
both. This is a documentation error in §7's table, not a defect in the
implementation, and nothing was changed to make the numbers agree.

---

## 7. Deliberately updated pins

| Pin | Old | New | Why |
|---|---|---|---|
| Newest 185-unit baseline artifacts | `docs/reports/w5-cmetrics-unit-provider/{diagnostics,source,chunk-contracts,source-effects}.json.gz` | `docs/reports/corpus-repin/artifacts/` (plus `contracts.json.gz` and the current-route inventory) | this card's re-measurement; predecessors kept intact as their own cards' evidence |
| `docs/reviews/r0_3/acceptance.json` → `wave_2026_09_07.not_re_measured` | "the four archived 185-unit Cycle 19 artifacts … re-measurement plus deliberate re-pinning is the named CORPUS-REPIN follow-up" | records the completed re-measurement, the unchanged verdict counts, and what remains open | the statement it made is no longer true |

Nothing else in this repository pins those artifacts: no Rust or Python test
reads them, and the only references are documentation counts and historical
`audit-parity.json` baselines inside other cards' reports, which stay as
written. `docs/reports/remaining-reason-codes/predict_185.py` deliberately keeps
pointing at the archived bytes — it *is* the read-only prediction this card
checks, and repointing it would erase the comparison.

**Integrator-owned files this card did not edit** (they carry the headline
counts and must be updated by the integrator): `AGENTS.md`, `CLAUDE.md`,
`README.md`, `DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance_matrix.json`. The headline they should carry is
unchanged — **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only,
unpromoted** — with `CORPUS-REPIN` now closed and the baseline pointer moved to
this report. `docs/reports/remaining-d02/build_matrix.py` still records
`provisional_unsafe: 42` and is already flagged out of date by its own
`generator_status` field; it was not regenerated here.

### 7.1 One fail-closed refusal, recorded not worked around

The first pinned-route attempt failed at the Cycle 18 Libevent source check:

```
Fluent Bit library audit failed: could not verify Cycle 18 Libevent source:
  Pin(SourceRevisionMismatch { pin: "libevent",
      expected: "d82464a277d0f42703702c4dfd9af6af38595a83",
      actual:   "a63fbae77bad2998959e7946726aa7fe76779eb2" })
```

The `third_party/libevent` submodule was uninitialised in this worktree, so the
revision probe read the superproject's HEAD. The gate is correct and was not
weakened: the submodule was initialised (`git submodule update --init
--recursive`, the standing per-checkout step) and the run repeated. The refusal
log is archived as `logs/audit-pinned-route-submodule-refusal.log.gz`.

---

## 8. What this card does not establish

* **No promotion.** Raw Cycle 19 units remain 2 Safe / 42 provisional Unsafe /
  137 Unknown plus 4 data-only, unpromoted. A provisional `Unsafe` is a scoped
  resource projection under declared assumptions, **never** a proven upstream
  defect.
* **Not byte-parity with the archived run.** The archived run's GCC compilation
  database is unreproducible (§1.1). The equality established here is of
  *verdicts, counts and per-unit statuses*, plus an itemised, bounded account of
  every field that moved.
* **Not a new corpus.** The unit selection, profile, contract manifest and
  route are the existing ones. `contracts.json` is archived for the first time
  because it exists; it is not a new obligation.
* **G02's route migration stays open.** The pinned compatibility route still
  runs on `HistoricalStdin` by decision; this card measured through it and did
  not migrate it.
* **G01's configuration ambiguity stays open.** §4 explains the 43rd row; it
  does not repair the adapter's silent choice between two configurations.
* **REASON-CODES-B3 stays open** and now provably covers `source-effects.json`
  as well as `chunk-contracts.json` (§6).
* The prepared build trees live under this worktree's gitignored
  `build/cycle19/` and are not committed; `provenance.json` records their
  digests so the run can be repeated.

---

## 9. Validation

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1651 passed, 0 failed, 1 ignored** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **38 Lean audits pass** |
| `python3 -m unittest discover -s scripts/tests` | **108 tests, OK** |
| pinned-route audit (a) | exit 0, 0 gateway failures |
| current-route audit (b) | exit 0, 0 gateway failures |

No production Rust, Lean, contract or profile file was changed by this card:
its whole diff is this report, its artifacts and three analysis scripts, plus
the one `acceptance.json` sentence in §7. Logs for every gate are archived
under [`logs/`](logs/).
