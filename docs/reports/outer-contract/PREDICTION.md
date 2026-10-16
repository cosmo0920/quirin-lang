# OUTER-CONTRACT: prediction

Frozen before the measurement. The design card's prediction
([../outer-contract-design/PREDICTION.md](../outer-contract-design/PREDICTION.md),
committed before any implementation) holds for the decided alternative B. This
addendum states it for the rules as implemented, computed by
[predict.py](scripts/predict.py) ([prediction.json](artifacts/prediction.json))
from the BEFORE rows of the design census (main's binary `2d42ddb4…`, on code
identical to the base `a423fca5d`, which moved only records).

## Static scope

**No static verdict moves** on any route, in custody, the null property or the
combined row. The custody report gains, on the rows resting on a member-invoking
declaration or a constructor's stated member, the two dependency lists
`run_set_members` and `stated_members`. With those lists removed, it is
predicted byte-identical to BEFORE. The null rows are predicted byte-identical.
Their summary differs only by the bounds selection record's `load_path_sha256`,
because the load-path manifest gained the outer views. The contract sidecar is
predicted byte-identical.

## Runtime scope (stage 1: `flb_external_plugin` loaded)

There is one contract, so the scenarios `flb_external_plugin` and `all` are
equal. The reach is predicted to be every record: `struct flb_config` holds
`void *` members.

| Route | Custody Safe → Pending / Unknown | Null Safe → Pending / Unknown | Combined Safe → Pending / Unknown |
| --- | ---: | ---: | ---: |
| corpus | 0 / 31 | 1 / 162 | 1 / 31 |
| system | 0 / 0 | 2 / 0 | 2 / 0 |
| slice | 0 / 0 | 16 / 0 | 0 / 0 |

* **Pending (covered).** These are the rows resting on
  `stable:flb_{input,output,filter,processor}_instance.p`:
  * corpus: `flb_chunk_route_plugin_matches`, which is combined Safe;
  * system: the same function in both configurations;
  * slice: the seven scope functions whose entry fact crosses a call, plus
    `flb_input_rate_gate_maybe_resume`, in both configurations.
* **Unknown (uncovered; corpus only).** These are the rows resting on a
  librdkafka closed-world fact that the Fluent Bit envelope does not cover:
  * custody: the 21 rows that list `rk_no_plugin_library`, the 9 `cJSON_*`
    rows (`cjson_allocator_hooks`) and one more. That row rests on
    `cjson_list_invariant` or `served_queue`. The design census counted both
    of those as not closed-world, but the implementation lists only build,
    reading and tracking premises as not closed-world, so the count is 31
    where the design predicted 30.
  * null: 162 rows. They are the 75 rows that list `rk_no_plugin_library`,
    and the rows resting on rule 14t's declared librdkafka members
    (`member:rd_kafka_topic_s.rkt_topic` and four others: 117 Safe rows,
    which the design census did not size), less the overlap.
* **Stated members.** The prediction reads no constructor's stated member, so
  the Unknown counts are lower bounds there.
* **Nothing else.** No row becomes Safe, and no Unknown or Unsafe row moves.
  The system and slice routes analyse no librdkafka unit, so nothing there
  becomes Unknown.

Stage 2 (`RK-PLUGIN-LIBRARY-OUTER-ENVELOPE`) is the step that would turn the
librdkafka rows Pending. It needs an `rk_plugin_library` envelope, and it
needs the Fluent Bit envelope to vouch for librdkafka's facts, because a
Fluent Bit plugin reaches them (design PREDICTION, B-both-cross).
