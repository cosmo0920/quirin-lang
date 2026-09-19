# Combined B01–B04 prediction

Published before the combined AFTER audit. The independent B01 and B02
measurements are already known; this predicts their interaction, not their
original results. Baseline is the integrator's exit-0 reproduction of
`8e633261` in `private/b-series/baseline/`.

Expected combined results: **8 Unknown → Safe projected functions, 57 removed
unsupported leaves, zero added leaves or exchanges, zero unit verdict moves,
and zero tracker retirements**. Functions should be **221 Safe / 84 provisional
Unsafe / 4,529 Unknown**; units remain **2 / 42 / 137 / 4 data-only**. Diagnostics
should be leak **272**, unknown_alias **4,309**, unsupported **49,696**, and
summary_conflict **1**. No raw unit is promoted.

The expected multiset is the disjoint union of B01's reviewed direct-field
result (12 return leaves, seven getters) and B02's 45 branch leaves (one mover,
`flb_input_chunk_down`). B01's wider intermediate 13-leaf draft is excluded:
`get_input_chunk_source_name` contains two member selections and remains refused.
B03 measured no delta. B04 predicts no corpus delta because its sole-blocker
candidates lack a qualifying resource-neutral inner contract; its isolated
measurement is still in progress when this prediction is written.

The seven B01 movers are `rd_kafka_ConsumerGroupDescription_coordinator`,
`rd_kafka_ListOffsetsResultInfo_topic_partition`,
`rd_kafka_MemberAssignment_partitions`, `rd_kafka_TopicPartitionInfo_leader`,
`rd_kafka_group_result_partitions`, `rd_kafka_topic_partition_result_partition`,
and `rd_kafka_share_partition_offsets_partition`.

Cross-card summary inference could invalidate this additive prediction. The
combined audit must enumerate every changed function and diagnostic coordinate;
any interaction is a measured miss, not permission to rewrite this forecast.

## Guard-repair checkpoint, before the repaired AFTER

The first combined audit matched the forecast above, but full validation and
review found an unmeasured local provisional-summary admission. B01 and the
shared B02/B03 call-lowering route are being restricted to callees without a
local definition. The original forecast is retained: **8 function moves, 57
removed leaves, zero additions/exchanges/unit moves/retirements**. The named
movers are opaque getters and the externally implemented `cio_chunk_is_up`
call in `flb_input_chunk_down`. The leaf forecast may miss if other changed
conditions call a locally defined callee; the repaired audit must name those
retained refusals rather than silently carrying the intermediate result.
