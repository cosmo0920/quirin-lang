import QCore.CellProtocolPerEnd

/-! Audited companion to CELL-PROTOCOL-PER-END-GUARDS (gateway rule 38f; user decision 2026-11-03, "Per-end sentinels"): each channel end carries its own sentinel; a sentinel's dead edge decides "no cell is live" exactly when no possible descriptor is on it, so `!= -1`'s does and `> 0`'s does not (descriptor 0, a success value of every creator, is on it); a `> 0` dead edge is never read as absent and a close there refuses; a join never drops a name that may be live and two stale names join stale; the holder's release ends the protocol only without a live cell, naming the zero reading; a record keeps its protocols, and its release refuses, end by end; the Rust per-end vectors hold. -/

#print axioms QCore.CellProtocolPerEnd.dead_edge_decides_iff_no_descriptor_on_it
#print axioms QCore.CellProtocolPerEnd.descriptor_zero_is_on_the_positive_dead_edge
#print axioms QCore.CellProtocolPerEnd.the_live_edge_is_disjoint_from_the_dead_set
#print axioms QCore.CellProtocolPerEnd.the_positive_dead_edge_never_reads_absent
#print axioms QCore.CellProtocolPerEnd.the_minus_one_dead_edge_reads_absent
#print axioms QCore.CellProtocolPerEnd.a_close_on_the_positive_dead_edge_refuses
#print axioms QCore.CellProtocolPerEnd.a_join_never_drops_a_possibly_live_name
#print axioms QCore.CellProtocolPerEnd.the_table_join_keeps_a_possibly_live_name
#print axioms QCore.CellProtocolPerEnd.two_stale_names_join_stale
#print axioms QCore.CellProtocolPerEnd.a_holder_release_ends_the_protocol_only_without_a_live_cell
#print axioms QCore.CellProtocolPerEnd.a_zero_reading_names_the_release
#print axioms QCore.CellProtocolPerEnd.a_record_keeps_its_protocols_iff_every_end_does
#print axioms QCore.CellProtocolPerEnd.a_record_release_refuses_iff_some_end_may_hold_live
#print axioms QCore.CellProtocolPerEnd.per_end_vectors_hold
