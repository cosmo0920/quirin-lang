import QCore.RelationalForwarding

/-! Audited companion to CALLEE-RETENTION-CLASS1 stage (a): relational forwarding and the per-class row (gateway rule 12s). -/

#print axioms QCore.RelationalForwarding.every_path_is_kept_by_one_class
#print axioms QCore.RelationalForwarding.no_path_is_kept_by_two_classes
#print axioms QCore.RelationalForwarding.representative_sat
#print axioms QCore.RelationalForwarding.meets_self
#print axioms QCore.RelationalForwarding.a_forwarded_call_applies_the_callee_class
#print axioms QCore.RelationalForwarding.disjoint_others_miss
#print axioms QCore.RelationalForwarding.the_published_class_holds_at_every_exit
#print axioms QCore.RelationalForwarding.a_caller_edge_selects_the_exit_disposition
#print axioms QCore.RelationalForwarding.the_row_covers_every_path
#print axioms QCore.RelationalForwarding.a_conversion_can_move_a_value_between_classes
