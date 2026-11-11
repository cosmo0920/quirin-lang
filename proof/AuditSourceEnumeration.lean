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

import QCore.SourceEnumeration

-- Unit `E01-F1`: the main-source function denominator. Model of
-- `crates/qcore-c-gateway/src/legacy_enumeration.rs`. Clang's line-marker
-- attribution, the gateway tokenizer's output and the analyzer's own body
-- finder are Rust/tool boundaries, recorded in
-- `docs/reports/remaining-e01-f1/README.md`.

-- Monotonicity: the enumeration is a superset of the historical raw scan, and
-- every input class it reads lands in the denominator.
#print axioms QCore.SourceEnumeration.mem_dedup
#print axioms QCore.SourceEnumeration.raw_is_kept
#print axioms QCore.SourceEnumeration.prototype_is_kept
#print axioms QCore.SourceEnumeration.identifier_list_is_kept
#print axioms QCore.SourceEnumeration.unanalyzable_is_counted

-- Honest accounting: the denominator is exactly its two classes, they are
-- disjoint, and the class the raw scan missed is real growth read by the
-- unmodified analyzer.
#print axioms QCore.SourceEnumeration.the_denominator_is_exactly_its_two_classes
#print axioms QCore.SourceEnumeration.unanalyzable_is_not_analysed
#print axioms QCore.SourceEnumeration.absent_from_raw_scan_is_new
#print axioms QCore.SourceEnumeration.absent_from_raw_scan_is_analysed

-- Fail-closed unit `Safe`, in both directions.
#print axioms QCore.SourceEnumeration.a_refused_inventory_cannot_support_safe
#print axioms QCore.SourceEnumeration.an_unread_definition_cannot_support_safe
#print axioms QCore.SourceEnumeration.safe_is_admitted_exactly_when_established_and_read

-- A completeness refusal is a scoped `Unknown`, and §L21.1 rule 1 holds over
-- this unit's ground vocabulary.
#print axioms QCore.SourceEnumeration.a_refusal_never_yields_unsafe
#print axioms QCore.SourceEnumeration.a_refusal_never_yields_safe
#print axioms QCore.SourceEnumeration.an_environment_ground_says_nothing_about_the_source
#print axioms QCore.SourceEnumeration.a_token_stream_ground_names_the_analysed_source
#print axioms QCore.SourceEnumeration.no_ground_is_both_environment_and_source
#print axioms QCore.SourceEnumeration.the_refusal_vocabulary_is_complete
#print axioms QCore.SourceEnumeration.both_subjects_are_reached

-- The file-scope grammar over the adversarial corpus: what is counted, what is
-- not, and what refuses.
#print axioms QCore.SourceEnumeration.a_prototype_definition_is_counted
#print axioms QCore.SourceEnumeration.a_prototype_declaration_is_not_counted
#print axioms QCore.SourceEnumeration.an_old_style_declaration_is_not_counted
#print axioms QCore.SourceEnumeration.a_knr_definition_is_counted_as_unread
#print axioms QCore.SourceEnumeration.a_pointer_declarator_list_is_read
#print axioms QCore.SourceEnumeration.a_typed_parameter_list_is_never_an_identifier_list
#print axioms QCore.SourceEnumeration.an_unreadable_declarator_list_refuses
#print axioms QCore.SourceEnumeration.an_unbalanced_construct_refuses
#print axioms QCore.SourceEnumeration.a_construct_that_does_not_open_a_parameter_list_refuses
#print axioms QCore.SourceEnumeration.an_expression_context_is_neither_counted_nor_ambiguous
#print axioms QCore.SourceEnumeration.a_comma_context_is_neither_counted_nor_ambiguous
#print axioms QCore.SourceEnumeration.the_guard_does_not_hide_a_real_unreadable_declarator
#print axioms QCore.SourceEnumeration.a_function_typedef_declarator_is_not_a_definition
#print axioms QCore.SourceEnumeration.a_specifier_in_the_name_position_is_not_a_definition
#print axioms QCore.SourceEnumeration.the_declaration_contexts_are_exactly_these

-- Non-vacuity: the repair adds rows, and no historical row disappears.
#print axioms QCore.SourceEnumeration.the_macro_case_grows_the_denominator
#print axioms QCore.SourceEnumeration.the_knr_case_grows_the_denominator_and_withholds_safe
#print axioms QCore.SourceEnumeration.a_compiled_out_definition_keeps_its_row
#print axioms QCore.SourceEnumeration.an_unavailable_expansion_keeps_the_historical_scan

-- Card CURRENT-RUN-DEFECTS: a unit whose every definition is compiled out
-- keeps its raw rows (never data-only), analyses none and is never Safe or
-- Unsafe; only its stated reason names the cause.
#print axioms QCore.SourceEnumeration.build_excluded_only_is_the_raw_scan
#print axioms QCore.SourceEnumeration.a_build_excluded_unit_is_never_data_only
#print axioms QCore.SourceEnumeration.a_build_excluded_unit_has_only_raw_rows
#print axioms QCore.SourceEnumeration.a_build_excluded_unit_is_never_safe
#print axioms QCore.SourceEnumeration.a_build_excluded_unit_is_never_unsafe
#print axioms QCore.SourceEnumeration.build_excluded_only_witnesses
