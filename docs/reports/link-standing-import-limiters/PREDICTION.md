# LINK-STANDING-IMPORT-LIMITERS: prediction

Committed before the first semantic commit, after the census
([README](README.md#census)). Nothing had been built from this card's changes
when it was written. Base: main `0f6e17118` (integrate/r12).

## What the card changes

1. **Stray probes** (rule 40 runs only). A home that only the run's member
   facts name, and whose name has no token at all in the caller's
   preprocessed unit, leaves the caller's scope before the budget: Clang
   cannot type a name the unit never declares, so condition (14) cannot hold.
   It is not held, not probed, not counted toward the budget and not listed as
   a caller refusal; the unit row counts it separately. A name the unit does
   declare or use is probed as before and still refuses when it cannot be
   typed.
2. **Replay budget** (rule 4c, every run with imports). A caller whose
   reachable homes exceed `REPLAY_BUDGET_PER_UNIT = 24` no longer holds none:
   it holds a deterministic subset of at most 24, whole per-function home sets
   first (smallest first, name tie-break), then the remaining reachable homes
   by name. Every withheld home is named on the unit row; its calls stay
   uncontracted. The bound and every per-home check are unchanged.

## Pinned routes

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The three measured routes run under the stated eight-home map
(`link-map.corpus.json`), never under a link-resolution artifact, so change 1
never applies. With eight homes no caller's reachable set exceeds 24, so
change 2 never applies either. No row, count or report byte is expected to
move.

## The user's `current` run (artifact + `--monkey-linked`)

Predicted from the census over the evidence run
(`private/current-linked-556460e31`, binary `89fa31d21`), whose reproduction
of every unit's scope matches the run's own progress lines exactly (186 of
186 reproduced units, 0 mismatches).

- **Stray refusals.** The 1,554 caller refusals "call type unavailable: Clang
  AST rejected (undeclared identifier)" for the 23 member-named homes leave
  `cross_unit_link_refusals`. The census counts 3,633 such probes over all
  186 units (the withheld units would have probed the rest).
- **Budget.** Of the 99 withheld units, 95 fit the budget once the strays
  leave the scope and hold their whole scope. Four stay above it and hold 24
  homes each, withholding:
  - `librdkafka/.../rdkafka.c` (27 reachable): `rd_kafka_topic_info_destroy_free`,
    `rd_kafka_topic_partition_destroy_free`,
    `rd_kafka_topic_partition_list_destroy_free`;
  - `librdkafka/.../rdkafka_admin.c` (31): `rd_kafka_consumer_group_state_name`,
    `rd_kafka_consumer_group_type_name`, `rd_kafka_topic_info_destroy_free`,
    `rd_kafka_topic_partition_destroy_free`,
    `rd_kafka_topic_partition_list_set_err`, `rd_map_str_cmp`, `rd_strcmp2`;
  - `monkey/.../mk_server.c` (27): `mk_string_copy_substr`, `mk_string_search`,
    `mk_string_search_n`;
  - `monkey/.../monkey.c` (26): `mk_string_search_n`, `mk_utils_worker_rename`.
- **Held homes before rule 40's per-caller conditions**: 1,545 in 138 units
  (base: 2,016 in 87 units, strays included). After those conditions (linked
  caller, type agreement) the held total rises from the base's 262.
- **Functions.** Unknown -> Safe is possible only for a function whose every
  diagnostic is a call to a home that becomes held. The census finds six:
  - `lz4frame.c` `ctxTypeID_to_size` (`LZ4_sizeofState`, `LZ4_sizeofStateHC`);
  - `rdkafka.c` `rd_kafka_share_consumer_closed_err` (`rd_kafka_error_code`,
    `rd_kafka_error_destroy`);
  - `rdkafka_ssl.c` `rd_kafka_transport_ssl_threadid_cb` (`thrd_current_id`);
  - `cio_chunk.c` `cio_chunk_hash` (`cio_file_hash`) and `cio_chunk_is_up`
    (`cio_file_is_up`);
  - `mk_lib.c` `mk_mq_send` (`mk_fifo_send`): base withholds every import of
    `mk_lib.c` by the budget (26 reachable, 22 of them strays).

  Each moves only if rule 40 admits the home for that caller and the replayed
  summary discharges the call; a diagnostic hidden behind the first one could
  keep any of them Unknown. Upper bound: these six. (`lz4.c`
  `LZ4_createStream` stays Unknown: its home `rd_kafka_mem_malloc` is refused
  as "call type differs", unchanged.) The 255 functions blocked by a home
  call plus another diagnostic may change diagnostics, not status.
- **No regression.** A unit within the budget at base keeps exactly its
  admitted homes (the strays it loses were refused anyway), so the six
  import-dependent Safe rows of the evidence run (`mk_http_cb_file_on_consume`,
  `net_plain_writev`, `pack_basic_header`, `cmt_labels_create`, `pack_id`,
  `rd_map_str_hash`) stay Safe. No Safe -> Unknown, no unit status change. A
  new Unsafe is not expected; any would be provisional.
- **Cost.** More callers replay homes, so the import phase grows; the replay
  of a home unit stays shared within the process. Measured in the report.
