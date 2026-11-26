# SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN: prediction

Committed before the implementation commit. Base `43410b230` (main).
USER DECISION 2026-11-25: "lanich four of queued cards".

## What was known when this was written

* **Probe at base** (`crates/qcore-c-gateway/tests/settled_leak_joined.rs`,
  commit `d09a6ae0f`): ten fixtures over the generic descriptor surface are
  all `Unknown` with no leak. Five have a path avoiding every refused
  release that leaves the identity live: the pinned
  `if (flag && k_close(fd) == 0) {}`, a refused close on one arm, a refused
  close inside a `while` loop, two refused closes on two arms of three, an
  early return beside a refused close, and a nullable stream closed
  `nonnull` on one arm. Four controls have no such path: the only path
  passes the refused close, both arms close, a `do`/`while` body closes on
  every path, and the other arm hands the stream to an uncontracted call (a
  refused hand-on, rule W44's marker).
* **NULLABLE-GUARDED-RELEASE's measurement** (its report, §4): at
  `686ddf8f4`, no opt-in row moved by rule 74c (A) alone. Its only measured
  (A) withdrawal was `rd_kafka_mock_handle_Metadata` with the held
  constructors branch merged, and (B) already makes that row leak-free.
* Not measured before this commit: how many reports (A) withdraws on the
  base corpus, and how many of those have a path that avoids every refused
  release.

## Planned change

A leak report rule 74c (A) would withdraw is re-decided by a restricted
custody fixed point for that identity: the same transfers, edge refinements
and joins as the work list, except that any state in which the identity is
marked unresolved is not propagated. Every refused release of the identity
marks it, so the restricted states are joins over exactly the CFG paths on
which nothing marked it. The report is published when the restricted
settled exit claims it by `report_leaks`' own conditions (live, not exempt,
not marked, no reference family) and a path from a producer of the identity
to that exit keeps it live on every restricted edge. Otherwise it stays
withdrawn. Only `Unknown -> Unsafe` moves (and new `Leak` leaves in rows
already `Unsafe`) can follow. Nothing can become `Safe`.

## Predicted movements (opt-in route, base vs tip)

**Safe -> non-Safe: 0. Unknown -> Safe: 0** (the change only re-publishes
leak leaves).

**Unknown -> Unsafe: 0** (0 to 2 possible). A report can return only where
(A) withdraws one at base, and NULLABLE-GUARDED-RELEASE measured no (A)
status move on this route. A row with no avoiding path keeps its withdrawal.
Any move is reported per row with its witness path as a provisional Unsafe.

**Unsafe -> anything: 0.** The base's provisional Unsafe rows keep their
status. A leaf-only change is possible: an extra `Leak` leaf in a row already
`Unsafe`.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host (no `build/cycle19` inputs or baselines). Prediction for the
integrator: no status moves on the corpus, system or tail-pause slice. The
possible exception is a row where (A) withdrew a leak that a path avoiding
every refused release still carries. Each such row is a provisional
`Unknown -> Unsafe` and is reported as unpredicted.

**Census** (`indirect-census` build, `QCORE_LEAK_WITNESS`, the tip): the
number of published leak witnesses with `settled: false` and the
provisional Unsafe rows resting only on them. Not predicted by number; the
NULLABLE-GUARDED-RELEASE census found the Metadata leak alone, on the held
branch.
