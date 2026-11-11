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

import QCore.DisjointFootprint

#print axioms QCore.DisjointFootprint.sliceBytes_within_extent
#print axioms QCore.DisjointFootprint.sliceBytes_nonempty
#print axioms QCore.DisjointFootprint.range_disjoint_iff_bytes_disjoint
#print axioms QCore.DisjointFootprint.disjoint_intervals_share_no_byte
#print axioms QCore.DisjointFootprint.overlapping_intervals_share_a_byte
#print axioms QCore.DisjointFootprint.interval_disjoint_symm
#print axioms QCore.DisjointFootprint.nonempty_interval_is_not_disjoint_from_itself
#print axioms QCore.DisjointFootprint.cross_allocation_disjointness_is_refused
#print axioms QCore.DisjointFootprint.stale_generation_is_refused
#print axioms QCore.DisjointFootprint.admitted_disjointness_has_a_ground
#print axioms QCore.DisjointFootprint.admitted_memory_disjointness_shares_no_byte
#print axioms QCore.DisjointFootprint.two_halves_of_one_allocation_are_admitted
#print axioms QCore.DisjointFootprint.field_stays_inside_its_element
#print axioms QCore.DisjointFootprint.addressable_fields_with_disjoint_ranges_are_disjoint
#print axioms QCore.DisjointFootprint.shared_bitfield_unit_is_never_disjoint
#print axioms QCore.DisjointFootprint.distinct_bitfield_units_are_decided_by_bytes
#print axioms QCore.DisjointFootprint.the_bitfield_rule_precedes_the_interval_rule
#print axioms QCore.DisjointFootprint.an_addressable_field_shares_no_storage_unit
#print axioms QCore.DisjointFootprint.descriptors_sharing_a_cell_are_never_disjoint
#print axioms QCore.DisjointFootprint.descriptors_over_distinct_cells_are_disjoint
#print axioms QCore.DisjointFootprint.memory_and_descriptor_are_incomparable
#print axioms QCore.DisjointFootprint.admitted_disjointness_is_symmetric
#print axioms QCore.DisjointFootprint.reallocation_retires_every_handle_of_the_old_generation
#print axioms QCore.DisjointFootprint.destruction_retires_every_handle
#print axioms QCore.DisjointFootprint.retirement_preserves_other_allocations
#print axioms QCore.DisjointFootprint.a_stale_handle_decides_nothing
#print axioms QCore.DisjointFootprint.live_handles_of_one_generation_are_decided
#print axioms QCore.DisjointFootprint.the_same_pair_after_a_reallocation_is_refused
#print axioms QCore.DisjointFootprint.without_the_premise_nothing_changes
#print axioms QCore.DisjointFootprint.the_premise_reads_only_distinct_allocations
#print axioms QCore.DisjointFootprint.cross_allocation_disjointness_needs_the_premise
#print axioms QCore.DisjointFootprint.an_admitted_judgement_is_grounded_or_premised
#print axioms QCore.DisjointFootprint.the_premise_never_admits_a_shared_unit
#print axioms QCore.DisjointFootprint.pair_vectors_hold
#print axioms QCore.DisjointFootprint.a_punned_view_is_refused
#print axioms QCore.DisjointFootprint.an_admitted_view_names_its_premise
#print axioms QCore.DisjointFootprint.a_unit_or_descriptor_has_no_view
#print axioms QCore.DisjointFootprint.an_admitted_transfer_stays_inside_its_handles
#print axioms QCore.DisjointFootprint.an_admitted_memcpy_shares_no_byte
#print axioms QCore.DisjointFootprint.a_cross_allocation_memcpy_needs_the_premise
#print axioms QCore.DisjointFootprint.copy_vectors_hold
#print axioms QCore.DisjointFootprint.composition_binds_every_footprint
#print axioms QCore.DisjointFootprint.composition_backs_every_mutex_disjointness
#print axioms QCore.DisjointFootprint.two_resources_without_the_premise_do_not_compose
#print axioms QCore.DisjointFootprint.an_underived_interval_does_not_compose
#print axioms QCore.DisjointFootprint.the_previous_generation_is_the_former_current
#print axioms QCore.DisjointFootprint.the_summary_has_no_footprint
#print axioms QCore.DisjointFootprint.two_generations_of_a_site_are_not_assumed_disjoint
#print axioms QCore.DisjointFootprint.generation_vectors_hold
