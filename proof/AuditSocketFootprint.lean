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

import QCore.SocketFootprint

#print axioms QCore.SocketFootprint.initial_is_bounded
#print axioms QCore.SocketFootprint.assigned_cell_is_bounded
#print axioms QCore.SocketFootprint.step_preserves_bounded
#print axioms QCore.SocketFootprint.run_preserves_bounded
#print axioms QCore.SocketFootprint.create_assigns_the_fresh_cell
#print axioms QCore.SocketFootprint.a_fresh_cell_is_no_existing_cell
#print axioms QCore.SocketFootprint.an_accepted_pair_shares_one_cell
#print axioms QCore.SocketFootprint.an_accept_keeps_every_other_assignment
#print axioms QCore.SocketFootprint.a_create_keeps_every_other_assignment
#print axioms QCore.SocketFootprint.an_unresolved_connect_is_refused
#print axioms QCore.SocketFootprint.an_unqueued_accept_is_refused
#print axioms QCore.SocketFootprint.an_orphaned_connector_is_refused
#print axioms QCore.SocketFootprint.an_admitted_accept_takes_an_existing_cell
#print axioms QCore.SocketFootprint.a_derived_pair_is_refused_as_a_shared_cell
#print axioms QCore.SocketFootprint.a_distinct_derived_cell_decides_disjoint
