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

//! Card **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** (gateway rule 12k): a
//! statement the lexical gate refused for its call's actuals, whose callee
//! has no contract or summary in force, names the uncontracted callee when
//! the callee is what decides.
//!
//! ## The gap
//!
//! The lexical gate of [`super::Parser::classify_simple_base`] refuses a
//! statement (`indirect memory access is outside the resource projection`) as
//! soon as a token outside an admitted span reads memory, before the call's
//! contract is looked up. Rule W44
//! ([`super::Parser::call_argument_access`]) reads such actuals only at a
//! position the callee's contract declares `Borrowed` (or a declared data
//! tail), so for an uncontracted callee it never admits, and the refusal
//! names the access although the statement would be refused for the call even
//! with every actual read. Measured on the base (card
//! CG10-INDIRECT-ACCESS-SOLE-BLOCKERS): of the 481 Unknown functions the access
//! refusal blocks alone, 227 are call statements and 102 whole-call returns
//! with an uncontracted callee. The leaf names the wrong blocker, so the
//! callee closure (P4) does not count them.
//!
//! ## The rule
//!
//! Asked **last** (after every route of the statement or return position
//! declined), only for exactly the gate's refusal, a statement with no braces
//! that reads no tracked descriptor, whose first call names a callee with no
//! contract in force and no held contract (a review or pin hold names its own
//! reason, and its verdict is `Pending`): the statement is classified again
//! with every memory-reading actual of that call read as rule W44 reads an
//! actual at a `Borrowed` position
//! ([`super::Parser::reclassified_with_borrowed_actuals`]). When that
//! reading is refused with exactly `call to uncontracted function `F``, `F`
//! the same callee, the statement keeps the gate's refusal **with that
//! description** and the gate's own writes. Anything else (an actual neither
//! reader recognises, another refusal, an admission) keeps the gate's refusal
//! byte for byte. A whole-call `return f(args);` is read through its
//! synthetic statement `PLACE = f(args)` (C-GATEWAY-RETURN-CALL-W4), and only
//! its returned evaluation's description changes.
//!
//! The reading is never a lowering: its counters are restored and its
//! instruction discarded. The node stays `Unknown` with the same writes, so
//! no verdict moves (a non-held `Unsupported` leaf is `Unknown` whatever its
//! text). The leaf moves from the access family to the uncontracted-call
//! family, where the callee closure counts it.
//!
//! The Lean model is `QCore.UncontractedAccessAttribution`.

use super::{
    LegacyInstruction, Parser, Token, call_argument_access::GATE_REFUSAL, contains_braces,
    find_call, pin_hold, return_call, review_hold,
};

/// The refusal an uncontracted call carries, as the classifier spells it.
pub(super) fn uncontracted_refusal(callee: &str) -> String {
    format!("call to uncontracted function `{callee}`")
}

impl Parser<'_> {
    /// Rule 12k, the statement form: `instruction` with the uncontracted
    /// callee's refusal as its description, or `instruction` unchanged.
    pub(super) fn uncontracted_access_attribution(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: LegacyInstruction,
    ) -> LegacyInstruction {
        match self.uncontracted_reading(tokens, line, "statement", &instruction) {
            Some(description) => {
                let LegacyInstruction::Unknown { writes, .. } = instruction else {
                    unreachable!("the reading is asked only for the gate's refusal");
                };
                LegacyInstruction::Unknown {
                    description,
                    writes,
                }
            }
            None => instruction,
        }
    }

    /// Rule 12k, the whole-call return form: the returned evaluation with the
    /// uncontracted callee's refusal as its description, or unchanged.
    pub(super) fn uncontracted_return_attribution(
        &mut self,
        expression: &[Token],
        line: usize,
        evaluation: Option<(LegacyInstruction, bool)>,
    ) -> Option<(LegacyInstruction, bool)> {
        let (instruction, escapes) = evaluation?;
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE)
            || return_call::whole_expression_call(expression, &texts).is_none()
        {
            return Some((instruction, escapes));
        }
        let mut synthetic = Vec::with_capacity(expression.len() + 2);
        synthetic.extend(return_call::assignment_prefix(line));
        synthetic.extend_from_slice(expression);
        match self.uncontracted_reading(&synthetic, line, "return", &instruction) {
            Some(description) => {
                let LegacyInstruction::Unknown { writes, .. } = instruction else {
                    unreachable!("the reading is asked only for the gate's refusal");
                };
                Some((
                    LegacyInstruction::Unknown {
                        description,
                        writes,
                    },
                    escapes,
                ))
            }
            None => Some((instruction, escapes)),
        }
    }

    /// The uncontracted callee's refusal when the callee is what decides
    /// `tokens`, refused by the gate as `instruction`; `None` otherwise.
    fn uncontracted_reading(
        &mut self,
        tokens: &[Token],
        line: usize,
        form: &str,
        instruction: &LegacyInstruction,
    ) -> Option<String> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        if description != GATE_REFUSAL || contains_braces(tokens) {
            return None;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !self.descriptor_names.is_empty()
            && super::descriptor_custody::statement_reads_descriptor(&texts, &self.descriptor_names)
                .is_some()
        {
            return None;
        }
        let (_, callee) = find_call(&texts)?;
        if self.contracts.contract(callee).is_some()
            || review_hold::held_callee(callee).is_some()
            || pin_hold::held_callee(callee).is_some()
        {
            return None;
        }
        let expected = uncontracted_refusal(callee);
        let reading = self.reclassified_with_borrowed_actuals(tokens);
        let answer = match &reading {
            None => "an actual neither reader recognises",
            Some(LegacyInstruction::Unknown { description, .. }) if *description == expected => {
                "attributed"
            }
            Some(LegacyInstruction::Unknown { description, .. }) => description.as_str(),
            Some(_) => "admitted with the actuals read",
        };
        let decided = answer == "attributed";
        self.record_uncontracted_access(tokens, line, form, callee, answer);
        decided.then_some(expected)
    }

    /// Parity only: one row per reading, with its answer.
    fn record_uncontracted_access(
        &self,
        tokens: &[Token],
        line: usize,
        form: &str,
        callee: &str,
        answer: &str,
    ) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        crate::typed_access_path::record_row(serde_json::json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts,
            "position": "uncontracted-access",
            "rule": "12k",
            "form": form,
            "callee": callee,
            "attributed": answer == "attributed",
            "answer": answer,
        }));
    }
}
