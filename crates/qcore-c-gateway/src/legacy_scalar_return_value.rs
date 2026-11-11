// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

/// The hook's answer (`QCore.ScalarReturnValue.Decision`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Admit,
    Decline,
}

impl Decision {
    #[cfg(test)]
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::Decline => "decline",
        }
    }
}

/// `QCore.ScalarReturnValue.decideReturn`: `refused`, every existing route
/// refused the return; `scalar_result`, the function's declared result is a
/// scalar value; `hidden`, the value names a tracked descriptor, an opaque
/// callee spelling or braces; `walker`, the W43 value walker admits it.
pub(crate) fn decide(refused: bool, scalar_result: bool, hidden: bool, walker: bool) -> Decision {
    if refused && scalar_result && !hidden && walker {
        Decision::Admit
    } else {
        Decision::Decline
    }
}

/// One row of the shared table: `(refused, scalar_result, hidden, walker)`
/// and the decision's code (`QCore.ScalarReturnValue.DecisionVector`).
#[cfg(test)]
pub(crate) type DecisionVector = ((bool, bool, bool, bool), &'static str);

/// `QCore.ScalarReturnValue.decisionVectors`, row for row.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[DecisionVector] = &[
    ((false, false, false, false), "decline"),
    ((false, false, false, true), "decline"),
    ((false, false, true, false), "decline"),
    ((false, false, true, true), "decline"),
    ((false, true, false, false), "decline"),
    ((false, true, false, true), "decline"),
    ((false, true, true, false), "decline"),
    ((false, true, true, true), "decline"),
    ((true, false, false, false), "decline"),
    ((true, false, false, true), "decline"),
    ((true, false, true, false), "decline"),
    ((true, false, true, true), "decline"),
    ((true, true, false, false), "decline"),
    ((true, true, false, true), "admit"),
    ((true, true, true, false), "decline"),
    ((true, true, true, true), "decline"),
];

impl Parser<'_> {
    /// Hook: the last route of a refused `return` (the caller asks it only
    /// for one). `None` keeps the refusal.
    pub(super) fn scalar_return_value(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        let scalar_result = self.return_facts.scalar_result;
        if expression.is_empty() || !scalar_result {
            return None;
        }
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let hidden = contains_braces(expression)
            || self.opaque_callee(expression)
            || descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names)
                .is_some();
        let plan = if hidden {
            None
        } else {
            crate::typed_field_access::typed_scalar_value(
                &texts,
                &self.typed_model,
                &self.typed_types,
            )
        };
        let decision = decide(true, scalar_result, hidden, plan.is_some());
        crate::typed_access_path::record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts,
            "position": "scalar-return-value",
            "admitted": decision == Decision::Admit,
            "hidden": hidden,
            "bases": plan.as_ref().map(|plan| plan.bases.clone()),
        }));
        if decision != Decision::Admit {
            return None;
        }
        let plan = plan?;
        let read = LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
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

#[cfg(test)]
#[path = "../tests/unit/legacy_scalar_return_value.rs"]
mod tests;
