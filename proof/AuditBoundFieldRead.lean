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

import QCore.AggregateStore
import QCore.BoundFieldRead

/-! Card O04-READ-BOUND: reading a member back through a base this body bound.

Both fail-closed directions are audited — every premise of the syntactic
whitelist refuses on its own, the flow-sensitive gate refuses independently of
the declarations, every custody-bearing argument position is refused by one
quantified theorem plus its per-mode corollary, and a read whose owner or whose
linked child this body already released can never denote. The borrowed effect is
audited to be the identity on every disposition, which is what makes the
admission incapable of manufacturing a discharge. Disjointness from card C02-P's
route and non-vacuity are audited in both directions. -/

-- Every premise of the syntactic whitelist refuses on its own.
#print axioms QCore.BoundFieldRead.unread_declarations_never_admit_a_read
#print axioms QCore.BoundFieldRead.a_formal_base_never_admits_a_read
#print axioms QCore.BoundFieldRead.a_non_record_pointer_base_never_admits_a_read
#print axioms QCore.BoundFieldRead.an_incomplete_record_never_admits_a_read
#print axioms QCore.BoundFieldRead.a_rebound_base_never_admits_a_read
#print axioms QCore.BoundFieldRead.an_unacquired_base_never_admits_a_read
#print axioms QCore.BoundFieldRead.an_escaping_base_pointer_never_admits_a_read
#print axioms QCore.BoundFieldRead.an_aliased_base_never_admits_a_read
#print axioms QCore.BoundFieldRead.a_partial_actual_never_admits_a_read
#print axioms QCore.BoundFieldRead.a_scalar_member_never_admits_a_read
#print axioms QCore.BoundFieldRead.a_refused_member_shape_never_admits_a_read
#print axioms QCore.BoundFieldRead.a_qualified_member_never_admits_a_read
#print axioms QCore.BoundFieldRead.an_uncontracted_callee_never_admits_a_read

-- O04-READ-BOUND-CUSTODY: only a borrowed position is ever admitted.
#print axioms QCore.BoundFieldRead.a_custody_bearing_position_never_admits_a_read
#print axioms QCore.BoundFieldRead.no_owned_consumed_released_stored_removed_or_out_position_admits_a_read
#print axioms QCore.BoundFieldRead.an_admitted_read_is_borrowed

-- The flow-sensitive read premise, including card C00's lesson in both places.
#print axioms QCore.BoundFieldRead.an_unresolved_owner_never_resolves_a_read
#print axioms QCore.BoundFieldRead.a_read_through_a_released_aggregate_never_resolves
#print axioms QCore.BoundFieldRead.a_read_through_a_caller_held_aggregate_never_resolves
#print axioms QCore.BoundFieldRead.an_unbound_member_never_resolves_a_read
#print axioms QCore.BoundFieldRead.an_unknown_member_never_resolves_a_read
#print axioms QCore.BoundFieldRead.an_unlinked_member_never_resolves_a_read
#print axioms QCore.BoundFieldRead.a_drifted_binding_never_resolves_a_read
#print axioms QCore.BoundFieldRead.a_released_linked_child_never_resolves_a_read
#print axioms QCore.BoundFieldRead.an_escaped_linked_child_never_resolves_a_read
#print axioms QCore.BoundFieldRead.an_unresolved_linked_child_never_resolves_a_read
#print axioms QCore.BoundFieldRead.a_resolved_read_has_a_live_child

-- Fail-closed transfer: both gates are required, and neither alone suffices.
#print axioms QCore.BoundFieldRead.a_refused_read_never_denotes
#print axioms QCore.BoundFieldRead.an_unresolved_read_never_denotes
#print axioms QCore.BoundFieldRead.only_a_doubly_admitted_read_denotes

-- What an admitted read does, and does not, do.
#print axioms QCore.BoundFieldRead.an_admitted_read_changes_no_disposition
#print axioms QCore.BoundFieldRead.an_admitted_read_never_discharges
#print axioms QCore.BoundFieldRead.an_admitted_read_never_releases

-- Disjointness from card C02-P's route.
#print axioms QCore.BoundFieldRead.the_two_read_routes_are_disjoint

-- Non-vacuity, in both directions.
#print axioms QCore.BoundFieldRead.the_read_whitelist_is_not_vacuous
#print axioms QCore.BoundFieldRead.the_admitted_read_denotes
#print axioms QCore.BoundFieldRead.one_scalar_member_flips_the_admitted_read
#print axioms QCore.BoundFieldRead.one_releasing_position_flips_the_admitted_read
#print axioms QCore.BoundFieldRead.one_unacquired_base_flips_the_admitted_read
#print axioms QCore.BoundFieldRead.one_aliased_base_flips_the_admitted_read
#print axioms QCore.BoundFieldRead.a_released_owner_flips_the_admitted_read
#print axioms QCore.BoundFieldRead.a_released_child_flips_the_admitted_read
#print axioms QCore.BoundFieldRead.an_unlinked_member_flips_the_admitted_read
#print axioms QCore.BoundFieldRead.the_borrowed_actual_route_is_not_vacuous

-- Card ALIAS-AGGREGATE: the store/read asymmetry of the opaque member. It is
-- admitted by neither read half, in any position, insulated or not.
#print axioms QCore.BoundFieldRead.an_opaque_member_never_admits_a_read
#print axioms QCore.BoundFieldRead.an_opaque_member_never_admits_a_custody_read
#print axioms QCore.BoundFieldRead.an_opaque_member_is_admitted_by_no_read_half
#print axioms QCore.BoundFieldRead.one_opaque_member_flips_the_admitted_read
