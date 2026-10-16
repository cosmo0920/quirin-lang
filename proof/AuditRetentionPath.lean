import QCore.RetentionPath

/-! Audited companion to FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (gateway rule 12t, contract specification C04.14; USER DECISIONS 2026-10-16, "Tracked holder path" and "R3: quant state"): a record held by a resolved holder path ends only by a removal of exactly that registration, so a holder that would outlive the held object is never `Safe`; an unresolved path never decides; an unmatched removal keeps every record; a release while any record (an interior included) holds the identity refuses. -/

#print axioms QCore.RetentionPath.run_append
#print axioms QCore.RetentionPath.an_unresolved_path_never_decides
#print axioms QCore.RetentionPath.an_unmatched_removal_keeps_every_record
#print axioms QCore.RetentionPath.a_removal_drops_exactly_its_record
#print axioms QCore.RetentionPath.step_keeps_a_path_record
#print axioms QCore.RetentionPath.a_path_record_ends_only_by_its_own_removal
#print axioms QCore.RetentionPath.a_path_record_without_its_removal_is_never_safe
#print axioms QCore.RetentionPath.release_refused_iff
#print axioms QCore.RetentionPath.a_safe_path_never_releases_a_held_identity
#print axioms QCore.RetentionPath.a_safe_path_leaves_no_outliving_holder_holding
#print axioms QCore.RetentionPath.path_decision_vectors_hold
#print axioms QCore.RetentionPath.path_trace_vectors_hold
