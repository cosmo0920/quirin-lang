import QCore.CellProtocolCloseActual

/-! Audited companion to CELL-PROTOCOL-CLOSE-ACTUAL (gateway rule 38d; user decision 2026-11-02, "B as recommended", Q3: a close of a declared cell member is that cell's `#stale`): the access gate admits exactly one more actual at `close`'s custody position (rule 38 in force, rule 38c's close statement of a declared record's one-segment cell, a function-map callee, a descriptor-kind release position, the integer member read of the step's own cell) and every other site keeps the refusal; an admitted close feeds rule 38c's `#stale` step (a live single identity released once, its pending names staled, no live name of it left) and is never a silent release (no fact: the gate's refusal; a second close, the dead edge and the join's identity refuse); two admitted closes and the sentinel's dead store restore the protocol; the Rust admission and reading vectors hold. -/

#print axioms QCore.CellProtocolCloseActual.the_admission_set_is_exactly_the_stated_one
#print axioms QCore.CellProtocolCloseActual.every_other_site_keeps_the_refusal
#print axioms QCore.CellProtocolCloseActual.rule_38_off_admits_nothing
#print axioms QCore.CellProtocolCloseActual.an_admitted_close_without_a_fact_refuses
#print axioms QCore.CellProtocolCloseActual.an_admitted_close_is_the_cells_stale
#print axioms QCore.CellProtocolCloseActual.the_closed_identity_has_no_live_name
#print axioms QCore.CellProtocolCloseActual.an_admitted_close_never_releases_silently
#print axioms QCore.CellProtocolCloseActual.admitted_closes_and_the_sentinel_restore_the_protocol
#print axioms QCore.CellProtocolCloseActual.a_second_admitted_close_refuses
#print axioms QCore.CellProtocolCloseActual.a_close_of_the_same_name_after_its_target_refuses
#print axioms QCore.CellProtocolCloseActual.a_close_on_the_dead_edge_refuses
#print axioms QCore.CellProtocolCloseActual.a_close_of_the_joined_identity_refuses
#print axioms QCore.CellProtocolCloseActual.admission_vectors_hold
#print axioms QCore.CellProtocolCloseActual.reading_vectors_hold
