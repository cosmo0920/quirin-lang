# Reproducing the CT41 measurement

Card `CT-CTWIN-LANE` (wave 35). The numbers in `ct41-measurement.json` were
derived from the `.c` files in `ct41-shapes/` beside it, so this archive *is*
the thing that was measured.

**Nothing in the committed test suite runs these shapes.** CT41 is unmapped, so
it has no runner, and adding one would be exactly the vacuous mapping this lane
declined (see the report's §1). The harness below is therefore supplied as an
artifact rather than as a test.

## Steps

1. Copy `ct41_measure.rs.txt` to `compiler/tests/zz_ct41_measure.rs`.
2. Run:

   ```sh
   cargo test -p qcore-compiler --test zz_ct41_measure -- --nocapture --test-threads=1
   ```

3. Compare the printed blocks with `ct41-measurement.json`'s `shapes` array.
4. Delete `compiler/tests/zz_ct41_measure.rs`. It is not part of the suite and
   the standing gate does not expect it.

The harness reads the shapes by relative path from `compiler/`, so it must be
run from a checkout in which `docs/reports/ct-ctwin-lane/artifacts/ct41-shapes/`
is present.

## What a difference would mean

* **A and B differing** would contradict card `CTWIN-PUBLISH` §5's decision and
  its Rust pin `a_volatile_qualifier_moves_no_resource_projection_verdict`. That
  would be a defect in the projection, not a newly mappable CT41: it would mean
  a verdict depended on a fact the projection never derived.
* **D and E differing** would mean the same thing one shape over.
* **A, B, D or E moving while A≡B and D≡E hold** is an ordinary consequence of
  gateway work and is not about CT41 at all — it happened between wave 17 and
  this lane (the `AOM-BINDING-POISON` narrowing, attributed in
  `ct41-measurement.json`'s `delta_from_wave_17_attributed`).
* **C ceasing to be `safe`** would mean the isolating control no longer isolates,
  and the attribution of A/B's `unknown` to the flag store would need re-taking.

## Environment of the recorded run

Ubuntu clang 21.1.8 (`/usr/bin/clang`), `x86_64-pc-linux-gnu`, `QCORE_CLANG`
unset, on `main` at `a300a39d` plus this card. Clang is required: with no usable
Clang the gateway entry points do not run and the harness would measure nothing.
