# W46-ACCESS-PATH-CENSUS: frozen prediction for P2 and P3

Frozen 2026-09-27 on base `9bce3986`, before any P2 or P3 implementation. P2
(ACCESS-PATH-STATEMENTS) and P3 (ACCESS-PATH-POSITIONS) of the
[unsupported-coverage plan](../../plans/unsupported-coverage/README.md) are
measured against this file. It predicts **verdicts, not leaves**. Later
corrections go beside it in those cards' reports, never into it.

**Evidence.** The per-function lists are in
[artifacts/prediction.json](artifacts/prediction.json). It has a schema line,
then `summary` and `freed.P2` / `freed.P3`, each entry with its verdict,
reason, obligations and closure round. The per-leaf census is
[artifacts/census-leaves.jsonl.gz](artifacts/census-leaves.jsonl.gz). The
rules are implemented in [scripts/predict.py](scripts/predict.py), over the
census of [README §2](README.md#2-mechanism).

## 1. What each scope removes

| Scope | Removes a leaf group when |
|---|---|
| **P2** statements | the family is `IND`; every construct at the leaf's line is a statement or `for` clause; every Clang access path in it is expressible; and no path sits in a call argument |
| **P3** statements, conditions, returns, call arguments | the family is `IND`, `COND` or `RET` and every path is expressible, in any position |

Three further conditions apply to P3 groups:
- **Conditions.** A `COND` group whose condition-desugaring census code is not
  about an access or a call stays refused. Those codes are a type operator, an
  assignment, braces, a custody result, a recursive local callee, and so on.
- **Empty conditions.** A `COND` group with neither a path nor a call stays
  refused.
- **Returns.** A `RET` group with no path stays refused: its refusal is a call,
  an aggregate or its form.

**Calls remain.** Once the access is modelled, the construct is a call:
- It is served by a declared contract, or by a Safe callee the same unit
  defines (the existing summary rule).
- A header-inline body the corpus never names uncontracted in that unit is
  also served.
- Anything else blocks: an external callee, an uncontracted in-corpus callee
  that is not Safe, a callee in another unit, or an indirect call.

In P3, a pointer or address actual passed to a callee whose contract takes
custody (released, consumed, stored, published, out) blocks, as
`W44-CALL-ARGUMENT-CUSTODY-POSITION` records.

**Freed.** A function is *freed* when it is Unknown on the baseline, has no
`unknown_alias`, `leak`, `summary_conflict` or other unsupported leaf, the
scope removes every `IND`/`COND`/`RET` group it has, and every residual call is
served.

## 2. Obligations and the predicted verdict of a freed function

For every freed function:

- **Every pointer a path reads through** (each `->`, `*`, and `[i]` over a
  pointer) is owed non-null and live.
  - **Untracked root.** A root the analyzer never binds to a resource identity
    discharges by scope: its `may_bind` is false in the analyzer's own census
    streams. Every admitted route's scope string says why: "never the liveness
    or nullness of an object the projection does not track".
  - **Pointers loaded from memory** (`a->b` then `->c`) discharge by scope the
    same way. So does a file-scope root.
  - **Tracked root.** A root with `may_bind` true, or never observed
    (fail-closed), needs one known non-null live identity. It is predicted
    discharged only when a controlling expression at an earlier line
    null-tests it after its last assignment. Otherwise the prediction is
    **Unknown**, with the expected `unresolved resource identity` exchange at
    that root. That is W44-INDIRECT-CHAIN-ACCESS's precedent, where 50 of 50
    `flow` leaves were exchanged.
- **A non-pointer store** leaves the custody frame unchanged. A **null store**
  of a pointer member is a non-custody store.
- **A pointer read through a path** is a borrowed alias with no custody.
- **Provisional Unsafe** would be predicted for a tracked root passed to a
  releasing call earlier in the same block with no assignment between (a
  use-after-release candidate). The census finds **none**.

## 3. The prediction

The table uses the **existing summary rule** (`same-unit`). The `cross-unit`
column counts the extra functions freed only if a Safe callee in another unit
served its callers. No rule does that today (`W44-CALLEE-CROSS-UNIT-SUMMARY`).

| Scope | Freed | Safe | Unknown (reason) | provisional Unsafe | Closure rounds | + cross-unit |
|---|---:|---:|---:|---:|---|---:|
| P2 | **45** | **38** | 7 (unguarded tracked root) | 0 | 1 (round 1 adds 0) | +3 (3 Safe) |
| P3 | **220** | **83** | 137 (unguarded tracked root; 30 name a root whose `may_bind` no stream observed, 28 only such roots) | 0 | 2 (round 1 adds 1 Unknown) | +8 (3 Safe, 5 Unknown) |

P3 includes P2.

**By library** (same-unit; Safe / Unknown):

| Library | P2 | P3 |
|---|---|---|
| chunkio | 1 / 1 | 1 / 4 |
| cmetrics | 3 / 0 | 5 / 5 |
| ctraces | 1 / 0 | 2 / 1 |
| fluent-bit-chunk | 2 / 0 | 2 / 0 |
| librdkafka | 30 / 4 | 70 / 114 |
| monkey-libevent | 1 / 2 | 3 / 13 |
| **total** | **38 / 7** | **83 / 137** |

### Arms of the Safe predictions

These separate what P2 and P3 must explain on a miss.

**P2, 38 Safe:**
- 22 have no pointer obligation at all.
- 16 have only scope-discharged hops.
- 0 rest on a null-tested tracked root.
- 18 of the 38 carry **no typed access at all**. Their only leaves are the
  lexical gate reading a non-access: a declarator `*`, a `sizeof` operand, a
  multiplication, a cast, or a bracketed array declarator. They move only if P2
  replaces the lexical gate's access detection with the typed answer and the
  ordinary classifier then admits the statement.

**P3, 83 Safe:**
- 28 have no pointer obligation.
- 54 have only scope-discharged hops.
- 1 rests on a null-tested tracked root, the only guard-dependent Safe.

### Leaf-level upper bounds

Leaves are not the prediction, but these bound what the scopes can remove:
- **P2** removes at most **4,423** `IND` leaves: the expressible statement
  groups.
- **P3** removes at most **12,902** `IND`, **7,983** `COND` and **170** `RET`
  leaves.

Removed call-bearing statements are exchanged for `call to uncontracted
function` leaves where their callee is unserved.

## 4. The P2 list (45 + 3 cross-unit)

Obligations use these labels:
- `root-untracked:scope`: a root the analyzer never binds, discharged by scope.
- `loaded-pointer:scope`: a hop through a pointer read from memory, discharged
  by scope.
- `root-tracked-unguarded`: a tracked root with no dominating null test.
- `no-pointer-hop`: a `.`, or a subscript of an array.
- `none`: no typed access.

| Library | File | Function | Predicted | Obligations |
|---|---|---|---|---|
| chunkio | `chunkio.c` | `cio_options_init` | Unknown | root-tracked-unguarded 9 |
| chunkio | `chunkio.c` | `cio_set_log_level` | Safe | root-untracked:scope 1 |
| chunkio | `cio_file.c` | `cio_file_real_size` | Safe (cross-unit) | none |
| cmetrics | `cmetrics.c` | `cmt_initialize` | Safe (cross-unit) | none |
| cmetrics | `cmt_encode_opentelemetry.c` | `initialize_exponential_histogram_data_point_list` | Safe | none |
| cmetrics | `cmt_encode_opentelemetry.c` | `initialize_histogram_data_point_list` | Safe | none |
| cmetrics | `cmt_encode_opentelemetry.c` | `initialize_metric_list` | Safe | none |
| ctraces | `ctr_encode_opentelemetry.c` | `otel_span_set_kind` | Safe | root-untracked:scope 6 |
| fluent-bit-chunk | `flb_input_chunk.c` | `flb_input_chunk_get_name` | Safe | root-untracked:scope 1 |
| fluent-bit-chunk | `flb_input_chunk.c` | `flb_input_chunk_is_storage_overlimit` | Safe | root-untracked:scope 1 |
| librdkafka | `lz4frame.c` | `LZ4F_resetDecompressionContext` | Safe | root-untracked:scope 1 |
| librdkafka | `lz4frame.c` | `ctxTypeID_to_size` | Safe (cross-unit) | none |
| librdkafka | `rdbuf.c` | `rd_slice_widen` | Safe | root-untracked:scope 2 |
| librdkafka | `rdhdrhistogram.c` | `rd_hdr_countsIndex` | Safe | root-untracked:scope 2 |
| librdkafka | `rdkafka.c` | `rd_kafka_set_log_level` | Safe | root-untracked:scope 1 |
| librdkafka | `rdkafka_admin.c` | `rd_kafka_ConfigResource_cmp` | Unknown | root-tracked-unguarded 4 |
| librdkafka | `rdkafka_admin.c` | `rd_kafka_ConsumerGroupListing_cmp_group_id` | Safe | none |
| librdkafka | `rdkafka_admin.c` | `rd_kafka_DeleteGroup_cmp` | Safe | none |
| librdkafka | `rdkafka_admin.c` | `rd_kafka_DeleteTopic_cmp` | Safe | none |
| librdkafka | `rdkafka_admin.c` | `rd_kafka_NewPartitions_cmp` | Safe | none |
| librdkafka | `rdkafka_admin.c` | `rd_kafka_NewTopic_cmp` | Safe | none |
| librdkafka | `rdkafka_broker.c` | `rd_kafka_broker_cmp_by_id` | Safe | none |
| librdkafka | `rdkafka_cgrp.c` | `rd_kafka_cgrp_clear_last_err` | Safe | root-untracked:scope 2 |
| librdkafka | `rdkafka_conf.c` | `rd_kafka_sw_str_sanitize_inplace` | Safe | none |
| librdkafka | `rdkafka_metadata_cache.c` | `rd_kafka_metadata_partition_id_cmp` | Safe | none |
| librdkafka | `rdkafka_mock_handlers.c` | `rd_kafka_mock_msgset_est_record_size` | Unknown | root-tracked-unguarded 4 |
| librdkafka | `rdkafka_msgset_reader.c` | `rd_kafka_aborted_txn_cmp_by_pid` | Safe | none |
| librdkafka | `rdkafka_msgset_reader.c` | `rd_kafka_offset_cmp` | Safe | none |
| librdkafka | `rdkafka_request.c` | `rd_kafkap_leader_discovery_metadata_init` | Unknown | root-tracked-unguarded 3 |
| librdkafka | `rdkafka_request.c` | `rd_kafkap_leader_discovery_set_CurrentLeader` | Unknown | loaded-pointer:scope 4, root-tracked-unguarded 5, root-untracked:scope 2 |
| librdkafka | `rdkafka_request.c` | `rd_kafkap_leader_discovery_tmpabuf_add_alloc_topic` | Safe | root-untracked:scope 3 |
| librdkafka | `rdkafka_request.c` | `rd_kafkap_leader_discovery_tmpabuf_add_alloc_topics` | Safe | root-untracked:scope 2 |
| librdkafka | `rdkafka_sticky_assignor.c` | `ConsumerGenerationPair_cmp_generation` | Safe | none |
| librdkafka | `rdkafka_sticky_assignor.c` | `ut_testConflictingPreviousAssignments` | Safe | none |
| librdkafka | `rdkafka_telemetry_decode.c` | `clear_unit_test_data` | Safe | no-pointer-hop 14 |
| librdkafka | `rdkafka_telemetry_decode.c` | `unit_test_telemetry_decoded_int64` | Safe | no-pointer-hop 1 |
| librdkafka | `rdkafka_telemetry_decode.c` | `unit_test_telemetry_set_connects` | Safe | root-untracked:scope 1 |
| librdkafka | `rdkafka_telemetry_decode.c` | `unit_test_telemetry_set_connects2` | Safe | root-untracked:scope 1 |
| librdkafka | `rdkafka_telemetry_decode.c` | `unit_test_telemetry_set_coordinator_assigned_partitions` | Safe | loaded-pointer:scope 1, root-untracked:scope 1 |
| librdkafka | `rdkafka_timer.c` | `rd_kafka_timer_cmp` | Safe | none |
| librdkafka | `rdmap.c` | `mykey_cmp` | Safe | none |
| librdkafka | `rdmap.c` | `person_cmp` | Safe | none |
| librdkafka | `rdunittest_fetcher.c` | `ut_set_types` | Safe | loaded-pointer:scope 1, root-untracked:scope 2 |
| librdkafka | `rdxxhash.c` | `XXH32_reset` | Safe | no-pointer-hop 3 |
| librdkafka | `rdxxhash.c` | `XXH64_reset` | Safe | no-pointer-hop 3 |
| monkey-libevent | `mk_server/mk_fifo.c` | `fifo_drop_msg` | Unknown | root-tracked-unguarded 2 |
| monkey-libevent | `mk_server/mk_lib.c` | `mk_http_status` | Safe | root-untracked:scope 1 |
| monkey-libevent | `mk_server/mk_scheduler.c` | `mk_sched_broadcast_signal` | Unknown | loaded-pointer:scope 1, root-tracked-unguarded 1 |

## 5. The P3 Safe list (83 + 3 cross-unit, marked †)

The 137 P3 Unknown predictions, each with its unguarded root and line, are in
`prediction.json`.

`cio_set_log_level`, `cio_file_real_size`†, `cmt_initialize`†, `compute_flat_scope_index`, `initialize_exponential_histogram_data_point_list`, `initialize_histogram_data_point_list`, `initialize_metric_list`, `check_buckets`, `otel_span_set_kind`, `ctr_id_get_buf`, `flb_input_chunk_get_name`, `flb_input_chunk_is_storage_overlimit`, `cJSON_GetErrorPtr`, `cJSON_SetNumberHelper`, `parse_hex4`, `update_offset`, `LZ4F_resetDecompressionContext`, `ctxTypeID_to_size`†, `extra_alloc`, `rd_slice_widen`, `rd_hdr_countsIndex`, `rd_kafka_consumer_group_state_name`, `rd_kafka_consumer_group_type_name`, `rd_kafka_messages_get`, `rd_kafka_name`, `rd_kafka_opaque`, `rd_kafka_set_log_level`, `rd_kafka_ConfigEntry_name`, `rd_kafka_ConfigEntry_value`, `rd_kafka_ConsumerGroupListing_cmp_group_id`, `rd_kafka_DeleteGroup_cmp`, `rd_kafka_DeleteTopic_cmp`, `rd_kafka_NewPartitions_cmp`, `rd_kafka_NewTopic_cmp`, `rd_kafka_Node_host`, `rd_kafka_Node_rack`, `rd_kafka_broker_cmp_by_id`, `rd_kafka_mk_brokername`, `rd_kafka_cgrp_clear_last_err`, `rd_kafka_cgrp_rebalance_protocol`, `rd_kafka_conf_get_default_topic_conf`, `rd_kafka_sw_str_sanitize_inplace`, `rd_kafka_event_config_string`, `rd_kafka_event_message_count`, `rd_kafka_metadata_partition_id_cmp`, `rd_kafka_handle_mock_cluster`, `rd_kafka_mock_cluster_bootstraps`, `rd_kafka_mock_cluster_handle`, `rd_kafka_mock_cgrp_classic_check_state`, `rd_kafka_aborted_txn_cmp_by_pid`, `rd_kafka_offset_cmp`, `rd_kafka_op_get_offset`, `rd_kafka_topic_partition_get_current_leader_epoch`, `rd_kafka_topic_partition_get_leader_epoch`, `rd_kafka_topic_partition_list_count_abs_offsets`, `rd_kafka_topic_partition_list_get_err`, `rd_kafka_topic_partition_list_reset_offsets`, `rd_kafka_topic_partition_list_set_err`, `rd_kafkap_leader_discovery_tmpabuf_add_alloc_topic`, `rd_kafkap_leader_discovery_tmpabuf_add_alloc_topics`, `rd_kafka_sasl_cyrus_ready`, `rd_kafka_share_partition_offsets_list_get`, `rd_kafka_share_partition_offsets_offsets`, `ConsumerGenerationPair_cmp_generation`, `ut_get_consumer_rack`, `ut_testConflictingPreviousAssignments`, `clear_unit_test_data`, `unit_test_telemetry_decoded_int64`, `unit_test_telemetry_set_connects`, `unit_test_telemetry_set_connects2`, `unit_test_telemetry_set_coordinator_assigned_partitions`, `get_client_rack`, `get_group_id`, `get_group_instance_id`, `get_transactional_id`, `rd_kafka_timer_cmp`, `rd_list_first`, `rd_list_last`, `mykey_cmp`, `person_cmp`, `ut_set_types`, `XXH32_reset`, `XXH64_reset`, `mk_ptr_print`, `_next_buffer`, `mk_http_status`.

## 6. How P2 and P3 are scored against this file

1. **Safe predictions.**
   - A predicted-Safe function that stays Unknown is a **miss**, attributed to
     its first blocking leaf after the card. The no-typed-access arm (18 in P2)
     is scored separately.
   - A predicted-Safe function that becomes provisional Unsafe is a **miss**,
     classified by reading.
2. **Unknown predictions.**
   - One that moves to Safe is an **unforecast move**. It must name the
     dominating fact the census's lexical guard missed.
   - One that stays Unknown is a hit only when its new leaf is the predicted
     `unresolved resource identity` at the named root, or a leaf the census's
     reason covers.
3. **Functions outside the lists.** Any Unknown → Safe move outside the lists
   is unforecast and must be explained, for example by a callee the card made
   Safe in the same unit.
4. **Safe to non-Safe.** Any Safe → non-Safe move is a named correction, never
   an expected outcome.
5. **Provisional Unsafe.** The prediction is **0**. Every added `leak`,
   `use_after_release`, `invalid_release` or `double_release` leaf is
   classified by reading: real, artifact or premise gap. An artifact blocks the
   card (common brief).
