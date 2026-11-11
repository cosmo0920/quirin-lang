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

import QCore.Cycle18CallbackObligations

#print axioms QCore.Cycle18CallbackObligations.composed_discharges_every_relevant_obligation
#print axioms QCore.Cycle18CallbackObligations.composed_has_no_blocking_scenario
#print axioms QCore.Cycle18CallbackObligations.composed_has_joined_family
#print axioms QCore.Cycle18CallbackObligations.open_relevant_obligation_is_unknown
#print axioms QCore.Cycle18CallbackObligations.blocking_scenario_is_unknown
#print axioms QCore.Cycle18CallbackObligations.fully_discharged_row_composes
#print axioms QCore.Cycle18CallbackObligations.irrelevant_pending_obligation_does_not_block
#print axioms QCore.Cycle18CallbackObligations.measured_wait_callback_is_unknown
#print axioms QCore.Cycle18CallbackObligations.refuted_failure_branch_is_never_taken
#print axioms QCore.Cycle18CallbackObligations.varying_callee_does_not_refute
#print axioms QCore.Cycle18CallbackObligations.opaque_callee_path_does_not_refute
#print axioms QCore.Cycle18CallbackObligations.pinned_initialization_branch_is_refuted
#print axioms QCore.Cycle18CallbackObligations.satisfied_guard_does_not_refute
#print axioms QCore.Cycle18CallbackObligations.independent_selection_ignores_flags
#print axioms QCore.Cycle18CallbackObligations.cb_event_classes_partition_every_flags_value
#print axioms QCore.Cycle18CallbackObligations.teardown_paths_release_everything
#print axioms QCore.Cycle18CallbackObligations.success_path_preserves_registration
#print axioms QCore.Cycle18CallbackObligations.success_then_teardown_releases_everything
#print axioms QCore.Cycle18CallbackObligations.no_activation_after_teardown
#print axioms QCore.Cycle18CallbackObligations.close_failure_branch_is_not_released
#print axioms QCore.Cycle18CallbackObligations.success_path_alone_is_not_released
#print axioms QCore.Cycle18CallbackObligations.reordered_teardown_is_refused
