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
