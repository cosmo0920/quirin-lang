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

import QCore.StdIo

-- Card `STD-MODULES`: the `std.fmt`, `std.io` and `std.logger` bundled modules.

-- The registry, generalised from two modules to five. The existing names are
-- retained; the three new capabilities are gated by the same argument.
#print axioms QCore.ModuleImports.empty_imports_grant_no_concurrency
#print axioms QCore.ModuleImports.empty_imports_grant_no_extern_calls
#print axioms QCore.ModuleImports.empty_imports_grant_no_output
#print axioms QCore.ModuleImports.empty_imports_grant_no_logger
#print axioms QCore.ModuleImports.empty_imports_grant_no_rendering
#print axioms QCore.ModuleImports.unknown_module_rejects
#print axioms QCore.ModuleImports.admitted_capability_requires_its_module
#print axioms QCore.ModuleImports.admitted_concurrency_requires_import
#print axioms QCore.ModuleImports.admitted_extern_calls_require_the_extern_import
#print axioms QCore.ModuleImports.admitted_output_requires_the_io_import
#print axioms QCore.ModuleImports.admitted_logging_requires_the_logger_import
#print axioms QCore.ModuleImports.admitted_rendering_requires_the_fmt_import

-- No module substitutes for another.
#print axioms QCore.ModuleImports.the_concurrency_import_alone_grants_no_extern_call
#print axioms QCore.ModuleImports.the_io_import_alone_grants_no_logging
#print axioms QCore.ModuleImports.the_logger_import_alone_grants_no_output
#print axioms QCore.ModuleImports.the_fmt_import_alone_grants_no_stream
#print axioms QCore.ModuleImports.neither_writer_grants_the_rendering
#print axioms QCore.ModuleImports.concurrency_grants_no_output_capability

-- Unused capabilities need no import; duplicates are idempotent; the modules
-- are independent.
#print axioms QCore.ModuleImports.sequential_code_needs_no_extern_import
#print axioms QCore.ModuleImports.code_that_prints_nothing_needs_no_io_import
#print axioms QCore.ModuleImports.code_that_logs_nothing_needs_no_logger_import
#print axioms QCore.ModuleImports.code_that_renders_nothing_needs_no_fmt_import
#print axioms QCore.ModuleImports.duplicate_import_is_idempotent
#print axioms QCore.ModuleImports.duplicate_io_import_is_idempotent
#print axioms QCore.ModuleImports.both_imports_are_independent
#print axioms QCore.ModuleImports.the_three_std_modules_are_independent
#print axioms QCore.ModuleImports.missing_required_capability_rejects
#print axioms QCore.ModuleImports.sequential_code_needs_no_concurrency

-- The v1 effect discipline: a rendering cannot touch custody, proved
-- structurally from an operand type with no resource constructor.
#print axioms QCore.StdIo.no_program_allocates
#print axioms QCore.StdIo.no_program_mutates
#print axioms QCore.StdIo.derived_row_is_io_or_empty
#print axioms QCore.StdIo.writing_derives_io
#print axioms QCore.StdIo.silent_program_derives_nothing
#print axioms QCore.StdIo.a_bare_return_is_silent
#print axioms QCore.StdIo.log_level_carries_io_without_a_source_line

-- Stream separation, and the barrier that moves no observation.
#print axioms QCore.StdIo.print_never_writes_stderr
#print axioms QCore.StdIo.log_never_writes_stdout
#print axioms QCore.StdIo.every_level_uses_one_stream
#print axioms QCore.StdIo.streams_left_identity
#print axioms QCore.StdIo.streams_right_identity
#print axioms QCore.StdIo.streams_compose_associative
#print axioms QCore.StdIo.flush_changes_no_line_count
#print axioms QCore.StdIo.repeated_flush_changes_no_line_count
#print axioms QCore.StdIo.a_bare_barrier_writes_nothing
#print axioms QCore.StdIo.the_barrier_still_carries_io

-- Capability gating composed with the resolver.
#print axioms QCore.StdIo.printing_requires_the_io_module
#print axioms QCore.StdIo.logging_requires_the_logger_module
#print axioms QCore.StdIo.rendering_requires_the_fmt_module
#print axioms QCore.StdIo.no_module_substitutes_for_another

-- Card `STD-FMT-BORROW`: the borrowed render, and why this route has none.
-- The refusal is the absence of a custody model, and the two namespaces are two
-- lookups in two environments that cannot reach each other in either direction.
#print axioms QCore.StdIo.a_route_without_a_custody_model_renders_no_borrow
#print axioms QCore.StdIo.a_borrowed_render_reads_no_scalar
#print axioms QCore.StdIo.a_scalar_render_reads_no_identity
#print axioms QCore.StdIo.a_literal_render_reads_neither_environment
#print axioms QCore.StdIo.the_operand_forms_are_exactly_these
#print axioms QCore.StdIo.a_borrowed_render_still_carries_io
#print axioms QCore.StdIo.a_borrowed_render_allocates_and_mutates_nothing
#print axioms QCore.StdIo.a_borrowed_render_writes_one_line_on_its_own_stream
