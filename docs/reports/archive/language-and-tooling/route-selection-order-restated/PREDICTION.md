# ROUTE-SELECTION-ORDER-RESTATED: prediction

Frozen before any change to `crates/`, `compiler/` or `proof/`. Base: local
`main` at `1a399e6a1`. Authority: the ledger entry of the same name
(registered by
[QR-LEXICAL-SELECTORS-DECLARATION-NAMES](../qr-lexical-selectors-declaration-names/README.md)).

## What the base does, by reading

`compile_elaborated_source` (`crates/qcore-verification/src/driver.rs`) writes
the route order **three times**:

1. the closure refusal (card `W48-QR-CLOSURES`): an if-chain over the first
   seven selectors (FFI, field footprint, composed epoch, epoch, mutex,
   composed tasks, tasks), naming the refusal of the first that holds;
2. the contract guard, `qcore_frontend::route_selection::source_only_route`:
   a 21-entry list of the driver's selectors in the driver's order, plus a
   closure entry;
3. the route if-chain itself: 21 arms and the sequential fallback.

The list (2) and the chain (3) agree entry for entry with two exceptions,
both read rather than run:

* the chain's socket-and-event composition arm (`SOCKET-EVENT-COMPOSE`,
  `has_source_event ∧ has_source_socket ∧ ¬buffer ∧ ¬string ∧ ¬http`, lowered
  by `compile_socket_source` under the name `source.socket.v1`) is absent from
  the list; its conjunction implies the list's next entry (`has_source_event`,
  named `source.event.v1`), so the guard refuses such a contract module, but
  names the event route rather than the socket route that would compile it;
* the list's last entry ("a closure's scoped-task route") is unreachable: a
  module with a closure is refused `Q600` by (1) before the guard unless the
  composed-task or task selector holds, and those two stand earlier in the
  list.

Two fail-closed arms stand behind the guard and are reached by no fixture:
the completeness check after routing (`compile_files_with_registry_inner`: a
contract module admitted on any checked variant but `Sequential`) and the
print card's guard at the head of `compile_scalar_calls`.

## The change (planned)

One table, `driver::source_routes::SOURCE_ROUTES`, in the driver's route
order: each entry is the route's name, its selector (absent, selected, or
refused at selection: the print selector's ambiguous pair), its lowering entry,
whether it is source-only, and its closure rule (hosts closures, or the
refusal a closure receives). `compile_elaborated_source` computes the
selected entry **once** (first match, in table order; the last entry, the
sequential route, always selects) and reads it for all three uses, in the
base's order: the closure refusal, the ambiguous-use refusal (unchanged,
not a selector), the contract guard (`mode == Contract` and the entry is
source-only), the entry's selection refusal, then its lowering.
`route_selection::source_only_route` is deleted.

Arms made unreachable by construction, to be deleted with the argument stated:
the completeness check (a contract module is lowered only by an entry that is
not source-only; the table's one such entry is the sequential route, whose
lowering builds `CheckedSourceModule::Sequential` on every admission) and the
guard in `compile_scalar_calls` (its four callers are lowering entries of
source-only rows). A unit test pins the table's order, names, flags and
closure rules; Lean extends `QCore.RouteSelection` with a table model: first
match equals the if-chain in table order, the guard refuses a contract module
exactly when the selected entry is source-only, an entry added anywhere is
guarded, and a contract module is lowered only on the sequential entry.

## Predicted outcome

* **Route selection unchanged** for every module: same order, same first
  match, same lowering per arm.
* **Tracked route census** (718 sources × 2 configurations, 1,436 rows):
  **0 moved**, byte-identical to BEFORE (no tracked contract module reaches a
  source-only route; 214 stay `Contract`/`Sequential`).
* **Synthetic selector census** (2,658 rows): **byte-identical** to BEFORE.
  No census module both dispatches and sockets, so the one renamed guard
  message is reached by none of its rows (checked on the BEFORE stream).
* **One guard message changes**, reached only by a new fixture: a
  `module contract;` module that dispatches and sockets is refused `Q020`
  naming `source.socket.v1` (the route the table would lower it on) instead of
  `source.event.v1`. A named correction, not a selection change.
* **Corpus, system and tail-pause slice arbiters**: byte-identical (935 / 9 /
  3,890; 93 / 4 / 351; 52 / 1 / 347 / 4 Pending); null-property streams
  byte-identical. The C audit does not compile Quirin sources.
* Lean: audit count unchanged at 315 (the existing `AuditRouteSelection`
  gains theorems); no placeholders, no project axioms.
* Ledger: `ROUTE-SELECTION-ORDER-RESTATED` retired (both DISCHARGE branches:
  one table, and each remaining arm driven or deleted); follow-ups 763 → 762.
