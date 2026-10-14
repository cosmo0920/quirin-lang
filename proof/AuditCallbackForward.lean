import QCore.CallbackForward

/-! Audited companion to SERVE-FORWARDED-CALLBACK-FORMALS (gateway rule 15w): a callback formal handed on unchanged to the same clause position of a callee satisfies the caller's own declaration exactly when the callee's clause is in force and corresponds; every invocation the callee makes under its clause meets the caller's clause; a changed formal, a local use beside a forward, a callee not in force or non-corresponding actuals keep the refusal; and a recursive forward alone never stands under the least fixed point. -/

#print axioms QCore.CallbackForward.forwarded_invocations_meet_the_caller_clause
#print axioms QCore.CallbackForward.an_exact_forward_stands_iff_the_callee_clause_is_in_force
#print axioms QCore.CallbackForward.a_forward_to_a_callee_not_in_force_refuses
#print axioms QCore.CallbackForward.a_forward_that_does_not_correspond_refuses
#print axioms QCore.CallbackForward.a_changed_formal_refuses
#print axioms QCore.CallbackForward.any_other_mention_refuses
#print axioms QCore.CallbackForward.no_mention_refuses
#print axioms QCore.CallbackForward.a_recursive_forward_alone_never_stands
#print axioms QCore.CallbackForward.forward_vectors_hold
