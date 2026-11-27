# AG2-UNRESOLVED-ALIAS: prediction

Committed after the census (`0cb1f9375`) and before the semantic commit. The
integrator measures with base `rev:a9c6dcf44` on corpus (eight-home),
twelve-home, system and slice, and reruns the user's `current` after landing.
The rows below are those of the user's `current` report of 2026-11-18; no
route was measured for this file. A unit-scoped reproduction of that run on
the 49 configurations of the alias-only rows' units at base (`quirin-audit
current` with a filtered compile database; 1,894 rows, every status equal to
the user's report, eight rows differing only in run-wide callback-closure
diagnostics) is the baseline the card's own check will compare against
(report §4).

## What the card changes

Gateway rule 62: route G's copy `D = S` from an unbound source leaves an
automatic local `D` unbound with no unresolved-identity report when `S` is a
string or character literal, an enumeration constant, or an unbound
single-level pointer formal whose copies are only read through (every copy
use a rebinding, a copy, a null test, a dereference base or a call actual,
the never-bound premise holding for the formal and every copy). Everything
else keeps the old transfer and its report.

## Per route and function

* **current** (`current --monkey-linked`): the 27 rows blocked only by S1
  move `Unknown -> Safe` where every copy use is a read-through and no read
  through the now-unbound copy is refused by another rule:
  the 15 `cmt_decode_msgpack.c` `unpack_*` decode callbacks,
  `get_context_arena`, the seven librdkafka comparators
  (`rd_kafka_assignor_cmp_str`, `rd_kafka_group_member_cmp`,
  `rd_kafka_header_cmp_str`, `rd_kafka_topic_partition_cmp`,
  `rd_kafka_topic_partition_cmp_topic`, `rd_kafka_topic_info_topic_cmp`,
  `rd_kafka_topic_partition_not_regex`), `rd_kafka_broker_start_reauth_cb`,
  `rd_kafka_conf_ssl_passwd_cb`, `rd_kafka_transport_ssl_passwd_cb` and
  `rd_kafka_mock_connection_write_out_tmr_cb`. Also
  `rd_kafka_member_assigned_partitions_pair_cmp` (its copies are S1 split over
  two lines, which the lexical census filed as S6) and
  `append_summary_metric_value` (S3). `LZ4F_optimalBSID` (S2 plus a scalar
  local returned) may move; it is not declared. Rows that stay Unknown:
  `LZ4F_getBlockSize` (its enumerator is copied into a *formal*, refused by
  rule 62 (1)), `rd_kafka_global_cnt_get` (S7), and every combination row
  with an S4/S5/S6 leaf (`format_acl_error_message`, `mk_esc_filename`,
  `rd_kafka_op_err_destroy`, `rd_buf_get_segment_at_offset`,
  `mk_utils_gen_hash` whose copy is stepped). Rows whose comparator
  argument is handed to a callee the never-bound closure does not see as
  `Borrowed` keep their report (`rd_kafka_assignor_cmp_str` hands `a` whole
  to `rd_kafkap_str_cmp_str2`): this is the declared move most at risk.
  Diagnostics: up to 347 S1, 217 S2 and 133 S3 primary leaves are removed
  where the reading holds, on rows that otherwise stay Unknown. Callers:
  a function this run newly validates publishes a `Borrowed` summary for the
  copied formal (where W4 previously bound it `Owned` or derived nothing), so
  same-unit callers and callback registrations may change content or move
  `Unknown -> Safe`; these are not enumerated. No `Safe -> non-Safe`, no
  Unsafe added or removed: the reading only removes a report and leaves a
  name unbound, and every custody use of an unbound name refuses.
* **corpus** (eight-home) and **twelve-home**: the same functions where the
  pinned source (`66910c10`) carries them, `Unknown -> Safe`, declared below
  by `file::function`; the same caution on `rd_kafka_assignor_cmp_str`.
* **system** and **slice**: not derived (no unit of these routes was
  reproduced). Fluent Bit core callbacks commonly copy a `void *` context
  (`struct flb_x *ctx = data;`), so `Unknown -> Safe` moves and content
  changes are possible there; they are not declared and would be reported as
  unpredicted.

```measure-prediction
route corpus
move cmt_decode_msgpack.c::unpack_exp_histogram_count Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_negative_bucket Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_positive_bucket Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_sum Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_sum_set Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_zero_count Unknown -> Safe
move cmt_decode_msgpack.c::unpack_histogram_bucket Unknown -> Safe
move cmt_decode_msgpack.c::unpack_histogram_count Unknown -> Safe
move cmt_decode_msgpack.c::unpack_metric_hash Unknown -> Safe
move cmt_decode_msgpack.c::unpack_metric_value_type Unknown -> Safe
move cmt_decode_msgpack.c::unpack_metric_value_uint64 Unknown -> Safe
move cmt_decode_msgpack.c::unpack_summary_count Unknown -> Safe
move cmt_decode_msgpack.c::unpack_summary_quantile Unknown -> Safe
move cmt_decode_msgpack.c::unpack_summary_quantiles_set Unknown -> Safe
move cmt_decode_msgpack.c::unpack_summary_sum Unknown -> Safe
move cmt_encode_opentelemetry.c::get_context_arena Unknown -> Safe
move cmt_encode_text.c::append_summary_metric_value Unknown -> Safe
move rdkafka_assignor.c::rd_kafka_assignor_cmp_str Unknown -> Safe
move rdkafka_assignor.c::rd_kafka_group_member_cmp Unknown -> Safe
move rdkafka_broker.c::rd_kafka_broker_start_reauth_cb Unknown -> Safe
move rdkafka_cert.c::rd_kafka_conf_ssl_passwd_cb Unknown -> Safe
move rdkafka_header.c::rd_kafka_header_cmp_str Unknown -> Safe
move rdkafka_mock.c::rd_kafka_mock_connection_write_out_tmr_cb Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_cmp Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_cmp_topic Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_not_regex Unknown -> Safe
move rdkafka_range_assignor.c::rd_kafka_member_assigned_partitions_pair_cmp Unknown -> Safe
move rdkafka_ssl.c::rd_kafka_transport_ssl_passwd_cb Unknown -> Safe
move rdkafka_topic.c::rd_kafka_topic_info_topic_cmp Unknown -> Safe
route twelve-home
move cmt_decode_msgpack.c::unpack_exp_histogram_count Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_negative_bucket Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_positive_bucket Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_sum Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_sum_set Unknown -> Safe
move cmt_decode_msgpack.c::unpack_exp_histogram_zero_count Unknown -> Safe
move cmt_decode_msgpack.c::unpack_histogram_bucket Unknown -> Safe
move cmt_decode_msgpack.c::unpack_histogram_count Unknown -> Safe
move cmt_decode_msgpack.c::unpack_metric_hash Unknown -> Safe
move cmt_decode_msgpack.c::unpack_metric_value_type Unknown -> Safe
move cmt_decode_msgpack.c::unpack_metric_value_uint64 Unknown -> Safe
move cmt_decode_msgpack.c::unpack_summary_count Unknown -> Safe
move cmt_decode_msgpack.c::unpack_summary_quantile Unknown -> Safe
move cmt_decode_msgpack.c::unpack_summary_quantiles_set Unknown -> Safe
move cmt_decode_msgpack.c::unpack_summary_sum Unknown -> Safe
move cmt_encode_opentelemetry.c::get_context_arena Unknown -> Safe
move cmt_encode_text.c::append_summary_metric_value Unknown -> Safe
move rdkafka_assignor.c::rd_kafka_assignor_cmp_str Unknown -> Safe
move rdkafka_assignor.c::rd_kafka_group_member_cmp Unknown -> Safe
move rdkafka_broker.c::rd_kafka_broker_start_reauth_cb Unknown -> Safe
move rdkafka_cert.c::rd_kafka_conf_ssl_passwd_cb Unknown -> Safe
move rdkafka_header.c::rd_kafka_header_cmp_str Unknown -> Safe
move rdkafka_mock.c::rd_kafka_mock_connection_write_out_tmr_cb Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_cmp Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_cmp_topic Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_not_regex Unknown -> Safe
move rdkafka_range_assignor.c::rd_kafka_member_assigned_partitions_pair_cmp Unknown -> Safe
move rdkafka_ssl.c::rd_kafka_transport_ssl_passwd_cb Unknown -> Safe
move rdkafka_topic.c::rd_kafka_topic_info_topic_cmp Unknown -> Safe
```
