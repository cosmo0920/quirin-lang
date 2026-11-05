import QCore.MemberFormalType

/-! Card **LOG-PRINT-BODY-CONTRACTS** (gateway rule 12n, a fail-closed repair):
a store of a formal that is not a function pointer opens the member. Only a
function-pointer formal forwards its callers' actuals; an open unit reading,
anywhere in the run, keeps the run's closure open and the invocation is
refused; the defect (a `void *` formal read as a function pointer) would have
closed the member empty. Which C spelling is a function pointer is the
reading's extraction premise (rule 28), not modelled here. -/

#print axioms QCore.MemberFormalType.a_non_function_formal_store_is_open
#print axioms QCore.MemberFormalType.an_open_unit_keeps_the_member_open
#print axioms QCore.MemberFormalType.a_non_function_formal_store_never_decides
#print axioms QCore.MemberFormalType.the_defect_would_close_the_member_empty
