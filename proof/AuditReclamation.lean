import QCore.Reclamation

/-! Audited companion to CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule
11z): hazard-pointer and epoch-based reclamation protocols (gap entry W10). -/
#print axioms QCore.Reclamation.hp_init_inv
#print axioms QCore.Reclamation.hazardFree_spec
#print axioms QCore.Reclamation.hp_step_preserves
#print axioms QCore.Reclamation.hp_access_safe
#print axioms QCore.Reclamation.hp_accepted_safe
#print axioms QCore.Reclamation.hp_trace_safe
#print axioms QCore.Reclamation.ebr_init_inv
#print axioms QCore.Reclamation.quiescent_spec
#print axioms QCore.Reclamation.ebr_step_preserves
#print axioms QCore.Reclamation.ebr_access_safe
#print axioms QCore.Reclamation.ebr_accepted_safe
#print axioms QCore.Reclamation.ebr_trace_safe
#print axioms QCore.Reclamation.hp_vectors_hold
#print axioms QCore.Reclamation.ebr_vectors_hold
#print axioms QCore.Reclamation.hp_use_after_retire_refused
