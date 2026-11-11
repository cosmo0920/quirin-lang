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

//! Card **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** round 2 (gateway rules 14a and
//! 14b): two statement shapes the lexical gate refused and no older route
//! read, asked after every statement fallback and before rule 12k, only for
//! exactly the gate's refusal of a statement with no braces that reads no
//! tracked descriptor. The Clang readings are
//! [`crate::residual_access_reading`]'s.
//!
//! * **Rule 14a.** A scalar statement whose value holds exactly one call is
//!   lowered as the call alone (classified as the statement `f(args)`, and
//!   when the gate refuses its actuals, rule W44's block: the typed reads of
//!   the bases its actuals dereference, their second obligation, the call),
//!   then the statement's own typed reads and store with the call's value an
//!   opaque scalar. Only when the callee's contract in force (no review, pin
//!   or refutation hold) has a `None` result and every declared parameter
//!   `Borrowed`, and the call lowers to exactly that contracted call with no
//!   destination: the call then takes and releases no custody and binds
//!   nothing, so no read the statement makes can be decided differently on
//!   either side of it, which is all C's unspecified operand order could
//!   change. Every other answer keeps the gate's refusal byte for byte, with
//!   the counters the lowering advanced restored.
//! * **Rule 14b.** A pointer local given an interior pointer of a path whose
//!   every root is never bound is the target-free borrowed alias a pointer
//!   read binds, with the path's per-hop obligations.
//!
//! The Lean model is `QCore.ResidualAccessReadings`.

use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};

use super::{
    LegacyInstruction, Parser, Statement, Token, contains_braces, pin_hold, refuted_contract,
    return_call_access::is_gate_refusal, review_hold,
};
use crate::residual_access_reading::InteriorForm;
use crate::typed_access_path::StatementPlan;

/// What rule 14a or 14b answered, for the parity row.
enum Answer {
    Interior(InteriorForm),
    ScalarCall(String),
}

impl Parser<'_> {
    /// Rules 14a and 14b: the statement's lowering, or `None` (the gate's
    /// refusal stands and every counter is as before).
    pub(super) fn residual_access_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        if !is_gate_refusal(instruction) || contains_braces(tokens) {
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
        let first_line = tokens.first()?.line;
        let decisions = crate::consultation_decisions::Decisions::new();
        let reading = self.residual_reading(tokens, &texts, first_line, &decisions);
        decisions.record_places(
            &self.typed_path_function,
            first_line,
            &texts,
            "residual-access",
            reading.is_some(),
        );
        let (plan, answer, call) = reading?;
        let mut block = Vec::new();
        if let Some((callee, call_tokens)) = call {
            let Some(lowered) = self.lowered_scalar_call(tokens, &call_tokens, &callee) else {
                self.record_residual_access(&texts, line, &Answer::ScalarCall(callee), false);
                return None;
            };
            block.extend(lowered);
        }
        block.push(self.plan_instruction(plan));
        self.record_residual_access(&texts, line, &answer, true);
        Some(Statement::Block(
            block
                .into_iter()
                .map(|instruction| Statement::Instruction(instruction, line))
                .collect(),
        ))
    }

    /// The Clang reading: rule 14b's plan, or rule 14a's plan with the call's
    /// callee and token text.
    #[allow(clippy::type_complexity)]
    fn residual_reading(
        &self,
        tokens: &[Token],
        texts: &[&str],
        line: usize,
        decisions: &crate::consultation_decisions::Decisions,
    ) -> Option<(StatementPlan, Answer, Option<(String, Vec<String>)>)> {
        let index = self.typed_positions_index()?;
        let occurrence =
            || crate::typed_place::occurrence(self.tokens, self.typed_place_origin.get(), tokens);
        let node = index.statement_node(texts, line, &occurrence)?;
        let never_bound = |name: &str| self.joined_never_bound(name);
        if let Some((plan, form)) =
            index.interior_pointer_statement(node, &never_bound, Some(decisions))
        {
            return Some((plan, Answer::Interior(form), None));
        }
        let reading = index.scalar_call_statement(node, &never_bound, Some(decisions))?;
        let call_tokens = index.node_tokens(reading.call)?;
        let callee = reading.callee.clone();
        Some((
            reading.plan,
            Answer::ScalarCall(callee.clone()),
            Some((callee, call_tokens)),
        ))
    }

    /// Rule 14a's call: the contracted call with no destination, after rule
    /// W44's reads when the gate refused its actuals; `None` restores every
    /// counter.
    fn lowered_scalar_call(
        &mut self,
        tokens: &[Token],
        call_texts: &[String],
        callee: &str,
    ) -> Option<Vec<LegacyInstruction>> {
        let contract = self.contracts.contract(callee)?;
        // The reading found exactly one call.
        if !crate::residual_access_reading::scalar_call_admitted(
            1,
            contract.result == ResourceReturnMode::None,
            contract
                .parameters
                .iter()
                .all(|mode| matches!(mode, ResourceParameterMode::Borrowed)),
        ) || review_hold::held_callee(callee).is_some()
            || pin_hold::held_callee(callee).is_some()
            || refuted_contract::held_callee(callee).is_some()
            || super::caller_frame_hold::held_callee(callee).is_some()
            || super::conflict_hold::held_callee(callee).is_some()
        {
            return None;
        }
        let start = (0..tokens.len()).find(|&start| {
            tokens.len() - start >= call_texts.len()
                && tokens[start..start + call_texts.len()]
                    .iter()
                    .zip(call_texts)
                    .all(|(token, text)| token.text == *text)
        })?;
        let call_tokens = &tokens[start..start + call_texts.len()];
        let saved = self.probe_state();
        let instruction = self.classify_simple(call_tokens);
        let accepted = |call: &LegacyInstruction| {
            matches!(call, LegacyInstruction::ContractCall { function, destination: None, contract, .. }
                if function == callee && contract.result == ResourceReturnMode::None)
        };
        let lowered = if accepted(&instruction) {
            Some(vec![instruction])
        } else if is_gate_refusal(&instruction) {
            self.call_argument_access(call_tokens, &instruction)
                .filter(|sequence| sequence.last().is_some_and(accepted))
        } else {
            None
        };
        if lowered.is_none() {
            self.restore_probe_state(&saved);
        }
        lowered
    }

    /// The plan's typed reads and store, as the typed statement route lowers
    /// a plan with no pointer store and no readback.
    fn plan_instruction(&self, plan: StatementPlan) -> LegacyInstruction {
        if plan.accesses == 0 {
            return match plan.scalar.destination {
                Some(destination) => LegacyInstruction::ForgetScalar { destination },
                None => LegacyInstruction::NoOp,
            };
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
            destination: plan.scalar.destination,
            frame: plan
                .scalar
                .writes_through_pointer
                .then(|| super::RefusedWrites::Sparing(self.frame_protected.clone())),
            chains: plan.chains,
            borrowed_destination: plan.borrowed_destination,
            readback: None,
        }
    }

    /// Parity only: one row per reading that matched a shape.
    fn record_residual_access(&self, texts: &[&str], line: usize, answer: &Answer, admitted: bool) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let (rule, form, callee) = match answer {
            Answer::Interior(form) => ("14b", form.code(), None),
            Answer::ScalarCall(callee) => ("14a", "scalar-call", Some(callee.as_str())),
        };
        crate::typed_access_path::record_row(serde_json::json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts,
            "position": "residual-access",
            "rule": rule,
            "form": form,
            "callee": callee,
            "old_route": "old_refusal",
            "old_admitted": false,
            "new_admitted": admitted,
            "disagreement": admitted,
        }));
    }
}
