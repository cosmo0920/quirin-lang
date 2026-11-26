import QCore.NullJoinDischarge

/-! Card **UNTRACKED-BASE-NULL-JOIN-DISCHARGE**: a binding some reaching path
held as the null constant is never rule 14n's premised alias (gateway rule
83). The pointer join made the null constant joined with an unbound copy or
a pointer read `unknown` and possibly null, rule 14n's premised shape, so the
base obligation discharged under a premise the function's own null constant
refutes on one path.

The flag `nullOnPath` is joined by disjunction with each side's definite
null and cleared by a rebinding; the premise refuses a flagged name. Every
join of paths one of which holds the null constant refuses; with no null
constant on any path the verdict is the base's; a guard-proved non-null still
discharges.

The lowering of C statements to path bindings is trust boundary 28's; the
Rust join is bound to `joinVectors` by
`crates/qcore-c-gateway/tests/unit/legacy_null_join.rs`, and the C spellings
by `crates/qcore-c-gateway/tests/untracked_base_null_join.rs`. -/

-- Soundness.
#print axioms QCore.NullJoinDischarge.a_null_on_some_path_is_never_discharged
#print axioms QCore.NullJoinDischarge.a_null_on_some_path_is_never_premised

-- Only refuses more, and nothing else moves.
#print axioms QCore.NullJoinDischarge.the_fix_only_refuses_more
#print axioms QCore.NullJoinDischarge.no_null_keeps_the_verdict

-- Guards and rebinding.
#print axioms QCore.NullJoinDischarge.a_guard_proved_non_null_is_discharged
#print axioms QCore.NullJoinDischarge.an_overwritten_null_keeps_the_premise
#print axioms QCore.NullJoinDischarge.an_unbound_base_discharges

-- The gap.
#print axioms QCore.NullJoinDischarge.old_premises_a_joined_null

-- The vectors bound to the Rust join.
#print axioms QCore.NullJoinDischarge.join_vectors_hold
