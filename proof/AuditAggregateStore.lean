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

import QCore.TypedFieldAccess
import QCore.ExitResiduals
import QCore.AggregateStore

/-! Card O04-B08: the typed aggregate field store (O00 blocker B08) and the
containment edge it records. Both fail-closed directions are audited — every
premise of the destination whitelist refuses on its own, the flow-sensitive
aggregate-liveness gate refuses independently of the declarations, and neither a
release of the aggregate nor a second store into the same member ever discharges
a linked identity. Non-vacuity is audited in both directions. -/

-- Every premise of the destination whitelist refuses on its own.
#print axioms QCore.AggregateStore.unread_declarations_never_admit_a_store
#print axioms QCore.AggregateStore.a_formal_base_never_admits_a_store
#print axioms QCore.AggregateStore.a_non_record_pointer_base_never_admits_a_store
#print axioms QCore.AggregateStore.an_incomplete_record_never_admits_a_store
#print axioms QCore.AggregateStore.a_rebound_base_never_admits_a_store
#print axioms QCore.AggregateStore.an_unacquired_base_never_admits_a_store
#print axioms QCore.AggregateStore.an_escaping_base_pointer_never_admits_a_store
#print axioms QCore.AggregateStore.an_aliased_base_never_admits_a_store
#print axioms QCore.AggregateStore.a_chained_destination_never_admits_a_store
#print axioms QCore.AggregateStore.a_refused_member_shape_never_admits_a_store
#print axioms QCore.AggregateStore.a_qualified_member_never_admits_a_store
#print axioms QCore.AggregateStore.a_tracked_value_never_reaches_a_scalar_member
#print axioms QCore.AggregateStore.a_null_value_never_reaches_a_scalar_member

-- The flow-sensitive aggregate premise: card C02's rule in its store direction.
#print axioms QCore.AggregateStore.an_unbound_aggregate_never_resolves
#print axioms QCore.AggregateStore.an_ambiguous_aggregate_never_resolves
#print axioms QCore.AggregateStore.an_unknown_aggregate_never_resolves
#print axioms QCore.AggregateStore.a_possibly_null_aggregate_never_resolves
#print axioms QCore.AggregateStore.a_caller_held_aggregate_never_resolves
#print axioms QCore.AggregateStore.a_released_aggregate_never_resolves
#print axioms QCore.AggregateStore.an_escaped_aggregate_never_resolves
#print axioms QCore.AggregateStore.an_unresolved_aggregate_never_resolves

-- Fail-closed transfer: both gates are required, and neither alone suffices.
#print axioms QCore.AggregateStore.a_refused_store_never_links
#print axioms QCore.AggregateStore.an_unresolved_owner_never_links
#print axioms QCore.AggregateStore.only_a_doubly_admitted_store_links
#print axioms QCore.AggregateStore.an_untracked_value_never_links
#print axioms QCore.AggregateStore.a_null_value_never_links

-- The containment edge, and premise P-B08-SHALLOW.
#print axioms QCore.AggregateStore.an_escaping_aggregate_carries_its_live_child
#print axioms QCore.AggregateStore.the_escape_cascade_never_releases
#print axioms QCore.AggregateStore.an_escaping_aggregate_leaves_a_released_child_unresolved
#print axioms QCore.AggregateStore.a_released_aggregate_changes_no_child
#print axioms QCore.AggregateStore.a_released_aggregate_never_discharges_a_live_child
#print axioms QCore.AggregateStore.a_double_store_never_discharges_the_first_identity
#print axioms QCore.AggregateStore.a_double_store_changes_no_disposition

-- Bridging into the exit ledger.
#print axioms QCore.AggregateStore.a_refused_store_settles_no_exit
#print axioms QCore.AggregateStore.a_live_linked_child_is_a_residual

-- Non-vacuity, in both directions.
#print axioms QCore.AggregateStore.the_store_whitelist_is_not_vacuous
#print axioms QCore.AggregateStore.the_admitted_store_links
#print axioms QCore.AggregateStore.one_refused_member_shape_flips_the_admitted_store
#print axioms QCore.AggregateStore.one_unacquired_base_flips_the_admitted_store
#print axioms QCore.AggregateStore.one_aliased_base_flips_the_admitted_store
#print axioms QCore.AggregateStore.a_released_owner_flips_the_admitted_store
#print axioms QCore.AggregateStore.a_caller_held_owner_flips_the_admitted_store

-- Card ALIAS-AGGREGATE: the third member class. Admitted as a store
-- destination, refused as a bound read (audited in AuditBoundFieldRead), with
-- every other premise of the whitelist unchanged.
#print axioms QCore.AggregateStore.an_opaque_member_store_is_admitted
#print axioms QCore.AggregateStore.the_admitted_opaque_store_links
#print axioms QCore.AggregateStore.the_opaque_member_does_not_require_an_exact_tag
#print axioms QCore.AggregateStore.an_opaque_member_refuses_a_value_that_is_not_a_declared_record_pointer
#print axioms QCore.AggregateStore.a_single_pointer_member_still_requires_the_exact_tag
#print axioms QCore.AggregateStore.an_opaque_member_store_keeps_every_base_premise
#print axioms QCore.AggregateStore.an_opaque_member_store_keeps_the_qualifier_premise
#print axioms QCore.AggregateStore.a_released_owner_flips_the_admitted_opaque_store
#print axioms QCore.AggregateStore.a_caller_held_owner_flips_the_admitted_opaque_store
#print axioms QCore.AggregateStore.the_member_classes_are_exclusive
