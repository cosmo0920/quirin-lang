import QCore.FormalBindingEnvironments

/-! Card **SP10-FORMAL-BINDING-DESCRIPTIONS** (C04.18.2, C04.18.3, C04.18.9
SP10 block): binding manifests and flow models name their environments by
typed terms, never by English descriptions.

Every SP10 term has a Lean meaning over `QCore.PluginPremiseDecomposition`
and `HypothesisGraph.Context`; every recorded reference resolves, and every
plugin and detach atom parameter belongs to a recorded component. The source
correspondence is never derived (E2's form). A text value is not an
environment reference and an English quantifier or conclusion is not a
binding field (QF005).

The Rust boundary is pinned by
`crates/qcore-contract/tests/formal_binding_descriptions.rs`
(`the_sp10_block_is_new_typed_and_bound_to_lean`,
`the_recorded_references_are_the_lean_table`,
`the_environment_decision_vectors_are_the_lean_table`). -/

#print axioms QCore.FormalBindingEnvironments.sp10_names_are_prefixed
#print axioms QCore.FormalBindingEnvironments.sp10_is_disjoint_from_sp9
#print axioms QCore.FormalBindingEnvironments.sp10_terms_denote
#print axioms QCore.FormalBindingEnvironments.unregistered_term_denotes_nothing
#print axioms QCore.FormalBindingEnvironments.proper_list_flag
#print axioms QCore.FormalBindingEnvironments.forged_flag
#print axioms QCore.FormalBindingEnvironments.frame_terms_are_model_actions
#print axioms QCore.FormalBindingEnvironments.binding_source_correspondence_is_never_derived
#print axioms QCore.FormalBindingEnvironments.every_reference_resolves
#print axioms QCore.FormalBindingEnvironments.plugin_parameters_are_covered
#print axioms QCore.FormalBindingEnvironments.detach_parameters_are_covered
#print axioms QCore.FormalBindingEnvironments.text_is_not_an_environment_reference
#print axioms QCore.FormalBindingEnvironments.environment_admitted_iff
#print axioms QCore.FormalBindingEnvironments.environment_vectors_agree
#print axioms QCore.FormalBindingEnvironments.text_is_not_a_binding_field
#print axioms QCore.FormalBindingEnvironments.binding_admitted_iff
