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

import QCore.NativeLibrary

-- Card `UNSAFE-LINKAGE-ARTIFACT`: the `source.native-library.v1` manifest slice
-- that lets an unchecked-linkage waiver say **which build** it was waived from.
-- Model of the parts this card added to `crates/qcore-core/src/native_library.rs`
-- and `extern_call.rs`; the manifest's concrete syntax, its total-or-reject
-- loader, the driver's diagnostics and the CLI's premise printing are separate
-- Rust boundaries, recorded in
-- `docs/reports/wave19-unsafe-linkage-artifact/README.md`.

-- **Totality survives the manifest.** The sentence card `UNSAFE-LINKAGE` exists
-- for, restated with the manifest in the model: there is still no path from a
-- program containing an uncontracted callee to an inventory that does not
-- mention it, and no residence assignment whatever can drop one.
#print axioms QCore.NativeLibrary.manifest_assumption_totality
#print axioms QCore.NativeLibrary.there_is_no_assumption_free_path
#print axioms QCore.NativeLibrary.every_resident_contributes_exactly_one_premise
#print axioms QCore.NativeLibrary.the_manifest_never_removes_a_premise
#print axioms QCore.NativeLibrary.a_premise_names_a_resident_of_the_list

-- **The manifest sharpens, and only sharpens.** An unmanifested waiver keeps the
-- wave-18 record byte for byte; a manifested one names its artifact; the two are
-- never confusable; and a contracted callee names no artifact and waives
-- nothing.
#print axioms QCore.NativeLibrary.an_unmanifested_waiver_keeps_the_bare_premise
#print axioms QCore.NativeLibrary.a_manifested_waiver_names_its_artifact
#print axioms QCore.NativeLibrary.a_bare_premise_is_never_a_sharpened_one
#print axioms QCore.NativeLibrary.a_contracted_callee_names_no_artifact
#print axioms QCore.NativeLibrary.a_contracted_callee_contributes_no_waiver

-- **The admitted surface does not move.** The closed table decides first and
-- unchanged, so a manifest cannot buy a pointer — and could not have declared
-- one either, since a contract-less entry carrying a pointer is not an
-- admissible entry. The entry gate is not vacuous.
#print axioms QCore.NativeLibrary.a_refused_surface_stays_refused_under_every_manifest
#print axioms QCore.NativeLibrary.a_manifest_cannot_buy_a_pointer_parameter
#print axioms QCore.NativeLibrary.a_manifest_cannot_buy_a_pointer_result
#print axioms QCore.NativeLibrary.a_pointer_entry_is_not_an_admissible_contract_less_entry
#print axioms QCore.NativeLibrary.a_pointer_result_entry_is_not_admissible
#print axioms QCore.NativeLibrary.a_scalar_entry_is_admissible
#print axioms QCore.NativeLibrary.an_admitted_resident_was_admitted_by_the_closed_table

-- **Disagreement refuses, and never withdraws.** A manifest that contradicts a
-- declaration refuses it, and a symbol its manifest names is never admitted
-- under the weaker bare premise — a withdrawal would reward a contradiction
-- with a premise that says less about the artifact.
#print axioms QCore.NativeLibrary.a_disagreeing_manifest_refuses
#print axioms QCore.NativeLibrary.a_named_symbol_is_never_admitted_bare
#print axioms QCore.NativeLibrary.agreement_is_declaration_equals_header

-- Two admitted results for the same model inputs publish the same premise.
-- This does not prove that a Rust consumer faithfully rederives those inputs.
#print axioms QCore.NativeLibrary.the_published_premise_is_a_function_of_the_admission

-- The refusals above refuse something reachable: the manifest route admits, with
-- the sharpened premise, and an unnamed symbol is admitted bare beside it.
#print axioms QCore.NativeLibrary.the_manifest_route_admits_and_names_the_artifact
#print axioms QCore.NativeLibrary.an_unnamed_symbol_is_admitted_bare
