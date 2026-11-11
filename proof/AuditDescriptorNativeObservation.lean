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

import QCore.DescriptorNativeObservation

-- W43-DESCRIPTOR-NATIVE-CHECK: observation-level descriptor lifecycle model.
-- These are model theorems about accepted (violation-free) traces; the Rust
-- replay's correspondence to the model is maintained by review (rule 28), and
-- native records remain observations, not proofs.

-- Step characterizations and generation monotonicity.
#print axioms QCore.DescriptorNativeObservation.run_append
#print axioms QCore.DescriptorNativeObservation.step_acquire_some
#print axioms QCore.DescriptorNativeObservation.step_use_some
#print axioms QCore.DescriptorNativeObservation.step_release_some
#print axioms QCore.DescriptorNativeObservation.step_generation_monotone
#print axioms QCore.DescriptorNativeObservation.run_events_not_older

-- A release matches its generation's kind.
#print axioms QCore.DescriptorNativeObservation.release_matches_live_kind
#print axioms QCore.DescriptorNativeObservation.release_keeps_generation
#print axioms QCore.DescriptorNativeObservation.kind_stable_at_generation
#print axioms QCore.DescriptorNativeObservation.release_kind_is_acquisition_kind

-- Each generation is released at most once; no use after release.
#print axioms QCore.DescriptorNativeObservation.single_owner_release_is_terminal
#print axioms QCore.DescriptorNativeObservation.nothing_names_a_released_generation
#print axioms QCore.DescriptorNativeObservation.single_owner_generation_released_at_most_once
#print axioms QCore.DescriptorNativeObservation.single_owner_generation_not_used_after_release
#print axioms QCore.DescriptorNativeObservation.counted_release_keeps_generation_until_last

-- The leak set at exit is well defined.
#print axioms QCore.DescriptorNativeObservation.runAll_projects
#print axioms QCore.DescriptorNativeObservation.leak_generation_unique
#print axioms QCore.DescriptorNativeObservation.live_generation_was_acquired
#print axioms QCore.DescriptorNativeObservation.leaked_generation_has_origin

-- Executable anchors matching the Rust unit-test shapes.
#print axioms QCore.DescriptorNativeObservation.clean_fd_lifecycle_accepted
#print axioms QCore.DescriptorNativeObservation.double_close_rejected
#print axioms QCore.DescriptorNativeObservation.use_after_close_rejected
#print axioms QCore.DescriptorNativeObservation.stream_released_as_directory_rejected
#print axioms QCore.DescriptorNativeObservation.counted_dl_references_accepted
#print axioms QCore.DescriptorNativeObservation.live_identity_reacquisition_rejected

-- Phase B: recorded executions bound to a declared owned-result contract.
#print axioms QCore.DescriptorNativeObservation.consistent_failure_acquires_nothing
#print axioms QCore.DescriptorNativeObservation.consistent_success_acquires_the_declared_result
#print axioms QCore.DescriptorNativeObservation.never_failing_contract_is_contradicted_by_a_failure
#print axioms QCore.DescriptorNativeObservation.bound_acquisition_has_the_declared_kind
#print axioms QCore.DescriptorNativeObservation.release_of_a_dead_identity_is_rejected
#print axioms QCore.DescriptorNativeObservation.release_of_another_kind_is_rejected
#print axioms QCore.DescriptorNativeObservation.zero_descriptor_is_acquired_and_missed_by_a_positive_guard
#print axioms QCore.DescriptorNativeObservation.open_failure_binding_anchor
#print axioms QCore.DescriptorNativeObservation.open_success_binding_anchor
#print axioms QCore.DescriptorNativeObservation.wrong_kind_success_binding_anchor
