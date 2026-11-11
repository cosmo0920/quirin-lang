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

import QCore.LoopSiteGenerations

/-! Audited companion to CG04-LOOP-SITE-GENERATIONS (generation-indexed acquisition sites, gateway rule 11a). -/

#print axioms QCore.LoopSiteGenerations.tag_is_two_to_the_29
#print axioms QCore.LoopSiteGenerations.identity_vectors_hold
#print axioms QCore.LoopSiteGenerations.generations_name_their_site
#print axioms QCore.LoopSiteGenerations.step_vectors_hold
#print axioms QCore.LoopSiteGenerations.refused_iff_live_and_exposed
#print axioms QCore.LoopSiteGenerations.the_decision_is_monotone
#print axioms QCore.LoopSiteGenerations.merge_live
#print axioms QCore.LoopSiteGenerations.shift_live_older
#print axioms QCore.LoopSiteGenerations.reacquisition_keeps_the_earlier_obligation
#print axioms QCore.LoopSiteGenerations.an_owed_generation_stays_owed_or_refused
#print axioms QCore.LoopSiteGenerations.the_failure_edge_removes_only_the_new_generation
#print axioms QCore.LoopSiteGenerations.a_live_earlier_generation_survives_the_failure_edge
#print axioms QCore.LoopSiteGenerations.map_unresolved_live
#print axioms QCore.LoopSiteGenerations.the_summary_never_discharges
#print axioms QCore.LoopSiteGenerations.the_summary_never_discharges_along_a_run
#print axioms QCore.LoopSiteGenerations.unresolved_merge
#print axioms QCore.LoopSiteGenerations.unresolved_marked
#print axioms QCore.LoopSiteGenerations.unresolved_fresh
#print axioms QCore.LoopSiteGenerations.unresolved_none
#print axioms QCore.LoopSiteGenerations.unresolved_released
#print axioms QCore.LoopSiteGenerations.unresolved_linked
#print axioms QCore.LoopSiteGenerations.operations_keep_unresolved_reported
#print axioms QCore.LoopSiteGenerations.a_live_summary_is_never_safe
#print axioms QCore.LoopSiteGenerations.the_one_site_flow_hides_the_leak
#print axioms QCore.LoopSiteGenerations.acquisitions_succ
#print axioms QCore.LoopSiteGenerations.acquisitions_settle
#print axioms QCore.LoopSiteGenerations.for_break_reports_exactly_the_concrete_leak
#print axioms QCore.LoopSiteGenerations.the_joined_exit_reports_the_leak
#print axioms QCore.LoopSiteGenerations.release_each_iteration_succ
#print axioms QCore.LoopSiteGenerations.release_each_iteration_settles
#print axioms QCore.LoopSiteGenerations.release_each_iteration_is_safe
#print axioms QCore.LoopSiteGenerations.release_after_the_loop_reports_the_earlier_generations
#print axioms QCore.LoopSiteGenerations.link_each_iteration_is_refused_from_the_third
#print axioms QCore.LoopSiteGenerations.refusals_persist
#print axioms QCore.LoopSiteGenerations.a_refused_acquisition_is_never_safe
#print axioms QCore.LoopSiteGenerations.shape_vectors_hold
