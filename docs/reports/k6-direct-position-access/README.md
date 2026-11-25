# K6-DIRECT-POSITION-ACCESS: direct-position access, re-censused

Card K6 of round 37 (brief `private/integrator-briefs/r37-k246-common.md`), base
`5087f9295` (origin/main, round 34 landed). Bundle: an expressible typed access
path directly in a statement, return or condition (no call actual) that the typed
layer refuses for its root premise or value form (census tags
`expr:statement:direct`, `expr:cond:direct`, `expr:return:direct`); owners
W47-ACCESS-PATH-POSITION-RESIDUALS (primary), CG10-BOUND-ROOT-PATH-REFUSED,
CG10-BOUND-ROOT-NULLABLE-OR-TRACKED; rules 10k, 10n; census draft
[predictions/K6.md](../current-access-alias-census/predictions/K6.md).

## 1. Re-census (11-07 vs 11-24)

Source: the user's `current` report written 2026-11-24 09:58 (Fluent Bit
`046accf6`, 1,173 Safe / 3,750 Unknown / 11 Unsafe rows; read-only), joined to
CURRENT-ACCESS-ALIAS-CENSUS's 11-07 leaf tags by (file, function, `-E -P` line,
family, sub) with K1's join ([scripts/recensus.py](scripts/recensus.py), the K1
script with the bundle set to the three `direct` tags;
[census/recensus.json](census/recensus.json)). 11,541 of 12,259 IND/COND/RET leaf
groups and 9,420 of 10,108 alias leaves join; an unjoined group carries `new` and
is never counted removable. [scripts/sites.py](scripts/sites.py) lists every K6
leaf group with its origin text ([census/sites.jsonl.gz](census/sites.jsonl.gz)).

| Measure | 11-07 (base `7b726587b`) | 11-24 (user's run) |
|---|---:|---:|
| function rows / Unknown | 4,912 / 3,832 | 4,934 / 3,750 |
| K6 leaves | 1,571 | 1,395 (statement 875, condition 465, return 55) |
| Unknown rows touched | 769 | 671 |
| rows freed alone / distinct functions | 21 / 21 | **25 / 25** |
| freed alone by `statement` / `cond` / `return` alone | 6 / 1 / 14 | 8 / 2 / 15 |

The 21 functions of the 11-07 frozen list now: **0 Safe**, **20 still freed
alone**, 1 (`mk_plugin_time_now_human`) whose return leaf moved line in the
`--monkey-linked` text (`new`, not a semantic change). Five functions joined
the freed-alone set since 11-07 (their other blockers went):
`LZ4HC_init_internal`, `rd_kafka_ListConsumerGroups_result_errors`,
`rd_kafka_topic_partition_topic`, `rd_kafka_sasl_cyrus_cb_canon`,
`sort_by_map_elem_val_toppar_list_cnt`.

The 11-24 run predates round 34's landing (rules 75, 77, 78, 10v amended), so
the 25 rows are re-read at the base `5087f9295` before anything is counted
(section 2).

### 1.1 What earlier cards refused on purpose

Nothing below is re-admitted by this card.

- **An interior address returned** (array decay, `&B->F`, `&B->A[I]`, a pointer
  offset): W45-RETURN-EXPRESSION-FAMILIES and rule 10k (2) ("an array member
  returned by decay is an interior address and refuses (rule 5j)"), rule 10n
  (1) ("an address is never returned"), AG3 (`AG3-RETURN-MEMBER-ADDRESS-DERIVED`),
  rule 77 ("do not admit ... an address or an array decay as a value"). An
  interior address is a result derived from `B`; published as a result that
  carries nothing it would let a caller release `B` and keep using the result
  unaccused. Its discharge is a `Derived { parameter, extent }` result through
  the derived-summary route, which is that owner's card, not this one.
  **11 of the 25 rows**: `rd_kafka_name` (`rk->rk_name`, `char[128]`),
  `rd_kafka_ConfigEntry_name` (`entry->kv->name`, `char name[1]`),
  `rd_kafka_share_partition_offsets_offsets` (a flexible array member),
  `rd_kafka_MemberDescription_assignment`, `rd_kafka_TopicDescription_topic_id`,
  `rd_kafka_conf`, `mk_plugin_time_now_human` (`&B->F`),
  `rd_kafka_UserScramCredentialsDescription_scramcredentialinfo`,
  `rd_kafka_mock_partition_find` (`&B->A[I]`), `cJSON_GetErrorPtr`
  (`global_error.json + global_error.position`) and `extra_alloc`
  (`p = rbuf->rbuf_extra + of; ... return p;`).
- **A pointer difference or an offset outside an indirection**: rule 78 reads
  `*(P + I)` only and says not to read "a difference of two pointers"
  (`LZ4HC_init_internal`'s `(size_t)(hc4->end - hc4->prefixStart)`).
- **A sibling call beside a deciding operand** (rule 10z (3)):
  `rd_kafka_broker_weight_usable`'s `(rd_clock() - (tx_last > 0 ? tx_last : rkb->rkb_ts_state))`.
- **A record object outside automatic storage** (rule 12j: "do not read ... a
  `static`, `extern`, `register`, thread-local or file-scope root"):
  `clear_unit_test_data`, `unit_test_telemetry_decoded_int64` store scalar
  members of the file-scope `static struct ... unit_test_data`.
- **A whole read of a formal copied into a local** (rule 77, rule 62's copy):
  `rd_kafka_topic_partition_topic`'s `rktp = (const rd_kafka_toppar_t *)rktpar`.
