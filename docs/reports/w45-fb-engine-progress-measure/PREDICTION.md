# W45-FB-ENGINE-PROGRESS-MEASURE: prediction

Frozen before any implementation code. Base `74105dd0e` (local `main`). Only
this file exists in the card's report directory at the freeze.

## 1. The mechanism

Gateway rule 44 records a **checked measure** for a counted `for` loop as a
bound fact beside the resource verdict. It is a new optional property,
selected by the environment variable `QCORE_LOOP_MEASURE_PROPERTY` (which
names its JSON-lines output), in the same way as the bounds
(`QCORE_BOUNDS_PROPERTY`) and null (`QCORE_NULL_PROPERTY`) selections. Unset,
nothing runs and no artifact is written. Set, the rule writes one row per
analysed function, carrying the custody status the report published and,
per loop, either a checked measure or a named refusal. It reads no custody
state, writes no custody state and produces no progress verdict.

## 2. What moves

Nothing. The custody analysis, its contracts, its routes and its report are
untouched, so with the selection unset every route's report is byte-identical
to the base, components sidecar included.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

The three live profiles (`eight-home`, `fluent-bit-system`,
`fluent-bit-tail-pause`) keep their figures.

## 3. The slice's bound facts (with the selection set)

| Loop | Predicted fact |
|---|---|
| `src/flb_engine.c` `handle_output_events` :909 | checked, constant bound from the array-count idiom `sizeof(values) / sizeof(values[0])` over `uint64_t values[FLB_ENGINE_OUTPUT_EVENT_BATCH_SIZE]`, extent read from Clang's type: at most 1 iteration |
| `src/flb_input_chunk.c` `flb_input_chunk_get_direct_routes` :2286 | checked, `stored_count - 0`, a `uint16_t` local never address-taken and not written in the loop: at most 65,535 |
| `src/flb_task.c` `flb_task_create` :652 | see §4 |
| `plugins/in_storage_backlog/sb.c` `cb_queue_chunks` :611 | refused, no counter (a `while` loop) |
| `src/flb_scheduler.c` `ipow` :237 | refused, no counter (a `while` loop; a shift measure would need `exp >= 0`) |
| `src/flb_scheduler.c` `random_uniform` :106 | refused, no counter |

## 4. A predicted deviation from the brief

The brief lists `flb_task_create` :652 as accepted at most 65,535. Its bound
`direct_output_count` is an `int` local whose address is passed to
`flb_input_chunk_get_direct_routes` at :648, before the loop, and the loop
body calls other functions. Fail-closed, the rule refuses a bound whose
address is taken anywhere in the function (a callee may retain the address
and a later call in the body may write through it), and `int` is not a
16-bit type whose range bounds the loop by itself. The prediction is
therefore **refused, `bound-address-taken`**, recorded as a deviation with a
follow-up for an out-parameter non-retention premise.
