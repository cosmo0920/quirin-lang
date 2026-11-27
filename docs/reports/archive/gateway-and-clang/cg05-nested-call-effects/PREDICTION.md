# CG05-NESTED-CALL-EFFECTS: frozen prediction

Frozen before any AFTER run, on base `27dc7c9a` (local and origin `main` at
dispatch). It re-freezes CG00's [CG05 prediction](../cg00-clang-gap-baseline/predictions/CG05.md),
amended below with reasons. The fixture probes of the new routes had run
(`tests/cg05_nested_call_effects.rs`); no corpus AFTER had.

## BEFORE

- Binary: release `quirin-audit` built at `27dc7c9a` (`d305eb08…`).
- Corpus arbiter: `private/cg05/before`, exit 0, `resource-diagnostics.json`
  `7393fd21…` = CG03's archived AFTER byte for byte; **564 / 10 / 4,260**.
- Families the rule-10y consumers are asked after
  (`scripts/families.py`, `scripts/census.py`):

| Family | Leaves | Unknown functions | Sole-blocked |
|---|---:|---:|---:|
| `nested call … inside a modeled call` | 177 | 96 | 7 |
| `resource call with a formal embedded in an unmodeled expression` | 18 | 13 | 0 |
| `return` position refusal | 253 | 201 | 66 |

The nested family's inner callees are dominated by callees with **no
contract in force** in the caller's unit: `rd_kafka_err2str` 42,
`XXH_readLE64_align` 28, `rd_kafka_msgq_len` 12, `rd_list_cnt` 8,
`pthread_getspecific` 8, `rd_kafka_error_string` 7. Those stay refused by
design (a nested call's effects must come from a contract in force).

## CG00's function list, re-derived on this base

All 23 functions CG00 predicted freed alone are still Unknown at `27dc7c9a`
(`scripts/rederive.py`, `private/cg05/rederive-before.json`); none has moved
since `ae17152b`. Each was read for its blockers (a second cause hides behind
CG00's first-cause label in 16 of them):

| Functions | Blocker read at this base | Prediction |
|---|---|---|
| `cJSON_DeleteItemFromArray`, `…FromObject`, `…FromObjectCaseSensitive` | `cJSON_Delete(cJSON_DetachItemFrom…(…))`: the inner callee's own row is Unknown (no validated summary) and undeclared | **stay Unknown** (the inner call has no contract in force) |
| `rd_kafka_srand` | `srand(rd_seed())`: `rd_seed` is in `rdrand.c`, another unit | **stay Unknown** |
| `rd_kafka_error_to_legacy` | `rd_snprintf(…, rd_kafka_error_string(error))`: inner row Unknown | **stay Unknown** |
| `rd_kafka_topic_partition_cmp_topic_id` | a `return` over `rd_kafka_topic_partition_get_topic_id`, whose row is Unknown | **stay Unknown** |
| `cio_file_native_compose_path` | a `strlen` nested leaf plus two `indirect memory access` leaves (`cio_file_native_report_runtime_error()` expands to an `errno` place) | **stay Unknown** (the strlen leaf may go) |
| `snappy.c::hash` | a `return` whose value holds `get_unaligned32`, a Safe same-unit function (validated summary) | **Safe** (upper bound) |
| 15 `path:call` functions (`cio_file_native_delete_by_path`, four `ctr_encode_opentelemetry` initializers, `rd_kafka_errno`, two `rd_kafka_DescribeCluster_result_*`, `rtrim`, `str_is_printable`, `rd_kafka_conf_validate_str`, `line_protocol_escape`, `msg_write`, `net_plain_send_file`, `rd_hdr_getCountAtIndex`) | a place whose address holds a call: `errno` (`(*__errno_location ())`), `isspace`/`isprint` (`(*__ctype_b_loc ())[c]`), `f(r)->m`, `a[f(i)]`, refused by the lexical indirect-access gate before any nested-call rule | **stay Unknown** |

**Correction to the input (to be confirmed by the AFTER):** CG00's `E11
path:call` reason (223 leaves) is not "a call occupying part of an argument";
it is a *call-rooted place* (an access path whose base or index is a call's
value). No rule of this card reads a place, so none of those leaves can move
here. It is registered as a residual with its size.

## Predicted outcome

1. **BEFORE reproduces the baseline** (held above).
2. **Safe moves: at most 5**, each attributed by reading. Of CG00's 23, at
   most 1 (`snappy.c::hash`). Beyond the list, at most 4 from the
   `formal-embedded` and `return` families whose calls all have a contract
   in force.
3. **No Safe → non-Safe move.** The consumers are asked after every older
   route of their position and restore the site counters on refusal.
4. **Added leaks or violations are possible** where an owned inner result is
   only borrowed by the outer call (`printf("%s", strdup(x))`); each is
   classified by reading. None is presented as an upstream defect.
5. **Leaf exchanges:** an admitted statement can uncover a later refusal of
   the same function; the nested family shrinks by the admitted
   consultations, the rest keep the older refusal byte for byte.
6. **Sidecars byte-identical**; the system slice and the tail-pause slice do
   not move status (no nested-call leaf in the system selection is
   contracted at both levels).
