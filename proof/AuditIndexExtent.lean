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

import QCore.IndexExtent

/-! Audited companion to CG16-INDEX-AND-OFFSET-EXTENTS (gateway rule 11s). -/

#print axioms QCore.IndexExtent.element_access_sound
#print axioms QCore.IndexExtent.formation_sound
#print axioms QCore.IndexExtent.out_of_bounds_sound
#print axioms QCore.IndexExtent.missing_extent_never_in_bounds
#print axioms QCore.IndexExtent.one_past_formed_never_accessed
#print axioms QCore.IndexExtent.empty_extent_admits_no_access
#print axioms QCore.IndexExtent.byte_access_sound
#print axioms QCore.IndexExtent.scale_sound
#print axioms QCore.IndexExtent.declared_never_overrides_source
#print axioms QCore.IndexExtent.a_larger_claim_is_a_conflict
#print axioms QCore.IndexExtent.write_withdraws_every_mention
#print axioms QCore.IndexExtent.write_keeps_unrelated
#print axioms QCore.IndexExtent.unsafe_only_for_a_reachable_violation
#print axioms QCore.IndexExtent.safe_only_when_every_obligation_discharges
#print axioms QCore.IndexExtent.held_never_safe
#print axioms QCore.IndexExtent.held_never_accuses
#print axioms QCore.IndexExtent.pending_only_as_the_sole_obstacle
#print axioms QCore.IndexExtent.another_obstacle_keeps_unknown
#print axioms QCore.IndexExtent.custody_safe_bounds_unknown_is_unknown
#print axioms QCore.IndexExtent.custody_safe_bounds_pending_is_pending
#print axioms QCore.IndexExtent.custody_safe_bounds_unsafe_is_unsafe
#print axioms QCore.IndexExtent.a_missing_requested_row_is_unknown
#print axioms QCore.IndexExtent.combined_safe_needs_both
#print axioms QCore.IndexExtent.admitted_evidence_matches
#print axioms QCore.IndexExtent.element_vectors_hold
#print axioms QCore.IndexExtent.element_vectors_count
#print axioms QCore.IndexExtent.byte_vectors_hold
#print axioms QCore.IndexExtent.byte_vectors_count
#print axioms QCore.IndexExtent.resolve_vectors_hold
#print axioms QCore.IndexExtent.resolve_vectors_count
#print axioms QCore.IndexExtent.verdict_vectors_hold
#print axioms QCore.IndexExtent.verdict_vectors_count
