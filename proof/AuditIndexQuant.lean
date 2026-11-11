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

import QCore.IndexQuant

/-! Audited companion to QUANT-INDEX-CHECKS (gateway rule 12l). -/

#print axioms QCore.IndexQuant.rel_ival_mem
#print axioms QCore.IndexQuant.lo_ok_max
#print axioms QCore.IndexQuant.hi_ok_min
#print axioms QCore.IndexQuant.inter_mem
#print axioms QCore.IndexQuant.any_and_left
#print axioms QCore.IndexQuant.any_and_right
#print axioms QCore.IndexQuant.mem_reading
#print axioms QCore.IndexQuant.reading_sound
#print axioms QCore.IndexQuant.reading_complete
#print axioms QCore.IndexQuant.or_reading_literal
#print axioms QCore.IndexQuant.or_holds_literal
#print axioms QCore.IndexQuant.over_wide_disjunction_admits_every_index
#print axioms QCore.IndexQuant.mem_nonempty
#print axioms QCore.IndexQuant.closed_mem_le
#print axioms QCore.IndexQuant.hull_bounds
#print axioms QCore.IndexQuant.bounded_hull_sound
#print axioms QCore.IndexQuant.empty_admits_nothing
#print axioms QCore.IndexQuant.unbounded_admits_beyond_every_bound
#print axioms QCore.IndexQuant.no_obligation_without_a_bounded_set
#print axioms QCore.IndexQuant.covered_sound
#print axioms QCore.IndexQuant.disjoint_sound
#print axioms QCore.IndexQuant.inside_sound
#print axioms QCore.IndexQuant.outside_sound
#print axioms QCore.IndexQuant.an_unknown_index_is_never_inside
#print axioms QCore.IndexQuant.quant_refusal_never_safe
#print axioms QCore.IndexQuant.quant_refusal_never_accuses
#print axioms QCore.IndexQuant.both_in_bounds_sound
#print axioms QCore.IndexQuant.covering_the_ends_covers_the_hull
#print axioms QCore.IndexQuant.combine_inside
#print axioms QCore.IndexQuant.shifted_within_sound
#print axioms QCore.IndexQuant.admitted_vectors_hold
#print axioms QCore.IndexQuant.admitted_vectors_count
#print axioms QCore.IndexQuant.decide_vectors_hold
#print axioms QCore.IndexQuant.decide_vectors_count
#print axioms QCore.IndexQuant.shift_vectors_hold
#print axioms QCore.IndexQuant.shift_vectors_count
