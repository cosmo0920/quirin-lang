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

import QCore.LayoutExtents

/-! Audited companion to CG17-LAYOUT-DERIVED-EXTENTS (gateway rule 11u). -/
#print axioms QCore.LayoutExtents.every_counted_element_lies_inside
#print axioms QCore.LayoutExtents.the_count_is_maximal
#print axioms QCore.LayoutExtents.an_unestablished_size_counts_nothing
#print axioms QCore.LayoutExtents.flex_vectors_hold
#print axioms QCore.LayoutExtents.an_admitted_declaration_carries_the_bound_obligation
#print axioms QCore.LayoutExtents.an_escaping_address_is_never_admitted
#print axioms QCore.LayoutExtents.a_bound_with_an_effect_is_never_admitted
#print axioms QCore.LayoutExtents.admitted_iff
#print axioms QCore.LayoutExtents.the_bound_is_read_once
#print axioms QCore.LayoutExtents.the_lifetime_starts_once
#print axioms QCore.LayoutExtents.accesses_lie_inside_the_lifetime
#print axioms QCore.LayoutExtents.the_address_is_dead_after_the_block
#print axioms QCore.LayoutExtents.vla_vectors_hold
#print axioms QCore.LayoutExtents.aligned_is_sound
#print axioms QCore.LayoutExtents.misaligned_is_sound
#print axioms QCore.LayoutExtents.an_unknown_offset_is_undecided
#print axioms QCore.LayoutExtents.safe_iff_every_obligation_is_aligned
#print axioms QCore.LayoutExtents.a_misaligned_obligation_is_never_safe
#print axioms QCore.LayoutExtents.alignment_vectors_hold
#print axioms QCore.LayoutExtents.an_enclosing_answer_has_its_premise
#print axioms QCore.LayoutExtents.an_unrelated_pointer_is_refused
#print axioms QCore.LayoutExtents.the_round_trip_returns_the_enclosing_object
#print axioms QCore.LayoutExtents.the_round_trip_needs_the_premise
#print axioms QCore.LayoutExtents.container_vectors_hold
