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
//! controlling expression the condition desugaring (rule 5h) refuses on one of
//! its call-bearing leaves. A child of `legacy_condition_desugar.rs`, reached
//! through two thin hooks there.
//!
//! **(1) A residual read beside identity calls.** Rule 5h lowers each call of
//! a leaf through the one application point and then requires the leaf's
//! residual (every call replaced by `0`) to be custody-neutral as written. A
//! residual that reads memory (`d_len(s) > b->n`, `f(x) == p->len`) was
//! refused, although the same read alone is decided by the typed condition
//! read (`condition_access`). This module admits that residual read only when
//! the leaf has no assignment and **every** instruction the leaf's calls
//! lowered to is the identity on the abstract state (an identity contract call
//! or a custody-neutral actual read). C leaves the order of a call and an
//! operand read of one leaf unspecified (C11 6.5p2-3); because the calls
//! change no map, both orders reach the same abstract state and the read's
//! obligations (each dereferenced base unbound, or one known, non-null, live
//! identity) are checked against that same state, so the order chosen (calls,
//! then the read) is no choice at all. The read is the typed read's own
//! instruction, with no destination: it refines neither edge, so the leaf's
//! value is never used to prune a path (the residual's `0` is not the call's
//! value). A switch keeps its refusal (its dispatch must be a `NoOp`).
//!
//! **(2) A named refusal for an uncontracted call.** When every route of the
//! position, the desugaring, the elaborator (rule 10s) and the `va_arg` route
//! (rule 12c) refused, and the desugaring's refusal was a call the classifier
//! refused with exactly ``call to uncontracted function `F` `` for that call's
//! callee `F`, the position's refusal node names it,
//! ``call to uncontracted function `F` in branch condition`` (the position's
//! own suffix: `loop`, `for-loop`, `do-while`): a one-node plan holding the
//! refusal, branching to both exits exactly as the refusal node did, with the
//! same writes. The suffix keeps the node a controlling expression's refusal
//! for the refused-operand hand-on rule (`legacy_refused_operands.rs`), which
//! reads the position from the text, so no withdrawal and no accusation moves. A re-attribution of rule 12k's kind
//! for controlling expressions, never an admission: the node stays `Unknown`.
//! A condition that reads a tracked descriptor, a switch, and any other
//! refusal keep the position's text.

use super::super::descriptor_custody;
use super::{ConditionPlan, Parser, PlanNode, Position, Target};
use crate::legacy_c::{LegacyInstruction, Token};

/// Rule 63 (1) switch: the residual read beside identity calls.
pub(crate) const RESIDUAL_READS: bool = true;
/// Rule 63 (2) switch: the uncontracted call's named refusal.
pub(crate) const UNCONTRACTED_ATTRIBUTION: bool = true;

/// The classifier's text for a call whose callee has no contract in force.
pub(super) fn uncontracted_description(callee: &str) -> String {
    format!("call to uncontracted function `{callee}`")
}

/// The named refusal of a controlling expression: the classifier's text and
/// the position's suffix, which the generic refusal ends with too.
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

/// Whether every instruction a leaf's calls lowered to leaves the abstract
/// state unchanged: identity contract calls and actual reads with no
/// destination.
pub(super) fn all_identity(lowered: &[LegacyInstruction]) -> bool {
    !lowered.is_empty()
        && lowered.iter().all(|instruction| match instruction {
            LegacyInstruction::ContractCall { .. } => super::is_identity_call(instruction),
            LegacyInstruction::IndirectScalarAccess {
                destination: None,
                frame: None,
                ..
            }
            | LegacyInstruction::ArgumentReads { .. } => true,
            _ => false,
        })
}

impl Parser<'_> {
    /// Rule 63 (2) for a consumer that reads a plan as a decision (the value
    /// connective route, rule 10z): `planned_condition` with a named refusal
    /// read as no plan, so that consumer keeps exactly its older answer.
    pub(in super::super) fn planned_condition_decision(
        &mut self,
        line: usize,
        condition: &[Token],
        position: Position,
        refused: bool,
    ) -> Option<Box<ConditionPlan>> {
        let plan = self.planned_condition(line, condition, position, refused);
        if self.desugar.named_refusal {
            self.desugar.named_refusal = false;
            return None;
        }
        plan
    }

    /// Rule 63 (1): the decision of a call-bearing leaf whose residual the
    /// neutral route refused: the typed condition read of the residual, when
    /// every lowered instruction is the identity. `None` keeps the refusal.
    pub(super) fn residual_read_decision(
        &self,
        lowered: &[LegacyInstruction],
        residual: &[Token],
        position: Position,
    ) -> Option<LegacyInstruction> {
        if !RESIDUAL_READS || position == Position::Switch || !all_identity(lowered) {
            return None;
        }
        let read = *self.condition_access(residual)?;
        matches!(
            read,
            LegacyInstruction::IndirectScalarAccess {
                destination: None,
                frame: None,
                ..
            }
        )
        .then_some(read)
    }

    /// Rule 63 (2): the one-node plan naming the uncontracted callee the
    /// desugaring refused, asked after every route of the position refused.
    /// `refused_call` is the desugaring's `(callee, description)` for this
    /// condition. `None` keeps the position's refusal.
    pub(super) fn attributed_condition_refusal(
        &self,
        condition: &[Token],
        position: Position,
        refused_call: Option<(String, String)>,
    ) -> Option<Box<ConditionPlan>> {
        let (callee, description) = refused_call?;
        if !UNCONTRACTED_ATTRIBUTION
            || position == Position::Switch
            || description != uncontracted_description(&callee)
        {
            return None;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !texts.contains(&callee.as_str())
            || descriptor_custody::value_carries_descriptor(&texts, &self.descriptor_names)
                .is_some()
        {
            return None;
        }
        Some(Box::new(ConditionPlan {
            nodes: vec![PlanNode {
                instruction: LegacyInstruction::Unknown {
                    description: attributed_description(&callee, position)?,
                    writes: self.expression_writes(condition),
                },
                successors: vec![Target::True, Target::False],
            }],
            entry: 0,
        }))
    }
}
