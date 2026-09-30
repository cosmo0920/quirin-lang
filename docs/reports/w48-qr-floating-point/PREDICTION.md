# W48-QR-FLOATING-POINT: prediction

Frozen before any source change, at base `2ef0f8b5` (= local `main` =
origin/main; the worktree's HEAD was already that commit, so no reset or
fast-forward was needed). Taken first: the BEFORE fixture freeze
(`artifacts/fixtures-before.json`, card 0's `freeze_fixtures.py`, debug
`quirin` built at the base, sha256 `80ad97b8…`, 701 files — check 394 exit 0 /
307 exit 1, emit 185 / 516) and the spelling census
(`artifacts/census-before.json`, `census_float_spelling.py`, 701 tracked
`.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc` files): `f32` 0, `f64` 20 (all in `.qv`
contract signatures and two gateway fixtures, the loader's NEIGHBOURS
vocabulary), `partial_eq` 0, `partial_ord` 0, `as f32`/`as f64` 0, either word
in an effect row 0, `DIGITS . DIGITS` tokens 10 (socket addresses and four
`.qrlib` version strings), exponent spellings (`1e5`, `1.5e-3`) **0**; 118
reserved words, none of the card's.

## Where the base stands

`f32`/`f64` exist only in the contract loader's `SCALAR_ABI_TYPES` (card
NEIGHBOURS), which the extern route's `AbiType` deliberately does not parse
(`Q033`). No `.qr` route has a float value, a float literal or an effect that
gates a comparison. The lexer already produces `Float("D.D")`; nothing reads
it as a number.

## Plan the prediction is about

1. **Kernel.** `scalar_types::ScalarType` gains `F32`/`F64`, kept **out of**
   `ALL` and out of `ScalarType::parse`, so every position that parses the
   integer table today keeps refusing a float until it opts in through a
   separate float-admitting parse. A float value travels in the same `i128`
   slot as its IEEE bit pattern (non-negative, `u32`/`u64` bits), so records,
   captures and payloads carry it without a second representation. The kernel
   states arithmetic (`+ - * /`, unary `-`; no `%`), the conversions (int →
   float and `f64 → f32` round to nearest-even and are total; `f32 → f64`
   exact; float → int truncates and is `OutOfRange` for NaN, ±inf and a
   truncation that does not fit), comparisons (IEEE, NaN unordered), and the
   renderings (`dec` shortest round-trip digits in the ECMAScript
   `Number::toString` layout, `-0` for negative zero, `nan`/`inf`/`-inf`;
   `hex` normalised `0x1.hhhp±e`), using the host's IEEE operations.
2. **Lexer.** `DIGITS [. DIGITS] (e|E) [+|-] DIGITS` becomes one `Float`
   token; the census says no tracked file spells it, so no token stream moves.
3. **Scalar route.** Float literals (typed by context, default `f64`, parsed
   once at the context type, a literal that rounds to ±inf or a nonzero one
   that rounds to zero refused), no implicit int↔float (an integer literal is
   not a float), effect rows gain `partial_eq` / `partial_ord`, never inferred,
   derived transitively through calls and checked against the declared row.
   The emitted C: one helper per float operation, comparison and conversion,
   float → int tested against exact hex-float thresholds before the cast, a
   self-contained shortest-round-trip renderer (a port of the Dragon4
   shortest mode Rust's own formatting agrees with), `_Static_assert`s on the
   binary32/binary64 `<float.h>` parameters and `FLT_EVAL_METHOD == 0`, and an
   `#error` under `__FAST_MATH__`/finite-math-only.
4. **Other positions (item 2)** in the order records → extern values →
   tasks/closures → loop bodies → payloads → FFI `struct!`, each either
   admitting the full float rules or keeping its refusal; **item 3** the
   extern `AbiType` gains `F32`/`F64` and one FFI test with the card's own C
   function.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixture freeze AFTER vs BEFORE (701 files, `check` and `emit`, exit + both streams) | byte-identical for every existing file; the only new entries the card's examples | 80% |
| census AFTER | only the card's own files add `f32`/`f64`/`partial_eq`/`partial_ord`/exponent spellings; no reserved word added | 90% |
| existing tests changed | 0–3 files (a pin of a refusal wording that now names the float types, or a type-table count) | 60% |
| native boundary table (f32, f64: ±0, ±min/max subnormal, ±min normal, ±max, ±1, NaN, ±inf, around 2^24/2^53; every operation, conversion, comparison) at `-O0`/`-O2` UBSan and plain `-O2`, `-ffp-contract=off`, `-Wall -Wextra -Werror -pedantic` | bit-identical to the reference (NaN compared as NaN), every float → int fault on the typed path, 0 sanitizer reports | 75% |
| native renderer vs the Rust reference (boundary table plus a seeded random sample) | byte-identical | 70% |
| Lean | one new module and audit, ~10–20 theorems, no placeholder or project axiom | 75% |
| corpus arbiter `resource-diagnostics.json` | identical to the base's archive (`c012e785…`) — no C-audit path touched | 95% |
| standing gate first run | green | 40% |
| items completed | 1 (80%); 2 in full (35%), records + extern values at least (65%); 3 (55%) | |
