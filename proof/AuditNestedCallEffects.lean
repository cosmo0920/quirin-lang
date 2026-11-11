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

import QCore.NestedCallEffects

/-! Audited companion to CG05-NESTED-CALL-EFFECTS (gateway rule 10y). -/

#print axioms QCore.NestedCallEffects.overlap_symm
#print axioms QCore.NestedCallEffects.overlap_vectors_hold
#print axioms QCore.NestedCallEffects.overlap_sound
#print axioms QCore.NestedCallEffects.no_clash
#print axioms QCore.NestedCallEffects.disjoint_of_no_clash
#print axioms QCore.NestedCallEffects.footprint_commute_sound
#print axioms QCore.NestedCallEffects.store_respects
#print axioms QCore.NestedCallEffects.overlapping_writes_do_not_commute
#print axioms QCore.NestedCallEffects.a_release_and_a_borrow_of_one_identity_are_refused
#print axioms QCore.NestedCallEffects.a_read_unordered_with_its_release_is_refused
#print axioms QCore.NestedCallEffects.two_releases_are_refused
#print axioms QCore.NestedCallEffects.fresh_results_from_borrowed_reads_commute
#print axioms QCore.NestedCallEffects.run_append
#print axioms QCore.NestedCallEffects.step_through
#print axioms QCore.NestedCallEffects.interleaving_runs_as_append
#print axioms QCore.NestedCallEffects.interleave_mem
#print axioms QCore.NestedCallEffects.interleave_append
#print axioms QCore.NestedCallEffects.inner_effects_precede_the_outer_entry
#print axioms QCore.NestedCallEffects.post_is_permitted
#print axioms QCore.NestedCallEffects.permitted_mem
#print axioms QCore.NestedCallEffects.permitted_runs_as_post
#print axioms QCore.NestedCallEffects.post_map
#print axioms QCore.NestedCallEffects.check_admits
#print axioms QCore.NestedCallEffects.every_interleaving_runs_alike
#print axioms QCore.NestedCallEffects.a_non_commuting_pair_is_refused
#print axioms QCore.NestedCallEffects.only_listed_forms_are_admitted
#print axioms QCore.NestedCallEffects.order_vectors_count
