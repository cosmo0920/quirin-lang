# W48-QR-FLOAT-BUFFER-PAYLOADS: prediction

Frozen before any source change, at base `d11179c9` (= origin/main at launch,
the W48-QR-FLOATING-POINT tip; the worktree's HEAD was already that commit, so
no reset or fast-forward was needed; local `main` has since moved to
`93789344` with W48-QRLIB-LINKAGE, unpushed, and is not this card's base).
Taken first: the BEFORE fixture freeze (`artifacts/fixtures-before.json`,
card 0's `freeze_fixtures.py`, debug `quirin` built at the base, sha256
`0319bb6b…`, 702 files — check 395 exit 0 / 307 exit 1, emit 186 / 516).

## Where the base stands

`f32`/`f64` are admitted on the scalar, extern, FFI and task routes. On
`source.buffer.v1` the record reader, the conversion reader
(`ScalarType::parse`), the value-statement reader (`parse_value_expression`,
no floats) and the checker's `convert` (integers only) all refuse a float, and
the value statement's consumer refuses a float literal by name
(`W48-QR-FLOAT-BUFFER-ROUTE`). The route's effect row reads `io`, `alloc`,
`mut` only, and its `Decider` is `Decider::default()` (no comparison effect
permitted). `Buffer<T>` reads its payload with `ScalarType::parse`, and
`protected TYPE` likewise, so a float payload is refused at both
(`W48-QR-FLOAT-PAYLOADS`). The loop bound decision (`typed_bound`) refuses
`bool` by name and would otherwise read a float's **bit pattern** as a count,
which no program can reach today because no float enters the lowering's tier.

## Plan the prediction is about

1. **Buffer route.** The entry's row gains `partial_eq`/`partial_ord`
   (carried beside the `EffectRow`, as the extern route does), fed to the
   checker's `Decider`; float record fields (float literals at a float field,
   field reads and writes); `as f32`/`as f64` in the conversion and float
   conversions in the checker; float literals in value statements and a
   literal binding `let x = 1.5;` (an `f64`, as on the extern route); loop
   bodies through the same reader; a float loop bound refused by name on every
   route (a fail-closed repair of the bit-pattern reading, before any float can
   reach the tier). The emitted C gains the float includes, the IEEE
   environment and the renderers when a float is held.
2. **Payloads.** `Buffer<f32>`/`Buffer<f64>` on the task route (sizes 4/8 via
   `size_bytes`, the typed payload's bits logged by `memcpy`, a float arm in the
   sink printing the bit pattern and every NaN as `nan`); `protected f32`/`f64`
   on the mutex route (4/8 bytes). Lean: a new module re-proving the typed
   `WRITE-LEND-TASKS` disjointness at float extents and stating that every byte
   pattern of a float payload is a value.
3. **Composed string route.** Its entry row's comparison effects, its reader
   reading float spellings through the buffer route's reader, and the emitter's
   float environment.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixture freeze AFTER vs BEFORE (702 files) | byte-identical for every existing file; new entries only the card's examples | 75% |
| existing tests changed | 1–4 files: the float card's "a float payload stays refused" row, pins of the buffer route's float refusal, struct literals gaining a field | 60% |
| native tables (buffer route, payloads) at `-O0`/`-O2` UBSan and plain `-O2`, `-ffp-contract=off -Wall -Wextra -Werror -pedantic` | equal to the reference bit for bit (NaN as NaN), 0 sanitizer reports | 70% |
| TSan on the payload rows with workers | 0 reports | 80% |
| Lean | one new module and audit, 8–16 theorems, no placeholder or project axiom | 80% |
| corpus arbiter `resource-diagnostics.json` | byte-identical to the base's archive (`2fc3af8c…`, W48-QR-FLOATING-POINT's AFTER) | 95% |
| standing gate first run | green | 40% |
| items completed | 1 (75%); 2 (60%); 3 (45%) | |
