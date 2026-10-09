import QCore.DeclaredEntryCheck

/-! Audited companion to DECLARED-ASSUMPTION-BODY-CHECKS: a W4 conflict that only refines a declaration is settled by the declaration's own entry check (rule 13m), evidence only (rule 13n), and the new evidence covers what `entry` covers. -/

#print axioms QCore.DeclaredEntryCheck.new_evidence_needs_the_entry_check
#print axioms QCore.DeclaredEntryCheck.new_evidence_needs_a_conflict
#print axioms QCore.DeclaredEntryCheck.a_container_or_counted_user_declaration_is_never_checked
#print axioms QCore.DeclaredEntryCheck.a_withdrawn_or_refuted_declaration_is_never_checked
#print axioms QCore.DeclaredEntryCheck.a_failed_check_keeps_the_conflict_a_failure
#print axioms QCore.DeclaredEntryCheck.a_contradiction_still_fails
#print axioms QCore.DeclaredEntryCheck.without_a_passing_check_nothing_changes
#print axioms QCore.DeclaredEntryCheck.only_a_conflict_changes
#print axioms QCore.DeclaredEntryCheck.the_new_evidence_covers_only_custody
#print axioms QCore.DeclaredEntryCheck.the_new_evidence_never_covers_a_clause_entry_does_not
#print axioms QCore.DeclaredEntryCheck.recording_vectors_hold
