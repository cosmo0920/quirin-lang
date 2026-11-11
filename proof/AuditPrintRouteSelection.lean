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

import QCore.PrintRouteSelection

/-! Card **QR-PRINT-ROUTE-MATCHES-DECLARATION**
(`crates/qcore-frontend/src/source_scalar_calls.rs`, `print_call_selection`;
the guard at `compile_scalar_calls` in `crates/qcore-verification/src/driver.rs`):
`source.scalar-calls.v1` is selected only by a `print (` call at a statement
head inside a source function body; a declaration name never selects it, an
ambiguous occurrence refuses, and a contract module is never compiled there.
Not a proof about the Rust lexer or body spans (trust boundary 28). -/

#print axioms QCore.PrintRouteSelection.outside_body_ignored
#print axioms QCore.PrintRouteSelection.occurrences_outside_bodies
#print axioms QCore.PrintRouteSelection.declaration_name_never_selects
#print axioms QCore.PrintRouteSelection.selected_has_a_call
#print axioms QCore.PrintRouteSelection.declared_print_never_selected
#print axioms QCore.PrintRouteSelection.ambiguous_occurrence_refuses
#print axioms QCore.PrintRouteSelection.contract_module_never_compiled
#print axioms QCore.PrintRouteSelection.cjson_declaration_reaches_later_routes
#print axioms QCore.PrintRouteSelection.printing_example_selected
#print axioms QCore.PrintRouteSelection.print_as_value_refused
#print axioms QCore.PrintRouteSelection.printing_beside_a_declared_print_refused
#print axioms QCore.PrintRouteSelection.printing_in_a_contract_module_refused
