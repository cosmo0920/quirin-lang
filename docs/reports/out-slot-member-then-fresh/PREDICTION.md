# OUT-SLOT-MEMBER-THEN-FRESH: prediction

Committed after the probes (`6aa742484`) and before any semantic change. Base
`287dce8ed` (main, r86). USER DECISION 2026-11-28: option (a), declare the
op's member custody: the member list is moved out of the op (and nulled)
before `rd_kafka_op_destroy`, so the out slot receives either the moved member
or the fresh list.

## What was known when this was written

* The custody claim holds in the bodies at Fluent Bit `046accf6`
  (`rdkafka_subscription.c:316-340`, `:344-368`): the member is read into
  `*partitions`/`*topics` and set to `NULL` before `rd_kafka_op_destroy`, whose
  `GET_ASSIGNMENT`/`GET_SUBSCRIPTION` arm (`rdkafka_op.c:380-390`) would
  otherwise free it with `rd_kafka_topic_partition_list_destroy`; the reply's
  member is a copy the cgrp handler stores (`rdkafka_cgrp.c:7519-7541`).
* The existing vocabulary cannot state it (the probe pins in
  `crates/qcore-c-gateway/tests/op_constructor_declarations.rs`): a result
  state needs `resource_return_owned(nonnull)` while `rd_kafka_op_req2` is
  `nullable` (`quant-live-unbacked`); a member of a result maps only to
  `#absent` or a function designator (`quant-live-value` for `#moved`,
  `#kept`, `#released`); C04.19.6's `#moved` hands the object to a holder
  that is never the caller; route G gives a member read out of an op no
  identity, even one the frame itself linked (`UnknownAlias`); and no
  return-code class can state the non-zero exits (unwritten, or the moved
  member), so no C04.13 disposition covers the function.
* So the card stops with options and lands only pins and records: no code,
  contract or rule change.

## Predicted movements (opt-in route, base vs tip)

* **None.** The tip changes one test file and reports; the measured binary is
  built from identical `crates/` and `providers/` sources. Both named rows stay
  `Unknown` with their base leaves (three indirect accesses, rule 74l's slot
  refusal, unresolved `rko`/`rkcg`/the slot).
* The system-slice route moves nothing for the same reason.

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
