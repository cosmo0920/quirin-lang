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

import QCore.WriteLendTasks

-- Card `WRITE-LEND-TASKS`: write lends crossing the spawn boundary — the
-- residual `SLICE-PERMISSIONS-WRITE-LEND` named ("both parties are one
-- sequential entry's bindings — the F1 task-crossing remainder"). The model
-- is the scoped-task ledger's range write leases: two workers each holding
-- write authority over its own disjoint byte range of one payload for
-- spawn..join, with the wave-29 overlap decision imported rather than
-- restated.

-- The vocabulary the card adds, printed so that a later change to any of it
-- is visible in this surface rather than only inside a proof.
#print axioms QCore.WriteLendTasks.admitsLease
#print axioms QCore.WriteLendTasks.openLease
#print axioms QCore.WriteLendTasks.closeLease
#print axioms QCore.WriteLendTasks.leasesHolding
#print axioms QCore.WriteLendTasks.ownerMayWrite
#print axioms QCore.WriteLendTasks.writeAuthorities
#print axioms QCore.WriteLendTasks.addedSurface

-- **The admission is the disjointness question** — the wave-29 `overlaps`,
-- imported, deciding which second lease is admitted; no byte is leased
-- twice; an empty range is not a lease (a deliberate divergence from the
-- sequential lend's empty-lend admission, in the refusing direction); and a
-- third simultaneous holder is a represented refusal by count.
#print axioms QCore.WriteLendTasks.a_second_lease_is_admitted_iff_disjoint
#print axioms QCore.WriteLendTasks.two_admitted_leases_share_no_byte
#print axioms QCore.WriteLendTasks.an_empty_range_is_not_a_lease
#print axioms QCore.WriteLendTasks.a_third_simultaneous_lease_is_refused

-- **The invariant survives every interleaving**: every ledger state
-- reachable by admitted opens and closes has pairwise-disjoint leases, and
-- with pairwise-disjoint leases at most one lease holds any given byte.
#print axioms QCore.WriteLendTasks.reachable_states_are_pairwise_disjoint
#print axioms QCore.WriteLendTasks.at_most_one_lease_holds_a_byte

-- **The exclusivity theorem** — the statement the card exists for: at any
-- point of any spawn/join interleaving, no byte has two write authorities.
-- The owner counts as an authority exactly when no lease is outstanding
-- (the whole-payload rule, `use_resource`'s `LeaseOutstanding` refusal), so
-- the count covers the owner-versus-holder direction as well as the
-- holder-versus-holder one.
#print axioms QCore.WriteLendTasks.no_byte_has_two_write_authorities
#print axioms QCore.WriteLendTasks.the_owner_is_suspended_exactly_while_a_lease_is_out

-- **The authority returns at the join, exactly**: closing the last lease
-- reconstructs the owner's whole-payload authority, a join never disturbs a
-- sibling's lease, and a closed holder retains nothing.
#print axioms QCore.WriteLendTasks.closing_the_last_lease_restores_the_owner
#print axioms QCore.WriteLendTasks.a_join_never_disturbs_a_siblings_lease
#print axioms QCore.WriteLendTasks.a_closed_holder_retains_nothing

-- What was added, as one list, so a widening changes a theorem rather than
-- slipping past.
#print axioms QCore.WriteLendTasks.the_added_capture_is_exactly_one
