import QCore.ReturnFamilyArithWalkConditional

/-! Card **RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL**: rule 74w's walk in
family S (`legacy_return_family`) no longer refuses only because it read a
parenthesised conditional as the additive group around a term of an arm's
sum (gateway rule 82). It classifies such a group within the arm, the arm's
`?` or `:` standing in for the parentheses; when that is the only
unclassified reading, family S refuses unless rule 74x made the root a base.
A root in a condition and every other unclassified group keep the refusal.

The only refusal removed is the arm delimiter's, and the root it covered is
a rule 74x base whose obligation discharges wherever the tip admits. No
refusal is added, every old base and `->` root is kept, and nothing admits
more than a reading in which both rules read the arm.

Each crossing's classifications, the context and rule 74x's verdict (the
Rust reader's output, bound to `walkVectors`) are trust boundary 28's. It is
pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_arith_walk_conditional.rs`
and by `crates/qcore-c-gateway/tests/return_family_arith_walk_conditional.rs`. -/

-- The walk: the arm outcome replaces only an arm-delimited refusal.
#print axioms QCore.ReturnFamilyArithWalkConditional.walk_off_the_arm
#print axioms QCore.ReturnFamilyArithWalkConditional.an_arm_was_unclassified
#print axioms QCore.ReturnFamilyArithWalkConditional.a_condition_keeps_its_refusal
#print axioms QCore.ReturnFamilyArithWalkConditional.another_group_keeps_its_refusal

-- One root: the only refusal removed, none added, old readings kept.
#print axioms QCore.ReturnFamilyArithWalkConditional.old_refuses_the_arm
#print axioms QCore.ReturnFamilyArithWalkConditional.no_new_refusal
#print axioms QCore.ReturnFamilyArithWalkConditional.the_only_refusal_removed
#print axioms QCore.ReturnFamilyArithWalkConditional.old_base_kept
#print axioms QCore.ReturnFamilyArithWalkConditional.old_arrow_kept
#print axioms QCore.ReturnFamilyArithWalkConditional.both_refusal_kept
#print axioms QCore.ReturnFamilyArithWalkConditional.both_base_kept
#print axioms QCore.ReturnFamilyArithWalkConditional.both_arrow_kept

-- The read: fail closed.
#print axioms QCore.ReturnFamilyArithWalkConditional.names_kept
#print axioms QCore.ReturnFamilyArithWalkConditional.admits_weaker
#print axioms QCore.ReturnFamilyArithWalkConditional.where_the_base_admitted_nothing_admits_more
#print axioms QCore.ReturnFamilyArithWalkConditional.nothing_admits_more_than_both
#print axioms QCore.ReturnFamilyArithWalkConditional.a_relaxed_root_is_discharged

-- The readings.
#print axioms QCore.ReturnFamilyArithWalkConditional.the_arm_is_read
#print axioms QCore.ReturnFamilyArithWalkConditional.the_condition_still_refuses
#print axioms QCore.ReturnFamilyArithWalkConditional.an_arm_74x_refuses_still_refuses

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyArithWalkConditional.walk_vectors_hold
