import QCore.ReturnExpressionEvaluation

-- Card B01-RETURN-EXPRESSION. Rust model:
-- `Parser::return_expression_instruction` in
-- `crates/qcore-c-gateway/src/legacy_c.rs`; behavioural pins:
-- `crates/qcore-c-gateway/tests/return_position_modeled_call.rs`.

#print axioms QCore.ReturnExpressionEvaluation.an_admitted_call_is_unconditional
#print axioms QCore.ReturnExpressionEvaluation.an_admitted_call_has_no_synthetic_collision
#print axioms QCore.ReturnExpressionEvaluation.a_conditional_call_stays_refused
#print axioms QCore.ReturnExpressionEvaluation.a_collision_stays_refused
#print axioms QCore.ReturnExpressionEvaluation.an_admitted_return_is_exactly_the_call_effect
#print axioms QCore.ReturnExpressionEvaluation.a_preserved_obligation_remains_visible
#print axioms QCore.ReturnExpressionEvaluation.the_positive_family_is_admitted
#print axioms QCore.ReturnExpressionEvaluation.a_declared_release_is_applied
