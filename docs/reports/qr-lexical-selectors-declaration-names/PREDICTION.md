# QR-LEXICAL-SELECTORS-DECLARATION-NAMES: prediction

Frozen before any change to `crates/`, `providers/` or `proof/`. Base: local
`main` at `4b5c77041`. Written after the census (task item 1) and before the
fix (item 2), as the brief orders.

## The census, measured on the base

`compiler/tests/qr_lexical_selector_census.rs` `declaration_name_census`
(ignored; `scripts/selector-census.sh before`, exit 0,
`private/qrls/selector-census-before.jsonl`; compared by
`scripts/selector_compare.py`, artifact
[`artifacts/selector-census-before.json`](artifacts/selector-census-before.json)).
62 selector words × 3 modes × 2 bases × 7 declaration shapes, 48 controls
(`flb_noop`), and the `throws<ArithmeticError>` extern-row variant: **2,658
rows**. Every row parses (no selector word is refused as a name by the
parser), and no row's compile outcome differs from its control's without its
selection differing.

**321 rows move a selection** (selector: rows): the `Q015` import gate 264,
`has_source_tasks` 126, `has_scalar_calls` 45, `has_source_epoch` 12.

| words | declaration shapes | modes × bases | selection moved |
| --- | --- | --- | --- |
| `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write` | all 7 | all 6 | `Q015` gate and the task route (126) |
| `qcore_error_payload`, `spawn`, `join` | all 7 | all 6 | `Q015` gate (126) |
| `task`, `mutex` | type argument `x: W<i32>` of an extern parameter | all 6 | `Q015` gate (12) |
| `epoch` | record name, error name (`epoch {`) | all 6 | the epoch route (12) |
| `qcore_acquire`, `qcore_check` | all 7 | scalar base, all 3 modes | `has_scalar_calls` deselected (42) |
| `throws<ArithmeticError>` | an extern declaration's row | plain base, all 3 modes | `has_scalar_calls` selected (3) |

Not moved by any declaration: `print` (the print card's fix), `mutex guards` /
`mutex rank` (no declaration spells the pair), `bits<`/`field<` (read from a
record's own field types, and conjoined with a task capture), and every
function-body statement selector (std-io, buffer, string, socket, event,
HTTP, extern call, task and lock statements, FFI declarations).

**Ledger claims that do not reproduce, by reading.** `source_composed`'s
"line words" are the statement heads of the entry's body
(`statement_head` over body statements), not a token-stream test, and the
buffer and socket intrinsic **routes** (`buffer_uses`, `socket_uses`) read
function-body statements only; the buffer intrinsics that a declaration name
moves are the `Q015` gate's and `has_source_tasks`'s `qcore_buffer_*` words.
The whole-token selectors are `concurrency_span`'s token loop,
`has_source_tasks`, `has_source_epoch`, `has_source_mutex` and
`has_scalar_calls`'s two lexical tests (plus the print card's selector).

Tracked positions (`every_tracked_selector_word_stands_where_pinned`, always
run, passing on the base): every tracked selector occurrence is inside a
source function (body, or its signature's effect row / result type / throws
row) except two contract clauses (`join` in libc `pthread_sync.qv`, `spawn` in
monkey `callback_task_handoff.qv`) and cjson's `extern fn print`. No tracked
source names a source function or parameter after a selector word.

## The fix (planned)

One shared reading, `qcore_frontend::route_selection::SelectorScope`: a
token-stream selector reads a token only inside a **source function
declaration** (its body, its parameter and result types, its effect and
throws rows), never a source function's own name or a parameter's name,
never an extern declaration, record, error, alias or other declaration, a
contract clause or a library binding. Applied to `concurrency_span`'s token
loop, `has_source_tasks`, `has_source_epoch`, `has_source_mutex` (uniformly,
though no declaration moves it) and `has_scalar_calls`'s two lexical tests;
`print_call_selection` reuses its body spans. Not body-only: the tracked
effect rows (`effects { spawn, join }`, 154 files) and `throws` rows are read
by the gate and the scalar selector today and must stay read.

Fail-closed arms: a module that declares a name a selecting use in a source
function spells (`extern fn spawn` beside `spawn w(...)`) is refused at
selection (`Q020`), never routed; and a `module contract;` module selected by
any source-only route (every route but the sequential one) is refused
(`Q020`), generalising the print card's scalar-route guard.

## Predicted outcome

* Synthetic census AFTER: **0** rows move a selection, 0 outcome-only rows;
  every row reads as its control.
* Tracked route census (718 sources × 2 configurations, 1,436 rows):
  **0** moved; routes, outcomes and emitted C byte-identical. The pin above
  unchanged.
* No tracked module reaches the ambiguity or the contract-route refusal.
* Corpus, system and slice arbiters byte-identical to BEFORE (binary
  `6c6fbcb5…`): corpus **935 / 9 / 3,890** `bdf5f1c8…`, system 93 / 4 / 351
  `f596d64b…`, slice 52 / 1 / 347 / 4 Pending; null property identical.
  (No contract text changes; contract modules stay on the sequential route.)
* Lean: `QCore.PrintRouteSelection` generalised by a route-selection model in
  which no declaration-name token selects any selector and a contract module
  reaches no source-only route; registered audit, no placeholders, no axioms.
