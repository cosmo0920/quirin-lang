import QCore.ResultSet

/-! Audited companion to CELL-GUARD-RESULT-RANGE-CHECKED (contract specification C09.4, grammar G05j, gateway rule 38e; user decision 2026-11-03, option C, a result-set form that allows unions, with the programming-language operators | & ^ as union, intersection and symmetric difference): the operators mean union, intersection and symmetric difference over the integers of the C type, every member lies in the universe; a test that reads a contract class exactly on a known set decides that class on both edges and keeps reading it on every narrower set; a set out of force (held, withdrawn, refuted, unchecked, unbound) is replaced by the whole universe, so no decision rests on an assumed set; the Rust set, member and class vectors hold. -/

#print axioms QCore.ResultSet.denote_or
#print axioms QCore.ResultSet.denote_and
#print axioms QCore.ResultSet.denote_xor
#print axioms QCore.ResultSet.denote_in_universe
#print axioms QCore.ResultSet.holds_true_edge
#print axioms QCore.ResultSet.holds_false_edge
#print axioms QCore.ResultSet.fails_true_edge
#print axioms QCore.ResultSet.fails_false_edge
#print axioms QCore.ResultSet.narrowing_keeps_holds
#print axioms QCore.ResultSet.narrowing_keeps_fails
#print axioms QCore.ResultSet.out_of_force_reads_the_universe
#print axioms QCore.ResultSet.a_decision_out_of_force_is_exact_on_every_value
#print axioms QCore.ResultSet.set_vectors_hold
#print axioms QCore.ResultSet.member_vectors_hold
#print axioms QCore.ResultSet.class_vectors_hold
