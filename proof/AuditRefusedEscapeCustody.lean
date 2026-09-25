import QCore.RefusedEscapeCustody

/-! Card **W44-LEAK-PIN-AUDIT**: the refused-escape rule. A statement the
projection refuses whose value side (a store's right-hand side or a `return`
operand) hands a live identity on leaves that identity with an unresolved
effect, the projection's existing reading of "an unmodelled action may have
discharged it".

Five directions are audited. **Only unresolved effects grow**: no status and no
exemption changes, and an admitted node is untouched. **Only withdrawal**: a
leak reported after the rule was reported before it; a site the statement did
not hand on keeps its report exactly; a handed-on live site is withdrawn.
**Composition**: the rule distributes over the union join. **No `Safe`**: the
rule fires only where the node reported `Unsupported`, so a fully withdrawn
function is `Unknown`. **Stated collateral**: the union join withdraws a clean
path joined with an escaped one, which the card measures leaf by leaf.

What this is not: a proof about the Rust (trust boundary 28), or about what an
uncontracted callee does with an argument, which the rule deliberately leaves
to `REFUSED-CALL-ARGUMENT-PREMISE`. A remaining leak is a provisional candidate,
never an upstream defect. -/

#print axioms QCore.RefusedEscapeCustody.the_rule_only_grows_unresolved
#print axioms QCore.RefusedEscapeCustody.the_rule_changes_no_status
#print axioms QCore.RefusedEscapeCustody.an_admitted_node_is_untouched
#print axioms QCore.RefusedEscapeCustody.the_rule_only_withdraws
#print axioms QCore.RefusedEscapeCustody.an_unhanded_site_keeps_its_report
#print axioms QCore.RefusedEscapeCustody.a_refused_escape_is_withdrawn
#print axioms QCore.RefusedEscapeCustody.the_rule_distributes_over_the_join
#print axioms QCore.RefusedEscapeCustody.a_withdrawal_by_the_rule_is_never_safe
#print axioms QCore.RefusedEscapeCustody.a_fully_withdrawn_function_is_unknown
#print axioms QCore.RefusedEscapeCustody.the_union_join_withdraws_a_clean_path
