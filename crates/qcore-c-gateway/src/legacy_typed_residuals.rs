//! Card **W47-ACCESS-PATH-POSITION-RESIDUALS** (gateway rule 10n): the P3
//! position residual shapes that C's evaluation order and the callee's
//! custody premise admit, each a fallback asked after every older route of
//! its position (and rule 10k's typed reader) refused.
//!
//! * **A connective return** (step 2): `return E;` whose Clang node is a
//!   top-level `&&`, `||` or `?:` of scalar type, carrying no call, no
//!   assignment, no increment and no tracked descriptor. It is lowered to
//!   what C evaluates: `E` desugared exactly as rule 5h desugars a
//!   controlling expression (each operand only on the edges where C
//!   evaluates it, a `?:` arm only on its condition's edge), with **both**
//!   exits joined into one `return` whose exit is `NoOp`. The value is a
//!   scalar (`&&`/`||` yield `int`; a scalar `?:` its arms' common type), so
//!   nothing is escaped or handed back. A pointer-typed `?:` keeps the
//!   refusal: its value is handed back, which rule 5b's premises decide.
//!
//! The anonymous inline enumeration member (step 1) is a scalar-evidence arm
//! of the reader itself, in [`crate::typed_position_shapes`].

use super::condition_desugar::Position as ConditionPosition;
use super::{Parser, Statement, Token, contains_braces, contains_call, descriptor_custody};
use crate::typed_access_path::{is_scalar, record_row};
use crate::typed_c_ast::{children, kind};

/// Step 2 of rule 10n: a scalar connective return is desugared.
pub(crate) const RETURN_CONNECTIVES: bool = true;

impl Parser<'_> {
    /// Hook: the last route of a `return` expression, after every older route
    /// and rule 10k's typed return refused. `Some` is the desugared
    /// evaluation followed by a `NoOp` return; `None` keeps the refusal.
    pub(super) fn typed_connective_return(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        if !RETURN_CONNECTIVES
            || expression.is_empty()
            || contains_call(expression)
            || contains_braces(expression)
            || self.opaque_callee(expression)
        {
            return None;
        }
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        // No assignment, compound assignment, increment or decrement (the
        // tokens are single characters: `==` is `=` `=`, `+=` is `+` `=`).
        if (0..texts.len()).any(|index| is_assignment(&texts, index))
            || texts
                .windows(2)
                .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
        {
            return None;
        }
        if descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names).is_some() {
            return None;
        }
        let first = expression.first()?.line;
        let index = self.typed_positions_index()?;
        let node = index.expression_node(&texts, first)?;
        let mut top = node;
        while kind(top) == "ParenExpr" {
            top = children(top).first()?;
        }
        let connective = match kind(top) {
            "BinaryOperator" => matches!(top["opcode"].as_str(), Some("&&" | "||")),
            "ConditionalOperator" => true,
            _ => false,
        };
        if !connective || !is_scalar(top) {
            return None;
        }
        let uses = self.typed_position_uses.get();
        let plan = self.condition_plan(expression, ConditionPosition::Branch);
        let typed_operands = self.typed_position_uses.get() - uses;
        self.typed_position_uses.set(uses);
        if crate::typed_access_path::parity_enabled() {
            record_row(serde_json::json!({
                "function": self.typed_path_function,
                "line": first,
                "texts": texts,
                "position": "return-connective",
                "old_route": "old_refusal",
                "old_admitted": false,
                "new_admitted": plan.is_ok(),
                "typed_operands": typed_operands,
            }));
        }
        let plan = plan.ok()?;
        let writes = self.expression_writes(expression);
        Some(Statement::Block(vec![
            Statement::If {
                line,
                condition_supported: false,
                condition_call: None,
                condition_access: None,
                writes: writes.clone(),
                null_condition: None,
                distinct_condition: None,
                element_condition: None,
                condition_plan: Some(Box::new(plan)),
                cell_test: None,
                then_body: Vec::new(),
                else_body: Vec::new(),
            },
            Statement::Return {
                line,
                expression_supported: true,
                writes,
                returned_place: None,
                returned_evaluation: None,
                returned_evaluation_escapes: false,
                returns_null_constant: false,
            },
        ]))
    }
}

/// Rule 10k's assignment test (`legacy_typed_positions::is_assignment`): an
/// `=` that is neither half of `==`, `!=`, `<=`, `>=`; `<<=` and `>>=` are
/// assignments.
fn is_assignment(texts: &[&str], index: usize) -> bool {
    if texts[index] != "=" {
        return false;
    }
    let previous = index.checked_sub(1).and_then(|i| texts.get(i)).copied();
    let next = texts.get(index + 1).copied();
    next != Some("=")
        && (!matches!(previous, Some("=" | "!" | "<" | ">"))
            || (matches!(previous, Some("<" | ">"))
                && index.checked_sub(2).and_then(|i| texts.get(i)).copied() == previous))
}
