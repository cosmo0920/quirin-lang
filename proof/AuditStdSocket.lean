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

import QCore.StdSocket

-- Card `QUIRIN-SOCKET-V1`: `source.socket.v1`, the `std.socket` bundled
-- module's checked TCP and UDP surface over loopback.

-- The custody discipline. The rules themselves are the unedited ownership
-- checker's; these state what this route's operation list gives it, and that a
-- transport statement gives it nothing at all — which is why the two halves can
-- be checked independently.
#print axioms QCore.StdSocket.an_empty_ledger_holds_no_descriptors
#print axioms QCore.StdSocket.an_acquisition_adds_exactly_one
#print axioms QCore.StdSocket.a_release_removes_exactly_its_own
#print axioms QCore.StdSocket.a_release_keeps_every_other_entry
#print axioms QCore.StdSocket.a_closed_socket_is_not_held
#print axioms QCore.StdSocket.transport_statements_leave_the_ledger_unchanged
#print axioms QCore.StdSocket.an_accept_acquires_beside_its_listener

-- The determinism discipline, stream half. A receive is admitted only where the
-- program's own committed history already satisfies it, so an admitted receive
-- has its bytes before it runs: non-blocking as a theorem, not as a timing
-- assumption.
#print axioms QCore.StdSocket.a_receive_on_an_empty_stream_is_refused
#print axioms QCore.StdSocket.an_admitted_receive_takes_its_full_count
#print axioms QCore.StdSocket.a_receive_takes_a_prefix
#print axioms QCore.StdSocket.a_commit_never_shortens
#print axioms QCore.StdSocket.a_receive_leaves_exactly_the_rest

-- The soundness of the published receipts, which is what makes the emitted
-- program's byte comparison a check of a false premise rather than a ritual.
#print axioms QCore.StdSocket.draining_returns_the_committed_bytes
#print axioms QCore.StdSocket.two_commits_are_returned_in_order

-- The datagram half, stated separately because it is different: taken whole,
-- from a named sender, or refused.
#print axioms QCore.StdSocket.a_datagram_receive_on_an_empty_queue_is_refused
#print axioms QCore.StdSocket.a_partial_datagram_is_refused
#print axioms QCore.StdSocket.an_admitted_datagram_receive_takes_the_head
#print axioms QCore.StdSocket.a_datagram_arrives_whole

-- The effect discipline. `Alloc` and `Io` and never `Mut`, the last stated over
-- the whole grammar so a widening changes a theorem.
#print axioms QCore.StdSocket.a_creation_derives_alloc
#print axioms QCore.StdSocket.a_close_derives_alloc
#print axioms QCore.StdSocket.a_transfer_derives_io
#print axioms QCore.StdSocket.a_binding_derives_nothing
#print axioms QCore.StdSocket.no_statement_derives_mut

-- The operand wall: two forms, neither a socket, stated as one list so a
-- widening changes a theorem rather than slipping past it.
#print axioms QCore.StdSocket.the_operand_forms_are_exactly_two
#print axioms QCore.StdSocket.an_operand_is_never_a_custody_act

-- The module gate, proved against the same registry and by the same generic
-- argument every other bundled module is.
#print axioms QCore.StdSocket.the_socket_capability_needs_its_module
#print axioms QCore.StdSocket.the_socket_module_grants_no_stream

-- The registry's own new rows (this card grows it from eight modules to nine).
#print axioms QCore.ModuleImports.empty_imports_grant_no_socket
#print axioms QCore.ModuleImports.admitted_socket_requires_its_module
#print axioms QCore.ModuleImports.the_socket_import_alone_grants_no_stream
#print axioms QCore.ModuleImports.the_output_modules_grant_no_socket
#print axioms QCore.ModuleImports.the_socket_module_is_independent_of_the_value_modules
#print axioms QCore.ModuleImports.concurrency_grants_no_socket_capability
#print axioms QCore.ModuleImports.neither_extern_module_grants_the_socket
#print axioms QCore.ModuleImports.code_that_sockets_nothing_needs_no_socket_import
#print axioms QCore.ModuleImports.duplicate_socket_import_is_idempotent
#print axioms QCore.ModuleImports.the_socket_module_is_independent_of_its_neighbours
#print axioms QCore.ModuleImports.a_near_miss_socket_spelling_rejects
