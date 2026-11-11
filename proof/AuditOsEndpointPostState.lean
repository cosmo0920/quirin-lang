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

import QCore.OsEndpointPostState

-- Card O03: the target-qualified Linux `close(2)` failure post-state relation.
-- Model of `crates/qcore-core/src/os_endpoint.rs`; specification in
-- `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`.

-- Exhaustive-domain infrastructure.
#print axioms QCore.OsEndpointPostState.mem_allTargets
#print axioms QCore.OsEndpointPostState.mem_allOutcomes
#print axioms QCore.OsEndpointPostState.mem_allPremises
#print axioms QCore.OsEndpointPostState.of_domainAll

-- Standing invariants, over the whole 3 x 64 x 10 domain.
#print axioms QCore.OsEndpointPostState.no_outcome_ever_discharges_the_data_duty
#print axioms QCore.OsEndpointPostState.a_poisoned_state_permits_no_recovery
#print axioms QCore.OsEndpointPostState.a_poisoned_state_carries_the_conservative_residual
#print axioms QCore.OsEndpointPostState.only_a_live_epoch_may_be_closed
#print axioms QCore.OsEndpointPostState.the_description_is_released_only_with_the_last_reference
#print axioms QCore.OsEndpointPostState.denied_record_locks_are_never_released

-- The arms the documentation determines, and the target divergence.
#print axioms QCore.OsEndpointPostState.linux_eintr_releases_the_descriptor
#print axioms QCore.OsEndpointPostState.posix2024_eintr_retains_the_descriptor
#print axioms QCore.OsEndpointPostState.the_targets_prescribe_opposite_recovery_on_eintr
#print axioms QCore.OsEndpointPostState.an_unpinned_eintr_is_indeterminate_and_permits_no_retry
#print axioms QCore.OsEndpointPostState.linux_deferred_io_releases_and_fails_the_data_duty
#print axioms QCore.OsEndpointPostState.ebadf_never_releases

-- Premises that must hold before any release can be concluded.
#print axioms QCore.OsEndpointPostState.a_release_requires_an_observed_result
#print axioms QCore.OsEndpointPostState.a_failed_close_releases_only_with_an_observed_errno
#print axioms QCore.OsEndpointPostState.success_without_sole_custody_never_releases
#print axioms QCore.OsEndpointPostState.the_not_attempted_arm_requires_a_live_epoch

-- Non-vacuity.
#print axioms QCore.OsEndpointPostState.every_descriptor_state_is_reachable
#print axioms QCore.OsEndpointPostState.attempted_and_indeterminate_differ_only_in_provenance

-- Joins.
#print axioms QCore.OsEndpointPostState.a_join_of_disagreeing_states_is_poisoned
#print axioms QCore.OsEndpointPostState.a_join_never_invents_a_permitted_recovery
#print axioms QCore.OsEndpointPostState.a_join_across_targets_is_refused

-- Endpoint identity, and the suppressed-wrapper contrast.
#print axioms QCore.OsEndpointPostState.a_reused_number_is_a_distinct_epoch
#print axioms QCore.OsEndpointPostState.the_suppressed_wrapper_post_state_is_not_the_linux_post_state
