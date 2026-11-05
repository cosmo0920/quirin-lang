import QCore.NullLookupEvent

/-! Audited companion to NULL-WITNESS-GUARD-CORRELATION (gateway rule 39). -/

#print axioms QCore.NullLookupEvent.a_named_miss_is_decided_or_free
#print axioms QCore.NullLookupEvent.a_tied_lookup_is_named_only_when_decided_null
#print axioms QCore.NullLookupEvent.a_correlated_untied_miss_is_refused
#print axioms QCore.NullLookupEvent.correlation_never_names
#print axioms QCore.NullLookupEvent.touching_never_frees
#print axioms QCore.NullLookupEvent.a_call_before_never_frees_a_global
#print axioms QCore.NullLookupEvent.a_local_or_unrooted_argument_is_correlated
#print axioms QCore.NullLookupEvent.one_correlated_argument_correlates
#print axioms QCore.NullLookupEvent.a_free_lookup_has_only_free_arguments
#print axioms QCore.NullLookupEvent.a_correlated_miss_never_contributes
#print axioms QCore.NullLookupEvent.a_refused_miss_never_concludes_infeasible
#print axioms QCore.NullLookupEvent.a_refused_miss_is_never_a_finding
#print axioms QCore.NullLookupEvent.miss_vectors_hold
#print axioms QCore.NullLookupEvent.state_vectors_hold
