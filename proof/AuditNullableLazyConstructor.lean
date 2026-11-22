import QCore.NullableLazyConstructor

/-! Card **NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP**: a lazy constructor decides its
name; a possibly-null return under a non-null result mode is refused as that
(gateway rule 74d).

Four directions are audited.

**The lazy guard.** After `if (!p) p = ctor();` `p` is non-null; the
constructing arm owes the fresh site; the other arm keeps the prior value and
its obligations.

**The abstract fact.** It is sound, joins by conjunction, and the lazy step
establishes it.

**The loop head.** It holds only when every arriving edge has it; the NULL
entry edge makes the zero-iteration exit null. A reassignment from a
possibly-null value or a refused write kills it.

**The renamed leaf.** The refusing return marks its site and claims no leak;
a refusal is never below `Unknown`; the old pair and the new leaf fold to the
same verdict; a nullable result mode accepts the return.

Faithful states in the Rust, and the correspondence with it, are trust
boundary 28's, pinned by
`crates/qcore-c-gateway/tests/nullable_lazy_constructor.rs`. -/

-- The lazy guard.
#print axioms QCore.NullableLazyConstructor.lazy_is_non_null
#print axioms QCore.NullableLazyConstructor.the_constructing_arm_owes_the_fresh_site
#print axioms QCore.NullableLazyConstructor.the_other_arm_keeps_the_prior_value

-- The abstract fact.
#print axioms QCore.NullableLazyConstructor.the_fact_is_sound
#print axioms QCore.NullableLazyConstructor.the_join_is_sound
#print axioms QCore.NullableLazyConstructor.the_lazy_step_establishes_it

-- The loop head and what kills the fact.
#print axioms QCore.NullableLazyConstructor.the_back_edges_join
#print axioms QCore.NullableLazyConstructor.a_loop_head_needs_every_edge
#print axioms QCore.NullableLazyConstructor.the_zero_iteration_path_is_null
#print axioms QCore.NullableLazyConstructor.every_back_edge_after_the_guard_has_it
#print axioms QCore.NullableLazyConstructor.a_reassignment_kills_it
#print axioms QCore.NullableLazyConstructor.a_refused_write_kills_it

-- The renamed leaf.
#print axioms QCore.NullableLazyConstructor.a_non_null_mode_refuses_it
#print axioms QCore.NullableLazyConstructor.a_nullable_result_mode_accepts_it
#print axioms QCore.NullableLazyConstructor.a_refused_return_claims_nothing
#print axioms QCore.NullableLazyConstructor.the_refusal_is_never_below_unknown
#print axioms QCore.NullableLazyConstructor.the_renaming_keeps_the_verdict
