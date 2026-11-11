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

import QCore.CmetricsApiContracts

#print axioms QCore.CmetricsApiContracts.failure_keeps_committed_obligations
#print axioms QCore.CmetricsApiContracts.cleanup_discharges_contents
#print axioms QCore.CmetricsApiContracts.cleanup_preserves_enclosing_storage
#print axioms QCore.CmetricsApiContracts.cleanup_does_not_clear_pointer_fields
#print axioms QCore.CmetricsApiContracts.a_nonempty_cleanup_is_not_generally_idempotent
#print axioms QCore.CmetricsApiContracts.getter_failure_can_keep_new_map_owned_metric
#print axioms QCore.CmetricsApiContracts.attaching_does_not_retire_the_parent
#print axioms QCore.CmetricsApiContracts.attaching_preserves_old_primary_members
#print axioms QCore.CmetricsApiContracts.embedded_result_needs_live_map
#print axioms QCore.CmetricsApiContracts.destroy_invalidates_every_metric_borrow
#print axioms QCore.CmetricsApiContracts.destroy_discharges_primary_ownership
#print axioms QCore.CmetricsApiContracts.expiration_preserves_the_enclosing_map
#print axioms QCore.CmetricsApiContracts.expiration_invalidates_an_old_borrow_epoch
#print axioms QCore.CmetricsApiContracts.parsed_model_is_not_a_trusted_call
#print axioms QCore.CmetricsApiContracts.shared_vectors
#print axioms QCore.CmetricsApiContracts.refused_counter_keeps_lookup_effects
#print axioms QCore.CmetricsApiContracts.counter_success_preserves_map_ownership
#print axioms QCore.CmetricsApiContracts.preparation_failure_preserves_old_array_ownership
#print axioms QCore.CmetricsApiContracts.replacement_retires_every_old_array
#print axioms QCore.CmetricsApiContracts.replacement_invalidates_old_array_borrow
#print axioms QCore.CmetricsApiContracts.replacement_preserves_metric_parent
#print axioms QCore.CmetricsApiContracts.null_quantile_input_preserves_flag
#print axioms QCore.CmetricsApiContracts.shared_setter_vectors
