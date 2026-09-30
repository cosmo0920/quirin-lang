import QCore.FieldLinkJoin

/-! Audited companion to B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED (partial field links, the escape refusal and the settled-exit floor, gateway rule 11c). -/

#print axioms QCore.FieldLinkJoin.link_vectors_hold
#print axioms QCore.FieldLinkJoin.join_link_comm
#print axioms QCore.FieldLinkJoin.a_one_sided_link_is_partial
#print axioms QCore.FieldLinkJoin.escape_vectors_hold
#print axioms QCore.FieldLinkJoin.escape_only_an_exactly_live_must_child
#print axioms QCore.FieldLinkJoin.a_possibly_live_child_is_escaped_or_refused
#print axioms QCore.FieldLinkJoin.an_escape_over_a_one_sided_live_child_refuses
#print axioms QCore.FieldLinkJoin.a_decided_escape_leaves_no_live_child
#print axioms QCore.FieldLinkJoin.the_live_bit_names_a_concrete_live_path
#print axioms QCore.FieldLinkJoin.the_join_adds_no_unresolved
#print axioms QCore.FieldLinkJoin.a_live_child_at_exit_is_reported
#print axioms QCore.FieldLinkJoin.join_safe_iff
#print axioms QCore.FieldLinkJoin.foldl_safe
#print axioms QCore.FieldLinkJoin.fold_safe_iff
#print axioms QCore.FieldLinkJoin.exit_vectors_hold
#print axioms QCore.FieldLinkJoin.an_unresolved_possibly_live_obligation_at_exit_is_never_safe
#print axioms QCore.FieldLinkJoin.the_model_is_sound
#print axioms QCore.FieldLinkJoin.the_model_accuses_only_concrete_leaks
#print axioms QCore.FieldLinkJoin.the_base_flow_hides_the_leak
#print axioms QCore.FieldLinkJoin.the_branch_reports_the_straight_line_leak
#print axioms QCore.FieldLinkJoin.shape_vectors_hold
