import QCore.PremiseFormula.PluginGuardedSymbols

/-! Card **SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS** (C04.18.6, C04.18.9 SP9
block): the 14 guarded symbols of the frozen plugin sidecar as
typed-rendering formulas over the prefixed SP9 terms (user decisions
2026-11-23: D1 A′, D2 b, D3 I2).

In the rendering of an invocation model, each symbol holds exactly when its
definition does:

* the seven prerequisites are `PluginPremiseDecomposition`'s `loadedArtifacts`,
  `callbackCandidates`, `loadedLifetime`, `loadedCustody`,
  `loadedRegistration`, `loadedUnload` and `outerBridge`, which together are
  `CallbackRefinement`;
* the cases are `registeredCase` and `defaultCase`;
* `registered_lifetime_valid` is `lifetimePostcondition`;
* `default_lifetime_valid` is the new `defaultLifetime` (D2 b). It is
  strictly weaker than `lifetimePostcondition`.

The admission, the trace correspondence and the refutation of registration
are never derived. The refutation entails the existing `unregistered`, while
a failed registration proof alone does not give it (D3 I2).

The Rust boundary is pinned by
`crates/qcore-contract/tests/plugin_guarded_symbols.rs`
(`the_symbols_are_the_lean_definitions`,
`the_sp9_block_is_new_prefixed_and_bound_to_lean`,
`each_prerequisite_is_its_production_atom_bound_to_the_sp9_terms`). -/

-- The seven prerequisites: their atoms' existing definitions.
#print axioms QCore.PluginGuardedSymbols.artifact_symbol_abi_holds
#print axioms QCore.PluginGuardedSymbols.callback_candidates_holds
#print axioms QCore.PluginGuardedSymbols.code_context_lifetime_holds
#print axioms QCore.PluginGuardedSymbols.interceptor_custody_holds
#print axioms QCore.PluginGuardedSymbols.interceptor_registration_holds
#print axioms QCore.PluginGuardedSymbols.lifecycle_unload_holds
#print axioms QCore.PluginGuardedSymbols.outer_bridge_holds
#print axioms QCore.PluginGuardedSymbols.prerequisites_are_the_callback_refinement

-- The seven symbols that were English.
#print axioms QCore.PluginGuardedSymbols.environment_holds
#print axioms QCore.PluginGuardedSymbols.invoke_holds
#print axioms QCore.PluginGuardedSymbols.registered_case_holds
#print axioms QCore.PluginGuardedSymbols.default_case_holds
#print axioms QCore.PluginGuardedSymbols.registered_lifetime_valid_holds
#print axioms QCore.PluginGuardedSymbols.default_lifetime_valid_holds
#print axioms QCore.PluginGuardedSymbols.unregistered_holds
#print axioms QCore.PluginGuardedSymbols.plugin_guarded_symbols_are_the_definitions

-- The alternatives, the two lifetimes (D2 b) and the independence of the cases.
#print axioms QCore.PluginGuardedSymbols.candidates_give_a_case
#print axioms QCore.PluginGuardedSymbols.lifetime_entails_default_lifetime
#print axioms QCore.PluginGuardedSymbols.default_lifetime_does_not_give_the_unload_order
#print axioms QCore.PluginGuardedSymbols.default_case_is_independent_of_registration

-- The assumptions no model derives (user decisions 2026-11-23).
#print axioms QCore.PluginGuardedSymbols.unregistered_symbol_entails_unregistered
#print axioms QCore.PluginGuardedSymbols.a_case_excludes_refutation
#print axioms QCore.PluginGuardedSymbols.trace_corresponds_is_never_derived
#print axioms QCore.PluginGuardedSymbols.registration_refutation_is_never_derived
#print axioms QCore.PluginGuardedSymbols.admission_is_never_derived
#print axioms QCore.PluginGuardedSymbols.absence_of_registration_is_not_refutation
