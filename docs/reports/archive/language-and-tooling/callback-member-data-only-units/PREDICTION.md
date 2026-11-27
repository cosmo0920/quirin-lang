# CALLBACK-MEMBER-DATA-ONLY-UNITS: prediction

Frozen before any implementation code, on base `main` `f3e1d724c`
(corpus 935 / 9 / 3,890, report `527a4cad…`; system 93 / 4 / 351; slice
52 / 1 / 347 / 4 Pending; null property corpus 1,113 / 1,435 / 2,355 / 18
Unsafe, system 84 / 125 / 245, slice 71 / 103 / 232).

## Census: why the four units record no facts

Rule 12n's facts reach the fold through one hook: rule 13y's `settle`
(`legacy_callback_route.rs`) records the unit's `MemberFacts` with
`contract_dependencies::member_evidence`, and `projection::record_projection`
hands them to `AuditReport::record_callback_members`. Both run only for a unit
the gateway analyses. `projection::project_prepared_unit_into` (and
`project_unit_into`) return early for a unit whose definition inventory is
empty (`counts.data_only_units`, `record_data_only`), before any analysis, so
no facts are recorded and the fold pairs the row with nothing: "N analysed
unit(s) recorded no facts" opens every member.

| Unit | Why it is data-only | What its AST holds for a function-pointer member |
| --- | --- | --- |
| `ctraces/ctr_utils.c` | the file is empty (0 bytes at the corpus revision) | nothing: only Clang's built-in declarations |
| `librdkafka/opentelemetry/common.pb.c` | nanopb descriptors only (`PB_BIND`, `PB_STATIC_ASSERT`) | initializer lists of `pb_msgdesc_t` (`struct pb_msgdesc_s`), whose `field_callback` member names `pb_default_field_callback` in some descriptors |
| `librdkafka/opentelemetry/metrics.pb.c` | the same | the same |
| `librdkafka/opentelemetry/resource.pb.c` | the same | the same |

None of the four stores into, initializes, or takes the address of a
`struct rd_kafka_op_s` member: they contribute nothing to either declared
member's closure.

## The change

Read each data-only unit's whole Clang AST (the unit's preprocessed text, its
dialect and frontend flags, exactly the input an analysed unit's index is read
from) and collect `MemberFacts` from it with the same collector rule 12n's
in-unit decision uses. A unit whose preprocessing failed, whose Clang run
fails, or whose JSON cannot be read records `None` and still opens every
member. Initializer lists of a record whose field list the unit defines map
each entry to its field (a designated or positional initializer of a
function-pointer field is a store of that value into `TAG.FIELD`); a list that
cannot be mapped keeps today's reading (the record is opened). No promotion:
the fold stays demote-only.

## Expected outcome

* **Fold section (`callback_members`, corpus sidecar):** `units_without_facts`
  4 → 0, `units_without_facts_named` empty; `rd_kafka_op_s.rko_op_cb` closed
  with the 27 candidates `members_over_recorded_units` lists today, and
  `rd_kafka_op_s.rko_serve` closed with its 4. `demoted` stays empty.
* **Verdicts:** no move on any route. No row rests on a decided member
  invocation (no contract exists for any candidate yet), so the fold demotes
  nothing either way. Corpus 935 / 9 / 3,890 with `resource-diagnostics.json`
  byte-identical (`527a4cad…`: the section is in the sidecar, not the report);
  system 93 / 4 / 351 and slice 52 / 1 / 347 / 4 Pending byte-identical.
* **Null property:** byte-identical rows and tallies on every route (the
  change reads no null evidence).
* **Wall time:** one additional whole-unit Clang AST read per data-only unit,
  and only when a declaration invokes a member; small against the arbiter's
  ~500 s.
* **System and slice sidecars:** each route's section closes wherever its
  data-only units were the only reason it was open; otherwise unchanged.
