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

import QCore.CmetricsDefaultSetterConditions
#print axioms QCore.CmetricsDefaultSetterConditions.histogram_preserves_bucket_count
#print axioms QCore.CmetricsDefaultSetterConditions.histogram_values_have_conversion_witness
#print axioms QCore.CmetricsDefaultSetterConditions.existing_summary_storage_needs_no_allocation
#print axioms QCore.CmetricsDefaultSetterConditions.zero_summary_count_needs_no_allocation
#print axioms QCore.CmetricsDefaultSetterConditions.summary_preparation_zeroes_new_storage
#print axioms QCore.CmetricsDefaultSetterConditions.summary_allocation_failure_has_no_success
#print axioms QCore.CmetricsDefaultSetterConditions.null_summary_input_preserves_values_and_flag
#print axioms QCore.CmetricsDefaultSetterConditions.admitted_summary_slot_receives_input
#print axioms QCore.CmetricsDefaultSetterConditions.unadmitted_summary_slot_is_preserved
#print axioms QCore.CmetricsDefaultSetterConditions.zero_copy_count_ignores_pointer
#print axioms QCore.CmetricsDefaultSetterConditions.positive_copy_count_rejects_null
#print axioms QCore.CmetricsDefaultSetterConditions.successful_copy_has_requested_length
#print axioms QCore.CmetricsDefaultSetterConditions.failed_preparation_keeps_lookup_and_discharges_temporaries
#print axioms QCore.CmetricsDefaultSetterConditions.successful_swap_invalidates_prior_array_epochs
#print axioms QCore.CmetricsDefaultSetterConditions.replacement_releases_follow_swap_and_unlock
#print axioms QCore.CmetricsDefaultSetterConditions.failed_preparation_releases_each_temporary_once
