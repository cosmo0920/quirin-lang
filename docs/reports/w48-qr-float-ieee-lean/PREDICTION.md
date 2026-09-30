# W48-QR-FLOAT-IEEE-LEAN-MODEL: prediction

Frozen at base `d11179c9` (= origin/main; local `main` had advanced to
`93789344`, card W48-QRLIB-LINKAGE's merge, while this worktree was being
set up — the launch brief names `d11179c9`, the worktree's HEAD, so no reset
or fast-forward was made). **Disclosure:** the encoding module
(`proof/QCore/Ieee754.lean`) and the rounding module
(`proof/QCore/Ieee754Round.lean`) were drafted and compiled, uncommitted,
before this file was written; nothing else (operations, vectors, Rust
generator, records) existed. The rows below about items 1–2 are therefore
weak predictions and are marked so.

Lean timings at the base, `python3 scripts/check_lean_audits.py`, this host
(16 CPUs): **cold 71.57 s** (fresh `.lake`, 302 build jobs, 250 audits) and
**warm 9.21 s** (nothing to rebuild).

## Plan

1. **Encoding** (`QCore.Ieee754`): a format as `(ew, tw)`, a datum as NaN /
   signed infinity / signed magnitude in units of the least subnormal;
   `decode`/`encode` of patterns, round trips, class partition.
2. **Rounding** (`QCore.Ieee754Round`): `roundTiesToEven` of an exact
   rational `n/d` with subnormals and overflow; nearest, ties-to-even,
   overflow threshold, monotonicity, exactness.
3. **Operations** (`QCore.Ieee754Ops`): `+ - * /` and negation as "round the
   exact result" with IEEE's special cases; the special-case table; the
   comparisons; `int → float`, `f64 → f32`, `f32 → f64`, `float → int`
   through `QCore.FloatTypes.toInt`; agreement of `int → float` with
   `FloatTypes.roundInt`.
4. **Vectors**: a Rust test that generates, from the kernel
   (`scalar_floats::arithmetic`/`negate`/`compare` and
   `scalar_types::convert`, i.e. the host's IEEE operations), a Lean table per
   format and fails when the checked-in table differs; Lean proves every row
   against the model by kernel computation (`decide +kernel`; the audit policy
   admits only `propext`, `Classical.choice`, `Quot.sound`, so
   `native_decide` is out).

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| items 1–4 complete (retire `W48-QR-FLOAT-IEEE-LEAN-MODEL`) | yes | 70% |
| item 5 (renderer round trip) proved | no; §L14.7 item 13 narrowed, a follow-up registered | 85% |
| disagreement between the model and `scalar_floats.rs` | none on any vector | 85% |
| if one is found | a NaN sign/payload question, never a finite value | 80% of the 15% |
| Lean modules added | 4–6 (encoding, rounding, operations, 1–2 vector files) | 75% |
| theorems registered in the new audit | 60–110 | 60% |
| vectors | 10,000–20,000 rows over both formats | 60% |
| `decide +kernel` over the vector files | under 60 s of elaboration in total | 65% |
| cold `check_lean_audits.py` after | under 130 s (base 71.57 s) | 70% |
| warm run after | within 3 s of 9.21 s | 70% |
| corpus figures | unmoved (no analyzer, contract or checker change; no arbiter run needed) | 99% |
| gate first run green | yes | 55% |
| existing tests changed | 0 files | 90% |
