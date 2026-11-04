import QCore.CellResultSet

/-! Audited companion to CELL-RESULT-SET-ALIGNMENT (gateway rule 38f as generalised; user decisions 2026-11-04, "Basically, Q1(a) and Q2(i) ...", "Write [1, MAX]", "(α) live_set suffix", "Accept the leak"): a per-end sentinel's cell carries a live set and an absent set; a dead edge reads "absent" only when it meets the live set nowhere and a live edge reads "live" only when it meets the absent set nowhere, each sound for every value; without declared sets the reading is rule 38f's; the declared [1, MAX] decides the `> 0` dead edge while POSIX's [0, MAX] leaves descriptor 0 on it; the Rust set vectors hold. -/

#print axioms QCore.CellResultSet.meets_of_mem
#print axioms QCore.CellResultSet.the_dead_edge_reading_is_sound
#print axioms QCore.CellResultSet.the_live_edge_reading_is_sound
#print axioms QCore.CellResultSet.without_sets_the_reading_is_rule_38f
#print axioms QCore.CellResultSet.the_declared_bound_decides_the_positive_dead_edge
#print axioms QCore.CellResultSet.posix_leaves_descriptor_zero_on_the_positive_dead_edge
#print axioms QCore.CellResultSet.set_vectors_hold
