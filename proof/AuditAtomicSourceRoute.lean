import QCore.AtomicSourceRoute

/-! Audited companion to CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i). -/
#print axioms QCore.AtomicSourceRoute.names_unique
#print axioms QCore.AtomicSourceRoute.routed_order_is_represented
#print axioms QCore.AtomicSourceRoute.routed_plain_agrees_with_model
#print axioms QCore.AtomicSourceRoute.route_class_is_plain
#print axioms QCore.AtomicSourceRoute.read_modify_write_refused
#print axioms QCore.AtomicSourceRoute.compare_exchange_refused
#print axioms QCore.AtomicSourceRoute.fence_refused
#print axioms QCore.AtomicSourceRoute.by_pointer_refused
#print axioms QCore.AtomicSourceRoute.consume_load_refused
#print axioms QCore.AtomicSourceRoute.order_one_never_routed
#print axioms QCore.AtomicSourceRoute.consume_never_routed
#print axioms QCore.AtomicSourceRoute.unlisted_refused
#print axioms QCore.AtomicSourceRoute.unevaluated_order_refused
#print axioms QCore.AtomicSourceRoute.routed_store_matches_publication
#print axioms QCore.AtomicSourceRoute.routed_load_matches_publication
#print axioms QCore.AtomicSourceRoute.releases_iff
#print axioms QCore.AtomicSourceRoute.acquires_iff
#print axioms QCore.AtomicSourceRoute.publication_verdict_carries_single_graph
#print axioms QCore.AtomicSourceRoute.failed_publication_is_unknown
#print axioms QCore.AtomicSourceRoute.undecided_publication_is_unknown
#print axioms QCore.AtomicSourceRoute.safe_publication_only_when_checked
#print axioms QCore.AtomicSourceRoute.route_vectors_hold
#print axioms QCore.AtomicSourceRoute.order_vectors_hold
