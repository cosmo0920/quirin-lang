import QCore.CrossUnitReplayBudget

/-! Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended): a caller holds only
the cross-unit homes it may reach and none above the per-unit replay budget.
The import work is linear in the units, holding fewer homes never supplies a
summary, an exhausted budget reads `Unknown`, and within the budget every
name the caller's analysis can consult reads what the full table supplies. -/

#print axioms QCore.CrossUnitReplayBudget.held_within_budget
#print axioms QCore.CrossUnitReplayBudget.replay_work_bound
#print axioms QCore.CrossUnitReplayBudget.held_is_a_run_home
#print axioms QCore.CrossUnitReplayBudget.exhausted_holds_none
#print axioms QCore.CrossUnitReplayBudget.within_budget_holds_reachable
#print axioms QCore.CrossUnitReplayBudget.restricted_lookup_is_full_or_absent
#print axioms QCore.CrossUnitReplayBudget.exhausted_import_call_is_unknown
#print axioms QCore.CrossUnitReplayBudget.consulted_lookup_unchanged
#print axioms QCore.CrossUnitReplayBudget.scope_vectors_hold
