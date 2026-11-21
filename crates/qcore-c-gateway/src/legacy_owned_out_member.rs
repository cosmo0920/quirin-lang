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

//! Card **CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT** (gateway rule 72): the
//! parser's half, the lowering of a call whose out slot is handed a member
//! path of a caller-supplied object. The reading and the decision are
//! [`crate::owned_out_member`]; the flow is rule 10v's
//! ([`super::pointer_store_flow`]).
//!
//! Asked only for exactly the gate's refusal, after every route of the
//! statement (or the whole-call return) declined and before rule 12k's
//! re-attribution, for a statement `CALL`, `NAME = CALL` or `return CALL`
//! with no braces that reads no tracked descriptor, whose first call ends the
//! statement. It lowers to the slot's chain read, the call's own block (the
//! call classified under its minted contract with the slot actual replaced by
//! the null constant, which a `Borrowed` position reads as data, and its other
//! actuals read as rule W44 reads them), and a `PointerStore` of value class
//! [`crate::owned_out_member::OWNED_OUT`] whose access is the call and whose
//! fallback is the gate's refusal with the statement's writes. Anything else
//! keeps the refusal byte for byte.

use super::{
    LegacyInstruction, Parser, RefusedWrites, Statement, Token, call_argument_access::GATE_REFUSAL,
    contains_braces, find_call, return_call, return_call_access::is_gate_refusal,
};
use crate::owned_out_member::{OWNED_OUT, caller_roots, minted_contract, minted_name, read_slot};

impl Parser<'_> {
    /// Rule 72, the statement form: the lowering of `tokens` (refused by the
    /// gate as `instruction`), or `None`.
    pub(super) fn owned_out_member_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let LegacyInstruction::Unknown {
            description,
            writes,
        } = instruction
        else {
            return None;
        };
        if description != GATE_REFUSAL {
            return None;
        }
        let lowered = self.owned_out_member_lowering(tokens, writes.clone(), true)?;
        Some(Statement::Block(
            lowered
                .into_iter()
                .map(|instruction| Statement::Instruction(instruction, line))
                .collect(),
        ))
    }

    /// Rule 72, the whole-call return form `return CALL;`: the call's
    /// lowering, then a return whose scalar value escapes nothing.
    pub(super) fn owned_out_member_return(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        let texts = expression
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE)
            || return_call::whole_expression_call(expression, &texts).is_none()
        {
            return None;
        }
        let writes = self.expression_writes(expression);
        let lowered = self.owned_out_member_lowering(expression, writes.clone(), false)?;
        let mut block = lowered
            .into_iter()
            .map(|instruction| Statement::Instruction(instruction, line))
            .collect::<Vec<_>>();
        block.push(Statement::Return {
            line,
            writes,
            expression_supported: true,
            returned_place: None,
            returned_evaluation: None,
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        });
        Some(Statement::Block(block))
    }

    /// The block `tokens` lowers to under rule 72, or `None` (every counter
    /// restored). `assignment` admits the form `NAME = CALL`.
    fn owned_out_member_lowering(
        &mut self,
        tokens: &[Token],
        writes: RefusedWrites,
        assignment: bool,
    ) -> Option<Vec<LegacyInstruction>> {
        if contains_braces(tokens) {
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
        let (call_index, callee) = find_call(&texts)?;
        // The statement is the call, or one plain name assigned the call.
        match &texts[..call_index] {
            [] => {}
            [name, "="] if assignment && super::is_identifier(name) => {}
            _ => return None,
        }
        let spans = super::call_argument_spans(&texts, call_index)?;
        let close = spans.last().map_or(call_index + 2, |&(_, end)| end);
        if close + 1 != texts.len() || texts[close] != ")" {
            return None;
        }
        let (_, slot) = minted_contract(self.contracts, callee)?;
        let minted = minted_name(callee);
        let minted_contract = self.contracts.contract(&minted)?.clone();
        let &(start, end) = spans.get(slot)?;
        let fused = crate::legacy_c::container_publication::fuse(&texts[start..end]);
        let actual = fused
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>();
        let slot_actual = read_slot(&actual)?;
        if !self.owned_out_member_roots().contains(&slot_actual.root) {
            return None;
        }
        let callee = callee.to_owned();
        let mut synthetic = Vec::with_capacity(tokens.len());
        for (index, token) in tokens.iter().enumerate() {
            if index == call_index {
                synthetic.push(Token {
                    text: minted.clone(),
                    line: token.line,
                });
            } else if index == start {
                synthetic.push(Token {
                    text: "0".to_owned(),
                    line: token.line,
                });
            } else if !(start..end).contains(&index) {
                synthetic.push(token.clone());
            }
        }
        let saved = self.probe_state();
        let classified = self.classify_simple(&synthetic);
        let mut sequence = match classified {
            LegacyInstruction::ContractCall { .. } => vec![classified],
            ref refused if is_gate_refusal(refused) => self
                .call_argument_access(&synthetic, refused)
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let call = match sequence.pop() {
            Some(LegacyInstruction::ContractCall {
                function,
                destination,
                arguments,
                contract,
                produced_sites,
                aggregate_reads,
                aggregate_consumed,
                aggregate_addresses,
                alias_reads,
                nested,
                container,
                retention,
            }) if function == minted
                && contract == minted_contract
                && produced_sites.is_empty()
                && container.is_none()
                && retention.is_none() =>
            {
                LegacyInstruction::ContractCall {
                    function: callee,
                    destination,
                    arguments,
                    contract,
                    produced_sites,
                    aggregate_reads,
                    aggregate_consumed,
                    aggregate_addresses,
                    alias_reads,
                    nested,
                    container,
                    retention,
                }
            }
            _ => {
                self.restore_probe_state(&saved);
                return None;
            }
        };
        // Rule 11d: a root the never-bound closure does not cover (a pointer
        // read out of the caller's object, `opts = map->opts`) is read as
        // untracked only where the AST reader declares it exactly once (it is
        // never address-taken: premise P-72-CALLER-OBJECT) and the flow finds
        // its binding absent or target-free, nothing rooted at it bound and no
        // refused path reaching the call.
        let never_bound = self.position_never_bound(&slot_actual.root);
        let chain = slot_actual.chain(
            never_bound,
            !never_bound && self.position_answered(&slot_actual.root),
        );
        let mut lowered = vec![LegacyInstruction::IndirectScalarAccess {
            alias_reads: self
                .read_alias(Some(&slot_actual.root))
                .into_iter()
                .collect(),
            bases: Vec::new(),
            destination: None,
            frame: None,
            chains: vec![chain.clone()],
            borrowed_destination: false,
            readback: None,
            typed_alternative: None,
        }];
        lowered.extend(sequence);
        lowered.push(LegacyInstruction::PointerStore {
            store: crate::pointer_store::PointerStore {
                root: slot_actual.root.clone(),
                place: slot_actual.place(),
                owner: Some(chain),
                value: OWNED_OUT,
                name: None,
            },
            access: Box::new(call),
            fallback: Box::new(LegacyInstruction::Unknown {
                description: GATE_REFUSAL.to_owned(),
                writes,
            }),
        });
        Some(lowered)
    }

    /// The caller-supplied roots of this body (premise P-72-CALLER-OBJECT),
    /// read once from the body's own tokens.
    fn owned_out_member_roots(&self) -> &std::collections::BTreeSet<String> {
        self.owned_out_roots.get_or_init(|| {
            let Some(end) = super::matching(self.tokens, self.owned_out_body_start, "{", "}")
            else {
                return std::collections::BTreeSet::new();
            };
            let body = self.tokens[self.owned_out_body_start + 1..end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            let fused = crate::legacy_c::container_publication::fuse(&body);
            let body = fused
                .iter()
                .map(|(text, _)| text.as_str())
                .collect::<Vec<_>>();
            caller_roots(&body, &self.formal_parameters)
        })
    }
}
