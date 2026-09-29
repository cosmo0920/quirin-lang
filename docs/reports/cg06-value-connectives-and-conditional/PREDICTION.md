# CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: frozen prediction

Frozen before the first AFTER measurement of this card, from CG00's frozen
prediction ([`predictions/CG06.md`](../cg00-clang-gap-baseline/predictions/CG06.md),
corpus `ae17152b…`), re-derived on this card's BEFORE and amended by reading
each candidate's source. Upper bounds, as CG00's are.

## Re-derivation on the BEFORE

BEFORE: the corpus arbiter on base `27dc7c9a` (`resource-diagnostics.json`
`7393fd21…`, byte-identical to CG03's archived AFTER; 564 Safe / 10
provisional Unsafe / 4,260 Unknown). `scripts/rederive.py` →
[`artifacts/rederive-before.json`](artifacts/rederive-before.json):

- CG00's 522 E06/E07 functions are all still Unknown; 2,052 of its 2,077
  E06/E07 leaves are still present at the same internal line and description
  (25 moved under earlier wave-48 cards).
- All 40 functions CG00 predicted freed alone are still Unknown, each with its
  CG00 leaf; none has moved. `ConsumerPair_hash` still carries its
  `unknown_alias` leaf.

## The design this prediction assumes

Route G only (T: no typed-source consumer decides a value connective, CG01's
census; T is unchanged). A statement or `return` every older route refused is
lowered by branching on each value region's deciding operand, the `if`
statement's own condition routes deciding it, and each edge's expression
(the region replaced by the arm C evaluates there) lowered by the position's
older routes. A leaf those routes refuse declines the whole expression. Order
premise: every sibling of a region is effect-free, and a deciding operand with
a call has only inert siblings. At most 8 leaves.

## Amended per-function prediction (40 CG00 functions)

Amendments are by reading; each names its second cause.

**Safe (upper bound), 19** — whole-value returns and scalar stores whose
leaves the older routes already decide as genuine statements:

- `cJSON_HasObjectItem` (`return CALL ? 1 : 0`)
- `rd_kafka_path_is_dir` (`return CALL(path, &out) && is_dir`)
- `rd_kafka_broker_destroy_error` (`return CALL ? K1 : K2`)
- `rd_kafka_broker_needs_connection`, `rd_kafka_broker_needs_persistent_connection`
  (`&&`/`||` chains of reads and calls)
- `rd_kafka_conf_validate_partitioner` (`||` of `!strcmp`)
- `rd_kafka_error_name`, `rd_kafka_error_string` (`p ? CALL(p->f) : ""`)
- `rd_kafka_toppar_fetch_decide_start_from_next_fetch_start`
- `rd_kafka_share_acknowledgement_mode_is_explicit`, `…_is_implicit`
- `get_client_rack`, `get_group_id`, `get_group_instance_id`,
  `get_transactional_id` (pointer `?:` returns of a member read and `NULL`)
- `rd_kafka_sasl_plain_conf_validate` (`both_set = P->a && P->b`)
- `ut_get_consumer_rack` (scalar `?:` declaration of constants)
- `log2_floor` (`return n == 0 ? -1 : 31 ^ rd_clz(n)`)
- `rd_kafka_buf_calc_timeout` (`RD_MIN` store into a member)

**Safe only if a call actual's typed read is served on a branch edge, 8** —
the region is a whole call actual whose arm is a member read; the leaf is a
call statement the older routes decide only through the typed actual reader,
which matches the actual's Clang node:

- `rd_kafka_log_print`, `rd_kafka_queue_forward`,
  `rd_kafka_MemberDescription_copy` (a return of the call),
  `rd_kafka_msgset_writer_compress_lz4`, `ConsumerPair_cmp`,
  `rd_kafka_txn_schedule_register_partitions`, `rd_kafka_mk_brokername`
  (string-literal arms only: lexical), `rd_kafka_ConfigResource_cmp`
  (`RD_CMP` declaration read from its node).

**Unknown, 13** — a second cause this card does not own:

- `rd_kafka_group_result_copy`, `rd_kafka_interceptor_failed`,
  `XXH_readBE32`, `XXH_readBE64`: a nested call in an arm or actual (CG05).
- `rd_kafka_outq_len`, `rd_kafka_AclBinding_request_size`: calls unsequenced
  with the region (siblings with effects; the latter also 16 leaves).
- `LZ4HC_countBack`: pointer subtraction in the region's operands (CG16).
- `rd_kafka_event_name`: the region is a `switch` expression, a controlling
  expression (the condition CFG's, which refuses a non-leaf switch).
- `rd_kafka_idemp_drain_toppar`, `rd_kafka_toppar_reset_base_msgid`,
  `ut_build_record_v2`, `rd_kafka_mock_msgset_est_record_size`: the region is
  nested inside a macro-expanded actual or an arithmetic operand of a call or
  store (`RD_KAFKAP_STR_PR`, `RD_KAFKAP_BYTES_LEN`), where the leaf is a
  synthesized statement no typed route matches.
- `ConsumerPair_hash`: its `unknown_alias` leaf stays.

**Predicted corpus figures:** Safe +19 to +27 (upper bound), no Safe →
non-Safe move, no new provisional Unsafe. Functions outside CG00's list may
move where a whole-value region was their only refusal; each is attributed
by reading in the report.
