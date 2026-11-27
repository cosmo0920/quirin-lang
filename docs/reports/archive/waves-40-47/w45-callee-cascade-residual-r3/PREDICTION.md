# W45-CALLEE-CASCADE-RESIDUAL round 3: prediction

Frozen before any contract is written. Base: local `main` at `ef6b32192`.

## BEFORE (reproduced)

Binary `private/w45r3/bin/before`, sha256 `551fa31b…` (equal to the corpus
`run.json` `binary_sha256`).

| route | totals | `resource-diagnostics.json` |
| --- | --- | --- |
| corpus (`66910c10`) | 915 / 9 / 3,910 | `ded14632…` (the brief's) |
| system | 91 / 4 / 353 | `f8aab120…` (embeds this worktree's paths) |
| tail-pause slice | 50 / 1 / 349, 4 Pending | `1654a682…` (same) |
| null property (corpus) | 1,113 Safe unconditional / 1,435 conditional / 2,355 Unknown / 18 Unsafe; 29 findings | |

Re-rank (`scripts/rank.py`, CG22's lineage: a function counts only when
every leaf reads ``call to uncontracted function `F` ``): corpus **506**
functions, **352** decided by one callee alone (CG22 recorded 501 / 348 on
its AFTER; the null cards since moved five rows into the population, among
them the two `cfl_list_size` callers); system 27 / 19; slice 42 / 28.
Artifacts: `artifacts/rank-before-{corpus,system,slice}.json`.

## The >=2 tier (34 corpus callees, 2 slice callees), by reading

Every body read at both revisions with `scripts/body.py`; all named here are
byte-identical at `66910c10` and `046accf6`.

### Declared (predicted movers)

| stage | callee | crate / module | clauses | predicted movers |
| --- | --- | --- | --- | --- |
| a | `rd_kafka_AclBinding_destroy` | librdkafka `client.qv` | released nonnull | `rd_kafka_AclBinding_free` |
| a | `rd_kafka_ConfigResource_destroy` | same | released nonnull | `rd_kafka_ConfigResource_free` |
| a | `rd_kafka_NewPartitions_destroy` | same | released nonnull | `rd_kafka_NewPartitions_free` |
| a | `rd_kafka_NewTopic_destroy` | same | released nonnull | `rd_kafka_NewTopic_free` |
| a | `rd_kafka_UserScramCredentialAlteration_destroy` | same | released nullable (it tests NULL) | `..._destroy_free` |
| b | `mk_fifo_queue_destroy` | libevent `monkey/memory.qv` | ctx borrowed, q released nonnull | 0 to 1 (`mk_fifo_queue_id_destroy` releases `mk_fifo_queue_get`'s result; `_destroy_all` an `mk_list_entry` element: both likely refused as unresolved) |
| b | `print` (cJSON's static) | librdkafka `cjson.qv` | item, format, hooks borrowed; owned nullable result, the `cjson_allocator_hooks` premise | `cJSON_Print`, `cJSON_PrintUnformatted` |
| c | `cfl_list_size` | cfl `sds_lifetime.qv` | head borrowed, return none | `count_metrics`, `is_metric_empty` |
| c | `unit_test_telemetry` | librdkafka `client.qv` | scalars and strings borrowed; `resource_callback_invoked(set_metric_value, _, _)` | `unit_test_telemetry_gauge`, `_sum` (all 18 setters are Safe in-unit: identity candidates) |
| c (slice) | `flb_hash_table_del` | fluent-bit `hash_table_publication.qv` | ht, key borrowed, return none | slice: `flb_tail_scan_unregister_aged_out_inode`, `_ignored_file_size` |

Stage (a) correction to the brief's "about 10": the five `_destroy_array`
loops hand an element read (`x[i]`) to the non-null release, which the access
gate refuses (precedent on this base: `rd_kafka_AlterConsumerGroupOffsets_destroy_array`
is Unknown with `indirect memory access is outside the resource projection`
while its `_free` is Safe). Predicted: the five `_free` wrappers move, the five
`_destroy_array` stay Unknown on the access leaf.

`print`: one definition named `print` in `lib/ src/ plugins/ include/` at both
revisions (`cJSON.c:1234`; the yyjson hits are comments), so the pinned
identity binds the name to cJSON's definition only; another definition at a
later revision changes the sites and holds the contract (`source-changed`).

### Optional 1-tier, only callees whose own body is Safe in its home unit (W4 validates)

| callee | crate / module | clauses | predicted mover |
| --- | --- | --- | --- |
| `rd_bytes_hash` | librdkafka `kafka_mem.qv` | neutral | `rd_kafka_Uuid_hash` |
| `rd_kafka_mem_calloc` | librdkafka `kafka_mem.qv` | rk, num, size borrowed; owned nullable | `LZ4_createStreamHC` |
| `cio_file_hash` | chunkio `chunk_residency.qv` | `resource_return_borrowed_from(cf, nullable)` (rule 13a body check) | `cio_chunk_hash` 0 to 1 (its base is a member read, `ch->backend`) |
| `cio_file_native_get_size` | chunkio `chunk_sizes.qv` | neutral | `cio_file_real_size` |
| `cio_stream_size_chunks_up` | chunkio `chunk_sizes.qv` | neutral | `flb_input_chunk_total_size` (corpus and system) |
| `cmt_atomic_initialize` | cmetrics `context_lifetime.qv` | neutral | `cmt_initialize` |

Stop after these: the yield per contract is 1.

### Not declared (decision or reader extension), with sizes (functions decided alone)

| callee | alone | why |
| --- | ---: | --- |
| `rd_kafka_log0` | 3 | configured logging callback (brief) |
| `add_item_to_object` | 3 | retention beside a disposition (brief) |
| `cJSON_DetachItemViaPointer` | 3 | detach: custody of `item` moves from the parent's child list to the result; `return_alias` says the result owns nothing (it does), `return_owned` says it is fresh (it is `item`): no clause; a decision |
| `format_metric` | 3 | slot forwarding (brief) |
| `rd_kafka_assignor_add` | 3 | callbacks (brief) |
| `get_or_create_external_metadata_kvlist` | 2 | brief |
| `rd_atomic32_add` | 2 | refcount primitive (brief) |
| `_mk_event_wait_2` | 2 | class 7 (brief) |
| `cJSON_ParseWithLengthOpts` | 2 | declared with a global holder; withheld (class 1) |
| `call_once` | 2 | at-most-once (brief) |
| `mk_header_prepare` | 2 | in-place lifecycle (brief) |
| `rd_kafka_destroy_app` | 2 | releases `rk` only when `flags` lacks `F_IMMEDIATE`, and invokes application-registered callbacks held in `rk` (log, rebalance/commit through consumer close, interceptors' `on_destroy`): an open callback set registered elsewhere, outside C04.19; a declaration would read them as effect-free |
| `rd_kafka_metadata_new_topic_mockv_internal` | 2 | a `va_list` parameter: the declared-ABI reader withdraws it (`abi-unreadable-declaration`, measured on `rd_kafka_error_new_v` in `rdkafka_error.c`), `W43-LIBC-ABI-READER-SHAPES` |
| `rd_kafka_op_destroy`, `rd_kafka_q_serve0`, `rd_kafka_q_pop_serve0`, `rd_kafka_q_serve_maybe_consume` | 2 each | callbacks (brief) |
| `rd_kafka_parse_Metadata0` | 2 | fresh custody through an out slot (brief) |
| `rd_kafka_topic_partition_list_difference0`, `_intersection0`, `_union0` | 2 each | `cmp`/`hash` stored into a frame `RD_MAP_INITIALIZER` and invoked by the map: rule 13y's body check withdraws a synchronous-invocation clause (the body names `F` outside a direct call); the borrowed reading of a callback is forbidden |
| `rd_kafka_txn_clear_partitions_flag` | 2 | class 3, a container's reference set |
| `sched_thread_cleanup` | 2 | releases every element of a caller's intrusive list; no clause states it (`resource_elements_dropped` is registry-only) |
| `setupRackAwareAssignment0`, `ut_init_member_internal` | 2 each | class 2, unit-test constructors into caller storage |
| slice `file_pending_flush` | 2 | two distinct static definitions of one name (dockermode, multiline); a name-keyed declaration binds both |

## Predicted outcome

* Corpus: +14 to +20 Unknown -> Safe (stage a 3 to 5, b 2 to 3, c 2 to 4,
  d 4 to 6); no Safe -> non-Safe move; 0 to 3 added leaks or violations.
* System: +0 to +1 (`flb_input_chunk_total_size`).
* Slice: +0 to +2 (`flb_hash_table_del`'s two callers).
* Null property: candidates may rise where a declared non-null release or a
  borrowed result now reaches a caller; no finding predicted.
* Remaining corpus population after: about 490, by one callee alone about 335.
</content>
</invoke>
