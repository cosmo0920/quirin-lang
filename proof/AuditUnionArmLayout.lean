import QCore.UnionArmLayout

/-! Audited companion to RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION (gateway rule 12n, amended): a union's places are separated by Clang's record layout. A write over bytes disjoint from a read's never changes the read; two places the rule keeps apart are known and disjoint, so a store into one never changes a read of the other; the whole union, a place whose bytes are not given and a union two units lay out differently alias every place; the same arm path always aliases and aliasing is symmetric; the layout only narrows Q3's sharing, never adding a store a member takes. -/

#print axioms QCore.UnionArmLayout.a_disjoint_write_never_changes_the_read
#print axioms QCore.UnionArmLayout.separated_places_are_disjoint
#print axioms QCore.UnionArmLayout.a_separated_store_never_changes_the_read
#print axioms QCore.UnionArmLayout.an_unplaced_place_aliases
#print axioms QCore.UnionArmLayout.an_ungiven_range_aliases
#print axioms QCore.UnionArmLayout.the_same_arm_aliases
#print axioms QCore.UnionArmLayout.places_alias_is_symmetric
#print axioms QCore.UnionArmLayout.disagreeing_units_unplace
#print axioms QCore.UnionArmLayout.a_disagreeing_union_aliases_everything
#print axioms QCore.UnionArmLayout.the_layout_takes_only_shared_stores
#print axioms QCore.UnionArmLayout.layout_vectors_hold
