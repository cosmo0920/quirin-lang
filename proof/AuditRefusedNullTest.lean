import QCore.RefusedNullTest

/-! Card **MONKEY-VHOST-PROPERTY-CUSTODY**: a refused null test never
establishes the non-null-ness a leak accusation rests on (gateway rule 12e).

Three directions are audited.

**The decision table.** On the null edge of a refused null test exactly a
site the tested binding may denote, nullable and possibly live, is withheld;
an undenoted or non-nullable site keeps its accusation. The Rust table is this
one (`null_edge_vectors_hold`).

**The shape.** Monkey's `mk_http_error_page` null edge was accused before the
rule (non-vacuity) and is `Unknown` under it, never `Safe`.

**Only a withdrawal.** A decided test is unchanged; the rule never adds an
accusation; a refused test establishes no leak on its null edge.

Faithful reading of the flow's denotation, nullable set and status bits, and
the Rust correspondence, are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/unit/refused_null_test.rs` and
`crates/qcore-c-gateway/tests/monkey_vhost_property_custody.rs`. -/

-- The decision table.
#print axioms QCore.RefusedNullTest.withhold_iff
#print axioms QCore.RefusedNullTest.an_undenoted_site_is_kept
#print axioms QCore.RefusedNullTest.a_non_nullable_site_is_kept
#print axioms QCore.RefusedNullTest.null_edge_vectors_hold

-- The shape.
#print axioms QCore.RefusedNullTest.the_old_reading_accused_the_null_edge
#print axioms QCore.RefusedNullTest.the_null_edge_is_unknown
#print axioms QCore.RefusedNullTest.the_null_edge_is_never_safe

-- Only a withdrawal.
#print axioms QCore.RefusedNullTest.a_decided_test_is_unchanged
#print axioms QCore.RefusedNullTest.the_rule_never_adds_an_accusation
#print axioms QCore.RefusedNullTest.a_refused_test_establishes_no_leak
