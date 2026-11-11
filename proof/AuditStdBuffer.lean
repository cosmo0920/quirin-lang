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

import QCore.StdBuffer

-- Card `STD-BUFFER`: `source.buffer.v1`, the `std.buffer` bundled module's
-- owned, extent-carrying byte buffer.

-- The extent is transported, not remembered: what `extent(b)` reads back is the
-- number `bytes(N)` declared, out of the ownership ledger's own footprint, and
-- no non-custody event can move it.
#print axioms QCore.StdBuffer.the_extent_read_back_is_the_extent_declared
#print axioms QCore.StdBuffer.a_store_moves_no_extent
#print axioms QCore.StdBuffer.a_load_moves_no_extent
#print axioms QCore.StdBuffer.an_extent_read_moves_no_extent
#print axioms QCore.StdBuffer.a_line_moves_no_extent
#print axioms QCore.StdBuffer.non_custody_events_leave_the_ledger_unchanged

-- The bounds discipline, in both directions and including the two ways an
-- access can fail to have one at all.
#print axioms QCore.StdBuffer.an_admitted_index_is_inside_the_extent
#print axioms QCore.StdBuffer.an_index_into_nothing_is_refused
#print axioms QCore.StdBuffer.an_index_with_no_value_is_refused
#print axioms QCore.StdBuffer.the_last_index_is_admitted_and_the_extent_itself_is_not
#print axioms QCore.StdBuffer.a_negative_index_is_refused
#print axioms QCore.StdBuffer.the_byte_range_is_exactly_a_byte
#print axioms QCore.StdBuffer.the_extent_range_is_bounded_at_both_ends

-- The custody discipline. The rules themselves are the unedited ownership
-- checker's; these state what its discharge means for the byte total.
#print axioms QCore.StdBuffer.a_release_removes_exactly_its_own
#print axioms QCore.StdBuffer.a_release_keeps_every_other_entry
#print axioms QCore.StdBuffer.an_empty_ledger_holds_no_bytes
#print axioms QCore.StdBuffer.a_held_buffer_is_live_bytes
#print axioms QCore.StdBuffer.an_acquisition_adds_exactly_its_extent
#print axioms QCore.StdBuffer.two_live_buffers_hold_the_sum

-- The effect discipline. `source.extern-call.v1` derives `alloc` and `mut` too,
-- from a declared contract's projections; here they follow from an acquisition
-- and a store the route checks itself.
#print axioms QCore.StdBuffer.an_acquisition_derives_alloc
#print axioms QCore.StdBuffer.a_store_derives_mut
#print axioms QCore.StdBuffer.a_read_derives_nothing
#print axioms QCore.StdBuffer.an_extent_read_derives_nothing
#print axioms QCore.StdBuffer.a_silent_buffer_program_derives_no_io

-- The operand wall: two forms, neither a buffer, stated as one list so a
-- widening changes a theorem rather than slipping past it.
#print axioms QCore.StdBuffer.the_operand_forms_are_exactly_two
#print axioms QCore.StdBuffer.an_operand_is_never_a_custody_act

-- The module gate, proved against the same registry and by the same generic
-- argument every other bundled module is.
#print axioms QCore.StdBuffer.the_buffer_capability_needs_its_module
#print axioms QCore.StdBuffer.the_buffer_module_grants_no_stream

-- The registry's own new rows (card `STD-BUFFER` grows it from six to seven).
#print axioms QCore.ModuleImports.empty_imports_grant_no_buffer
#print axioms QCore.ModuleImports.admitted_buffer_requires_its_module
#print axioms QCore.ModuleImports.the_io_import_alone_grants_no_buffer
#print axioms QCore.ModuleImports.the_buffer_import_alone_grants_no_stream
#print axioms QCore.ModuleImports.concurrency_grants_no_buffer_capability
#print axioms QCore.ModuleImports.neither_extern_module_grants_the_buffer
#print axioms QCore.ModuleImports.code_that_buffers_nothing_needs_no_buffer_import
#print axioms QCore.ModuleImports.duplicate_buffer_import_is_idempotent
#print axioms QCore.ModuleImports.the_buffer_module_is_independent_of_the_output_modules
#print axioms QCore.ModuleImports.a_near_miss_buffer_spelling_rejects
