# LIBRDKAFKA-HEADER-INLINES prediction

Committed after the census (`c7529d30a`) and before any contract, view, lock
or test changes. Base side: `baseline:c8c7edf21` (the main checkout's
`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`; no baseline is
published for this card's base `945c0db1b`): `eight-home` (corpus,
66910c10), `fluent-bit-system` (system) and `fluent-bit-tail-pause` (slice).
`945c0db1b` differs from `c8c7edf21` by records,
CONTRACT-ADOPTION-NOT-IN-FORCE (no code or contract change) and
CELL-RESULT-SET-WRITTEN-SET-DERIVATION (measured: no status move on any
route; one slice content change), so the base's rows are the baseline's.

**What changes.** One new resource-v1 module,
`librdkafka/header_inlines.qv` in `qcore-provider-librdkafka`, with 59
declarations of librdkafka `static` header definitions stated from their
bodies (the request-buffer writers and updaters, `rd_kafkap_str_new` /
`_bytes_new`, `rd_kafka_set_last_error`, `rd_kafka_q_enq` / `_reenq` and five
other queue helpers, seven message-queue helpers, accessors, utilities and
the partition-leader pair); twelve name `rk_plugin_callback_closure`. It joins
the crate's resource view and the route manifest; locks are regenerated.
No analyzer code changes.

**Rows that can move.** At the base, 260 corpus rows have an uncontracted
leaf on one of the 59 names (952 leaves). Six of them have no other
diagnostic: every leaf is a call to a declared name. Those six are predicted
Unknown -> Safe (the twelve premised declarations change no mode, C04.18; a
row resting on one is published with the premise):

- `rd_slice_crc32` (`rdbuf.c`): `rd_crc32_init`, `rd_crc32_update`;
- `rd_kafka_share_fetch_fanout_enqueue` (`rdkafka.c`): `rd_kafka_q_enq` of
  its formal `rko` (the least certain: the consumption of a formal);
- `rd_kafka_share_record_poll_start` (`rdkafka.c`): `rd_atomic64_set`;
- `rd_kafka_buf_upgrade_flexver_request` (`rdkafka_buf.c`):
  `rd_kafka_buf_write_i8`;
- `rd_kafka_queue_yield` (`rdkafka_queue.c`): `rd_kafka_q_yield` of a member
  read;
- `rd_kafka_toppar_needs_query` (`rdkafka_topic.c`):
  `rd_kafka_broker_get_state`.

The other 254 rows keep other diagnostics and stay Unknown; their
uncontracted leaves on the 59 names leave, and a declaration's own
refusals (a consumed or borrowed-result actual the body cannot name) may take
the place of some. A body whose only blockers were these calls may now
validate under W4 and change its callers; none is predicted. System and slice
select no librdkafka unit: no row moves there.

```measure-prediction
route corpus
move librdkafka::rdbuf.c::rd_slice_crc32 Unknown -> Safe
move librdkafka::rdkafka.c::rd_kafka_share_fetch_fanout_enqueue Unknown -> Safe
move librdkafka::rdkafka.c::rd_kafka_share_record_poll_start Unknown -> Safe
move librdkafka::rdkafka_buf.c::rd_kafka_buf_upgrade_flexver_request Unknown -> Safe
move librdkafka::rdkafka_queue.c::rd_kafka_queue_yield Unknown -> Safe
move librdkafka::rdkafka_topic.c::rd_kafka_toppar_needs_query Unknown -> Safe
delta Safe +6
route system
none
route slice
none
```

**The user's `current` run** (Fluent Bit `046accf6`, profile
`contract/profiles/fluent-bit`), rerun by the integrator after landing:

- **Callees leaving the uncontracted list:** the 59 declared names, all
  called in the run, 524 (function, callee) pairs; 2,790 -> 2,731 callees.
  The other 30 in-scope header callees (105 pairs) stay, each with the
  reason the report records.
- **Functions leaving the "only uncontracted" set** (508 rows): 12 of them
  call a declared name; 6 call nothing else uncontracted and are predicted
  Safe (`rd_slice_crc32`, `rd_kafka_share_fetch_fanout_enqueue`,
  `rd_kafka_share_record_poll_start`, `rd_kafka_buf_upgrade_flexver_request`,
  `rd_kafka_queue_yield`, `rd_kafka_toppar_needs_query`): 508 -> 502. The
  other 6 keep an undeclared callee.
- **Unit status:** no unit changes (8 Safe, 7 provisional Unsafe, 167
  Unknown, 4 data-only): no unit's remaining diagnostics are all calls to the
  59 names.
