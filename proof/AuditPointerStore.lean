import QCore.PointerStore

/-! Audited companion to CG11-POINTER-VALUED-STORES (gateway rule 10v). -/

#print axioms QCore.PointerStore.the_final_owner_decides_resolution
#print axioms QCore.PointerStore.decision_vectors_hold
#print axioms QCore.PointerStore.admitted_exactly_when
#print axioms QCore.PointerStore.example_c_is_refused
#print axioms QCore.PointerStore.an_owned_field_is_never_overwritten
#print axioms QCore.PointerStore.an_unresolved_owner_refuses
#print axioms QCore.PointerStore.another_root_is_another_object
#print axioms QCore.PointerStore.a_deeper_value_place_is_another_object
#print axioms QCore.PointerStore.a_direct_member_place_is_not_the_root_object
#print axioms QCore.PointerStore.frame_preserved
#print axioms QCore.PointerStore.no_obligation_lost_on_overwrite
#print axioms QCore.PointerStore.an_owning_field_overwrite_refuses
#print axioms QCore.PointerStore.a_tracked_old_value_refuses
#print axioms QCore.PointerStore.no_double_release_through_a_stored_alias
#print axioms QCore.PointerStore.the_member_release_shape_is_refused
#print axioms QCore.PointerStore.an_unlisted_destination_refuses
#print axioms QCore.PointerStore.a_never_bound_root_owns_untracked
#print axioms QCore.PointerStore.a_live_root_is_a_tracked_owner
