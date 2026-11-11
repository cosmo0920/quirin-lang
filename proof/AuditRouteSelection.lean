-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.RouteSelection

/-! Card **QR-LEXICAL-SELECTORS-DECLARATION-NAMES**
(`crates/qcore-frontend/src/route_selection.rs`, `SelectorScope`, used by
`module_imports::concurrency_span`, `source_tasks::has_source_tasks`,
`source_epoch::has_source_epoch`, `source_mutex::has_source_mutex` and
`source_scalar_calls::has_scalar_calls`; the two refusals at
`compile_elaborated_source` in `crates/qcore-verification/src/driver.rs`):
every token-stream route selector and the `Q015` gate skip a declaration's own
name (a record's excepted, by constraint 14's `AGGREGATE-SELECTOR-WORDS`), a
parameter's name and a contract clause, so such a name never selects; a use that
names a declared word refuses; a contract module is never compiled on a
source-only route. Generalises `QCore.PrintRouteSelection`. Not a proof about
the Rust lexer or spans (trust boundary 28).

Card **ROUTE-SELECTION-ORDER-RESTATED**
(`crates/qcore-verification/src/driver/source_routes.rs`, `SOURCE_ROUTES`,
read once by `compile_elaborated_source`): first match over the one route
table is the if-chain in table order; the contract guard refuses exactly the
source-only entries of the table selection reads, so an added entry is
guarded; a contract module is lowered only on the sequential route and a
closure only on the two scoped-task routes. The Rust rows are pinned to
`driverTable` by `source_routes::tests` (trust boundary 28). -/

#print axioms QCore.RouteSelection.test_holds_only_at_a_head
#print axioms QCore.RouteSelection.mem_windows
#print axioms QCore.RouteSelection.fires_only_at_a_read_site
#print axioms QCore.RouteSelection.declaration_name_never_selects
#print axioms QCore.RouteSelection.Position.read_of_use
#print axioms QCore.RouteSelection.ambiguous_false_of_no_read_head
#print axioms QCore.RouteSelection.declaration_names_reach_sequential
#print axioms QCore.RouteSelection.ambiguous_use_refuses
#print axioms QCore.RouteSelection.contract_module_never_compiled
#print axioms QCore.RouteSelection.buffer_new_contract_sequential
#print axioms QCore.RouteSelection.epoch_error_sequential
#print axioms QCore.RouteSelection.spawn_function_sequential
#print axioms QCore.RouteSelection.spawn_record_requires_import
#print axioms QCore.RouteSelection.spawn_record_imported_sequential
#print axioms QCore.RouteSelection.epoch_record_epoch_route
#print axioms QCore.RouteSelection.effect_row_requires_import
#print axioms QCore.RouteSelection.body_buffer_new_tasks
#print axioms QCore.RouteSelection.body_buffer_new_declared_refused
#print axioms QCore.RouteSelection.body_buffer_new_contract_refused
#print axioms QCore.RouteSelection.throws_row_scalar
#print axioms QCore.RouteSelection.choose_eq_chain
#print axioms QCore.RouteSelection.choose_mem
#print axioms QCore.RouteSelection.guard_refuses_iff_source_only
#print axioms QCore.RouteSelection.added_entry_is_guarded
#print axioms QCore.RouteSelection.lowered_is_chosen
#print axioms QCore.RouteSelection.contract_never_lowered_on_source_only
#print axioms QCore.RouteSelection.closure_lowered_only_on_hosting
#print axioms QCore.RouteSelection.contract_lowered_only_on_last
#print axioms QCore.RouteSelection.choose_total_of_last
#print axioms QCore.RouteSelection.mem_entriesFrom
#print axioms QCore.RouteSelection.driver_table_length
#print axioms QCore.RouteSelection.driver_table_reads_contracts_only_on_sequential
#print axioms QCore.RouteSelection.driver_table_hosts_closures_only_on_task_routes
#print axioms QCore.RouteSelection.driver_contract_lowered_only_on_sequential
#print axioms QCore.RouteSelection.driver_closure_lowered_only_on_task_routes
#print axioms QCore.RouteSelection.dispatch_and_socket_contract_names_socket
#print axioms QCore.RouteSelection.restated_list_named_event
#print axioms QCore.RouteSelection.dispatch_and_socket_executable_lowered
