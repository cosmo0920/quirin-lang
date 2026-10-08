import QCore.DerivedOutSlot

/-! Audited companion to SLOT-MOVE-AND-INTERIOR-BORROW: derived out slots (gateway rule 12z). -/

#print axioms QCore.DerivedOutSlot.a_tracked_slot_is_never_overwritten
#print axioms QCore.DerivedOutSlot.a_bind_needs_an_untracked_slot
#print axioms QCore.DerivedOutSlot.run_append
#print axioms QCore.DerivedOutSlot.step_keeps_refused
#print axioms QCore.DerivedOutSlot.run_keeps_refused
#print axioms QCore.DerivedOutSlot.the_slot_value_is_never_released
#print axioms QCore.DerivedOutSlot.the_base_release_ends_the_slot_value
#print axioms QCore.DerivedOutSlot.a_safe_path_never_uses_a_value_its_base_outlived
#print axioms QCore.DerivedOutSlot.out_vectors_hold
