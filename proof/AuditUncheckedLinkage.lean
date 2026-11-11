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

import QCore.UncheckedLinkage
import QCore.ModuleImports

-- Card `UNSAFE-LINKAGE`: linking an external library that has no contract at
-- all, under an explicit recorded waiver. Model of the parts this card added to
-- `crates/qcore-core/src/extern_call.rs`; the source parser, the driver's
-- identity gates, the C backend and the recorded native evidence are separate
-- Rust boundaries, recorded in
-- `docs/reports/wave18-unsafe-linkage/README.md`.

-- **Trust is carried, never laundered.** The sentence the card exists for:
-- there is no path from a program containing an uncontracted callee to an
-- assumption inventory that does not name it, and no uncontracted callee is
-- ever published as a contracted one.
#print axioms QCore.UncheckedLinkage.assumption_totality
#print axioms QCore.UncheckedLinkage.every_callee_contributes_exactly_one_premise
#print axioms QCore.UncheckedLinkage.an_uncontracted_callee_is_never_published_as_contracted
#print axioms QCore.UncheckedLinkage.a_contracted_callee_contributes_no_waiver
#print axioms QCore.UncheckedLinkage.a_waiver_premise_names_an_uncontracted_callee

-- **The default does not move.** A callee list with no uncontracted member
-- derives exactly the inventory it derived before, and carries no waiver.
#print axioms QCore.UncheckedLinkage.the_contracted_inventory_is_unchanged
#print axioms QCore.UncheckedLinkage.no_waiver_without_an_uncontracted_callee
#print axioms QCore.UncheckedLinkage.a_contracted_row_is_the_existing_one

-- **Custody cannot cross an unchecked boundary.** Every pointer position and
-- every `unit` parameter refuses by its own named constructor; `ptr<const, u8>`
-- has its own theorem because its refusal is deliberate rather than incidental.
#print axioms QCore.UncheckedLinkage.a_pointer_parameter_refuses
#print axioms QCore.UncheckedLinkage.a_flat_mutable_pointer_parameter_refuses
#print axioms QCore.UncheckedLinkage.a_read_only_pointer_parameter_refuses
#print axioms QCore.UncheckedLinkage.a_pointer_to_pointer_parameter_refuses
#print axioms QCore.UncheckedLinkage.a_unit_parameter_refuses
#print axioms QCore.UncheckedLinkage.a_pointer_result_refuses
#print axioms QCore.UncheckedLinkage.a_read_only_pointer_result_refuses

-- The refusals above are not vacuous: the scalar table admits, and what it
-- admits carries no custody, acquires nothing and is marked as uncontracted —
-- so `assumption_totality` applies to everything the table admits.
#print axioms QCore.UncheckedLinkage.the_unchecked_table_is_inhabited
#print axioms QCore.UncheckedLinkage.a_unit_result_is_admitted
#print axioms QCore.UncheckedLinkage.an_admitted_unchecked_callee_carries_no_custody
#print axioms QCore.UncheckedLinkage.an_admitted_unchecked_callee_acquires_nothing
#print axioms QCore.UncheckedLinkage.an_admitted_unchecked_callee_is_marked

-- **The derived row is maximal**, so the waiver is visible in the entry's own
-- signature and not only at the declaration. Every proper sub-row refuses.
#print axioms QCore.UncheckedLinkage.an_unchecked_row_is_maximal
#print axioms QCore.UncheckedLinkage.a_narrower_declared_row_refuses_an_unchecked_call
#print axioms QCore.UncheckedLinkage.an_empty_declared_row_admits_no_unchecked_call
#print axioms QCore.UncheckedLinkage.the_whole_declared_row_admits_an_unchecked_call

-- Admission and the premise it produces are functions of the declaration, so a
-- rederivation reproduces the premise that was published.
#print axioms QCore.UncheckedLinkage.the_premise_is_a_function_of_the_callee

-- The file-local module gate is a second, independent opt-in: the waiver is
-- granted by its own module and by no other, and grants no other capability
-- back.
#print axioms QCore.ModuleImports.admitted_unchecked_linkage_requires_its_module
#print axioms QCore.ModuleImports.empty_imports_grant_no_unchecked_linkage
#print axioms QCore.ModuleImports.the_extern_import_alone_grants_no_waiver
#print axioms QCore.ModuleImports.the_waiver_alone_grants_no_extern_call
#print axioms QCore.ModuleImports.the_two_extern_modules_are_independent
#print axioms QCore.ModuleImports.code_that_waives_nothing_needs_no_waiver_import
#print axioms QCore.ModuleImports.a_near_miss_waiver_spelling_rejects
