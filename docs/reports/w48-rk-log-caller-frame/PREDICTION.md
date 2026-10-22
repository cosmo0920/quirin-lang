# W48-RK-LOG-CALLER-FRAME: prediction

Committed before any contract, Rust or Lean change. Base: main `e282aaccc`
(contract cycle4 integrated). BASE is this card's own fresh corpus run of the
base binary; no cycle4 or other-card figure is borrowed.

## What will change

- A new librdkafka provider module declares `rd_kafka_log0` (and the static
  `rd_kafka_log_buf` it calls) **caller-frame**: every formal and the variadic
  tail borrowed (`resource_variadic_borrowed_tail()`), no result, nothing kept
  or released of the caller's. Each declaration names its premises:
  - `rk_plugin_callback_closure` (the existing P-RK-PLUGIN-CALLBACK-CLOSURE,
    for the `conf->log_cb` arm);
  - one new premise for the queue IO-event callback that `rd_kafka_q_io_event`
    may run synchronously during the enqueue
    (`rd_kafka_q_cb_event_enable` is reached only from the public
    `rd_kafka_queue_cb_event_enable`, which no analysed unit calls);
  - one new premise that the forward-queue reference the enqueue takes is not
    the last when it is released (sequentially provable; only a concurrent
    unforward-and-release can break it).
- Expected outcome of the challenge: the enqueue effect is **not** provable
  Safe outright; callers become **conditionally Safe** under exactly those
  premises. Proved without a premise: the filtered, `rk == NULL`,
  `!log_queue`, `!rk->rk_logq` and callback-absent paths, the disabled-queue
  reply/destroy of the fresh op (its reply queue is empty, its LOG cleanup
  frees only its own fresh string), the copied serve callback (never invoked
  by the enqueue), and the balanced forward keep/drop in sequential execution.
- Rust: a checked caller-frame projection (rule 32) of the existing
  `logger-effects-v1` model: every one of its 15 obligations is classified
  internal (with its argument) or caller-visible (with the premise covering
  it); every path proved, premised, infeasible or outside the frame; the
  production declaration must state exactly the derived premise set. The 15
  internal obligations stay Open.
- Lean: `QCore.RkLogCallerFrame` (caller-frame admission, sequential enqueue
  discharge, premise-set structure), registered in an `Audit*.lean`.
- The body check stays the existing route: `rd_kafka_log0`'s body is analysed
  against its declaration (W4/entry); `rd_kafka_log_buf`'s body keeps its own
  leaves and its declaration stays a declared assumption (not refuted: its
  body does not validate).

## Predicted corpus movement (pinned arbiter, 66910c10)

- Unknown -> Safe: the planner's three freed callers
  (`rd_kafka_mock_cgrp_classic_member_active`,
  `rd_kafka_mock_cgrp_consumer_member_active`,
  `rd_kafka_mock_sharegroup_member_active`), each conditionally Safe on the
  three premises above; plus `rd_kafka_log0` itself, whose own body's only
  leaf is the `rd_kafka_log_buf` call (now read through its declaration).
  Predicted total: **4 Safe gains** (range 3-8 if a further caller's only
  other blocker is one of these).
- `rd_kafka_log_buf` stays Unknown (its own leaves are its body's).
- No Safe regression; no new Unsafe; no Pending added.
- Most of the 35 Unknown callers naming `rd_kafka_log0` as an uncontracted
  callee keep other leaves: nested inner calls in the variadic tail
  (`rd_kafka_err2str(...)`, CG05-UNCONTRACTED-INNER-CALLEES), the
  `&rk->rk_conf` place gate (CG10 / CG05-CALL-ROOTED-PLACES), value-connective
  edges (CG06). Leaves naming `rd_kafka_log0` as uncontracted go to zero; the
  CG06 and W47 populations shrink only where `rd_kafka_log0` being uncontracted
  was the reason, measured, not assumed.
- System and tail-pause slice: no librdkafka unit is selected, so no row moves.
