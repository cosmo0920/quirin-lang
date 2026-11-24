import QCore.ReturnFamilyDerefCast

/-! Card **RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST**: a starred may-bind
root of family S (`legacy_return_family`) counts its `*`s both up to the
nearest cast group and through every cast group, and is admitted only when
both counts are one under both of rule 74h's readings (gateway rule 74m,
`QCore.ReturnFamilyStarCast` not restated here).

The tip only refuses more than the base, an admitted root is one dereference
under every reading C may take, `* ( T * ) * q` refuses, and `* ( T * ) q` and
`( T ) * q` keep their base readings.

Each item's class (the Rust reader's output, bound to `derefVectors`) is trust
boundary 28's. It is pinned by
`crates/qcore-c-gateway/tests/unit/legacy_return_family_deref_cast.rs` and by
`crates/qcore-c-gateway/tests/return_family_deref_resolved_cast.rs`. -/

-- The two counts.
#print axioms QCore.ReturnFamilyDerefCast.inner_le_through
#print axioms QCore.ReturnFamilyDerefCast.inner_mono
#print axioms QCore.ReturnFamilyDerefCast.through_mono

-- The admission.
#print axioms QCore.ReturnFamilyDerefCast.the_fix_only_refuses_more
#print axioms QCore.ReturnFamilyDerefCast.admitted_is_one_dereference
#print axioms QCore.ReturnFamilyDerefCast.old_admits_a_second_dereference
#print axioms QCore.ReturnFamilyDerefCast.the_converted_value_keeps_its_refusal
#print axioms QCore.ReturnFamilyDerefCast.a_cast_of_one_dereference_is_unchanged

-- The token reader's vectors.
#print axioms QCore.ReturnFamilyDerefCast.deref_vectors_hold
#print axioms QCore.ReturnFamilyDerefCast.old_vectors_hold
