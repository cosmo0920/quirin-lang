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
the Rust lexer or spans (trust boundary 28). -/

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
