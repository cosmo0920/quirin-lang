import QCore.LocalTypedAtoms

/-! Card **SP6-SYMBOLIC-HYPOTHESIS-ATOMS** (C04.18.7): the source-local
hypotheses of `quirin-source-analysis::cjson_local_binding` (three finite
native-domain atoms) and `quirin-source-analysis::plugin_source_witnesses`
(four fixture-trace atoms) as typed atoms. Each, under the interpretation by
its bound model, holds exactly when the bound proposition does.

The Rust boundary is pinned by the `typed_atoms` unit tests of both modules
(`the_finite_atoms_are_the_lean_definitions`,
`the_trace_atoms_are_the_lean_definitions`). -/

-- The cJSON native domain (QCore.LocalSourceBindings).
#print axioms QCore.LocalTypedAtoms.finite_detach_post_atom_iff
#print axioms QCore.LocalTypedAtoms.finite_entry_equations_atom_iff
#print axioms QCore.LocalTypedAtoms.incomplete_fixed_case_domain_atom_iff

-- The plugin fixture traces (QCore.PluginSourceWitnesses).
#print axioms QCore.LocalTypedAtoms.indexed_marker_iff
#print axioms QCore.LocalTypedAtoms.last_marker_iff
#print axioms QCore.LocalTypedAtoms.duplicate_no_loader_calls_atom_iff
#print axioms QCore.LocalTypedAtoms.duplicate_completed_atom_iff
#print axioms QCore.LocalTypedAtoms.go_creator_completed_atom_iff
#print axioms QCore.LocalTypedAtoms.go_creator_implies_registration_atom_iff
