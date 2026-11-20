import QCore.PremiseFormula.Librdkafka

/-! Card **SP2-SYMBOLIC-LIBRDKAFKA-PREMISES** (C04.18.9): the 220
`resource_premise` clauses of `qcore-provider-librdkafka` as sixteen formulas
under twelve names, over the SP2 block of the closed vocabulary.

The sixteen statements are well-formed; `#false` is the falsum; where the
prose says "so" or "because", the stated consequence follows from the stated
prerequisites in every coherent model; the print statement of
`cjson_allocator_hooks` entails its cJSON_New_Item statement; the
`detach_list_representation` formula holds exactly when the nine
`QCore.CjsonRepresentationPremises` propositions of its decomposition do. The base
judgment's theorems (`check_denotes`, `decision_vectors_agree`) cover the
extended vocabulary and decision vectors (`AuditPremiseFormula.lean`).

The Rust boundary is pinned by
`crates/qcore-frontend/tests/premise_formula_librdkafka.rs`
(`the_librdkafka_formulas_are_the_lean_definitions`) and
`crates/qcore-frontend/tests/premise_formula.rs`
(`the_vocabulary_is_the_lean_table`, `the_decision_vectors_are_the_lean_table`). -/

-- The sixteen statements.
#print axioms QCore.PremiseFormula.librdkafka_formulas_wf

-- The falsum.
#print axioms QCore.PremiseFormula.false_negates
#print axioms QCore.PremiseFormula.quant_false_empty

-- asserted_allocation: the stated consequence follows.
#print axioms QCore.PremiseFormula.eval_asserted_calloc
#print axioms QCore.PremiseFormula.returned_from_asserting_nonnull
#print axioms QCore.PremiseFormula.asserted_calloc_prerequisites_suffice
#print axioms QCore.PremiseFormula.eval_asserted_malloc
#print axioms QCore.PremiseFormula.asserted_malloc_prerequisites_suffice
#print axioms QCore.PremiseFormula.eval_asserted_toppar
#print axioms QCore.PremiseFormula.asserted_toppar_prerequisites_suffice

-- share_ack_batches_nonempty: the stated consequence follows.
#print axioms QCore.PremiseFormula.eval_share_ack
#print axioms QCore.PremiseFormula.share_ack_prerequisites_suffice

-- rk_log_enqueue_no_event_callback: the stated consequence follows.
#print axioms QCore.PremiseFormula.eval_log_no_event
#print axioms QCore.PremiseFormula.log_no_event_prerequisites_suffice

-- cjson_allocator_hooks: the print statement widens the cJSON_New_Item one.
#print axioms QCore.PremiseFormula.hooks_print_entails_new_item

-- detach_list_representation: the formula is the nine atoms' Lean propositions.
#print axioms QCore.PremiseFormula.eval_detach
#print axioms QCore.PremiseFormula.option_eq_of_values
#print axioms QCore.PremiseFormula.detach_formula_is_the_nine_atoms
