# Final corrected access-path attribution for integration

This review uses the **combined corrected gateway** audit at `e6e31f49`,
compared with the contract-only baseline at `bfa510b7` (the same 230 provider
declarations on both sides). It supersedes the 219-contract, pre scalar-type
repair observations in this card's earlier `artifacts/after` and
`artifacts/system-after`. The full arbiter comparison, complete per-leaf rows,
and exact source replay are in the integrator's
`docs/reports/w46-contract-access-expansion/census/combined-{corpus,system}`.
`final-mechanism-review.json` here enumerates every nondirect changed leaf
with its source file, origin, unit index, source SHA, frontend SHA, same-function
admitted statements and mechanism category. It also lists all 36 corpus Safe
moves and both system configuration rows. The generator is
`integration/build_final_review.py`; no leaf is selected away to produce this
review.

## Exact identity and results

The corpus denominator remains 185 units and 4,834 functions. Compared with
contract-only, the corrected gateway moves **36 Unknown to Safe**, with no
Safe correction and no new provisional Unsafe: 411 Safe / 9 provisional
Unsafe / 4,414 Unknown becomes **447 / 9 / 4,378**. Unit status becomes
6 Safe / 8 provisional Unsafe / 167 Unknown / 4 data-only; `rdcrc32.c` is
the newly Safe unit. Unsupported leaves fall 41,045→39,085, alias leaves
rise 3,575→4,760, and leak leaves stay 34. The configured system slice
moves two rows of `flb_engine_get_chunk_backpressure_percent` to Safe (one
unique function): 54 / 4 / 390→**56 / 4 / 388** configured rows and
28 / 2 / 204→**29 / 2 / 203** unique functions. System unsupported falls
3,291→3,069, alias rises 712→892; four counted-user leaves remain.

Each of the 124 changed corpus units has exactly one replay match against
the pinned raw source with the archived dialect, include paths, definitions
and `HistoricalStdin` input. The recorded `/usr/bin/clang -E -P` command
exited 0 and reproduced the parity `source_sha256` in every matched unit;
23 nonmatching attempts remain in the replay record. The LZ4 shared-header
case is resolved by bytes: the archived `e2bfb700...` digest matches
`lz4hc.c`, while replaying `lz4.c` under the same request gives a distinct
digest. Monkey uses its configured separate source root, despite a
byte-identical copy under Fluent Bit. The system's fifteen compilation rows
carry `compile_task_identity.preprocessed_source_sha256`; eight distinct
hashes bind them directly to parity, including repeated configurations.
There is no unresolved source-to-unit binding in the final attribution.

## Safe moves and prediction

The 36 corpus Safe gains comprise eight CMetrics pointer declarators,
thirteen comparison helpers with provably untracked pointer declarators,
one indexed byte store with scalar field reads (`rd_kafka_Uuid_str`),
nine scalar member/index paths, and five other custody-neutral scalar
statements. The eight CMetrics helpers declare a local pointer, fill it
from `calloc`, and return it; their prior indirect refusal was on the
declarator itself, not the allocation. The comparison helpers' local
pointer aliases are never-bound to tracked custody. Examples of the typed
path group are `rd_slice_widen`'s `size_t end` store,
`rd_hdr_countsIndex`'s reads of histogram counts,
`rd_kafka_set_log_level`, `cio_set_log_level`, the telemetry counters, and
`mk_http_status`. The scalar group includes `rd_crc32_reflect`,
`rd_kafka_yield`, `hash_bytes`, `mk_string_copy_substr`, and
`content_length_parse`. Each exact statement text and source digest is in
`final-mechanism-review.json`. No pointer-valued store is credited as a P2
Safe gain. System's one unique Safe function reads scalar chunk counters
from `ins` in an arithmetic assignment in `src/flb_engine.c`; the two
compile configurations share the same analyzed source SHA.

The frozen P1 forecast has **32/45 correct verdicts**: 25 of 38 predicted
Safe rows gained and all seven predicted Unknown remain Unknown. Its
stricter changed-diagnostic metric is 30/45 because two predicted Unknown
rows (`fifo_drop_msg`, `mk_sched_broadcast_signal`) have unchanged
diagnostics. There are 13 predicted-Safe misses and 11 unforecast Safe
gains. Nine misses retain the old bounded causes in this card's report:
pointer casts (`flb_input_chunk_get_name`,
`flb_input_chunk_is_storage_overlimit`), pointer increment/multi-alias
declarator (`rd_kafka_sw_str_sanitize_inplace`), call arguments
(`rd_kafkap_leader_discovery_tmpabuf_add_alloc_topic(s)`), an indexed
pointer store plus call arguments (`ut_testConflictingPreviousAssignments`),
a global tracked-place boundary (`clear_unit_test_data`), and address escape
through `memset`/`memcpy` (`XXH32_reset`, `XXH64_reset`).

The four additional misses are the **positive builtin scalar-type boundary**
introduced by the post-suite soundness repair. `otel_span_set_kind` writes
the generated enum-typed `otel_span->kind` (`ctr_encode_opentelemetry.c:634`
and `trace.pb-c.h:35,376`); `LZ4F_resetDecompressionContext` writes enum
`dctx->dStage` (`lz4frame.c:1194,1210,1276`);
`rd_kafka_cgrp_clear_last_err` writes enum `rkcg_last_err`
(`rdkafka_cgrp.h:357`, `rdkafka.h:284,705`), while its separate integer
timestamp field is now admitted; and `ut_set_types` copies elements of an
enum pointer into an enum array (`rdunittest_fetcher.c:140–147`,
`rdkafka_share_acknowledgement.h:34–43,67`). The reader requires positive
builtin scalar evidence after an anonymous aggregate typedef was falsely
admitted by its old negative string test. These enum values are not presently
proved scalar by extraction, so they stay Unknown. Amend the existing
`W43-INDIRECT-UNTYPED-SINGLE-ACCESS` followup with a positive, owner-aware
enum classification/replay arm; this is a type-evidence extension, not a
reason to relax the union or unknown-aggregate refusal.

The 11 unforecast Safe gains are five CMetrics initializers
(`initialize_{numerical_data_point,resource_metrics,scope_metrics,summary_data_point}_list`
and `otlp_kvpair_list_initialize`), four librdkafka functions
(`rd_crc32_reflect`, `rd_kafka_Uuid_str`, `rd_kafka_yield`, `hash_bytes`),
and two Monkey functions (`mk_string_copy_substr`, `content_length_parse`).

## Every nondirect leaf

The corpus has 1,974 removed unsupported leaves: 1,902 at an exactly matched
new statement line and **72 nondirect**. Its 1,199 added alias leaves are
1,103 direct and **96 nondirect**. Fourteen removed alias and fourteen added
unsupported leaves are all nondirect. Thus the attached JSON classifies
**196 nondirect corpus leaves**. The system has 220 direct removed
unsupported and two callee-summary withdrawals, 148 direct and 36 later
added alias leaves, and four removed alias leaves: **42 nondirect system
leaves**. The direct groups are exhaustively present in the integrator's
per-leaf attribution; this review is focused on the leaves whose source
coordinate alone does not establish a direct statement match.

Of the 72 corpus nondirect removed unsupported leaves, 63 are withdrawn
uncontracted-callee refusals. Forty-five are for the header-inline
`rd_kafka_buf_ApiVersion_set` (`rdkafka_buf.h:1051–1056`), whose two
integer field stores have typed parity admissions across affected units;
eight are for `rd_kafka_q_mark_served` (`rdkafka_queue.h:534–537`), whose
scalar `sent` store has the same path evidence. Four are calls to newly Safe
`hash_bytes`, two to newly Safe `mk_string_copy_substr`, one each to newly
Safe `otlp_kvpair_list_initialize` and `rd_hdr_countsIndex`. Two calls to
`rd_kafka_anyconf_is_modified` also lose their former uncontracted-call
diagnostic, but that callee itself remains Unknown: its scalar assignment is
admitted while a return expression remains refused. Those two are a
diagnostic cascade, **not** a Safe summary claim. The six removed nullable
leaves and six of fourteen removed alias leaves occur at returns of five
CMetrics `calloc` initializers and `rd_kafka_Uuid_str` after earlier typed
declarator/byte-store admissions; prior refused-write handling could poison
the separate returned binding. The other eight removed alias leaves are
scalar locals (`nextSrcSizeHint`, `do_disconnect`, `confhdr`, `ts_delay`,
`is_paused`, `next_subscription_size`, `label_length`, `plugin_length`)
after typed field reads/stores. The last three removed unsupported leaves
are two branch-condition reprojections in `rdkafka_conf.c:3460,4815`
which now report `UnknownAlias conf`, and a `rdkafka_topic.c:250` indirect
refusal now reporting `UnknownAlias lrkt`; all remain Unknown.

The 96 nondirect added alias leaves are 91 later same-function obligations
and five cross-function/condition reprojections. The 91 arise after typed
statements expose a borrowed or otherwise unresolved root (`dst`, `config`,
`rkb`, `rkbuf`, `buckets`, `rktp`, and the other exact names in the JSON);
the source replay and same-function admitted-statement lists identify the
origin for every one, without claiming that the later alias discharges.
The five no-direct-statement leaves are four `conf` condition/call sites in
`rdkafka_conf.c` and `lrkt` at `rdkafka_topic.c:250`, replacing less precise
refusals. All fourteen added unsupported leaves are independent residuals:
seven indirect accesses, five nested calls (three `get_u32_at_offset`, one
`get_unaligned32`, one `rd_kafka_broker_ApiVersion_supported`), one return
expression in `rd_kafka_anyconf_is_modified`, and one nullable `result` use
in `cfl_kvlist_to_otlp_kvpair_list`. They remain non-Safe; no new
provisional Unsafe or inferred upstream violation is attributed to them.

The system's two nondirect removed unsupported leaves are the
`flb_engine_adaptive_flush_update` calls whose callee becomes Safe. Its 36
later added alias leaves follow newly admitted borrowed/scalar paths in
seven functions; for example `flb_input_chunk_place_new_chunk` reads
`i_ins = ic->in` then guards `i_ins->storage_type`. Four removed alias
leaves in `flb_input_chunk_write_header_v2` are `label_length` and
`plugin_length` in two configurations, following scalar reads of
`routes[index].label_length` and `.plugin_name_length` before their guards
(`src/flb_input_chunk.c:1947–1950,1978`, analyzed SHA
`4fce97f43c687bfa82e771cff913ae89a52809da103281f2c63274549a366f52`).

The integrator should use the final combined census, replay and this review
for STATUS and increment counts. The older P2 isolated counts remain a
historical, superseded checkpoint. P3 positions, tracked mixed places,
pointer-valued stores, union members and legacy-route parity remain named
followups; no old route was retired by P2.
