import QCore.ReturnFamilyStarOutside

/-! Card **RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES**: a starred may-bind
root of family S (`legacy_return_family`) is refused when a parenthesised
group enclosing it may be dereferenced, by a `*` before it or a `[` or `->`
after it (gateway rule 74s, `QCore.ReturnFamilyDerefCast` not restated here).

The tip only refuses more than the base, an admitted root is one dereference
counting every enclosing group under every reading C may take, `* ( * q + 1 )`
counts two and refuses, and `( * q )`, `* q + 1` and `( T ) * q` keep their
admission.

Each item, enclosing group and its classes (the Rust reader's output, bound to
`outsideVectors`) are trust boundary 28's. They are pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_star_outside.rs` and by
`crates/qcore-c-gateway/tests/return_family_star_outside_parentheses.rs`. -/

-- An enclosing group.
#print axioms QCore.ReturnFamilyStarOutside.deref_below_may

-- The admission.
#print axioms QCore.ReturnFamilyStarOutside.the_fix_only_refuses_more
#print axioms QCore.ReturnFamilyStarOutside.admitted_has_no_outside_dereference
#print axioms QCore.ReturnFamilyStarOutside.admitted_is_one_dereference
#print axioms QCore.ReturnFamilyStarOutside.old_admits_a_second_dereference
#print axioms QCore.ReturnFamilyStarOutside.a_parenthesised_dereference_is_unchanged

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyStarOutside.outside_vectors_hold
#print axioms QCore.ReturnFamilyStarOutside.old_vectors_hold
