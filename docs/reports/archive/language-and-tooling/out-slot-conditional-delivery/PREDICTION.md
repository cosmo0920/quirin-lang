# OUT-SLOT-CONDITIONAL-DELIVERY: prediction

Committed before the semantic commit. Base `43410b230` (main). USER DECISION
2026-11-25: "lanich four of queued cards", choosing the return-family and
this-lane set, which includes this card: build the keep-or-deliver mode with
Lean parity.

## What was known when this was written

From the probes at base ([evidence/probe-base.txt](evidence/probe-base.txt))
and from reading the named rows at Fluent Bit `046accf6`:

* `void fill(void **out) { if (!*out) *out = q_alloc(); }` publishes no
  summary (rule 74l: `out_mode` refuses the exit that keeps the slot), and a
  caller `void *p = 0; fill(&p); q_free(p);` is `Unknown` (`call to
  uncontracted function`).
* Each named row has other blockers on the base run (the source report's
  census): `rd_kafka_handle_OffsetFetch` 26 access leaves and nine
  uncontracted callees; `rd_kafka_broker_share_session_add_remove_toppar`
  `rd_list_add(*slot, …)` into the slot (a statement `f(*x, …)` the
  classifier reads as a declaration, and a container store into the caller's
  object); `rd_kafka_cgrp_consumer_assignment_with_metadata` the registry
  poison; `rd_kafka_assignment`/`rd_kafka_subscription` an op's member moved
  into the slot; `cio_file_native_lookup_user`/`_group` a second store over
  the first; `rd_kafka_handle_ApiVersion` the `rd_kafka_buf_read_*` macros.

## Planned change

A keep-or-deliver summary in a table of its own (never the function map),
validated by two runs of the body (null slot: W4's `Out`; a caller-held live
object in the slot: kept, live, at every exit), read by callers only at a
whole-statement call with the slot actual `&NAME`, lowered to a branch on
`NAME`'s nullness (null arm: `Out`; non-null arm: `Owned` on the object).

## Predicted movements (opt-in route, base vs tip)

* **No status move expected on the named rows**: none of the six bodies
  validates (each keeps another blocker), so none publishes a summary, and
  their callers keep `call to uncontracted function`.
* Elsewhere: zero to a few rows could move `Unknown -> Safe` (or `Unknown ->
  Unsafe` for a caller that ignores a delivery) if some other body in the
  tree is a clean keep-or-deliver shape with a caller that passes `&NAME` of
  a known-null or held pointer. Expected: none, since the shape is rare and
  usually followed by a use of `*out` as an argument.
* Leaf text: a caller of a published keep-or-deliver function with another
  actual shape would change `call to uncontracted function` to the named
  refusal; expected: none, for the reason above.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines); expected there: no status move.
