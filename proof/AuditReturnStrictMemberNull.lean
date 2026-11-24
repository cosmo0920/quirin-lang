import QCore.ReturnStrictMemberNull

/-! Card **RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT**: card B01's opaque
member read in `return` position (`return_expression_instruction`, asked
before family S) lowers a `ROOT -> FIELD` read to family S's read node, with
the root as its only base, instead of an obligation-free `OpaqueValue`
(gateway rule 74v).

B01 admitted every expression it read, so nothing admits more than at the
base. A value read is unchanged. A root read is admitted exactly when family
S's base obligation discharges. Every other spelling defers to the later
routes.

Each token's class (the Rust reader's output, bound to `readVectors`) is
trust boundary 28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_member_read.rs` and by
`crates/qcore-c-gateway/tests/return_strict_member_null.rs`. -/

-- Only refuses more.
#print axioms QCore.ReturnStrictMemberNull.the_fix_only_refuses_more
#print axioms QCore.ReturnStrictMemberNull.outside_b01_unchanged

-- The three readings.
#print axioms QCore.ReturnStrictMemberNull.a_value_read_is_unchanged
#print axioms QCore.ReturnStrictMemberNull.a_root_read_needs_its_base
#print axioms QCore.ReturnStrictMemberNull.a_declined_read_defers

-- Family S's verdict.
#print axioms QCore.ReturnStrictMemberNull.the_bare_read_gets_family_s_verdict
#print axioms QCore.ReturnStrictMemberNull.a_discharged_root_keeps_its_admission

-- What the reading names.
#print axioms QCore.ReturnStrictMemberNull.the_root_is_the_dereferenced_name
#print axioms QCore.ReturnStrictMemberNull.a_value_read_dereferences_nothing

-- The gap.
#print axioms QCore.ReturnStrictMemberNull.old_misses_the_base

-- The token reader's vectors.
#print axioms QCore.ReturnStrictMemberNull.read_vectors_hold
