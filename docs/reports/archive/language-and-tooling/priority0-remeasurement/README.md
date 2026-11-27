# PRIORITY0-ATTRIBUTION: the priority-0 remeasurement, row by row

Round 39, base `8f988f246` (tree identical to `9aed1913a`). USER DECISION
2026-11-26 (Priority 0): "remeasure Quirin's current main (9aed1913a) with
the existing pinned inputs and account for the differences row by row";
acceptance criteria 1-3 are this page's subject, criterion 4 (publication)
is not. No analyzer, contract or Lean change; nothing was re-measured.

## Inputs, tool and completeness

The measurement (read-only):
`.claude/worktrees/priority0/private/measure/priority0-20261125T173228Z/`,
one run of three sides, prev `rev:556460e31`, common `rev:2c07390b6`, tip
`rev:9aed1913a`, all four routes, inputs-base the main checkout. Every
route of both of its comparisons (prev->tip, common->tip) is `complete`
with no unmatched row, no unmatched unit row and no composition conflict;
the three sides' per-route inputs (route digest, compile-database digests,
spelling, source date) are identical.

The chain tool is QUIRIN-AUDIT-CONNECT's `quirin-audit measure chain`, built
release from `abc7ce7ad` in a detached worktree with its own target
directory (plugin `quirin-audit-measure` sha256 `bcccc212…f85636`), run as
two `--runs` steps over the side directories, prev->common and common->tip,
with `--declared` every PREDICTION.md of a card landed after `556460e31`
(85 files; 86 blocks after splitting the two files that carry two, each on the step its card landed in) and `--attribute` every
native-comparison record committed after it (45 records)
([scripts/run_chain.py](scripts/run_chain.py), argv in
[chain/argv.json](chain/argv.json)). The link joins (no refused field);
every route is `complete`. The tool exits 1 by design: 111 new `Safe` rows
had no attribution it can make (below). Record:
[chain/chain.json.gz](chain/chain.json.gz) (content sha256 `9bb99d0c0d0eb475dd98ce88fef02f486f0e05009ec431b96535745be6ad7c0e`, archive `4d15b7a00fd624940ee7e1950caaa3600b48eb1a38421521ee3f507bb050cc1e`), all digests in
[artifacts/SHA256.json](artifacts/SHA256.json).

## Figures

Function rows Safe / Pending / Unknown / provisional Unsafe; unit rows
Safe / Unknown / provisional Unsafe (corpus also 4 data-only units).

| Route | prev `556460e31` | common `2c07390b6` | tip `9aed1913a` |
|---|---|---|---|
| corpus (`eight-home`, 4,834) | 1,079 / 0 / 3,745 / 10 | 1,119 / 0 / 3,703 / 12 | **1,187 / 0 / 3,634 / 13** |
| corpus units (185) | 9 / 164 / 8 | 9 / 163 / 9 | 9 / 162 / 10 |
| corpus NULL (4,921) | 2,549 / 2,355 / 17 | 2,549 / 2,355 / 17 | 2,553 / 2,351 / 17 |
| `twelve-home` (4,834) | as corpus | as corpus | as corpus |
| system (`fluent-bit-system`, 448) | 103 / 0 / 341 / 4 | 112 / 0 / 332 / 4 | **114 / 0 / 329 / 5** |
| system units (15) | 0 / 13 / 2 | 0 / 13 / 2 | 0 / 12 / 3 |
| slice (`fluent-bit-tail-pause`, 404) | 58 / 4 / 341 / 1 | 63 / 4 / 336 / 1 | **65 / 4 / 334 / 1** |

System and slice NULL (454 and 406 rows) do not move. prev reproduces the
`556460e31` record and common the `2c07390b6` record (integrator check).

| Route | Moved rows (function + unit) | prev->common | common->tip | Evidence-only rows |
|---|---|---|---|---|
| corpus | 119 + 4 | 42 U->S, 2 S->U, 2 U->Unsafe; unit 1 | 69 U->S, 1 S->U, 2 U->Unsafe, 1 Unsafe->U; units 3 | 1,495 |
| twelve-home | 119 + 4 | the same rows and moves | the same | 1,498 |
| system | 20 + 1 | 13 U->S, 4 S->U | 2 U->S, 1 U->Unsafe; unit 1 | 131 |
| slice | 7 | 5 U->S | 2 U->S | 91 |

No row moves in both steps, and no row's verdict at prev equals its verdict
at tip while differing at common (no cancelled move at the chain's own
granularity). Cancellations inside a step are found below from the
committed landing records.

## How each move is attributed

1. The chain tool attributes a move when a card landed in that step
   predicted it (`--declared`) or measured and predicted it in its own
   pinned record (`--attribute`): 124 corpus / twelve-home, 12 system and
   5 slice rows.
2. Every other move is tied to the card whose own measurement or report
   shows it, and checked here against the record it names
   ([scripts/moves_table.py](scripts/moves_table.py), `CAUSES`). Where a
   card did not measure the pinned routes, the move is placed by the
   **committed pinned landing records**: 23 integration records whose
   inputs equal this measurement's on every route they measure
   ([scripts/landing_records.py](scripts/landing_records.py),
   [artifacts/landing-records.json.gz](artifacts/landing-records.json.gz)).
   Their side figures bracket every unmeasured interval of main:

   | Checkpoint (pinned record side) | corpus | system | slice |
   |---|---|---|---|
   | `556460e31` prev | 1,079 / 3,745 / 10 | 103 / 341 / 4 | 58 / 4 / 341 / 1 |
   | `0f6e17118` (r12; base of four records) | 1,081 / 3,743 / 10 | 103 / 341 / 4 | 59 / 4 / 340 / 1 |
   | `36b2cf931`, `5c824e443` (CG25 bases) | 1,078 / 3,746 / 10 | 107 / 337 / 4 | 63 / 4 / 336 / 1 |
   | `a9c6dcf44` (AG base) | 1,079 / 3,745 / 10 | 107 / 337 / 4 | 63 / 4 / 336 / 1 |
   | `2c07390b6` common (= AG tip `8a85e233f`, SP1) | 1,119 / 3,703 / 12 | 112 / 332 / 4 | 63 / 4 / 336 / 1 |
   | `70e10930f` (r46; SP2 base) | 1,131 / 3,691 / 12 | 114 / 330 / 4 | 65 / 4 / 334 / 1 |
   | `b069dbda8`, `38cf960fa` (SP5) | 1,150 / 3,672 / 12 | 114 / 330 / 4 | 65 / 4 / 334 / 1 |
   | `1f2007667` (r56; SP6 base) | 1,150 / 3,673 / 11 | 114 / 329 / 5 | as above |
   | `9193b196a` (after r57/r58) | 1,152 / 3,670 / 12 | 114 / 329 / 5 | as above |
   | `e6d86d596`, `2aaf3a585` (r62, SP9) | 1,153 / 3,669 / 12 | as above | as above |
   | `4be28c3cd` (r71; round-34 base) | 1,153 / 3,668 / 13 | as above | as above |
   | `c5cd10688`, `5087f9295` (round 34) | 1,169 / 3,652 / 13 | as above | as above |
   | `e4db0e6a8` / `110a69c8e` (MES) | 1,181 / 3,640 / 13 | as above | as above |
   | `e79a72c46` (round 37) = tip | 1,187 / 3,634 / 13 | 114 / 329 / 5 | 65 / 4 / 334 / 1 |

   Each unmeasured interval's moves are exactly the rows assigned to it
   below (counts per status match at both ends), with one exception that
   is a cancellation (next section).
3. twelve-home is the same preparation as corpus with another link map. A
   twelve-home move with no attribution of its own is the corpus row's,
   moved identically in the same step (the cards' blocks had no
   twelve-home route): 36 rows.

Every custody move has one of the three (criterion 2); none is held. One
NULL-property `Safe` was held for its published assumptions and is accepted
by the user's decision of 2026-11-26 (below).

### Attribution summary

Rows per route (function and unit rows), and the moves on corpus, system
and slice (twelve-home repeats corpus).

| Card | corpus | twelve-home | system | slice | Moves |
|---|---|---|---|---|---|
| AG1-INDIRECT-MEMBER-ACCESS | 9 | 9 | | | 6 U->S, 2 U->Unsafe, unit U->Unsafe |
| AG2-UNRESOLVED-ALIAS | 33 | 33 | 5 | | 38 U->S |
| AG3-CONDITION-RETURN-CALLS | 1 | 1 | | | 1 U->S |
| AMPERSAND-RUN-PARITY | 1 | 1 | | | 1 U->S |
| CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (r3, r4) | 1 + 7 | 1 + 7 | | | 8 U->S |
| CG10-MEMBER-STORE-RETAINED-BY | 11 | 11 | 2 | 2 | 15 U->S |
| CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | 10 | 10 | | | 10 U->S |
| CG10-READER-GAPS | 2 | 2 | | | 2 U->S |
| CG22-CALLBACK-CANDIDATE-CONTRACTS | 1 | 1 | | | 1 U->S |
| CG25-IN-PLACE-REGEX | 1 | 1 | | | 1 S->U |
| CURRENT-CONTEXT-NEW-TRIAGE (D3) | 1 | 1 | | | 1 U->S |
| K1-CALL-ACTUAL-ACCESS | 2 | 2 | | | 2 U->S |
| K2-NO-TYPED-ACCESS / K6-DIRECT-POSITION-ACCESS | 2 + 4 | 2 + 4 | | | 6 U->S |
| K3-POINTER-VALUED-STORES | 3 | 3 | | | 3 U->S |
| K5-LOCAL-ORIGIN-ALIASES | 9 | 9 | | | 9 U->S |
| K7-RESIDUAL-CAUSES | 2 | 2 | | | 2 U->S |
| LATER-DECLARATOR-INITIALISER-UNREAD | 3 | 3 | | | 1 U->S, 1 U->Unsafe, unit U->Unsafe |
| LERI-HELD-AND-LINK-MEMBER-PREMISES | | | 4 | | 4 S->U |
| LIST-ENTRY-RELEASE-IDENTITY | | | 4 | 1 | 5 U->S |
| MEMBER-EFFECT-STORES | 12 | 12 | | | 12 U->S |
| NULLABLE-GUARDED-RELEASE | 2 | 2 | 2 | | Unsafe->U + unit; U->Unsafe + unit |
| RK-TOPIC-PARTITION-LIST-CONSTRUCTORS | 4 | 4 | | | 2 U->S, 1 U->Unsafe, unit U->Unsafe |
| RULE50-CONSUMER-COMPOSITION | 2 | 2 | | | 2 S->U |
| THREAD-LOCAL-CUSTODY-FREE-READ | | | 4 | 4 | 8 U->S |
| **total** | **123** | **123** | **21** | **7** | |

By how the cause is shown: chain tool 141 (corpus 80, twelve-home 44,
system 12, slice 5) plus 36 twelve-home rows as corpus; the card's own
measurement, its prediction block missing the row, 69 (corpus 32,
twelve-home 32, system 5); the card's report predicting it outside its
block 12; the card's rule on the pinned tree's body 2
(`ctr_link_set_attributes`); and a cause verified here 14 (the
NULLABLE-GUARDED-RELEASE and RULE50 rows, `flb_output_set_context` and
`flb_input_set_context`). The K3/K5/K7 rows are attributed through the
round-34 record, whose declared check is the four cards' union block, and
their own blocks; K1's two rows and K5's `mk_string_html_escape`, the
record's three unpredicted rows, are attributed by R34-K-REPAIR's corrected
prediction (its report section 4).

### The attribution table

Every moved row, its routes (`0`/`1` the unit configuration), its verdict at
prev / common / tip (the step is where it changes), the card and the class
of evidence. The evidence text of each row is in
[artifacts/moves-table.json.gz](artifacts/moves-table.json.gz).

| Row | Routes | prev / common / tip | Card | Class |
|---|---|---|---|---|
| `flb_input_chunk.c::input_chunk_has_magic_bytes` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | card-measured |
| `lz4frame.c::LZ4F_optimalBSID` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | card-measured |
| `plugins/in_storage_backlog/sb.c::sb_is_rejected_stream` | system0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | card-measured |
| `rdkafka_msg.c::rd_kafka_message_leader_epoch` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | card-measured |
| `rdkafka_partition.c::rd_kafka_topic_partition_cmp_opaque` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | card-measured |
| `src/flb_input_chunk.c::input_chunk_has_magic_bytes` | system0, system1 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | card-measured |
| `src/flb_task.c::flb_task_retry_count` | system0, system1 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | card-measured |
| `rdkafka_telemetry_encode.c::get_member_id` | corpus0, twelve-home0 | Unknown / Unknown / Safe | AMPERSAND-RUN-PARITY | card-measured |
| `ctr_encode_opentelemetry.c::otel_span_set_name` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | card-measured |
| `ctr_encode_opentelemetry.c::otel_span_set_trace_state` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | card-measured |
| `mk_core/mk_memory.c::mk_ptr_set` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | card-measured |
| `nanopb/pb_common.c::pb_field_iter_begin` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | card-measured |
| `rdkafka_conf.c::rd_kafka_confval_disable` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | card-measured |
| `rdkafka_conf.c::rd_kafka_confval_init_int` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | card-measured |
| `snappy.c::init_snappy_decompressor` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | card-measured |
| `cmt_decode_msgpack.c::unpack_opts_desc` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `cmt_decode_msgpack.c::unpack_opts_name` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `cmt_decode_msgpack.c::unpack_opts_ns` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `cmt_decode_msgpack.c::unpack_opts_ss` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `cmt_decode_msgpack.c::unpack_static_label_component` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `ctr_decode_msgpack.c::unpack_instrumentation_scope_name` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `ctr_decode_msgpack.c::unpack_instrumentation_scope_version` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `ctr_decode_msgpack.c::unpack_link_trace_state` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `ctr_decode_msgpack.c::unpack_span_schema_url` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `ctr_decode_msgpack.c::unpack_span_status_message` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT | card-measured |
| `rdkafka_assignment.c::rd_kafka_assignment_pause` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-READER-GAPS | card-measured |
| `rdkafka_assignment.c::rd_kafka_assignment_resume` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-READER-GAPS | card-measured |
| `rdkafka_broker.c::rd_kafka_broker_destroy_error` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K1-CALL-ACTUAL-ACCESS | card-measured |
| `rdkafka_partition.c::rd_kafka_toppar_is_on_cgrp` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K1-CALL-ACTUAL-ACCESS | card-measured |
| `mk_core/mk_string.c::mk_string_html_escape` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES | card-measured |
| `rdkafka_msg.c::rd_kafka_msgq_allow_wakeup_at` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K6-DIRECT-POSITION-ACCESS | card-measured |
| `rdkafka_conf.c::rd_kafka_sw_str_sanitize_inplace` | corpus0, twelve-home0 | Unknown / Unknown / Safe | LATER-DECLARATOR-INITIALISER-UNREAD | card-measured |
| `ctr_link.c::ctr_link_set_trace_state` | corpus0, twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | card-measured |
| unit `rdkafka_sticky_assignor.c` | corpus0, twelve-home0 | Unknown / Unknown / Unsafe | RK-TOPIC-PARTITION-LIST-CONSTRUCTORS | card-measured |
| `rdkafka_sticky_assignor.c::ut_testSameSubscriptions` | corpus0, twelve-home0 | Unknown / Unknown / Unsafe | RK-TOPIC-PARTITION-LIST-CONSTRUCTORS | card-measured |
| `lz4.c::LZ4_prepareTable` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | card-predicted-prose |
| `lz4hc.c::LZ4HC_clearTables` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | card-predicted-prose |
| `rdxxhash.c::XXH32_digest` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | card-predicted-prose |
| `rdxxhash.c::XXH64_digest` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | card-predicted-prose |
| `rdkafka_sasl_scram.c::rd_kafka_sasl_safe_string` | corpus0, twelve-home0 | Unknown / Unknown / Unsafe | LATER-DECLARATOR-INITIALISER-UNREAD | card-predicted-prose |
| unit `rdkafka_sasl_scram.c` | corpus0, twelve-home0 | Unknown / Unknown / Unsafe | LATER-DECLARATOR-INITIALISER-UNREAD | card-predicted-prose |
| `ctr_link.c::ctr_link_set_attributes` | corpus0, twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | pinned-tree |
| `cJSON.c::cJSON_CreateArrayReference` | corpus0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| `cJSON.c::cJSON_CreateObjectReference` | corpus0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| `cJSON.c::cJSON_CreateStringReference` | corpus0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_new` | corpus0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_new_with_id_and_name` | corpus0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| unit `rdunittest_acknowledge.c` | corpus0 | Unknown / Unsafe / Unsafe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| `rdunittest_acknowledge.c::ut_ack_create_message` | corpus0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| `rdunittest_acknowledge.c::ut_case_acknowledge_accept` | corpus0 | Unknown / Unsafe / Unsafe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| `rdunittest_acknowledge.c::ut_case_acknowledge_multiple_partitions` | corpus0 | Unknown / Unsafe / Unsafe | AG1-INDIRECT-MEMBER-ACCESS | tool |
| `cmt_decode_msgpack.c::unpack_exp_histogram_count` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_exp_histogram_negative_bucket` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_exp_histogram_positive_bucket` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_exp_histogram_sum` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_exp_histogram_sum_set` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_exp_histogram_zero_count` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_histogram_bucket` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_histogram_count` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_metric_hash` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_metric_value_type` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_metric_value_uint64` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_summary_count` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_summary_quantile` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_summary_quantiles_set` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_decode_msgpack.c::unpack_summary_sum` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_encode_opentelemetry.c::get_context_arena` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `cmt_encode_text.c::append_summary_metric_value` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_assignor.c::rd_kafka_assignor_cmp_str` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_assignor.c::rd_kafka_group_member_cmp` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_broker.c::rd_kafka_broker_start_reauth_cb` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_cert.c::rd_kafka_conf_ssl_passwd_cb` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_header.c::rd_kafka_header_cmp_str` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_mock.c::rd_kafka_mock_connection_write_out_tmr_cb` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_cmp` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_cmp_topic` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_not_regex` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_range_assignor.c::rd_kafka_member_assigned_partitions_pair_cmp` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_ssl.c::rd_kafka_transport_ssl_passwd_cb` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `rdkafka_topic.c::rd_kafka_topic_info_topic_cmp` | corpus0, twelve-home0 | Unknown / Safe / Safe | AG2-UNRESOLVED-ALIAS | tool |
| `mk_server/mk_http_parser.c::header_cmp` | corpus0 | Unknown / Safe / Safe | AG3-CONDITION-RETURN-CALLS | tool |
| `cio_stream.c::get_stream_path` | corpus0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R3 | tool |
| `rdkafka_broker.c::rd_kafka_broker_update_reconnect_backoff` | corpus0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | tool |
| `rdxxhash.c::XXH32_digest_endian` | corpus0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | tool |
| `rdxxhash.c::XXH64_digest_endian` | corpus0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | tool |
| `cJSON.c::suffix_object` | corpus0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | tool |
| `rdkafka_buf.c::rd_kafka_buf_set_maker` | corpus0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | tool |
| `rdkafka_mock.c::rd_kafka_mock_partition_set_leader0` | corpus0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | tool |
| `rdkafka_feature.c::rd_kafka_ApiVersion_check` | corpus0 | Unknown / Safe / Safe | CG22-CALLBACK-CANDIDATE-CONTRACTS | tool |
| `rdregex.c::rd_regex_match` | corpus0 | Safe / Safe / Unknown | CG25-IN-PLACE-REGEX | tool |
| `mk_server/mk_http_thread.c::mk_http_thread_initialize_tls_once` | corpus0 | Unknown / Safe / Safe | CURRENT-CONTEXT-NEW-TRIAGE | tool |
| `lz4.c::LZ4_hashPosition` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K2-NO-TYPED-ACCESS, K6-DIRECT-POSITION-ACCESS | tool |
| `rdkafka_share_acknowledgement.c::rd_kafka_share_partition_offsets_list_new` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K2-NO-TYPED-ACCESS, K6-DIRECT-POSITION-ACCESS | tool |
| `mk_core/mk_iov.c::mk_iov_consume` | corpus0 | Unknown / Unknown / Safe | K3-POINTER-VALUED-STORES, R34-K-REPAIR | tool |
| `nanopb/pb_decode.c::buf_read` | corpus0 | Unknown / Unknown / Safe | K3-POINTER-VALUED-STORES, R34-K-REPAIR | tool |
| `snappy.c::writer_set_expected_length` | corpus0 | Unknown / Unknown / Safe | K3-POINTER-VALUED-STORES, R34-K-REPAIR | tool |
| `cJSON.c::cJSON_GetArraySize` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES, R34-K-REPAIR | tool |
| `cio_file.c::format_acl_error_message` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES, R34-K-REPAIR | tool |
| `cmt_encode_opentelemetry_batch.c::request_data_point_count` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES, R34-K-REPAIR | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_get_current_leader_epoch` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES, R34-K-REPAIR | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_get_leader_epoch` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES, R34-K-REPAIR | tool |
| `rdkafka_partition.c::rd_kafka_toppar_get0` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES, R34-K-REPAIR | tool |
| `rdkafka_sasl_cyrus.c::rd_kafka_sasl_cyrus_ready` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES, R34-K-REPAIR | tool |
| `snappy.c::get_hash_table` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K5-LOCAL-ORIGIN-ALIASES, R34-K-REPAIR | tool |
| `rdkafka_mock.c::rd_kafka_mock_partition_next_leader_response` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K6-DIRECT-POSITION-ACCESS | tool |
| `rdkafka_mock_handlers.c::rd_kafka_mock_msgset_est_record_size` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K6-DIRECT-POSITION-ACCESS | tool |
| `rdkafka_request.c::rd_kafkap_leader_discovery_tmpabuf_add_alloc_brokers` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K6-DIRECT-POSITION-ACCESS | tool |
| `cJSON.c::buffer_skip_whitespace` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K7-RESIDUAL-CAUSES, R34-K-REPAIR | tool |
| `cmt_decode_statsd.c::cmt_get_statsd_type` | corpus0, twelve-home0 | Unknown / Unknown / Safe | K7-RESIDUAL-CAUSES, R34-K-REPAIR | tool |
| `src/flb_output.c::flb_output_task_queue_enqueue` | system0, system1 | Safe / Unknown / Unknown | LERI-HELD-AND-LINK-MEMBER-PREMISES | tool |
| `src/flb_scheduler.c::flb_sched_timer_cb_destroy` | system0, system1 | Safe / Unknown / Unknown | LERI-HELD-AND-LINK-MEMBER-PREMISES | tool |
| `plugins/in_tail/tail_file.c::flb_tail_file_remove_all` | slice0 | Unknown / Safe / Safe | LIST-ENTRY-RELEASE-IDENTITY | tool |
| `src/flb_engine.c::flb_engine_destroy_tasks` | system0, system1 | Unknown / Safe / Safe | LIST-ENTRY-RELEASE-IDENTITY | tool |
| `src/flb_storage.c::flb_storage_input_destroy` | system0, system1 | Unknown / Safe / Safe | LIST-ENTRY-RELEASE-IDENTITY | tool |
| `cmt_decode_msgpack.c::create_counter_instance` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `cmt_decode_msgpack.c::create_exp_histogram_instance` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `cmt_decode_msgpack.c::create_gauge_instance` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `cmt_decode_msgpack.c::create_histogram_instance` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `cmt_decode_msgpack.c::create_summary_instance` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `cmt_decode_msgpack.c::create_untyped_instance` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `mk_server/mk_lib.c::mk_vhost_set_property` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `rdbuf.c::rd_buf_init` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `rdkafka_assignment.c::rd_kafka_assignment_init` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `rdkafka_fetcher.c::rd_kafkap_Fetch_reply_tags_set_topic_cnt` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `snappy.c::rd_kafka_snappy_init_env` | corpus0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_copy_void` | corpus0 | Unknown / Unknown / Safe | RK-TOPIC-PARTITION-LIST-CONSTRUCTORS | tool |
| `rdkafka_partition.c::rd_kafka_topic_partition_list_copy_opaque` | corpus0 | Unknown / Unknown / Safe | RK-TOPIC-PARTITION-LIST-CONSTRUCTORS | tool |
| `src/flb_engine.c::flb_engine_evl_get` | system0, system1 | Unknown / Safe / Safe | THREAD-LOCAL-CUSTODY-FREE-READ | tool |
| `src/flb_lib.c::flb_cf_context_get` | slice0, slice1 | Unknown / Safe / Safe | THREAD-LOCAL-CUSTODY-FREE-READ | tool |
| `src/flb_lib.c::flb_context_get` | slice0, slice1 | Unknown / Safe / Safe | THREAD-LOCAL-CUSTODY-FREE-READ | tool |
| `src/flb_scheduler.c::flb_sched_ctx_get` | system0, system1 | Unknown / Safe / Safe | THREAD-LOCAL-CUSTODY-FREE-READ | tool |
| `cJSON.c::cJSON_CreateArrayReference` | twelve-home0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| `cJSON.c::cJSON_CreateObjectReference` | twelve-home0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| `cJSON.c::cJSON_CreateStringReference` | twelve-home0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| `rdkafka_partition.c::rd_kafka_topic_partition_new` | twelve-home0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| `rdkafka_partition.c::rd_kafka_topic_partition_new_with_id_and_name` | twelve-home0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| unit `rdunittest_acknowledge.c` | twelve-home0 | Unknown / Unsafe / Unsafe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| `rdunittest_acknowledge.c::ut_ack_create_message` | twelve-home0 | Unknown / Safe / Safe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| `rdunittest_acknowledge.c::ut_case_acknowledge_accept` | twelve-home0 | Unknown / Unsafe / Unsafe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| `rdunittest_acknowledge.c::ut_case_acknowledge_multiple_partitions` | twelve-home0 | Unknown / Unsafe / Unsafe | AG1-INDIRECT-MEMBER-ACCESS | tool; twelve-home as corpus |
| `mk_server/mk_http_parser.c::header_cmp` | twelve-home0 | Unknown / Safe / Safe | AG3-CONDITION-RETURN-CALLS | tool; twelve-home as corpus |
| `cio_stream.c::get_stream_path` | twelve-home0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R3 | tool; twelve-home as corpus |
| `rdkafka_broker.c::rd_kafka_broker_update_reconnect_backoff` | twelve-home0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | tool; twelve-home as corpus |
| `rdxxhash.c::XXH32_digest_endian` | twelve-home0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | tool; twelve-home as corpus |
| `rdxxhash.c::XXH64_digest_endian` | twelve-home0 | Unknown / Unknown / Safe | CG10-INDIRECT-ACCESS-SOLE-RESIDUALS-R4 | tool; twelve-home as corpus |
| `cJSON.c::suffix_object` | twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | tool; twelve-home as corpus |
| `rdkafka_buf.c::rd_kafka_buf_set_maker` | twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | tool; twelve-home as corpus |
| `rdkafka_mock.c::rd_kafka_mock_partition_set_leader0` | twelve-home0 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | tool; twelve-home as corpus |
| `rdkafka_feature.c::rd_kafka_ApiVersion_check` | twelve-home0 | Unknown / Safe / Safe | CG22-CALLBACK-CANDIDATE-CONTRACTS | tool; twelve-home as corpus |
| `rdregex.c::rd_regex_match` | twelve-home0 | Safe / Safe / Unknown | CG25-IN-PLACE-REGEX | tool; twelve-home as corpus |
| `mk_server/mk_http_thread.c::mk_http_thread_initialize_tls_once` | twelve-home0 | Unknown / Safe / Safe | CURRENT-CONTEXT-NEW-TRIAGE | tool; twelve-home as corpus |
| `mk_core/mk_iov.c::mk_iov_consume` | twelve-home0 | Unknown / Unknown / Safe | K3-POINTER-VALUED-STORES, R34-K-REPAIR | tool; twelve-home as corpus |
| `nanopb/pb_decode.c::buf_read` | twelve-home0 | Unknown / Unknown / Safe | K3-POINTER-VALUED-STORES, R34-K-REPAIR | tool; twelve-home as corpus |
| `snappy.c::writer_set_expected_length` | twelve-home0 | Unknown / Unknown / Safe | K3-POINTER-VALUED-STORES, R34-K-REPAIR | tool; twelve-home as corpus |
| `cmt_decode_msgpack.c::create_counter_instance` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `cmt_decode_msgpack.c::create_exp_histogram_instance` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `cmt_decode_msgpack.c::create_gauge_instance` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `cmt_decode_msgpack.c::create_histogram_instance` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `cmt_decode_msgpack.c::create_summary_instance` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `cmt_decode_msgpack.c::create_untyped_instance` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `mk_server/mk_lib.c::mk_vhost_set_property` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `rdbuf.c::rd_buf_init` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `rdkafka_assignment.c::rd_kafka_assignment_init` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `rdkafka_fetcher.c::rd_kafkap_Fetch_reply_tags_set_topic_cnt` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `snappy.c::rd_kafka_snappy_init_env` | twelve-home0 | Unknown / Unknown / Safe | MEMBER-EFFECT-STORES | tool; twelve-home as corpus |
| `rdkafka_partition.c::rd_kafka_topic_partition_copy_void` | twelve-home0 | Unknown / Unknown / Safe | RK-TOPIC-PARTITION-LIST-CONSTRUCTORS | tool; twelve-home as corpus |
| `rdkafka_partition.c::rd_kafka_topic_partition_list_copy_opaque` | twelve-home0 | Unknown / Unknown / Safe | RK-TOPIC-PARTITION-LIST-CONSTRUCTORS | tool; twelve-home as corpus |
| `src/flb_input.c::flb_input_set_context` | slice0, slice1 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | verified |
| `src/flb_output.c::flb_output_set_context` | system0, system1 | Unknown / Unknown / Safe | CG10-MEMBER-STORE-RETAINED-BY | verified |
| unit `plugins/in_storage_backlog/sb.c` | system0 | Unknown / Unknown / Unsafe | NULLABLE-GUARDED-RELEASE | verified |
| `plugins/in_storage_backlog/sb.c::sb_remove_chunk_from_segregated_backlog` | system0 | Unknown / Unknown / Unsafe | NULLABLE-GUARDED-RELEASE | verified |
| unit `rdhttp.c` | corpus0, twelve-home0 | Unsafe / Unsafe / Unknown | NULLABLE-GUARDED-RELEASE | verified |
| `rdhttp.c::unittest_http_get_params_append` | corpus0, twelve-home0 | Unsafe / Unsafe / Unknown | NULLABLE-GUARDED-RELEASE | verified |
| `mk_server/mk_scheduler.c::mk_sched_threads_destroy_conn` | corpus0, twelve-home0 | Safe / Unknown / Unknown | RULE50-CONSUMER-COMPOSITION | verified |
| `mk_server/mk_scheduler.c::sched_thread_cleanup` | corpus0, twelve-home0 | Safe / Unknown / Unknown | RULE50-CONSUMER-COMPOSITION | verified |

## New Safe rows: their evidence (criterion 3)

244 new `Safe` function rows (corpus 111, twelve-home 111, system 15,
slice 7) and 4 corpus + 4 twelve-home NULL rows. Each function row is read
at the tip side ([scripts/new_safe_evidence.py](scripts/new_safe_evidence.py),
[artifacts/newsafe.json.gz](artifacts/newsafe.json.gz)): status `Safe` with
**zero** diagnostics, the analyzer's statement that every scoped obligation
of `resource-custody.v1` discharged; what it rests on is in its own record:

| Route | Rests on no declaration | on declarations | of which hypothesis premises | of which NULL premises |
|---|---|---|---|---|
| corpus, twelve-home (each) | 27 | 84 | 13 | 7 |
| system | 6 | 9 | 0 | 0 |
| slice | 6 | 1 | 0 | 0 |

The rule each rests on is the card's, checked by its card gate and Lean
module; for the rows the card did not predict the mechanism was checked
here against the row's base leaf and the rule text:

* `ctr_link_set_attributes` (corpus, twelve-home): its only common leaf is
  rule 38i's `store of a custody-carrying value into link->attr, a member
  of the formal link (member-store-through-formal)` at `ctr_link.c:99`;
  the pinned body is `if (!attr) return -1; link->attr = attr; return 0;`,
  exactly rule 70's `F->m = q` (formal `q` of object-pointer type, both
  never rebound), read as `retained_by(attr, link.attr)`. The tree the
  card measured (`046accf6`) releases the old member first and stays
  Unknown (its evidence lists the row Unknown -> Unknown).
* `flb_output_set_context` (system, both configurations) and
  `flb_input_set_context` (slice, both): single statements
  `ins->context = context` / `in->context = context` with `void *context`;
  their only common leaf is the access refusal of that store; rule 70 as
  above. CG10-MEMBER-STORE-RETAINED-BY predicted no system or slice move
  without measuring either route.
* AG2's unpredicted rows (corpus 4, system 5) were each checked against
  base diagnostics and source in its report (rule 62); K6's
  `rd_kafka_msgq_allow_wakeup_at` and MES's `ctr_link_set_trace_state` in
  the round-37 landing note.

Rows resting on declarations not yet validated against a body, which the
verdict contract allows only with the assumption published (each row's
`declarations` and `hypothesis_context` carry it): the ten unpack_* rows
(CG10-OOM-DECLARATIONS-UNVALIDATED), CG10 round 4's seven
(CG10-R4-NEUTRAL-DECLARATIONS-UNVALIDATED) and RK-TOPIC's two
(RK-TOPIC-PARTITION-CONSTRUCTORS-UNVALIDATED).

**NULL property.** `unittest_list_to_map`, `unittest_set_intersect`,
`unittest_set_subtract` (`rdkafka_cgrp.c`) and
`rd_kafka_mock_cgrp_consumer_member_needs_assignment` go NULL Unknown ->
Safe in common->tip (corpus and twelve-home), every newly proved check
citing `result:declared:rd_kafka_topic_partition_list_new`,
`result:declared:rd_kafka_topic_partition_new` or a callee summary
returning the former
(`rd_kafka_mock_cgrp_consumer_member_target_assignment_assignable`,
`rdkafka_mock_cgrp.c:1075`): RK-TOPIC-PARTITION-LIST-CONSTRUCTORS'
non-null result declarations, which its custody block does not predict.
`_new` validates against its body (W4) and is published as no assumption;
`_list_new` does not (RK-TOPIC-PARTITION-CONSTRUCTORS-UNVALIDATED), and
`unittest_list_to_map`, which uses its result directly, publishes it
(`declared:rd_kafka_topic_partition_list_new: a non-null result (resource
contract)`). Their custody (`combined`) stays Unknown
([scripts/null_moves.py](scripts/null_moves.py),
[artifacts/null-moves.json.gz](artifacts/null-moves.json.gz)).

**HELD: `rdkafka_mock_cgrp.c::rd_kafka_mock_cgrp_consumer_member_needs_assignment`,
NULL property, corpus and twelve-home.** Its new proof at `:1218` is
`result:summary:rd_kafka_mock_cgrp_consumer_member_target_assignment_assignable`,
whose `returns_non_null: true` comes from `ret =
rd_kafka_topic_partition_list_new(…)` (`:1075`), the unvalidated declaration.
Neither row publishes that assumption: the callee's NULL record lists only
`inferred … #member` and no dependency on `_list_new`, the caller's the
same. The direct use publishes it and the use through a summary does not,
so this `Safe`'s assumption set is incomplete as published (hard invariant
2). The cause is known; the evidence the verdict contract requires is not
on the row. Follow-up P0-NULL-SUMMARY-DECLARED-ASSUMPTION.

**Accepted (USER DECISION 2026-11-26, verbatim: "Accept it").** The row is
accepted as `Safe` resting on the known `rd_kafka_topic_partition_list_new`
non-null declaration (RK-TOPIC-PARTITION-LIST-CONSTRUCTORS, unvalidated:
RK-TOPIC-PARTITION-CONSTRUCTORS-UNVALIDATED), while that assumption is still
not published through the callee summary. It is no longer held for the
Priority 0 reconciliation; P0-NULL-SUMMARY-DECLARED-ASSUMPTION stays open to
publish the declaration on rows that rest on it through a summary. Custody
figures are unaffected.

No other new Safe is held: every custody `Safe` above has its cause and an
empty diagnostic list, and the other three NULL `Safe` rows rest on the
validated `_new` or publish `_list_new`.

## Safe -> Unknown

| Row | Routes | Card | Verdict |
|---|---|---|---|
| `rdregex.c::rd_regex_match` | corpus, twelve-home | CG25-IN-PLACE-REGEX (rule 11v (7) generalised) | **Justified soundness repair**, predicted: `regerror(r, &re, …)` on the failure edge is a use of an uncertified in-place object; the over-refusal is stated in its prediction |
| `flb_output_task_queue_enqueue`, `flb_sched_timer_cb_destroy` (both configurations) | system | LERI-HELD-AND-LINK-MEMBER-PREMISES (rule 46 (3)) | **Justified tightened reading**, predicted: the held arm with a formal now checks P-LERI-HELD-LINKS from the body's own initialisation, which these bodies do not perform |
| `mk_sched_threads_destroy_conn`, `sched_thread_cleanup` | corpus, twelve-home | RULE50-CONSUMER-COMPOSITION on these inputs | **Justified fail-closed reading of the pinned inputs**, unpredicted: rule 50 composes `mk_http_thread_destroy`'s helper `mk_thread_destroy` at its consumers over the run's set of `mk_thread.cb_destroy`; here 34 units of the run are unread because `build/cycle19/c-gateway-wave-pin/fluent-bit-clang-production/lib/luajit-cmake/lj_vm.S` is absent from the prepared tree (34 or 35 unread units per member on all three sides; 205 such leaves at prev), so the set is not closed (`callback-member-open`). The card measured a 4,704-row corpus whose run read every unit. The condition is MEMBER-PREPASS-ASSEMBLY-UNITS's, which names these routes' `lj_vm.S` |

**Cancelled inside step 1** (net Unknown; invisible to any comparison of
the sides): `mk_sched_threads_purge` and `mk_sched_threads_destroy_all`
went Unknown -> Safe in LIST-ENTRY-RELEASE-IDENTITY (r9; its record
`artifacts/native-comparison-corpus.json` measured both; checkpoint
`0f6e17118` stands at prev + 2) and back to Unknown when RULE50's landing
withdrew `sched_thread_cleanup`'s summary (their leaf at prev and tip alike:
`call to uncontracted function sched_thread_cleanup`). With the two rows
above this is the corpus's 1,082 -> 1,078 between D3's tip `8e190aa83` and
the CG25 base `36b2cf931` (four Safe rows to Unknown, RULE50's merge
`5b5ff2912` an ancestor of the latter and not the former), and the "one
back" before the AG base `a9c6dcf44` is `rd_kafka_ApiVersion_check`
(CG22-CALLBACK-CANDIDATE-CONTRACTS, `05085afa1`, an ancestor of
`a9c6dcf44` and not of `5c824e443`): the two corpus gaps
LIVE-ROUTE-MOVES-BETWEEN-MEASURED-LANDINGS records. **No Safe -> Unknown
regression is held.**

## Provisional Unsafe (reported separately; none is an upstream defect)

| Row | Routes | Step | Card | Reading |
|---|---|---|---|---|
| `rdunittest_acknowledge.c::ut_case_acknowledge_accept`, `ut_case_acknowledge_multiple_partitions` and the unit | corpus, twelve-home | 1 | AG1-INDIRECT-MEMBER-ACCESS (predicted) | `ut_ack_create_message` now validated as returning an owned message; the leaks are at `RD_UT_ASSERT` early returns (`:264`, `:533`, `:534`) before `ut_ack_destroy_message`. Each failing condition calls the uncontracted `ut_ack_get_type`, so the path's feasibility is not validated: reachable only when the unit test itself fails. **Not validated** |
| `rdkafka_sticky_assignor.c::ut_testSameSubscriptions` and the unit | corpus, twelve-home | 2 | RK-TOPIC-PARTITION-LIST-CONSTRUCTORS (measured, reported) | `subscription` from the declared `rd_kafka_topic_partition_list_new` leaks at two `RD_UT_ASSERT(!err, …)` and two `verifyValidityAndBalance` returns (`:2999`-`:3014`); the conditions call the uncontracted `verifyValidityAndBalance0`. A test-failure path. **Not validated** |
| `rdkafka_sasl_scram.c::rd_kafka_sasl_safe_string` and the unit | corpus, twelve-home | 2 | LATER-DECLARATOR-INITIALISER-UNREAD (report fixture) | **Artefact.** `char *safe = NULL, *d = NULL` is now read one per statement, which removes the refusal that kept the row Unknown; the loop `for (pass = 0; pass < 2; pass++) … if (pass == 0) d = safe = rd_malloc(len + 1);` is not correlated with `pass`, so a path allocating twice over a held `safe` is kept. In C the allocation happens once and `safe` is returned. The same row was the CURRENT-UNSAFE-TRIAGE modelling defect that CHAINED-ASSIGNMENT-INNER-DESTINATION withdrew to Unknown at `556460e31`; it is back as Unsafe through another reading. Tracked by PASS-CORRELATED-LOOP-ALLOCATION |
| `plugins/in_storage_backlog/sb.c::sb_remove_chunk_from_segregated_backlog` and the unit | system | 2 | NULLABLE-GUARDED-RELEASE (unpredicted; placed by its records) | below |

`sb_remove_chunk_from_segregated_backlog(target_chunk, backlog, destroy)`
(`sb.c:214-246`, the system route's tree) walks `backlog->chunks`, and for
the matching wrapper calls `mk_list_del(&chunk->_head)` (`:226`, declared
`resource_unpublished(entry)`: the unlinked wrapper becomes this body's
obligation), adjusts the size accounting, `sb_destroy_chunk(chunk)` only
`if (destroy)` (`:240-242`), and breaks. At common its leaves were
`unresolved resource identity chunk` and `nullable resource chunk used by a
non-null contract` at `:241`; rule 74c's member-address frame reads
`&chunk->_head` as an address inside `*chunk`, so `chunk` stays resolved
through the unlink and the release, and the `destroy == 0` exit keeps the
unlinked wrapper live: `leak: live obligation acquired at sb.c:226`. The
move is placed in NULLABLE-GUARDED-RELEASE's landing by the records:
system stands at 114 / 330 / 4 at SP5's tip `38cf960fa` and at
114 / 329 / 5 at SP6's base `1f2007667`, and the only code commits between
them are that card's (`4d5325d05`, `686ddf8f4`).

Reachability: the function is `static`, and its only call
(`sb_remove_chunk_from_segregated_backlogs`, `sb.c:258`) passes
`FLB_TRUE`; the leak needs `destroy == 0`, which no call in the program
supplies. The verdict is the function's own scope with `destroy`
unconstrained, so it is **not a validated reachable violation**: it is
provisional, and on the chunk-lifecycle path it says only that the
false arm of `destroy` would leak the wrapper. The declared specification
of this function (`storage_backlog.qv`, body-checked) stays withdrawn
(`not established`), as at common. Follow-up
P0-SB-REMOVE-CHUNK-DESTROY-FLAG.

The one provisional Unsafe that **left**: `rdhttp.c::unittest_http_get_params_append`
and its unit, Unsafe -> Unknown in step 2, placed in the same landing by
the corpus figures (Unknown 3,672 -> 3,673, Unsafe 12 -> 11 between the
same two checkpoints). Its leak (`rk`, acquired at `rdhttp.c:802`, live at
the `RD_UT_ASSERT` return `:805`) is withdrawn by rule 74c (A) beside the
row's Unknown leaves; fail-closed, and the class
SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN records (a leak on a path avoiding
the refused release is withdrawn with it).

Provisional Unsafe totals at tip: corpus 13 function / 10 unit rows,
system 5 / 3, slice 1 / 1; none is filed or presented upstream.

## Evidence-only rows

Rows whose verdict did not move but whose evidence did (leaves added or
removed, contract dependencies, NULL premises, hypothesis atoms, other
fields): corpus 1,495, twelve-home 1,498, system 131, slice 91 in the chain
(a row is counted once per step it changes in). Each step's change is
classified by [scripts/evidence_classes.py](scripts/evidence_classes.py)
([artifacts/evidence-classes.json.gz](artifacts/evidence-classes.json.gz)),
in this order of strength:

* `record:` the same row changes in a committed pinned landing record
  inside the step (same inputs);
* `decl:` / `atom:` / `text:` an added declaration, hypothesis atom or
  leaf text that did not exist at the step's start and was introduced
  inside it by the named commit (`git grep` at both ends, `git log -S`
  for the commit; source and contract trees only);
* `cascade`: a summary dependency added or withdrawn, which follows the
  callee's own row;
* `note:` a leaf tag or field the named card's own report gives as its
  effect (`NOTE_MAP`, step 2 only; weaker, stated as such).

| Route | Step | Rows with an evidence change | Attributed | Unattributed |
|---|---|---|---|---|
| corpus | prev->common | 817 | 812 | 5 |
| corpus | common->tip | 710 | 518 | 192 |
| slice | prev->common | 54 | 37 | 17 |
| slice | common->tip | 35 | 11 | 24 |
| system | prev->common | 97 | 61 | 36 |
| system | common->tip | 32 | 8 | 24 |
| twelve-home | prev->common | 820 | 815 | 5 |
| twelve-home | common->tip | 710 | 518 | 192 |

Causes, step 1 (rows; a row may have several):

| Cause | corpus | twelve-home | system | slice |
|---|---|---|---|---|
| record:ag1-indirect-member-access+combined | 805 | 808 | 47 | 35 |
| record:ag3-condition-return-calls | 495 | 495 | 35 | 29 |
| record:ag2-unresolved-alias | 304 | 309 | 15 | 6 |
| record:ag1-indirect-member-access | 77 | 77 | 0 | 0 |
| cascade | 54 | 56 | 9 | 0 |
| unattributed | 5 | 5 | 36 | 17 |
| decl:368a56875 CURRENT-CONTEXT-NEW-TRIAGE D3 [f0d91bf51] | 4 | 4 | 0 | 0 |
| record:current-context-new-triage+d3 | 4 | 0 | 10 | 2 |
| atom:b19659a5a WIP rule 50 reader, loader placement and declarations [893181cc4] | 1 | 1 | 0 | 0 |
| decl:b19659a5a WIP rule 50 reader, loader placement and declarations [893181cc4] | 1 | 1 | 0 | 0 |
| record:thread-local-custody-free-read | 0 | 0 | 4 | 0 |
| text:875d5e6d4 LERI-HELD-AND-LINK-MEMBER-PREMISES [01a7e8084] | 0 | 0 | 0 | 1 |

Causes, step 2 (rows; a row may have several):

| Cause | corpus | twelve-home | system | slice |
|---|---|---|---|---|
| unattributed | 192 | 192 | 24 | 24 |
| decl:e03c4835f RK-TOPIC-PARTITION-LIST-CONSTRUCTORS [3c79b276c] | 147 | 147 | 0 | 0 |
| text:62f63371a NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP [663892ace] | 105 | 105 | 0 | 3 |
| record:r34-k-repair | 66 | 66 | 2 | 0 |
| note:K2-NO-TYPED-ACCESS / K6-DIRECT-POSITION-ACCESS (reading-site declaration-reader digest re-pinned for rules 79/81) | 54 | 54 | 0 | 0 |
| cascade | 39 | 39 | 4 | 4 |
| note:CG10-EMBEDDED-LIST-BORROWED-CONTAINER (r45 note: access leaves become retention-undecided) | 33 | 33 | 0 | 0 |
| note:CG10-MEMBER-STORE-RETAINED-BY / MEMBER-EFFECT-STORES (rules 70, 38k) | 29 | 29 | 0 | 0 |
| note:CG10-EMBEDDED-LIST-BORROWED-CONTAINER (r45 note: access leaves become callback-registry-undecided) | 27 | 27 | 0 | 1 |
| record:k6-direct-position-access | 27 | 27 | 0 | 0 |
| decl:bedc1a5c1 Rule 72 [02ff22744] | 22 | 22 | 0 | 0 |
| decl:e9aeaa511 Declare rd_jitter and the xxHash finalizers; gateway rule 66 [02ff22744] | 22 | 22 | 0 | 0 |
| note:OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY | 21 | 21 | 0 | 3 |
| note:MEMBER-EFFECT-STORES (rule obligations of rule 38k) | 7 | 7 | 2 | 0 |
| record:member-effect-stores | 7 | 7 | 2 | 0 |
| note:CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT (rule 72) | 6 | 6 | 0 | 0 |
| note:NULLABLE-GUARDED-RELEASE (rule 74c member-address frame) | 6 | 6 | 0 | 0 |

Classes of the unattributed rows (rows per class):

| Route | Step | Class | Rows |
|---|---|---|---|
| corpus | 1 | leaf+ unsupported | 2 |
| corpus | 1 | field rule_obligations | 2 |
| corpus | 1 | leaf- unknown_alias | 1 |
| corpus | 2 | leaf- unknown_alias | 140 |
| corpus | 2 | leaf- unsupported | 76 |
| corpus | 2 | field null_premises | 27 |
| corpus | 2 | dep-removed null_premises | 26 |
| corpus | 2 | leaf+ unsupported | 18 |
| corpus | 2 | leaf+ unknown_alias | 7 |
| slice | 1 | leaf+ unsupported | 12 |
| slice | 1 | leaf- unknown_alias | 5 |
| slice | 2 | leaf- unknown_alias | 21 |
| slice | 2 | leaf- unsupported | 4 |
| slice | 2 | dep-removed null_premises | 3 |
| slice | 2 | field null_premises | 3 |
| slice | 2 | leaf+ unknown_alias | 1 |
| system | 1 | leaf+ unsupported | 24 |
| system | 1 | leaf- unknown_alias | 12 |
| system | 2 | leaf- unknown_alias | 24 |
| system | 2 | leaf- unsupported | 11 |
| system | 2 | leaf+ unknown_alias | 5 |
| system | 2 | dep-removed null_premises | 4 |
| system | 2 | field null_premises | 4 |
| system | 2 | leaf+ unsupported | 4 |
| twelve-home | 1 | leaf+ unsupported | 2 |
| twelve-home | 1 | field rule_obligations | 2 |
| twelve-home | 1 | leaf- unknown_alias | 1 |
| twelve-home | 2 | leaf- unknown_alias | 140 |
| twelve-home | 2 | leaf- unsupported | 76 |
| twelve-home | 2 | field null_premises | 27 |
| twelve-home | 2 | dep-removed null_premises | 26 |
| twelve-home | 2 | leaf+ unsupported | 18 |
| twelve-home | 2 | leaf+ unknown_alias | 7 |

**Unattributed: 495 row-steps** (corpus 197, twelve-home 197, system 60,
slice 41), listed by row and class in the artifact. They are
overwhelmingly removals of generic leaves (`unresolved resource
identity`, `indirect memory access …`) and NULL-premise id changes in
step 2, whose cause sits in one of the r41-r73 landings that did not
measure the pinned routes; a removal names no rule, so no record or text
can tie it to one of them. None changes a verdict, and every row of them
keeps its status at all three sides. Follow-up P0-EVIDENCE-ONLY-UNATTRIBUTED
(discharge: a pinned measurement per unmeasured landing).

## G1 (certificate containment)

No record states G1 closed. `docs/REEVALUATION_SPEC.md` §9 defines G1 as
"WM01–WM03 and VP01–VP02 fixes; explicit profile restrictions" with the
criterion that invalid candidates and insufficient evidence cannot obtain
the affected certificates while positive controls survive. Its own "Six-
finding repair on 2026-09-05" records the five fixes (WM01-WM03 "fixed in
the represented atomic subset" / "for represented loads/stores"; VP01-VP02
certificate issuance contained), and every landing gate since runs
`reevaluation_probe` with exit 0 (`failed_expectations: 0`, the five
probes and both controls passing). No record evaluates the "explicit
profile restrictions" deliverable or declares the gate met, and the spec
itself lists capability-specific witnesses and third-party `Unsafe`
witness validation as open. G1 is therefore **not claimed**. Follow-up
P0-G1-CLOSURE-RECORD.

## Publication consequence

`land.py publish --previous 556460e31` cannot run here: this host has no
`private/baselines/556460e31` (the 20 baselines under the main checkout's
`private/baselines/` do not include it). The publication of the tip's
figures above stays owed; this page is its row-by-row account. Nothing
here changes a profile baseline.

## Tool findings (QUIRIN-AUDIT-CONNECT, not fixed here)

* `--declared` refuses a PREDICTION.md with more than one
  `measure-prediction` block (`exactly one is read`). Two committed files
  have two: CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3 (a revision) and
  MEMBER-EFFECT-STORES (a block written after its landing measurement,
  "for the record only"). They were split verbatim into
  [chain/predictions/](chain/predictions/), round 3's both blocks and
  MES's first only. Follow-up CONNECT-DECLARED-REVISION-BLOCKS.
* The chain reconciles the resource-custody rows only; the NULL
  document's moves above were found by
  [scripts/null_moves.py](scripts/null_moves.py). Follow-up
  CONNECT-CHAIN-NULL-PROPERTY.
* Records measured at other bases and binaries never join, so the chain
  sees no cancellation inside a step; the LIST-ENTRY/RULE50 pair was found
  through the records' side figures. Follow-up
  CONNECT-CHAIN-FIGURE-CHECKPOINTS.
* A unit row attributed by derivation carries no source label
  (`derived_from_function_rows` only), and a `--runs` chain over the four
  pinned routes peaked at 1.7 GiB resident against the `short` lane's
  1 GiB admission budget (receipt in the run's slot record). Noted in
  CONNECT-DECLARED-REVISION-BLOCKS.

## Records

* [chain/](chain/): the chain record (gzip; content and archive sha256 in
  [artifacts/SHA256.json](artifacts/SHA256.json)), its argv, the plugin
  report, stdout and exit, and the split prediction blocks.
* [artifacts/](artifacts/): the landing records' placement, the moves
  table, the evidence classes, the new-Safe evidence and the NULL moves.
* [scripts/](scripts/): every script that produced them, read-only on the
  measurement.
* [integration-amendments.json](integration-amendments.json): follow-ups.
