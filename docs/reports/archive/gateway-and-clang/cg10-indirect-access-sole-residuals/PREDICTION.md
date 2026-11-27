# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: prediction

Frozen in two parts. Part 1 is committed before any implementation code of the
card is committed or built (the rule-12k draft existed only in the working
tree, set aside while the BEFORE binaries were built from the base sources).
Part 2 is appended and committed after rule 12k's own measurement and before
any contract or further code.

## Part 1 (base `2ecd39531`)

### BEFORE

Measured with the base binary (`scripts/build.sh before`, `scripts/measure.sh
before`): corpus `66910c10` **695 / 9 / 4,130**, `resource-diagnostics.json`
`d2f5a3f7…`, byte-identical to the integrator's baseline and to card
CG10-INDIRECT-ACCESS-SOLE-BLOCKERS' AFTER. The base code is that card's tip, so
its AFTER census (`../cg10-indirect-access-sole-blockers/artifacts/classify-after.json`)
classifies this corpus; the card's own census run (`scripts/census-run.sh
census-before`) re-derives it.

### Task 2 census, from that classification (`scripts/callee_census.py`, `scripts/callee_kind.py`)

The two classes hold 331 functions whose every leaf is in them (227 + 102 alone,
plus 2 with both), 685 leaves, 281 distinct first callees; 56 more functions
carry these classes beside another. Fixed point in rank order: 14 (first
callee), 61 (6), 83 (11), 97 (16), 117 (26), 142 (36), … 331 (281). By where the
top 45 callees' bodies are: cross-unit 22 callees / 69 freed alone, same-unit
11 / 38, header inline 6 / 28, external 6 / 7.

| callee | alone | touched | body | its blocker (to be read) |
| --- | ---: | ---: | --- | --- |
| `rd_kafka_interceptor_method_add` | 14 | 14 | same unit, Unknown | retains `ic_opaque` and `func` in a list element: decision class 1 |
| `rd_avg_add` | 12 | 15 | header inline | not a reported row |
| `cmt_atomic_load` | 12 | 13 | cross-unit, Safe | no cross-unit route; five agreeing backend definitions |
| `cmt_atomic_store` | 10 | 18 | cross-unit, Safe | as above |
| `rd_atomic64_get` | 8 | 10 | header inline | `__atomic` builtin (class 6); `rd_atomic32_get` is declared |
| `rd_kafka_log0` | 5 | 24 | same unit, Unknown | logging, calls a configured callback |
| `rd_kafka_timer_start0` | 5 | 7 | cross-unit, Unknown | retains a callback and its argument (class 1) |
| `rd_kafka_confval_set_type` | 5 | 5 | cross-unit, Unknown | |
| `ctr_errno_print` | 4 | 6 | cross-unit, Safe | the actual is `errno` (`(*__errno_location())`) |
| `rd_list_add` | 3 | 9 | cross-unit, Unknown | retains its element (class 1) |
| `rd_atomic32_add`, `rd_atomic64_add` | 3, 3 | 5, 3 | header inline | `__atomic` builtin |
| `cnd_timedwait_ms` | 2 | 2 | cross-unit, Safe | no cross-unit route |
| `cmt_atomic_compare_exchange` | 1 | 9 | cross-unit, Safe | as `cmt_atomic_load` |

### Task 1: rule 12k (re-attribution)

* **No status moves** on any route (corpus, system, slice): the node stays the
  same refusal with the same writes, and a non-held `Unsupported` leaf is
  `Unknown` whatever its text. Certain by construction; any status move is a
  defect of the implementation.
* Every changed leaf is one `indirect memory access is outside the resource
  projection` leaf replaced by one ``call to uncontracted function `F` `` leaf
  at the same line. No leaf added or removed.
* Of the 685 leaves of the two classes, **400–600** are re-attributed (the
  rest have an actual neither reader recognises, `errno` reads and nested
  calls among them, or an access outside the actuals); **170–270** of the 331
  functions end with every leaf an uncontracted-call leaf.
* Across the whole corpus (the access family touches 2,918 functions, 13,057
  leaves): **800–3,000** leaves re-attributed.

### Task 3: what will be attempted (sized in part 2)

In rank order, only where the callee decides (part 2 measures it) and the facts
are derivable by reading: declared, pinned contracts for the cross-unit
callees whose home body is `Safe` (`cmt_atomic_load`, `cmt_atomic_store`,
`cmt_atomic_compare_exchange` in `qcore-provider-cmetrics`; `cnd_timedwait_ms`
in `qcore-provider-librdkafka`), and for the librdkafka header inlines beside
the declared `rd_atomic32_get` (`rd_atomic64_get`, `rd_atomic32_add`,
`rd_atomic64_add`, `rd_atomic32_set`, `rd_avg_add` after reading why its body
publishes nothing). Not attempted: decision-class callees (retained pointers,
callbacks, reference counts) are sized, not decided. Gross range before part 2:
**+20 to +60** Unknown → Safe on the corpus, no Safe → non-Safe move.

## Part 2 (after rule 12k's own corpus measurement, before any contract)

### Rule 12k measured on the corpus (base + `e6414f3ca`, `scripts/attribution.py`)

`00462adf…`, **695 / 9 / 4,130, no status move** (prediction held). 1,457
functions change leaves: access leaves 13,083 → 10,121 (−2,962), uncontracted
call leaves 8,433 → 11,442 (+3,009; 47 more than removed because a line holding
two refused statements carried one access leaf for both, and naming one callee
shows both). Every changed leaf is that replacement (0 problems). Of the 685
leaves of the two classes, 463 now name their callee and 222 keep the access
refusal (inside the 400–600 predicted); of the 331 functions, **215** are now
blocked by uncontracted calls alone (170–270 predicted) and 116 keep an access
leaf. Corpus-wide, 758 Unknown functions are blocked only by uncontracted calls
(477 before). Corpus re-attribution 2,962 leaves, inside 800–3,000.

### Ranking with rule 12k (`scripts/decides.py`, the 331)

rd_avg_add 12 alone / 14 touched; cmt_atomic_store 10 / 14; cmt_atomic_load 8 /
8; rd_kafka_confval_set_type 5; rd_atomic64_get 4; rd_kafka_interceptor_method_add
4 (was 14: its `(void *)on_…` cast actual is read by neither reader, so 10 keep
the access leaf); rd_atomic64_add 3; rd_kafka_queue_new0 3; rd_list_add 3;
rd_list_clear 3; …; cnd_timedwait_ms 2; cmt_atomic_compare_exchange 1 / 5;
rd_atomic32_set 1 / 3.

### The contracts this card adds (in force only once pinned)

* `qcore-provider-cmetrics` (`cmetrics/metric_values.qv`): `cmt_atomic_load`,
  `cmt_atomic_store`, `cmt_atomic_compare_exchange`, borrowed / none; every
  backend definition read (identical at both revisions); home body `Safe`.
* `qcore-provider-librdkafka` (`librdkafka/memory.qv`, beside
  `rd_atomic32_get`): `rd_atomic64_get`, `rd_atomic32_add`, `rd_atomic64_add`,
  `rd_atomic32_set` (header bodies over `__atomic` builtins, as
  `rd_atomic32_get`), `cnd_timedwait_ms` (home body `Safe`) and
  `rd_hdr_histogram_record` (home body `Safe`), the one uncontracted call of
  the header body `rd_avg_add`, which is then left to W4 in each including
  unit rather than declared.

Ceiling (every Unknown function whose every leaf is an uncontracted call to
one of these, plus `rd_avg_add`): **47** corpus functions (23 cmetrics value
accessors and exchanges, 18 librdkafka telemetry functions over `rd_avg_add` and
`rd_atomic64_*`, `rd_kafka_brokers_wait_state_change`,
`rd_kafka_metadata_cache_wait_change`, `rd_kafka_bufq_reset`, `rd_kafka_cert_dup`,
`rd_kafka_idemp_inflight_toppar_add`, `map_unlock`).

**Predicted: +30 to +50 Unknown → Safe on the corpus** (each freed statement
still carries W44's typed reads and second obligation, an `UnknownAlias` on a
bound base keeps a function Unknown; `rd_avg_add`'s 12 callers depend on its
body validating once `rd_hdr_histogram_record` is declared, which I give
better than even odds; callers of the freed functions may follow through their
summaries: 0–8 more). No Safe → non-Safe move. 0–2 new provisional `Unsafe`
(a body read further may expose a leak), each to be read and classified.
System and slice: 0 to +3 (to be measured; these callees are corpus
libraries the slice routes do not compile, except Fluent Bit's own cmetrics
use).
