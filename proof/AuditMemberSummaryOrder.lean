import QCore.MemberSummaryOrder

/-! Audited companion to RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN (gateway rule 12n, amended): a decided member invocation's closed candidates are summarised before their caller, so a formal released through the call reads as a tracked local; the formal is never Safe where the local is not, and a candidate that could alias the formal keeps it non-Safe. -/

#print axioms QCore.MemberSummaryOrder.a_closed_member_candidate_precedes_its_caller
#print axioms QCore.MemberSummaryOrder.an_open_member_adds_no_edge
#print axioms QCore.MemberSummaryOrder.installed_classes_agree
#print axioms QCore.MemberSummaryOrder.an_uninstalled_candidate_refuses_at_summary_time
#print axioms QCore.MemberSummaryOrder.a_formal_reads_as_the_local_once_candidates_precede
#print axioms QCore.MemberSummaryOrder.the_formal_is_never_safe_where_the_local_is_not
#print axioms QCore.MemberSummaryOrder.a_candidate_that_could_alias_keeps_the_formal_unsafe
#print axioms QCore.MemberSummaryOrder.fixture_vectors_hold
