import QCore.SlotMove

/-! Audited companion to SLOT-MOVE-AND-INTERIOR-BORROW: class 2 slot moves (gateway rule 12x). -/

#print axioms QCore.SlotMove.a_move_needs_one_identity_held_nowhere_else
#print axioms QCore.SlotMove.an_identity_held_elsewhere_is_refused
#print axioms QCore.SlotMove.run_append
#print axioms QCore.SlotMove.step_keeps_refused
#print axioms QCore.SlotMove.run_keeps_refused
#print axioms QCore.SlotMove.lookup_filter_keeps_slot
#print axioms QCore.SlotMove.step_move
#print axioms QCore.SlotMove.the_slot_keeps_its_identity_and_its_obligation
#print axioms QCore.SlotMove.every_old_alias_is_invalid_after_a_move
#print axioms QCore.SlotMove.a_use_of_an_old_alias_is_refused
#print axioms QCore.SlotMove.the_failure_arm_keeps_the_original
#print axioms QCore.SlotMove.a_safe_path_never_uses_an_invalid_name
#print axioms QCore.SlotMove.slot_vectors_hold
#print axioms QCore.SlotMove.slot_trace_vectors_hold
