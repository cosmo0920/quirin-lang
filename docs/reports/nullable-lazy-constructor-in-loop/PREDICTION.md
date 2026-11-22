# NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP: prediction

Committed before the implementation commit. Base `1b7028430` (main, with
RK-TOPIC-PARTITION-LIST-CONSTRUCTORS landed). USER DECISION 2026-11-23:
"NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP -> We need to launch this card after the
dependent card is landed."

## What was known when this was written

No implementation was measured on the corpus. Known:

* The eight rows' nullable leaves and lines from the constructors card's
  base/tip comparison (`docs/reports/rk-topic-partition-list-constructors/evidence/changed-rows-base-tip.txt`)
  and the guarded-release card's merged run, mapped to the bodies in the
  prepared checkout (`private/fb-046accf6`, librdkafka 2.15.0):

  | row | leaf name, line | where in the body |
  |---|---|---|
  | `rd_kafka_cgrp_owned_but_not_exist_partitions` | `result`, 24493 | `return result;` (`rdkafka_cgrp.c:7614`) |
  | `rd_kafka_cgrp_calculate_subscribe_revoking_partitions` | `revoking`, 23536 | `return revoking;` after `destroy(revoking); revoking = NULL;` (`:5958`) |
  | `rd_kafka_cgrp_consumer_assignment_with_metadata` | `assignment_with_metadata`, 21948 (with `unresolved resource identity`) | `return assignment_with_metadata;` (`:3048`), binding unknown |
  | `rd_kafka_cgrp_handle_SyncGroup_memberstate` | `assignment`, 21306 | unconditional destroy after a join with an uncontracted reader's result (`rd_kafka_buf_read_topic_partitions`) |
  | `rd_kafka_mock_cgrp_consumer_member_needs_revocation` | `intersection`, 21432 and 21434 | return and destroy after a join with an uncontracted result (`rd_kafka_topic_partition_list_intersection_by_id`) |
  | `rd_kafka_mock_handle_ConsumerGroupHeartbeat` | `existing_assignment`, 24072 | `RD_IF_FREE` release after a loop of refused reads |
  | `balance` | `leftoverUnassignedPartitions`, 21898 | destroy under a boolean flag that records whether the copy was made (`rdkafka_sticky_assignor.c:1210`) |
  | `rd_kafka_mock_handle_ShareFetch` | `forgotten_partitions` ×3 | `RD_IF_FREE` releases after `while` loops of refused reads |

* Local fixtures (real Clang-backed gateway, synthetic contracts) at the
  base: the lazy guard `if (!r) r = ctor();` is **already decided** in a loop
  and outside one (rules 14l, 10b): a loop with a borrowed use and a guarded
  release is `Safe`; a returned lazy loop with no refusal is `Safe` through the
  validated `Owned { nullable: true }` summary. The nullable leaf appears only
  when another refusal blocks the summary and the contract-free run's result
  mode (non-null) meets `return r;` with `r` NULL on a real path. The leaf is
  then a pair: `unresolved resource identity r` and `nullable resource r used
  by a non-null contract`, although the identity is resolved (one site).
* So the ledger's shape (1) is not a missing guard refinement: its row's leaf
  is on the return of a genuinely possibly-NULL owned value, and the
  `revoking` row (named shape (2)) is the same return shape, not a poisoned
  binding. The other rows are other shapes (a poisoned binding, a join with an
  uncontracted result, a flag-correlated release, guarded releases of
  bindings poisoned in loops of refused reads).

## Planned change

A resolved binding (identity derivation, one target, no unknown alternative)
that may be null, returned under a non-null result mode, is refused by one
leaf, `possibly-null resource `r` returned under a non-null result mode`,
in place of the identity and nullable pair, with **exactly the same marking**
(`mark_unresolved_effect`). The abstract state, the work-list schedule and
every leak claim are unchanged; the refusal stays, so no row can become
`Safe`. Every other binding keeps the existing path. No guard machinery is
added: the lazy guard needs none.

## Predicted movements (opt-in route, base vs tip)

**No status moves:** Safe→non-Safe 0, Unknown→Safe 0, Unsafe→Unknown 0,
Unknown→Unsafe 0. Totals equal to the base's.

**Leaves:** `nullable resource … used by a non-null contract` falls and
`unresolved resource identity` falls, each by the number of returns of a
resolved possibly-null binding under a non-null mode (tens corpus-wide, not
hundreds); the new leaf rises by the same row set. Of the eight rows:

* `owned_but_not_exist_partitions` and `calculate_subscribe_revoking_partitions`
  lose their nullable leaf (2 of the 11) and carry the new leaf; both stay
  `Unknown` on their other refusals.
* `needs_revocation`'s return leaf (21432) may be renamed if the joined
  binding is resolved there; its destroy leaf (21434) stays.
* `consumer_assignment_with_metadata`, `SyncGroup_memberstate`,
  `ConsumerGroupHeartbeat`, `balance` and `ShareFetch` keep their nullable
  leaves (not a return of a resolved binding); each is named in the report by
  its real obstacle.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host (no `build/cycle19` inputs or baselines). Expected there: no status
moves on any route; leaf-only renames where a resolved, possibly-null binding
is returned by a body whose summary did not validate. Any status move would be
unpredicted.
