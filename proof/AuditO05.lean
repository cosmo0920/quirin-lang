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

import QCore.AggregateComposition

#print axioms QCore.AggregateComposition.an_empty_aggregate_is_not_a_vacuous_success
#print axioms QCore.AggregateComposition.a_structurally_malformed_program_is_unknown
#print axioms QCore.AggregateComposition.a_malformed_program_reports_only_structural_refusals
#print axioms QCore.AggregateComposition.any_unsupported_class_blocks_the_composition
#print axioms QCore.AggregateComposition.an_unadmitted_callback_blocks_the_composition
#print axioms QCore.AggregateComposition.a_composed_program_has_no_unsupported_class
#print axioms QCore.AggregateComposition.duplicate_identities_are_unknown
#print axioms QCore.AggregateComposition.a_composed_program_separates_every_identity
#print axioms QCore.AggregateComposition.a_reused_live_address_is_unknown
#print axioms QCore.AggregateComposition.an_unreleased_member_is_unknown
#print axioms QCore.AggregateComposition.a_member_released_twice_is_unknown
#print axioms QCore.AggregateComposition.a_never_acquired_member_is_unknown
#print axioms QCore.AggregateComposition.a_release_before_its_acquisition_is_unknown
#print axioms QCore.AggregateComposition.no_status_fact_discharges_custody
#print axioms QCore.AggregateComposition.no_durability_premise_discharges_custody
#print axioms QCore.AggregateComposition.an_unresolved_disposition_is_unknown
#print axioms QCore.AggregateComposition.a_retained_endpoint_without_a_custodian_is_unknown
#print axioms QCore.AggregateComposition.a_retained_allocation_is_unknown
#print axioms QCore.AggregateComposition.a_composition_that_never_releases_the_parent_is_unknown
#print axioms QCore.AggregateComposition.a_live_callback_over_a_member_payload_is_unknown
#print axioms QCore.AggregateComposition.an_unaccounted_spawned_task_is_unknown
#print axioms QCore.AggregateComposition.an_unjoined_task_capturing_a_member_is_unknown
#print axioms QCore.AggregateComposition.an_outstanding_guard_over_a_member_is_unknown
#print axioms QCore.AggregateComposition.an_overlapping_containment_tree_is_unknown
#print axioms QCore.AggregateComposition.an_owned_hold_the_parent_also_releases_is_unknown
#print axioms QCore.AggregateComposition.a_borrow_that_is_never_dropped_is_unknown
#print axioms QCore.AggregateComposition.an_unreclaimed_publication_is_unknown
#print axioms QCore.AggregateComposition.composition_is_never_a_violation
#print axioms QCore.AggregateComposition.composed_iff_no_refusal
#print axioms QCore.AggregateComposition.an_unresolved_member_never_composes
#print axioms QCore.AggregateComposition.a_composed_program_offers_no_status_fact
#print axioms QCore.AggregateComposition.a_composed_program_discharges_no_durability_premise
#print axioms QCore.AggregateComposition.a_composed_program_releases_every_member
#print axioms QCore.AggregateComposition.a_composed_program_acquires_every_member
#print axioms QCore.AggregateComposition.a_composed_program_quiesces_every_member_callback
#print axioms QCore.AggregateComposition.a_composed_program_joins_every_capturing_task
#print axioms QCore.AggregateComposition.the_reference_aggregate_composes
#print axioms QCore.AggregateComposition.a_live_callback_makes_the_reference_unknown
#print axioms QCore.AggregateComposition.a_reused_live_descriptor_number_makes_the_reference_unknown
