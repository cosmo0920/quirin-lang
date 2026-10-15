import QCore.MemberPrepassRoutes

/-! Audited companion to MEMBER-PREPASS-OTHER-ROUTES (gateway rule 12n, amended in place): the member-facts pre-pass on the current and no-build routes, the checkpoint that carries the run's facts, and the scope file re-checked at run time. An exclusion holds only while its evidence stands and no evidence file names a final token; a dropped exclusion reads as its unit, so a stale exclusion never closes a member, and exclusions that hold change nothing; a composer whose checkpoints carry no run facts refuses a member-invoking run, a composition over the carried closure that succeeds publishes exactly the run's fold, and a run whose units decided over that closure always composes. -/

#print axioms QCore.MemberPrepassRoutes.a_named_token_drops_the_exclusion
#print axioms QCore.MemberPrepassRoutes.failed_evidence_drops_the_exclusion
#print axioms QCore.MemberPrepassRoutes.a_dropped_exclusion_reads_as_the_unit
#print axioms QCore.MemberPrepassRoutes.a_stale_exclusion_never_closes_a_member
#print axioms QCore.MemberPrepassRoutes.held_exclusions_keep_the_run_set
#print axioms QCore.MemberPrepassRoutes.a_closed_closure_keeps_itself
#print axioms QCore.MemberPrepassRoutes.a_checkpoint_without_facts_refuses
#print axioms QCore.MemberPrepassRoutes.the_composed_decision_is_the_runs
#print axioms QCore.MemberPrepassRoutes.a_run_decided_over_its_facts_composes
