# RK-TOPIC-PARTITION-LIST-CONSTRUCTORS: prediction

Committed before the implementation commit. Base `8e7e8d509` (integrate/r48).
USER DECISION 2026-11-21: launch the card the ledger entry of the same name
specifies.

**What was known when this was written.** The base measurement (section 1)
and the bodies read at both audited revisions. No probe with the
declarations was run: the prediction is from the base report and the
bodies alone.

## 1. Base

Opt-in `quirin-audit current --monkey-linked` over Fluent Bit 046accf6, base
binary `ff817b5c…` (built from `8e7e8d509`), side tree `git archive
8e7e8d509`: **1,159 Safe / 2 Pending / 3,636 Unknown / 7 Unsafe**; 2,135
Unknown rows carry the indirect-access refusal (9,027 leaves), 209 carry
nothing else.

`scripts/callers.py` on the base report: **68 rows** name one of the card's
functions in a `call to uncontracted function` leaf (83 leaves:
`rd_kafka_topic_partition_list_new` 44 in 43 rows, `rd_kafka_topic_partition_destroy`
14 in 13, `_list_copy` 12 in 12, `rd_kafka_topic_partition_new` 12 in 11,
`rd_kafka_topic_partition_copy` 1). **All 68 are Unknown**; 2 carry nothing
else (`rd_kafka_topic_partition_copy_void`, `rd_kafka_topic_partition_list_copy_opaque`).
The most frequent co-blocking callee is `rd_kafka_topic_partition_list_add`
(22 rows; with the card's functions it alone blocks 2 more:
`ut_topic_partitions`, `ut_create_topic_partition_lists`). Round 4 named four
more rows whose call is hidden behind the access refusal
(`rd_kafka_DeleteRecords_new`, `rd_kafka_ElectLeaders_new`,
`rd_kafka_ListOffsetsResultInfo_new`, `rd_kafka_assignment_init`; each
Unknown on access leaves only).

In their home unit `rdkafka_partition.c`: `rd_kafka_topic_partition_new`,
`rd_kafka_topic_partition_destroy` (and `_destroy_free`,
`_new_with_id_and_name`, `rd_kafka_topic_partition_list_destroy_free`) are
**Safe**; `rd_kafka_topic_partition_list_new` (uncontracted static `_grow`),
`_list_copy` and `rd_kafka_topic_partition_copy` (uncontracted static
`_update`) are Unknown.

## 2. Planned changes

`providers/qcore-provider-librdkafka/contracts/librdkafka/topic_partition_constructors.qv`
(its own module, registered in `cycle19-resource-contracts.json` and the load
path), pinned at `66910c10` and `046accf6`:

* owned, nonnull results: `rd_kafka_topic_partition_list_new(size)`,
  `rd_kafka_topic_partition_list_copy(src)`, `rd_kafka_topic_partition_copy(src)`,
  `rd_kafka_topic_partition_new(topic, partition)`; every formal borrowed;
* the release `rd_kafka_topic_partition_destroy(rktpar)`: released, nonnull,
  naming `rk_plugin_callback_closure` as `rd_kafka_topic_partition_destroy0`
  does.

The reference half (decision class 3) is decided as encapsulation: the toppar
references the copies take live in the result's own elements and are
dropped by its destructor (`rd_kafka_topic_partition_list_destroy`, already
declared released); no reference is returned to the caller. No gateway rule,
no Rust change. `rd_kafka_topic_partition_list_add` is left undeclared (its
result points into an array a later add reallocates; a borrowed-from result
would admit in the unsafe direction).

## 3. Predicted movements (opt-in route)

**Unknown -> Safe**, likely: `rd_kafka_topic_partition_copy_void`,
`rd_kafka_topic_partition_list_copy_opaque` (each returns the declared
owned result: an owned-return summary). Possible:
`rd_kafka_ListOffsetsResultInfo_destroy` and
`rd_kafka_share_partition_offsets_destroy` (a member's partition released,
then the holder; each keeps one `unresolved resource identity` leaf on base
whose cause may be the undeclared destructor), `rd_kafka_ElectLeaders_new`,
`rd_kafka_DeleteRecords_new`, `rd_kafka_ListOffsetsResultInfo_new` (an
owned result stored into the body's own fresh allocation and returned: the
field-link model, if the access reader reaches the statement). Range **2-8**.

**Safe -> non-Safe: 0 expected.** Every base row naming a declared function
uncontracted is already Unknown. The one exposure: `rd_kafka_topic_partition_new`
is Safe in its home unit on a derived summary; if the derived summary
disagrees with the declaration (nullable against nonnull) the declaration
could be held there and `rd_kafka_topic_partition_new_with_id_and_name` (or
other in-unit callers) could lose their summary. Not expected; any such row
is reported.

**New Unsafe: 0 expected, at most a few possible.** A newly tracked list in
a caller whose other leaves vanish could show an unreleased path (an early
error return after the constructor). Any would be reported per row as
provisional, never as an upstream defect.

**Leaves:** the 83 `call to uncontracted function` leaves for the five
functions disappear; most of the 68 rows stay Unknown on their other leaves
(`rd_kafka_topic_partition_list_add` and 300-odd other uncontracted callees,
access and identity leaves), with leaves re-attributed downstream (new
`unresolved resource identity` or custody leaves where the tracked list
reaches an uncontracted callee or a member store). Access rows and
access-alone change by at most the round-4 rows above.

**Validation of the declarations against their bodies:**
`rd_kafka_topic_partition_new` and `rd_kafka_topic_partition_destroy`
expected to validate in `rdkafka_partition.c` (their rows are Safe);
`_list_new`, `_list_copy` and `rd_kafka_topic_partition_copy` will not
(uncontracted static `_grow`, `_update`, `_add_copy`).

```measure-prediction
route corpus
move rd_kafka_topic_partition_copy_void Unknown -> Safe
move rd_kafka_topic_partition_list_copy_opaque Unknown -> Safe
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Corpus (and twelve-home): the
route analyses librdkafka at `66910c10`, whose bodies are byte-identical
and where the declarations are pinned, so the same two rows are predicted,
with the same possible extras where the route analyses `rdkafka_admin.c`,
`rdkafka_share_acknowledgement.c` and `rdkafka_partition.c`. System and
slice: none (they analyse no librdkafka unit).
