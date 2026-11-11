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

//! Card **W47-ACCESS-PATH-POSITIONS** (plan P3, gateway rule 10k): the
//! Clang-typed access-path layer of W46-ACCESS-PATH-STATEMENTS applied in a
//! controlling-expression operand, a `return` expression and a call actual.
//!
//! Every hook here is a **fallback**: it is consulted only where every
//! existing route of its position refused, so no admission an older route
//! makes is decided by different code (hard invariant 3), and a declined
//! answer leaves the refusal byte for byte.
//!
//! * **Conditions.** A call-free operand of a controlling expression the
//!   existing routes refuse is read by [`StatementIndex::condition`] and
//!   lowered to [`LegacyInstruction::IndirectScalarAccess`] with no
//!   destination and no frame: the per-hop obligations are checked at the
//!   operand's own node, before either successor. Operands are wired by the
//!   W44 condition desugaring (rule 5h), so the operand of `&&`, `||` and
//!   `?:` that C does not evaluate on a path contributes neither obligation
//!   nor discharge there, and a guard such as `p && p->x` reaches `p->x` only
//!   on the edge the existing null branch refines. One `NAME = PATH` operand
//!   is P2's statement followed by the existing null/neutral decision on
//!   `NAME`. A call-free condition is desugared only when at least one
//!   operand needed this reader; otherwise its refusal stays.
//! * **Returns.** One scalar value, or one pointer-valued path read whose
//!   every root the body never binds, is evaluated at the return node with a
//!   `NoOp` exit: the frame is unchanged, no custody is handed back and no
//!   place is escaped (constraint 5b's `None` answer).
//! * **Call actuals.** An actual at a `Borrowed` or declared data-tail
//!   position of a callee whose contract or validated summary is in force,
//!   read as a scalar value or (every root never bound) a pointer value,
//!   joins W44-INDIRECT-CALL-ARGUMENT-ACCESS's probe unchanged: its reads and
//!   per-link obligations precede the call, the call's own contract applies,
//!   and a custody-taking position or an uncontracted callee keeps its
//!   refusal.

use super::condition_desugar::Position as ConditionPosition;
use super::{
    LegacyInstruction, Parser, Token, contains_braces, contains_call, descriptor_custody,
    is_identifier,
};
use crate::neutral_position::ExpressionRole;
use crate::typed_access_path::{StatementIndex, StatementPlan, record_position};
use crate::typed_position_shapes::Residual;

impl Parser<'_> {
    pub(super) fn typed_positions_index(&self) -> Option<&StatementIndex> {
        let function = &self.typed_path_function;
        self.typed_path_index
            .get_or_init(|| StatementIndex::read(function))
            .as_ref()
    }

    /// The premise a pointer read handed out of the body by `return`
    /// requires of every root: the custody closure proves it never bound, and
    /// the declaration reader's `may_bind` over-approximation (constraint 5b)
    /// does not name it. Rule 10l serves both from the AST where the lexical
    /// declaration reader answers nothing.
    fn pointer_root(&self, name: &str) -> bool {
        self.position_pointer_root(name)
    }

    /// The read of one admitted plan with no destination and no frame.
    fn typed_read(&self, plan: StatementPlan) -> LegacyInstruction {
        if plan.accesses == 0 {
            return LegacyInstruction::NoOp;
        }
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: plan
                .scalar
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.scalar.bases,
            destination: None,
            frame: None,
            chains: plan.chains,
            borrowed_destination: false,
            readback: None,
        }
    }

    fn texts(tokens: &[Token]) -> Vec<&str> {
        tokens.iter().map(|token| token.text.as_str()).collect()
    }

    /// Hook: the last decision of a call-free condition operand, after every
    /// existing route refused it.
    pub(super) fn typed_condition_decision(&self, tokens: &[Token]) -> Option<LegacyInstruction> {
        if contains_call(tokens) || contains_braces(tokens) || self.opaque_callee(tokens) {
            return None;
        }
        let texts = Self::texts(tokens);
        if descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names).is_some() {
            return None;
        }
        let line = tokens.first()?.line;
        // Rule 12q: this consultation's own parity decisions.
        let decisions = crate::consultation_decisions::Decisions::new();
        let plan = self.typed_positions_index()?.condition(
            &texts,
            line,
            |name| self.position_never_bound(name),
            Some(&decisions),
        );
        decisions.record_places(
            &self.typed_path_function,
            line,
            &texts,
            "condition",
            plan.is_some(),
        );
        record_position(
            &self.typed_path_function,
            line,
            &texts,
            "condition",
            plan.as_ref(),
        );
        if plan.is_none() {
            self.record_residual_shape(line, &texts, Residual::Condition);
        }
        let plan = plan?;
        self.typed_position_uses
            .set(self.typed_position_uses.get() + 1);
        Some(self.typed_read(plan))
    }

    /// Hook: a call-free condition operand holding one `NAME = PATH`, as the
    /// typed statement followed by the decision on `NAME`. `None` keeps the
    /// operand's refusal.
    pub(super) fn typed_assignment_operand(
        &self,
        tokens: &[Token],
        position: ConditionPosition,
    ) -> Option<(LegacyInstruction, LegacyInstruction)> {
        if contains_call(tokens) || contains_braces(tokens) || self.opaque_callee(tokens) {
            return None;
        }
        let texts = Self::texts(tokens);
        let assignments = (0..texts.len())
            .filter(|&index| is_assignment(&texts, index))
            .collect::<Vec<_>>();
        let [at] = assignments.as_slice() else {
            return None;
        };
        let name_index = at.checked_sub(1)?;
        let name = texts[name_index];
        if !is_identifier(name) {
            return None;
        }
        // The group is the whole operand, or one `( NAME = VALUE )` in it.
        let (start, end) = if name_index == 0 {
            (0, texts.len())
        } else {
            let open = name_index.checked_sub(1)?;
            if texts[open] != "(" {
                return None;
            }
            let close = close_of(&texts, open)?;
            (open, close + 1)
        };
        let statement_end = if start == 0 && name_index == 0 {
            texts.len()
        } else {
            end - 1
        };
        let declared = self.position_declared_local(name);
        if !declared
            || self.descriptor_names.contains(name)
            || self.formal_parameters.contains(name)
            || texts
                .iter()
                .enumerate()
                .any(|(index, text)| !(start..end).contains(&index) && *text == name)
        {
            return None;
        }
        let statement = &tokens[name_index..statement_end];
        let statement_texts = Self::texts(statement);
        if descriptor_custody::value_carries_descriptor(&statement_texts, &self.descriptor_names)
            .is_some()
        {
            return None;
        }
        let line = statement.first()?.line;
        // Rule 12q: this consultation's own parity decisions.
        let decisions = crate::consultation_decisions::Decisions::new();
        let plan = self.typed_positions_index()?.assignment(
            &statement_texts,
            line,
            |candidate| self.position_never_bound(candidate),
            Some(&decisions),
        );
        decisions.record_places(
            &self.typed_path_function,
            line,
            &statement_texts,
            "condition-assignment",
            plan.is_some(),
        );
        record_position(
            &self.typed_path_function,
            line,
            &statement_texts,
            "condition-assignment",
            plan.as_ref(),
        );
        if plan.is_none() {
            self.record_residual_shape(line, &statement_texts, Residual::Assignment);
        }
        let plan = plan?;
        if plan.scalar.destination.as_deref() != Some(name) || plan.scalar.writes_through_pointer {
            return None;
        }
        let assignment = if plan.accesses == 0 {
            LegacyInstruction::ForgetScalar {
                destination: name.to_owned(),
            }
        } else {
            LegacyInstruction::IndirectScalarAccess {
                typed_alternative: None,
                alias_reads: plan
                    .scalar
                    .record_bases
                    .iter()
                    .filter_map(|base| self.read_alias(Some(base)))
                    .collect(),
                bases: plan.scalar.bases,
                destination: Some(name.to_owned()),
                frame: None,
                chains: plan.chains,
                borrowed_destination: plan.borrowed_destination,
                readback: None,
            }
        };
        // The residual decides on `NAME` exactly as `if (NAME …)` would.
        let mut residual = Vec::with_capacity(tokens.len());
        residual.extend_from_slice(&tokens[..start]);
        for text in ["(", name, ")"] {
            residual.push(Token {
                text: text.to_owned(),
                line: tokens[start].line,
            });
        }
        residual.extend_from_slice(&tokens[end..]);
        let decision = match self.null_condition(&residual) {
            Some(test) if test.validity.is_none() => test.branch(),
            Some(_) => return None,
            None if self.supported_expression(&residual, ExpressionRole::Decides) => {
                LegacyInstruction::NoOp
            }
            None => return None,
        };
        if position == ConditionPosition::Switch && decision != LegacyInstruction::NoOp {
            return None;
        }
        self.typed_position_uses
            .set(self.typed_position_uses.get() + 1);
        Some((assignment, decision))
    }

    /// Hook: the last route of a `return` expression, after every existing
    /// route declined. `Some((read, false))`: the read at the return node and
    /// a `NoOp` exit.
    pub(super) fn typed_return_instruction(
        &self,
        expression: &[Token],
    ) -> Option<(LegacyInstruction, bool)> {
        if expression.is_empty()
            || contains_call(expression)
            || contains_braces(expression)
            || self.opaque_callee(expression)
        {
            return None;
        }
        let texts = Self::texts(expression);
        if descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names).is_some() {
            return None;
        }
        let line = expression.first()?.line;
        // Rule 12q: this consultation's own parity decisions.
        let decisions = crate::consultation_decisions::Decisions::new();
        let plan = self.typed_positions_index()?.returned(
            &texts,
            line,
            |name| self.position_never_bound(name),
            |name| self.pointer_root(name),
            Some(&decisions),
        );
        decisions.record_places(
            &self.typed_path_function,
            line,
            &texts,
            "return",
            plan.is_some(),
        );
        record_position(
            &self.typed_path_function,
            line,
            &texts,
            "return",
            plan.as_ref(),
        );
        if plan.is_none() {
            self.record_residual_shape(line, &texts, Residual::Return);
        }
        Some((self.typed_read(plan?), false))
    }

    /// Hook: one call actual the lexical recogniser refused, at a position the
    /// caller already found admissible.
    pub(super) fn typed_call_argument(
        &self,
        callee: &str,
        index: usize,
        arity: usize,
        argument: &[&str],
        line: usize,
    ) -> Option<crate::typed_access_positions::ArgumentPlan> {
        // Rule 12q: this consultation's own parity decisions.
        let decisions = crate::consultation_decisions::Decisions::new();
        let plan = self.typed_positions_index()?.argument(
            callee,
            index,
            arity,
            argument,
            line,
            |name| self.position_never_bound(name),
            Some(&decisions),
        );
        decisions.record_places(
            &self.typed_path_function,
            line,
            argument,
            "call-argument",
            plan.is_some(),
        );
        record_position(
            &self.typed_path_function,
            line,
            argument,
            "call-argument",
            plan.as_ref().map(|plan| &plan.plan),
        );
        if plan.is_none() {
            self.record_residual_shape(
                line,
                argument,
                Residual::Argument {
                    callee,
                    index,
                    arity,
                },
            );
        }
        plan
    }

    /// Card W47-ACCESS-PATH-POSITION-RESIDUALS: parity only, the shape of a
    /// refused typed position. Changes nothing.
    fn record_residual_shape(&self, line: usize, texts: &[&str], residual: Residual<'_>) {
        crate::typed_position_shapes::record(
            self.typed_positions_index(),
            &self.typed_path_function,
            line,
            texts,
            residual,
        );
    }

    /// Parity only: the lexical call-argument recogniser admitted this
    /// actual; record the typed candidate beside it. Changes nothing.
    pub(super) fn record_typed_argument_legacy(
        &self,
        callee: &str,
        index: usize,
        arity: usize,
        argument: &[&str],
        line: usize,
        read: &super::call_argument_access::ArgumentRead,
    ) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let candidate = self.typed_positions_index().and_then(|typed| {
            typed.argument(
                callee,
                index,
                arity,
                argument,
                line,
                |name| self.position_never_bound(name),
                None,
            )
        });
        crate::typed_access_path::record_position_legacy(
            &self.typed_path_function,
            line,
            argument,
            "call-argument",
            "call-argument-access",
            &format!("{read:?}"),
            candidate.as_ref().map(|plan| &plan.plan),
        );
    }

    /// Parity only: an existing route admitted this position; record the
    /// typed candidate beside it. Changes nothing.
    pub(super) fn record_typed_position_legacy(
        &self,
        tokens: &[Token],
        position: &str,
        route: &str,
        instruction: &LegacyInstruction,
    ) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let texts = Self::texts(tokens);
        let Some(line) = tokens.first().map(|token| token.line) else {
            return;
        };
        let never_bound = |name: &str| self.position_never_bound(name);
        let candidate = self
            .typed_positions_index()
            .and_then(|index| match position {
                "return" => index.returned(
                    &texts,
                    line,
                    never_bound,
                    |name| self.pointer_root(name),
                    None,
                ),
                _ => index.condition(&texts, line, never_bound, None),
            });
        crate::typed_access_path::record_position_legacy(
            &self.typed_path_function,
            line,
            &texts,
            position,
            route,
            &format!("{instruction:?}"),
            candidate.as_ref(),
        );
    }
}

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

fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}
