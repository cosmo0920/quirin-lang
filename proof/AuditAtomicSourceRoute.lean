import QCore.AtomicSourceRoute

/-! Audited companion to CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i), extended
by CG28-WEAK-MEMORY-EVENTS (rule 11t). -/
#print axioms QCore.AtomicSourceRoute.names_unique
#print axioms QCore.AtomicSourceRoute.routed_order_is_represented
#print axioms QCore.AtomicSourceRoute.routed_plain_agrees_with_model
#print axioms QCore.AtomicSourceRoute.route_class_is_routed
#print axioms QCore.AtomicSourceRoute.read_modify_write_routed
#print axioms QCore.AtomicSourceRoute.fence_routed
#print axioms QCore.AtomicSourceRoute.consume_fence_is_acquire
#print axioms QCore.AtomicSourceRoute.legacy_sync_refused
#print axioms QCore.AtomicSourceRoute.by_pointer_refused
#print axioms QCore.AtomicSourceRoute.consume_load_refused
#print axioms QCore.AtomicSourceRoute.consume_order_refused
#print axioms QCore.AtomicSourceRoute.order_one_never_routed
#print axioms QCore.AtomicSourceRoute.consume_never_routed
#print axioms QCore.AtomicSourceRoute.unlisted_refused
#print axioms QCore.AtomicSourceRoute.unevaluated_order_refused
#print axioms QCore.AtomicSourceRoute.routed_failure_admitted_by_model
#print axioms QCore.AtomicSourceRoute.routed_failure_no_stronger
#print axioms QCore.AtomicSourceRoute.cas_strength_by_name
#print axioms QCore.AtomicSourceRoute.gnu_cas_unevaluated_weak_flag_refused
#print axioms QCore.AtomicSourceRoute.gnu_cas_strength_by_weak_flag
#print axioms QCore.AtomicSourceRoute.routed_cas_failure_admitted
#print axioms QCore.AtomicSourceRoute.routed_store_matches_publication
#print axioms QCore.AtomicSourceRoute.routed_load_matches_publication
#print axioms QCore.AtomicSourceRoute.releases_iff
#print axioms QCore.AtomicSourceRoute.acquires_iff
#print axioms QCore.AtomicSourceRoute.publication_verdict_carries_single_graph
#print axioms QCore.AtomicSourceRoute.failed_publication_is_unknown
#print axioms QCore.AtomicSourceRoute.undecided_publication_is_unknown
#print axioms QCore.AtomicSourceRoute.safe_publication_only_when_checked
#print axioms QCore.AtomicSourceRoute.route_vectors_hold
#print axioms QCore.AtomicSourceRoute.cas_vectors_hold
#print axioms QCore.AtomicSourceRoute.order_vectors_hold
