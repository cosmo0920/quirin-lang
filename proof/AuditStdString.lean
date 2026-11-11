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

import QCore.StdString

-- Card `STRING-SLICE`: `source.string.v1`, the `std.string` bundled module's
-- immutable UTF-8 `{ptr, len}` string view.

-- The length is exact and is never a scan. The embedded-NUL statement is the
-- load-bearing one: a scan-to-terminator and this model disagree on a value the
-- route admits, which is why §3.7 forbids `strlen` and the emitted C reads a
-- field.
#print axioms QCore.StdString.a_literal_view_has_the_payloads_length
#print axioms QCore.StdString.an_embedded_nul_is_inside_the_length
#print axioms QCore.StdString.a_derived_length_is_the_difference
#print axioms QCore.StdString.the_empty_slice_is_a_value

-- Slicing is `{p + low, high - low}` into the same storage, and it composes. The
-- storage equality is the zero-copy claim and the lifetime claim at once: a
-- derived view addresses the storage it came from and nothing can move it.
#print axioms QCore.StdString.a_derived_view_keeps_its_storage
#print axioms QCore.StdString.a_derived_offset_is_the_source_plus_low
#print axioms QCore.StdString.slicing_a_slice_composes_the_offsets
#print axioms QCore.StdString.slicing_a_slice_keeps_the_original_storage
#print axioms QCore.StdString.the_whole_slice_is_the_view_itself

-- The admission condition, in every direction and including the two ways a
-- slice can fail to have a decided bound at all.
#print axioms QCore.StdString.an_admitted_slice_is_in_range_and_on_boundaries
#print axioms QCore.StdString.a_bound_with_no_value_is_refused
#print axioms QCore.StdString.the_upper_bound_is_closed_at_len
#print axioms QCore.StdString.a_negative_bound_is_refused
#print axioms QCore.StdString.an_inverted_pair_is_refused

-- The UTF-8 boundary rule: refused rather than rounded, and preserved by every
-- admitted slice.
#print axioms QCore.StdString.a_bound_inside_a_code_point_is_refused
#print axioms QCore.StdString.the_boundaries_of_a_two_byte_code_point_are_its_ends
#print axioms QCore.StdString.a_boundary_slice_preserves_well_formedness
#print axioms QCore.StdString.an_empty_view_addresses_no_bytes

-- A view is not a resource: the effect row of a string-only program is empty,
-- which is this model's form of "no allocation, no copy, no custody".
#print axioms QCore.StdString.a_string_only_program_derives_nothing
#print axioms QCore.StdString.a_slice_derives_no_alloc
#print axioms QCore.StdString.a_slice_derives_no_mut
#print axioms QCore.StdString.only_a_line_derives_io

-- The bound wall: two forms, neither a string, stated as one list so a widening
-- changes a theorem rather than slipping past it.
#print axioms QCore.StdString.the_bound_forms_are_exactly_two

-- The module gate, proved against the same registry and by the same generic
-- argument every other bundled module is.
#print axioms QCore.StdString.the_string_capability_needs_its_module
#print axioms QCore.StdString.the_string_module_grants_no_stream
#print axioms QCore.StdString.the_string_module_grants_no_buffer

-- The registry's own new rows (card `STRING-SLICE` grows it from seven to
-- eight). The two independence rows are amendment §5 and §7 as registry facts.
#print axioms QCore.ModuleImports.empty_imports_grant_no_string
#print axioms QCore.ModuleImports.admitted_string_requires_its_module
#print axioms QCore.ModuleImports.the_buffer_import_alone_grants_no_string
#print axioms QCore.ModuleImports.the_string_import_alone_grants_no_buffer
#print axioms QCore.ModuleImports.the_string_import_alone_grants_no_stream
#print axioms QCore.ModuleImports.the_output_modules_grant_no_string
#print axioms QCore.ModuleImports.concurrency_grants_no_string_capability
#print axioms QCore.ModuleImports.neither_extern_module_grants_the_string
#print axioms QCore.ModuleImports.code_that_strings_nothing_needs_no_string_import
#print axioms QCore.ModuleImports.duplicate_string_import_is_idempotent
#print axioms QCore.ModuleImports.the_string_module_is_independent_of_its_neighbours
#print axioms QCore.ModuleImports.a_near_miss_string_spelling_rejects
