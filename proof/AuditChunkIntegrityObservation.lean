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

import QCore.ChunkIntegrityObservation

-- Card O01: the bounded chunk-file integrity observation relation.
-- Model of `crates/qcore-core/src/chunk_integrity.rs`; the recorded family and
-- its identities are archived under `docs/reports/remaining-o01/`.
-- Every theorem must depend only on `propext`, `Classical.choice` and
-- `Quot.sound`; no project axiom, no `sorry`, no placeholder.

-- Complete enumerations and the classification derived from an image alone.
#print axioms QCore.ChunkIntegrityObservation.mem_allClasses
#print axioms QCore.ChunkIntegrityObservation.mem_allInjections
#print axioms QCore.ChunkIntegrityObservation.every_content_class_is_reachable
#print axioms QCore.ChunkIntegrityObservation.a_short_image_is_a_size_failure_whatever_else_is_wrong
#print axioms QCore.ChunkIntegrityObservation.broken_magic_is_decided_before_the_checksum
#print axioms QCore.ChunkIntegrityObservation.a_clean_image_satisfies_every_rule
#print axioms QCore.ChunkIntegrityObservation.a_disagreeing_checksum_is_a_checksum_failure

-- The two axes stay separate.
#print axioms QCore.ChunkIntegrityObservation.an_injection_never_changes_the_derived_class
#print axioms QCore.ChunkIntegrityObservation.a_corruption_is_never_an_injected_failure
#print axioms QCore.ChunkIntegrityObservation.the_error_code_alone_cannot_separate_the_axes
#print axioms QCore.ChunkIntegrityObservation.a_corrupted_image_decides_the_outcome_alone
#print axioms QCore.ChunkIntegrityObservation.only_a_corrupted_image_reports_corruption
#print axioms QCore.ChunkIntegrityObservation.the_corruption_codes_are_pairwise_distinct
#print axioms QCore.ChunkIntegrityObservation.a_chunk_is_returned_only_for_a_clean_unsuppressed_open

-- Custody: allocations, endpoints, and who released them.
#print axioms QCore.ChunkIntegrityObservation.accepted_allocations_balance
#print axioms QCore.ChunkIntegrityObservation.accepted_leaves_no_outstanding_generation
#print axioms QCore.ChunkIntegrityObservation.the_harness_never_releases_a_library_allocation
#print axioms QCore.ChunkIntegrityObservation.accepted_endpoint_epochs_are_split
#print axioms QCore.ChunkIntegrityObservation.a_residual_is_exactly_a_harness_reclamation
#print axioms QCore.ChunkIntegrityObservation.a_suppressed_call_is_never_credited_to_the_library
#print axioms QCore.ChunkIntegrityObservation.only_a_suppressed_close_leaves_a_descriptor_residual
#print axioms QCore.ChunkIntegrityObservation.accepted_declares_the_class_its_bytes_have
#print axioms QCore.ChunkIntegrityObservation.accepted_content_keeps_its_class
#print axioms QCore.ChunkIntegrityObservation.accepted_outcome_is_the_determined_one
#print axioms QCore.ChunkIntegrityObservation.an_injection_is_accounted_for_in_both_directions
#print axioms QCore.ChunkIntegrityObservation.a_suppressed_call_carries_an_errno

-- The verdict, and what it is not.
#print axioms QCore.ChunkIntegrityObservation.no_arm_publishes_anything_but_an_observation
#print axioms QCore.ChunkIntegrityObservation.a_refused_record_publishes_nothing
#print axioms QCore.ChunkIntegrityObservation.a_verdict_carries_both_axes_separately
#print axioms QCore.ChunkIntegrityObservation.a_verdict_reports_only_harness_reclamations

-- Non-vacuity, and the refusals that make the relation fail closed.
#print axioms QCore.ChunkIntegrityObservation.the_relation_accepts_the_recorded_shapes
#print axioms QCore.ChunkIntegrityObservation.the_combined_case_keeps_both_axes
#print axioms QCore.ChunkIntegrityObservation.a_claimed_corruption_that_is_not_in_the_bytes_is_refused
#print axioms QCore.ChunkIntegrityObservation.a_suppressed_close_credited_to_the_library_is_refused
#print axioms QCore.ChunkIntegrityObservation.a_residual_without_a_reclamation_is_refused
#print axioms QCore.ChunkIntegrityObservation.a_residual_no_suppression_explains_is_refused
#print axioms QCore.ChunkIntegrityObservation.an_outstanding_generation_is_refused
#print axioms QCore.ChunkIntegrityObservation.a_harness_claim_on_a_library_allocation_is_refused
#print axioms QCore.ChunkIntegrityObservation.a_corruption_may_not_borrow_an_injection_report
#print axioms QCore.ChunkIntegrityObservation.an_injection_may_not_borrow_a_corruption_code
#print axioms QCore.ChunkIntegrityObservation.a_suppressed_call_without_an_errno_is_refused
#print axioms QCore.ChunkIntegrityObservation.a_replaced_fixture_is_refused

