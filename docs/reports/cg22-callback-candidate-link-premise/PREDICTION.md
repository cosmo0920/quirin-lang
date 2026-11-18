# CG22-CALLBACK-CANDIDATE-LINK-PREMISE: prediction

Written and committed before the profile, its link map or any test exists.
Base `974434463` (main). The card is data-only: it adds a new live evaluation
profile `twelve-home` whose stated link map (`QCORE_CROSS_UNIT_LINK_MAP`) is
the eight-home map plus four comparator homes, and changes no analyzer code.

## What is compared

One analyzer (the release build of the base export, used on both sides) on
the pinned corpus route, on this card's worktree-private preparation:

* BEFORE: the route under the eight-home map (`contract/evaluations/eight-home/link-map.json`,
  sha256 `98a4db76…`), run twice (`base`, `again`) for stability;
* AFTER: the same route under the twelve-home map, compared with
  `run:<before>/sides/base` by `quirin-audit measure --declared` this file.

The four added symbols (rule 4c homes), all external-linkage definitions with
a Safe row in their home unit on the CG22-CALLBACK-CANDIDATE-CONTRACTS census:

| symbol | home unit | invocation(s) refusing `callback-uncontracted` |
| --- | --- | --- |
| `rd_kafka_metadata_broker_cmp` | `librdkafka/rdkafka_metadata.c` | `rdkafka_aux.c::rd_kafka_Node_new_from_brokers` (`bsearch`) |
| `rd_kafka_metadata_broker_internal_cmp` | `librdkafka/rdkafka_metadata.c` | `rdkafka_aux.c::rd_kafka_Node_new_from_brokers` (`bsearch`), `rdkafka_assignor.c::ut_populate_internal_topic_metadata` (`bsearch`) |
| `rd_strcmp3` | `librdkafka/rdstring.c` | `rdkafka_range_assignor.c::rd_kafka_topic_assignment_state_rack_search`, `rdkafka_sticky_assignor.c::rd_kafka_partition_internal_find_rack` (`bsearch`) |
| `rd_kafka_assignor_topic_cmp` | `librdkafka/rdkafka_assignor.c` | `rdkafka_roundrobin_assignor.c::rd_kafka_roundrobin_assignor_assign_cb` (`qsort`) |

The unmeasured variant map of the registering card
(`link-map.eleven-home.json`) lists only the first three; the fourth is
needed for the roundrobin leaf, so this profile has twelve homes.

## Prediction

1. **No function status moves on the corpus route** (block below). Each of
   the five callers carries other leaves (CG22-CALLBACK-CANDIDATE-CONTRACTS
   census: 6, 17, 4, 2 and 9 leaves), so discharging the comparator leaf
   leaves each Unknown.
2. **The six `callback-uncontracted` leaves at those invocations are
   discharged**: each invocation decides over the imported home summary
   (rule 4c channel, rule 13u's candidate in force), and the five rows list
   the imported homes. Uncertain for `rd_strcmp3` (calls `rd_strcmp`) and
   `rd_kafka_assignor_topic_cmp` (calls `strcmp`): if either home summary is
   not exported or not an identity, that leaf stays (`callback-uncontracted`
   or `callback-effectful`) and the prediction is wrong on that point, not
   the verdicts.
3. A discharged leaf may be replaced on the same row by the comparator's
   premises (null premises on what the invocation hands it, as with
   `rd_kafka_ApiVersion_key_cmp`) or by a leaf the refusal used to mask;
   content changes only.
4. Rows whose assumptions carry the link-map digest change content (the new
   map's digest replaces `98a4db76…`); no status change from it.
5. No `Safe -> non-Safe`, no `Unsafe` added or removed. Home units are
   unchanged (an export does not change its home's row). The replay budget
   (24 homes per unit) is not reached by twelve homes.

```measure-prediction
route corpus
none
```
