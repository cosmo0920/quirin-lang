import QCore.MayAlias

/-! Card C02-P-ALIAS: the bounded may-alias refinement.

Card C02-P recorded, as a boundary of the whole resource projection, that two
distinct declared names may denote the same object and that a name-based
projection relates them not at all. This audit covers the refinement that closes
it for one bounded relation, in both directions.

* **The class relation** — symmetric, never reflexive, never across declared
  record tags, never for a name the reader will not type
  (`C02-P-ALIAS-UNTYPED`) and never for two locals (`C02-P-ALIAS-LOCALS`), with
  the two positive directions audited so the refusals are not vacuous.
* **The refusal** — a released, undischarged peer refuses; and conversely every
  refusal names one, so the rule invents nothing.
* **Both discharges** — provenance refuses an unbound, a dead, a caller-held and
  an ambiguous binding, each on its own; a guard discharges exactly its own
  normalized pair, is symmetric, and is dropped when either of its names is
  rebound while a third name's rebinding leaves it intact.
* **Rebinding** — never removes a release, so the peer of a reassigned releasing
  name stays refused; and never adds a guard.
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
#print axioms QCore.MayAlias.two_locals_are_never_peers
#print axioms QCore.MayAlias.two_same_tagged_formals_are_peers
#print axioms QCore.MayAlias.a_formal_and_a_local_are_peers

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

-- Rebinding never discharges a release.
#print axioms QCore.MayAlias.rebinding_only_drops_guards
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
