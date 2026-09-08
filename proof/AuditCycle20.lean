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
