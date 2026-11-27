# SCHEDULER-DESTROY-MEMBER-STATES: prediction

Committed before the first semantic commit. Nothing had been measured or
built from this card's changes when it was written; the base is main
`89fa31d21` (integrate/r11 landing).

## What the card changes

On the user's decisions of 2026-11-14 ("Go with 1, 2A and typed refinements
for 3"):

1. `mk_http_thread_destroy` (`qcore-provider-libevent`,
   `contracts/monkey/intrusive_list.qv`) gains three member-path states on
   its released formal `mth` (C04.19.6, rule 30): `parent |-> #stale`,
   `session |-> #kept`, `request |-> #kept`, beside its unlink and release.
2. The inline `mk_thread_destroy` (`mk_thread_libco.h`) gets its own
   declaration (released `th`, the `cb_destroy` member invocation, `callee
   |-> #stale`, `caller`/`data |-> #kept`), and rule 30's body reading
   follows a direct call of such a unit `static` helper that receives a
   stated member's value of the formal (directly or through a single local
   copy), composing the helper's declaration: what it releases is the
   object the member held (gateway rule 50 if a new rule is cleaner).
3. Typed refinements (`quant proposition`, `resource_lifecycle_v1`,
   `resource_premise_decomposition`, `resource_premise_flow_v1`) for the
   premises with no member form; premise prose narrowed to what no symbolic
   clause states. Every premise name stays.

The declaration's text changes, so it is re-pinned at the two revisions it is
pinned at (046accf6, 66910c10); `mk_thread_destroy` is pinned there too.

## Predicted moves

No status moves on any route.

* **Consumers of `mk_http_thread_destroy`** (corpus: `sched_thread_cleanup`,
  `mk_sched_threads_destroy_conn`, and through rule 43's summary
  `mk_sched_threads_purge`, `mk_sched_threads_destroy_all`): each binds the
  element through `container_of(..., _head)` and links no tracked object at
  `mth->parent`, `mth->session` or `mth->request`, so rule 30's direct-call
  composition (`compose_direct`) finds no linked object to decide and the
  custody reading is the one at base. The premise names they rest on are
  unchanged; their texts and the declaration's hypothesis graph (now
  version 3 with refinements) change, so these rows are expected to differ
  in content (premise statements, graph metadata), not status.
* **`mk_http_thread_destroy`'s own row** stays Unknown. Its call
  `mk_thread_destroy(th)` meets a declaration whose member-path states stand
  beside a member invocation, which a direct call refuses
  (`member-path-direct`, as every non-plain member-path declaration does), so
  it may trade that leaf; it gains no Safe.
* **Rule 30 / rule 50 standing** (evidence, not status): in
  `mk_server/mk_http_thread.c` the three new states are expected to read
  validated (the unlink's link member `_head` no longer ends the reading
  inside `mk_list_del`), and the stale `parent`'s object is expected to be
  established as taken by the followed `mk_thread_destroy(th)` when its
  `cb_destroy` set reads closed and empty over the run's member facts.
  Where the run's facts are not installed (no pre-pass) it stays a declared
  assumption.
* The system and slice routes contain no Monkey scheduler unit: no row there
  rests on either declaration. No change.
* No Unknown → Safe, no Safe → Unknown, no Unsafe added or removed; the null
  property is unchanged.

If the pins cannot be written at a revision, the declaration is out of force
there and the four corpus rows above would fall to Unknown: that would be a
pinning failure, reported as such, not a semantic result.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
