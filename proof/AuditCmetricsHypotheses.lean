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

import QCore.CmetricsHypotheses
#print axioms QCore.CmetricsHypotheses.five_null_fields_decompose
#print axioms QCore.CmetricsHypotheses.valid_handles_decompose
#print axioms QCore.CmetricsHypotheses.occupied_fields_have_distinct_identities
#print axioms QCore.CmetricsHypotheses.valid_handle_has_unique_owner
#print axioms QCore.CmetricsHypotheses.explicit_cleanup_inputs_establish_canExit
#print axioms QCore.CmetricsHypotheses.admitted_cleanup_preserves_record_and_releases_contents
#print axioms QCore.CmetricsHypotheses.guarded_quantile_decomposition
#print axioms QCore.CmetricsHypotheses.null_metric_needs_no_quantile_storage
#print axioms QCore.CmetricsHypotheses.null_array_needs_no_quantile_storage
#print axioms QCore.CmetricsHypotheses.invalid_index_needs_no_quantile_storage
#print axioms QCore.CmetricsHypotheses.exclusion_union_decomposition
#print axioms QCore.CmetricsHypotheses.exclusion_preserved_by_footprint_subset
#print axioms QCore.CmetricsHypotheses.allocation_bounds_imply_element_and_byte_range
#print axioms QCore.CmetricsHypotheses.positive_width_allocation_implies_element_count_fits
#print axioms QCore.CmetricsHypotheses.signed_unsigned_conversion_intersection
#print axioms QCore.CmetricsHypotheses.zero_truncated_finite_value_has_unsigned_conversion
