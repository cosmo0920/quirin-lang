# Landing batch D (integrate/r77): refusal-kind leaks, member-path invocation

USER DECISION 2026-11-27: "land as rNN." Batch D lands two cards in one
integrate branch with one full landing gate:

| Card | Report | Rule |
|---|---|---|
| SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE | [report](../settled-leak-other-refusal-schedule/README.md) | 84 amended (74c, 6 point to it) |
| MEMBER-PATH-INVOCATION-PREMISE | [report](../member-path-invocation-premise/README.md) | 30 amended (13v points to it) |

Each card's own report holds its evidence; this page records the integration.
The landing gate record is [integration/gate.json](integration/gate.json).

## Integration

* Both cards were based on main `98c51878f`, and main had not moved when they
  were merged. They share only the generated rule docs, which were
  regenerated; both merges were clean.
* Each card's amendments were applied in the order above with
  `--profiles-base origin/main`.
* SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE leaves the work-list loop untouched, so
  `Replay::step` needs no mirror change; its `debug_assertions` replay check
  runs in this batch's gate over the merged tree, MEMBER-PATH's
  `member_path_invocation` route included.
* Measurements are each card's own (opt-in `current --monkey-linked` over
  Fluent Bit 046accf6): SETTLED-LEAK moves `ut_testSameSubscriptions` from
  provisional Unsafe to Unknown; MEMBER-PATH moves no row. No combined
  measurement was run. Pinned-route parity was not run on this host (no
  `build/cycle19` inputs), and `land.py publish` is owed.
