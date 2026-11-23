import QCore.ReturnFamilyStarCast

/-! Card **RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST**: family S of
`legacy_return_family` reads a single `*` or `&` after a `)` its declaration
reader does not resolve as a type through rule 74f's one shared detector
`ampersand_runs::may_close_a_cast`, and keeps both readings of an undecided
group (gateway rule 74h, `QCore.CastBeforeAmpersand` not restated here).

Three tests are audited against every reading C may take of an undecided
operator: the `&` refusal, the starred base of an untracked root, and a
may-bind root's dereference count. Each keeps what any reading raises, and
each refuses wherever the base refused.

Each `)`'s class (the Rust readers' output, bound to `familyVectors`) is trust
boundary 28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_star_cast.rs` and by
`crates/qcore-c-gateway/tests/return_family_star_cast.rs`. -/

-- The readings.
#print axioms QCore.ReturnFamilyStarCast.operand_below
#print axioms QCore.ReturnFamilyStarCast.cast_above
#print axioms QCore.ReturnFamilyStarCast.the_old_reading_is_the_operand_reading

-- The `&` refusal.
#print axioms QCore.ReturnFamilyStarCast.address_keeps_every_reading_refusal
#print axioms QCore.ReturnFamilyStarCast.address_old_refusal_kept

-- The starred base.
#print axioms QCore.ReturnFamilyStarCast.starred_keeps_every_reading_base
#print axioms QCore.ReturnFamilyStarCast.old_starred_kept
#print axioms QCore.ReturnFamilyStarCast.old_reading_misses_the_base

-- A may-bind root's count.
#print axioms QCore.ReturnFamilyStarCast.takeWhile_length_mono
#print axioms QCore.ReturnFamilyStarCast.count_mono
#print axioms QCore.ReturnFamilyStarCast.starred_admission_needs_every_reading
#print axioms QCore.ReturnFamilyStarCast.member_admission_needs_every_reading
#print axioms QCore.ReturnFamilyStarCast.old_count_is_the_operand_count
#print axioms QCore.ReturnFamilyStarCast.the_fix_only_refuses_more_starred
#print axioms QCore.ReturnFamilyStarCast.the_fix_only_refuses_more_member
#print axioms QCore.ReturnFamilyStarCast.the_cast_reading_alone_would_admit
#print axioms QCore.ReturnFamilyStarCast.a_resolved_cast_is_unchanged
#print axioms QCore.ReturnFamilyStarCast.an_undecided_second_star_refuses

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyStarCast.family_vectors_hold
