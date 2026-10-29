# Pilot round 2: combined prediction for rules 33, 34 and 36

Committed before the shared measurement starts (brief
`private/integrator-briefs/pilots-r2-evidence.md`), at `62c8e461c`. The one
measurement serves RULE33-PILOT, RULE34-PILOT and RULE36-PILOT:
BASE `rev:0d15e7b19` (main with every change except the three moves) against
TIP `rev:62c8e461c` (the three moves, the registry rows and their Lean switch
modules; `git diff 0d15e7b19 62c8e461c -- crates proof compiler contract`).

The block below merges the three cards' `measure-prediction` blocks
([33](../rule33-pilot/PREDICTION.md), [34](../rule34-pilot/PREDICTION.md),
[36](PREDICTION.md)), each committed before its card's code: every one
declares an identity, so the merge is an identity. BASE -> TIP moves no row
on any of the three live routes; reports are byte-identical up to the checked
one-to-one substitution of the corpus side's working-directory and
revision-derived digests (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY).

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The contributions (`--rules-off 33,34,36 --per-rule`: TIP against TIP with
each rule off, and with all three off) are not part of the declared
comparison. Expected direction only, as each card forecast: rule 33 off moves
`cJSON_DetachItemFromArray` and `cJSON_DeleteItemFromArray` (corpus)
`Safe -> Unknown`; rule 34 off moves the Monkey rows
`sched_thread_cleanup` and `mk_sched_threads_destroy_conn` (corpus) away from
`Safe`; rule 36 off moves nothing (W48-BU-CMT-MAP-METRIC-GET measured its gain
as 0). No system or slice row is forecast to move. No row may move toward
`Safe`; any such row is a soundness finding, not a contribution. Rows that
move only with two or more rules off are interactions, recorded as the tool
reports them, never attributed to one rule.
