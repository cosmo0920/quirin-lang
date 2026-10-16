import QCore.ReturnCodeHandOff

/-! Audited companion to SUPPLIED-OWNED-RETURN-CODE-HANDOFF (gateway rules 12m and 13l, contract specification C04.19.5): ownership after the call is a function of the result and the guard; a caller branch the check admits never touches an object the callback took, and leaves the caller what the guard does not admit, composing through a forwarder; a guard the caller does not branch on never admits; an alias means its `@` name and an undeclared one rejects; the `#absent` migration preserves meaning and a bare `absent` rejects. -/

#print axioms QCore.ReturnCodeHandOff.owner_after_is_the_guard
#print axioms QCore.ReturnCodeHandOff.owner_after_eq
#print axioms QCore.ReturnCodeHandOff.owner_after_ne
#print axioms QCore.ReturnCodeHandOff.an_admitted_branch_never_touches_a_taken_object
#print axioms QCore.ReturnCodeHandOff.the_caller_keeps_what_the_guard_leaves
#print axioms QCore.ReturnCodeHandOff.an_unbranched_guard_never_admits
#print axioms QCore.ReturnCodeHandOff.a_converted_result_never_admits
#print axioms QCore.ReturnCodeHandOff.an_admitted_branch_never_converts_the_result
#print axioms QCore.ReturnCodeHandOff.admission_composes_through_a_forwarder
#print axioms QCore.ReturnCodeHandOff.an_alias_means_its_name
#print axioms QCore.ReturnCodeHandOff.an_undeclared_alias_rejects
#print axioms QCore.ReturnCodeHandOff.the_migration_preserves_absent
#print axioms QCore.ReturnCodeHandOff.a_bare_absent_rejects
#print axioms QCore.ReturnCodeHandOff.a_function_designator_is_unchanged
#print axioms QCore.ReturnCodeHandOff.owner_vectors_hold
#print axioms QCore.ReturnCodeHandOff.branch_vectors_hold
