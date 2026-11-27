# CG04-CONDITION-ACQUISITION: frozen prediction

Re-frozen from CG00's [`predictions/CG04.md`](../cg00-clang-gap-baseline/predictions/CG04.md)
(corpus `ae17152b…`, 526 / 10 / 4,298) on this card's base, local `main`
`27dc7c9a`. The BEFORE corpus run on that base reproduces CG03's archived AFTER
byte for byte (`resource-diagnostics.json` `7393fd21…`, 564 / 10 / 4,260).

**Freezing point.** Committed after the first implementation commit
(`3325d36f`: the split use, per-leaf type operators and increments, the Lean
model) and before any corpus run of an implementation build. The brief asked
for the freeze before implementing; this one follows a WIP commit, and no
AFTER measurement had been read when it was written.

## Re-derived function list

[`scripts/rederive.py`](scripts/rederive.py) over CG00's 31 functions and this
base's BEFORE:

- **Already moved before this card (3):** `ctr_id.c::ctr_id_cmp`,
  `rdkafka.c::rd_kafka_consumer_group_state_code`,
  `rdkafka.c::rd_kafka_consumer_group_type_code` are `Safe` on the base
  (moved by cards between `ae17152b` and `27dc7c9a`). Not attributable here.
- **28 remain Unknown.** Each was read at its source condition (CG13 showed
  CG00 labels a leaf by its first cause only). Hidden second causes:

| Group | Functions | Blocking condition | Prediction |
|---|---|---|---|
| alias leaves | `get_object_item`, `metrics_check_label_value_existence`, `pb_decode_tag`, `rd_kafka_assignor_rebalance_protocol_check`, `rd_kafka_conf_prop_find`, `rd_kafka_share_find_entry_for_offset` | `unknown_alias` leaves beside the condition | **Unknown** (6) |
| address of a place | the six `cmt_cat.c` `*_lookup`, `map_lock`, `cmt_metric_exp_hist_lock` | `&x->f` actual (`cmt_opts_compare(&counter->opts, …)`, `cmt_atomic_compare_exchange(&map->metric_lock, …)`) | **Unknown** (8): CG03's substitution names this form CG11's |
| nested call | `mk_socket_set_nonblocking` | `fcntl(fd, F_SETFL, fcntl(fd, F_GETFL, 0) \| O_NONBLOCK)` | **Unknown** (1): CG05 |
| actual forms and destinations | `filter_get_label_index`, `mk_plugin_lookup`, `pb_decode_svarint`, `pb_skip_varint`, `pb_encode`, `rd_kafka_test_fatal_error`, `rd_kafka_ApiVersion_is_queryable`, `rd_kafkap_Fetch_reply_tags_parse`, `rd_kafkap_Produce_reply_tags_parse`, `rd_kafka_topic_partition_list_set_offset`, `rd_kafka_share_find_ack_batch`, `rd_kafka_filter_broker_by_GetTelemetrySubscription`, `rd_kafka_snappy_uncompressed_length` | a place or local address actual at a borrowed or out formal (`strcmp(p->shortname, …)`, `pb_read(stream, &byte, 1)`), a typedef'd pointer destination (`!(rktpar = …find(…))`) | **Safe, upper bound 13**, only if the condition leaf takes the typed substitution rule 10u gives the statement consumer, and the destination widening below; point estimate **8** |

**So the frozen figures are:** at most 13 of CG00's 31 functions move Unknown →
Safe, point estimate 8; 15 stay Unknown for named reasons outside this card;
3 had already moved.

## Outside CG00's list

- **E09 (acquisitions).** CG00 attributed 191 leaves over 146 functions, of
  which only 3 functions are freed by CG04 alone. Most acquisitions in
  conditions sit in functions with other blockers. Predicted: at most **5**
  Unknown → Safe from the split alone, and **0** new provisional Unsafe from
  it. Any new `Leak` is read; a leak on a failure branch the source does not
  have is an artifact and blocks the card.
- **Partner functions.** CG00 counted 59 functions freed by CG04 with exactly
  one other card (CG10 32, CG08 6, CG02 4, CG07 3, CG06 3). CG02, CG03 and CG10
  have landed, so up to **36** of those (CG10 32 + CG02 4) could now move with
  this card alone; not every partner's leaves were discharged, so no point
  estimate is given.
- **Corrections.** No Safe → non-Safe move is expected. A move of that kind
  would be a named correction (for example, a call a type operand held that an
  older route applied).

## Absorbed follow-ups: expected outcome

| Follow-up | Registered size | Expected |
|---|---|---|
| `W44-CONDITION-CALL-ASSIGNMENT` | 196 leaves / 160 functions | shrinks: a Clang-typed local destination (`void *`, typedef) is admitted; formals, member and pointer stores stay (CG11) |
| `W44-CONDITION-CALL-FREE-OPERAND` | 1,211 / 622 | re-measured only; the operand routes are unchanged |
| `W44-CONDITION-CALL-TYPE-OPERATOR` | 259 / 113 | mostly discharged: a parenthesised call-free type operand is residual text; a call inside one refuses by name |
| `W44-CONDITION-CALL-REFUSED-SHAPES` | 74 / 62 | the 3 owned-result leaves and 8 increments move; member and expression callees, braces stay |
| `W44-CONDITION-DISCARDED-DERIVED-RESULT` | decision | the reading stands (a discarded derived result is a discarded `None`); closed by decision |
| `W43-INDEX-TRAVERSAL-CONDITION-WRITE` | 271 / 166 | partly: an increment or assignment in a call-free leaf the elaborator decides; `*p++` (pointer step) stays |
| `CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED` | 6 variants, 6 leaves | discharged by one test per variant |
