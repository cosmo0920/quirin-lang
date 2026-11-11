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

import QCore.MayAlias

/-! Card C02-P-ALIAS: the bounded may-alias refinement, extended by card
ALIAS-LOCALS.

Card C02-P recorded, as a boundary of the whole resource projection, that two
distinct declared names may denote the same object and that a name-based
projection relates them not at all. This audit covers the refinement that closes
it for one bounded relation, in both directions.

* **The class relation** — symmetric, never reflexive, never across declared
  record tags and never for a name the reader will not type
  (`C02-P-ALIAS-UNTYPED`), with the positive direction audited so the refusals
  are not vacuous. Card ALIAS-LOCALS lifted v1's "at least one formal" bound, so
  two same-tagged **locals** are peers; that retired v1's
  `two_locals_are_never_peers`, `two_same_tagged_formals_are_peers` and
  `a_formal_and_a_local_are_peers` by name, replaced by
  `two_same_tagged_names_are_peers` and `two_locals_of_one_class_are_peers`
  (`docs/reports/remaining-alias-locals/README.md` §5).
* **The refusal** — a released, undischarged peer refuses; and conversely every
  refusal names one, so the rule invents nothing.
* **Both discharges** — provenance refuses an unbound, a dead, a caller-held and
  an ambiguous binding, each on its own; a guard discharges exactly its own
  normalized pair, is symmetric, and is dropped when either of its names is
  rebound while a third name's rebinding leaves it intact.
* **The definite-alias relation kind** (ALIAS-LOCALS) — symmetric, positive and
  negative directions, fail-closed on every uncommitted binding, defeating the
  guard, leaving a non-definite pair's guard discharge exactly as it was,
  refining rather than widening the peer relation, and load-bearing.
* **Rebinding** — never removes a release, so the peer of a reassigned releasing
  name stays refused; and never adds a guard or a guard discharge.
* **The join** — releases union, guards intersect, a refusal on either incoming
  path survives, and the two lengths move in the directions the work list's
  termination argument needs.
* **The verdict** — the rule can only move `safe` to `unresolved`; it can never
  promote. -/

-- The class relation.
#print axioms QCore.MayAlias.sameTag_is_symmetric
#print axioms QCore.MayAlias.peer_is_symmetric
#print axioms QCore.MayAlias.a_name_is_never_its_own_peer
#print axioms QCore.MayAlias.a_different_tag_is_never_a_peer
#print axioms QCore.MayAlias.an_untyped_name_is_never_a_peer
#print axioms QCore.MayAlias.two_same_tagged_names_are_peers
#print axioms QCore.MayAlias.two_locals_of_one_class_are_peers

-- The refusal, in both directions.
#print axioms QCore.MayAlias.a_released_peer_refuses_the_read
#print axioms QCore.MayAlias.a_refusal_names_a_released_undischarged_peer
#print axioms QCore.MayAlias.no_release_refuses_nothing
#print axioms QCore.MayAlias.releasing_a_name_does_not_refuse_itself
#print axioms QCore.MayAlias.releasing_a_non_peer_refuses_nothing

-- The provenance discharge, and every way it fails closed.
#print axioms QCore.MayAlias.a_live_body_made_binding_discharges
#print axioms QCore.MayAlias.an_unbound_name_is_never_discharged
#print axioms QCore.MayAlias.a_dead_binding_never_discharges
#print axioms QCore.MayAlias.a_caller_held_binding_never_discharges
#print axioms QCore.MayAlias.an_ambiguous_binding_never_discharges

-- The guard discharge, and its invalidation.
#print axioms QCore.MayAlias.the_guard_discharges_its_pair
#print axioms QCore.MayAlias.the_guard_is_symmetric
#print axioms QCore.MayAlias.a_guard_discharges_no_other_peer
#print axioms QCore.MayAlias.rebinding_drops_the_guard
#print axioms QCore.MayAlias.rebinding_the_other_name_drops_the_guard
#print axioms QCore.MayAlias.rebinding_an_unrelated_name_keeps_the_guard

-- The definite-alias relation kind (ALIAS-LOCALS).
#print axioms QCore.MayAlias.two_names_bound_to_one_identity_are_definitely_aliased
#print axioms QCore.MayAlias.two_names_bound_to_different_identities_are_not
#print axioms QCore.MayAlias.the_definite_relation_is_symmetric
#print axioms QCore.MayAlias.an_uncommitted_binding_is_never_a_definite_alias
#print axioms QCore.MayAlias.an_unbound_name_is_never_a_definite_alias
#print axioms QCore.MayAlias.a_definite_alias_defeats_the_guard
#print axioms QCore.MayAlias.the_guard_still_discharges_a_non_definite_pair
#print axioms QCore.MayAlias.a_released_definite_alias_refuses_under_a_guard
#print axioms QCore.MayAlias.the_definite_alias_exception_is_load_bearing
#print axioms QCore.MayAlias.a_definite_alias_of_untyped_names_still_refuses_nothing

-- Rebinding never discharges a release.
#print axioms QCore.MayAlias.rebinding_only_drops_guards
#print axioms QCore.MayAlias.rebinding_only_drops_the_guard_discharge
#print axioms QCore.MayAlias.rebinding_preserves_every_release
#print axioms QCore.MayAlias.rebinding_the_releasing_name_keeps_the_refusal

-- Monotonicity and the join.
#print axioms QCore.MayAlias.adding_a_release_only_refuses_more
#print axioms QCore.MayAlias.the_join_unions_the_releases
#print axioms QCore.MayAlias.the_join_only_keeps_shared_guards
#print axioms QCore.MayAlias.the_join_preserves_a_refusal
#print axioms QCore.MayAlias.the_join_intersects_the_guards
#print axioms QCore.MayAlias.the_join_keeps_a_guard_both_paths_proved
#print axioms QCore.MayAlias.the_join_never_shortens_the_releases
#print axioms QCore.MayAlias.the_join_never_lengthens_the_guards

-- The rule adds refusals only, and is not vacuous.
#print axioms QCore.MayAlias.a_refusal_is_unresolved
#print axioms QCore.MayAlias.no_refusal_changes_nothing
#print axioms QCore.MayAlias.the_rule_never_promotes
#print axioms QCore.MayAlias.the_rule_is_not_vacuous
#print axioms QCore.MayAlias.the_guard_discharge_is_load_bearing
#print axioms QCore.MayAlias.the_provenance_discharge_is_load_bearing
