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

import QCore.SocketInetAddresses

-- Card `SOCKET-INET-ADDRESSES`: declared IPv4 and IPv6 addresses with ports for
-- `source.socket.v1` — inbound `bind`, outbound `connect` and `sendto`.

-- Address admission is total, and the two families are disjoint. Nothing here
-- relates `0.0.0.0` to `::`; that disjointness is the model half of the
-- `IPV6_V6ONLY` decision below.
#print axioms QCore.SocketInetAddresses.every_address_has_exactly_one_family
#print axioms QCore.SocketInetAddresses.the_two_families_are_disjoint
#print axioms QCore.SocketInetAddresses.an_implicit_bind_is_ipv4_loopback_on_a_kernel_chosen_port

-- Resolution refuses outward and never inward. These five are the
-- external-peer refusal, stated as theorems about the model rather than as
-- properties of a diagnostic: what resolution returns is always a bind this
-- program made, an address nothing bound reaches nothing, a kernel-chosen port
-- is not a name, and a program written before this card — which declares only
-- implicit locals — cannot be reached by an address at all.
#print axioms QCore.SocketInetAddresses.an_ephemeral_address_reaches_nothing
#print axioms QCore.SocketInetAddresses.no_bind_reaches_nothing
#print axioms QCore.SocketInetAddresses.reaching_names_a_bind_of_this_program
#print axioms QCore.SocketInetAddresses.a_declared_bind_is_reachable_by_its_address
#print axioms QCore.SocketInetAddresses.an_undeclared_address_reaches_nothing
#print axioms QCore.SocketInetAddresses.an_implicit_program_is_unreachable_by_address

-- The two families never stand in for each other, and agreement is exactly
-- equality: there is no third answer and no coercion between the spaces.
#print axioms QCore.SocketInetAddresses.a_family_agrees_with_itself
#print axioms QCore.SocketInetAddresses.the_families_never_stand_in_for_each_other
#print axioms QCore.SocketInetAddresses.agreement_is_equality

-- The `IPV6_V6ONLY` decision, stated as a premise per arm and proved constant.
-- Linux defaults the option off and Windows defaults it on, so a change to any
-- one arm must be a failed proof rather than a silent per-platform difference.
#print axioms QCore.SocketInetAddresses.the_v6only_decision_is_made_explicitly_on_every_arm
#print axioms QCore.SocketInetAddresses.the_v6only_decision_does_not_vary_by_arm

-- A missing address family is recorded, never replaced. The first of these is
-- the one the card needs: creation never yields a socket in a family other than
-- the one the program declared, so there is no fallback and an emitted program
-- never binds an address in a family its source did not write down.
#print axioms QCore.SocketInetAddresses.creation_never_changes_the_family
#print axioms QCore.SocketInetAddresses.a_missing_ipv6_is_recorded
#print axioms QCore.SocketInetAddresses.an_unsupported_family_yields_no_socket

-- A collision inside one program is decided at check time; two kernel-chosen
-- ports are two addresses and never collide.
#print axioms QCore.SocketInetAddresses.two_ephemeral_binds_never_collide
#print axioms QCore.SocketInetAddresses.a_repeated_declared_address_collides
