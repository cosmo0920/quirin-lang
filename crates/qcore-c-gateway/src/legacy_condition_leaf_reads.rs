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

//! Card **AG3-CONDITION-RETURN-CALLS** (gateway rule 63): two readings of a
//! controlling expression (`if`, `while`, `for`, `do`-`while`) that every
//! route of its position refused, rule 5h's desugaring, the elaborator (rule
//! 10s) and the `va_arg` route (rule 12c) included. Asked last, from one thin
//! hook per position in `legacy_c.rs`, so no expression an older route
//! decides is decided differently. `legacy_condition_desugar.rs` is not
//! edited: it is a pinned artifact of the libc reading-site evidence.
//!
//! **(1) A residual read beside identity calls.** One leaf (no top-level
//! `&&`, `||` or `?:`), no assignment, increment or brace, whose calls are
//! direct, not nested in one another, and each instantiated through the one
//! application point (rule 10u, use `Tested`, exactly as rule 5h instantiates
//! a leaf call). When **every** instruction they lower to is the identity on
//! the abstract state (an identity contract call, or an actual read with no
//! destination and no frame), and the residual (each call replaced by `0`) is
//! not custody-neutral as written but is read by the typed condition read
//! (`condition_access`, which answers a read with no destination and no
//! frame), the condition is the calls, then that read as its branch. C
//! leaves the order of a leaf's call and its operand reads unspecified (C11
//! 6.5p2-3); with identity calls every order reaches the same state and
//! checks the read's obligations against it, so the order chosen is no
//! choice. The read refines neither edge: the residual's `0` is not the
//! call's value.
//!
//! **(2) A named refusal for an uncontracted call.** Otherwise, when the
//! condition's direct calls (none nested in another) are, in textual order,
//! instantiable calls followed by one the classifier refuses with exactly
//! ``call to uncontracted function `F` ``, the position's refusal node names
//! it: ``call to uncontracted function `F` in branch condition`` (the
//! position's own suffix: `loop`, `for-loop`, `do-while`), one plan node
//! holding the refusal, branching to both exits exactly as the refusal node
//! did, with the same writes. Every probe's state is restored: nothing of the
//! classification is kept. The suffix keeps the node a controlling
//! expression's refusal for the refused-operand hand-on rule
//! (`legacy_refused_operands.rs`, `refuses_a_condition`), which reads the
//! position from the text, so no withdrawal and no accusation moves. A
//! re-attribution of rule 12k's kind, never an admission. A condition that
//! reads a tracked descriptor keeps the position's text.

use super::call_instantiation_route::CallRefusal;
use super::condition_desugar::{ConditionPlan, PlanNode, Position, Target, is_identity_call};
use super::{
    ExpressionRole, LegacyInstruction, Parser, TYPE_OPERATORS, Token, contains_braces,
    descriptor_custody, is_identifier,
};
use crate::call_instantiation::ResultUse;

/// Rule 63 (1) switch: the residual read beside identity calls.
pub(crate) const RESIDUAL_READS: bool = true;
/// Rule 63 (2) switch: the uncontracted call's named refusal.
pub(crate) const UNCONTRACTED_ATTRIBUTION: bool = true;

/// Names a call-shaped token pair that this rule never reads as a call.
const NOT_CALLEES: [&str; 2] = ["__builtin_expect_with_probability", "__builtin_choose_expr"];

/// The branch hint rule 5h reads as its first operand when its second is one
/// token of decimal digits (`likely`/`unlikely`).
const HINT: &str = "__builtin_expect";

/// The classifier's text for a call whose callee has no contract in force.
pub(super) fn uncontracted_description(callee: &str) -> String {
    format!("call to uncontracted function `{callee}`")
}

/// The named refusal of a controlling expression: the classifier's text and
/// the position's suffix, which the position's own refusal ends with too.
pub(super) fn attributed_description(callee: &str, position: Position) -> Option<String> {
    let suffix = match position {
        Position::Branch => "branch condition",
        Position::Loop => "loop condition",
        Position::ForLoop => "for-loop condition",
        Position::DoWhile => "do-while condition",
        Position::Switch => return None,
    };
    Some(format!("{} in {suffix}", uncontracted_description(callee)))
}

/// Whether every instruction lowered from a leaf's calls leaves the abstract
/// state unchanged: identity contract calls and actual reads with no
/// destination and no frame.
pub(super) fn all_identity(lowered: &[LegacyInstruction]) -> bool {
    !lowered.is_empty()
        && lowered.iter().all(|instruction| match instruction {
            LegacyInstruction::ContractCall { .. } => is_identity_call(instruction),
            LegacyInstruction::IndirectScalarAccess {
                destination: None,
                frame: None,
                ..
            }
            | LegacyInstruction::ArgumentReads { .. } => true,
            _ => false,
        })
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

/// The second operand of `__builtin_expect ( E , K )` at `open` (its `(`)
/// when it is one token of decimal digits: rule 5h reads the hint as `E`.
fn literal_hint(texts: &[&str], open: usize, close: usize) -> bool {
    let mut depth = 0usize;
    let mut comma = None;
    for (index, text) in texts.iter().enumerate().take(close).skip(open + 1) {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                if comma.is_some() {
                    return false;
                }
                comma = Some(index);
            }
            _ => {}
        }
    }
    comma.is_some_and(|comma| {
        close == comma + 2 && texts[comma + 1].chars().all(|ch| ch.is_ascii_digit())
    })
}

/// The direct calls of `texts`, `(callee, close)` in textual order, and
/// whether a literal branch hint was read through; `None` when a call is
/// nested in another call's actuals, is made through a member or a
/// parenthesised expression, or is another hint.
fn direct_calls(texts: &[&str]) -> Option<(Vec<(usize, usize)>, bool)> {
    // A call through a parenthesised expression (or a cast of a group):
    // `( … ) (`.
    if texts.windows(2).any(|pair| pair == [")", "("]) {
        return None;
    }
    let mut calls: Vec<(usize, usize)> = Vec::new();
    let mut hinted = false;
    for index in 0..texts.len().saturating_sub(1) {
        if texts[index + 1] != "(" {
            continue;
        }
        let name = texts[index];
        if !is_identifier(name) || TYPE_OPERATORS.contains(&name) {
            continue;
        }
        if NOT_CALLEES.contains(&name) {
            return None;
        }
        if index > 0 && matches!(texts[index - 1], "." | ">") {
            return None;
        }
        let close = close_of(texts, index + 1)?;
        if calls.iter().any(|(_, end)| index < *end) {
            return None;
        }
        if name == HINT {
            // Transparent, as rule 5h reads it; anything else refuses.
            if !literal_hint(texts, index + 1, close) {
                return None;
            }
            hinted = true;
            continue;
        }
        calls.push((index, close));
    }
    Some((calls, hinted))
}

/// Whether `texts` holds an assignment, compound assignment, increment or
/// decrement (tokens are single characters: `==` is `=` `=`).
fn writes_anything(texts: &[&str]) -> bool {
    texts
        .windows(2)
        .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
        || (0..texts.len()).any(|index| {
            if texts[index] != "=" {
                return false;
            }
            let previous = index.checked_sub(1).map(|i| texts[i]);
            let next = texts.get(index + 1).copied();
            next != Some("=")
                && (!matches!(previous, Some("=" | "!" | "<" | ">"))
                    || (matches!(previous, Some("<" | ">"))
                        && index.checked_sub(2).map(|i| texts[i]) == previous))
        })
}

/// Whether `texts` holds a top-level `&&`, `||` or `?`.
fn has_connective(texts: &[&str]) -> bool {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.saturating_sub(1),
            "?" if depth == 0 => return true,
            "&" | "|" if depth == 0 && texts.get(index + 1) == Some(text) => return true,
            _ => {}
        }
    }
    false
}

impl Parser<'_> {
    /// The hook: asked at each controlling-expression position after rule
    /// 5h's `planned_condition` answered no plan. `refused` is the position's
    /// own "every older route refused" (the argument it passed to
    /// `planned_condition`). `None` keeps the position's refusal.
    pub(super) fn condition_leaf_reads(
        &mut self,
        condition: &[Token],
        position: Position,
        refused: bool,
    ) -> Option<Box<ConditionPlan>> {
        if !refused
            || position == Position::Switch
            || condition.is_empty()
            || contains_braces(condition)
            || self.opaque_callee(condition)
        {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names).is_some() {
            return None;
        }
        let (calls, hinted) = direct_calls(&texts)?;
        if calls.is_empty() {
            return None;
        }
        // The residual read is asked only for a condition with no hint.
        if !hinted && let Some(plan) = self.residual_read_plan(condition, &texts, &calls) {
            return Some(plan);
        }
        self.named_uncontracted_refusal(condition, &texts, &calls, position)
    }

    /// Rule 63 (1).
    fn residual_read_plan(
        &mut self,
        condition: &[Token],
        texts: &[&str],
        calls: &[(usize, usize)],
    ) -> Option<Box<ConditionPlan>> {
        if !RESIDUAL_READS || has_connective(texts) || writes_anything(texts) {
            return None;
        }
        let saved = self.probe_state();
        let plan = self.residual_read_plan_unguarded(condition, calls);
        if plan.is_none() {
            self.restore_probe_state(&saved);
        }
        plan
    }

    fn residual_read_plan_unguarded(
        &mut self,
        condition: &[Token],
        calls: &[(usize, usize)],
    ) -> Option<Box<ConditionPlan>> {
        let mut lowered = Vec::new();
        for &(callee, close) in calls {
            let name = condition[callee].text.clone();
            let sequence = self
                .instantiate_call(&name, &condition[callee..=close], None, ResultUse::Tested)
                .ok()?;
            lowered.extend(sequence);
        }
        if !all_identity(&lowered) {
            return None;
        }
        let mut residual = Vec::with_capacity(condition.len());
        let mut index = 0usize;
        while index < condition.len() {
            if let Some(&(_, close)) = calls.iter().find(|(callee, _)| *callee == index) {
                residual.push(Token {
                    text: "0".to_owned(),
                    line: condition[index].line,
                });
                index = close + 1;
                continue;
            }
            residual.push(condition[index].clone());
            index += 1;
        }
        if self.supported_expression(&residual, ExpressionRole::Decides) {
            return None;
        }
        let read = *self.condition_access(&residual)?;
        if !matches!(
            read,
            LegacyInstruction::IndirectScalarAccess {
                destination: None,
                frame: None,
                ..
            }
        ) {
            return None;
        }
        let mut nodes = lowered
            .into_iter()
            .enumerate()
            .map(|(at, instruction)| PlanNode {
                instruction,
                successors: vec![Target::Node(at + 1)],
            })
            .collect::<Vec<_>>();
        nodes.push(PlanNode {
            instruction: read,
            successors: vec![Target::True, Target::False],
        });
        Some(Box::new(ConditionPlan { nodes, entry: 0 }))
    }

    /// Whether `name` designates a function rather than a function pointer
    /// object: no formal, no declared local, and no `( * name )` declarator
    /// anywhere in the unit. A call through a pointer object is an open set
    /// (rule 11p), never an "uncontracted function"; it keeps the
    /// position's text.
    fn names_a_function(&self, name: &str) -> bool {
        !self.formal_parameters.contains(name)
            && !self.position_declared_local(name)
            && !self.tokens.windows(4).any(|window| {
                window[0].text == "("
                    && window[1].text == "*"
                    && window[2].text == name
                    && window[3].text == ")"
            })
    }

    /// Rule 63 (2).
    fn named_uncontracted_refusal(
        &mut self,
        condition: &[Token],
        texts: &[&str],
        calls: &[(usize, usize)],
        position: Position,
    ) -> Option<Box<ConditionPlan>> {
        if !UNCONTRACTED_ATTRIBUTION {
            return None;
        }
        let saved = self.probe_state();
        let mut named = None;
        for &(callee, close) in calls {
            let name = texts[callee];
            match self.instantiate_call(name, &condition[callee..=close], None, ResultUse::Tested) {
                Ok(_) => continue,
                Err(CallRefusal::Unapplied(description))
                    if description == uncontracted_description(name)
                        && self.names_a_function(name) =>
                {
                    named = attributed_description(name, position);
                    break;
                }
                Err(_) => break,
            }
        }
        // Nothing of the classification is kept.
        self.restore_probe_state(&saved);
        let description = named?;
        Some(Box::new(ConditionPlan {
            nodes: vec![PlanNode {
                instruction: LegacyInstruction::Unknown {
                    description,
                    writes: self.expression_writes(condition),
                },
                successors: vec![Target::True, Target::False],
            }],
            entry: 0,
        }))
    }
}
