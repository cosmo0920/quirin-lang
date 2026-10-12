# ROUTE-SELECTION-ORDER-RESTATED: the route order is written once

Card ROUTE-SELECTION-ORDER-RESTATED (wave 48, card-high, 2026-10-12). Base:
local `main` at `1a399e6a1`, which it stayed at. [PREDICTION.md](PREDICTION.md)
was committed at `4e94f1d92`, before any change to `crates/`, `compiler/` or
`proof/`. Authority: the ledger entry of the same name (registered by
[QR-LEXICAL-SELECTORS-DECLARATION-NAMES](../qr-lexical-selectors-declaration-names/README.md)).
New rule **15v** in [source-routes](../../agents/source-routes.md#constraint-15v)
(`15w` unused); constraint 14's QR-LEXICAL amendment amended in place.

## 1. What the base did (reproduced by reading)

`compile_elaborated_source` (`crates/qcore-verification/src/driver.rs`) wrote
the route order three times: the closure refusal's if-chain over the first
seven selectors (card `W48-QR-CLOSURES`), the contract guard's 21-entry list
`route_selection::source_only_route` plus a closure entry, and the route
if-chain (21 arms and the sequential fallback). The ledger's claims reproduce,
with two findings the ledger did not state:

* the list had already **drifted** from the chain by one arm: the
  socket-and-event composition (`SOCKET-EVENT-COMPOSE`, lowered by the socket
  route) had no entry, so a contract module that dispatches and sockets was
  refused naming `source.event.v1`, the next entry its conjunction implies;
* the list's closure entry was unreachable: a module with a closure is
  refused `Q600` before the guard unless the composed-task or task selector
  holds, and both stand earlier in the list.

Neither the completeness check after routing nor the print card's guard in
`compile_scalar_calls` was reached by any test or census row (the ledger's
claim; confirmed by the deletion below changing no row).

## 2. The table (task item 1)

[`driver/source_routes.rs`](../../../crates/qcore-verification/src/driver/source_routes.rs),
a child module of the driver: `SOURCE_ROUTES`, 22 rows in the chain's order,
each the route's **name** (as the guard states it), its **selector**
(`Absent`, `Selected`, or `Refused` at selection: the print selector's
ambiguous pair), its **lowering entry**, whether it is **source-only**, and its
**closure rule** (hosts closures, or the `Q600` refusal it gives). `select`
returns the first row whose selector is not `Absent`; the last row, the
sequential route, always selects. `compile_elaborated_source` now selects once
and reads that one row, in the base's order: the closure refusal, the
ambiguous-use refusal (`route_selection::ambiguous_selector_use`, not a
selector, unchanged), the contract guard (`mode == Contract` and the row is
source-only, `Q020` naming the row), the row's selection refusal, then its
lowering. The chain's comments moved onto the rows they explain.
`route_selection::source_only_route` and the closure if-chain are deleted.

**Selection is unchanged**: the rows are the chain's arms in the chain's
order, each with the arm's condition and lowering; first match over a list is
the if-chain in list order (Lean `choose_eq_chain`); selectors are pure, so
evaluating them in order up to the first match is what the chain did. The
order is pinned by `source_routes::tests::the_table_is_the_pinned_order` and,
row for row, by Lean `driverTable`.

**One guard message changes**, by design and only for the composition the
list missed: a `module contract;` module that dispatches and sockets is now
refused naming `source.socket.v1` (the route that lowers it) instead of
`source.event.v1` — a named correction, reached by the new fixture
`a_contract_module_that_dispatches_and_sockets_is_refused_naming_the_socket_route`
and by no census row (no tracked or synthetic module dispatches).

## 3. Every remaining arm (task item 2)

| arm | outcome | argument or fixture |
| --- | --- | --- |
| completeness check after routing (`compile_files_with_registry_inner`) | **deleted** | The guard reads the row that is lowered, so a contract module is lowered only by a row that is not source-only; the one such row is the sequential route (pinned: `only_the_last_entry_reads_contract_modules`), whose lowering builds `CheckedSourceModule::Sequential` on every admission. Lean `contract_lowered_only_on_last`, `driver_contract_lowered_only_on_sequential`. |
| print card's guard in `compile_scalar_calls` | **deleted** | Its four callers are the lowering entries of four source-only rows (records → scalar, print, std-io's scalar branch, scalar calls); the guard refuses a contract module on each before its lowering runs. Lean `contract_never_lowered_on_source_only`. |
| contract guard | kept | Existing fixtures (`qr_lexical_selector_selection.rs`, `qr_print_route_selection.rs`) plus [`route_selection_table.rs`](../../../compiler/tests/route_selection_table.rs): the scoped-task row, the socket-and-event row, and a contract module on the sequential row admitted. |
| closure rule, FFI and field-footprint rows | kept | Driven for the first time: `a_closure_on_the_ffi_route_meets_that_entry_rule`, `a_closure_on_the_field_footprint_route_meets_that_entry_rule` (the epoch and mutex rows by `closures.rs`; a hosted closure by `a_closure_on_the_task_route_is_hosted`). |
| closure rule, general refusal (rows 7–21) | kept, **undriven** | The base's `else` arm. By reading unreachable (a closure is lifted only at a `spawn SCOPE \|…\|` head, which makes the task selector hold), but nothing tests that; registered as `ROUTE-TABLE-CLOSURE-GENERAL-RULE-UNDRIVEN`. Fail-closed either way. |
| selection refusal (ambiguous `print (`) | kept | `qr_print_route_selection.rs`. |

## 4. Identity (task item 3)

Binaries and censuses ([`stage.sh`](scripts/stage.sh), [`before.sh`](scripts/before.sh),
each step under the shared lock): BEFORE `private/rsor/bin/before`
(`93ac37cf…`, the base); AFTER `private/rsor/bin/after` (`8c6d494a…`,
`7f61b2481`, the tip's code). Each corpus `run.json`'s `binary_sha256` equals
its binary; `--jobs 4`, the null property selected.

| measurement | BEFORE | AFTER |
| --- | --- | --- |
| tracked route census (718 sources × 2 configurations) | 1,436 rows, stream `b3b5ae1b…` | **byte-identical**, 0 moved ([artifact](artifacts/census-before-after.json)) |
| synthetic selector census | 2,658 rows, stream `be404107…` | **byte-identical** (105 selection-moved rows, the base's record-name and type positions; [artifact](artifacts/selector-census-after.json)) |
| corpus arbiter | 935 / 9 / 3,890, `bdf5f1c8…` | **byte-identical** |
| system route | 93 / 4 / 351 | **byte-identical** |
| tail-pause slice | 52 / 1 / 347, 4 Pending | **byte-identical** |
| null property (corpus; system; slice) | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | every stream **byte-identical** |

[runs-after](artifacts/runs-after.txt) has the per-route comparison. The
corpus totals and digest are the brief's; the system and slice reports and the
tracked census embed the worktree path, so their digests are this worktree's
(totals equal the brief's). Corpus wall time 519.3 s BEFORE, 533.7 s AFTER.
No row moved, so there is no correction to name beyond the guard message of
section 2, which no census row reaches.

Prediction against outcome: as predicted, except the ledger: the prediction
expected 763 → 762, and the card registers the residual it found (section 3),
so 763 → 763.

## 5. Lean (task item 4)

`QCore.RouteSelection` gains a selector-table section, audited in the existing
`AuditRouteSelection.lean` (18 more `#print axioms`; the audit count stays
315). For every table of rows (name, selector over an abstract module,
source-only, hosts closures) and the driver's reading `dispatch` (closure
refusal, then guard, then lowering): `choose_eq_chain` (first match is the
if-chain in table order); **`guard_refuses_iff_source_only`** (a contract
module is refused by the guard exactly when its selected row is source-only,
naming it); `added_entry_is_guarded`; `lowered_is_chosen` (the guard and the
lowering read one row); **`contract_never_lowered_on_source_only`**;
**`contract_lowered_only_on_last`** (every other row source-only ⇒ a contract
module is lowered only on the last); `closure_lowered_only_on_hosting`;
`choose_total_of_last`. For `driverTable` (the Rust rows):
`driver_contract_lowered_only_on_sequential`,
`driver_closure_lowered_only_on_task_routes`, and the vectors
`dispatch_and_socket_contract_names_socket`, `restated_list_named_event` (the
base's list named the event route), `dispatch_and_socket_executable_lowered`.
Axioms `propext` and `Quot.sound` only, no placeholders. Premise (trust
boundary 28): the Rust rows are `driverTable`'s (pinned by the unit tests) and
the selectors are pure.

## 6. Gate

[`gate.json`](gate.json), exits in [`gate/`](gate/); scripts
[`gate.sh`](scripts/gate.sh), [`write_gate_json.py`](scripts/write_gate_json.py),
[`finish-gate.sh`](scripts/finish-gate.sh); every step under the shared lock,
nothing piped, logs under `private/rsor/`.

Standing gate **g1**, every code step at `3acebdd4d` (the code is that of
`7f61b2481`): `cargo check --workspace --all-targets` 0, submodule 0, fmt 0,
clippy `--locked` 0; `run_test_binaries.py` 486 units, 499 summaries,
**7,928 passed, 0 failed**, 14 ignored, exit 0 (`private/rsor/tests-gate-g1/`;
the base's 7,917 plus the seven fixtures and four table pins); both
reevaluation probes 0; `check_lean_audits.py` **315** passed; the Python
suite **365 OK** (08, and 09b after `build_matrix.py --gate gate.json`, 09a,
0); `build_docs_index.py` and `--check` 0. Clang present (Ubuntu clang
21.1.8). No provider gate: `providers/` is unchanged.

## 7. Follow-ups

* Retired: `ROUTE-SELECTION-ORDER-RESTATED` — both DISCHARGE branches hold:
  one table read by the route selection and the contract guard, and each
  remaining arm either driven by a fixture or deleted with its argument.
* Registered: `ROUTE-TABLE-CLOSURE-GENERAL-RULE-UNDRIVEN` (section 3).

Ledger: [`scripts/update_ledger.py`](scripts/update_ledger.py); follow-ups 763
→ 763.
