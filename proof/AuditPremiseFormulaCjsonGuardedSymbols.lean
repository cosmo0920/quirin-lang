import QCore.PremiseFormula.CjsonGuardedSymbols

/-! Card **SP6-GUARDED-SYMBOL-PROSE** (C04.18.6, C04.18.9 SP7 block): the 17
guarded symbols of `cJSON_DetachItemViaPointer` as typed-rendering formulas.

In the rendering of a call model, each symbol holds exactly when its
definition does:

* the nine prerequisites are their `CjsonRepresentationPremises`
  propositions at entry;
* `failure_case` is `rawFailure`;
* `original_list_unchanged` and `remaining_list_valid` are the existing
  postconditions;
* `success_case` is the prerequisites and the returned item;
* `membership` is the call's membership conjuncts.

The two assumptions the user decided on 2026-11-23 are never derived:
`#source_corresponds` (environment E2) and the caller's refutation
(invalid_membership I2). The refutation entails the existing
`invalidMembership`, while non-ownership alone does not give it. A string is
not a guarded proposition (QF003).

The Rust boundary is pinned by
`crates/qcore-contract/tests/cjson_guarded_symbols.rs`
(`the_symbols_are_the_lean_definitions`, `the_sp7_block_is_new_typed_and_bound_to_lean`,
`the_guarded_decision_vectors_are_the_lean_table`). -/

-- The nine prerequisites at entry, and over the exit terms.
#print axioms QCore.CjsonGuardedSymbols.chain_holds
#print axioms QCore.CjsonGuardedSymbols.distinct_holds
#print axioms QCore.CjsonGuardedSymbols.empty_holds
#print axioms QCore.CjsonGuardedSymbols.first_prev_holds
#print axioms QCore.CjsonGuardedSymbols.last_next_holds
#print axioms QCore.CjsonGuardedSymbols.link_targets_holds
#print axioms QCore.CjsonGuardedSymbols.no_writer_holds
#print axioms QCore.CjsonGuardedSymbols.parent_outside_holds
#print axioms QCore.CjsonGuardedSymbols.predecessor_holds
#print axioms QCore.CjsonGuardedSymbols.exit_chain_holds
#print axioms QCore.CjsonGuardedSymbols.exit_distinct_holds
#print axioms QCore.CjsonGuardedSymbols.exit_empty_holds
#print axioms QCore.CjsonGuardedSymbols.exit_first_prev_holds
#print axioms QCore.CjsonGuardedSymbols.exit_last_next_holds
#print axioms QCore.CjsonGuardedSymbols.exit_link_targets_holds
#print axioms QCore.CjsonGuardedSymbols.exit_no_writer_holds
#print axioms QCore.CjsonGuardedSymbols.exit_parent_outside_holds
#print axioms QCore.CjsonGuardedSymbols.exit_predecessor_holds
#print axioms QCore.CjsonGuardedSymbols.nine_entry_holds
#print axioms QCore.CjsonGuardedSymbols.nine_exit_holds

-- The eight symbols that were English.
#print axioms QCore.CjsonGuardedSymbols.detach_holds
#print axioms QCore.CjsonGuardedSymbols.environment_holds
#print axioms QCore.CjsonGuardedSymbols.failure_case_holds
#print axioms QCore.CjsonGuardedSymbols.membership_holds
#print axioms QCore.CjsonGuardedSymbols.invalid_membership_holds
#print axioms QCore.CjsonGuardedSymbols.original_list_unchanged_holds
#print axioms QCore.CjsonGuardedSymbols.remaining_list_valid_holds
#print axioms QCore.CjsonGuardedSymbols.prerequisites_iff
#print axioms QCore.CjsonGuardedSymbols.success_case_holds
#print axioms QCore.CjsonGuardedSymbols.guarded_symbols_are_the_definitions

-- The two assumptions no model derives (user decisions 2026-11-23).
#print axioms QCore.CjsonGuardedSymbols.invalid_symbol_entails_invalid_membership
#print axioms QCore.CjsonGuardedSymbols.membership_excludes_refutation
#print axioms QCore.CjsonGuardedSymbols.source_corresponds_is_never_derived
#print axioms QCore.CjsonGuardedSymbols.caller_refutation_is_never_derived
#print axioms QCore.CjsonGuardedSymbols.absence_of_membership_is_not_refutation

-- A string is not a guarded proposition (QF003).
#print axioms QCore.CjsonGuardedSymbols.string_is_not_a_guarded_proposition
#print axioms QCore.CjsonGuardedSymbols.guarded_admitted_iff_typed
#print axioms QCore.CjsonGuardedSymbols.guarded_vectors_agree
