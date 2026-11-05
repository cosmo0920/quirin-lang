import QCore.CellProtocolReleaseWindowFrames

/-! Audited companion to CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (gateway rule 38g; user decision 2026-11-04, "(a) Callee frames"): a callee whose footprint excludes a record's protocol members keeps every end and reads none; a unit's body check establishes a frame only over bodies that name no protocol member, read in full and call only established bodies, never through a missing or recursive body; a frame stands only where declared, passes unheld only when checked or accepted, is held for review when only declared, never has a refutation lifted by acceptance, and without one the window refuses as before; the Rust body and window vectors hold. -/

#print axioms QCore.CellProtocolReleaseWindowFrames.a_framed_call_keeps_every_end
#print axioms QCore.CellProtocolReleaseWindowFrames.a_framed_call_reads_no_end
#print axioms QCore.CellProtocolReleaseWindowFrames.an_established_body_names_no_member
#print axioms QCore.CellProtocolReleaseWindowFrames.an_established_body_calls_only_established_bodies
#print axioms QCore.CellProtocolReleaseWindowFrames.a_missing_body_is_never_established
#print axioms QCore.CellProtocolReleaseWindowFrames.a_recursive_body_is_never_established
#print axioms QCore.CellProtocolReleaseWindowFrames.no_declaration_no_frame
#print axioms QCore.CellProtocolReleaseWindowFrames.only_a_checked_or_accepted_frame_passes_unheld
#print axioms QCore.CellProtocolReleaseWindowFrames.a_declared_frame_is_held
#print axioms QCore.CellProtocolReleaseWindowFrames.acceptance_never_lifts_a_refutation
#print axioms QCore.CellProtocolReleaseWindowFrames.without_a_frame_the_window_refuses_as_before
#print axioms QCore.CellProtocolReleaseWindowFrames.window_vectors_hold
#print axioms QCore.CellProtocolReleaseWindowFrames.body_vectors_hold
