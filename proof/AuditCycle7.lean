import QCore.Lowering

-- This audit prints the axiom dependencies of every Cycle 7 safety theorem.
-- Run it with: lake env lean AuditCycle7.lean
#print axioms QCore.Lowering.source_cfg_simulation
#print axioms QCore.Lowering.error_edge_has_cleanup_label
#print axioms QCore.Lowering.partial_initialization_gets_exact_cleanup
#print axioms QCore.Lowering.cleanup_release_ids
#print axioms QCore.Lowering.cleanup_releases_exactly_live_resources
#print axioms QCore.Lowering.cleanup_releases_in_lifo_order
#print axioms QCore.Lowering.cleanup_releases_each_identity_at_most_once
#print axioms QCore.Lowering.cfg_c_simulation
#print axioms QCore.Lowering.c_lowering_simulation
#print axioms QCore.Lowering.normal_value_and_transfer_are_preserved
#print axioms QCore.Lowering.error_tag_and_cleanup_trace_are_preserved
