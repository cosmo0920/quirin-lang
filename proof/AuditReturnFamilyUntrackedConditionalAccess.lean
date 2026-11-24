import QCore.ReturnFamilyUntrackedConditionalAccess

/-! Card **RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS**: when `->`, `[` or a
unary `*` accesses a parenthesised conditional of family S
(`legacy_return_family`), through cast groups, parentheses and classified
additive groups, every untracked, non-scalar root of either arm is a base of
the read node, and a `->` root carries its C02-P-ALIAS reading (gateway rule
74x). The condition is a read and is never decided, even when constant. An
arm the reader cannot classify, or an access through an unclassified
additive group, refuses.

Every old base and `->` root is kept. A group nothing accesses reads
nothing. Bases and `->` roots only add obligations and the one new refusal
is the expression's, so nothing admits more than at the base.

Each group's layers, context and arms (the Rust reader's output, bound to
`condVectors`) are trust boundary 28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_conditional_access.rs`
and by `crates/qcore-c-gateway/tests/return_family_untracked_conditional_access.rs`. -/

-- The arms: both taken, never the condition, an unclassified arm refuses.
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.both_arms_are_taken
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.the_condition_is_not_decided
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.an_unclassifiable_arm_refuses

-- The group's access.
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.no_access_keeps_the_reading
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.an_unclassified_group_refuses

-- The read: old readings kept, nothing admits more.
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.old_base_kept
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.old_arrow_kept
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.names_kept
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.the_fix_only_refuses_more

-- The readings.
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.old_misses_the_base
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.the_condition_is_no_base

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyUntrackedConditionalAccess.cond_vectors_hold
