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

import QCore.CmetricsScalarWrapperConditions
#print axioms QCore.CmetricsScalarWrapperConditions.failed_lookup_has_no_scalar_call
#print axioms QCore.CmetricsScalarWrapperConditions.refused_decrease_has_no_scalar_call
#print axioms QCore.CmetricsScalarWrapperConditions.reset_enabled_still_reads_observed_value
#print axioms QCore.CmetricsScalarWrapperConditions.no_decrease_allows_scalar_call
#print axioms QCore.CmetricsScalarWrapperConditions.counter_result_domain
#print axioms QCore.CmetricsScalarWrapperConditions.plain_success_calls_scalar_without_value_read
#print axioms QCore.CmetricsScalarWrapperConditions.plain_failure_has_no_scalar_call
#print axioms QCore.CmetricsScalarWrapperConditions.refused_phase_preserves_lookup_effects
#print axioms QCore.CmetricsScalarWrapperConditions.scalar_phase_preserves_post_lookup_ownership
