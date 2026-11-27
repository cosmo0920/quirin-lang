# FB-CHUNK-LIFECYCLE-CONTRACTS prediction

Committed after the probe (`88cbbe592`) and before the semantic change.
Base `56584db0b` (main, r82). The base census is [base/CENSUS.md](base/CENSUS.md).

**What changes.** One class (a) contract, the only callee the census found
statable from its own body without re-admitting an earlier refusal:
`flb_input_name` (`src/flb_input.c:1143-1150`), declared in
`fluent_bit/input_chunk_name.qv` as `resource_borrowed(ins)` and
`resource_return_borrowed_from(ins, nullable)`. No gateway rule, schema or
route changes.

**What it reaches at base.** `call to uncontracted function
`flb_input_name`` leaves:

* system route: 8 leaves in 7 rows. Lifecycle rows:
  `flb_input_chunk.c::get_input_chunk_source_name` (`:537`),
  `input_chunk_append_raw` (`:3296`), `update_logs_tag_records_metrics`
  (`:154`). Other units: `flb_storage.c::metrics_append_input` (2),
  `flb_engine.c::handle_dlq_if_available`, `flb_engine.c::handle_output_event`,
  `flb_engine_dispatch.c::record_retry_failure_metrics`.
* opt-in current: the same three `flb_input_chunk.c` rows (3 leaves).

Every one of those rows has other leaves (1 to 70), and no row's only leaf is
this call, so none can become Safe.

**Predicted leaves.** Each of those uncontracted-call leaves leaves the row.
`get_input_chunk_source_name` trades it for `unmodeled call, aggregate or
indirect access in return expression` at the same line (its actual is the
member read `input_chunk->in`; reproduced in the card's fixture
`the_source_name_shape_trades_its_uncontracted_call`). For the other rows a
trade for another refusal at the same line is possible and is not predicted
leaf by leaf; the existing `nested call `flb_input_name` inside a modeled
call` leaves (`input_chunk_append_raw` 3, `flb_storage_input_create` 2) stay.
No other leaf moves.

**Predicted statuses: no move on either route.** Totals unchanged: system
116 / 0 / 328 / 4; opt-in current 1,205 / 2 / 3,590 / 7.

```measure-prediction
route system
none
```

The opt-in current route (not a profile route) is predicted the same: no
status move, the three `flb_input_chunk.c` leaves as above.
