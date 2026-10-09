# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: prediction

Frozen and committed before any implementation code of this round was
written. Base: local `main` at `84ec54d1d`.

## BEFORE (reproduced)

Base binary `private/cg10r2/bin/before` (`3f385e08…`, `scripts/build.sh
before`, `scripts/measure-binary.sh before`):

| route | totals | digest |
| --- | --- | --- |
| corpus `66910c10` | **881 / 9 / 3,944** | report `ab8f5f14…`, verdict rows `321a8913…` (the integrator's baseline) |
| system | 87 / 4 / 357 | `98066723…` |
| tail-pause slice | 45 / 1 / 354, 4 Pending | `8c37c395…` |

The census run (`scripts/census-run.sh census-before`, the
`indirect-census` binary `private/cg10r2/bin/before-census`) wrote the same
report byte for byte (`ab8f5f14…`), and the measured binary was restored
after it.

## The population, re-measured

Unknown functions whose only refusal family is `indirect memory access is
outside the resource projection`: **228 functions, 380 leaves** (205 / 347
after the first round; 203 of the 228 carry over, 25 are new, exposed by
later cards' contracts). `scripts/classify.py` and `scripts/residual_table.py`
(the first round's, unchanged) give
[artifacts/classify-before.json](artifacts/classify-before.json) and
[artifacts/residual-before.json](artifacts/residual-before.json):

| class (residual table) | alone | leaves | touched |
| --- | ---: | ---: | ---: |
| uncontracted callee, an actual neither reader recognises | 41 | 99 | 70 |
| whole-call return, the same | 24 | 25 | 24 |
| `custody-position:released` (by design) | 13 | 52 | 29 |
| `pointer-store:other` | 12 | 15 | 15 |
| `probe-declined` | 10 | 20 | 15 |
| `elaboration`: no value form | 10 | 15 | 14 |
| `pointer-store:formal-pointer` | 10 | 16 | 13 |
| `unclassified:CallArgument` | 6 | 6 | 6 |
| `call-result-store` | 5 | 16 | 13 |
| `unrecognised-actual:address` | 5 | 16 | 11 |
| `unrecognised-actual:other` | 5 | 6 | 6 |
| whole-call return, no rule-12k reading | 5 | 5 | 5 |

Fixed point in that table's rank order: 41, 65, 90, 103, 116, 128, ….

## Ranking by functions freed alone (task 1)

`scripts/leaf_detail.py` joins each leaf with what the readers recorded
([artifacts/detail-before.json](artifacts/detail-before.json));
`scripts/rank.py` turns each leaf into the fixes it needs and counts the
functions a fix frees **alone** ([artifacts/rank-before.json](artifacts/rank-before.json)).
An uncontracted-callee leaf needs `contract:F` **and** a reader fix for every
actual neither reader recognises, so a reader lift there frees nothing by
itself: it only re-attributes. That is the correction to the brief's expected
heads: the 41 + 24 functions of the two largest classes are freed alone by
**no** reader lift; 26 of them need only `&`-actual reading plus contracts,
most for decision-class callees (`rd_kafka_interceptor_method_add`,
`rd_kafka_timer_start0`: class 1; `rd_kafka_log0`: a configured callback).

Freed alone, by fix: `custody-position:released` 13 (refused by design, left),
`pointer-store:other` 12, `pointer-store:formal-pointer` 10, elaboration no
value form 10, `probe-declined` 10, `unclassified:CallArgument` 6,
`reader:address` 5, `reader:other` 5, `call-result-store` 5, … . Read by hand,
those classes are mixtures; the lifts below are the sub-shapes that are
readable under existing premises:

| lift | rule | shape | candidates (freed alone if the lift holds) |
| --- | --- | --- | --- |
| A | 14a | a scalar statement (`T x = E`, `x = E`, `P = E` with `P` a typed scalar place) whose value holds exactly one call, its callee contracted with a `None` result and no custody-taking position, its actuals read by W44, the rest read by the typed path reader | `rd_kafka_broker_outbufs_space`, `calculate_share_acknowledgements_send_rate`, `_bytes_consumed_rate`, `_fetch_rate`, `_heartbeat_rate` (`probe-declined`); `rd_kafka_cgrp_set_last_err`, `rd_kafka_cgrp_update_session_timeout`, `rd_kafka_broker_weight_usable` (`call-result-store`); `rd_kafka_msgbatch_set_first_msg` (`probe-declined`): **9** |
| B | 14b | a pointer local initialised or assigned an interior address `&PATH` or `PATH ± n` (`PATH` a pointer read) under a never-bound root: the same target-free borrowed alias a pointer read binds | `append`, `peek`, `sink_peek`, `mk_sched_broadcast_signal`, `rd_kafka_share_ack_type_from_msg_op`, `update_offset`, `extra_alloc` (elaboration no value form): **7** |
| C | 14c | glibc's `errno` spelling `(*__errno_location())` read as a scalar actual with no base, plus three trivial neutral declarations `cio_errno_print`, `ctr_errno_print`, `cmt_errno_print` (provider crates chunkio, ctraces, cmetrics) | `cio_file_native_compose_path`, `cio_file_native_delete_by_path`, `initialize_resource_spans`, `initialize_scope_spans`, `initialize_spans`, `otlp_kvpair_list_initialize`: **6** |
| D | 14d | a `Borrowed` actual `&B->F[I]` whose element is a scalar (rule 10n(1) extended from a member to a scalar element), roots never bound | `metric_hist_exchange` (two units), `cmt_metric_hist_get_value`, `summary_quantile_exchange`: **4** |

Not lifted, by name: `custody-position:released` (by design);
`pointer-store:formal-pointer` and the identity-carrying `pointer-store:other`
values (a formal's or local's object pointer stored into another object: rule
10v's Q36 owner premise, the class-1 `retained_by` decision); owned call
results stored into members (`rd_buf_init`, `rd_kafka_snappy_init_env`,
`…_set_metadata_from_rktp_stored`, the field-link model's);
`unclassified:CallArgument` (a release of `arr[i]`, an element of a caller's
array, which rule 5k excludes by design); the uncontracted classes (contracts
for decision-class callees, `W45-CALLEE-CASCADE-RESIDUAL`). Below these, every
remaining fix frees at most 3 functions alone.

## Prediction

* Corpus: **+16 to +26 Unknown → Safe** (A 6–9, B 4–7, C 4–6, D 3–4,
  summaries validated through them 0–4); 881 → 897–907. No Safe → non-Safe
  move, no leak or violation added (leak and violation leaves unchanged).
* The access-alone population falls from 228 to **200–212**; lifts B and D
  also re-attribute or clear access leaves in functions that stay Unknown
  behind other blockers.
* Lift C re-attributes the uncontracted-callee leaves whose only unread
  actual was `errno` to their callee (rule 12k then reads them): counted
  separately, no status move from that alone.
* System and slice: 0–2 status moves each; no Safe → non-Safe.
* Safe roll-up: every new Safe row of A, B and D rests on the declarations its
  calls already consult (no new declaration); C's rows rest on the three new
  declared assumptions, so **4–6** new Safe rows rest on declared assumptions
  introduced by this card.
