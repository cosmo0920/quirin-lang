import QCore.DescriptorCells

/-! Audited companion to MK-EVENT-DESCRIPTOR-QUANT (gateway rule 38, contract specification C04.6.2; user decision 2026-11-01, "I approve the changes of D1-D5."): a descriptor is an identity and a cell a name of it; a fresh identity is released once, a second release through any name refuses; the witness decides a creation's class and an untested one never discharges; an untracked cell decides nothing and a live or undecided cell refuses at an exit; the member-equality guard is decided only from live identities (distinct numbers under the hypothesis that two open descriptors never share one), an undecided guard refuses and changes no cell, and a guarded removal then keeps and poisons its record; removal and release commute; the shorthand and its canonical parts are one set; the Rust decision vectors hold. -/

#print axioms QCore.DescriptorCells.a_fresh_identity_is_owed_once
#print axioms QCore.DescriptorCells.a_stale_cell_never_releases_again
#print axioms QCore.DescriptorCells.the_same_alias_releases_once
#print axioms QCore.DescriptorCells.the_witness_decides_the_cell
#print axioms QCore.DescriptorCells.an_untested_witness_never_discharges
#print axioms QCore.DescriptorCells.an_untracked_cell_decides_nothing
#print axioms QCore.DescriptorCells.an_escaping_cell_refuses
#print axioms QCore.DescriptorCells.equal_identities_decide_eq
#print axioms QCore.DescriptorCells.distinct_live_descriptors_decide_ne
#print axioms QCore.DescriptorCells.an_undecided_guard_gives_the_join
#print axioms QCore.DescriptorCells.the_join_never_releases
#print axioms QCore.DescriptorCells.an_undecided_guarded_removal_keeps_and_poisons
#print axioms QCore.DescriptorCells.removal_and_cell_release_commute
#print axioms QCore.DescriptorCells.each_shorthand_and_its_parts_mean_the_same
#print axioms QCore.DescriptorCells.both_levels_are_one_canonical_set
#print axioms QCore.DescriptorCells.decision_vectors_hold
