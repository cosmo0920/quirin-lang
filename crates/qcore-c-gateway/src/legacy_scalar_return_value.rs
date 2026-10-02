//! Card **W48-BOTTOM-UP-CALLEE-BODIES** (gateway rule 12f): a scalar
//! `return` value read as the value of a statement.
//!
//! # Where
//!
//! One hook, asked last in the `return` lowering, only for a `return` every
//! existing route refused (hard invariant 3: no admission an older route
//! makes is decided by different code, and a declined answer leaves the
//! refusal byte for byte).
//!
//! # The reading
//!
//! C11 6.8.6.4p3: a `return E;` evaluates `E` and hands its value to the
//! caller converted as if by assignment to an object of the function's
//! result type. So in a function whose declared result is a scalar value,
//! `return E;` evaluates exactly what the statement `T tmp = E;` evaluates,
//! and hands out a scalar, which carries no resource identity. The statement
//! position already decides that value with card W43-INDIRECT-ACCESS-GENERAL's
//! typed value walker ([`crate::typed_field_access::typed_scalar_value`]): no
//! call, no unary `&`, no assignment, increment or comma, no pointer-valued
//! name, at least one typed access (`*B`, `B[I]`, `B->F` or `S.F`). The
//! return is lowered with the same instruction the statement's value gets
//! (one `IndirectScalarAccess` carrying every base's dereference obligation
//! and the record bases' alias reads, with no destination and no frame) and
//! an exit that escapes nothing (`returned_evaluation_escapes: false`).
//!
//! The case this card was opened for is a scalar member of an automatic
//! aggregate whose address a call took (`rd_clock`: `return ts.tv_sec * … +
//! ts.tv_nsec / …;`). The typed access-path reader asks a never-bound root
//! of every `S.F` path because it also serves stores and pointer reads; the
//! statement position reads the same `S.F` value with no obligation, since
//! reading a member of a live automatic object dereferences nothing and
//! moves no identity (W48-RETURN-AGGREGATE-MEMBER-READ).
//!
//! # What stays refused, by name
//!
//! A pointer result (`return_facts.scalar_result` false), a value the walker
//! refuses (a call, `&`, a pointer-valued operand, a nested path), a value
//! that names a tracked descriptor, an opaque callee spelling and braces.
//! The Lean model is `QCore.ScalarReturnValue`.

use serde_json::json;

use super::{LegacyInstruction, Parser, Statement, Token, contains_braces, descriptor_custody};

impl Parser<'_> {
    /// Hook: the last route of a refused `return`. `None` keeps the refusal.
    pub(super) fn scalar_return_value(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        if expression.is_empty()
            || !self.return_facts.scalar_result
            || contains_braces(expression)
            || self.opaque_callee(expression)
        {
            return None;
        }
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names).is_some() {
            return None;
        }
        let plan = crate::typed_field_access::typed_scalar_value(
            &texts,
            &self.typed_model,
            &self.typed_types,
        );
        crate::typed_access_path::record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts,
            "position": "scalar-return-value",
            "admitted": plan.is_some(),
            "bases": plan.as_ref().map(|plan| plan.bases.clone()),
        }));
        let plan = plan?;
        let read = LegacyInstruction::IndirectScalarAccess {
            alias_reads: plan
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.bases,
            destination: None,
            // A value writes nothing: the walker admits no assignment,
            // increment or call.
            frame: None,
            chains: Vec::new(),
            borrowed_destination: false,
            readback: None,
        };
        Some(Statement::Return {
            line,
            writes: self.expression_writes(expression),
            expression_supported: true,
            returned_place: None,
            returned_evaluation: Some(Box::new(read)),
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        })
    }
}
