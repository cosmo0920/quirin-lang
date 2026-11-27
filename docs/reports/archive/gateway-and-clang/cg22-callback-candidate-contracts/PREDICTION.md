# CG22-CALLBACK-CANDIDATE-CONTRACTS: prediction

Committed after the census and before the semantic commit. Base `32336c33d`
(origin/main), measured here on this card's own corpus preparation
([inputs](artifacts/inputs.json), [prep.sh](artifacts/prep.sh)): BEFORE
`measure/before/sides/base` (binary `47db0eba…` built from the base export),
corpus **1,082 / 6 / 3,616** (report `0f3f068e…`).

## Census (re-measured on the base)

`callback-uncontracted` (rule 13u): **51 leaves / 49 functions / 18 decided
alone** (the registering card measured 46 / 44 / 17 on its base;
[reasons](artifacts/reasons-before.json), [leaves](artifacts/leaves-before.txt)).
Of the 46 candidates named ([status](artifacts/candidate-status-before.txt)):

* **41 Unknown in their own unit**: the cmetrics and ctraces decoder trees
  (36 leaves, every table holding at least one Unknown handler), the
  librdkafka comparators `rd_kafka_group_member_cmp`,
  `rd_qsort_r_trampoline`, `toppar_sort_by_list_cnt`, and two op-callback
  candidates declarations landed since registration expose
  (`rd_kafka_coord_rkb_monitor_cb` through `rd_kafka_poll_cb`,
  `rd_kafka_toppar_op_serve` through `rd_kafka_q_serve` / `rd_kafka_op_req0`).
  They stay refused: a callback Unknown in its own unit has no contract in
  force, and no route here gives it one.
* **4 Safe in another unit** (`rd_kafka_metadata_broker_cmp`,
  `_internal_cmp`, `rd_strcmp3`, `rd_kafka_assignor_topic_cmp`), external
  definitions. Rule 4c's import is the route that would put them in force; it
  needs each name in the stated link map, which the eight-home profile's map
  does not list. Their callers (`rd_kafka_Node_new_from_brokers`,
  `ut_populate_internal_topic_metadata`,
  `rd_kafka_topic_assignment_state_rack_search`,
  `rd_kafka_partition_internal_find_rack`,
  `rd_kafka_roundrobin_assignor_assign_cb`) each carry another blocker, so
  even under an extended map no status would move (read from their leaves,
  not measured).
* **1 with no row**: `rd_kafka_ApiVersion_key_cmp`, a header `static`
  (`rdkafka_proto.h:208`) that rule 4a's derivation scope never reaches,
  because it follows direct calls only and the comparator is only handed to
  `bsearch` / `qsort`.

## What the card changes

Rule 4a's derivation scope also follows the designators a body names beside a
call to a declaration in force with a synchronous invocation or a table
dispatch (rule 13u's W4 order edges), still only to unselected
internal-linkage definitions the unit carries. The unchanged derivation then
publishes the comparator's summary only if it validates; the invocation still
decides over the candidates' contracts in force, so an Unknown candidate
keeps its caller refused.

## Per route and function

* **corpus**: `rd_kafka_ApiVersion_check` (`rdkafka_feature.c`, its only leaf
  the `bsearch` refusal; the registering card measured it `Safe` under a
  whole-text harness) **Unknown -> Safe**. Safe +1. Its caller
  `rd_kafka_features_check` trades `call to uncontracted function
  rd_kafka_ApiVersion_check` and stays Unknown (other leaves). The other
  callers of `rd_kafka_ApiVersion_key_cmp` (`rdkafka_broker.c`
  `rd_kafka_broker_ApiVersion_supported0` and its second `bsearch`,
  `rdkafka_feature.c` `rd_kafka_ApiVersions_copy`'s `qsort`,
  `rdkafka_request.c` `rd_kafka_handle_ApiVersion`'s `qsort`) may change
  content only; none moves. `callback-uncontracted` 51 / 49 / 18 -> **50 / 48
  / 17**. No other row moves; no Safe -> non-Safe; no Unsafe added or
  removed.
* **system, slice**: not measured by this card (worktree-private preparation
  of the corpus route only, as RULE50-CONSUMER-COMPOSITION prepared it). No
  `callback-uncontracted` leaf of the registering card's census was on either
  route (their callback leaves were `msgpack_packer_init` access refusals and
  `pthread_once` over an unprototyped designator, `callback-undecided`); a
  header `static` handed to a synchronous invocation there would be the only
  way a row moves, in the accepting direction only.

```measure-prediction
route corpus
move rdkafka_feature.c::rd_kafka_ApiVersion_check Unknown -> Safe
delta Safe +1
```
