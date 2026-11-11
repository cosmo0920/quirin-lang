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

import QCore.CapacityBoundary

-- Card O02: one capacity boundary family — the file-backed grow boundary and
-- its kernel refusal. Model of `crates/qcore-core/src/capacity_boundary.rs`;
-- recorded native evidence in `docs/reports/remaining-o02/README.md`.

-- Exhaustive-domain infrastructure.
#print axioms QCore.CapacityBoundary.mem_allTargets
#print axioms QCore.CapacityBoundary.mem_allPremises
#print axioms QCore.CapacityBoundary.of_domainAll

-- Standing invariants over the checked domain.
#print axioms QCore.CapacityBoundary.no_arm_authorises_an_unreserved_store
#print axioms QCore.CapacityBoundary.a_poisoned_arm_permits_nothing
#print axioms QCore.CapacityBoundary.capacity_is_never_content
#print axioms QCore.CapacityBoundary.an_unpinned_target_concludes_nothing
#print axioms QCore.CapacityBoundary.a_concurrent_writer_poisons_every_arm
#print axioms QCore.CapacityBoundary.an_unobserved_result_poisons_every_attempt

-- The arms the documentation determines.
#print axioms QCore.CapacityBoundary.a_length_only_grant_reserves_nothing
#print axioms QCore.CapacityBoundary.an_allocating_grant_reserves_the_range
#print axioms QCore.CapacityBoundary.an_allocating_grant_without_the_backing_is_poisoned
#print axioms QCore.CapacityBoundary.a_capacity_refusal_keeps_the_length_and_defers_the_retry
#print axioms QCore.CapacityBoundary.a_length_only_refusal_may_not_have_grown_the_file
#print axioms QCore.CapacityBoundary.an_allocating_refusal_may_have_changed_the_length
#print axioms QCore.CapacityBoundary.reading_errno_from_the_returning_call_is_a_premise_violation
#print axioms QCore.CapacityBoundary.a_partial_transfer_advances_by_exactly_what_moved
#print axioms QCore.CapacityBoundary.a_partial_outcome_from_a_resizing_call_is_poisoned
#print axioms QCore.CapacityBoundary.an_unhandled_boundary_signal_poisons_the_transfer
#print axioms QCore.CapacityBoundary.the_domain_is_declared
#print axioms QCore.CapacityBoundary.the_retry_verdict_follows_the_recovery

-- What a mapping witnesses, and what it does not.
#print axioms QCore.CapacityBoundary.a_mapping_witnesses_only_backed_bytes_inside_the_file
#print axioms QCore.CapacityBoundary.a_mapping_beyond_the_end_witnesses_nothing_there

-- The recorded trace, its epochs and its teardown.
#print axioms QCore.CapacityBoundary.the_recorded_trace_is_accepted
#print axioms QCore.CapacityBoundary.a_length_only_grant_never_raises_the_backed_prefix
#print axioms QCore.CapacityBoundary.content_claimed_into_the_hole_is_refused
#print axioms QCore.CapacityBoundary.a_trace_without_a_teardown_record_is_incomplete
#print axioms QCore.CapacityBoundary.an_event_after_the_teardown_record_is_refused

-- The mutation matrix: lost residuals, forged successful mappings, confused
-- generations and forged grants, all refused, and not vacuously.
#print axioms QCore.CapacityBoundary.every_mutation_is_refused
#print axioms QCore.CapacityBoundary.the_mutation_matrix_is_not_vacuous
