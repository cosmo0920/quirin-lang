# K5-LOCAL-ORIGIN-ALIASES: prediction

Committed after the re-census and before the semantic commit. The integrator
measures with base `rev:3ca85c7a6` on corpus (eight-home), twelve-home, system
and slice, and reruns the user's `current` after landing. The rows below are
those of the user's `current` report of 2026-11-24 (Fluent Bit `046accf6`,
1,173 Safe / 3,750 Unknown / 11 Unsafe). No route was measured for this file;
each declared move was read from the function's body at `046accf6` against
the reading below (README §3).

## What the card changes

Gateway rule 77: an automatic object-pointer local every write of which is a
null pointer constant, a string literal, a copy of another such local or a
**load** (the lvalue conversion of an object-pointer member or element whose
base reaches, through loads, `.` members, subscripts or `(&PLACE)->`, a
pointer formal rule 5b's return premise answers for, or another such local),
and every use of which is a null test, a dereference base, a call actual, a
`return`, its own rebinding or a copy into another such local, is decided as
the load itself:

* a **null test** of it whose binding has no target (identity derivation,
  unwidened state) narrows the binding exactly as before but reports no
  `UnknownAlias` and marks no edge unmodelled (so rule 11d's path premise
  then holds for the reads after it);
* a **`return`** of it under the same binding escapes nothing (rule 10k (2)'s
  exit for `return R->F;`).

Every custody use of such a local (release, transfer, owned or
custody-taking position) still finds no target and refuses.

## Per route and function

* **current**: of the 29 rows the re-census found blocked by K5 alias leaves
  alone, these 8 move `Unknown -> Safe` (every leaf is a null test or a
  `return` of an admitted local, rooted at a formal only read):
  `rd_kafka_topic_partition_get_leader_epoch`,
  `rd_kafka_topic_partition_get_current_leader_epoch` (`if (!(parpriv =
  rktpar->_private))`), `rd_kafka_sasl_cyrus_ready` (`handle =
  rk->rk_sasl.handle`), `cJSON_GetArraySize` (the `child` cursor),
  `request_data_point_count` (two nested loaded locals; the dereference
  leaves at lines 538 and 544 go with the guards, because the guards no
  longer mark the path unmodelled for rule 11d), `format_acl_error_message`
  (`user`/`group` loaded, null-tested, reassigned a literal),
  `get_hash_table` (`table = env->hash_table; ... return table;`) and
  `rd_kafka_toppar_get0` (`rktp = rkt->rkt_p[partition]` or `rkt->rkt_ua`,
  tested; the declared move most at risk: its `rd_kafka_toppar_keep_fl`
  actual is then read on an edge no longer unmodelled).
  Rows that stay Unknown, with their named reasons (README §4):
  `rd_kafka_timers_next` (its root is mentioned inside `likely(...)`'s
  `__builtin_expect` actual, which rule 5b's closure does not prove never
  bound), `fetch_metadata_kvlist_key` and `context_get_head` (the loaded
  base is a call result, not a formal), `metrics_check_label_value_existence`,
  `cio_chunk_close_stream` and `mk_fifo_queue_destroy_all` (a `container_of`
  statement expression; the last two also release the element),
  `mk_vhost_fdt_chain_lookup` and `append` (an interior address, rule 5j),
  `pack_header` (no null test or return: its leaves are dereferences in a
  loop, outside this rule), `rd_kafka_conf_prop_find` and
  `rd_kafka_anyconf_warn_deprecated` (a file-scope array, AG2's S7 refusal),
  `rd_kafka_global_cnt_get` (S7), `LZ4F_getBlockSize` (an enumerator into a
  formal), `rd_kafka_Uuid_str`, `rd_kafka_op_new0` and `mk_file_get_info`
  (non-pointer names), `net_plain_close` (a descriptor formal released),
  `LZ4F_headerSize` (a formal handed on), `mk_fifo_queue_id_destroy`,
  `mk_sched_event_free` (call results released or published),
  `mk_rconf_section_entry_add` (`mk_mem_alloc`'s nullable result, a real
  obligation) and `rd_kafka_topic_partition_new_with_topic_id` (a store
  through a derived result).
  Diagnostics: on rows that stay Unknown, `unknown_alias` leaves of the
  `guarded-other`, `borrowed-path-unguarded`, `derived-local-unguarded` and
  `poisoned-by-refusal` groups that are null tests or returns of admitted
  locals are removed, and dereference leaves after them may go too; rows can
  newly show a refusal the removed leaf hid. Callers: a function this run
  newly validates may publish a summary, so same-unit callers may change
  content or move `Unknown -> Safe`; not enumerated. No `Safe -> non-Safe`
  move. An `Unsafe` is possible only where a leak that the old unmodelled
  edge hid becomes visible; none is expected (the fixtures show a leak on
  such a path was already reported at base).
* **corpus** (eight-home) and **twelve-home**: the same functions where the
  pinned source (`66910c10`) carries them, declared below by
  `file::function`, with the same caution on `rd_kafka_toppar_get0`.
* **system** and **slice**: not derived (no unit of these routes was read).
  Fluent Bit core code tests and returns loaded members of its own records
  (`ins = ctx->ins; if (!ins) ...`), so `Unknown -> Safe` moves and content
  changes are possible there; they are not declared and would be reported as
  unpredicted.

```measure-prediction
route corpus
move cJSON.c::cJSON_GetArraySize Unknown -> Safe
move cio_file.c::format_acl_error_message Unknown -> Safe
move cmt_encode_opentelemetry_batch.c::request_data_point_count Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_get_current_leader_epoch Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_get_leader_epoch Unknown -> Safe
move rdkafka_partition.c::rd_kafka_toppar_get0 Unknown -> Safe
move rdkafka_sasl_cyrus.c::rd_kafka_sasl_cyrus_ready Unknown -> Safe
move snappy.c::get_hash_table Unknown -> Safe
route twelve-home
move cJSON.c::cJSON_GetArraySize Unknown -> Safe
move cio_file.c::format_acl_error_message Unknown -> Safe
move cmt_encode_opentelemetry_batch.c::request_data_point_count Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_get_current_leader_epoch Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_get_leader_epoch Unknown -> Safe
move rdkafka_partition.c::rd_kafka_toppar_get0 Unknown -> Safe
move rdkafka_sasl_cyrus.c::rd_kafka_sasl_cyrus_ready Unknown -> Safe
move snappy.c::get_hash_table Unknown -> Safe
```

## Post-measurement amendment (R34-K-REPAIR, 2026-11-25)

The block above is unchanged; this section records the one move the landing
measurement of the combined K tip (`c5cd10688`) found for this card that the
frozen forecast did not declare. A unit-scoped reproduction (Monkey linked)
at this card's tip `ad6b12a75` against `3ca85c7a6` moves it
([R34-K-REPAIR](../r34-k-repair/README.md) §1):

| Route | Function | Base | Measured | Rule 77 clause |
|---|---|---|---|---|
| corpus, twelve-home | `mk_core/mk_string.c::mk_string_html_escape` | Unknown | Safe | `writes(escape) ⊆ {null, literal}`, `uses(escape) ⊆ {nullTest, callArgument}`: the guard at line 510 is decided; with no refusal left the in-unit summary validates `Owned {nullable: true}`, which permits the nullable `return mk_string_dup("")` at line 462 (rule 74d's leaf belongs to the contract-free run only) |

The forecast missed it because the row carried a second leaf (line 462),
which is a consequence of the first, not an independent blocker.
