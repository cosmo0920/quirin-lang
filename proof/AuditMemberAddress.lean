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
import QCore.MemberAddress

/-! Card O04-B09B10: handing a callee the address of a member (O00 blockers
B09/B10), and the custody-taking half of the bound-read decision.

Both fail-closed directions are audited for the member-address rule — every
premise of the whitelist refuses on its own, the flow-sensitive owner gate
refuses independently of the declarations, every custody-bearing argument
position is refused by one quantified theorem plus its per-mode corollary, and
an address through an aggregate this body already released can never be handed
over. The borrowed effect is audited to be the identity on every disposition,
and an admitted address is audited to name no linkable member, which together
make the admission incapable of manufacturing a discharge or disturbing a field
link. Non-vacuity is audited on both base routes.

The custody-taking read direction extends `QCore.BoundFieldRead` additively:
`admitsRead` and every theorem about it are unchanged and still model the
borrowed half exactly, `admitsCustodyRead` models the new half, and the two are
audited to be disjoint on position, to refuse the four remaining
custody-bearing modes in both halves, and to collapse to the borrowed half alone
whenever premise **F** does not hold. -/

-- Every premise of the member-address whitelist refuses on its own.
#print axioms QCore.MemberAddress.unread_declarations_never_admit_an_address
#print axioms QCore.MemberAddress.a_partial_address_never_admits
#print axioms QCore.MemberAddress.a_non_record_pointer_base_never_admits_an_address
#print axioms QCore.MemberAddress.an_incomplete_record_never_admits_an_address
#print axioms QCore.MemberAddress.a_pointer_member_address_is_never_admitted
#print axioms QCore.MemberAddress.a_refused_member_shape_never_admits_an_address
#print axioms QCore.MemberAddress.an_uncontracted_callee_never_admits_an_address
#print axioms QCore.MemberAddress.a_refused_base_never_admits_an_address

-- Only a borrowed position, over the whole vocabulary.
#print axioms QCore.MemberAddress.a_custody_bearing_position_never_admits_an_address
#print axioms QCore.MemberAddress.an_admitted_address_is_borrowed
#print axioms QCore.MemberAddress.no_owned_consumed_released_stored_removed_or_out_position_admits_an_address
#print axioms QCore.MemberAddress.an_admitted_address_has_a_non_pointer_member

-- The flow-sensitive owner gate, including card C00's lesson in this direction.
#print axioms QCore.MemberAddress.an_address_through_a_released_aggregate_never_resolves
#print axioms QCore.MemberAddress.an_address_through_a_caller_held_aggregate_never_resolves
#print axioms QCore.MemberAddress.an_address_through_an_ambiguous_aggregate_never_resolves
#print axioms QCore.MemberAddress.an_address_through_a_nullable_aggregate_never_resolves
#print axioms QCore.MemberAddress.an_untracked_base_always_resolves

-- Fail-closed transfer: both gates are required, and neither alone suffices.
#print axioms QCore.MemberAddress.a_refused_address_is_never_handed
#print axioms QCore.MemberAddress.an_unresolved_address_is_never_handed
#print axioms QCore.MemberAddress.only_a_doubly_admitted_address_is_handed

-- What an admitted address does, and does not, do.
#print axioms QCore.MemberAddress.an_admitted_address_changes_no_disposition
#print axioms QCore.MemberAddress.an_admitted_address_never_discharges
#print axioms QCore.MemberAddress.an_admitted_address_names_no_linkable_member

-- Interaction with premise F of the bound-read rule.
#print axioms QCore.MemberAddress.an_admitted_address_preserves_insulation
#print axioms QCore.MemberAddress.a_pointer_member_address_breaks_insulation

-- Non-vacuity, on both base routes and in both directions.
#print axioms QCore.MemberAddress.the_address_whitelist_is_not_vacuous
#print axioms QCore.MemberAddress.the_untracked_route_is_not_vacuous
#print axioms QCore.MemberAddress.the_bound_address_is_handed
#print axioms QCore.MemberAddress.the_untracked_address_needs_no_live_owner
#print axioms QCore.MemberAddress.one_pointer_member_flips_the_admitted_address
#print axioms QCore.MemberAddress.one_releasing_position_flips_the_admitted_address
#print axioms QCore.MemberAddress.one_refused_base_flips_the_admitted_address
#print axioms QCore.MemberAddress.one_partial_argument_flips_the_admitted_address
#print axioms QCore.MemberAddress.a_released_owner_flips_the_bound_address
#print axioms QCore.MemberAddress.an_ambiguous_owner_flips_the_bound_address

-- The custody-taking read half, and its relation to the borrowed half.
#print axioms QCore.BoundFieldRead.premise_f_is_required
#print axioms QCore.BoundFieldRead.an_exposed_base_never_admits_a_custody_read
#print axioms QCore.BoundFieldRead.an_exposed_base_admits_only_the_borrowed_half
#print axioms QCore.BoundFieldRead.no_owned_stored_removed_or_out_position_takes_custody
#print axioms QCore.BoundFieldRead.no_refused_position_is_admitted_by_either_half
#print axioms QCore.BoundFieldRead.the_two_read_halves_are_disjoint
#print axioms QCore.BoundFieldRead.an_aliased_base_never_admits_a_custody_read
#print axioms QCore.BoundFieldRead.an_unacquired_base_never_admits_a_custody_read
#print axioms QCore.BoundFieldRead.a_scalar_member_never_admits_a_custody_read
#print axioms QCore.BoundFieldRead.a_custody_read_releases_at_a_released_position
#print axioms QCore.BoundFieldRead.a_custody_read_escapes_at_a_consumed_position
#print axioms QCore.BoundFieldRead.a_second_read_of_a_transferred_member_never_resolves
#print axioms QCore.BoundFieldRead.only_the_custody_effect_can_change_a_disposition
#print axioms QCore.BoundFieldRead.the_custody_whitelist_is_not_vacuous
#print axioms QCore.BoundFieldRead.the_uninsulated_custody_read_is_refused
#print axioms QCore.BoundFieldRead.the_borrowed_half_refuses_the_custody_example
#print axioms QCore.BoundFieldRead.one_scalar_member_flips_the_custody_read
#print axioms QCore.BoundFieldRead.one_owned_position_flips_the_custody_read
