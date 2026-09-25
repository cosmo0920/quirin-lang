import QCore.CalleeSummaryScope

/-! Card **W44-CORPUS-CALLEE-SUMMARIES**: the W4 derivation scope for
corpus-defined callees (`crates/qcore-c-gateway/src/legacy_callee_scope.rs`).

Five directions are audited. **The scope admits only what linkage pins**: every
member is selected or unreserved and of internal linkage, an unselected external
or implementation-reserved definition is never in scope, and the selection
always is. **An admitted callee's calls run
its analysed body**, and the linkage premise is shown necessary by a body an
external definition's call may run instead. **Declarations win and inferences
are installed exactly**, and anything inferred that is installed was validated
in scope. **Widening only adds summaries**: a summary derived under a smaller
environment survives under any extension of it. A witness completes it.

What this is not: a proof about the Rust readers (trust boundary 28's
correspondence premise), or about the selected definitions' own pre-existing
call-resolution premise. -/

-- The scope admits only what linkage pins.
#print axioms QCore.CalleeSummaryScope.every_scope_member_is_selected_or_internal
#print axioms QCore.CalleeSummaryScope.an_unselected_external_definition_is_never_in_scope
#print axioms QCore.CalleeSummaryScope.an_unselected_reserved_definition_is_never_in_scope
#print axioms QCore.CalleeSummaryScope.the_selection_is_in_scope

-- An admitted callee's calls run its analysed body; the premise is necessary.
#print axioms QCore.CalleeSummaryScope.an_admitted_callee_runs_only_its_analysed_body
#print axioms QCore.CalleeSummaryScope.an_external_definition_may_run_another_body

-- Declarations win; inferences are installed exactly and were validated.
#print axioms QCore.CalleeSummaryScope.a_declaration_is_never_overridden
#print axioms QCore.CalleeSummaryScope.an_installed_inference_is_exactly_the_derived_contract
#print axioms QCore.CalleeSummaryScope.an_installed_inference_was_validated_in_scope

-- Widening only adds summaries.
#print axioms QCore.CalleeSummaryScope.an_existing_summary_survives_the_widening

-- Witness.
#print axioms QCore.CalleeSummaryScope.the_witness_scope
