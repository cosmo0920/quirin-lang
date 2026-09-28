import QCore.LoopOwner

/-! Card **W47-LOOP-OWNER-CONTRACTS** (gateway rule 10p, contract
specification §C04.9): the loop-owner obligation over derived thread
identity.

Four directions are audited.

**Derived ownership.** An edge keeps the thread; a loop dispatch fixes the
mode and runs the callback on the dispatcher's thread, which is the loop's
owner; an unresolved dispatcher has no owner; an infeasible threaded-flag
branch is pruned.

**Discharge only on the owner.** A discharged site has no unresolved path, a
derived caller, and every context on its mode's single owner; one resolved
non-owner context refuses; the per-unit route derives no thread and refuses
every obligated call; only a discharge admits.

**No concurrent mutator.** A discharge derives the Cycle 18 single-dispatcher
premise (`EventDriven.Execution.sameBaseSingleDispatcher`) for this route
instead of assuming it, leaves no conflicting pair of accesses, and a foreign
access conflicts with the dispatcher's own walk (the reactor's L05).

**C2 and C3.** An enqueue never discharges "applied"; an unfixed mode claims
nothing; the paired backend collectors' state is a boundary claim only on the
owner with both results succeeded, and never for a resume without active
ingestion.

**Hop C** (card W47-LOOP-OWNER-COROUTINE-HOP). A coroutine body runs on its
derived resumers' threads; one resumed only on the owner runs on the owner;
one non-owner resumer refuses; an undetermined resumer, a body's continuation
after a yield included, never discharges. An input whose derived descriptor
flags run no coroutine is not dispatched through the hop; an underived flag
keeps it. The configured reading keeps the arm any agreeing build takes, drops
a decided-false arm, and reads nothing after a decided-true arm.

**Typed dispatch** (card W47-LOOP-OWNER-TYPED-DISPATCH). Type-keyed
dispatch is a sub-relation of field-keyed dispatch, equal to it where no typed
fact exists; a call through record `T` reaches no registrant stored only in
another record; every removed edge is cross-type or belongs to an unmatched
dispatch, whose registrants are unresolved and never discharged. A runtime
plugin copy's instance drops the coroutine route only when both the creating
plugin's flag and the copy's derive none; a copy written with the coroutine
flag, or an underived part, keeps it.

**The list-head instance** (card W47-LOOP-OWNER-INSTANCE-FLOW). From a fresh
context's empty input list, a path that creates exactly once and mutates the
list in no unmodelled way leaves exactly the created instance, so a derived
head is the one creation on every path; a second creation, an unmodelled
mutation or no creation leaves the head underived and keeps the coroutine
route. An escaping holder keeps the derivation only when what it reaches
writes neither the instance's flag nor which instance runs the callback; the
route is dropped only through one non-coroutine creation, such an escape and a
non-coroutine copy. The old test's resolved test-thread context is never
discharged.

The lexical census reading, the hop tables, P-INSTANCE and the Rust
correspondence are trust boundary 28's premises, pinned by
`crates/qcore-core/tests/unit/loop_owner.rs`,
`crates/quirin-source-analysis/tests/loop_owner_census.rs` and
`crates/quirin-source-analysis/tests/loop_owner_coroutine.rs` and
`crates/quirin-source-analysis/tests/loop_owner_typed_dispatch.rs` and
`crates/quirin-source-analysis/tests/loop_owner_instance_flow.rs`. -/

-- Derived ownership.
#print axioms QCore.LoopOwner.a_step_keeps_the_thread
#print axioms QCore.LoopOwner.a_dispatched_callback_runs_on_the_owner
#print axioms QCore.LoopOwner.a_worker_callback_runs_on_the_owner
#print axioms QCore.LoopOwner.an_unresolved_dispatcher_has_no_owner
#print axioms QCore.LoopOwner.an_infeasible_branch_is_pruned

-- Discharge only on the owner.
#print axioms QCore.LoopOwner.unresolved_is_never_discharged
#print axioms QCore.LoopOwner.no_derived_caller_is_never_discharged
#print axioms QCore.LoopOwner.context_discharged_on_owner
#print axioms QCore.LoopOwner.discharged_means_every_context_runs_on_its_owner
#print axioms QCore.LoopOwner.a_non_owner_context_refuses
#print axioms QCore.LoopOwner.the_per_unit_route_refuses_every_obligated_call
#print axioms QCore.LoopOwner.only_a_discharge_admits

-- No concurrent mutator.
#print axioms QCore.LoopOwner.discharged_accesses_run_on_the_owner
#print axioms QCore.LoopOwner.the_owner_route_derives_the_dispatcher_premise
#print axioms QCore.LoopOwner.a_discharged_loop_has_no_concurrent_mutator
#print axioms QCore.LoopOwner.a_foreign_access_conflicts_with_the_dispatcher

-- C2 and C3.
#print axioms QCore.LoopOwner.an_enqueue_never_applies
#print axioms QCore.LoopOwner.a_threaded_control_only_enqueues
#print axioms QCore.LoopOwner.an_unfixed_mode_claims_nothing
#print axioms QCore.LoopOwner.a_boundary_claim_needs_both_results_and_the_owner
#print axioms QCore.LoopOwner.a_discarded_result_gives_no_conclusion
#print axioms QCore.LoopOwner.a_resume_without_ingestion_is_refused

-- Hop C: a coroutine runs on the thread that resumes it.
#print axioms QCore.LoopOwner.a_coroutine_runs_on_its_resumers
#print axioms QCore.LoopOwner.a_coroutine_resumed_only_on_the_owner_runs_on_the_owner
#print axioms QCore.LoopOwner.a_non_owner_resumer_refuses
#print axioms QCore.LoopOwner.an_undetermined_resumer_never_discharges
#print axioms QCore.LoopOwner.a_body_that_may_yield_is_never_discharged
#print axioms QCore.LoopOwner.the_dispatcher_resumes_on_its_own_thread
#print axioms QCore.LoopOwner.an_input_without_a_coroutine_is_not_dispatched_by_the_hop
#print axioms QCore.LoopOwner.an_underived_flag_keeps_the_hop

-- The configured reading of a conditional group.
#print axioms QCore.LoopOwner.taken_within_kept
#print axioms QCore.LoopOwner.the_taken_arm_is_read
#print axioms QCore.LoopOwner.a_decided_false_arm_is_not_read
#print axioms QCore.LoopOwner.nothing_after_a_decided_arm_is_read

-- Typed dispatch: a sub-relation that only removes cross-type edges.
#print axioms QCore.LoopOwner.type_keyed_is_a_sub_relation
#print axioms QCore.LoopOwner.an_untyped_dispatch_is_field_keyed
#print axioms QCore.LoopOwner.a_call_through_a_record_reaches_only_its_registrants
#print axioms QCore.LoopOwner.a_removed_edge_is_cross_type_or_refused
#print axioms QCore.LoopOwner.an_unmatched_dispatch_never_discharges

-- Step B: the instance flag of a runtime plugin copy.
#print axioms QCore.LoopOwner.no_coroutine_needs_both
#print axioms QCore.LoopOwner.a_coroutine_copy_keeps_the_route
#print axioms QCore.LoopOwner.an_underived_part_keeps_the_route
#print axioms QCore.LoopOwner.the_route_is_dropped_only_when_both_derive_none

-- The list-head instance, the holder escape and the old test's path
-- (card W47-LOOP-OWNER-INSTANCE-FLOW).
#print axioms QCore.LoopOwner.a_derived_head_is_the_one_creation
#print axioms QCore.LoopOwner.every_path_reads_the_one_creation
#print axioms QCore.LoopOwner.a_second_creator_keeps_the_route
#print axioms QCore.LoopOwner.an_unmodelled_mutation_keeps_the_route
#print axioms QCore.LoopOwner.a_path_without_a_creation_derives_nothing
#print axioms QCore.LoopOwner.an_underived_head_keeps_the_route
#print axioms QCore.LoopOwner.an_escape_that_writes_neither_keeps_the_derivation
#print axioms QCore.LoopOwner.an_escape_that_writes_the_flag_or_the_instance_keeps_the_route
#print axioms QCore.LoopOwner.the_route_is_dropped_only_through_one_non_coroutine_creation
#print axioms QCore.LoopOwner.the_old_test_path_is_never_discharged
