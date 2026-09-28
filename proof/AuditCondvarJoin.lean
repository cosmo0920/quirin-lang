import QCore.CondvarJoin

/-! Card **W47-CONDVAR-JOIN-PUBLICATION** (gateway rule 10q, contract
specification §C04.10, the narrowed amendment of constraint 27): condition
variable publication (C4) and join before release (C5).

Five directions are audited.

**Publication through the condvar edge.** A wait releases and reacquires its
mutex; sections run atomically in the order the mutex serialises them. On the
Cycle 14 lifetime model (`QCore.Concurrency.Lifetime`) the wait is the
waiter's unlock and relock around the publisher's section: the
publish/await/join shape is an accepted trace, and a release before the join
is refused. Before
the publication the predicate is unset, so no recheck claims; after it, every
recheck that sees the predicate reads the published snapshot, provided the
publisher alone writes the predicate and the results.

**The waiter (NC8).** Only an observed predicate claims; a spurious or early
wakeup waits again; a timeout that leaves the loop claims nothing.

**Join before release.** A thread acts only while live and only a checked join
ends it; a released object's retaining threads started before the release
never access it afterwards. A timeout, a publication, an unchecked or a failed
join change nothing.

**Kept refusals refuse.** Broadcast, a wait outside a predicate loop, a signal
without the mutex, one condvar with two mutexes and every other kept shape;
a discharged wait is in a predicate loop under its mutex; a discharged signal
holds the paired mutex; the per-unit route never discharges.

**Release decisions.** A discharged release saw a checked join on every path;
one unchecked, failed or absent join refuses.

**Stop paths (card W47-CONDVAR-JOIN-STOP-PATHS).** A null arm is dropped
only with every part of its derivation, and under P-DEREF a field
dereferenced before the test and written after it only by pre-publication
stores is non-null at the test; a store to a reachable object can null it.
Under P-POSIX-JOIN an unchecked first join succeeds when its four cases are
excluded; a detached target, a repeated join and a self join never succeed,
and an underived case does not count. A live descendant that retains the
object blocks its release; a discharged release joined every descendant; a
repeated join, an unresolved or a never-joined descendant refuse the
discharge. The old test's release shapes and the fixed test's stay not
discharged.

The lexical census reading, its binding tables, P-SYNC-INSTANCE,
P-JOIN-IDENTITY, P-RETAIN, P-CHECK and the Rust correspondence are trust
boundary 28's premises, pinned by `crates/qcore-core/tests/unit/condvar_join.rs`
and `crates/quirin-source-analysis/tests/condvar_join_census.rs`. -/

-- Publication through the condvar edge.
#print axioms QCore.CondvarJoin.the_condvar_edge_is_an_accepted_lifetime_trace
#print axioms QCore.CondvarJoin.a_release_before_the_join_is_refused_by_the_lifetime_model
#print axioms QCore.CondvarJoin.no_claim_before_publication
#print axioms QCore.CondvarJoin.after_publication_the_snapshot_holds
#print axioms QCore.CondvarJoin.observed_done_reads_the_published_snapshot

-- The waiter (NC8).
#print axioms QCore.CondvarJoin.a_claim_needs_the_predicate
#print axioms QCore.CondvarJoin.a_wakeup_without_the_predicate_claims_nothing
#print axioms QCore.CondvarJoin.a_spurious_wakeup_waits_again
#print axioms QCore.CondvarJoin.a_timeout_that_leaves_claims_nothing
#print axioms QCore.CondvarJoin.a_run_claims_only_at_an_observed_predicate

-- Join before release.
#print axioms QCore.CondvarJoin.a_timeout_is_not_a_join
#print axioms QCore.CondvarJoin.publication_is_not_a_join
#print axioms QCore.CondvarJoin.an_unchecked_or_failed_join_grants_nothing
#print axioms QCore.CondvarJoin.an_accepted_release_joined_every_retainer
#print axioms QCore.CondvarJoin.ended_stays_ended
#print axioms QCore.CondvarJoin.an_ended_thread_never_acts
#print axioms QCore.CondvarJoin.join_before_release_leaves_no_access_after_it

-- Kept refusals refuse.
#print axioms QCore.CondvarJoin.every_kept_refusal_refuses
#print axioms QCore.CondvarJoin.a_wait_outside_a_predicate_loop_refuses
#print axioms QCore.CondvarJoin.one_condvar_with_two_mutexes_refuses_a_wait
#print axioms QCore.CondvarJoin.a_discharged_wait_rechecks_under_its_mutex
#print axioms QCore.CondvarJoin.a_broadcast_refuses
#print axioms QCore.CondvarJoin.a_signal_without_the_mutex_refuses
#print axioms QCore.CondvarJoin.a_discharged_signal_holds_the_paired_mutex
#print axioms QCore.CondvarJoin.the_per_unit_route_refuses_every_synchronization_call

-- Release decisions and composition.
#print axioms QCore.CondvarJoin.a_discharged_release_saw_a_checked_join_on_every_path
#print axioms QCore.CondvarJoin.publication_timeout_or_an_unchecked_join_refuses
#print axioms QCore.CondvarJoin.only_a_discharge_admits
#print axioms QCore.CondvarJoin.a_join_is_no_less_severe
#print axioms QCore.CondvarJoin.a_joined_discharge_needs_both

-- Stop paths (W47-CONDVAR-JOIN-STOP-PATHS).
#print axioms QCore.CondvarJoin.a_pre_publication_store_leaves_a_reachable_field
#print axioms QCore.CondvarJoin.pre_publication_stores_leave_a_reachable_field
#print axioms QCore.CondvarJoin.the_null_arm_is_infeasible_after_a_dereference
#print axioms QCore.CondvarJoin.a_published_store_can_null_the_field
#print axioms QCore.CondvarJoin.a_dropped_null_arm_had_its_derivation
#print axioms QCore.CondvarJoin.without_a_dominating_dereference_the_null_arm_stays
#print axioms QCore.CondvarJoin.a_store_after_publication_keeps_the_null_arm
#print axioms QCore.CondvarJoin.an_unchecked_first_join_with_every_case_excluded_succeeds
#print axioms QCore.CondvarJoin.a_detached_thread_join_never_succeeds
#print axioms QCore.CondvarJoin.a_repeated_join_never_succeeds
#print axioms QCore.CondvarJoin.a_join_from_the_thread_itself_never_succeeds
#print axioms QCore.CondvarJoin.an_underived_case_does_not_count
#print axioms QCore.CondvarJoin.a_discharged_release_joined_every_descendant
#print axioms QCore.CondvarJoin.a_repeated_join_refuses
#print axioms QCore.CondvarJoin.an_unchecked_first_join_counts_only_with_every_case_excluded
#print axioms QCore.CondvarJoin.an_unresolved_descendant_blocks_the_discharge
#print axioms QCore.CondvarJoin.a_never_joined_descendant_refuses
#print axioms QCore.CondvarJoin.a_live_descendant_blocks_the_release
#print axioms QCore.CondvarJoin.the_fixed_test_release_is_not_discharged
#print axioms QCore.CondvarJoin.the_old_test_stays_not_discharged
#print axioms QCore.CondvarJoin.the_repeated_join_alone_keeps_the_fixed_test_not_discharged
