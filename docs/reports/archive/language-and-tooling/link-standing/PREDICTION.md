# LINK-STANDING prediction

Committed before any semantic change, against base `a0ac2cae9`
(FLUENT-BIT-LINK-PREPARE's tip on origin/main `1e522ca5d`). The
measurement's base side is
`baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`.

Rule 40 lets the `current` route adopt a derived link resolution, read from
a supplied link-resolution artifact (`quirin-link-resolution/1`), as rule
4c's resolution. It does so only when the artifact is supplied
(`QCORE_CROSS_UNIT_LINK_RESOLUTION`), and only under checked conditions:

- the database digest matches and the bound digests verify;
- the caller's configuration is linked;
- the home is one strong global definition in the executable;
- the home's route key is unambiguous;
- the call type equals the home definition's type.

The corpus, system and slice routes are pinned routes. They run the
eight-home stated map (`QCORE_CROSS_UNIT_LINK_MAP`) and no artifact. Rule 40
does not apply to them, and a pinned route that is given an artifact refuses
to run. Rule 4c's selection, scope, budget and replay are unchanged when no
artifact is supplied, so no verdict or diagnostic leaf is predicted to move:

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## The user's `current` run

### Without the artifact

The run is unchanged byte for byte (Fluent Bit 046accf6, no link map, no
artifact). No import route runs and no callee leaves the uncontracted list.

### With the artifact

This is the run the integrator makes: the artifact of FLUENT-BIT-LINK-PREPARE's
`run-1-second`, whose database equals the run's. The upper reference is
CURRENT-RUN-WORKER-SPIN's 165-home experiment run
(`docs/reports/current-run-worker-spin/acceptance-rows.json`: 291 changed rows,
16 Unknown -> Safe). That run is restricted here to rows that keep at least
one resolved import leaf whose callee is among the artifact's 1,384 derived
names, in a caller outside `monkey-libevent`:

| Before -> after | Experiment | Predicted with the artifact |
|---|---:|---:|
| Unknown -> Safe | 16 | **at most 14** |
| Unknown -> Unknown (an import leaf gone, deeper leaves may appear) | 273 | at most 246 |
| Unsafe -> Unsafe (an import leaf gone) | 2 | at most 1 |
| uncontracted-import leaves resolved | 411 | at most 357 |

The 14 predicted Unknown -> Safe rows, each blocked only by derived homes in
the experiment:

| Unit | Function | Imports that become resolved |
|---|---|---|
| chunkio `cio_chunk.c` | `cio_chunk_get_content_size` | `cio_error_reset` |
| chunkio `cio_chunk.c` | `cio_chunk_get_real_size` | `cio_error_reset`, `cio_file_real_size` |
| chunkio `cio_chunk.c` | `cio_chunk_hash` | `cio_file_hash` |
| chunkio `cio_chunk.c` | `cio_chunk_is_up` | `cio_file_is_up` |
| chunkio `cio_chunk.c` | `cio_chunk_tx_begin` | `cio_error_reset` |
| chunkio `cio_chunk.c` | `cio_chunk_tx_rollback` | `cio_error_reset` |
| chunkio `cio_chunk.c` | `cio_chunk_unlock` | `cio_error_reset` |
| cmetrics `cmt_encode_cloudwatch_emf.c` | `pack_basic_header` | `cmt_labels_count` |
| ctraces `ctr_encode_msgpack.c` | `pack_id` | `ctr_id_to_lower_base16` |
| librdkafka `lz4frame.c` | `ctxTypeID_to_size` | `LZ4_sizeofState`, `LZ4_sizeofStateHC` |
| librdkafka `rdkafka.c` | `rd_kafka_share_consumer_closed_err` | `rd_kafka_error_code`, `rd_kafka_error_destroy` |
| librdkafka `rdkafka_fetcher.c` | `rd_kafka_op_offset_cmp` | `rd_kafka_op_get_offset` |
| librdkafka `rdkafka_ssl.c` | `rd_kafka_transport_ssl_threadid_cb` | `thrd_current_id` |
| librdkafka `rdmap.c` | `rd_map_str_hash` | `rd_string_hash` |

This includes 5 of the 7 cross-unit sole-blocked functions of
BODY-SUMMARY-REUSE's census: `ctxTypeID_to_size`, `cio_chunk_hash`,
`cio_chunk_is_up`, `rd_kafka_share_consumer_closed_err` and
`rd_kafka_transport_ssl_threadid_cb`.

**Monkey stays refused on this base.** `mk_mq_send` (`mk_fifo_send`) and
`net_plain_writev` (`mk_iov_send`) stay Unknown, as do every row in a
`monkey-libevent` caller (30 of the experiment's 291 rows). Rule 40 has no
Monkey case. On this base, the route analyses Monkey from its separate
libevent database, and that configuration is not among the artifact's linked
configurations. So two general conditions refuse it:

- a Monkey caller is withheld because its configuration is not linked;
- a Monkey home is refused because its configuration is not a linked
  configuration of the artifact.

The 24 Monkey callees (55 sites) become derivable, with no change to rule
40, once CURRENT-MONKEY-LINKED-CONFIGURATION makes the route analyse
`lib/monkey` under Fluent Bit's own, linked configuration.

The prediction is an upper bound for three reasons:

- **Call type.** A home is held only where Clang's type of the callee in the
  caller unit equals its type in the home unit, after desugaring. Any record
  named in the type must also have equal canonical layouts where both units
  complete it. Every derived census callee is declared by a header the caller
  includes, so no refusal is expected among the 14. An unequal or unavailable
  type keeps the call uncontracted.
- **Replay budget.** The experiment named 165 homes, and its busiest callers
  reached exactly 24, the budget. The artifact derives 1,384 names. Only those
  with a Safe row and a validated W4 summary export a home, but that can still
  be more homes than the experiment's 143 exports. A caller that may reach
  more than 24 holds none, unchanged from rule 4c, and its unit row names the
  exhausted budget. This can withhold callers that the experiment fed.
- **Deeper leaves.** A resolved import can expose deeper leaves. The
  experiment exposed 174 of them in 105 rows.

No unit status is predicted to change: every affected unit has other Unknown
rows.

The assumptions change. When the import phase runs, the
`cross_unit_link_resolution` assumption names the artifact as derived
evidence (its SHA-256, the database, executable and map digests, and each
imported home's definition record). It names no stated premise. Each
caller's unit row lists the rule 40 refusals of the homes it could reach
(`cross_unit_link_refusals`).
