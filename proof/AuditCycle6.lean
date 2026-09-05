import QCore.ErrorRegion

-- This audit prints the axiom dependencies of every Cycle 6 safety theorem.
-- Run it with: lake env lean AuditCycle6.lean
#print axioms QCore.ErrorRegion.seal_is_path_specific
#print axioms QCore.ErrorRegion.quant_open_matching_region
#print axioms QCore.ErrorRegion.quant_open_wrong_region
#print axioms QCore.ErrorRegion.discharge_restores_region_baseline
#print axioms QCore.ErrorRegion.discharge_is_reverse_acquisition_order
#print axioms QCore.ErrorRegion.discharge_releases_exactly_pack_resources
#print axioms QCore.ErrorRegion.discharge_abandons_exactly_region_credit
#print axioms QCore.ErrorRegion.cleanup_capability_consumed_exactly_once
#print axioms QCore.ErrorRegion.live_obligations_cannot_be_abandoned
#print axioms QCore.ErrorRegion.strict_credit_cannot_be_abandoned_on_normal_catch
#print axioms QCore.ErrorRegion.payload_resource_is_excluded
#print axioms QCore.ErrorRegion.remaining_cleanup_resource_is_not_in_payload
#print axioms QCore.ErrorRegion.partial_initialization_is_exact
#print axioms QCore.ErrorRegion.nested_cleanup_order_is_deterministic
