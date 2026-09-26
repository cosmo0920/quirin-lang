import QCore.CheckedSpecification

/-! Card **W45-FB-BACKLOG-CUSTODY**: checked specifications (contract spec
§C06.2, `crates/qcore-c-gateway/src/legacy_checked_spec.rs`).

Three directions are audited.

**Soundness of the descent.** Everything left in force is established under
exactly what is left in force, settling only withdraws, a specification with
no body (or in a recursive component) is never in force, and the descent
reaches a fixed point within the round bound the Rust uses.

**Never trusted.** A defining function's row is `Safe` only when its
specification is in force, and so only when its body established it.

**Non-vacuity.** A failing specification is withdrawn; an established one
stays.

That the check is the W4 gate over the unit's own body, and that call sites
apply exactly the settled environment, are Rust boundaries pinned by
`crates/qcore-c-gateway/tests/intrusive_list_iteration.rs`. -/

-- Soundness of the descent.
#print axioms QCore.CheckedSpecification.settle_sound
#print axioms QCore.CheckedSpecification.settle_subset
#print axioms QCore.CheckedSpecification.never_without_a_body
#print axioms QCore.CheckedSpecification.settle_is_a_fixed_point_within_its_bound

-- Never trusted.
#print axioms QCore.CheckedSpecification.a_row_is_safe_only_if_established

-- Non-vacuity.
#print axioms QCore.CheckedSpecification.a_failing_specification_is_withdrawn
#print axioms QCore.CheckedSpecification.an_established_specification_stays
