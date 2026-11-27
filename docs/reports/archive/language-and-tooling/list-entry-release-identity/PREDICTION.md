# LIST-ENTRY-RELEASE-IDENTITY: prediction

Frozen before any tip measurement. Base `74105dd0e`. The census
(`scripts/census.py`, BEFORE reports of the system and tail-pause slice
routes measured on this card's own preparations,
`artifacts/census-{system,slice}-before.json`) read every leaf of a non-Safe
row that stands on a destroyer call, an unlink, or a call to one of the six
undeclared list-entry teardown functions, classified by the lexical shape of
the released actual.

**Disclosure.** The gateway code, the Lean module and the fixtures were
drafted in the worktree before this file was committed; none of it had been
measured on a native route when this prediction was written, and the
census reports are BEFORE reports of base `74105dd0e`'s binary.

## Census at base (destroyer callees only; the `flb_free`-family leaves are
## listed in the artifacts and not this card's)

| Route | Shape | Leaves | Rows |
|---|---|---|---|
| system | unresolved identity, element of a formal-rooted list (`element-formal`) | **50** | 10 |
| system | unresolved identity, other actual (local acquisition, poisoned) | 42 | 13 |
| system | unlink of a formal's own object (W45-FORMAL-ENTRY-UNLINK) | 20 | 9 |
| system | indirect access, member-read actual | 16 | 6 |
| system | uncontracted undeclared teardown function | 16 | 4 |
| system | unresolved identity, formal | 10 | 4 |
| system | element of another container / unlink of one | 6 + 8 | 2 + 2 |
| slice | unresolved identity, other (`flb_tail_config_destroy(ctx)` in `flb_tail_config_create`/`in_tail_init`) | 44 | 6 |
| slice | unresolved identity, formal | 16 | 8 |
| slice | unresolved identity, element-formal | **11** | 5 |
| slice | unlink of a formal's own object | 10 | 6 |
| slice | element-other / undeclared teardown / member | 5 / 4 / 2 | 3 / 3 / 1 |

The dominant generic shape on the system route is the **element of a
formal-rooted list released by a declared destroyer** (50 of 160 destroyer
leaves): rule 10f(2) already admits such an element, but only at an unlink,
and no declared destroyer states its unlink, so the release reads an unbound
name. On the slice the largest group is one local (`ctx`), which is not a
list entry and stays out of scope.

## Mechanism

1. The ten destroyers whose bodies (read at the three trees,
   `artifacts/unlinks.json`) call `mk_list_del(&P->_head)` before
   `flb_free(P)` declare `resource_unpublished(P)` beside the release, so
   rule 34 takes the element's custody first.
2. Rule 43, part 1: an element destroyer may take an object the path already
   holds (live, unpublished, never unlinked), so a formal hand-off
   (`flb_sched_timer_cb_destroy`) and a body acquisition keep their release.
3. Rule 43, part 2: a body whose element custody is rooted at its formals
   publishes its summary with `ElementsReleased` at those formals; a caller
   forgets every object it had published and carries the clause on.

## Predicted moves

* system: `flb_engine_destroy_tasks` and `flb_storage_input_destroy` (each
  one leaf per configuration, an element-formal destroyer leaf)
  Unknown → Safe in both configurations; `flb_sched_timer_cleanup` and
  `flb_task_retry_clean` lose their element leaves and may stay Unknown on
  another leaf. Leaf trades in the other element-formal rows
  (`flb_output_exit`, `flb_output_init_all`, `flb_sched_destroy`,
  `flb_task_destroy`, the two `*_release_space`): the destroyer leaves go,
  member reads before the destroy may stay (W45-FOREIGN-UNLINK-RESIDUAL-SHAPES
  part 6). Callers of the newly summarised bodies trade
  `call to uncontracted function` leaves. `flb_sched_timer_cb_destroy` stays
  Safe (the held arm). No Safe row moves.
* slice: `flb_tail_file_remove_all` (two element-formal leaves, nothing else)
  Unknown → Safe; `in_tail_exit` and `flb_tail_config_destroy` lose their
  uncontracted `flb_tail_file_remove_all` leaf and stay Unknown.
* corpus: no status move (the 66910c10 selection's Fluent Bit units are not
  the teardown files); leaf trades possible.
* No leak, double release or Unsafe added on any route.

```measure-prediction
route system
move flb_engine_destroy_tasks Unknown -> Safe
move flb_storage_input_destroy Unknown -> Safe
route slice
move flb_tail_file_remove_all Unknown -> Safe
route corpus
none
```

## Where this could be wrong

* An element bound by `mk_list_foreach_safe_r` (reverse) or read through a
  member before its destroy may not be admitted by rule 10f(2), leaving the
  leaf (`flb_input_exit_all` reads `ins->p` first).
* A declaration with a counted-user clause (`flb_task_retry_destroy`'s
  `resource_user_neutral()`) may be checked against its body by rule 10e in
  the unit defining it; a container clause there could make it opaque in
  that unit.
* OR-6 (`flb_output_task_queue_flush_one` / `flush_next`) is a mutually
  recursive pair: neither publishes first, so the summary cannot reach it;
  predicted unchanged.
