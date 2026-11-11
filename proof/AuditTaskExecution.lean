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

import QCore.TaskExecution

#print axioms QCore.TaskExecution.checked_copy_preserves_ordered_values
#print axioms QCore.TaskExecution.checked_copy_has_exact_parameter_types
#print axioms QCore.TaskExecution.checked_copy_preserves_arity
#print axioms QCore.TaskExecution.i32_representation_is_bounded
#print axioms QCore.TaskExecution.failed_create_changes_no_handles
#print axioms QCore.TaskExecution.failed_create_produces_no_handle
#print axioms QCore.TaskExecution.successful_create_commits_fresh_handle
#print axioms QCore.TaskExecution.completion_retains_exact_outcome
#print axioms QCore.TaskExecution.completed_result_cannot_be_overwritten
#print axioms QCore.TaskExecution.unrelated_handle_write_preserves_completion
#print axioms QCore.TaskExecution.explicit_creator_join_collects_exact_outcome
#print axioms QCore.TaskExecution.noncreator_cannot_collect_completion
#print axioms QCore.TaskExecution.checked_join_consumes_handle_once
#print axioms QCore.TaskExecution.checked_join_preserves_result_type
#print axioms QCore.TaskExecution.checked_propagation_cannot_strand_outstanding_handles
#print axioms QCore.TaskExecution.completed_unjoined_child_prevents_propagation
#print axioms QCore.TaskExecution.normal_and_error_outcomes_are_distinct
#print axioms QCore.TaskExecution.worker_error_tag_is_preserved
#print axioms QCore.TaskExecution.outcome_discrimination_preserves_normal_value
#print axioms QCore.TaskExecution.outcome_discrimination_preserves_exact_error
#print axioms QCore.TaskExecution.consumed_result_cannot_be_discriminated_again
#print axioms QCore.TaskExecution.create_preserves_other_handle
#print axioms QCore.TaskExecution.complete_preserves_other_handle
#print axioms QCore.TaskExecution.join_preserves_other_handle
#print axioms QCore.TaskExecution.uncollected_step_retains_handle
#print axioms QCore.TaskExecution.completion_survives_until_explicit_collection
#print axioms QCore.TaskExecution.completed_handle_cannot_be_recreated
#print axioms QCore.TaskExecution.failed_create_preserves_prior_sibling
#print axioms QCore.TaskExecution.failed_create_does_not_join_or_cancel_running_sibling
#print axioms QCore.TaskExecution.failed_create_retains_completed_sibling_for_explicit_join
#print axioms QCore.TaskExecution.creation_cannot_mark_an_unjoined_sibling_joined
#print axioms QCore.TaskExecution.completion_cannot_mark_an_unjoined_sibling_joined
#print axioms QCore.TaskExecution.handler_cannot_discharge_sibling_without_explicit_join
#print axioms QCore.TaskExecution.checked_handler_exit_is_quiescent_and_preserves_outcome
#print axioms QCore.TaskExecution.checked_normal_and_error_handlers_both_require_quiescence
#print axioms QCore.TaskExecution.handler_join_preserves_exact_worker_error
#print axioms QCore.TaskExecution.failed_create_then_handler_join_preserves_worker_error
#print axioms QCore.TaskExecution.create_preserves_nonabsent_sibling
#print axioms QCore.TaskExecution.complete_preserves_nonabsent_sibling
#print axioms QCore.TaskExecution.join_preserves_nonabsent_sibling
#print axioms QCore.TaskExecution.handler_cannot_erase_existing_handle
#print axioms QCore.TaskExecution.handler_exit_requires_explicit_join_of_every_prior_sibling
#print axioms QCore.TaskExecution.failed_creation_handler_exit_joins_prior_sibling
#print axioms QCore.TaskExecution.empty_declared_error_row_rules_out_error
#print axioms QCore.TaskExecution.typed_empty_error_row_has_normal_scalar
#print axioms QCore.TaskExecution.typed_infallible_extraction_preserves_value_and_consumes_result
#print axioms QCore.TaskExecution.checked_infallible_extraction_requires_empty_declared_errors
#print axioms QCore.TaskExecution.infallible_extraction_does_not_accept_actual_error
#print axioms QCore.TaskExecution.consumed_infallible_result_cannot_be_extracted_again
#print axioms QCore.TaskExecution.attribution_never_moves_the_refusal
#print axioms QCore.TaskExecution.an_admitted_exit_is_never_attributed
#print axioms QCore.TaskExecution.a_named_owner_is_a_lent_frame_resource
#print axioms QCore.TaskExecution.the_preempting_gate_reads_the_backstops_own_judgment
#print axioms QCore.TaskExecution.an_unlent_frame_keeps_the_plain_refusal
#print axioms QCore.TaskExecution.ct24s_cell_is_attributed_to_the_destroy_before_join
