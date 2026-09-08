import QCore.PackageTemplateBinding

-- Card P01-F2: binding an installed manifest to the template embedded in the
-- package executable, without executing it. Model of
-- `crates/qcore-verifier-host/src/embedded_template.rs`; threat model, field
-- coverage and refusal matrix in `docs/reports/remaining-p01-f2/README.md`.

-- Occurrence counting, the basis of the exactly-one-envelope rule.
#print axioms QCore.PackageTemplateBinding.count_append
#print axioms QCore.PackageTemplateBinding.count_eq_zero_of_markerless

-- Locating the envelope in the executable's bytes.
#print axioms QCore.PackageTemplateBinding.afterOpening_of_markerless
#print axioms QCore.PackageTemplateBinding.beforeClosing_of_markerless
#print axioms QCore.PackageTemplateBinding.extract_envelope
#print axioms QCore.PackageTemplateBinding.extract_absent
#print axioms QCore.PackageTemplateBinding.extract_duplicate_is_ambiguous
#print axioms QCore.PackageTemplateBinding.extract_unterminated_is_refused
#print axioms QCore.PackageTemplateBinding.extract_reversed_is_refused

-- An accepted payload is always the executable's own bracketed run.
#print axioms QCore.PackageTemplateBinding.beforeClosing_bracketed
#print axioms QCore.PackageTemplateBinding.locate_bracketed
#print axioms QCore.PackageTemplateBinding.extract_payload_locate
#print axioms QCore.PackageTemplateBinding.extract_payload_is_bracketed

-- The declared field inventory and the first-divergence search.
#print axioms QCore.PackageTemplateBinding.mem_allFields
#print axioms QCore.PackageTemplateBinding.firstDivergence_none_iff
#print axioms QCore.PackageTemplateBinding.firstDivergence_some

-- What the binding decides, in both directions.
#print axioms QCore.PackageTemplateBinding.bind_eq_bound_iff
#print axioms QCore.PackageTemplateBinding.divergent_names_a_real_divergence
#print axioms QCore.PackageTemplateBinding.every_field_is_bound
#print axioms QCore.PackageTemplateBinding.the_template_digest_is_not_consulted
#print axioms QCore.PackageTemplateBinding.a_digest_edit_is_refused

-- Fail-closed: no envelope, no template and no agreement ever verify.
#print axioms QCore.PackageTemplateBinding.no_envelope_never_binds
#print axioms QCore.PackageTemplateBinding.a_malformed_template_never_binds
#print axioms QCore.PackageTemplateBinding.a_digest_intact_declaration_edit_never_binds
#print axioms QCore.PackageTemplateBinding.a_stripped_envelope_is_named_absent
#print axioms QCore.PackageTemplateBinding.a_second_envelope_is_named_ambiguous
#print axioms QCore.PackageTemplateBinding.refusals_are_not_acceptance

-- The honest package still verifies: the gate is not vacuously closed.
#print axioms QCore.PackageTemplateBinding.an_unmodified_package_binds
