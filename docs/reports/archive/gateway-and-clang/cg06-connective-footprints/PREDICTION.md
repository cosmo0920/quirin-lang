# CG06-VALUE-CONNECTIVE-RESIDUALS (sibling-effect part): frozen prediction

Frozen before the admitting commit and before any AFTER run. Base: local
`main` `d4df096f` (round 7 complete). CG00's frozen
[`predictions/CG06.md`](../cg00-clang-gap-baseline/predictions/CG06.md) is the
original card's prediction (its 40 functions were measured by CG06 itself); it
names no function for this residual, so this file re-derives the population
instead of copying it.

## Population (re-derived on the base)

The BEFORE corpus run (`private/cg06r/before`, binary built at `d4df096f`,
`3b09309a…`, **628 / 10 / 4,196**: CG12's integration figures; the bytes
differ from CG12's `394b190b…` only by CG18's known delta, 2 functions, 4
indirect-access leaves become `unknown_alias`) records **2,485** de-duplicated
rule-10z consultations ([parity-census-before.json](artifacts/parity-census-before.json)):
255 admitted; refused 1,133 sibling with an effect, 411 an edge, 311
undecided deciding operand, 284 no unique node (I04), 67 under a comma, 14
under an indirect callee, **4** deciding call beside a non-inert sibling, 4
over 8 leaves, 2 a nested assignment or declaration. The two classes this card
owns reproduce exactly (1,133 and 4).

A census build (commit `c2461609`: the composition computed and named in the
parity row, nothing admitted; its corpus `resource-diagnostics.json` is
byte-identical to the BEFORE, `3b09309a…`) splits them
([census-before.json](artifacts/census-before.json)):

| Class | Sibling-effect rows (functions) | Decision rows (functions) |
|---|---:|---:|
| (a) contracted, footprints commute on both arms | **985** (125) | **2** (1) |
| (b) contracted, footprints overlap | 3 (2): `mconn` read against `rd_list_cnt` | 0 |
| (c) uncontracted sibling call | 143 (99): `rd_kafka_err2str` 46, `rd_kafka_err2name` 23, `rd_kafka_fetch_pos2str` 15, `rd_kafka_broker_name` 13, `rd_kafka_offset2str` 7, … | 0 |
| (d) a sibling effect that is not a call | 1 (1): `grps[i++] = rd_strndup(…)` | 0 |
| sibling CG05's reading refuses (indirect call, CG22) | 1 (1): `rd_map_copy` | 0 |
| the region CG05's reading refuses (uncontracted deciding call) | 0 | 2 (2) |
| address meeting (rule 11m's extra premise) | 0 | 0 |

## Prediction

- **Consultations.** At most 987 of the 1,137 become admitted by composition;
  each is then lowered only if every edge is decided by an older route, so the
  admitted count is lower. Two known shapes refuse after admission: a region
  set over 8 leaves (`rd_kafka_AclBinding_request_size`'s four `?:`, 16
  leaves), and an edge whose older route refuses (an uncontracted outer or
  inner call, a refused actual).
- **Functions (composition).** Upper bound **2** Unknown → Safe: the only
  functions whose every BEFORE leaf stands on a class-(a) line
  ([prediction.json](artifacts/prediction.json)). Read: both stay Unknown.
  `rd_kafka_AclBinding_request_size` refuses on its leaf count;
  `rd_kafka_buf_read_CurrentLeader`'s leaves on those lines are the
  `rd_kafka_buf_read_*` macro's own branch conditions and `unknown_alias`
  reads on the same lines, which no connective lowering removes. **Predicted:
  0 moves from the composition.**
- **Edges (amendment, with reason).** Implementing the admission showed that
  an admitted edge whose statement holds nested calls reaches rule 10y's
  consumer, which matches its Clang node by line and tokens; an edge's
  statement is spelled nowhere, so every such edge refused ("no unique Clang
  node"). Rule 11m therefore lets rule 10y's two consumers read the edge's own
  substituted node (the node rule 10z's `leaf_events` already reads) where no
  source node matches. This applies to every rule-10z edge, not only composed
  ones, so the 411 edge refusals can shrink too. Upper bound: **24** functions
  (the 2 above and 22 whose every leaf stands on a class-(a) or edge-refused
  line); CG06's report already read 6 of the 22 as refused on the edge for
  another reason (`rd_kafka_error_name`, `rd_kafka_error_string`:
  uncontracted `err2name`/`err2str`; `rd_kafka_log_print`: an array member's
  decay; `rd_kafka_queue_forward`, `rd_kafka_msgset_writer_compress_lz4`,
  `rd_kafka_txn_schedule_register_partitions`: other actuals). **Predicted:
  0–4 moves in total**, every one attributed by reading in the report.
- **No Safe → non-Safe move.** An admitted composition changes no refusal into
  a verdict except through an edge an older route decides; a leaf added on an
  edge (an `unknown_alias` read the old refusal hid, a leak or violation an
  edge exposes) is classified by reading.
- **System and tail-pause slices.** The census has no class-(a) row there
  that this prediction reads as movable; predicted unmoved (44 / 2 / 188 and
  29 / 1 / 239 / 1 Pending), leaves possibly changed.

## Corrections to the brief, recorded before implementation

- The brief's admitted control, `log(ctx, p ? strlen(p) : 0,
  release_and_status(q))` with **two names** `p` and `q`, cannot be admitted
  by CG05's footprints as they are: two identity cells always overlap
  (`cells_overlap`; names may alias). It is pinned refused. The admitted
  control releases an identity the footprints prove distinct: a fresh owned
  result (`r_release_code(r_wrap(p))`).
- The brief's "a deciding call beside a non-inert sibling" class is 4 rows on
  this base (unchanged); 2 compose, 2 have an uncontracted deciding call.
