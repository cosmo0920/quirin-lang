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

//! Card **MEMBER-EFFECT-STORES**, body half (gateway rule 38k): the parser's
//! half and the flow of the hand-off store. The conditions, their formulas,
//! the premise and the decision are [`crate::member_handoff`].
//!
//! Asked only for exactly the gate's refusal of a statement with no braces
//! that reads no tracked descriptor, after rule 72 and before rule 12k. The
//! statement `P->F… = V` is read from the Clang AST of the store and the
//! fused tokens of the body; every failing condition keeps the refusal (rule
//! 38i's name of it where rule 38i names it), with every counter restored.
//!
//! The lowering is the destination's per-hop read, the stored call's actual
//! reads (rule W44's, as rule 72 lowers them), then a rule-10v
//! `PointerStore` of value class [`crate::member_handoff::HANDOFF`] whose
//! access is the stored call bound to [`crate::member_handoff::VALUE_NAME`]
//! (or nothing, for a local) and whose fallback is the gate's refusal.

use std::collections::BTreeSet;

use qcore_schemas::contract::{
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};
use serde_json::Value;

use super::{
    LegacyInstruction, Parser, Statement, Token, call_argument_access::GATE_REFUSAL,
    contains_braces, pin_hold, refuted_contract, return_call_access::is_gate_refusal, review_hold,
    strip_parentheses,
};
use crate::indirect_chain_access::{AccessChain, Owner, chain_owner};
use crate::member_handoff::{self as rule, Facts, Reason, ValueKind};
use crate::typed_access_path::{is_pointer_type, qual};
use crate::typed_c_ast::{children, kind};
use crate::typed_field_access::is_name;
use crate::typed_place::{RootKind, StepKind};
use crate::union_bitfield::MemberStorage;

/// The index of the statement's only top-level simple `=` (raw tokens).
fn simple_assignment(texts: &[&str]) -> Option<usize> {
    let mut depth = 0usize;
    let mut found = None;
    for (index, text) in texts.iter().enumerate() {
        match *text {
            "(" | "[" => depth += 1,
            ")" | "]" => depth = depth.checked_sub(1)?,
            "=" if depth == 0 => {
                let previous = index.checked_sub(1).map(|p| texts[p]);
                if texts.get(index + 1) == Some(&"=")
                    || previous.is_some_and(|p| {
                        matches!(
                            p,
                            "=" | "!" | "<" | ">" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^"
                        )
                    })
                    || found.is_some()
                {
                    return None;
                }
                found = Some(index);
            }
            _ => {}
        }
    }
    found
}

/// The lexical value: its kind's spelling, after parentheses and leading
/// casts `( TYPE * )`. `Ok((tokens offset, is_call, name))`.
fn lexical_value<'a>(value: &'a [&'a str]) -> Option<(usize, bool, &'a str)> {
    let mut offset = 0usize;
    let mut rest = value;
    loop {
        let stripped = strip_parentheses(rest);
        // `strip_parentheses` drops balanced outer pairs only.
        offset += (rest.len() - stripped.len()) / 2;
        rest = stripped;
        if rest.first() == Some(&"(")
            && let Some(close) = rule::matching(rest, 0, "(", ")")
            && close + 1 < rest.len()
            && rest[close - 1] == "*"
            && rest[1..close]
                .iter()
                .all(|t| *t == "*" || t.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()))
        {
            offset += close + 1;
            rest = &rest[close + 1..];
            continue;
        }
        break;
    }
    match rest {
        [name] if is_name(name) => Some((offset, false, name)),
        [callee, "(", .., ")"]
            if is_name(callee) && rule::matching(rest, 1, "(", ")") == Some(rest.len() - 1) =>
        {
            Some((offset, true, callee))
        }
        _ => None,
    }
}

/// Whether `callee` has a hold: its refusal is its own.
fn held(callee: &str) -> bool {
    review_hold::held_callee(callee).is_some()
        || pin_hold::held_callee(callee).is_some()
        || refuted_contract::held_callee(callee).is_some()
        || super::caller_frame_hold::held_callee(callee).is_some()
        || super::conflict_hold::held_callee(callee).is_some()
}

/// The stored value as Clang reads it: the bare node, and whether every cast
/// around it is pointer-to-pointer (`BitCast`, `NoOp`), with the rvalue read
/// of a name.
fn peel_value(mut node: &Value) -> (&Value, bool) {
    let mut casts = true;
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" => match node["castKind"].as_str() {
                Some("BitCast" | "NoOp" | "LValueToRValue") => {}
                _ => {
                    casts = false;
                    return (node, casts);
                }
            },
            "CStyleCastExpr" => match node["castKind"].as_str() {
                Some("BitCast" | "NoOp") => {}
                _ => {
                    casts = false;
                    return (node, casts);
                }
            },
            _ => return (node, casts),
        }
        match children(node).first() {
            Some(child) => node = child,
            None => return (node, false),
        }
    }
}

/// The direct callee of a `CallExpr`, when it is a function designator.
fn direct_callee(call: &Value) -> Option<&str> {
    let mut callee = children(call).first()?;
    while matches!(kind(callee), "ImplicitCastExpr" | "ParenExpr") {
        if kind(callee) == "ImplicitCastExpr"
            && callee["castKind"].as_str() != Some("FunctionToPointerDecay")
        {
            return None;
        }
        callee = children(callee).first()?;
    }
    (kind(callee) == "DeclRefExpr"
        && callee["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
    .then(|| callee["referencedDecl"]["name"].as_str())
    .flatten()
}

impl Parser<'_> {
    /// The gate's refusal a candidate statement stands for: `instruction`
    /// itself, or the fallback of W45's sub-object store of a name into a
    /// pointer member (W45 admits it only for a value with no identity, so it
    /// never admits a hand-off; the local's reading classifies nothing, so no
    /// counter moves when rule 38k takes the statement instead).
    fn member_handoff_refusal(instruction: &LegacyInstruction) -> &LegacyInstruction {
        match instruction {
            LegacyInstruction::SubObjectStore {
                destination,
                value: super::sub_object_store::StoredValue::Name(_),
                fallback,
                ..
            } if destination.member == super::sub_object_store::MemberKind::Pointer
                && destination.base.is_some() =>
            {
                fallback
            }
            other => other,
        }
    }

    /// Rule 38k: the lowering of the store `tokens` (refused by the gate as
    /// `instruction`, the run's first token at `origin`), or `None`.
    pub(super) fn member_handoff_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        origin: Option<usize>,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let sub_object = matches!(instruction, LegacyInstruction::SubObjectStore { .. });
        let LegacyInstruction::Unknown {
            description,
            writes,
        } = Self::member_handoff_refusal(instruction)
        else {
            return None;
        };
        if description != GATE_REFUSAL || contains_braces(tokens) {
            return None;
        }
        let origin = origin?;
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
        // The run is the stream's own tokens at `origin`.
        if self
            .tokens
            .get(origin..origin + tokens.len())
            .is_none_or(|run| run.iter().zip(&texts).any(|(t, text)| t.text != *text))
        {
            return None;
        }
        let equal = simple_assignment(&texts)?;
        let left = strip_parentheses(&texts[..equal]);
        let [base, "-", ">", first, rest @ ..] = left else {
            return None;
        };
        if !is_name(base)
            || !is_name(first)
            || rest.len() % 2 != 0
            || !rest
                .chunks(2)
                .all(|pair| pair[0] == "." && is_name(pair[1]))
        {
            return None;
        }
        let mut place = vec![(*first).to_owned()];
        place.extend(rest.chunks(2).map(|pair| pair[1].to_owned()));
        let spelled = format!("{base}->{}", place.join("."));
        let (value_offset, is_call, value_name) = lexical_value(&texts[equal + 1..])?;
        if sub_object && is_call {
            return None;
        }
        let base = (*base).to_owned();
        let value_name = value_name.to_owned();
        // A candidate: from here every answer is recorded.
        let reading = self.member_handoff_reading(
            tokens,
            &texts,
            line,
            origin,
            &base,
            &place,
            equal + 1 + value_offset,
            is_call,
            &value_name,
        );
        let (facts, access, callee_tokens) = match reading {
            Ok(reading) => reading,
            Err(reason) => {
                rule::record(&self.typed_path_function, line, &spelled, Err(reason));
                return None;
            }
        };
        if let Err(reason) = rule::decide(&facts) {
            rule::record(&self.typed_path_function, line, &spelled, Err(reason));
            return None;
        }
        let saved = self.probe_state();
        let (sequence, stored, name) = match callee_tokens {
            Some((start, end)) => {
                let mut synthetic = vec![
                    Token {
                        text: rule::VALUE_NAME.to_owned(),
                        line,
                    },
                    Token {
                        text: "=".to_owned(),
                        line,
                    },
                ];
                synthetic.extend_from_slice(&tokens[start..end]);
                let classified = self.classify_simple(&synthetic);
                let mut sequence = match classified {
                    LegacyInstruction::ContractCall { .. } => vec![classified],
                    ref refused if is_gate_refusal(refused) => self
                        .call_argument_access(&synthetic, refused)
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                let call = sequence.pop();
                let admitted = matches!(
                    &call,
                    Some(LegacyInstruction::ContractCall {
                        function,
                        destination,
                        contract,
                        container,
                        retention,
                        ..
                    }) if *function == value_name
                        && destination.as_deref() == Some(rule::VALUE_NAME)
                        && matches!(contract.result, ResourceReturnMode::Owned { .. })
                        && container.is_none()
                        && retention.is_none()
                );
                let Some(call) = call.filter(|_| admitted) else {
                    self.restore_probe_state(&saved);
                    rule::record(
                        &self.typed_path_function,
                        line,
                        &spelled,
                        Err(Reason::ValueShape),
                    );
                    return None;
                };
                (sequence, call, rule::VALUE_NAME.to_owned())
            }
            None => (Vec::new(), LegacyInstruction::NoOp, value_name.clone()),
        };
        rule::record(&self.typed_path_function, line, &spelled, Ok(()));
        // The fallback is the gate's refusal, under rule 38i's name where
        // rule 38i names it.
        let fallback = match self.member_custody_attribution(
            tokens,
            line,
            Statement::Instruction(
                LegacyInstruction::Unknown {
                    description: GATE_REFUSAL.to_owned(),
                    writes: writes.clone(),
                },
                line,
            ),
        ) {
            Statement::Instruction(instruction, _) => instruction,
            _ => LegacyInstruction::Unknown {
                description: GATE_REFUSAL.to_owned(),
                writes: writes.clone(),
            },
        };
        // Read over W45's sub-object store, its own instruction is the
        // fallback: W45 then decides a value with no identity as before.
        let fallback = if sub_object {
            instruction.clone()
        } else {
            fallback
        };
        let (read, chain) = access;
        let mut lowered = vec![read];
        lowered.extend(sequence);
        lowered.push(LegacyInstruction::PointerStore {
            store: crate::pointer_store::PointerStore {
                root: base,
                place: spelled,
                owner: Some(chain),
                value: rule::HANDOFF,
                name: Some(name),
            },
            access: Box::new(stored),
            fallback: Box::new(fallback),
        });
        Some(Statement::Block(
            lowered
                .into_iter()
                .map(|instruction| Statement::Instruction(instruction, line))
                .collect(),
        ))
    }

    /// The parser's facts of one candidate, the destination's read and chain,
    /// and the stored call's token range in `tokens`; `Err` names a refusal
    /// read before the decision could be asked.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn member_handoff_reading(
        &self,
        tokens: &[Token],
        texts: &[&str],
        line: usize,
        origin: usize,
        base: &str,
        place: &[String],
        value_at: usize,
        is_call: bool,
        value_name: &str,
    ) -> Result<
        (
            Facts,
            (LegacyInstruction, AccessChain),
            Option<(usize, usize)>,
        ),
        Reason,
    > {
        // The stored value's kind, from the contract in force.
        let value = if is_call {
            if held(value_name) {
                return Err(Reason::ValueShape);
            }
            match self.contracts.contract(value_name).map(|c| c.result) {
                Some(ResourceReturnMode::Owned { .. }) => ValueKind::OwnedResult,
                Some(ResourceReturnMode::Reallocated { nullable, .. }) => {
                    if nullable {
                        return Err(Reason::ReallocSelf);
                    }
                    ValueKind::Reallocated
                }
                _ => ValueKind::Other,
            }
        } else if self.formal_parameters.contains(value_name) {
            ValueKind::Other
        } else {
            ValueKind::LocalAcquisition
        };
        if !self.formal_parameters.contains(base) {
            return Err(Reason::LocalAliasOfFormal);
        }
        // The body, fused.
        let function = self.typed_path_function.clone();
        let (body_start, _) =
            super::find_function_body(self.tokens, &function).ok_or(Reason::PlaceShape)?;
        let body_end =
            super::matching(self.tokens, body_start, "{", "}").ok_or(Reason::PlaceShape)?;
        if origin <= body_start || origin + texts.len() >= body_end {
            return Err(Reason::PlaceShape);
        }
        let raw = self.tokens[body_start + 1..body_end]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let fused = crate::legacy_c::container_publication::fuse(&raw);
        let body = fused
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>();
        let at = |raw_index: usize| {
            fused
                .iter()
                .position(|(_, original)| *original == raw_index)
        };
        let relative = origin - body_start - 1;
        let base_raw = relative
            + texts
                .iter()
                .position(|text| *text == base)
                .ok_or(Reason::PlaceShape)?;
        let store_base = at(base_raw).ok_or(Reason::PlaceShape)?;
        let store_end = at(relative + texts.len()).ok_or(Reason::PlaceShape)?;
        if body.get(store_end) != Some(&";") {
            return Err(Reason::PlaceShape);
        }
        let not_in_loop = rule::loop_ranges(&body).is_some_and(|ranges| {
            !ranges
                .iter()
                .any(|&(open, close)| open <= store_base && store_base <= close)
        });
        let place_answer = rule::place_and_formal(&body, base, place, store_base, store_end);
        let local_kept = match value {
            ValueKind::LocalAcquisition => {
                let value_raw = relative + value_at;
                let value_index = at(value_raw).ok_or(Reason::LocalUsed)?;
                rule::local_kept(&body, value_name, value_index, store_end).is_ok()
            }
            _ => true,
        };
        let calls_borrowed = rule::calls_after(&body, store_end).iter().all(|callee| {
            !held(callee)
                && self.contracts.contract(callee).is_some_and(|contract| {
                    contract
                        .parameters
                        .iter()
                        .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
                        && contract.container_effects.is_empty()
                })
        });
        let nothing_released_after = calls_borrowed
            && !rule::indirect_after(&body, base, store_end)
            && place_answer != Err(Reason::BorrowedUseAfter);
        // The Clang reading of the store.
        let index = self.typed_positions_index().ok_or(Reason::PlaceShape)?;
        self.typed_place_origin.set(Some((origin, tokens.len())));
        let occurrence =
            || crate::typed_place::occurrence(self.tokens, self.typed_place_origin.get(), tokens);
        let node = index.statement_node(texts, line, &occurrence);
        self.typed_place_origin.set(None);
        let node = node.ok_or(Reason::PlaceShape)?;
        if kind(node) != "BinaryOperator" || node["opcode"].as_str() != Some("=") {
            return Err(Reason::PlaceShape);
        }
        let [left, right] = children(node) else {
            return Err(Reason::PlaceShape);
        };
        let never_bound = |name: &str| self.joined_never_bound(name);
        let mut reader = index.reader(&never_bound);
        let destination = crate::typed_place::read_place(left, reader.fields())
            .map_err(|_| Reason::PlaceShape)?;
        let formal_root = destination.decl_kind == "ParmVarDecl";
        let place_shape = destination.root == RootKind::Pointer
            && destination.name == base
            && destination.steps.len() == place.len()
            && destination.steps.iter().enumerate().all(|(at, step)| {
                step.storage == MemberStorage::Ordinary
                    && step.member == place[at]
                    && step.kind
                        == if at == 0 {
                            StepKind::Arrow
                        } else {
                            StepKind::Dot
                        }
            });
        let (bare, pointer_casts) = peel_value(right);
        let value_matches = match (value, kind(bare)) {
            (ValueKind::OwnedResult | ValueKind::Reallocated, "CallExpr") => {
                direct_callee(bare) == Some(value_name)
            }
            (ValueKind::LocalAcquisition, "DeclRefExpr") => {
                let decl = &bare["referencedDecl"];
                decl["kind"].as_str() == Some("VarDecl")
                    && decl["name"].as_str() == Some(value_name)
                    && qual(bare).is_some_and(is_pointer_type)
            }
            _ => false,
        };
        let value = if value_matches {
            value
        } else {
            ValueKind::Other
        };
        let pointer_casts = pointer_casts && qual(right).is_some_and(is_pointer_type);
        // The destination's chain obligation, as rule 10v reads it.
        reader.pointer_store_target = true;
        reader.path(left, true, false).ok_or(Reason::PlaceShape)?;
        let plan = reader.plan;
        let [chain] = plan.chains.as_slice() else {
            return Err(Reason::PlaceShape);
        };
        let chain = chain.clone();
        let read = LegacyInstruction::IndirectScalarAccess {
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
        };
        let callee_tokens = if is_call {
            let start = value_at;
            let close = rule::matching(texts.get(start + 1..).unwrap_or_default(), 0, "(", ")")
                .ok_or(Reason::ValueShape)?;
            Some((start, start + 2 + close))
        } else {
            None
        };
        let facts = Facts {
            value,
            nonnull_clause: value == ValueKind::Reallocated,
            pointer_casts,
            formal_root,
            place_shape,
            formal_kept: self.caller_formals.contains(base),
            not_in_loop,
            place_only_at_store: place_answer != Err(Reason::PlaceUsed),
            local_kept,
            nothing_released_after,
            // Decided on the flow.
            untracked_owner: true,
            old_value_unbound: true,
            value_live_owed: true,
        };
        Ok((facts, (read, chain), callee_tokens))
    }
}

/// The owner class rule 10v reads: both the root and the place's object
/// untracked.
fn owner_untracked(state: &super::AbstractState, chain: &AccessChain) -> bool {
    let root = AccessChain {
        links: Vec::new(),
        ..chain.clone()
    };
    matches!(
        (chain_owner(state, &root), chain_owner(state, chain)),
        (Ok(Owner::Untracked), Ok(Owner::Untracked))
    )
}

/// `mes_h3_no_stale_value` on the state after the stored value is bound: the
/// value is one live identity of this flow (possibly null on its path).
fn value_live_owed(state: &super::AbstractState, name: &str) -> bool {
    let Some(pointer) = state.pointers.get(name) else {
        return false;
    };
    // Exactly one identity: a target-free binding is a borrowed alias as
    // well as a null value, so it is never read as either.
    if pointer.unknown || !pointer.derivation.is_identity() || pointer.targets.len() != 1 {
        return false;
    }
    pointer.targets.iter().all(|site| {
        state.resources.get(site).is_some_and(|status| {
            status & super::LIVE != 0
                && status & (super::RELEASED | super::ESCAPED | super::CONTAINED) == 0
        }) && !state.unresolved_effect_sites.contains(site)
    })
}

/// The consumption a hand-off applies: `resource_consumed(v, nullable)`.
fn consumption(name: &str) -> LegacyInstruction {
    LegacyInstruction::ContractCall {
        function: rule::CONSUMER.to_owned(),
        destination: None,
        arguments: vec![Some(name.to_owned())],
        contract: ResourceFunctionContract::new(
            [ResourceParameterMode::Consumed { nullable: true }],
            ResourceReturnMode::None,
        ),
        produced_sites: Vec::new(),
        aggregate_reads: Vec::new(),
        aggregate_consumed: Vec::new(),
        aggregate_addresses: Vec::new(),
        alias_reads: Vec::new(),
        nested: Vec::new(),
        container: None,
        retention: None,
    }
}

/// The flow of a rule-38k store (asked by rule 10v's flow for the value
/// class [`rule::HANDOFF`]): the owner and old value on the state at the
/// node, the stored value after its access, then the consumption and rule
/// 10v's forgetting. Every failing premise makes the node its fallback.
pub(super) fn transfer(
    function: &str,
    node: &super::LegacyCfgNode,
    state: super::AbstractState,
    diagnostics: &mut BTreeSet<super::LegacyDiagnostic>,
    base: &dyn Fn(
        &super::LegacyCfgNode,
        super::AbstractState,
        &mut BTreeSet<super::LegacyDiagnostic>,
    ) -> super::AbstractState,
    dispatch: &dyn Fn(
        &super::LegacyCfgNode,
        super::AbstractState,
        &mut BTreeSet<super::LegacyDiagnostic>,
    ) -> super::AbstractState,
) -> super::AbstractState {
    let super::LegacyInstruction::PointerStore {
        store,
        access,
        fallback,
    } = &node.instruction
    else {
        return base(node, state, diagnostics);
    };
    let refuse = |reason: Reason, state, diagnostics: &mut BTreeSet<_>| {
        rule::record(function, node.line, &store.place, Err(reason));
        dispatch(&super::with_instruction(node, fallback), state, diagnostics)
    };
    let (Some(chain), Some(name)) = (&store.owner, &store.name) else {
        return refuse(Reason::PlaceShape, state, diagnostics);
    };
    if !owner_untracked(&state, chain) {
        return refuse(Reason::Owner, state, diagnostics);
    }
    if state.pointers.iter().any(|(place, pointer)| {
        super::pointer_store_flow::rooted_at(place, &store.root)
            && (!pointer.targets.is_empty() || pointer.unknown)
    }) {
        return refuse(Reason::OwnedOldValue, state, diagnostics);
    }
    let mut scratch = BTreeSet::new();
    let bound = base(
        &super::with_instruction(node, access),
        state.clone(),
        &mut scratch,
    );
    if !value_live_owed(&bound, name) {
        return refuse(Reason::StaleValue, state, diagnostics);
    }
    diagnostics.extend(scratch);
    let mut state = base(
        &super::with_instruction(node, &consumption(name)),
        bound,
        diagnostics,
    );
    state
        .pointers
        .retain(|place, _| !super::pointer_store_flow::rooted_at(place, &store.root));
    if name == rule::VALUE_NAME {
        state.pointers.remove(name);
    }
    state
}
