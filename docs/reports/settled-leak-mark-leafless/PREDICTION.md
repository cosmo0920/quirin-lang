# SETTLED-LEAK-MARK-LEAFLESS: prediction

Committed before the implementation commit. Base `287dce8ed` (integrate/r86).
User decision of 2026-11-28: option (b), an `Unknown` reason "mark without
refusal".

## What was known when this was written

* **Probe at base** (`8ba33c206`,
  `crates/qcore-c-gateway/tests/cross_unit_elements_released/mark_leafless.rs`).
  Rule 43's imported `ElementsReleased` clause (`drain`) marks every object
  the caller published, with no refusal. Three caller shapes, each in two
  schedules, are `Unsafe` with one leak when the arm leaving `x` open reaches
  the exit first and `Unknown` with no leak when the drained arm does:
  * `ONE_ARM`: publish and drain on one arm, the other arm open;
  * `RETAINED`: the same, the open arm through a retained loop exit edge
    (`unmodeled` with no diagnostic before the settle point);
  * `JOINED`: a one-arm publication, the drain after the join.
* **Probe census** (`QCORE_MARK_CENSUS` on those fixtures): `x` is live,
  marked and claimable at the settled exit beside no leaf in five of the six
  programs. Rule 84's witness carries it in `ONE_ARM` and in the
  published-first `JOINED` program (its mark is a stale iterate's: the
  published set joins by intersection and the mark by union), and not in
  `RETAINED`. The unpublished-first `JOINED` program has no mark.
* **Suite census at `07833c86a`** (the previous card's): 9 lines beside no
  leaf, all rule 43 marks on an exempt entry-custody site, never claimable.
* **Fluent Bit census at `07833c86a`**: no line at all on the opt-in route.
* Not known when this was written: the suite census at this base and the
  Fluent Bit census at this base (running).

## Planned change

Beside no blocking leaf at the settle point, a marked current-generation
identity the settled exit would claim but for the mark (live, not exempt, no
reference family) is decided from the settled states: published as a `Leak`
on rule 84's witness, and otherwise withdrawn with the `Unknown` reason
`mark-without-refusal` at the exit. Rule 74c's held reports leave it alone.
An exempt or non-live marked identity is not a candidate.

## Predicted movements (opt-in `current --monkey-linked` and the system route, base vs tip)

* **anything -> Safe: 0.** A decided candidate leaves a leak or the reason.
* **Safe -> Unknown: 0** expected, allowed only where the reason fires (a
  live claimable identity marked by a modelled effect, beside no leaf, on no
  witnessed path); each would be named.
* **Unknown -> Unsafe: 0** expected: a witnessed open arm the schedule hid.
* **Unsafe -> Unknown: 0** expected: a leak only an iterate claimed, with no
  witness.
* Leaf-only changes in rows already `Unknown` or `Unsafe` (the reason beside
  other leaves, a leak beside `EXIT_WITHHELD`): possible, 0 expected.

The expected value of each figure is 0: rule 43's clause reaches a caller only
through a derived or imported `ElementsReleased` summary, and the previous
card's census found no leafless mark in Fluent Bit.

**Suite.** The 9 exempt lines keep their verdicts (`Safe`) and record no
reason. Only the new fixtures move (their pins change from the base's
schedule-read verdicts to one verdict per shape).

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). The block declares no move on them.
For the integrator, a pinned row whose leak rests on an iterate claim of an
identity a modelled effect marks beside no leaf would move `Unsafe ->
Unknown` (unwitnessed) or stay `Unsafe` (witnessed); a hidden refusal-free arm
would move `Unknown -> Unsafe`; a row with no other leaf would move `Safe ->
Unknown` where the reason fires. Each is to be reported as unpredicted.
