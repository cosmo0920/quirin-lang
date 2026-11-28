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

import QCore.MeasureConnect

/-! Audited companion to QUIRIN-AUDIT-CONNECT (measure chain, measure compose,
declared contract dependencies). -/

#print axioms QCore.MeasureConnect.Chain.lastBefore_succ_none
#print axioms QCore.MeasureConnect.Chain.firstFrom_succ_none
#print axioms QCore.MeasureConnect.Chain.verdict_unlisted
#print axioms QCore.MeasureConnect.Chain.lastBefore_sound
#print axioms QCore.MeasureConnect.Chain.firstFromAux_sound
#print axioms QCore.MeasureConnect.Chain.lastBefore_none
#print axioms QCore.MeasureConnect.Chain.firstFromAux_none
#print axioms QCore.MeasureConnect.Chain.recon_sound
#print axioms QCore.MeasureConnect.Chain.joined_same_report
#print axioms QCore.MeasureConnect.Chain.chain_net_is_last
#print axioms QCore.MeasureConnect.Chain.link_refused_of_differing_identity
#print axioms QCore.MeasureConnect.Chain.moved_listed
#print axioms QCore.MeasureConnect.Chain.a_cancelled_move_is_reported
#print axioms QCore.MeasureConnect.Model.heldUnknown_held
#print axioms QCore.MeasureConnect.Model.fold_mem_or_safe
#print axioms QCore.MeasureConnect.Model.member_le_composed
#print axioms QCore.MeasureConnect.Model.composition_is_the_join
#print axioms QCore.MeasureConnect.Model.composition_is_order_independent
#print axioms QCore.MeasureConnect.Model.composeReasoned_value
#print axioms QCore.MeasureConnect.Model.composition_keeps_every_reason
#print axioms QCore.MeasureConnect.Model.held_dependency_holds_dependents
#print axioms QCore.MeasureConnect.Model.reach_mono
#print axioms QCore.MeasureConnect.Model.held_mono
#print axioms QCore.MeasureConnect.Model.held_is_order_independent
#print axioms QCore.MeasureConnect.Model.resolution_is_acyclic
#print axioms QCore.MeasureConnect.Model.holdVerdict_of_unknown
#print axioms QCore.MeasureConnect.Model.holdVerdict_of_held
#print axioms QCore.MeasureConnect.Model.holdVerdict_le_contribution
#print axioms QCore.MeasureConnect.formula_cx_joined_is_every_identity
#print axioms QCore.MeasureConnect.formula_cx_every_identity_is_joined
#print axioms QCore.MeasureConnect.formula_cx_refuse_unrecorded
#print axioms QCore.MeasureConnect.formula_cx_refuse_tool
#print axioms QCore.MeasureConnect.formula_cx_refuse_revision
#print axioms QCore.MeasureConnect.formula_cx_refuse_analyzer
#print axioms QCore.MeasureConnect.formula_cx_refuse_bundles
#print axioms QCore.MeasureConnect.formula_cx_refuse_build_commit
#print axioms QCore.MeasureConnect.formula_cx_refuse_link_resolution
#print axioms QCore.MeasureConnect.formula_cx_refuse_rules_off
#print axioms QCore.MeasureConnect.formula_cx_refuse_dependency_graph
#print axioms QCore.MeasureConnect.formula_cx_refuse_inputs
#print axioms QCore.MeasureConnect.formula_cx_refuse_no_shared_route
#print axioms QCore.MeasureConnect.formula_cx_joined_shares_the_side
#print axioms QCore.MeasureConnect.formula_cx_complete_is_every_step
#print axioms QCore.MeasureConnect.formula_cx_incomplete_not_measured
#print axioms QCore.MeasureConnect.formula_cx_incomplete_status
#print axioms QCore.MeasureConnect.formula_cx_incomplete_unmatched
#print axioms QCore.MeasureConnect.formula_cx_incomplete_unmatched_units
#print axioms QCore.MeasureConnect.formula_cx_incomplete_conflicts
#print axioms QCore.MeasureConnect.formula_cx_incomplete_row_disagrees
#print axioms QCore.MeasureConnect.formula_cx_unlisted_is_unchanged
#print axioms QCore.MeasureConnect.formula_cx_net_is_first
#print axioms QCore.MeasureConnect.formula_cx_net_is_last
#print axioms QCore.MeasureConnect.formula_cx_moved_is_a_change
#print axioms QCore.MeasureConnect.formula_cx_moved_is_reported
#print axioms QCore.MeasureConnect.formula_cx_cancelled_is_reported
#print axioms QCore.MeasureConnect.formula_cx_cascade_is_reported
#print axioms QCore.MeasureConnect.formula_cx_evidence_change_is_reported
#print axioms QCore.MeasureConnect.formula_cx_reports_only
#print axioms QCore.MeasureConnect.formula_cx_new_safe_is_a_move_to_safe
#print axioms QCore.MeasureConnect.formula_cx_unattributed_move_is_listed
#print axioms QCore.MeasureConnect.formula_cx_unattributed_new_safe_is_reviewed
#print axioms QCore.MeasureConnect.formula_cx_review_is_never_accepted
#print axioms QCore.MeasureConnect.formula_cx_dependency_change_listed
#print axioms QCore.MeasureConnect.formula_cx_refuse_libc_redeclared
#print axioms QCore.MeasureConnect.formula_cx_refuse_conflicting_declarations
#print axioms QCore.MeasureConnect.formula_cx_refused_is_unknown
#print axioms QCore.MeasureConnect.formula_cx_composed_bounds_every_member
#print axioms QCore.MeasureConnect.formula_cx_composed_bounds_the_host
#print axioms QCore.MeasureConnect.formula_cx_composed_is_a_member
#print axioms QCore.MeasureConnect.formula_cx_every_reason_is_kept
#print axioms QCore.MeasureConnect.formula_cx_uncertified_plugin_is_pending
#print axioms QCore.MeasureConnect.formula_cx_checker_bounds_contribution
#print axioms QCore.MeasureConnect.formula_cx_refuse_misclassified_record
#print axioms QCore.MeasureConnect.formula_cx_satisfied_is_every_check
#print axioms QCore.MeasureConnect.formula_cx_every_check_is_satisfied
#print axioms QCore.MeasureConnect.formula_cx_unsatisfied_holds_the_dependent
#print axioms QCore.MeasureConnect.formula_cx_held_dependency_holds_dependents
#print axioms QCore.MeasureConnect.formula_cx_cycle_holds
#print axioms QCore.MeasureConnect.formula_cx_refuse_libc_target_elsewhere
#print axioms QCore.MeasureConnect.formula_cx_missing_is_unknown
#print axioms QCore.MeasureConnect.formula_cx_out_of_range_is_unknown
#print axioms QCore.MeasureConnect.formula_cx_undeclared_crate_is_unknown
#print axioms QCore.MeasureConnect.formula_cx_digest_change_is_at_least_pending
#print axioms QCore.MeasureConnect.formula_cx_cycle_is_unknown
#print axioms QCore.MeasureConnect.formula_cx_hold_is_pending_or_unknown
#print axioms QCore.MeasureConnect.formula_cx_pending_hold_is_human_resolvable
#print axioms QCore.MeasureConnect.formula_cx_hold_bounds_contribution
#print axioms QCore.MeasureConnect.formula_cx_held_is_never_safe
#print axioms QCore.MeasureConnect.cxTerms_count
#print axioms QCore.MeasureConnect.formulaTable_count
#print axioms QCore.MeasureConnect.Model.elaboration_is_the_dependency_set
#print axioms QCore.MeasureConnect.Model.link_forms_elaborate_alike
#print axioms QCore.MeasureConnect.Model.a_held_link_target_holds_its_dependent
#print axioms QCore.MeasureConnect.formula_cx_relies_is_a_deciding_read
#print axioms QCore.MeasureConnect.formula_cx_deciding_read_relies
#print axioms QCore.MeasureConnect.formula_cx_undeciding_read_not_required
#print axioms QCore.MeasureConnect.formula_cx_co_use_is_not_reliance
#print axioms QCore.MeasureConnect.formula_cx_required_is_a_cross_bundle_reliance
#print axioms QCore.MeasureConnect.formula_cx_cross_bundle_reliance_is_required
#print axioms QCore.MeasureConnect.formula_cx_same_bundle_not_required
#print axioms QCore.MeasureConnect.formula_cx_reliance_is_linked
#print axioms QCore.MeasureConnect.formula_cx_unlinked_reliance_fails_the_check
#print axioms QCore.MeasureConnect.formula_cx_link_needs_marker
#print axioms QCore.MeasureConnect.formula_cx_marker_needs_link
#print axioms QCore.MeasureConnect.formula_cx_link_is_a_dependency
#print axioms QCore.MeasureConnect.formula_cx_dependency_is_a_link
#print axioms QCore.MeasureConnect.formula_cx_refuse_libc_effect_elsewhere
#print axioms QCore.MeasureConnect.formula_cx_refuse_foreign_effect_in_libc
#print axioms QCore.MeasureConnect.formula_cx_effect_refused_is_a_misplaced_effect
#print axioms QCore.MeasureConnect.formula_cx_libc_owner_effect_accepted
#print axioms QCore.MeasureConnect.formula_cx_observation_accepted
#print axioms QCore.MeasureConnect.formula_cx_effect_refusal_fails_the_check
#print axioms QCore.MeasureConnect.formula_cx_failed_effect_check_has_a_refusal
