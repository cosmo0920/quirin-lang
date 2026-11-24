import QCore.ReturnFamilyUntrackedDerefCast

/-! Card **RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST**: an untracked root of
family S (`legacy_return_family`) that a `*` dereferences through opening
parentheses and cast groups is a base of the read node (gateway rule 74r,
`QCore.ReturnFamilyStarCast` not restated here).

Every base the base read is kept, a base only adds an obligation so nothing
admits more than at the base, every reading C may take of rule 74h's
undecided groups keeps its base, and a word of a cast's type name keeps the
base's reading.

Each item's class (the Rust reader's output, bound to `untrackedVectors`) is
trust boundary 28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_deref_cast.rs`
and by `crates/qcore-c-gateway/tests/return_family_untracked_deref_cast.rs`. -/

-- The two readings.
#print axioms QCore.ReturnFamilyUntrackedDerefCast.old_starred_kept
#print axioms QCore.ReturnFamilyUntrackedDerefCast.a_type_word_keeps_the_old_reading
#print axioms QCore.ReturnFamilyUntrackedDerefCast.starred_is_through
#print axioms QCore.ReturnFamilyUntrackedDerefCast.through_mono
#print axioms QCore.ReturnFamilyUntrackedDerefCast.starred_keeps_every_reading_base

-- Bases only add obligations.
#print axioms QCore.ReturnFamilyUntrackedDerefCast.old_bases_kept
#print axioms QCore.ReturnFamilyUntrackedDerefCast.the_fix_only_refuses_more

-- The readings.
#print axioms QCore.ReturnFamilyUntrackedDerefCast.old_misses_the_base
#print axioms QCore.ReturnFamilyUntrackedDerefCast.a_multiplication_is_no_base
#print axioms QCore.ReturnFamilyUntrackedDerefCast.an_undecided_star_through_a_cast_is_a_base

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyUntrackedDerefCast.untracked_vectors_hold
#print axioms QCore.ReturnFamilyUntrackedDerefCast.old_vectors_hold
