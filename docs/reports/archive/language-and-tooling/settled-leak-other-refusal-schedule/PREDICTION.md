# SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE: prediction

Committed before the implementation commit. Base `98c51878f` (main). USER
DECISION 2026-11-26: "launch the SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE card".

## What was known when this was written

* **Probe at base** (`crates/qcore-c-gateway/tests/settled_leak_other_refusal.rs`,
  commit `1ebe639f6`). Three refusal kinds were tested: a refused hand-on
  (`u_keep(s)`), a refused escape (`*out = s`) and a refused condition call
  (`if (u_ok(s))`). Each sits on one arm, and the other arm leaves the
  stream open. Each is `Unsafe` with one leak when the avoiding arm reaches
  the exit first, and `Unknown` with no leak when the refusing arm does. The
  same refusal on every path is `Unknown` in both orders.
* **Existing pins that rest on a schedule claim at base.** Two pins publish a
  leak whose settled exit marks the identity by a non-release refusal:
  * `chained_assignment::the_scram_shape_withdraws_the_leak_and_does_not_become_safe`
    (its "old" spelling, beside rule 42's refused re-acquisition, the snappy
    shape);
  * `scalar_route_exit_edge::a_body_that_writes_the_deciding_scalar_reaches_the_unknown_arm`
    (the `z_unknown` row).
* **SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN's census at its tip**: 31
  `settled: false` current-generation witness lines. Two provisional Unsafe
  rows rest only on them: `ut_testSameSubscriptions` (4 leaks) and
  `rd_kafka_snappy_java_uncompress` (3 leaks).
* Not known when this was written: the base census by refusal kind (the
  probe commit `9676ea22a` adds the census field; the run is in progress).

## Planned change

Rule 84 is extended to every refusal kind. Its candidates become, at every
exit, the current-generation identities that the settled incoming state (or
the exit's own output) marks unresolved by any refusal, beside an `Unknown`
leaf. Rule 74c's held reports leave those candidates to rule 84, whether or
not an iterate held them. A candidate is published only when rule 84's
restricted replay still claims it and a producer-to-exit witness keeps it
live, unmarked and unrefused. The restricted replay cuts every marked or
`unmodeled` edge. Otherwise the candidate is withdrawn.

## Predicted movements (opt-in `current --monkey-linked`, base vs tip)

* **Safe -> anything: 0. anything -> Safe: 0.** A decided report always sits
  beside an `Unknown` leaf.
* **Unsafe -> Unknown: 1 to 2.**
  * `rd_kafka_snappy_java_uncompress`: predicted to move. Its current-
    generation leaks lie downstream of the refused re-acquisition. That
    node's `unmodeled` output cuts every path, as in the scram pin.
  * `ut_testSameSubscriptions`: predicted to move unless a witness reaches
    one of its four assertion returns without a refusal. Its function body
    has refused statements on the paths before the returns, so a witness is
    unlikely.
* **Unknown -> Unsafe: 0 to a few**, not predicted by row. This needs an
  `Unknown` row where a non-release refusal marks a leaked identity on one
  arm and a refusal-free arm leaves it live, with the marked arm reaching
  the exit first (the probe's second order). Each such row would be a
  provisional Unsafe, reported with its witness.
* **Leaf-only changes**: possible in the other provisional Unsafe rows
  (removed or added `Leak` leaves), not predicted by row.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host: there are no `build/cycle19` inputs or baselines. The block declares
no move on them. For the integrator, a corpus row whose leak rests only on
an iterate claim marked by a non-release refusal would move Unsafe ->
Unknown and be reported as unpredicted. So would a row with a refusal-free
arm beside a marking arm (Unknown -> Unsafe).

**Census** (`indirect-census` build, `QCORE_LEAK_WITNESS`, base and tip):
the base `settled: false` lines classified by the refusal kind of their
markers. At the tip, no provisional Unsafe row should rest on a
`settled: false` line whose site is marked at the settled exit beside an
`Unknown` leaf.
