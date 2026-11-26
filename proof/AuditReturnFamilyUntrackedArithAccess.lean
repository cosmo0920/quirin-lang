import QCore.ReturnFamilyUntrackedArithAccess

/-! Card **RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC**: an untracked,
non-scalar root of family S (`legacy_return_family`) that `->`, `[` or a
unary `*` accesses through a parenthesised additive group, as well as cast
groups and parentheses, is a base of the read node, and a `->` root carries
its C02-P-ALIAS reading (gateway rule 74w). An access through a sum whose
other operands the reader cannot classify refuses.

Every old base and `->` root is kept. With no sum crossed or no access, the
new test reads nothing. Bases and `->` roots only add obligations and the one
new refusal is the expression's, so nothing admits more than at the base.

Each layer's and context's class (the Rust reader's output, bound to
`arithVectors`) is trust boundary 28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_arith_access.rs`
and by `crates/qcore-c-gateway/tests/return_family_untracked_arith_access.rs`. -/

-- The test.
#print axioms QCore.ReturnFamilyUntrackedArithAccess.no_sum_keeps_the_reading
#print axioms QCore.ReturnFamilyUntrackedArithAccess.no_access_keeps_the_reading
#print axioms QCore.ReturnFamilyUntrackedArithAccess.a_classified_sum_is_accessed
#print axioms QCore.ReturnFamilyUntrackedArithAccess.an_unclassified_access_refuses

-- The read: old readings kept, nothing admits more.
#print axioms QCore.ReturnFamilyUntrackedArithAccess.old_base_kept
#print axioms QCore.ReturnFamilyUntrackedArithAccess.old_arrow_kept
#print axioms QCore.ReturnFamilyUntrackedArithAccess.names_kept
#print axioms QCore.ReturnFamilyUntrackedArithAccess.the_fix_only_refuses_more

-- The readings.
#print axioms QCore.ReturnFamilyUntrackedArithAccess.old_misses_the_base
#print axioms QCore.ReturnFamilyUntrackedArithAccess.another_pointer_refuses

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyUntrackedArithAccess.arith_vectors_hold
