# W37-CORPUS-CONTRACTS-4 — the ex-ante prediction

**Committed BEFORE the AFTER run was started**, so the predicted-vs-measured
comparison in the report is a real one and not a retrofit. Computed from the
BEFORE run alone, whose `diagnostics.json` and `chunk-contracts.json` reproduce
`docs/reports/joined-provenance-call-frames/artifacts/` byte for byte
(`a7aa3b5f9ab193a0` and `c4f438e4ed3dab3c`).

Wave 36's residual `W36-COVER-HIDDEN-BLOCKER` records that a sole-blocker cover
is an **upper bound, never a forecast**: a refusal reachable only once the first
blocker is removed is invisible to a leaf-based cover. This card's brief asks
for the exchange to be predicted instead of discovered, so every target
function's body was read and the forecast below states, per function, what is
expected to become reachable.

## 1. Counts

| Quantity | Predicted |
|---|---|
| Uncontracted-call leaves named by the ten, corpus-wide | **179** |
| Functions whose last visible blocker is removed (upper bound) | **11** |
| Functions predicted to actually reach `Safe` (after the exchange forecast) | **10** |
| Functions predicted to EXCHANGE for a newly reachable refusal | **1** (`pack_string`) |
| Functions predicted UNMOVED (declaration withdrawn) | **0** |
| Unit status moves | **0** |
| `leak` | **272 → 272**, pin holds |
| `summary_conflict` | **1 → 1**, no new conflict |
| `double_release` | **1 → 1** |

## 2. The exchange forecast, function by function

| Target function | Blocker declared | Forecast | Ground, read from the body |
|---|---|---|---|
| `cmetrics/cmt_decode_msgpack.c :: unpack_meta_ver` | `cmt_mpack_consume_uint_tag` | **LEAF-FREE** | `:1417` is a plain assignment statement `result = cmt_mpack_consume_uint_tag(reader, &value);`. The only argument of interest is `&value`, the address of a **scalar** local — constraint 10's declared-scalar exclusion, not a tracked identity. No nested call, no call in a return expression. |
| `cmetrics/cmt_encode_cloudwatch_emf.c :: pack_cmetrics_type` | `mpack_write_cstr` | **LEAF-FREE** | Eight plain statements, `:96`–`:116`, every argument a formal pointer or a string **literal**. Nothing nested. |
| `cmetrics/cmt_encode_prometheus_remote_write.c :: check_staled_timestamp` | `cmt_metric_get_timestamp` | **LEAF-FREE** | `:737` plain assignment from a modeled call with a **scalar** result; both `return` statements are call-free arithmetic. |
| `cmetrics/cmt_metric.c :: cmt_metric_set` | `cmt_metric_set_double` | **LEAF-FREE** | `:68`, a one-statement body forwarding its own three formals. |
| `ctraces/ctr_encode_msgpack.c :: pack_double` | `mpack_write_double` | **LEAF-FREE** | `:47`, one plain statement, two formals. |
| `ctraces/ctr_encode_msgpack.c :: pack_int64` | `mpack_write_i64` | **LEAF-FREE** | `:42`, one plain statement, two formals. |
| **`ctraces/ctr_encode_msgpack.c :: pack_string`** | `mpack_write_str` | **EXCHANGE** | `:37` is `mpack_write_str(writer, str, cfl_sds_len(str));` — **`cfl_sds_len` is a real external function nested inside the modeled call's argument list**. Its baseline leaf set is the single `mpack_write_str` row: the nested call emits nothing today *because the outer call refuses first*. Once the outer call is modeled the projection reaches the inner one. This is exactly `CJSON-DELETE-NESTED-DETACH`'s shape, which cost W36 three of its 23. |
| `ctraces/ctr_span.c :: ctr_span_end` | `cfl_time_now` | **LEAF-FREE** | `:272` plain assignment from a zero-argument modeled call; the following `ctr_span_end_ts(ctx, span, ts)` is a second call but is **already decided** (it is defined in this same unit and W4 derives it), which is why the baseline shows this function blocked by `cfl_time_now` alone. |
| `librdkafka/lz4.c :: LZ4_resetStreamState` | `LZ4_resetStream` | **LEAF-FREE** | `:2709` plain statement; its argument is a **cast** of a `void *` formal, not a call. Residual risk noted: a cast in argument position is a shape this forecast has no prior measurement for. |
| `librdkafka/rdkafka_partition.c :: rd_kafka_topic_partition_list_sort_by_topic` | `rd_kafka_topic_partition_list_sort` | **LEAF-FREE** | `:3679` plain statement; arguments are a formal pointer, a **function designator** (typed `fnptr`, `msgpack_packer_init`'s precedent) and `NULL`. |
| `librdkafka/rdkafka_partition.c :: rd_kafka_topic_partition_list_sort_by_topic_id` | same | **LEAF-FREE** | `:3685`, identical shape. |

## 3. Two named risks, stated as risks rather than buried

1. **`cfl_time_now`'s unprototyped header.** `lib/cfl/include/cfl/cfl_time.h:25`
   spells `uint64_t cfl_time_now();` — an empty parameter list, which in C is
   *unspecified parameters* rather than arity zero. If constraint 8's arity gate
   read that as a contradiction the declaration would be **withdrawn** and
   `ctr_span_end` would be UNMOVED. The forecast says LEAF-FREE because the
   corpus already carries `flb_time_now` against the identically-shaped
   `FLB_EXPORT double flb_time_now();` (`include/fluent-bit/flb_lib.h:94`),
   manifest-selected and not withdrawn — precedent, measured rather than hoped.
2. **Six of the ten are defined in analysed units** (`cmt_metric.c`,
   `cmt_mpack_utils.c`, `lz4.c`, `rdkafka_partition.c`), so constraint 8's duty
   gate applies and a `SummaryConflict` is possible — the `rd_kafka_mem_free`
   shape that cost W36 a `Safe`. The forecast says **no new conflict**, on a
   structural ground rather than optimism: every one of the ten is the
   **neutral element** (`N(m)`: every formal `Borrowed`, result `None`), the
   weakest statement the vocabulary can make, so a derived summary can
   contradict it only by claiming a formal is released, transferred, delivered
   or restored — and none of these bodies performs any such operation on any
   parameter.

## 4. What is deliberately not predicted

The number of **added** leaves. W36 measured 128 added against 217 removed, the
`unknown_alias` rise being the mechanism working (modelling a call makes later
mentions of its operands unresolved rather than unexamined). Every declaration
here is custody-neutral and releases nothing, so the `unknown_alias` growth
should be far smaller than W36's +94 — but "should be smaller" is an
expectation, not a forecast, and it is recorded as the former.
