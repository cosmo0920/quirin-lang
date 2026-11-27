# W48-QR-SCALAR-TYPES-R5: frozen prediction

Frozen after the BEFORE fixture freeze and before any source, test or Lean
change. Base: local `main` at `6883623c` (= origin/main); the worktree's HEAD
was already that commit, so neither `git reset --hard main` nor
`git merge --ff-only main` was needed.

## BEFORE (measured)

`quirin` debug binary built at the base (sha256 `681a37fe2239be0a…`). The first
half's `freeze_fixtures.py` over the 630 tracked `.qr`/`.qv`/`.qrlib` files
([BEFORE](artifacts/fixtures-before.json)): check exit 0 on 358, exit 1 on 272;
emit exit 0 on 153, exit 1 on 477.

Probes at the base (`private/w48-r5/probes/`, not tracked):

* `let n = 3 as u8; for i in 0..n { print dec i; }` on `source.buffer.v1`
  refuses at `Q098` ("`n` is not a check-time-constant bound");
* `let y = x + 1;` on `source.buffer.v1` refuses at `Q051` (the route binds a
  literal, a name, `bytes`, `extent`, `lend` or a byte read);
* `for i in 0..3 { let j = i as u8; print dec j; }` is admitted (the
  conversion over the `i32` index, decided per iteration);
* **found:** on the composed string/buffer route a conversion inside a `for`
  body (`let j = i as u8;`) is admitted by `check` and `emit`, and the emitted
  C does not compile — it calls `qcore_buffer_as_i32_u8`, which the composed
  emitter never defines (`buffer_loop_block` drops the conversion set). The
  R3 record says that route "lowers none"; through a loop body it does.

## Mechanism (planned)

1. **`W48-QR-TYPES-LOOP-BOUNDS`.** The lowering's constant tier (the one a
   bound is decided in) gains a typed half: a conversion over a value of the
   tier is folded there by the shared decision (`scalar_values::Decider`), so
   `let n = 200 as u8;` makes `n` a known `u8`. The bound decision moves into
   the core (`source_loop`), total over every §G02t type: `bool` refused,
   `isize`/`usize` need the recorded target (threaded into the lowering),
   zero/negative/above-4096 refused as before (values widened to the full
   table). `BufferEvent::Repeat` carries the index type; the checker binds the
   index at that type per iteration and re-checks that the count is a value of
   it; the emitter writes `for (T i = 0; i < N; i += 1)` at the type (an `i32`
   loop's bytes unchanged). Lean: every index is a value of the type, the step
   `i + 1` never faults and the loop runs exactly `N` iterations, at every
   width including `u64` near its maximum and signed bounds near `MIN`/`MAX`
   (those are refused by the ceiling or the sign rule).
2. **`W48-QR-BUFFER-ROUTE-ARITHMETIC`.** The buffer route gains the shared value
   statement (arithmetic, comparison, negation) read by
   `source_value_statement` and decided by `Decider`; every operand on this
   route is known at check time (the loop index per iteration), so each
   statement folds and a fault refuses at `Q053`; emitted through §G02t's
   shared `q_scalar_*` helpers (witnesses). Results over the lowering tier are
   bounds too.
3. `W48-QR-TYPES-PAYLOADS`, then `W48-QR-TYPES-EPOCH-TASKS` /
   `W48-QR-F02-FIXED-OUTCOMES`, as time allows; a position not finished keeps
   its refusal and follow-up.

The composed string/buffer route keeps refusing every typed statement: it will
refuse a conversion, a value statement and a non-`i32` loop at any depth by
name (a correction for the found conversion-in-a-body shape).

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixtures AFTER vs BEFORE (630) | byte-identical except new examples | 80% |
| existing tests changed | only constructions of `BufferEvent::Repeat` (new field) and pins of a widened refusal | 65% |
| native tables (UBSan `-O0`/`-O2`, plain `-O2`) | every stream equals the reference, 0 sanitizer reports | 80% |
| Lean | new audit file, no placeholder, no axiom | 85% |
| corpus arbiter | byte-identical `ae17152b…` | 97% |
| standing gate | green, new tests only (after at most one load-sensitive rerun) | 65% |
| items completed | 1 (85%), 2 (60%), 3 (15%), 4 (5%) | |
