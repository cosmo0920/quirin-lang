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

import QCore.DerefDecomposition

/-! Audited companion to NULL-DEREF-QTT-GROUNDING: the dereference read through the
`quant` family (existence, extent, liveness), its derived entailments and the two
levels' elaboration (contract specification C09.3, language specification L07,
grammar G05d; USER DECISIONS 2026-10-11). -/

#print axioms QCore.DerefDecomposition.null_has_no_extent
#print axioms QCore.DerefDecomposition.zeroCheck_admits_index_zero
#print axioms QCore.DerefDecomposition.zero_check_covers_element_zero
#print axioms QCore.DerefDecomposition.checks_zero_entails_null_deref
#print axioms QCore.DerefDecomposition.checks_zero_entails_extent
#print axioms QCore.DerefDecomposition.null_deref_does_not_entail_checks_zero
#print axioms QCore.DerefDecomposition.an_unknown_extent_is_never_in_bounds
#print axioms QCore.DerefDecomposition.callerDemand_is_substitution
#print axioms QCore.DerefDecomposition.relevant_obligates_existence
#print axioms QCore.DerefDecomposition.relevant_obligates_a_stated_part
#print axioms QCore.DerefDecomposition.an_unstated_part_is_never_obligated
#print axioms QCore.DerefDecomposition.zero_lower_obligates_no_part
#print axioms QCore.DerefDecomposition.body_grade_decides_the_obligations
#print axioms QCore.DerefDecomposition.existence_and_extent_do_not_discharge_liveness
#print axioms QCore.DerefDecomposition.a_released_identity_is_refused_although_it_exists_and_has_extent
#print axioms QCore.DerefDecomposition.each_part_fails_alone
#print axioms QCore.DerefDecomposition.zero_check_means_extent
#print axioms QCore.DerefDecomposition.shorthand_means_its_parts
#print axioms QCore.DerefDecomposition.elab_preserves
#print axioms QCore.DerefDecomposition.elaboration_preserves_meaning
#print axioms QCore.DerefDecomposition.mem_merge
#print axioms QCore.DerefDecomposition.merge_preserves_meaning
#print axioms QCore.DerefDecomposition.merging_an_agreeing_overlap_is_idempotent
#print axioms QCore.DerefDecomposition.merge_merge
#print axioms QCore.DerefDecomposition.shorthand_and_decomposition_are_one_canonical_set
#print axioms QCore.DerefDecomposition.a_mix_is_the_shorthands_canonical_set
#print axioms QCore.DerefDecomposition.one_canonical_set_one_meaning
