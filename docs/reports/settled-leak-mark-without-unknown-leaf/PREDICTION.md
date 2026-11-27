# SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF: prediction

Committed before the implementation commit. Base `07833c86a`
(integrate/r77).

## What was known when this was written

* **Probe at base** (`crates/qcore-c-gateway/tests/settled_leak_mark_without_unknown.rs`,
  probe commit). A call held for review (`k_close` under a deferred contract)
  on one arm, and an arm leaving the handle open: `Unsafe` with one leak when
  the open arm reaches the exit first; `Unknown` with no leak when the held
  arm does (the row is `Pending` after the first analysis and rule 10i's
  standing analysis records the leak candidate). The hold on both arms is
  `Pending` in both orders.
* **Gateway suite census at base** (`QCORE_MARK_CENSUS`, `indirect-census`):
  130 marked current-generation `(exit, identity)` lines in functions with no
  `Unknown` leaf. 121 sit beside a `Pending` leaf and 9 beside no leaf at all
  (`cross_unit_elements_released`: rule 43's `ElementsReleased` marks an
  entry-custody site, which is never claimable). **None** was held by an
  iterate and **none** is witnessed, so no existing suite pin changes.
* Not known when this was written: the Fluent Bit census (running).

## Planned change

Rule 84's side condition widens from an `Unknown` leaf to an `Unknown` or
`Pending` leaf. Such a marked identity is published exactly on rule 84's
witness and withdrawn otherwise; a withdrawal keeps the `Pending` leaf, so the
row is at least `Pending` and rule 10i's standing analysis may raise it to
`Unknown`. A mark beside no blocking leaf stays with rule 74c.

## Predicted movements (opt-in `current --monkey-linked`, base vs tip)

* **anything -> Safe: 0. Safe -> anything: 0.** A decided report sits beside
  an `Unknown` or `Pending` leaf, and a function with neither is untouched.
* **Pending -> Unsafe: 0 to 2** (the two `Pending` rows), only where a
  refusal-free path leaves a marked identity open.
* **Unknown -> Unsafe: 0 to a few**: a row whose first analysis has only hold
  leaves, raised to `Unknown` by the standing analysis, with a witnessed open
  arm hidden by the schedule.
* **Unsafe -> Unknown or Pending: 0 to a few**: a row whose leak rests only
  on an iterate claim of an identity a hold marks, with no witness.
* Leaf-only changes in other `Unsafe` rows: possible, not predicted by row.

The expected value for each figure is 0: the opt-in route has two `Pending`
rows and the suite census found no held or witnessed case.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host (no `build/cycle19` inputs or baselines). The block declares no move on
them. For the integrator, a pinned row whose leak rests only on an iterate
claim of an identity a hold marks, beside no `Unknown` leaf, would move
`Unsafe -> Pending` or `Unknown`; a hidden refusal-free arm there would move
`Pending`/`Unknown -> Unsafe`. Each is to be reported as unpredicted.
