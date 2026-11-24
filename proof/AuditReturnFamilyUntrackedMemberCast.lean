import QCore.ReturnFamilyUntrackedMemberCast

/-! Card **RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST**: an untracked root of
family S (`legacy_return_family`) that `->` or `[` accesses through closing
parentheses and cast groups is a base of the read node, and a `->` root
carries its C02-P-ALIAS reading (gateway rule 74t).

Every base and `->` root the base read is kept. A base or a `->` root only
adds an obligation, so nothing admits more than at the base. The operand
reading of a type group accesses no root that the cast reading misses. A word
of a cast's type name is never made a root. An operand before the root stops
the walk.

Each item's class (the Rust reader's output, bound to `memberVectors`) is
trust boundary 28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_member_cast.rs`
and by `crates/qcore-c-gateway/tests/return_family_untracked_member_cast.rs`. -/

-- The two tests.
#print axioms QCore.ReturnFamilyUntrackedMemberCast.old_member_kept
#print axioms QCore.ReturnFamilyUntrackedMemberCast.old_arrow_kept

-- Every reading of a type group keeps the base.
#print axioms QCore.ReturnFamilyUntrackedMemberCast.closeRun_le
#print axioms QCore.ReturnFamilyUntrackedMemberCast.closeRun_stable
#print axioms QCore.ReturnFamilyUntrackedMemberCast.opened_mono
#print axioms QCore.ReturnFamilyUntrackedMemberCast.accessed_mono
#print axioms QCore.ReturnFamilyUntrackedMemberCast.every_group_reading_keeps_the_base

-- What the new test never reads.
#print axioms QCore.ReturnFamilyUntrackedMemberCast.a_type_word_is_no_root
#print axioms QCore.ReturnFamilyUntrackedMemberCast.an_operand_before_the_root_is_no_access

-- Bases only add obligations.
#print axioms QCore.ReturnFamilyUntrackedMemberCast.bases_kept
#print axioms QCore.ReturnFamilyUntrackedMemberCast.old_bases_kept
#print axioms QCore.ReturnFamilyUntrackedMemberCast.the_fix_only_refuses_more

-- The readings.
#print axioms QCore.ReturnFamilyUntrackedMemberCast.old_misses_the_base
#print axioms QCore.ReturnFamilyUntrackedMemberCast.a_converted_operand_is_no_base

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyUntrackedMemberCast.member_vectors_hold
#print axioms QCore.ReturnFamilyUntrackedMemberCast.old_vectors_hold
