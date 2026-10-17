import QCore.OuterContract

/-! Audited companion to OUTER-CONTRACT (gateway rule 31: the runtime projection of outer contracts). -/

#print axioms QCore.OuterContract.Verdict.join_comm
#print axioms QCore.OuterContract.Verdict.join_assoc
#print axioms QCore.OuterContract.static_projection_is_identity
#print axioms QCore.OuterContract.runtime_never_promotes
#print axioms QCore.OuterContract.a_row_without_closed_world_facts_is_unchanged
#print axioms QCore.OuterContract.a_covered_fact_is_pending_with_its_contract
#print axioms QCore.OuterContract.a_pending_contribution_names_its_contract
#print axioms QCore.OuterContract.every_outer_pending_names_a_contract
#print axioms QCore.OuterContract.an_uncovered_fact_is_never_safe_or_pending
#print axioms QCore.OuterContract.runtime_is_the_join
#print axioms QCore.OuterContract.reachability_over_approximates
#print axioms QCore.OuterContract.an_unclassified_pointer_reaches_every_record
#print axioms QCore.OuterContract.a_primitive_pointer_reaches_no_record
#print axioms QCore.OuterContract.projection_vectors_hold
#print axioms QCore.OuterContract.rk_callback_envelope_is_pending_with_both_loaded
