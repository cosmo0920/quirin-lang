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

//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the return-expression routes
//! (ROUTE-MAP section 4, step 2): W4 return call, B01, W43 and W45
//! families, 10k, 11h.
//!
//! Moved verbatim from `legacy_c.rs` (only visibility widened to
//! `pub(super)` where the parent or a sibling module reads an item).

use super::*;

impl Parser<'_> {
    /// Card GAPO-RETURN-CORRECTNESS (rule 15z), parity runs only: one row
    /// (`position: "return-route-gap"`) per lowered `return` whose evaluation
    /// is a refusal or an admission a statement-only rule reads: rule 11w's
    /// tail refusal (`11w-tail`) or `va_*` refusal (`11w-va`), rule 10u's K03
    /// refusal (`10u-k03`) or an admitted call with an unbound custody-taking
    /// position (`10u-unbound`, which the flow refuses as `arguments do not
    /// match resource contract`). A refused return with no evaluation names
    /// no reason, so the count is a lower bound. It changes nothing.
    pub(super) fn record_return_route_gaps(
        &self,
        expression: &[Token],
        line: usize,
        statement: &Statement,
    ) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let Statement::Return {
            returned_evaluation: Some(evaluation),
            ..
        } = statement
        else {
            return;
        };
        let gap = match evaluation.as_ref() {
            LegacyInstruction::Unknown { description, .. }
                if description.starts_with(variadics_route::TAIL_PREFIX) =>
            {
                "11w-tail"
            }
            LegacyInstruction::Unknown { description, .. }
                if variadics_route::refused_va_builtin(description).is_some() =>
            {
                "11w-va"
            }
            LegacyInstruction::Unknown { description, .. }
                if description == call_instantiation_route::K03_REFUSAL =>
            {
                "10u-k03"
            }
            LegacyInstruction::ContractCall {
                contract,
                arguments,
                ..
            } if contract.parameters.iter().enumerate().any(|(index, mode)| {
                crate::call_instantiation::custody_taking(*mode)
                    && arguments.get(index).is_none_or(Option::is_none)
            }) =>
            {
                "10u-unbound"
            }
            _ => return,
        };
        crate::typed_access_path::record_row(serde_json::json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": expression.iter().map(|token| token.text.as_str()).collect::<Vec<_>>(),
            "position": "return-route-gap",
            "gap": gap,
            "new_admitted": false,
        }));
    }

    /// Card **C-GATEWAY-RETURN-CALL-W4**: the modeled call a `return`
    /// expression is, desugared to the assignment the analyzer already decides.
    ///
    /// The whole expression must be exactly one call, the reserved synthetic
    /// place must be unspelled in this stream, and the answer
    /// [`Self::classify_simple`] gives to `PLACE = <expression>` must be a
    /// [`LegacyInstruction::ContractCall`] into that place whose result mode
    /// escapes only custody the call produced. Anything else answers `None` and
    /// the `return` keeps the refusal it had.
    ///
    /// `classify_simple` moves more than `next_site` (the element lowerings,
    /// the element origins, the argument probe and the other
    /// [`probe_state::FieldClass::ProbeState`] fields; card
    /// GAPO-RETURN-CORRECTNESS, rule 15x, corrected the claim that `next_site`
    /// was its only mutation), so a declined probe hands back the whole
    /// [`probe_state::ParserProbeState`] and is observationally identical to
    /// never having run — which is what keeps site numbering, and therefore
    /// certificate identity, unmoved for every `return` this card refuses.
    /// The monotone facts of [`probe_state::FieldClass::MonotoneFact`] may
    /// only grow; the synthetic destination is a name no statement spells.
    pub(super) fn return_call_instruction(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<LegacyInstruction> {
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        return_call::whole_expression_call(expression, &texts)?;
        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE) {
            return None;
        }
        let mut synthetic = Vec::with_capacity(expression.len() + 2);
        synthetic.extend(return_call::assignment_prefix(line));
        synthetic.extend_from_slice(expression);
        let restore = self.probe_state();
        let instruction = self.classify_simple(&synthetic);
        // Card GAPO-RETURN-CORRECTNESS (rule 15z): rule 12h's H-U0, asked as
        // at a statement: a cast actual of a place is that place.
        let instruction = self
            .uncast_returned_call(
                &return_call::assignment_prefix(line),
                expression,
                &instruction,
                &restore,
            )
            .unwrap_or(instruction);
        let admitted = matches!(
            &instruction,
            LegacyInstruction::ContractCall {
                destination: Some(destination),
                contract,
                ..
            } if destination == return_call::RETURN_RESULT_PLACE
                && (return_call::escapes_only_fresh_custody(contract.result)
                    || (self.admit_derived_returns
                        && matches!(contract.result, ResourceReturnMode::Derived { .. })))
        );
        if admitted {
            Some(instruction)
        } else {
            self.restore_probe_state(&restore);
            None
        }
    }

    /// Card **B01-RETURN-EXPRESSION**: evaluate a refused return through the
    /// ordinary assignment classifier before falling back to the position's
    /// generic refusal.
    ///
    /// The pre-existing whole-call rule is asked first and unchanged. For the
    /// remaining expressions this rule admits two bounded shapes:
    ///
    /// * an `OpaqueValue` assigned to the reserved return place; the classifier
    ///   has established that the value carries no resource identity, so B01
    ///   emits a `NoOp` exit and performs no custody escape;
    /// * exactly one contracted call with `ResourceReturnMode::None` inside an
    ///   otherwise custody-neutral scalar expression. The call is classified
    ///   as a standalone statement so its parameter effects are applied, and
    ///   its resource-free result leaves nothing for the return to escape.
    ///
    /// Every refusal, including the classifier's own `Unknown`, keeps the old
    /// return-position wrapper and wording.
    ///
    /// A declined probe hands back the whole probe state (rule 15x), so a
    /// classification attempt cannot perturb later certificate identities.
    pub(super) fn return_expression_instruction(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<(LegacyInstruction, bool)> {
        // Card W45-PARENTHESISED-CALLEE.
        if self.opaque_callee(expression) {
            return None;
        }
        if let Some(call) = self.return_call_instruction(expression, line) {
            // Card CG03 (rule 10u): parity beside the whole-call admission.
            self.record_instantiation_parity(
                crate::call_instantiation::Consumer::Return,
                line,
                &call,
                crate::call_instantiation::ResultUse::Returned,
            );
            return Some((call, true));
        }
        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE) {
            return None;
        }
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if let Some((call_index, call_end)) =
            crate::legacy_condition_call::unconditional_call_span(&texts)
        {
            // This widening is for calls backed by a declared API contract.
            // A locally defined callee may acquire a provisional summary
            // during fixed-point analysis; treating that summary as the
            // declaration premise would weaken W4's pointer-actual boundary
            // for recursive expressions such as `1 + counted(p)`.
            let externally_declared = find_function_body(self.tokens, texts[call_index]).is_none();
            let outside = texts[..call_index]
                .iter()
                .chain(texts[call_end..].iter())
                .copied()
                .collect::<Vec<_>>();
            if externally_declared
                && crate::typed_field_access::is_scalar_operand_expression(
                    &outside,
                    &self.typed_model,
                    &self.typed_types,
                )
            {
                let restore = self.probe_state();
                let instruction = self.classify_simple(&expression[call_index..call_end]);
                // Card GAPO-RETURN-CORRECTNESS (rule 15z): rule 12h's H-U0.
                let instruction = self
                    .uncast_returned_call(
                        &[],
                        &expression[call_index..call_end],
                        &instruction,
                        &restore,
                    )
                    .unwrap_or(instruction);
                let admitted = matches!(
                    &instruction,
                    LegacyInstruction::ContractCall {
                        destination: None,
                        contract,
                        ..
                    } if contract.result == ResourceReturnMode::None
                );
                if admitted {
                    // Card CG03 (rule 10u): parity beside the B01 admission.
                    self.record_instantiation_parity(
                        crate::call_instantiation::Consumer::Return,
                        line,
                        &instruction,
                        crate::call_instantiation::ResultUse::Tested,
                    );
                    return Some((instruction, false));
                }
                self.restore_probe_state(&restore);
            }
        }
        let mut synthetic = Vec::with_capacity(expression.len() + 2);
        synthetic.extend(return_call::assignment_prefix(line));
        synthetic.extend_from_slice(expression);
        let restore = self.probe_state();
        let instruction = self.classify_simple(&synthetic);
        let direct_member_read = texts.iter().filter(|text| **text == ".").count()
            + texts.windows(2).filter(|pair| *pair == ["-", ">"]).count()
            == 1;
        let admitted = matches!(
            &instruction,
            LegacyInstruction::OpaqueValue { destination, .. }
                if destination == return_call::RETURN_RESULT_PLACE
        ) && direct_member_read;
        self.restore_probe_state(&restore);
        if admitted {
            return Some((instruction, false));
        }
        // Card W43-RETURN-EXPRESSION. Asked last, after every admitting route
        // above has declined: a whole-call return whose synthetic assignment
        // the classifier *refuses* is lowered to that refusal and a `NoOp`
        // exit, so the published reason names the call's own blocker. The
        // node stays `Unknown`, so this re-attributes and never admits; see
        // `return_call::lowers_refused_whole_call`.
        let refused = matches!(instruction, LegacyInstruction::Unknown { .. });
        if return_call::lowers_refused_whole_call(expression, &texts, refused) {
            return Some((instruction, false));
        }
        // Card W45-RETURN-EXPRESSION-FAMILIES. Asked after every route above
        // has declined; see `return_family`.
        if let Some(family) = self.return_family_instruction(expression, &texts, line) {
            self.record_typed_position_legacy(expression, "return", "return-family", &family.0);
            return Some(family);
        }
        // Card W47-ACCESS-PATH-POSITIONS (rule 10k).
        if let Some(typed) = self.typed_return_instruction(expression) {
            return Some(typed);
        }
        // Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h). Asked last:
        // the address of a static-duration object carries no identity.
        self.static_address_return(expression)
    }
}
