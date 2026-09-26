import QCore.RefusedCallHandOn

/-! Card **W45-FB-BACKLOG-CUSTODY** (integration review): a contracted call the
gateway refuses before its effects hands on its actuals, as W44-LEAK-PIN-AUDIT's
refused hand-on rule does for a refused statement
(`legacy_refused_operands::withdraw_refused_call`).

Audited: it is the W44 rule at a wider refusal signal; it only withdraws; a
site the call does not name keeps its leak; the named object is withdrawn;
and the node's refusal keeps the function out of `Safe`. The extraction of the
call's actuals is trust boundary 28, pinned by
`crates/qcore-c-gateway/tests/refused_call_hand_on.rs`. -/

#print axioms QCore.RefusedCallHandOn.is_the_hand_on_rule
#print axioms QCore.RefusedCallHandOn.it_only_withdraws
#print axioms QCore.RefusedCallHandOn.an_unnamed_site_keeps_its_leak
#print axioms QCore.RefusedCallHandOn.the_named_object_is_withdrawn
#print axioms QCore.RefusedCallHandOn.a_refused_call_is_never_safe
