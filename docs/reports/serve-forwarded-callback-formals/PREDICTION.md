# SERVE-FORWARDED-CALLBACK-FORMALS: frozen prediction

Frozen before any implementation code, on local `main` `2afce9973` (corpus
936 / 9 / 3,889 `ec4ccfeb…`, system 93 / 4 / 351, slice 52 / 1 / 347 / 4
Pending; null property corpus 1,113 / 1,435 / 2,355 / 18 Unsafe, system
84 / 125 / 245, slice 71 / 103 / 232; follow-ups 771; Lean 319).

## The four bodies, read at both revisions

`lib/librdkafka-2.15.0/src/rdkafka_queue.c` is byte-identical at `66910c10`
and `046accf6` (sha256 `a1d55367…` under both
`build/cycle19/clang/<revision>/fluent-bit-clang-source/`), so one reading
covers both.

| forwarder | lines | body | callee |
| --- | --- | --- | --- |
| `rd_kafka_q_pop_serve` | 507-515 | `return rd_kafka_q_pop_serve0(rkq, timeout_us, version, cb_type, callback, opaque, rd_false);` | `rd_kafka_q_pop_serve0` (static, 398-505) |
| `rd_kafka_q_pop_serve_maybe_consume` | 522-533 | the same with `rkq->rkq_rk->rk_type == RD_KAFKA_CONSUMER` last | `rd_kafka_q_pop_serve0` |
| `rd_kafka_q_serve` | 652-660 | `return rd_kafka_q_serve0(rkq, timeout_ms, max_cnt, cb_type, callback, opaque, rd_false);` | `rd_kafka_q_serve0` (558-649) |
| `rd_kafka_q_serve_maybe_consume` | 667-677 | the same with `rkq->rkq_rk->rk_type == RD_KAFKA_CONSUMER` last | `rd_kafka_q_serve0` |

Each names `callback` once, as the whole fifth actual of its one call, and
`cb_type` and `opaque` once each, as the whole fourth and sixth actuals; none
is reassigned, address-taken or stored. Each declaration (`client.qv`) states
`resource_callback_invoked(callback, _, _, supplied_owned, cb_type, opaque)`,
and so does each callee's at the same position (fifth parameter) with the
same actual list over its own `cb_type` and `opaque` (fourth and sixth). So
each forwarder is an exact forward in the brief's sense, clause for clause.

## Why the corpus does not move now

Rule 15w reads a forward only to a callee whose declaration is **in force in
the unit**. Both callees are defined in `rdkafka_queue.c`, and both are
withdrawn there by rule 13y today, independently of the forwarders:

* `rd_kafka_q_pop_serve0` hands `callback` to `rd_kafka_op_handle` (`:450-452`,
  sixth actual) and to itself (`:498-499`, the forward-queue recursion);
* `rd_kafka_q_serve0` hands `callback` to `rd_kafka_op_handle` (`:627-628`)
  and to itself (`:585-586`).

`rd_kafka_op_handle` (`rdkafka_op.c`) has no declaration, so a forward to it
never qualifies; and a forward of a declaration to itself is not read as
evidence for that declaration (the standing set is the least fixed point:
a declaration whose only support would be itself never stands). The two
callees therefore stay withdrawn, and with them the four forwarders.

**Prediction:** corpus, system and slice unchanged in status, every verdict
row byte-identical (no leaf changes either: the forwarders' rows keep
`callback-withdrawn` on the withdrawn callee, and `rd_kafka_q_pop` keeps
`callback-withdrawn` on the still-withdrawn `rd_kafka_q_pop_serve`); no
Safe → non-Safe; null property byte-identical on all three routes. Corpus
row movement **0** (the brief's bound is at most +4).

No other corpus declaration with a synchronous-invocation clause has a body
that hands its callback formal on unchanged: `rd_kafka_topic_partition_list_sort`
rebinds `cmp` (stays withdrawn); the cmetrics and ctraces array/map unpackers,
`bsearch` and `rd_list_*` validate by direct calls already.

## What would move once the parallel contracts land

Nothing through rule 15w alone in `rdkafka_queue.c`: the candidate contracts
(`OP-CALLBACK-CANDIDATE-CONTRACTS`, `OP-APP-RK-CALLBACKS-VIA-POLL-CB`) declare
op-callback candidates and application callbacks, not `rd_kafka_op_handle`,
and they do not change the self-forward. Even with the two callees standing,
the forwarders' rows would also need every member clause they repeat
(`rd_kafka_op_s.rko_serve`, `rd_kafka_op_s.rko_op_cb`) decided over the unit's
closed sets with contracted candidates, and the two `_maybe_consume` rows keep
their access-gate refusal at `rkq->rkq_rk->rk_type`; `rd_kafka_q_pop` passes a
null callback (`callback-undecided`). At most two rows
(`rd_kafka_q_serve`, `rd_kafka_q_pop_serve`) could move from this rule even
then. The blocker is registered as a follow-up with its size.

## Synthetic controls (the brief's five cases)

On a synthetic unit with trusted test declarations: an exact forward is
accepted (the forwarder's declaration stands, its caller decides over the
caller's own candidates, the forwarder's own row is `Safe`); a formal
reassigned before the forward, a forward to a different position, a forward
to an uncontracted callee, and a formal both forwarded and invoked locally
each keep the withdrawal (`callback-withdrawn` at every call) exactly as
today. Also: a forward to a callee withdrawn in the unit refuses, a chain of
two exact forwards to a declared callee stands, and a self-forward refuses.
