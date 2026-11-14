# SELECTION-ROUTES-SPLIT-CALLER-AND-HOME: prediction

Committed before the new route was measured under rule 40. What had been
run when this was written (base binary `95eb1575…`, built from `f47823dd1`;
this card's `prepare clang-build --link` of Fluent Bit `046accf6`):

* the new `system-teardown` selection under the **stated map** (the before
  side of the measured comparison), first with `src/flb_config.c` in it
  (`private/runs/teardown-map-1`: its census is refused, which refused the
  import phase; that file is now left out), then without it
  (`teardown-map-2`, running as this is committed);
* the unchanged `system` selection under **rule 40** (`system-resolution-1`):
  105 derived homes, 14 exports, 12 imported, no refusal under condition 13,
  as LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS measured (it counted 13
  imported on its own preparation).

The measure plugin's `--link-resolution` side and its fixtures were drafted
and committed (`d487691cb`) but had measured nothing.

## The measured comparison

`quirin-audit measure --declared` over two routes on this card's
preparation, with two sides of one analyzer binary (the tip's): `map` (the
stated eight-home map) and `linked` (`--link-resolution linked`: the
preparation's artifact). Routes: `system` (the live system selection) and
`system-teardown` (the system selection plus `src/flb_input.c`,
`src/flb_input_thread.c` and `src/flb_output_thread.c`). The analyzer is
unchanged by this card, so this comparison isolates what rule 40 adds over
the stated map on each selection; no base-versus-tip comparison of the
analyzer is needed (no `crates/qcore-*` or contract file changes).

## Predicted moves

No status moves on either route. Reasoning, from the stated-map rows
(`private/scripts/candidates.py`: every Unknown row of a linked, static
configuration whose leaves name an uncontracted call to a function that is
Safe in another static unit, the only homes rule 4c can export):

* No such row has only those leaves, so no row can reach Safe by an import
  alone; no import can turn a Safe row Unknown, since the map imports nothing
  on these selections (its eight homes are outside them).
* `system`: as the previous card measured (no move, four rows trade leaves).
* `system-teardown`, the released-element drains now reached:
  `flb_input_instance_destroy` (`src/flb_input.c`, static configuration,
  60 leaves) is expected to import `flb_engine_destroy_tasks` and
  `flb_storage_input_destroy` (both Safe homes) and lose their three
  uncontracted-call leaves, with rule 43's `ElementsReleased` applied at the
  call; it stays Unknown on its other 57 leaves (unresolved `ins`, indirect
  accesses, `flb_config_map_destroy`, `flb_kv_release`, member releases
  through the formal, ...). Its shared configuration fails condition 3.
  `flb_input_collectors_start` loses `flb_input_thread_collectors_signal_start`
  and stays Unknown. The drains `flb_sched_timer_cleanup`,
  `flb_sched_destroy`, `flb_output_exit` and `flb_task_retry_clean` are
  Unknown homes, so their callers (the thread loops, `flb_engine.c`) import
  nothing from them.
* Content changes (not part of the declared block): the rows above and the
  system route's four (`handle_output_event`, `flb_engine_dispatch`,
  `flb_task_retry_create`, `task_collect_output_references`); others are
  possible where a leaf other than an uncontracted call depends on an
  imported home, and each will be read.

```measure-prediction
route system
none
route system-teardown
none
```
