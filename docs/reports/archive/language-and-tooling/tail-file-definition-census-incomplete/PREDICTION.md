# TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE: prediction

Committed before any tip measurement. Base `f47823dd1` (integrate/r10). At
commit time the guard-correlation scan, its fixtures and the Lean model were
drafted in the working tree and run only on fixtures and on raw source files
(no route had run a tip binary). The base figures below are this card's own
base quick run (`private/runs/base-slice-resolution`, binary `56b7aedb…`
built from a `git archive` export of `f47823dd1`) on its own `prepare
clang-build --link` of upstream `90978ac6`.

## Root cause (read before the prediction)

`plugins/in_tail/tail_file.c` at `90978ac6`, `tag_compose`: lines 1255–1310
are `#ifdef FLB_HAVE_REGEX` … `else {` `#endif` (the arm opens one brace, the
implicit empty arm none), and lines 1368–1370 are `#ifdef FLB_HAVE_REGEX` `}`
`#endif` (the arm closes it). E01-RAW-COND reads each group alone, refuses
both (`ArmDepthDisagreement`) and carries the greater depth, so the raw scan
ends the file one level deep and hides 16 of its 48 definitions. It is the
slice's only unit with an enumeration refusal (base report: one
`enumeration completeness refused` reason).

## Mechanism (drafted)

Rule 49: when the raw scan's only ground is an arm disagreement, a
guard-correlation scan (worlds over `#ifdef NAME`/`#ifndef NAME` groups
without `#elif`; any other directive clears assumptions; every other group
must still agree) may decide it; its names are unioned into the raw half and
the refusal is dropped. Anything it cannot decide keeps the refusal.

Raw-source reach (every `.c` file of the three Fluent Bit trees): 19 files
per tree have the refusal as their only ground, 14 are decided. Of these the
live routes select only `plugins/in_tail/tail_file.c` (slice). The system
selection's 8 files and the corpus selection (cmetrics, ctraces, librdkafka,
chunkio, `src/flb_input_chunk.c`) contain none.

## Predicted measurements

* **Census.** `tail_file.c`'s enumeration refusal disappears; its census
  holds the same 48 definitions (the expansion half already had them).
* **Import phase (slice).** Runs on every route mode; the line `cross-unit
  import phase refused: incomplete definition census` is gone.
  * Rule 40: 124 derived homes (unchanged); home exports 8 → **10**
    (`tail_file.c` is now analysed for export: its externally linked Safe
    rows `flb_tail_file_remove_all` and `flb_tail_file_dormant_scan_begin`;
    its other Safe rows are `static`).
  * Stated map: its eight homes are not slice units, so nothing is imported.
* **Rows, rule 40 quick runs (slice): no status move**; totals stay 59 Safe /
  1 Unsafe / 340 Unknown / 4 Pending. No base Unknown row has only
  home-call leaves. Leaves traded on rows that stay Unknown: `in_tail_exit`
  (loses `flb_tail_file_remove_all`), `flb_tail_scan` (loses
  `flb_tail_file_dormant_scan_begin`), `flb_tail_config_destroy` (loses
  `flb_tail_file_remove_all`, and others if their homes export), plus any
  caller of the eight base exports; each keeps other leaves.
* **Rows, stated map:** none moves, no leaf trade.
* **Arbiter (`measure --declared`, stated map, slice): none.** System: not
  measured (no selected unit has the refusal, so the change is not reached);
  corpus: not measured, none, for the same reason.

```measure-prediction
route slice
none
```
