# W48-QR-FLOAT-IEEE-LEAN-MODEL: IEEE 754 binary32/binary64 in Lean

The float card's formalisation follow-up
([W48-QR-FLOATING-POINT](../w48-qr-floating-point/README.md), which modelled a
float only as NaN, ±inf or a dyadic). Base `d11179c9` (= origin/main at
launch; local `main` had moved to `93789344`, W48-QRLIB-LINKAGE's merge,
while the worktree was set up, and the launch brief names `d11179c9`, so no
reset was made). Lean and Rust test vectors only: no language, checker or
analyzer semantics changed, and no existing test moved. The prediction was
frozen with the encoding and rounding drafts disclosed
([PREDICTION.md](PREDICTION.md)).

## Modules and theorems

`AuditIeee754.lean` registers all 124 theorems (no placeholder, no project
axiom; `AuditAll.lean` checks the closure).

**Item 1, encoding** — `proof/QCore/Ieee754.lean`. A format is `(ew, tw)`
(`binary32 = ⟨8, 23⟩`, `binary64 = ⟨11, 52⟩`); a datum is `nan`,
`inf neg` or `fin neg N` with the magnitude `N` in units of the least
subnormal (`2^-149`, `2^-1074`), so every finite value is an integer and no
rational arithmetic is needed. `decode`/`encode` split and join sign, biased
exponent and trailing significand; `encode .nan` is the kernel's canonical
NaN (`quietNaN_binary32 = 0x7FC00000`, `quietNaN_binary64`).
`decode_encode` (every valid datum, a NaN as a NaN), `encode_decode` (every
non-NaN pattern of the width), `decode_valid`, `encode_lt`, the magnitude
bijection (`decodeMag_encodeMag`, `encodeMag_decodeMag`, `decodeMag_rep`,
`encodeMag_lt`, `sig_mul_shift`, `unbRep_iff`, `shift_le_kmax`), and
`classify_spec`: the class read from the fields alone (zero, subnormal,
normal, infinite, NaN) is exactly the decoded datum's class, `classify` being
a function so the five classes partition the patterns. `binary32_constants`,
`binary64_constants` pin width, precision, least subnormal and largest value.

**Item 2, rounding** — `proof/QCore/Ieee754Round.lean`. `roundMag f n d`
rounds the exact non-negative rational `x = n/d` (units) to nearest, ties to
even: `none` is the overflow to infinity; below `2^p` units the grid is every
unit, which is subnormal rounding (`rshift_small`). `dist n d N = |n − N·d|`
(`dist_eq_natAbs`), i.e. `|x − N|·d`. Proved: `roundMag_rep` (a finite value
of the format); `roundMag_nearest` (no finite value is nearer);
`roundMag_tie_even` (at a tie the significand and the pattern are even,
`encodeMag_even`); `roundMag_none_iff` (overflow exactly when
`2x ≥ maxN + 2^p·2^kmax`, i.e. from half an ulp above the largest finite
value, the tie going to infinity because `2^p − 1` is odd, `sig_maxN`);
`roundMag_mono`, `roundMag_mono_none` (monotone); `roundMag_exact`. The
unbounded-range statements (`roundUnb_nearest`, `roundUnb_tie`,
`roundUnb_mono`, `roundUnb_exact`, `gap`) carry the proofs.

**Item 3, operations** — `proof/QCore/Ieee754Ops.lean`. `add`, `sub`
(`add` of the negation), `mul`, `div`, `neg` are the exact result rounded
once (`round`), with IEEE's special cases. The special-case table:
`nan_propagates` (every operation, either operand), `inf_sub_inf`,
`add_inf`, `zero_mul_inf`, `inf_mul`, `div_by_zero` (`x/0 = ±inf`,
`0/0 = NaN`), `div_inf`, the signs of zero results (`add_zeros`,
`add_neg_self`: `x + (−x) = x − x = +0`, `neg_zero_sub_zero`, `zero_mul`,
`mul_div_sign`: the exclusive or of the signs for zero, underflow and
overflow alike), `add_comm`, `mul_comm`, `neg_neg`, `add_exact`, and
`round_fin`/`round_inf_iff`, which carry item 2's theorems to every finite
or infinite result of every operation and conversion.

**Item 4, conversions and comparisons** — same module. `ofInt` (integer →
float) is item 2's rounding of `n·2^qneg`, and `ofInt_roundNat` +
`ofInt_value` prove it equal to the float card's `QCore.FloatTypes.roundNat`
/ `roundInt` (via `roundUnb_scaled`); `ofInt_exact_small`. `convert` gives
`f64 → f32` (item 2's rounding, so nearest, ties to even, overflow to ±inf)
and `f32 → f64`, exact (`widen_exact`). `toInt` is `QCore.FloatTypes.toInt`
of the exact value: `toInt_some_iff` (success exactly for a finite value
whose truncation toward zero fits), `toInt_none_iff` (the `OutOfRange`
condition: NaN, ±inf or a truncation that does not fit), `toInt_bool`
(no `bool` target, as the kernel). Comparisons: `rank` orders non-NaN data
(`rank_fin_bounds`, `flt_fin`: the exact values' order);
`nan_unordered`, `feq_self` (`x == x` false exactly for a NaN),
`neg_zero_eq_zero`, `trichotomy` (exactly one of `<`, `==`, `>` off NaN,
none with one), `fle_iff`, `feq_symm_trans`, `flt_irrefl_trans`.
`gated_relations`: the operators `partial_eq` admits (`Cmp.effect`) are
exactly `==` (IEEE equality) and `!=` (its negation), those `partial_ord`
admits are `<`, `<=`, `>`, `>=` (IEEE less-than, less-or-equal and their
converses); `partial_not_total` states why they are gated. The mapping
`Cmp.effect` is replayed against `FloatEffect::of` (`effects_hold`).

**Patterns.** `sameValue` is `scalar_floats::same_value`; `negBits` flips
the sign bit, and `decode_negBits` proves it is `neg` on data, NaN
included; `arithBits`, `cmpBits`, `ofIntBits`, `convertBits`, `toIntBits`
are what the vectors replay.

## Vectors

`compiler/tests/float_ieee_vectors.rs` generates
`proof/QCore/Ieee754Vectors32.lean` and `Ieee754Vectors64.lean` from the Rust
kernel (`scalar_types::checked`, `negate`, `compare`, `convert` with a
recorded 64-bit target, and `FloatEffect::of`) and fails when a checked-in
table is not its current output (`QCORE_W48_IEEE_VECTORS_WRITE=1` rewrites
them). Each arithmetic, integer → float and format-conversion result is also
checked against the host's IEEE operation written directly, so the tables
are the host's results. Operands: the float card's 21 boundary values per
type (±0, ±min/±max subnormal, ±min normal, ±max, ±1, ±inf, a NaN, and
2^24 − 1, 2^24, 2^24 + 2 / 2^53 − 1, 2^53, 2^53 + 2) and 9 rounding probes
(0.5, −1.5, 3, 0.1, the neighbours of 1, half and a quarter ulp of the
largest value — `max + half` is the overflow tie — and 1.5 · least normal);
float → integer and format conversion add 12/13 truncation probes, and
`f64 → f32` 16 more (binary32's overflow threshold and its tie, subnormal
ties, the subnormal/normal tie, ties at 1); integer → float uses every
integer type's limits and ±(2^24|2^25|2^53|2^54 + {−1,0,1,2,3,6}).

| Table | binary32 rows (results) | binary64 rows (results) |
|---|---|---|
| `+ - * /` of every operand pair | 900 (3,600) | 900 (3,600) |
| negation (bit for bit, NaN included) | 30 (30) | 30 (30) |
| six comparisons of every pair | 900 (5,400) | 900 (5,400) |
| integer → float | 77 (77) | 77 (77) |
| float → ten integer types | 42 (420) | 43 (430) |
| `f32 → f64` / `f64 → f32` | 42 (42) | 59 (59) |
| comparison → effect | | 6 (6) |
| **total** | **1,991 (9,569)** | **2,009 (9,596)** |

**4,006 rows, 19,171 results**, each proved in Lean by `decide +kernel`
(kernel evaluation; `native_decide` would add `Lean.ofReduceBool`, which the
audit policy rejects). Results are compared bit for bit, a NaN result as a
NaN (`sameValue`: payload and sign unspecified); negation and comparisons bit
for bit. Every row holds: **no disagreement between the model and
`scalar_floats.rs`**.

`Nat.log2` is not a kernel-accelerated operation, and evaluating it on
binary64 magnitudes (up to about `2^4200`) made the binary64 table take
171 s. The model's definitions use `ilog2` instead — a 16-step binary search
over accelerated powers and comparisons, proved equal to `Nat.log2`
(`ilog2_eq`, `log2Search_spec`) — after which each table builds in about
16 s cold, the two in parallel.

## What is modelled and what is observed

Modelled and proved: the IEEE semantics above, as a specification.
Proved about the kernel: exactly the 4,006 rows. Observed, as before: the
kernel on any other operand (it is the host's IEEE hardware, not a Lean
extraction), the emitted C and the C compiler's float operations (the float
card's native boundary tables), the IEEE environment premises of §L14.7
(rounding mode, FTZ/DAZ, contraction, excess precision), and the renderer
(item 5, below). Specification §L14.7 now says this ("What Lean proves",
limitation 13).

## Findings

* **No disagreement** between the formalisation and `scalar_floats.rs` on
  any vector.
* **`QCore.FloatTypes.toInt` admits `bool`** as a float → integer target
  (`0.5 → 0`, `floatTypes_toInt_bool_differs`), while `scalar_types::convert`
  returns `None` for a float `as bool`. No replayed row uses `bool`, so the
  float card's vectors are unaffected; this card's `toInt` refuses `.bool`
  (`toInt_bool`) and `QCore.FloatTypes` was not edited
  (`W48-QR-FLOAT-TOINT-BOOL-TARGET`, S).

## Item 5 (stretch)

Not reached. The model now has decimal parsing's semantics (a decimal is an
exact rational, `round` reads it), but neither the shortest-digit search nor
the round trip is proved; §L14.7 limitation 13 is narrowed to say so
(`W48-QR-FLOAT-RENDER-ROUNDTRIP-PROOF`, M).

## Lean build time

`python3 scripts/check_lean_audits.py`, this host (16 CPUs), wall seconds
from `private/w48-qr-float-ieee-lean/lean-timed.sh` ([timings](artifacts/lean-timings.json)):

| Run | Base `d11179c9` | Tip `a2c4031a` |
|---|---|---|
| cold (fresh build), wall | 71.57 s (build 59.7 s) | **83.45 s** (build 73.1 s) |
| warm (nothing to rebuild), wall | 9.21 s | **9.84 s** |
| audits | 250 | 251 (`AuditIeee754.lean` 0.3 s) |

The new modules' cold builds: `Ieee754` 5.0 s, `Ieee754Round` 7.2 s,
`Ieee754Ops` 1.0 s, `Ieee754Vectors32` 16 s, `Ieee754Vectors64` 16 s (the
vector files build in parallel); the whole cold run is 11.9 s longer.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| items 1–4 complete, entry retired | yes, 70% | **yes** |
| item 5 | not proved, 85% | **not proved**; limitation 13 narrowed, follow-up |
| model vs `scalar_floats.rs` | no disagreement, 85% | **none** on 19,171 results |
| Lean modules | 4–6, 75% | **5** |
| theorems | 60–110, 60% | **124** (outside the range) |
| vectors | 10,000–20,000 rows, 60% | **4,006 rows / 19,171 results** (rows outside the range, results inside: a row packs four arithmetic or six comparison results) |
| `decide +kernel` total | under 60 s, 65% | **about 32 s** cold, after `ilog2` (192 s before it) |
| cold audits after | under 130 s, 70% | **83.45 s** |
| warm after | within 3 s, 70% | **+0.63 s** |
| corpus figures | unmoved, 99% | unmoved (no analyzer change; no arbiter run) |
| gate first run green | 55% | **no**: clippy (`redundant_locals`) and two provider timeouts under this card's concurrent load; run 2 green |
| existing tests changed | 0, 90% | **0** |

## Gate

Run 2 at `a2c4031a` (script `private/w48-qr-float-ieee-lean/gate.sh`, each
exit to a file, logs `private/w48-qr-float-ieee-lean/gate/`):

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 396 units, 409 summaries; **6,927 passed, 0 failed, 11 ignored** (`private/test-binaries/20260930T123639Z-3985394/summary.json`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **251 audits** (250 + `AuditIeee754.lean`) |
| provider suite | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | 129 passed, 0 failed, 1 ignored, 43 summaries (`providers/` unchanged) |
| matrix | `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w48-qr-float-ieee-lean/gate.json` | 0 | |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | **365 tests OK** (after the matrix) |
| ledger | `python3 docs/reports/w48-qr-float-ieee-lean/apply_ledger.py` (twice) | 0, 0 | 649 → 650, then 650 → 650 |

Run 1 (at `61a06930`, logs `private/w48-qr-float-ieee-lean/gate-run1/`) is
retained and discarded: clippy 101 (`clippy::redundant_locals` in the new
test, fixed) and the provider suite 101 (two `QP006` provider timeouts in
`configured_source_contract` while this card ran `cargo clippy` and
`cargo test` beside it); every other step 0. Exits in [gate/](gate/), record
in [gate.json](gate.json). No corpus arbiter: nothing on a corpus path
changed.

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[apply_ledger.py](apply_ledger.py); rerun it after rebasing, `named_followups`
649 → 650 at the base): retires `W48-QR-FLOAT-IEEE-LEAN-MODEL` (items 1–4
done) and registers `W48-QR-FLOAT-RENDER-ROUNDTRIP-PROOF` (M) and
`W48-QR-FLOAT-TOINT-BOOL-TARGET` (S).

## Files likely to conflict

`proof/QCore.lean` (import list), `docs/QCORE_SPEC_r0_3.md` §L14.7,
`docs/STATUS.md` (floating-point and follow-up rows),
`docs/development/increments/wave-48-continuation.md` (top paragraph),
`docs/reviews/r0_3/acceptance_matrix.json` (rerun `apply_ledger.py`, then
`build_matrix.py`), `docs/README.md`/docs index. A parallel float module
would touch `proof/QCore.lean` only; `FloatTypes.lean` and `scalar_floats.rs`
are untouched.
