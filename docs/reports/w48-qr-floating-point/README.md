# W48-QR-FLOATING-POINT: `f32` and `f64` in `.qr` sources

The card that comes before card 5 (`.qrlib` linkage tested with libm), base
`2ef0f8b5` (= local `main` = origin/main; the worktree's HEAD was already that
commit, so no reset or fast-forward was needed). `f32` and `f64`, IEEE 754
binary32 and binary64, join the §G02t table on the scalar route, the extern and
composed extern + io routes, the FFI route and the scoped-task route. Grammar:
implemented grammars [§G02t](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
("Floating point: `f32` and `f64`") and §G02s; semantics: specification §L14.0,
the new normative limitations
[§L14.7](../../QCORE_SPEC_r0_3.md#l147-floating-point-and-its-limitations),
§L20 and §L21; constraint 14's census entry. The prediction was frozen before
any source change ([PREDICTION.md](PREDICTION.md)).

## Mechanism

* **Kernel** (`crates/qcore-core/src/scalar_floats.rs`, new; dispatch in
  `scalar_types.rs`). `ScalarType::F32`/`F64` are **outside** `ALL` and outside
  `ScalarType::parse`; a position admits a float only through
  `parse_admitting_floats`, so every position this card did not widen keeps its
  refusal by construction. A float value is its IEEE **bit pattern** in the same
  `i128` slot every §G02t value uses (`contains` = a valid pattern, no integer
  range or width), so records, captures and certificates carry it unchanged.
  The kernel states IEEE `+ - * /` and negation with the host's operations,
  IEEE comparisons, the conversions, literal parsing (`FloatLiteral`: read once
  at each float type), the renderings, and the C spellings: hexadecimal float
  constants (a non-finite value through a union compound literal, no
  `<math.h>`), exact float → int threshold tests, the IEEE environment
  assertions and the renderers.
* **Lexer.** `DIGITS [. DIGITS] (e|E) [+|-] DIGITS` is one `Float` token when a
  digit follows the exponent marker; nothing else lexes differently.
* **Scalar route** (`scalar_calls.rs`, `scalar_calls_c_emit.rs`,
  `source_scalar_calls.rs`, `source_record.rs`): `Expr::Float`, literal typing
  by context (`f64` by default; a literal-only operand of `as` to a float type
  at that type), no `%`, `FloatEffect` in the row, the direct-use check while
  typing and the derived-row check in call-graph order; float record fields.
  Emission: one helper per float operation (`q_scalar_add_f64`), comparisons
  through the shared two-parameter helper, float → int through the checked
  conversion helper, `q_float_dec_T`/`q_float_hex_T` only where used.
* **Shared value statement** (`scalar_values.rs`, `source_value_statement.rs`):
  `Operand::Float`, the `Decider`'s `permitted`/`used` comparison effects
  (default: none, so a route that does not set them refuses every float
  comparison), float arithmetic that cannot raise, float → int that can; the
  reader reads float spellings only for a route that asks
  (`parse_value_expression_admitting_floats`), and the buffer route's
  consumer refuses one by name should it ever arrive.
* **Extern route** (`extern_call.rs`, `extern_c_emit.rs`, `source_extern.rs`,
  `source_std_io.rs`, `source_composed.rs`, `composed_std_io_c_emit.rs`):
  `AbiType::F32`/`F64` (`float`/`double`), `ExternArgument::Float`,
  `ExternEvent::BindFloat`, float `succeeds` outcomes read at the result type
  and compared **bit for bit** (`memcmp`), a float result logged without an
  integer conversion, the row's `partial_eq`/`partial_ord` (skipped by the
  composed std-io half's row), renderers on the composed route.
* **FFI route** (`ffi.rs`, `ffi_c_emit.rs`, `source_ffi.rs`,
  `contract_loader.rs`, `ffi_layout.rs`): `FfiValue::Float`, `struct!` fields
  and `#c_ffi` types read with floats, the layout probe covering `float` and
  `double`, bitwise outcome tests, renderers and the environment.
* **Task route** (`source_tasks.rs`, `task_execution.rs`, `reference.rs`,
  `task_c_emit.rs`, `task_trace_relation.rs`, `diagnostic_log_c_emit.rs`):
  `table_scalar` reads floats (parameters, results, captures, record leaves;
  closures lift onto the same workers), `TaskEffects::comparisons` feeds the
  checker's re-decision, float literals at typed bindings (an unannotated one is
  `f64`). A float worker result is logged as its bit pattern in `related`
  (`=F32`/`=F64`, result kinds 21/22) and traced as that number, **every NaN as
  `nan`** in the reference, in the sink's float arm (installed only where a
  float result exists) and in the F02 parser.
* **Driver/CLI**: `CompiledModule::float_assumption`, printed as
  `IEEE floating point: …` among the declared assumptions.
* **Lean**: `QCore.FloatTypes`, `AuditFloatTypes.lean` (20 theorems).

## Decisions as implemented

| Decision | Implementation |
|---|---|
| 1. Types | `f32`/`f64` at every position listed under *Positions*; C `float`/`double`. |
| 2. NaN and ±inf are values | IEEE arithmetic never faults; `x / 0.0` is ±inf or NaN; no float `%` (refused: it is libm's `fmod`). |
| 3. Comparisons gated by effects | `partial_eq` (`==`/`!=`), `partial_ord` (orderings), independent, never inferred. Scalar route: direct use checked against the row while typing, callees' derived rows in call-graph order. Extern route: the entry's one row. Task route: each function's own row (a spawn is not a call, as for `alloc`). Integer and `bool` comparisons ungated. |
| 4. Conversions | int → float and `f64 → f32` round to nearest, ties to even, total (overflow to ±inf); `f32 → f64` exact; float → int truncates, `OutOfRange` for NaN/±inf/out-of-range — refused when the value is known, else the typed error or exit 86 (tested in C against exact hex thresholds). Literals are read once at their type, including a literal converted to a float type. |
| 5. Spec limitations | §L14.7, 14 normative items (below). |
| 6. Output | `dec`: Rust's shortest round-trip digits in ECMAScript's `Number::toString` layout, **`-0` for negative zero** (chosen: the sign stays visible, unlike ECMAScript's `0`); `hex`: normalised C hex float (`0x1.8p+1`, subnormals `0x1p-1074`, zero `0x0p+0`); `nan`, `inf`, `-inf`. The emitted C uses a transcription of the Dragon4 shortest search Rust's own formatting agrees with, over a 40-limb bignum, and `memcpy` — no `printf` float conversion. |
| 7. Card 0's integers | unchanged: the fixture freeze below. |

## Positions

| Position | Status |
|---|---|
| §G02s parameters, results, locals, literals, arithmetic, comparisons, conversions, renderings, records of scalars (card 2) | **widened** |
| §G02d extern declarations, literal arguments, outcomes, float bindings, value statements; §G02i rendering; §G02d.1 unchecked declarations | **widened** (item 3: the extern ABI table has `f32`/`f64`) |
| §G02u `struct!` fields, `#c_ffi` parameters/results, literals, outcomes, `.qr` helpers | **widened** |
| §G02a/§G02j worker parameters, results, `copy` captures, value statements, record leaves (card 3); §G02v closures (card 4); the composed epoch route's task half | **widened** |
| `source.buffer.v1`: §G07.13 fields, conversions, value statements, loop bodies; the composed string route | refused, `W48-QR-FLOAT-BUFFER-ROUTE` (M) |
| §G02b `Buffer<f32/f64>`, §G02c `protected f32/f64` | refused, `W48-QR-FLOAT-PAYLOADS` (M) |
| float loop bound | refused on every route (the brief's rule) |
| `.qrlib` manifest / header spellings `float`/`double` | unchanged (card 5), `W48-QR-FLOAT-QRLIB-SPELLING` (S) |

## Specification limitations written (§L14.7)

1 NaN/±inf are values and ordering is partial (`x == x` can be false), enforced
by the effects; 2 NaN payload and sign neither preserved nor specified (every
NaN renders and traces `nan`); 3 signed zero (`-0.0 == 0.0`, different bits,
`-0`, outcomes compared bitwise); 4 silent precision loss in int → float and
`f64 → f32` (overflow to ±inf), literals rounded once; 5 float → int
`OutOfRange` for NaN, ±inf and non-fitting values; 6 subnormals kept, FTZ/DAZ
not allowed and undetectable by the program; 7 IEEE binary32/64 only, asserted;
8 rounding mode fixed to nearest-even, changes not modelled; 9 no excess
precision, `FLT_EVAL_METHOD == 0` asserted (x87 extended refused); 10 no
fast-math (refused at build), no contraction (one operation per helper,
`#pragma STDC FP_CONTRACT OFF` under Clang, `-ffp-contract=off` premise, MSVC
`/fp:strict`); 11 libm results not claimed identical across glibc, Apple libm
and UCRT; 12 no float `%`, no NaN/inf or hexadecimal float literal; 13
renderings are the language's, their native agreement observed not proved; 14
the positions not widened and the Linux/Clang-only native evidence.

## Corrections

| What | Before | After | Reason |
|---|---|---|---|
| `compiler/tests/unsafe_linkage.rs::a_spelling_outside_the_abi_vocabulary_refuses` | listed `f64` among spellings refused at `Q033` | lists `fnptr`; `f32`/`f64` now project and the call's integer literal is refused at `Q024` | the deliberate widening of item 3 |
| `compiler/tests/scalar_payloads.rs` `size()`, `crates/qcore-core/tests/unit/extern_call.rs` and the task tests' `TaskEffects` | exhaustive over the integer table / struct literals | gain the float arms / `float_effects`, `comparisons` fields | compile only; no behaviour change |
| 4 fixture messages (below) | the §G02t list without floats | lists `f32, f64` | the widened table, same code and exit |

## Evidence

**Fixture identity.** Card 0's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`80ad97b8…`, 701 files)
and at the gate tip (`595ecc99…`, 702 files)
([BEFORE](artifacts/fixtures-before.json), [AFTER](artifacts/fixtures-after.json),
[comparison](artifacts/fixtures-compare.txt)): **697 of 701 existing files
byte-identical** in exit, stdout and stderr of `check` and `emit`; the 4 others
change their refusal message only (same code, exit, span): the two
`usable_records/survey/ffi_*.qr` fixtures' `Q051` field-type list and
`remaining-v04`'s P08/P10 declaration-read `Q020` list, each now naming
`f32, f64`. The one new file is `examples/scalar/floating_point.qr` (check 0,
emit 0). No existing refusal became an admission and no admission a refusal.
A mid-card freeze (after the FFI route) differed only in the two survey rows.

**Native boundary table** (`compiler/tests/float_types.rs`, Clang 21.1.8,
[summary](artifacts/native-boundary.json), [Clang](artifacts/clang-version.txt)).
Values ±0, ±min subnormal, ±max subnormal, ±min normal, ±max, ±1, ±inf, a NaN,
and 2^24−1, 2^24, 2^24+2 (f32) / 2^53−1, 2^53, 2^53+2 (f64), 21 per type; every
`+ - * /` pair, negation, all six comparisons (each under its effect),
conversion to all ten integer types (plain and throwing), to the other float
type, and from every integer type's boundary values plus ±(2^24|2^53 + {−1,1,3}).
Per type, one harness of the program's own emitted functions compares each
result with the reference inside the program (bits equal, a NaN as a NaN).
Built at `-O0` and `-O2` with `-fsanitize=undefined,integer
-fno-sanitize=float-divide-by-zero -fno-sanitize-recover=all` and at plain `-O2`,
all `-std=c11 -Wall -Wextra -Werror -pedantic -ffp-contract=off`:
**4,972 cases × 2 types × 3 builds** — f32: 4,884 agreeing (1,764 arithmetic,
21 negation, 2,646 comparisons, 122 conversions to int with values, 88 typed
`OutOfRange` tags, 21 float → float, 100 int → float) and 88 faults exiting 86;
f64: 4,875 agreeing and 97 faults exiting 86 — **every build identical, 0
sanitizer reports, 0 bytes on either stream**. The kernel matches an
independent reference (host IEEE for arithmetic, field arithmetic for float →
int, a rounding construction for int → float) on the same table, and the
checker renders every case's reference value in a closed entry (a constant
float → int that faults is refused, in an entry and in an `f64` library fold).

**Renderers.** The same three builds: every boundary value `dec`/`hex`,
printed and logged, streams equal to the reference; and the emitted renderers
over a seeded sample — 12,000 random bit patterns of both types, every power
of two of both types (2,375, the asymmetric-neighbour case), 3,998 decimal
fractions and 1,999 random decimal magnitudes — **byte-identical** to the Rust
reference. Build-time negatives: `-ffp-eval-method=extended` refuses at the
`FLT_EVAL_METHOD` assertion, `-ffast-math` and `-ffinite-math-only` at the
`#error`.

**Extern and FFI with the card's own C** (`float_extern.rs`, `float_ffi.rs`):
functions taking and returning `double`/`float` (`q_half_f64`, `qr_point_sum`,
`qr_point_scale` by value with a `double`/`float` struct, `qr_half`) linked and
run under the three builds, streams equal to the reference; a false float
outcome (`1.25` for `1.5`, `-0.0` for `0.0`) exits 70 before its line; the
dynamic path runs every pair of 17 boundary values per type through
arithmetic, negation, comparisons and `as i32` with each result checked by the
callee bit for bit, NaN/±inf/out-of-range exiting 83 (`80 + OutOfRange`), 0
sanitizer reports.

**Tasks** (`float_tasks.rs`, [summary](artifacts/native-tasks.json)): per type,
five programs (`+ - * /`, `<=` under the worker's `partial_ord`) each spawning
49 workers on real threads over ±0, min subnormal, −1, max, +inf and NaN,
joined and rendered, under the three builds — every stream equal to the
reference execution and every traced result the host's IEEE operation (NaN as
`nan`); a closure capturing a float, a record with a float leaf crossing whole,
and a float worker on the composed epoch route (three builds) likewise.

**Negative controls** (each refused, each a test row): a float comparison with
no effect, `==`/`!=` under `partial_ord` only, `>=` under `partial_eq` only, a
caller not declaring a callee's effect (directly and transitively), a literal
comparison, a repeated row entry; mixed `f32`/`f64`, an implicit int ↔ float
(binding, literal, operand, argument, outcome, field), `%`; float → int of NaN,
±inf and out-of-range values at check time and on the dynamic path; a literal
that is not a float (`1.5e`, `1e+`), overflowing (`1e400`, `3.5e38` at f32) or
underflowing (`1e-400`); a float payload; the excess-precision and fast-math
builds; on the extern, FFI and task routes their own forms of the same.

**Lean** (`QCore.FloatTypes`, `AuditFloatTypes.lean`, 20 theorems, no
placeholder, no project axiom): `no_undeclared_float_comparison` (an admitted
function without an effect reaches no float comparison needing it, through any
call chain), `caller_declares_callee_effects` (propagation), the per-effect
corollaries, `partial_ord_does_not_admit_equality`,
`undeclared_callee_effect_is_refused`, `integer_comparisons_need_no_effect`;
`toInt_succeeds_iff` (float → int succeeds exactly when the truncation fits),
`toInt_nan`, `toInt_inf`, `toInt_in_range`, `toInt_needs_target`,
`truncDyadic_exact`; `roundNat_error_le_half` (round to nearest),
`roundNat_exact_small`, `roundInt_exact_small`, `roundInt_neg`; and the
`roundingVectors` (21 rows, ties to even both ways at 2^24/2^25/2^53/2^54 and
the 64-bit limits) and `truncationVectors` (17 rows) tables proved by `decide`
and replayed against the Rust kernel. **What is modelled:** a float as NaN,
±inf or a finite dyadic `m·2^e`; the effect check as an admission over a call
graph with a reachability semantics. IEEE arithmetic, comparisons, `f64 → f32`
and the shortest-digit search are not formalised (`W48-QR-FLOAT-IEEE-LEAN-MODEL`).

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixture freeze (701) | byte-identical, 80% | **697 identical**; 4 message-only moves naming `f32, f64` (not predicted: the widened table's lists) |
| census AFTER | only the card's files, 90% | **only `examples/scalar/floating_point.qr`**; 118 reserved words, none added |
| existing tests changed | 0–3 files, 60% | 1 behavioural (`unsafe_linkage.rs`) + 3 compile-only |
| native boundary table | bit-identical, 75% | **identical, 0 reports** |
| renderer vs reference | byte-identical, 70% | **identical** on the table and 18,372 samples |
| Lean | one module, 10–20 theorems, 75% | **20 theorems**, 250 audits |
| corpus arbiter | identical `2fc3af8c…`, 95% | **byte-identical** |
| gate first run | green, 40% | **green** (every step 0 on the first run) |
| items | 1 (80%); 2 full (35%), records + extern ≥ (65%); 3 (55%) | 1 and 3 complete; 2: scalar records, extern, FFI, tasks, closures, epoch half done; buffer route and payloads registered |

## Gate

At `dabdc595` (script `private/w48-qr-floating-point/gate.sh`, each exit to a
file, logs `private/w48-qr-floating-point/gate/`):

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 395 units, 408 summaries; **6,924 passed, 0 failed, 11 ignored** (`private/test-binaries/20260930T093257Z-2213596/summary.json`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **250 audits** (249 + `AuditFloatTypes.lean`) |
| provider suite | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | 129 passed, 0 failed, 1 ignored, 43 summaries (`providers/` unchanged) |
| matrix | `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w48-qr-floating-point/gate.json` | see gate.json | |
| docs index | `python3 scripts/build_docs_index.py` | see gate.json | |
| Python | `python3 -m unittest discover -s scripts/tests` | see gate.json | after the matrix |

Exits in [gate/](gate/), record in [gate.json](gate.json).

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT 66910c10 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with the gate
tip's release `quirin-audit` (`1bfd0e8a…`): exit 0.
`resource-diagnostics.json` sha256 `2fc3af8c39f0cf04…` (602 Safe / 10 Unsafe /
4,222 Unknown) is **byte-identical** to the base's archive (W48-QR-CLOSURES'
AFTER, itself CG08's), and so are `resource-diagnostics.chunk-contracts.json`
(`cbbcfd9c…`) and `monkey-source.json` (`57acfc58…`)
([identity](artifacts/arbiter-corpus.identity)). No C-audit path moved, as
predicted.

## Follow-ups

Registered (`named_followups` 643 → 649, [followups.json](followups.json),
applied by the idempotent [apply_ledger.py](apply_ledger.py)):
`W48-QR-FLOAT-BUFFER-ROUTE` (M), `W48-QR-FLOAT-PAYLOADS` (M),
`W48-QR-FLOAT-MSVC-NATIVE` (S), `W48-QR-FLOAT-QRLIB-SPELLING` (S, card 5),
`W48-QR-FLOAT-IEEE-LEAN-MODEL` (L), and `W48-SEQUENTIAL-UNREAD-EFFECT-NAMES`
(S; found: the sequential route admits `effects { banana, banana }` at the
base).

## Files likely to conflict with card 5

`crates/qcore-core/src/extern_call.rs` (`AbiType`), `extern_c_emit.rs`,
`crates/qcore-frontend/src/source_extern.rs`, `native_library.rs` /
`native_library_header.rs` (untouched here, where card 5 adds `float`/`double`),
`scalar_floats.rs`, `scalar_types.rs`, `crates/qcore-verification/src/driver.rs`
(`float_assumption`, the declaration read), `crates/qcorec/src/main.rs`,
specification §L14.0/§L14.7/§L20/§L21, grammars §G02t/§G02d.

## Scripts

* `census_float_spelling.py [OUT.json]` — the constraint-14 census
  ([before](artifacts/census-before.json), [after](artifacts/census-after.json)).
* Card 0's `freeze_fixtures.py`/`compare_fixtures.py` for the freeze.
* Native summaries: `QCORE_W48_FLOAT_NATIVE_SUMMARY=… QCORE_W48_FLOAT_TASKS_SUMMARY=…
  cargo test -p qcore-compiler --test float_types --test float_tasks native`.
