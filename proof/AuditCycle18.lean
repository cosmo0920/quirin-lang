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

import QCore.EventDriven
import QCore.EventObservation

#print axioms QCore.EventDriven.accepted_transition_preserves_lifecycle
#print axioms QCore.EventDriven.accepted_transition_is_generation_safe
#print axioms QCore.EventDriven.accepted_lifecycles_are_well_formed
#print axioms QCore.EventDriven.accepted_callback_has_live_declared_authority
#print axioms QCore.EventDriven.accepted_same_base_callbacks_are_exclusive
#print axioms QCore.EventDriven.accepted_cross_context_conflicts_are_ordered
#print axioms QCore.EventDriven.accepted_close_authority_and_generations_are_unique
#print axioms QCore.EventDriven.accepted_persistent_and_deferred_callbacks_preserve_custody
#print axioms QCore.EventDriven.accepted_teardown_is_quiescent
#print axioms QCore.EventDriven.accepted_cleanup_discharges_owned_resources_once
#print axioms QCore.EventDriven.accepted_progress_claim_is_qualified_by_blocking
#print axioms QCore.EventDriven.accepted_observation_refines_event_trace
#print axioms QCore.EventDriven.combined_cycle18_safety
#print axioms QCore.EventObservation.accepted_trace_is_exact
#print axioms QCore.EventObservation.accepted_trace_has_no_invented_synchronization
#print axioms QCore.EventObservation.accepted_callback_binds_expected_event_generation
#print axioms QCore.EventObservation.changing_the_event_generation_changes_the_callback_record
