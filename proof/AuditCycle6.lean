-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

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
