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

//! Card **CG10-INDIRECT-ACCESS-SOLE-BLOCKERS** (gateway rule 12i): a `return`
//! whose one call's actuals the lexical gate refused is read with rule W44's
//! call-argument access, exactly as the same call as a statement.
//!
//! ## The gap
//!
//! The return position decides `return f(args);` by classifying the synthetic
//! statement `PLACE = f(args)` (card C-GATEWAY-RETURN-CALL-W4) and B01's
//! `return E(f(args))` by classifying the call alone. The lexical gate refuses
//! both as soon as an actual reads memory (`f(p->name)`, `f(&rk->lock)`), and
//! W43-RETURN-EXPRESSION re-attributes the refusal to the return. The statement
//! position reads exactly those actuals with rule W44
//! ([`super::Parser::call_argument_access`]); the return position never asked
//! it. Measured on the base: 19 such leaves on 19 corpus functions, 15 of which
//! carry no other refusal.
//!
//! ## The rule
//!
//! Asked after every older route of the position declined or only
//! re-attributed the gate's refusal (hard invariant 3), for
//!
//! 1. **a whole-call return**: the synthetic `PLACE = f(args)` (the reserved
//!    place unspelled in the stream) refused by the gate, which W44 lowers to
//!    its block (the typed reads over the bases the used actuals dereference,
//!    their second obligation, then the call) whose call is a contracted call
//!    into exactly `PLACE` with a result the return position already accepts
//!    (`escapes_only_fresh_custody`; a derived result only in the CFG a derived
//!    summary is validated on). The return is then the existing pair
//!    `PLACE = f(args); return PLACE;`, with the reads before the call;
//! 2. **B01's one call in a scalar expression**: an externally declared callee
//!    whose call is the expression's only unconditional call and whose
//!    surrounding operands are scalar, the call refused by the gate, which W44
//!    lowers to a block whose call is a contracted call with no destination and
//!    a `None` result. The return then escapes nothing.
//!
//! The reads precede the call because C evaluates every actual before the
//! call (C11 6.5.2.2p10), and the call precedes the return's exit because the
//! value returned is the call's. Every premise of W44 holds unchanged (an
//! actual at a custody-taking position, an uncontracted callee, an
//! unrecognised actual or a declined probe keeps the refusal byte for byte),
//! and so does the return position's acceptance (a reallocated result stays
//! refused). A declined probe restores the site and element counters, so a
//! refusal is observationally the old one.
//!
//! The Lean model is `QCore.ReturnCallActuals`.

use qcore_schemas::contract::ResourceReturnMode;

use super::{
    LegacyInstruction, Parser, Statement, Token, canonical_place, find_function_body, return_call,
};

/// Rule 12i's decision for one refused `return`, as recorded in parity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Form {
    /// `return f(args);`
    WholeCall,
    /// `return E(f(args));` with `E` scalar (B01).
    ScalarCall,
}

impl Form {
    fn code(self) -> &'static str {
        match self {
            Self::WholeCall => "whole-call",
            Self::ScalarCall => "scalar-call",
        }
    }
}

/// Whether `instruction` is exactly the lexical gate's access refusal.
pub(super) fn is_gate_refusal(instruction: &LegacyInstruction) -> bool {
    matches!(instruction, LegacyInstruction::Unknown { description, .. }
        if description == super::call_argument_access::GATE_REFUSAL)
}

impl Parser<'_> {
    /// Rule 12i: the block a refused `return` lowers to, or `None` (the
    /// refusal stands and every counter is as before).
    pub(super) fn return_call_argument_access(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        if self.opaque_callee(expression)
            || !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE)
        {
            return None;
        }
        // Card GAPO-RETURN-CORRECTNESS (rule 15z): W44's re-classification
        // is not asked rule 12h's H-U0, so a returned call whose actual is an
        // identity-preserving cast of a place keeps its refusal here.
        if self.returned_call_hides_a_place(expression) {
            return None;
        }
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if return_call::whole_expression_call(expression, &texts).is_some() {
            let mut synthetic = Vec::with_capacity(expression.len() + 2);
            synthetic.extend(return_call::assignment_prefix(line));
            synthetic.extend_from_slice(expression);
            let admit_derived = self.admit_derived_returns;
            let accepted = self.lowered_call(&synthetic, |instruction| {
                matches!(instruction, LegacyInstruction::ContractCall {
                    destination: Some(destination),
                    contract,
                    ..
                } if destination == return_call::RETURN_RESULT_PLACE
                    && (return_call::escapes_only_fresh_custody(contract.result)
                        || (admit_derived
                            && matches!(contract.result, ResourceReturnMode::Derived { .. }))))
            });
            self.record_return_call_access(&texts, line, Form::WholeCall, accepted.is_some());
            let (reads, call) = accepted?;
            return Some(self.lowered_return(expression, &texts, line, reads, call, true));
        }
        let (call_index, call_end) = crate::legacy_condition_call::unconditional_call_span(&texts)?;
        let externally_declared = find_function_body(self.tokens, texts[call_index]).is_none();
        let outside = texts[..call_index]
            .iter()
            .chain(texts[call_end..].iter())
            .copied()
            .collect::<Vec<_>>();
        if !externally_declared
            || !crate::typed_field_access::is_scalar_operand_expression(
                &outside,
                &self.typed_model,
                &self.typed_types,
            )
        {
            return None;
        }
        let accepted = self.lowered_call(&expression[call_index..call_end], |instruction| {
            matches!(instruction, LegacyInstruction::ContractCall {
                destination: None,
                contract,
                ..
            } if contract.result == ResourceReturnMode::None)
        });
        self.record_return_call_access(&texts, line, Form::ScalarCall, accepted.is_some());
        let (reads, call) = accepted?;
        Some(self.lowered_return(expression, &texts, line, reads, call, false))
    }

    /// Classify `tokens`; when the gate refused it, ask rule W44 and accept
    /// its block only when its call satisfies `accept`. Every counter is
    /// restored otherwise.
    fn lowered_call(
        &mut self,
        tokens: &[Token],
        accept: impl Fn(&LegacyInstruction) -> bool,
    ) -> Option<(Vec<LegacyInstruction>, LegacyInstruction)> {
        let saved = self.probe_state();
        let instruction = self.classify_simple(tokens);
        let lowered = is_gate_refusal(&instruction)
            .then(|| self.call_argument_access(tokens, &instruction))
            .flatten()
            .and_then(|mut sequence| {
                let call = sequence.pop()?;
                accept(&call).then_some((sequence, call))
            });
        if lowered.is_none() {
            self.restore_probe_state(&saved);
        }
        lowered
    }

    /// The reads, then the existing return of the call's value.
    fn lowered_return(
        &self,
        expression: &[Token],
        texts: &[&str],
        line: usize,
        reads: Vec<LegacyInstruction>,
        call: LegacyInstruction,
        escapes: bool,
    ) -> Statement {
        let mut block = reads
            .into_iter()
            .map(|instruction| Statement::Instruction(instruction, line))
            .collect::<Vec<_>>();
        block.push(Statement::Return {
            line,
            writes: self.expression_writes(expression),
            expression_supported: false,
            returned_place: canonical_place(texts),
            returned_evaluation: Some(Box::new(call)),
            returned_evaluation_escapes: escapes,
            returns_null_constant: false,
        });
        Statement::Block(block)
    }

    /// Parity only: one row per consultation.
    fn record_return_call_access(&self, texts: &[&str], line: usize, form: Form, admitted: bool) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        crate::typed_access_path::record_row(serde_json::json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts,
            "position": "return-call-access",
            "rule": "12i",
            "form": form.code(),
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": admitted,
            "disagreement": admitted,
        }));
    }
}
