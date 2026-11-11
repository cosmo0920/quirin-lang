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

import QCore.VerifierEvidenceV2
import QCore.VerifierPlugin

#print axioms QCore.VerifierPlugin.checked_safe_has_valid_evidence
#print axioms QCore.VerifierPlugin.safe_without_evidence_is_unknown
#print axioms QCore.VerifierPlugin.invalid_envelope_is_unknown
#print axioms QCore.VerifierPlugin.invalid_evidence_is_unknown
#print axioms QCore.VerifierPlugin.missing_plugin_composition_is_unknown
#print axioms QCore.VerifierPlugin.unknown_plugin_prevents_safe

#print axioms QCore.VerifierPlugin.rejected_proposal_is_unknown

-- E03: the multi-provider composition relation.
#print axioms QCore.VerifierPlugin.Composition.disjoint_certificates_merge
#print axioms QCore.VerifierPlugin.Composition.duplicate_certificate_still_merges
#print axioms QCore.VerifierPlugin.Composition.composition_is_never_a_violation
#print axioms QCore.VerifierPlugin.Composition.alleged_component_is_unknown
#print axioms QCore.VerifierPlugin.Composition.unknown_component_is_unknown
#print axioms QCore.VerifierPlugin.Composition.no_outcomes_is_unknown
#print axioms QCore.VerifierPlugin.Composition.empty_inventory_is_unknown
#print axioms QCore.VerifierPlugin.Composition.safe_covers_every_required_obligation
#print axioms QCore.VerifierPlugin.Composition.uncovered_obligation_is_unknown
#print axioms QCore.VerifierPlugin.Composition.duplicate_certificate_adds_no_coverage
#print axioms QCore.VerifierPlugin.Composition.duplicate_evidence_cannot_cover_a_missing_obligation
#print axioms QCore.VerifierPlugin.Composition.distinct_unit_identities_are_unknown
#print axioms QCore.VerifierPlugin.Composition.incomparable_bounds_are_unknown

-- E03-B1: the published composite replaces the component representative.
-- `distinct_premise_sets_are_unknown` and `safe_premises_are_uniform` are
-- deliberately retired: they stated the pre-repair refusal of unequal but
-- compatible premise sets, which the composite premise union supersedes.
#print axioms QCore.VerifierPlugin.Composition.conflicting_premise_values_are_unknown
#print axioms QCore.VerifierPlugin.Composition.unequal_premise_sets_merge
#print axioms QCore.VerifierPlugin.Composition.uniform_premises_still_merge
#print axioms QCore.VerifierPlugin.Composition.composite_publishes_every_component_premise
#print axioms QCore.VerifierPlugin.Composition.composite_premises_come_from_components
#print axioms QCore.VerifierPlugin.Composition.safe_premises_are_consistent
#print axioms QCore.VerifierPlugin.Composition.representative_is_every_component
#print axioms QCore.VerifierPlugin.Composition.distinct_components_have_no_representative

-- E03-B2: unit identity names the artifact, not the capability.
#print axioms QCore.VerifierPlugin.Composition.capabilities_of_one_artifact_share_a_unit
#print axioms QCore.VerifierPlugin.Composition.distinct_artifacts_have_distinct_units
#print axioms QCore.VerifierPlugin.Composition.obligation_keys_stay_capability_distinct

-- E03-B3: provider-indexed coverage obligations.
#print axioms QCore.VerifierPlugin.Composition.distinct_providers_have_distinct_coverage_keys
#print axioms QCore.VerifierPlugin.Composition.distinct_denominators_have_distinct_coverage_keys
#print axioms QCore.VerifierPlugin.Composition.one_providers_certificate_leaves_the_others_key_uncovered

-- E03-B4: an absent provider refuses the composed verdict itself.
#print axioms QCore.VerifierPlugin.Composition.absent_provider_scope_is_unknown
#print axioms QCore.VerifierPlugin.Composition.present_scope_is_the_merge
#print axioms QCore.VerifierPlugin.Composition.honest_scope_still_composes
#print axioms QCore.VerifierPlugin.Composition.honest_scope_with_an_absent_provider_refuses

-- E03-B-O1: an absent provider's obligations are derived where the derivation
-- reads nothing from the manifest, and withheld with a reason where it does.
#print axioms QCore.VerifierPlugin.Composition.absent_derivation_states_the_installed_inventory
#print axioms QCore.VerifierPlugin.Composition.absent_derivation_withholds_a_manifest_dependent_inventory
#print axioms QCore.VerifierPlugin.Composition.the_measured_capability_table_is_not_vacuous
#print axioms QCore.VerifierPlugin.Composition.stated_absent_obligations_only_refuse
#print axioms QCore.VerifierPlugin.Composition.absent_provider_scope_is_unknown_however_much_is_stated

-- E03-B-O2: composition is associative under its own output.
#print axioms QCore.VerifierPlugin.Composition.collect_safe
#print axioms QCore.VerifierPlugin.Composition.flatten_leaves
#print axioms QCore.VerifierPlugin.Composition.regrouping_is_the_same_component_sequence
#print axioms QCore.VerifierPlugin.Composition.composition_is_associative
#print axioms QCore.VerifierPlugin.Composition.regrouped_components_merge
#print axioms QCore.VerifierPlugin.Composition.regrouping_cannot_hide_a_conflict
#print axioms QCore.VerifierPlugin.Composition.re_submitting_a_composite_adds_no_coverage

-- E03-B-O3: the tightest-envelope intersection, kind by kind.
#print axioms QCore.VerifierPlugin.Composition.tighter_paths_is_a_lower_bound
#print axioms QCore.VerifierPlugin.Composition.tighter_count_is_a_lower_bound
#print axioms QCore.VerifierPlugin.Composition.the_envelope_is_a_lower_bound
#print axioms QCore.VerifierPlugin.Composition.envelope_is_commutative
#print axioms QCore.VerifierPlugin.Composition.envelope_is_idempotent
#print axioms QCore.VerifierPlugin.Composition.a_single_path_absorbs
#print axioms QCore.VerifierPlugin.Composition.an_absent_count_does_not_relax
#print axioms QCore.VerifierPlugin.Composition.recorded_runs_are_contagious
#print axioms QCore.VerifierPlugin.Composition.every_count_kind_takes_the_minimum
#print axioms QCore.VerifierPlugin.Composition.pathsAtMost_refl
#print axioms QCore.VerifierPlugin.Composition.countAtMost_refl
#print axioms QCore.VerifierPlugin.Composition.atMost_refl
#print axioms QCore.VerifierPlugin.Composition.pathsAtMost_trans
#print axioms QCore.VerifierPlugin.Composition.countAtMost_trans
#print axioms QCore.VerifierPlugin.Composition.atMost_trans
#print axioms QCore.VerifierPlugin.Composition.fold_is_a_lower_bound
#print axioms QCore.VerifierPlugin.Composition.the_published_envelope_bounds_every_component
#print axioms QCore.VerifierPlugin.Composition.an_empty_composition_states_no_envelope

#print axioms QCore.VerifierEvidenceV2.accepted_has_complete_nonempty_inventory
#print axioms QCore.VerifierEvidenceV2.accepted_hook_has_recomputed_observation
#print axioms QCore.VerifierEvidenceV2.incomplete_inventory_rejects
#print axioms QCore.VerifierEvidenceV2.empty_inventory_rejects
#print axioms QCore.VerifierEvidenceV2.false_precondition_never_counts

#print axioms QCore.VerifierEvidenceV2.accepted_requires_observation_trust_and_bounds
#print axioms QCore.VerifierEvidenceV2.explicit_refusal_is_preserved
#print axioms QCore.VerifierEvidenceV2.explicit_refusal_derivation_rejects
#print axioms QCore.VerifierEvidenceV2.absent_flag_names_conditional_trust

#print axioms QCore.VerifierEvidenceV2.accepted_claim_preserves_scope_bounds_and_module
#print axioms QCore.VerifierEvidenceV2.substituted_claim_rejects

#print axioms QCore.VerifierPlugin.ProcessCompletion.failed_exit_precedes_input_failure
#print axioms QCore.VerifierPlugin.ProcessCompletion.partial_request_never_decodes
#print axioms QCore.VerifierPlugin.ProcessCompletion.decoding_requires_success_and_complete_input
