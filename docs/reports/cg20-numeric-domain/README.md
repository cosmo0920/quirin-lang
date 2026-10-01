# CG20-NUMERIC-DOMAIN: the C integer domain per target identity

Card `CG20-NUMERIC-DOMAIN` of the
[Clang gap card queue](../../plans/clang-gap-cards/README.md) (wave 48, round 9,
phase 4): entries N01–N07, N08 (transfer), C04 and C05; absorbs
`W45-FB-SCHED-RANDOM-RANGE` and `CG01-CYCLE18-EVALUATOR-WIDTH`. Base `b6a3dc42`
(local `main` = `origin/main`). Route: a new property over Clang's typed AST
(neither G nor T), plus hooks in G's lexical ABI reader and the cycle-18
verifier.

- **Published property.** *C integer arithmetic*: a function's own integer
  operations, increments, compound assignments, recorded conversions and
  bit-field stores reach no undefined behaviour and no unstated
  implementation-defined behaviour. Scope per function body, per declared or
  default Clang target; pointer arithmetic (CG16), floating arithmetic
  (N09/N10) and callee behaviour are outside. It is published **beside**
  custody, never in the custody denominator: projection coverage is not
  safety coverage, and this property is neither.
- **Rule:** [gateway 11q](../../agents/gateway.md#constraint-11q).
- **Lean:** `QCore.CNumeric`, `proof/AuditCNumeric.lean` (37 theorems, standard
  axioms only).
- **Prediction:** [PREDICTION.md](PREDICTION.md), committed as `785470b9`
  before the implementation commit `dd0fd7ea`.

## 1. What changed

**Target identity** (`c_numeric::TargetIntegerModel`). Widths come from the
analysing Clang for the unit's own flags: `-dM -E` over an empty unit gives
`__CHAR_BIT__`, `__SIZEOF_{SHORT,INT,LONG,LONG_LONG,POINTER}__`,
`__CHAR_UNSIGNED__` and `__{FLT,DBL,LDBL}_MANT_DIG__`, cross-checked against
`__{SCHAR,SHRT,INT,LONG,LONG_LONG}_MAX__`; `-print-target-triple` names the
target. A missing or inconsistent macro refuses the model. The property may
declare `--target=TRIPLE` (both the model query and the AST run take it); the
custody gateway still transports none (CG30). Under Clang 21.1.8,
`x86_64-pc-linux-gnu` reports LP64 (`long` 64) and `x86_64-pc-windows-msvc`
LLP64 (`long` 32, `long double` 53 bits).

**Conversions are imported.** The walker reads the conversions Clang recorded
(implicit casts, each operator's type, a compound assignment's
`computeLHSType`/`computeResultType`); an operator whose operand types differ
from its computation type is refused. `promote` and `usual_arithmetic` state
C17 6.3.1.1/6.3.1.8 only to bind Clang's records to the Lean vectors:
`clang_records_the_lean_usual_conversions` compiles all 13 `usualVectors` rows
under their target and finds Clang's `+` typed as Lean says, including
`long + unsigned int` → `long` (LP64) and → `unsigned long` (LLP64).

**The C interpretation of the core table** (`c_numeric::binary`, `convert`).
A C type maps to its fixed-width core type (`core_type`: never `isize`/`usize`,
and a width the core lacks refuses); `scalar_types::checked`/`convert` answer
where they can, and the layer decides the rest by C: unsigned `+ - *` and
conversions to unsigned wrap; signed overflow, `/ %` by zero, `MIN / -1`,
`MIN % -1`, a shift count outside `[0, width)` and a negative left operand of
`<<` are undefined; a negative `>>`, a conversion to a signed type that cannot
hold the value, an inexact integer-to-floating conversion and pointer/integer
conversions are implementation-defined. `reaches` decides each fault case on
its own; `binary` fails exactly when one is reached.

**Intervals and obligations** (`binary_interval`, `unary_interval`,
`convert_interval`). Each operation returns its result range and one
obligation per fault case: `Discharged` (no operand pair in the ranges reaches
it), `Violated` (every pair does; constant operands) or `Open`. Division and
remainder results are bounded (`|a / b| ≤ |a| / min|b|`, `|a % b| < max|b|`), so
`a / K + a % K` with `K = 7` discharges.

**The property** (`c_numeric_obligations::analyze_numeric_obligations`). Every
function with a body; a literal, a Clang `ConstantExpr` value or a
Clang-evaluated enumerator is a point, anything read from storage or a call is
its type's full range; no flow, path or loop fact. `Safe` only when every
obligation is discharged and nothing was refused; otherwise `Unknown`. A
`Violated` obligation is a definite fault *if its site is reached*;
reachability is not decided, so it is reported by site and the function is
`Unknown`, never `Unsafe`. Defined but value-changing events are listed
separately (`unsigned-wrap`, `wrapped-conversion`,
`mixed-signedness-comparison`). The measurement tool
`examples/numeric_obligations.rs` runs it with a recorded compile command.

**Width-independent constants** (absorbed `CG01-CYCLE18-EVALUATOR-WIDTH`).
`cycle18_callback_obligations::evaluate_integer` now calls
`c_numeric::portable_constant`: each node at its Clang type, a literal inside
its type's portable range (C17 5.2.4.2.1's minimums; plain `char` refused), an
`IntegralCast` only if exact, a `NoOp` only between equal types, `|`/`&` at the
result type, `+` and unary `-` in range (an unsigned negation of a nonzero value
is refused). Every other cast kind and operator refuses, as before. Such a value
is the same under every data model (`portable_in_every_model`), so no target
query is needed.

**Plain `long` in the lexical ABI reader** (`legacy_declared_abi::project_arithmetic`).
One `long` word is read as the 64-bit, pointer-width projection only when
`c_numeric::plain_long_is_lp64()` holds: the analysing Clang's default target
reports a 64-bit `long` and pointer (queried once per resolved Clang and
process). Under LLP64, ILP32 or a failed query the declaration is unreadable
(`abi-unreadable-declaration`); `long long` and `__int64` keep their reading.

**Shared items added (for the integrator).** New modules `c_numeric` and
`c_numeric_obligations` (both `pub`, registered in `lib.rs`); one guarded arm in
`project_arithmetic`; `evaluate_integer`'s body in `cycle18_callback_obligations`
(its private helper `integer_literal` removed). No enum gains a
variant; no shared signature changes.

## 2. The MSVC ABI reader's premise (amended here, not in its report)

The [MSVC ABI reader report](../msvc-abi-reader/README.md) records that
"Ordinary `long` still has the reader's LP64 premise" and that a newly readable
cdecl position involving it refuses. **This card removes that premise**: the
reader's LP64 reading of plain `long` / `unsigned long` (and `long int`,
typedef chains included) now holds only when the analysing Clang's default
target is LP64; on `x86_64-pc-windows-msvc` it is unreadable in every position,
not only cdecl ones. The cdecl refusal (`has_target_dependent_long`) stays as
it was. The Windows evidence records are unchanged; their fixtures declare
`size_t` as `unsigned long long` on Win64 (`tests/support/host_abi.rs`) and are
predicted unaffected, but **no Windows run was made here**: the Windows gate is
owed. Residual: an Apple `-arch` flag selects a unit's architecture and is not
seen by the process-wide query (`CG20-ABI-READER-ARCH-FLAG`). The doc comment
in `loader_abi.rs` that the ABI comparison "needs no Clang" no longer holds for
plain `long` (without Clang it is unreadable, fail-closed).

## 3. Cited claims: reproduced and corrected

CG00's BEFORE controls were rerun at the base: `clang_gap_phase4` (20 passed,
N01–N03, N04–N07, N08, C04, C05) and `qcore-core`'s `clang_gap_core` (11 passed,
the core halves N03–N06). Each reproduces.

| Entry | Claim | Outcome |
|---|---|---|
| N01 | no common width/signedness/rank/target domain | reproduced; now `c_numeric` for this property |
| N02 | conversions without numerical semantics | reproduced; Clang's recorded conversions now carry transfer and obligations |
| N03 | `IntegralCast` named, no bit-level result | reproduced; `convert_interval` |
| N04 | core checked `I32` lacks C wrap/UB | reproduced (`n04_core_arithmetic_refuses_where_c_wraps`); the core is reused only through `c_numeric::binary` |
| N05 | `/ %` accepted without exception proof | reproduced; obligations `DivisionByZero`, `QuotientOverflow` |
| N06 | G reads shifts neutral; **T does not represent shifts** | G reproduced. **Correction (stale input):** since CG01 (rule 10r) T reads `<<`/`>>` and refuses them by name in every consumer but the Monkey wrapper projection's `checked_binary` |
| N07 | comparisons without post-conversion semantics | reproduced; `mixed-signedness-comparison` event, comparison decided on converted values |
| N08 | integer↔FP transfer not modelled | reproduced; range obligations `FloatingToInteger`, `FloatingNarrowing`, `InexactIntegerToFloating` |
| C04 | every case edge kept | reproduced (provisional `Unsafe`); **not changed** (precision) |
| C05 | no branch correlation | reproduced (provisional `Unsafe`); **not changed** (precision) |

## 4. Prediction against outcome

| Route | BEFORE (binary `b199989e…`, base `b6a3dc42`) | AFTER (binary `46c19503…`, tip) |
|---|---|---|
| corpus `66910c10` (`--jobs 4`) | 629 / 10 / 4,195, `3d53a5d0…` | 629 / 10 / 4,195, `3d53a5d0…` (byte-identical) |
| system | 44 / 2 / 188, `e363acc8…` | identical, `e363acc8…` |
| tail-pause slice | 29 / 1 / 239 / 1 Pending, `acc73cd9…` | identical, `acc73cd9…` |

All six runs exit 0 (`scripts/measure.sh before|after`, logs under
`private/cg20/`). The BEFORE reproduces the baseline's totals on all three
routes and the corpus digest exactly. The system and slice digests differ from
the baseline's `79404fed…` and `93cc660c…` only because those reports embed the
checkout's path (the profile directory and the `resource_contract_sources`
digest over contract paths): a normalised diff against CG05's `after3` slice
report shows only those fields (`scripts/normalized_digest.py`). BEFORE and
AFTER were taken in this one checkout and are byte-identical. The corpus
typed-path parity streams are identical as multisets once Clang node ids
(per-run addresses) are normalised (466,022 rows each,
`scripts/parity_identity.py`); `audit.log` differs only in its output path.

Every prediction holds: custody byte-identical on all three routes (no function
moves, no correction); the named controls as predicted (§5); W45 re-registered
with exactly the predicted open obligations (§6); `CG01-CYCLE18-EVALUATOR-WIDTH`
closed with the cycle-18 tests unchanged (22 passed). The reevaluation probes
exit 0; their output was not diffed against the base, so "unchanged output" is
not claimed.

## 5. The property on the named controls (`tests/c_numeric_domain.rs`)

| Source | LP64 | LLP64 |
|---|---|---|
| `2147483647L + 1L` | `SignedOverflow` discharged, `Safe` | `SignedOverflow` **violated**, `Unknown` |
| `double f(long x) { return x; }` | `InexactIntegerToFloating` open | discharged, `Safe` |
| `x / 0` | `DivisionByZero` violated, `Unknown` | same |
| `(-2147483647 - 1) / -1` | `QuotientOverflow` violated | same |
| `1 << 32` | `ShiftCount` violated | same |
| `u < -1` | event only, `Safe` | same |
| `a / K + a % K`, `K = 7` | all discharged, `Safe` | same |
| `c + 1`, `unsigned char c` | discharged, `Safe` | same |
| `a + 1`, `int a` | `SignedOverflow` open | same |
| `(signed char)a` / `(unsigned char)a` | `SignedConversion` open / `Safe` | same |
| `s->bf = v`, `int bf : 3` | `SignedConversion` open | same |
| `(int)d`, `(float)d` | `FloatingToInteger` / `FloatingNarrowing` open | same |
| `a + b` on `double` | refused (N09/N10), `Unknown` | same |
| `p + 1` on `char *` | outside (CG16), `Safe` | same |

The CG00 BEFORE sources keep their custody `Safe` (`clang_gap_phase4`
unchanged); under this property all are `Unknown` except the mixed comparison
(`the_before_sources_under_the_arithmetic_property`, both targets).
`c_numeric_abi_llp64.rs` installs a wrapper Clang for `x86_64-pc-windows-msvc`
and shows the reader refusing `unsigned long`, `long unsigned int` and a
typedef of it, while `unsigned long long` and `unsigned __int64` agree.

## 6. W45-FB-SCHED-RANDOM-RANGE: re-registered, not closed

Reading of Fluent Bit `046accf6` (read-only): `scheduler.cap` and
`scheduler.base` are `FLB_CONF_TYPE_INT` entries (`src/flb_config.c:233-238`),
written only by `flb_config_set_property` through `atoi` into the
`unsigned int` fields via an `int *` (`:913-915`), with defaults 2000 and 5
(`:456-457`) and no ordering or range check anywhere in `src/`; the only reader
is `flb_sched_request_create` (`src/flb_scheduler.c:297`). The reading
therefore does **not** establish the range premise: with `scheduler.cap 4`
and `scheduler.base 5`, `backoff_full_jitter` passes `temp = 4` and
`random_uniform` computes `range = 0` and `RAND_MAX / range` (`:102`).
This is a source reading, not a validated reachable violation and not an
upstream defect claim; no native trace was made.

The property on the recorded compile command
(`scripts/numeric_scheduler.sh`, LP64): `random_uniform` is `Unknown` with
open `SignedOverflow` at `max - min + 1` (`:101`, both operators) and
`range * copies` (`:103`), open `DivisionByZero` at `RAND_MAX / range` (`:102`,
`QuotientOverflow` discharged: the dividend is the constant `RAND_MAX`) and at
`ra / copies` (`:110`), open `SignedConversion` at `val = time(NULL)` (`:97`)
and at the `sizeof(int)` actual (`:96`, a `sizeof` value is not evaluated by
this walker); `backoff_full_jitter` is `Unknown`: floating `*` refused
(N09/N10) and `FloatingToInteger` open at the `xmin` result (`:263`). The entry
is amended with this reading; its remaining discharge is the native trace.

## 7. Axes (§8 of the analysis)

- **Type:** the twelve standard integer types per target; enumerations,
  `_BitInt`, `__int128`, `_Complex` and atomic types refuse by name;
  bit-fields are read through Clang's promotion and a store is an
  obligation.
- **Position:** every expression position in a body (statements,
  conditions, initializers, returns, actuals); unevaluated operands
  (`sizeof`, `_Generic` associations) are walked too, which can only add
  obligations.
- **Evaluation order:** none is assumed; each site is decided on its operand
  ranges alone.
- **Target:** LP64 and LLP64 by test; any Clang target by its macros.
- **Verdict:** `Safe` / `Unknown` only. Stays refused: C04, C05, IEEE (N09,
  N10), pointer arithmetic, flow-sensitive ranges.

## 8. Lean

`QCore.CNumeric` (37 theorems, `propext`/`Classical.choice`/`Quot.sound` only):
`long_is_64_under_lp64`, `long_is_32_under_llp64`, `long_plus_uint_lp64`,
`long_plus_uint_llp64`, `char_promotes_to_int`, `uint_plus_int_is_uint`,
`promotion_vectors_hold`, `usual_vectors_hold`, `kernel_ok_iff_unreached`,
`kernel_error_reached`, `unsigned_arithmetic_defined`, `wrap_in_range`,
`signed_add_in_range`, `division_by_zero_reached`,
`quotient_overflow_reached`, `shift_count_reached`,
`negative_left_shift_reached`, `int_div_zero`, `int_min_div_neg_one`,
`one_shl_32`, `uint_max_plus_one_wraps`, `kernel_vectors_hold`,
`convert_unsigned_defined`, `convert_fits_exact`, `conversion_vectors_hold`,
`add_discharge_sound`, `sub_discharge_sound`, `mul_discharge_sound`,
`division_discharge_sound`, `quotient_discharge_sound`,
`shift_count_discharge_sound`, `negative_shift_discharge_sound`,
`shl_overflow_discharge_sound`, `conversion_discharge_sound`, `range_mono`,
`portable_in_every_model`, `portable_long_holds_on_llp64`. The Rust tests read
`lp64`, `llp64` and the four vector lists from the Lean file. The result-range
transfer (that each operation's value lies in the returned interval) is
checked exhaustively at width 8 in Rust
(`cg20_discharge_is_sound_exhaustively_at_width_eight`, every operator, both
signednesses, nine ranges), not proved in Lean
(`CG20-RESULT-RANGE-TRANSFER-LEAN`).

## 9. Follow-ups

- Retired: `CG20-NUMERIC-DOMAIN` (this card), `CG01-CYCLE18-EVALUATOR-WIDTH`
  (§1, one evaluator, its three callers' tests unchanged: `cycle18` 22 passed).
- Amended: `W45-FB-SCHED-RANDOM-RANGE` (§6).
- Registered: `CG20-SWITCH-AND-CORRELATION-PRECISION` (C04, C05),
  `CG20-ABI-READER-ARCH-FLAG` (§2), `CG20-RESULT-RANGE-TRANSFER-LEAN` (§8),
  `CG20-NUMERIC-CORPUS-CENSUS` (the property has no corpus-wide census or
  pinned route yet).

## 10. Gate

**The standing gate is incomplete.** Two full runs of `scripts/gate.sh`
(`private/cg20/gate2`, `private/cg20/gate3`) failed in the test runner's build
with `No space left on device`: this card's own dev build of the workspace's
test binaries is about 38 GiB (`cargo clean --profile dev` removed 37.7 GiB),
and with six cards building at once the host's free space fell from 59 GB to
0 during each attempt and kept swinging between 10 and 83 GB afterwards. Both
attempts measure disk, not the program; they are retained and discarded as
evidence, and the runs were stopped at the 40 GB floor.

| Step | Command | Exit / result | Log (`private/cg20/`) |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | `gate3/01-submodule.log` |
| fmt | `cargo fmt --all -- --check` | 0 | `gate/fmt.log` |
| clippy | `cargo clippy --locked --all-targets -- -D warnings` | 0 | `gate/clippy.log` |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `gate/probe.log` |
| follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `gate/followup.log` |
| Lean | `python3 scripts/check_lean_audits.py` | 0, **264** audits (263 + `AuditCNumeric.lean`) | `gate/lean.log` |
| Python | `python3 -m unittest discover -s scripts/tests` | 1: 365 tests, the one expected failure the matrix's derived `lean_audits` 263 → 264, cleared only by `build_matrix.py --gate` after the workspace count | `gate/python.log` |
| index | `python3 scripts/build_docs_index.py --check` | 0 | `gate/index-check.log` |
| **workspace tests** | `python3 scripts/run_test_binaries.py` | **owed** (both attempts disk-exhausted, exit 101 at build) | `gate2/03-tests.log`, `gate3/03-tests.log` |

Targeted runs at the tip's code (`dd0fd7ea`; later commits change records
only): `qcore-c-gateway --lib c_numeric` 10 passed; `--test c_numeric_domain`
7 passed; `--test c_numeric_abi_llp64` 1 passed; `--test loader_abi`,
`loader_abi_modes`, `clang_gap_phase4`, `w46_libc_event_contract_gaps` 33 +
16 + 20 + 4 passed (`t-abi.log`); `qcore-verification --lib cycle18` 22 passed
(`t-cycle18.log`); `qcore-core --test clang_gap_core` 11 passed (`t-core.log`).
Every exit is 0 and recorded beside its log.

**Owed:** the workspace runner, then `gate.json` and the matrix regeneration
(`docs/reports/remaining-d02/build_matrix.py . --gate gate.json`) and the
Python suite again; and a Windows run of the MSVC fixtures (§2). `providers/`
is unchanged, so the provider gate is not due. Clang 21.1.8
(`Ubuntu clang version 21.1.8 (6ubuntu1)`).
