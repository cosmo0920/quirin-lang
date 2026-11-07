import QCore.OutSlotCallWrite

/-! Audited companion to OUT-SLOT-CALL-WRITE-ALIASING (gateway rule 11r(8)). -/

#print axioms QCore.OutSlotCallWrite.no_reaching_actual_designates
#print axioms QCore.OutSlotCallWrite.kept_when_no_actual_reaches
#print axioms QCore.OutSlotCallWrite.a_reaching_call_poisons_a_bound_slot
#print axioms QCore.OutSlotCallWrite.bound_kept_only_when_excluded
#print axioms QCore.OutSlotCallWrite.unwritten_stays_sound
#print axioms QCore.OutSlotCallWrite.transfer_sound
#print axioms QCore.OutSlotCallWrite.the_old_reading_is_unsound
#print axioms QCore.OutSlotCallWrite.the_memcpy_shape_poisons
#print axioms QCore.OutSlotCallWrite.distinct_locals_keep
#print axioms QCore.OutSlotCallWrite.a_const_source_keeps
