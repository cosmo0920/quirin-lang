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

import QCore.ValueConnectives

/-! Audited companion to CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (gateway rule 10z). -/
#print axioms QCore.ValueConnectives.and_skips
#print axioms QCore.ValueConnectives.or_skips
#print axioms QCore.ValueConnectives.and_evaluates
#print axioms QCore.ValueConnectives.or_evaluates
#print axioms QCore.ValueConnectives.truth_boolean
#print axioms QCore.ValueConnectives.connective_value_boolean
#print axioms QCore.ValueConnectives.cond_selects
#print axioms QCore.ValueConnectives.cond_yes
#print axioms QCore.ValueConnectives.cond_no
#print axioms QCore.ValueConnectives.elvis_once
#print axioms QCore.ValueConnectives.elvis_skips
#print axioms QCore.ValueConnectives.elvis_not_cond
#print axioms QCore.ValueConnectives.run_reads
#print axioms QCore.ValueConnectives.elvis_reread
#print axioms QCore.ValueConnectives.decision_first
#print axioms QCore.ValueConnectives.inert_read_stable
#print axioms QCore.ValueConnectives.control_commutes
#print axioms QCore.ValueConnectives.merge_carries
#print axioms QCore.ValueConnectives.release_merged_refuses
#print axioms QCore.ValueConnectives.release_single
#print axioms QCore.ValueConnectives.release_never_picks
#print axioms QCore.ValueConnectives.branching_forms_elaborated
#print axioms QCore.ValueConnectives.unlisted_position_refused
#print axioms QCore.ValueConnectives.sequencing_positions_refused
#print axioms QCore.ValueConnectives.admission_vectors_hold
#print axioms QCore.ValueConnectives.sibling_effect_refused
#print axioms QCore.ValueConnectives.effectful_decision_needs_inert
#print axioms QCore.ValueConnectives.omitted_middle_effect_refused
#print axioms QCore.ValueConnectives.composed_vectors_hold
#print axioms QCore.ValueConnectives.composition_extends_admit
#print axioms QCore.ValueConnectives.uncomposed_is_admit
#print axioms QCore.ValueConnectives.composition_never_rereads
#print axioms QCore.ValueConnectives.ctl_respects
#print axioms QCore.ValueConnectives.composed_arm_runs_alike
#print axioms QCore.ValueConnectives.composed_region_runs_alike_on_each_arm
#print axioms QCore.ValueConnectives.decision_value_stable
#print axioms QCore.ValueConnectives.an_overlapping_sibling_is_refused
#print axioms QCore.ValueConnectives.the_released_identity_is_refused
#print axioms QCore.ValueConnectives.another_name_is_refused
#print axioms QCore.ValueConnectives.a_fresh_release_commutes
#print axioms QCore.ValueConnectives.unread_sibling_tree
#print axioms QCore.ValueConnectives.an_uncontracted_sibling_is_refused
#print axioms QCore.ValueConnectives.a_non_call_sibling_is_refused
#print axioms QCore.ValueConnectives.compose_admits
#print axioms QCore.ValueConnectives.footprint_vectors_count
