import QCore.MutexInvariant

/-! T02: the `CheckedMutexInvariantV1` profile. A new namespace beside
`QCore.Concurrency.Lifetime`, which is unchanged and still registered in
`AuditCycle14.lean`. -/

-- Share conservation
#print axioms QCore.Concurrency.MutexInvariant.parked_authority_is_conserved
#print axioms QCore.Concurrency.MutexInvariant.createMutexEffect_preserves_authority
#print axioms QCore.Concurrency.MutexInvariant.destroyMutexEffect_preserves_authority

-- Guard authority is exactly the declared footprint
#print axioms QCore.Concurrency.MutexInvariant.guard_authorizes_only_declared_footprint
#print axioms QCore.Concurrency.MutexInvariant.unrelated_guard_grants_no_authority
#print axioms QCore.Concurrency.MutexInvariant.unguarded_access_to_protected_footprint_is_rejected
#print axioms QCore.Concurrency.MutexInvariant.write_requires_open_invariant

-- Guard affinity
#print axioms QCore.Concurrency.MutexInvariant.only_holder_can_unlock
#print axioms QCore.Concurrency.MutexInvariant.guard_is_not_transferable

-- The invariant holds at every exit
#print axioms QCore.Concurrency.MutexInvariant.unlock_requires_closed_invariant
#print axioms QCore.Concurrency.MutexInvariant.error_exit_requires_closed_invariant

-- Guard-derived borrows and guard-scoped facts
#print axioms QCore.Concurrency.MutexInvariant.guard_borrows_die_at_unlock
#print axioms QCore.Concurrency.MutexInvariant.release_removes_the_guard
#print axioms QCore.Concurrency.MutexInvariant.use_fact_requires_a_live_guard
#print axioms QCore.Concurrency.MutexInvariant.facts_do_not_survive_unlock

-- Fixed shape and a single protector
#print axioms QCore.Concurrency.MutexInvariant.footprint_has_at_most_one_protector
#print axioms QCore.Concurrency.MutexInvariant.protected_shape_is_fixed
#print axioms QCore.Concurrency.MutexInvariant.unsupported_synchronization_is_refused
#print axioms QCore.Concurrency.MutexInvariant.critical_section_effects_preserve_declared_shapes

-- Mutex generations
#print axioms QCore.Concurrency.MutexInvariant.mutex_destruction_requires_quiescence
#print axioms QCore.Concurrency.MutexInvariant.accepted_destruction_kills_the_generation
#print axioms QCore.Concurrency.MutexInvariant.destroyed_mutex_generation_is_never_relocked
#print axioms QCore.Concurrency.MutexInvariant.destroyed_mutex_generation_rejects_repeat_destruction
#print axioms QCore.Concurrency.MutexInvariant.busy_mutex_rejects_destruction
#print axioms QCore.Concurrency.MutexInvariant.waited_mutex_rejects_destruction
#print axioms QCore.Concurrency.MutexInvariant.refused_mutex_destruction_leaves_generation_live

-- Derived synchronization witnesses
#print axioms QCore.Concurrency.MutexInvariant.synchronization_witness_must_be_derivable
#print axioms QCore.Concurrency.MutexInvariant.forged_witness_is_refused
#print axioms QCore.Concurrency.MutexInvariant.same_mutex_generation_orders_release_acquire
#print axioms QCore.Concurrency.MutexInvariant.distinct_mutexes_do_not_order

-- The discharge claim
#print axioms QCore.Concurrency.MutexInvariant.checked_certificate_omits_trusted_guard_assumptions

/-! T03: the `source.mutex.guard.v1` lowering onto this profile. Source
extraction remains a premise. Card T03-F added the resource-destruction
constructor, so scope exit is now covered in both halves; see the T03-F block
at the end of this file. -/

-- The lowering is total and loses nothing
#print axioms QCore.Concurrency.MutexInvariant.lowerSection_length
#print axioms QCore.Concurrency.MutexInvariant.lowerSection_brackets
#print axioms QCore.Concurrency.MutexInvariant.lowerSection_head

-- A lexical guard cannot escape or travel to a child task
#print axioms QCore.Concurrency.MutexInvariant.lowerSection_emits_no_task_event
#print axioms QCore.Concurrency.MutexInvariant.lowerScope_emits_no_task_event
#print axioms QCore.Concurrency.MutexInvariant.lowerSection_emits_no_refused_construct

-- One authority per critical section, and every generation is destroyed
#print axioms QCore.Concurrency.MutexInvariant.lowerSection_mints_exactly_its_own_guard
#print axioms QCore.Concurrency.MutexInvariant.lowerScope_mints_one_guard_per_section
#print axioms QCore.Concurrency.MutexInvariant.lowerScope_destroys_every_declared_generation

-- What an accepted lowering forces at the section exit
#print axioms QCore.Concurrency.MutexInvariant.run_append
#print axioms QCore.Concurrency.MutexInvariant.typed_error_exit_obeys_the_normal_release_rule
#print axioms QCore.Concurrency.MutexInvariant.lowered_exit_refuses_an_open_invariant
#print axioms QCore.Concurrency.MutexInvariant.lowered_exit_refuses_a_live_borrow
#print axioms QCore.Concurrency.MutexInvariant.accepted_section_leaves_no_live_guard

/-! T03-F: the guard-profile completions — resource destruction, the checked
invariant witness and the bounded task composition. Source extraction is still
a premise, and this model still has no clocks, so the Rust checker's
happens-before conditions on destruction are not among these. -/

-- Resource destruction: the second half of scope exit
#print axioms QCore.Concurrency.MutexInvariant.destroyEffect_preserves_authority
#print axioms QCore.Concurrency.MutexInvariant.destroy_requires_custody
#print axioms QCore.Concurrency.MutexInvariant.protected_resource_rejects_destruction
#print axioms QCore.Concurrency.MutexInvariant.foreign_share_rejects_destruction
#print axioms QCore.Concurrency.MutexInvariant.dead_resource_rejects_repeat_destruction
#print axioms QCore.Concurrency.MutexInvariant.destroyed_resource_rejects_access
#print axioms QCore.Concurrency.MutexInvariant.accepted_destruction_kills_the_resource
#print axioms QCore.Concurrency.MutexInvariant.refused_destruction_stops_the_trace
#print axioms QCore.Concurrency.MutexInvariant.lowerScope_destroys_every_declared_resource
#print axioms QCore.Concurrency.MutexInvariant.scopeExit_destroys_generations_before_resources

-- The checked `NoWriteOccurred` witness, and the premise it drops
#print axioms QCore.Concurrency.MutexInvariant.checked_witness_requires_no_write
#print axioms QCore.Concurrency.MutexInvariant.checked_witness_requires_an_open_invariant
#print axioms QCore.Concurrency.MutexInvariant.checked_witness_ignores_the_declared_predicate
#print axioms QCore.Concurrency.MutexInvariant.checked_and_declared_close_agree_on_effect
#print axioms QCore.Concurrency.MutexInvariant.checked_close_carries_no_declared_predicate_premise
#print axioms QCore.Concurrency.MutexInvariant.a_section_without_a_declared_close_emits_none

-- Bounded task composition inside the guard profile
#print axioms QCore.Concurrency.MutexInvariant.join_while_holding_a_guard_is_refused
#print axioms QCore.Concurrency.MutexInvariant.join_of_a_guard_holding_child_is_refused
#print axioms QCore.Concurrency.MutexInvariant.a_shareless_task_cannot_access_unguarded

-- The guard-local join (card T03-GUARD-JOIN). What an admitted join does, and
-- does not, disturb.
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_effect_is_the_ordinary_join
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_preserves_authority
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_preserves_declared_shapes
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_drops_nothing_the_invariant_needs
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_completes_exactly_the_child
#print axioms QCore.Concurrency.MutexInvariant.accepted_guard_local_join_discharges_its_obligations
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_keeps_the_creator_in_its_only_section

-- Everything outside the rule keeps refusing.
#print axioms QCore.Concurrency.MutexInvariant.the_quiet_subtree_contains_the_child
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_requires_a_quiet_subtree
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_refuses_a_keyed_descendant
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_requires_creation_in_the_same_section
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_requires_exactly_one_held_section
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_requires_the_joining_task_to_hold_the_section
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_requires_a_closed_invariant
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_requires_no_live_borrow
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_refuses_a_waiting_subtree
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_refuses_a_sharing_child
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_refuses_a_custodian_child
#print axioms QCore.Concurrency.MutexInvariant.guard_local_join_refuses_a_guard_holding_child

-- The ledger's own vocabulary: what marks a task, and what does not.
#print axioms QCore.Concurrency.MutexInvariant.spawn_records_the_creators_open_sections
#print axioms QCore.Concurrency.MutexInvariant.a_lock_is_recorded_as_mutex_keyed
#print axioms QCore.Concurrency.MutexInvariant.a_registered_waiter_is_recorded_as_mutex_keyed
#print axioms QCore.Concurrency.MutexInvariant.guarded_and_unguarded_access_are_recorded_differently
#print axioms QCore.Concurrency.MutexInvariant.an_end_borrow_is_recorded_as_mutex_keyed

-- Card CT33-RANKS: the opt-in lock-rank discipline. A rank is a declared
-- position in an order, so nothing below is a deadlock-freedom claim; the
-- soundness result is that a rank-respecting run's would-block relation has no
-- cycle.
#print axioms QCore.Concurrency.MutexInvariant.admitted_edges_increase_rank
#print axioms QCore.Concurrency.MutexInvariant.a_non_increasing_acquisition_is_refused
#print axioms QCore.Concurrency.MutexInvariant.equal_ranks_are_never_nested
#print axioms QCore.Concurrency.MutexInvariant.an_unranked_acquisition_is_refused
#print axioms QCore.Concurrency.MutexInvariant.an_unranked_held_generation_refuses_every_acquisition
#print axioms QCore.Concurrency.MutexInvariant.no_generation_is_acquired_while_held
#print axioms QCore.Concurrency.MutexInvariant.runRanked_edges_increase_rank
#print axioms QCore.Concurrency.MutexInvariant.path_rank_strictly_increases
#print axioms QCore.Concurrency.MutexInvariant.a_rank_respecting_run_admits_no_cycle

-- The discipline is a layer over the unchanged model: it only ever refuses.
#print axioms QCore.Concurrency.MutexInvariant.stepRanked_agrees_with_step_off_lock
#print axioms QCore.Concurrency.MutexInvariant.stepRanked_refines_step
#print axioms QCore.Concurrency.MutexInvariant.runRanked_refines_run
