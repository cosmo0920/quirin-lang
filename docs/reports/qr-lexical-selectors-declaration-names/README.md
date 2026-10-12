# QR-LEXICAL-SELECTORS-DECLARATION-NAMES: route selectors read no declaration name

Card QR-LEXICAL-SELECTORS-DECLARATION-NAMES (wave 48, card-high, 2026-10-12),
opened on the user's request. Base: local `main` at `4b5c77041`, which it
stayed at. [PREDICTION.md](PREDICTION.md) was committed at `687fa3732`, after
the census and before any change to `crates/`, `providers/` or `proof/`.
Authority: the ledger entry of the same name (registered by
[QR-PRINT-ROUTE-MATCHES-DECLARATION](../qr-print-route-matches-declaration/README.md)).
No new rule id (`15v`–`15y` unused): constraint 14 gains an amendment in
[source-routes](../../agents/source-routes.md) and SOURCE-SCALAR-CALLS-V1 a
pointer to it.

## 1. The census (task item 1)

**Which selectors read the token stream**, by reading `compile_elaborated_source`
(`crates/qcore-verification/src/driver.rs`) and every predicate it calls, and
`check_source_imports_in_scope`:

| selector | what it read on the base |
| --- | --- |
| `module_imports::concurrency_span` (the `Q015` gate) | any `qcore_buffer_new`/`_read`/`_write`, `qcore_error_payload`, `spawn`, `join`; `task <`, `mutex <`, `mutex guards`, `mutex rank` — anywhere but a library binding or a function contract |
| `source_tasks::has_source_tasks` | any `qcore_buffer_*` word but in a library binding |
| `source_epoch::has_source_epoch` | any `epoch {` |
| `source_mutex::has_source_mutex` | any `mutex guards` / `mutex rank` |
| `source_scalar_calls::has_scalar_calls` | any `qcore_acquire`/`qcore_check` deselects; any `throws < ArithmeticError` selects |
| `print_call_selection` | fixed by the print card |
| `has_source_bitfield_tasks` | `bits <`/`field <` in a record's own tokens, conjoined with a task capture |

Every other selector and gate reads function-body **statements**
(`std_io_uses`, `rendering_span`, `buffer_uses`, `string_uses`, `socket_uses`,
`http_uses`, `source_event_span`, `called_extern_names_in_scope`) or a
declaration's kind (`has_source_record`, `has_resource_calls`,
`has_source_ffi`). **Corrections to the ledger entry** (claims that do not
reproduce): `source_composed`'s "line words" are the entry body's statement
heads, not a token-stream test; and the buffer and socket **routes** read body
statements only — the "buffer intrinsics" a declaration moves are the gate's
and `has_source_tasks`'s `qcore_buffer_*` words.

**The synthetic census**
([`compiler/tests/qr_lexical_selector_census.rs`](../../../compiler/tests/qr_lexical_selector_census.rs)
`declaration_name_census`, run by [`selector-census.sh`](scripts/selector-census.sh),
compared by [`selector_compare.py`](scripts/selector_compare.py)): 62 words
(every word above plus every body-statement selector's vocabulary) × three
modes (`module contract;` `.qv`, `module library;` and executable `.qr`) × two
bases (no source call; a call `has_scalar_calls` selects) × seven declaration
positions (`extern fn` name with and without an inline contract, source
function name, record name, error name, a parameter name a contract clause
names, a type constructor in an extern parameter type), plus the extern
`throws<ArithmeticError>` row; each row against a **control** that names
`flb_noop`. It records whether the module parses, every selector's value and
the import gate in driver order, and the compile outcome. 2,658 rows.

| | rows moving a selection | by selector |
| --- | ---: | --- |
| base ([artifact](artifacts/selector-census-before.json)) | **321** | gate 264, tasks 126, scalar 45, epoch 12 |
| fix and final tip ([artifact](artifacts/selector-census-fix.json); the final tip's rows are byte-identical to the fix's) | **0** | — |

On the base: the three `qcore_buffer_*` words in every position and mode moved
the gate and the task route (126 rows); `qcore_error_payload`, `spawn`, `join`
the gate (126); `task`/`mutex` as an extern parameter's type constructor the
gate (12); a record or error named `epoch` (`type epoch {`) the epoch route
(12); `qcore_acquire`/`qcore_check` in any declaration deselected the scalar
route on the scalar base (42); the extern `throws<ArithmeticError>` row
selected it (3). Nothing is refused at parse, and no body-statement selector,
`print`, `mutex guards`/`rank` or `bits<`/`field<` moved. The print card's
probe reproduces as row `contract/plain/extern-contract/qcore_buffer_new`.

After the fix two rows differ from their controls in outcome alone: the
`throws<ArithmeticError>` row on the plain base in library and executable mode
is refused `Q020` by the extern-declaration reader (`i32 throws<ArithmeticError>`
is not a type an extern declaration can state), where its control (no row) is
admitted. That control is not the same text with a word replaced; no selector
reads the row. `no_declaration_moves_a_selection` (always run) pins all of
this: every row reads as its control, that row's refusal excepted by name.

Tracked positions (`every_tracked_selector_word_stands_where_pinned`, always
run, green on the base and the fix): every tracked selector occurrence is
inside a source function — its body or, for `spawn`/`join` (154 files each,
`effects { …, spawn, join }`), `task<`/`mutex<` (two archived probes' result
types) and `throws<ArithmeticError>` (4 examples), its signature — except
`join` in a libc contract clause, `spawn` in a monkey contract clause and
cJSON's `extern fn print`. No tracked source names a function or parameter
after a selector word. This is why the scope is not the body alone.

## 2. The fix (task item 2)

**One reading for every token-stream selector**, the new
`qcore_frontend::route_selection::SelectorScope`: a token is read only inside a
**source function declaration** (body, parameter and result types, effect and
`throws` rows), never that function's own name, a parameter's name, a contract
clause, a library binding, or any other declaration. Hooks (thin, each one
`scope.reads(…)` test): `concurrency_span`'s token loop (both previous
exclusions kept), `has_source_tasks`, `has_source_epoch`, `has_source_mutex`
(uniform; no declaration moved it), both lexical tests of `has_scalar_calls`;
`print_call_selection` reuses the scope's body spans instead of computing its
own. Comments are not tokens and string literals are not words, before and
after.

**Fail-closed arms** at `compile_elaborated_source`, after the closure refusal:

1. `ambiguous_selector_use`: a selecting use of a selector word in a module
   that also declares that word (`fn qcore_acquire()` beside `let a =
   qcore_acquire(…)`) is refused `Q020` at the use — the print card's "call
   beside a declared `print`" rule for every token-stream selector.
2. `source_only_route`: a `module contract;` module that any route but the
   sequential one would select is refused `Q020`, naming that route
   (generalising the print card's guard at `compile_scalar_calls`; the same
   message for the scalar route).
3. A completeness check after routing (`compile_files_with_registry_inner`): a
   contract module admitted on any route but the sequential one is refused.
   Arm 2 restates the driver's order; this check does not depend on it.

Reachability (stability item 1): arms 1 and 2 are driven by
[`qr_lexical_selector_selection.rs`](../../../compiler/tests/qr_lexical_selector_selection.rs)
and the census (56 synthetic contract modules with a record move from the
buffer route's `Q058` to arm 2's `Q020`). Arm 3 and the print card's guard are
reached by no fixture, since arm 2 catches every contract module first;
registered as `ROUTE-SELECTION-ORDER-RESTATED`.

Outcome changes of the fix on the synthetic modules
([artifact](artifacts/selector-census-before-fix.json), controls included):
337 rows, every one now its control's outcome: 132 refusals become admissions
(a `Q015`, `Q500`, `Q600` or `Q020` that a name caused), the rest a refusal
for a different reason than the name (the route the control reaches refuses
the same module, or arm 2).

## 3. Nothing tracked moves (task item 3)

The print card's tracked route census
([`qr_print_route_census.rs`](../../../compiler/tests/qr_print_route_census.rs),
[`census.sh`](scripts/census.sh), [`census_compare.py`](scripts/census_compare.py)):
718 tracked `.qr`/`.qv`/`.qcore`/`.qcorelib` sources × two configurations.

| comparison | rows | moved |
| --- | ---: | --- |
| base → fix (`74297fb5e`) | 1,436 | **0**, the two streams byte-identical (`b8171eda…`; [artifact](artifacts/census-before-fix.json)) |
| base → final tip (`7d2216f28`) | 1,436 | **0**, byte-identical ([artifact](artifacts/census-before-after.json)) |

No fixture changes route, so there is no correction to name; no tracked module
reaches either refusal arm (214 contract modules stay `Contract`/`Sequential`).

## 4. Lean (task item 4)

New `QCore.RouteSelection`, audited in `AuditRouteSelection.lean`,
generalising `QCore.PrintRouteSelection` (kept as the print selector's finer
model). A module is its token stream with each token's position; the six
token-stream tests (gate, tasks, epoch, mutex, scalar deselect, scalar throws)
are modelled as the Rust tests are. Theorems: `test_holds_only_at_a_head`,
`fires_only_at_a_read_site`, **`declaration_name_never_selects`** (for every
selector), **`declaration_names_reach_sequential`** (no gate, no refusal, no
source-only route, in every mode), `ambiguous_use_refuses`,
**`contract_module_never_compiled`**, and nine vectors (the buffer-new
contract, a record named `epoch`, a function named `spawn` → sequential; an
effect row without the import → `Q015`; a body `qcore_buffer_new` → tasks, and
refused when declared or in a contract module; a source `throws` row → scalar,
the extern row → sequential). Axioms: `propext`, `Quot.sound` only. Premise
(trust boundary 28): the Rust code computes the positions from the frontend's
spans and applies these tests; agreement is checked by the census, not proved.

## 5. Measurement (task item 5)

Binaries: BEFORE `private/qrls/bin/before` (`6c6fbcb5…`, the base), fix
`private/qrls/bin/fix` (`bab7cffd…`, `74297fb5e`), AFTER
`private/qrls/bin/after` (`4bba439b…`, `7d2216f28`, adding arm 3); each corpus
`run.json`'s `binary_sha256` equals its binary. Every route under the shared
lock, `--jobs 4`, the null property selected
([`measure.sh`](scripts/measure.sh), [`runs.py`](scripts/runs.py)).

| run | corpus | system | tail-pause slice |
| --- | --- | --- | --- |
| BEFORE | 935 / 9 / 3,890, `bdf5f1c8…` (the brief's) | 93 / 4 / 351, `f596d64b…` | 52 / 1 / 347, 4 Pending, `ee98ad3d…` |
| fix | byte-identical | byte-identical | byte-identical |
| AFTER | 935 / 9 / 3,890, `bdf5f1c8…`, byte-identical | byte-identical | byte-identical |

Null property: corpus 1,113 Safe unconditional / 1,435 conditional / 2,355 Unknown / 18 Unsafe; system 84 / 125 / 245; slice 71 / 103 / 232 — every null stream byte-identical to BEFORE's on both runs ([runs-fix](artifacts/runs-fix.txt),
[runs-after](artifacts/runs-after.txt)). Corpus wall time 518.6 s BEFORE,
527.2 s fix, 532.0 s AFTER.

Prediction against outcome: every prediction held (321 → 0 moved selections;
0 outcome-only rows except the two `throws`-row refusals by the declaration
reader, which the prediction did not foresee; tracked census 0 moved; arbiters
and null streams byte-identical).

## 6. Gate

GATE_SECTION

## 7. Follow-ups

* Retired: `QR-LEXICAL-SELECTORS-DECLARATION-NAMES` (its DISCHARGE holds: the
  census of each whole-token selector against declarations named after its
  words, one synthetic module per word, mode, base and position; a scoped
  reading for every selector a declaration moved, plus the contract guard;
  the route census unchanged).
* Registered: `ROUTE-SELECTION-ORDER-RESTATED` — arm 2's 21-entry selector list
  restates the driver's route order, and arm 3 and the print card's guard are
  driven by no fixture.

Ledger: [`scripts/update_ledger.py`](scripts/update_ledger.py); follow-ups 763
→ 763.
