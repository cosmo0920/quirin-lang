import QCore.TypeGroupQualifiedPointer

/-! Card **TYPE-GROUP-QUALIFIED-POINTER**: `cast_semantics::type_group_open`
reads C11 6.7.7's abstract declarator `pointer`, a `*` followed by a possibly
empty type-qualifier list, any number of times (gateway rule 74u).

The recogniser is exactly the grammar, it accepts every group the base
accepted, and every consumer (rule 12h(4)'s member count, rule 74m's through
count, rule 74r's starred base) only refuses more through more groups, so
family S admits at the tip only what it admitted at the base.

Each token's class and each step back from a root (the Rust reader's
output, bound to `typeGroupVectors`) is trust boundary 28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/cast_semantics_type_group.rs`,
`crates/qcore-c-gateway/tests/unit/legacy_return_family_type_group.rs` and
`crates/qcore-c-gateway/tests/return_family_type_group.rs`. -/

-- The grammar.
#print axioms QCore.TypeGroupQualifiedPointer.ptrs_iff
#print axioms QCore.TypeGroupQualifiedPointer.pointer_iff
#print axioms QCore.TypeGroupQualifiedPointer.specs_iff
#print axioms QCore.TypeGroupQualifiedPointer.accepts_iff
#print axioms QCore.TypeGroupQualifiedPointer.old_le_new
#print axioms QCore.TypeGroupQualifiedPointer.the_qualified_group_is_new
#print axioms QCore.TypeGroupQualifiedPointer.a_product_is_no_group

-- Each consumer is monotone.
#print axioms QCore.TypeGroupQualifiedPointer.count_mono
#print axioms QCore.TypeGroupQualifiedPointer.member_admission_shrinks
#print axioms QCore.TypeGroupQualifiedPointer.starred_admission_shrinks
#print axioms QCore.TypeGroupQualifiedPointer.through_mono
#print axioms QCore.TypeGroupQualifiedPointer.bases_grow
#print axioms QCore.TypeGroupQualifiedPointer.the_fix_only_refuses_more
#print axioms QCore.TypeGroupQualifiedPointer.qualified_only_refuses_more

-- The readings.
#print axioms QCore.TypeGroupQualifiedPointer.old_misses_the_qualified_group
#print axioms QCore.TypeGroupQualifiedPointer.a_second_dereference_refuses
#print axioms QCore.TypeGroupQualifiedPointer.a_member_root_through_the_group_refuses

-- The token reader's vectors.
#print axioms QCore.TypeGroupQualifiedPointer.type_group_vectors_hold
#print axioms QCore.TypeGroupQualifiedPointer.vectors_only_add
