# RULE50-CONSUMER-COMPOSITION: prediction

Committed before the first semantic commit. Nothing had been built or
measured from this card's changes when it was written. The base is main
`0f6e17118` (integrate/r12). The card folds in MEMBER-PATH-HELPER-AT-A-CALL.

## What the card changes

1. **A helper destroyer at a direct call** (MEMBER-PATH-HELPER-AT-A-CALL).
   A direct call of a declaration whose member-path states stand beside
   member invocations of the same released formal (only `mk_thread_destroy`
   today) stops refusing `member-path-direct`. It is read in two steps, in
   this order: first each member invocation, decided as rule 12n decides a
   direct call (C04.19.3; with the run's facts a closed empty set invokes
   nothing, and an open set refuses `callback-member-open`); then the states,
   as C04.19.6 reads a direct call. A callback candidate of such a
   declaration stays missing.
2. **The destroyer's consumers read the composition** (RULE50-CONSUMER-COMPOSITION).
   Today rule 50's composition is body evidence that no consumer reads.
   A consumer in another unit cannot read the destroyer's body, so the
   composition has to be stated: a new clause names the helper that takes
   the stale path's object, `resource_member_helper(mth,
   mk_http_thread.parent, mk_thread_destroy)` (gateway rule 51). Rule 50's
   body reading checks it, and counts it as covered by evidence `50` only
   where the reading establishes the take through that helper. At a direct
   call of the destroyer, a consumer composes, after the destroyer's own
   states, the helper's member invocations (each decided over the run's set,
   as in item 1) and then the helper's states at the object the caller linked
   at the stale path. The consumer's row then consults `mk_thread_destroy`,
   so the run's fold (rule 12n) checks `mk_thread.cb_destroy` for that row.
   It keeps the row only when the run's set is closed, and the consumer's own
   call refuses unless the set is closed and empty. Every case short of that
   refuses: a helper that is missing, withdrawn or not a helper destroyer, an
   open or non-empty set, no run facts, or a helper clause on the helper
   itself (a deeper frame).
3. **The premise decision.** `scheduler_callback_footprint` comes off
   `mk_http_thread_destroy` (`qcore-provider-libevent`,
   `contracts/monkey/intrusive_list.qv`) only if the composition and the fold
   now carry what it stated for every row resting on the declaration. The
   declaration's text changes either way, so it is re-pinned at `046accf6`
   and `66910c10`, and the provider lock is resynced.

## Predicted moves

No status moves on the corpus route.

* **Consumers of `mk_http_thread_destroy`**: `sched_thread_cleanup` and
  `mk_sched_threads_destroy_conn` (direct calls), `mk_sched_threads_purge` and
  `mk_sched_threads_destroy_all` (through rule 43's summary of
  `sched_thread_cleanup`), all Safe, and `mk_sched_drop_connection`
  (Unknown). The run's facts read `mk_thread.cb_destroy` closed and empty
  (rule 50 qualified the helper over them at base: evidence `50`), so the
  composed invocation invokes nothing. No caller links a tracked object at
  `mth->parent`, so the helper's states decide nothing. Predicted content
  changes, with no status change:
  * `mk_thread_destroy` joins the rows' `declarations`;
  * `mk_thread.cb_destroy` joins their `run_set_members`;
  * if the premise is dropped, `scheduler_callback_footprint`'s atom leaves
    their `hypothesis_dependencies`.
* **`mk_http_thread_destroy`'s own row** stays Unknown. Its call
  `mk_thread_destroy(th)` stops refusing `member-path-direct`. It is
  composed instead, and is expected to trade that leaf for an
  unresolved-identity or release leaf on `th` (a copy of a formal's member).
  The row keeps its TLS and session-chain leaves.
* No Unknown → Safe, no Safe → Unknown, no Unsafe added or removed. Null
  verdicts are unchanged.

**Routes not measured.** The system and slice routes analyse no Monkey
scheduler unit. No row there consults `mk_http_thread_destroy` or
`mk_thread_destroy`, no Fluent Bit declaration is a helper destroyer, and
none states the new clause, so nothing there is predicted to move. Their
preparations do not exist on this host, and this card measures the corpus
route only.

If a pin cannot be written at a revision, the declaration is out of force
there, and the four Safe corpus rows above would fall to Unknown. That would
be a pinning failure, reported as one, not a semantic result.

```measure-prediction
route corpus
none
```
