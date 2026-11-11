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

import QCore.SocketEventCompose

-- Card `SOCKET-EVENT-COMPOSE`: a checked socket registered with the
-- `source.event.v1` reactor. The composition of `std.socket` and `std.event`.

-- **The custody relation, decided: a registration is a loan.** None of the five
-- statements this card adds moves the descriptor ledger — not the registration,
-- not its retirement, not a dispatch turn, not either context statement — and
-- the socket a registration watches is still held by the name that created it.
-- That is what lets the socket route's custody half stay the *unedited* cycle-3
-- ownership checker's, with the loan expressed as that IR's own `BorrowShared`.
#print axioms QCore.SocketEventCompose.a_registration_acquires_no_descriptor
#print axioms QCore.SocketEventCompose.retiring_a_registration_releases_no_descriptor
#print axioms QCore.SocketEventCompose.the_context_statements_move_no_custody
#print axioms QCore.SocketEventCompose.a_watched_socket_is_still_held

-- **Dispatch neutrality on the socket's custody.** A turn changes neither the
-- ledger nor the loans.
#print axioms QCore.SocketEventCompose.a_dispatch_turn_changes_no_custody
#print axioms QCore.SocketEventCompose.a_dispatch_turn_changes_no_loan

-- **The three loan rules, which are the borrow rules.** A close and a
-- state-changing transport call are refused while the socket is borrowed — the
-- ownership IR's drop and write — and a transfer is admitted, because it is a
-- read and a shared loan admits reads. None of the three is this card's rule.
#print axioms QCore.SocketEventCompose.watching_borrows_the_socket
#print axioms QCore.SocketEventCompose.a_close_while_registered_is_refused
#print axioms QCore.SocketEventCompose.reconfiguring_a_registered_socket_is_refused
#print axioms QCore.SocketEventCompose.a_transfer_on_a_registered_socket_is_admitted

-- **Deregistration ordering.** Retiring removes exactly its own loan and keeps
-- every other, and once the one loan on a socket is retired its close is
-- admitted — which is "deregister before close" as a consequence of the loan
-- rather than as a rule.
#print axioms QCore.SocketEventCompose.retiring_removes_exactly_its_own
#print axioms QCore.SocketEventCompose.retiring_keeps_every_other_loan
#print axioms QCore.SocketEventCompose.retiring_the_only_loan_admits_the_close

-- The spans are lexical, because the ownership IR's borrow nodes are: a crossed
-- pair is refused by name rather than approximated, and a registration never
-- retired is refused too.
#print axioms QCore.SocketEventCompose.properly_nested_spans_are_admitted
#print axioms QCore.SocketEventCompose.crossed_spans_are_refused
#print axioms QCore.SocketEventCompose.a_registration_never_retired_is_refused

-- **Erasure.** Dropping the five new statements leaves the bare
-- `source.socket.v1` program and the ledger's answer is unchanged, so every
-- custody theorem about a socket program is a theorem about the same program
-- with registrations in it — the `erase` shape `QCore.ComposeEventLines` used
-- for a source line.
#print axioms QCore.SocketEventCompose.erasing_the_reactor_statements_leaves_the_ledger
#print axioms QCore.SocketEventCompose.erasing_a_composed_program_leaves_its_transport

-- **The determinism boundary**, which is `Q085`'s existing condition reached by
-- a new construct: a turn is admitted only where the declared readiness is the
-- whole of what this program's own history induced, a two-member ready set
-- admits no turn whatever it declares, and a write registration is never ready.
#print axioms QCore.SocketEventCompose.an_idle_turn_needs_an_empty_ready_set
#print axioms QCore.SocketEventCompose.a_declared_turn_names_the_one_ready_registration
#print axioms QCore.SocketEventCompose.two_ready_registrations_admit_no_turn
#print axioms QCore.SocketEventCompose.a_write_registration_is_never_ready
#print axioms QCore.SocketEventCompose.readiness_comes_only_from_the_programs_own_history
#print axioms QCore.SocketEventCompose.a_turn_drains_nothing

-- The effect discipline. The four bookkeeping statements derive nothing because
-- the emitted program performs no call for any of them; a turn performs the one
-- wait the composition adds; and `Mut` stays unreachable over the whole grammar.
#print axioms QCore.SocketEventCompose.a_dispatch_turn_derives_io
#print axioms QCore.SocketEventCompose.the_registration_statements_derive_nothing
#print axioms QCore.SocketEventCompose.no_statement_derives_mut

-- The module gate is a conjunction, and the transport half is the registry's own
-- gate restated rather than re-proved.
#print axioms QCore.SocketEventCompose.a_registration_needs_both_modules
#print axioms QCore.SocketEventCompose.the_socket_module_alone_grants_no_registration
#print axioms QCore.SocketEventCompose.the_event_module_alone_grants_no_registration
#print axioms QCore.SocketEventCompose.code_that_registers_nothing_needs_neither
#print axioms QCore.SocketEventCompose.the_transport_half_still_needs_its_module
