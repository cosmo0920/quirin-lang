# NULL-WITNESS-GUARD-CORRELATION prediction

Committed after the diagnosis (`a677c25f7`) and before any analyzer, Lean or
test change. **Base side: `rev:13337d0af`**, not a published baseline: the
newest baseline, `c8c7edf21`, lacks this round's merges (among them
LIBRDKAFKA-HEADER-INLINES, whose declarations produced the findings), so it
cannot serve as the before side of this fix. `855b34ade..13337d0af` changes no
analyzer, contract or Lean file; the base's rows are those of the
`855b34ade` landing measurement
(`private/measure/landing-855b34adef125604a6a94535ef15b1dcd7791d0e-20261105T134026829943Z`).

**What changes.** Only the null property's path search (rule 14r): a
lookup's miss (a nullable borrowed-from result) is a NULL event only when it
is feasible on the path by facts the search reads. When the unit holds the
lookup's body and it is a single read of a place over its parameters, the
result is that place: NULL only when the path's own test fixed it NULL (a
finding, as before), otherwise the witness is refused. Without such a body,
the miss is admitted only when every argument is rooted at a constant, a
formal or a global that no earlier decision, store, assignment or call on
the path read or wrote through (a global also with no call before);
otherwise the witness is refused. A refused witness ends its path
unresolved, never infeasible, so no candidate is removed and nothing becomes
`Safe`. Custody is not read by the null property: no custody row moves.

**Corpus null rows (c.null.deref.v1), predicted.** The six rows of the
diagnosis go back to `Unknown` (candidate), each finding's triage
`unresolved` with the refusal named:

| Row | null | combined |
|---|---|---|
| `librdkafka::rdkafka_msg.c::rd_kafka_msgq_allow_wakeup_at` | Unsafe -> Unknown | Unsafe -> Unknown |
| `librdkafka::rdkafka_request.c::rd_kafka_ProduceRequest` | Unsafe -> Unknown | Unsafe -> Unknown |
| `librdkafka::rdkafka_request.c::rd_kafka_handle_idempotent_Produce_success` | Unsafe -> Unknown | Unsafe -> Unknown |
| `librdkafka::rdkafka_request.c::rd_kafka_handle_Produce_metadata_update` | Unsafe -> Unknown | Unsafe -> Unknown |
| `librdkafka::rdkafka_metadata_cache.c::rd_kafka_metadata_cache_entry_new` | Unsafe -> Unknown | Unsafe -> Unknown |
| `librdkafka::rdkafka_topic.c::rd_kafka_topic_info_new_with_rack` | Unsafe -> Unknown | Unsafe -> Unknown |

Corpus null summary: `null` Unsafe 23 -> 17, Unknown 2,349 -> 2,355, both
Safe counts unchanged (1,114 unconditional, 1,435 conditional); `combined`
Unsafe 34 -> 28, Unknown 4,000 -> 4,006; checks `finding` 35 -> 28,
`candidate` 12,638 -> 12,645; reason `null-deref-finding` 23 -> 17; triage
`finding` 35 -> 28 records, `unresolved` 110 -> 117, `infeasible` 9
unchanged. The eight lookup candidates already `unresolved` stay so (their
reasons may now name the refusal). No other row is predicted to move: no
other tip finding's witness reads a lookup, and a refused lookup tested on
another candidate's path can only turn a finding into `unresolved`. System
and slice have no lookup source: no null move there.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The measure tool compares the custody rows (`resource-custody.v1`), which
this card does not touch; the null figures above are checked from each
side's `corpus-null.jsonl` and `corpus-null.summary.json`.

**The user's `current` run** (Fluent Bit `046accf6`) publishes custody only
(no null report): no callee leaves the uncontracted list, no function leaves
the "only uncontracted" set, no unit status changes.
