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

//! Card **CG03-CALL-SUMMARY-INSTANTIATION** (gateway rule 10u): the single
//! call-contract application point of [`crate::call_instantiation`],
//! connected to the custody gateway (route G).
//!
//! # The instantiation API
//!
//! [`Parser::instantiate_call`] is the one place a consumer asks for a call's
//! contract to be applied: callee → contract in force (the classifier's one
//! lookup, with every withdrawal, hold and premise rewriting) → per-formal
//! substitution → the `ContractCall` instruction the flow applies once. It
//! then decides the instantiated call's **acceptance** with
//! [`crate::call_instantiation::accept`], from the callee's standing and the
//! use C makes of the call's value, never from the consumer. The condition
//! desugaring (rule 5h) asks it for every call of a leaf and reproduces its
//! older refusals exactly; the `return` operand route and the elaborator's
//! effectful-actual statement below are its new consumers. Cards CG04 and
//! CG05 call the same function.
//!
//! # The consumers this card adds, each asked last
//!
//! * **A scalar `return` operand** ([`Parser::instantiated_return`]): a
//!   `return` every older route refused, in a function whose definition
//!   declares a scalar result, whose expression is one condition leaf (no
//!   connective, as the condition desugaring reads it) holding calls. Each
//!   call is instantiated with use `Tested`, exactly as the same leaf in an
//!   `if` would be; two calls must each be the identity on the abstract
//!   state (rule 5h's premise); the residual must be what a condition leaf
//!   decides with a `NoOp`. The exit escapes nothing: the value is a scalar.
//! * **A call whose actuals carry effects** ([`Parser::effectful_call_piece`]):
//!   a statement piece that is exactly one direct call, whose actuals'
//!   events are admitted scalar reads and writes (the elaborator's graph,
//!   which C sequences before the call) and whose effectful actuals are data
//!   at `Borrowed` formals. The events are lowered first, then the call, use
//!   `Discarded`.
//! * **A typed substitution** ([`Parser::substituted_call`]): a call statement
//!   whose lexical substitution the classifier refused (`call has an
//!   unmodeled formal address or borrowed expression`) or left incomplete (a
//!   custody formal with no actual, which the flow reports as `arguments do
//!   not match resource contract`). Each actual is read from Clang's AST
//!   ([`crate::call_instantiation::substitute`]); a value-preserving pointer
//!   cast of a named object is that object, a scalar expression with
//!   unevaluated type-query operands is data. The call is then classified
//!   again with those actuals and admitted only when every custody formal is
//!   substituted by the identity the AST names.
//!
//! Every consultation and every disagreement between a consumer's older
//! answer and the acceptance is recorded in the `QCORE_TYPED_PATH_PARITY`
//! stream (`position: "call-instantiation"`).

use serde_json::json;

use super::condition_desugar::Position;
use super::{
    LegacyInstruction, Parser, Statement, Token, call_argument_spans, find_call,
    find_function_body, is_identifier,
};
use crate::call_instantiation::{
    self, Actual, Consumer, Decline, ResultClass, ResultUse, Standing,
};
use crate::elaboration::{Elaboration, Event, Operand, strip_parens};
use crate::typed_access_path::{StatementIndex, StatementPlan, record_row};
use crate::typed_c_ast::{children, kind};

/// Why [`Parser::instantiate_call`] did not produce an accepted call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum CallRefusal {
    /// The acceptance declined the instantiated call.
    Declined(Decline),
    /// The classifier refused the call (its description).
    Unapplied(String),
    /// The classifier lowered the statement to something else.
    Lowered(Box<LegacyInstruction>),
    /// The classifier's destination is not the one the consumer asked for.
    UnexpectedDestination,
}

impl CallRefusal {
    fn code(&self) -> String {
        match self {
            Self::Declined(decline) => decline.code().to_owned(),
            Self::Unapplied(description) => format!("unapplied: {description}"),
            Self::Lowered(_) => "lowered".to_owned(),
            Self::UnexpectedDestination => "unexpected-destination".to_owned(),
        }
    }
}

/// Whether a `ContractCall` is the identity on the abstract state (rule 5h's
/// premise for two calls of one operand).
pub(super) fn is_identity(instruction: &LegacyInstruction) -> bool {
    super::condition_desugar::is_identity_call(instruction)
}

impl Parser<'_> {
    /// The callee's standing: declared (no body in this unit), a validated
    /// local summary, or a body in the enclosing function's recursive
    /// component.
    pub(super) fn call_standing(&mut self, callee: &str) -> Standing {
        if self.recursive_local_callee(callee).is_some() {
            Standing::Recursive
        } else if find_function_body(self.tokens, callee).is_some() {
            Standing::Validated
        } else {
            Standing::Declared
        }
    }

    /// **The instantiation API** (rule 10u): apply `callee`'s contract to the
    /// call statement `statement` (`CALL(…)` or `NAME = CALL(…)`, with
    /// `destination` the `NAME`) and decide its acceptance for `use_`.
    ///
    /// The standing is read before the classifier runs, so a declined
    /// recursive callee perturbs no site counter. On any refusal the caller
    /// restores the counters it saved.
    pub(super) fn instantiate_call(
        &mut self,
        callee: &str,
        statement: &[Token],
        destination: Option<&str>,
        use_: ResultUse,
    ) -> Result<Vec<LegacyInstruction>, CallRefusal> {
        let mut reading_transaction = crate::contract_dependencies::ReadingTransaction::start();
        let standing = self.call_standing(callee);
        if standing == Standing::Recursive {
            return Err(CallRefusal::Declined(Decline::RecursiveLocal));
        }
        let instruction = self.classify_simple(statement);
        // The statement position's typed argument reads (card
        // W44-INDIRECT-CALL-ARGUMENT-ACCESS) are part of the one application:
        // a call whose actuals the lexical gate refused is its actuals' typed
        // reads, then the call, in every position that asks.
        let sequence = match &instruction {
            LegacyInstruction::Unknown { description, .. }
                if description == super::call_argument_access::GATE_REFUSAL =>
            {
                match self.call_argument_access(statement, &instruction) {
                    Some(sequence)
                        if matches!(
                            sequence.last(),
                            Some(LegacyInstruction::ContractCall { .. })
                        ) && sequence[..sequence.len() - 1].iter().all(|read| {
                            matches!(
                                read,
                                LegacyInstruction::IndirectScalarAccess { .. }
                                    | LegacyInstruction::ArgumentReads { .. }
                            )
                        }) =>
                    {
                        sequence
                    }
                    _ => return Err(CallRefusal::Unapplied(description.clone())),
                }
            }
            _ => vec![instruction],
        };
        let call = sequence.last().expect("a sequence ends with its call");
        match call {
            LegacyInstruction::ContractCall {
                destination: written,
                contract,
                ..
            } => {
                if written.as_deref() != destination {
                    return Err(CallRefusal::UnexpectedDestination);
                }
                call_instantiation::accept(
                    standing,
                    ResultClass::of(contract.result),
                    is_identity(call),
                    use_,
                )
                .map_err(CallRefusal::Declined)?;
                reading_transaction.commit();
                Ok(sequence)
            }
            LegacyInstruction::Unknown { description, .. } => {
                Err(CallRefusal::Unapplied(description.clone()))
            }
            _ => Err(CallRefusal::Lowered(Box::new(call.clone()))),
        }
    }

    /// Parity: the acceptance beside a call an older consumer admitted
    /// through its own gate. Only a disagreement is written.
    pub(super) fn record_instantiation_parity(
        &mut self,
        consumer: Consumer,
        line: usize,
        instruction: &LegacyInstruction,
        use_: ResultUse,
    ) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let LegacyInstruction::ContractCall {
            function,
            contract,
            nested,
            ..
        } = instruction
        else {
            return;
        };
        let mut calls = vec![(
            consumer,
            function.clone(),
            ResultClass::of(contract.result),
            is_identity(instruction),
            use_,
        )];
        for inner in nested {
            let identity = inner.contract.parameters.iter().all(|mode| {
                matches!(
                    mode,
                    qcore_schemas::contract::ResourceParameterMode::Borrowed
                )
            }) && !inner.contract.has_container_effect();
            calls.push((
                Consumer::Nested,
                inner.function.clone(),
                ResultClass::of(inner.contract.result),
                identity,
                ResultUse::Passed,
            ));
        }
        for (consumer, callee, result, identity, use_) in calls {
            let component = self
                .call_graph_components
                .get_or_init(|| unit_graph(self.tokens));
            let standing = parity_standing(component, &self.typed_path_function, &callee);
            if let Err(decline) = call_instantiation::accept(standing, result, identity, use_) {
                record_row(json!({
                    "function": self.typed_path_function,
                    "line": line,
                    "position": "call-instantiation",
                    "consumer": consumer.code(),
                    "callee": callee,
                    "use": use_.code(),
                    "result": result.code(),
                    "identity": identity,
                    "old_route": consumer.code(),
                    "old_admitted": true,
                    "new_admitted": false,
                    "new_refusal": decline.code(),
                    "disagreement": true,
                    "comparison": "legacy-wider: an older consumer's admission the acceptance declines",
                }));
            }
        }
    }

    fn record_instantiation(
        &self,
        consumer: Consumer,
        line: usize,
        tokens: &[Token],
        old: &str,
        outcome: Result<(), String>,
    ) {
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": tokens.iter().map(|token| token.text.as_str()).collect::<Vec<_>>(),
            "position": "call-instantiation",
            "consumer": consumer.code(),
            "old_route": "old_refusal",
            "old_refusal": old,
            "old_admitted": false,
            "new_admitted": outcome.is_ok(),
            "new_refusal": outcome.err(),
            "disagreement": false,
        }));
    }

    /// Hook: a `return` every older route refused, in a scalar-result
    /// function. `None` keeps the refusal.
    pub(super) fn instantiated_return(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        // The reserved return place spelled by the body keeps every return
        // route's refusal, this one's included (it binds no such place, but
        // a body that spells an implementation-reserved name is not read).
        if !self.return_facts.scalar_result
            || expression.is_empty()
            || !super::contains_call(expression)
            || !super::return_call::place_is_unspelled(
                self.tokens,
                super::return_call::RETURN_RESULT_PLACE,
            )
            || self.opaque_callee(expression)
            || residual_takes_an_address(expression)
            || (!self.list_frames.is_empty()
                && self
                    .list_iteration_refusal(
                        &expression
                            .iter()
                            .map(|t| t.text.as_str())
                            .collect::<Vec<_>>(),
                    )
                    .is_some())
        {
            return None;
        }
        let saved = self.probe_state();
        let outcome = self.single_leaf_plan(expression, Position::Branch);
        let outcome = match outcome {
            Ok((calls, LegacyInstruction::NoOp)) if !calls.is_empty() => Ok(calls),
            Ok(_) => Err("the residual is not a custody-neutral scalar".to_owned()),
            Err(refusal) => Err(refusal),
        };
        self.record_instantiation(
            Consumer::Return,
            line,
            expression,
            "unmodeled call, aggregate or indirect access in return expression",
            outcome.as_ref().map(|_| ()).map_err(Clone::clone),
        );
        match outcome {
            Ok(calls) => {
                let mut statements = calls
                    .into_iter()
                    .map(|call| Statement::Instruction(call, line))
                    .collect::<Vec<_>>();
                statements.push(Statement::Return {
                    line,
                    writes: self.expression_writes(expression),
                    expression_supported: true,
                    returned_place: None,
                    returned_evaluation: None,
                    returned_evaluation_escapes: false,
                    returns_null_constant: false,
                });
                Some(Statement::Block(statements))
            }
            Err(_) => {
                self.restore_probe_state(&saved);
                None
            }
        }
    }

    /// Hook from the elaborator's statement lowering (rule 10s): a piece that
    /// is exactly one direct call whose actuals carry effects. `Ok(None)`
    /// leaves the piece to the elaborator's own refusal.
    pub(super) fn effectful_call_piece(
        &mut self,
        index: &StatementIndex,
        node: &serde_json::Value,
        elaborated: &Elaboration<StatementPlan>,
        line: usize,
    ) -> Result<Option<Vec<LegacyInstruction>>, String> {
        // `CALL(…)`, its value discarded, or `NAME = CALL(…)` with `NAME` a
        // named non-pointer object whose write is the graph's last event.
        let bare = strip_parens(node);
        let (call, destination) = match kind(bare) {
            "CallExpr" => (bare, None),
            "BinaryOperator" if bare["opcode"].as_str() == Some("=") => {
                let [left, right] = children(bare) else {
                    return Ok(None);
                };
                let left = strip_parens(left);
                let right = strip_parens(right);
                if kind(right) != "CallExpr"
                    || kind(left) != "DeclRefExpr"
                    || crate::typed_access_path::is_pointer(left)
                {
                    return Ok(None);
                }
                let Some(name) = left["referencedDecl"]["name"].as_str() else {
                    return Ok(None);
                };
                (right, Some(name.to_owned()))
            }
            _ => return Ok(None),
        };
        let events = &elaborated.graph.events;
        let call_event = match destination {
            None => events.len().checked_sub(1),
            Some(_) => events.len().checked_sub(2),
        };
        let Some(call_event) = call_event else {
            return Ok(None);
        };
        let Event::Call { callee } = &events[call_event] else {
            return Ok(None);
        };
        if destination.is_some()
            && !matches!(
                events.last(),
                Some(Event::Write {
                    place: crate::elaboration::Place::Object { .. },
                    pointer: false
                })
            )
        {
            return Ok(None);
        }
        // The call is the graph's only call.
        if events
            .iter()
            .enumerate()
            .any(|(id, event)| id != call_event && matches!(event, Event::Call { .. }))
        {
            return Ok(None);
        }
        let callee = callee.clone();
        let actual_events = elaborated
            .actuals
            .get(&call_event)
            .cloned()
            .unwrap_or_default();
        if !actual_events
            .iter()
            .any(|&event| !matches!(events[event], Event::Read(_)))
        {
            return Ok(None);
        }
        let Some(contract) = self.contracts.contract(&callee).cloned() else {
            return Err(format!("call `{callee}` has no contract in force"));
        };
        let actuals = children(call).get(1..).unwrap_or_default();
        if actuals.len() != contract.parameters.len() {
            return Err(format!("call `{callee}`: actuals do not match the formals"));
        }
        // Every event before the call, lowered as the elaborator lowers it:
        // the actuals' reads (with their obligations) and writes.
        let mut instructions = Vec::new();
        for id in 0..call_event {
            if let Some(instruction) = self
                .lower_event(elaborated, id)
                .map_err(|refusal| refusal.reason())?
            {
                instructions.push(instruction);
            }
        }
        // The call's tokens with each scalar actual at a `Borrowed` formal
        // replaced by a neutral literal: its value is data, and its reads and
        // writes were lowered above. A pointer actual keeps its spelling, so
        // the classifier substitutes it; an effectful one refuses.
        let mut tokens = Vec::new();
        if let Some(name) = &destination {
            tokens.push(Token {
                text: name.clone(),
                line,
            });
            tokens.push(Token {
                text: "=".to_owned(),
                line,
            });
        }
        tokens.push(Token {
            text: callee.clone(),
            line,
        });
        tokens.push(Token {
            text: "(".to_owned(),
            line,
        });
        for (formal, actual) in actuals.iter().enumerate() {
            if formal > 0 {
                tokens.push(Token {
                    text: ",".to_owned(),
                    line,
                });
            }
            let pointer = crate::typed_access_path::is_pointer(actual);
            let borrowed = matches!(
                contract.parameters[formal],
                qcore_schemas::contract::ResourceParameterMode::Borrowed
            );
            if !pointer && borrowed {
                tokens.push(Token {
                    text: "0".to_owned(),
                    line,
                });
            } else if holds_effect(actual) {
                return Err(format!(
                    "call `{callee}`: an effectful actual at a formal that is not data"
                ));
            } else {
                let texts = index
                    .node_tokens(actual)
                    .ok_or_else(|| "an actual without tokens".to_owned())?;
                tokens.extend(texts.into_iter().map(|text| Token { text, line }));
            }
        }
        tokens.push(Token {
            text: ")".to_owned(),
            line,
        });
        if !matches!(elaborated.result, Operand::Void | Operand::Value { .. }) {
            return Ok(None);
        }
        let use_ = if destination.is_some() {
            ResultUse::Stored
        } else {
            ResultUse::Discarded
        };
        let saved = self.probe_state();
        match self.instantiate_call(&callee, &tokens, destination.as_deref(), use_) {
            Ok(sequence) => {
                instructions.extend(sequence);
                // The destination's write, as the elaborator lowers it (a
                // scalar local's old facts are forgotten).
                if destination.is_some() {
                    match self.lower_event(elaborated, events.len() - 1) {
                        Ok(Some(write)) => instructions.push(write),
                        Ok(None) => {}
                        Err(refusal) => {
                            self.restore_probe_state(&saved);
                            return Err(refusal.reason());
                        }
                    }
                }
                Ok(Some(instructions))
            }
            Err(refusal) => {
                self.restore_probe_state(&saved);
                Err(format!("call `{callee}`: {}", refusal.code()))
            }
        }
    }

    /// Hook: a call statement whose lexical substitution the classifier
    /// refused or left incomplete. `None` keeps `instruction`.
    pub(super) fn substituted_call(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Vec<LegacyInstruction>> {
        let incomplete = match instruction {
            LegacyInstruction::Unknown { description, .. } => description == K03_REFUSAL,
            LegacyInstruction::ContractCall {
                contract,
                arguments,
                ..
            } => contract.parameters.iter().enumerate().any(|(index, mode)| {
                call_instantiation::custody_taking(*mode)
                    && arguments.get(index).is_none_or(Option::is_none)
            }),
            _ => false,
        };
        if !incomplete
            || tokens.is_empty()
            || super::contains_braces(tokens)
            || self.opaque_callee(tokens)
        {
            return None;
        }
        let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
        if texts
            .iter()
            .any(|text| is_identifier(text) && self.descriptor_names.contains(text))
        {
            return None;
        }
        let (call_index, callee) = find_call(&texts)?;
        let callee = callee.to_owned();
        // `CALL(…)` or `NAME = CALL(…)`, the call closing the statement.
        let prefix_ok =
            call_index == 0 || (call_index == 2 && is_identifier(texts[0]) && texts[1] == "=");
        let close = super::matching(tokens, call_index + 1, "(", ")")?;
        if !prefix_ok || close != tokens.len() - 1 {
            return None;
        }
        let contract = self.contracts.contract(&callee).cloned()?;
        let index = self.typed_positions_index()?.clone();
        let outcome = self.substitute_call(&index, tokens, call_index, &callee, &contract);
        self.record_instantiation(
            Consumer::Statement,
            line,
            tokens,
            match instruction {
                LegacyInstruction::Unknown { description, .. } => description,
                _ => "arguments do not match resource contract",
            },
            outcome.as_ref().map(|_| ()).map_err(Clone::clone),
        );
        outcome.ok()
    }

    fn substitute_call(
        &mut self,
        index: &StatementIndex,
        tokens: &[Token],
        call_index: usize,
        callee: &str,
        contract: &qcore_schemas::contract::ResourceFunctionContract,
    ) -> Result<Vec<LegacyInstruction>, String> {
        let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
        let wanted = &texts[call_index..];
        let node = Self::elaboration_node(index, tokens[call_index].line, wanted, &|node| {
            kind(node) == "CallExpr"
        })
        .ok_or_else(|| "no unique Clang call node".to_owned())?;
        let substituted = call_instantiation::substitute(node, &contract.parameters, &index.fields)
            .map_err(|(formal, refusal)| format!("formal {formal}: {}", refusal.reason()))?;
        let spans = call_argument_spans(&texts, call_index)
            .ok_or_else(|| "unreadable argument list".to_owned())?;
        if spans.len() != substituted.len() {
            return Err("argument spans do not match the formals".to_owned());
        }
        // The statement with each actual respelled as the AST reads it: a
        // converted identity as its name, data mentioning a name as `0`.
        let mut rewritten = tokens[..call_index + 2].to_vec();
        let mut changed = false;
        for (position, (substitution, &(start, end))) in substituted.iter().zip(&spans).enumerate()
        {
            if position > 0 {
                rewritten.push(tokens[start - 1].clone());
            }
            let at = tokens[start].line;
            match &substitution.actual {
                Actual::Identity { name, cast: true } => {
                    rewritten.push(Token {
                        text: name.clone(),
                        line: at,
                    });
                    changed = true;
                }
                Actual::Data if texts[start..end].iter().any(|text| is_identifier(text)) => {
                    rewritten.push(Token {
                        text: "0".to_owned(),
                        line: at,
                    });
                    changed = true;
                }
                _ => rewritten.extend_from_slice(&tokens[start..end]),
            }
        }
        rewritten.push(tokens[tokens.len() - 1].clone());
        if !changed {
            return Err("the typed substitution names what the lexical one did".to_owned());
        }
        let destination = (call_index == 2).then(|| texts[0]);
        let saved = self.probe_state();
        let use_ = if destination.is_some() {
            ResultUse::Stored
        } else {
            ResultUse::Discarded
        };
        let sequence = match self.instantiate_call(callee, &rewritten, destination, use_) {
            Ok(sequence) => sequence,
            Err(refusal) => {
                self.restore_probe_state(&saved);
                return Err(format!("reclassified: {}", refusal.code()));
            }
        };
        // Every custody formal is substituted by exactly the identity the
        // AST names.
        let Some(LegacyInstruction::ContractCall { arguments, .. }) = sequence.last() else {
            unreachable!("instantiate_call answers a sequence ending with a ContractCall");
        };
        for substitution in &substituted {
            if !call_instantiation::custody_taking(substitution.mode) {
                continue;
            }
            let expected = match (&substitution.actual, substitution.mode) {
                (
                    Actual::Identity { name, .. },
                    qcore_schemas::contract::ResourceParameterMode::Out { .. },
                ) => {
                    format!("*{name}")
                }
                (Actual::Identity { name, .. }, _) => name.clone(),
                (Actual::Address { name }, _) => name.clone(),
                _ => {
                    self.restore_probe_state(&saved);
                    return Err("a custody formal without an identity".to_owned());
                }
            };
            if arguments
                .get(substitution.formal)
                .and_then(Option::as_deref)
                != Some(expected.as_str())
            {
                self.restore_probe_state(&saved);
                return Err(format!(
                    "formal {}: the classifier did not bind `{expected}`",
                    substitution.formal
                ));
            }
        }
        Ok(sequence)
    }
}

/// The unit's direct-call graph, for the parity stream only: every name the
/// lexical definition scan finds a body for (the first `NAME ( … ) {`, as
/// `find_function_body` reads it) and its strongly connected component. A
/// callee shares the enclosing function's component exactly when it reaches
/// the function back, which is `recursive_local_callee`'s question; the
/// admissions ask that function itself. Card
/// GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): read once per parser
/// (`Parser::call_graph_components`), never from a thread-local keyed by the
/// stream's address.
fn unit_graph(tokens: &[Token]) -> std::collections::BTreeMap<String, usize> {
    use std::collections::{BTreeMap, BTreeSet};
    // First body per name; a name whose first call-shaped occurrence does
    // not close has no body, as in `find_function_body`.
    let mut bodies: BTreeMap<String, Option<(usize, usize)>> = BTreeMap::new();
    for index in 0..tokens.len().saturating_sub(1) {
        if tokens[index + 1].text != "(" || !is_identifier(&tokens[index].text) {
            continue;
        }
        let name = &tokens[index].text;
        if bodies.contains_key(name) {
            continue;
        }
        let Some(close) = super::matching(tokens, index + 1, "(", ")") else {
            bodies.insert(name.clone(), None);
            continue;
        };
        if tokens.get(close + 1).is_some_and(|token| token.text == "{")
            && let Some(end) = super::matching(tokens, close + 1, "{", "}")
        {
            bodies.insert(name.clone(), Some((close + 1, end)));
        }
    }
    let names = bodies
        .iter()
        .filter_map(|(name, body)| body.map(|_| name.clone()))
        .collect::<Vec<_>>();
    let position = names
        .iter()
        .enumerate()
        .map(|(at, name)| (name.clone(), at))
        .collect::<BTreeMap<_, _>>();
    let edges = names
        .iter()
        .map(|name| {
            let (start, end) = bodies[name].expect("a body");
            (start..end)
                .filter(|&at| tokens.get(at + 1).is_some_and(|token| token.text == "("))
                .filter_map(|at| position.get(&tokens[at].text).copied())
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    // Tarjan's strongly connected components, iteratively.
    let count = names.len();
    let mut index_of = vec![usize::MAX; count];
    let mut low = vec![0usize; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut component = vec![usize::MAX; count];
    let mut next_index = 0usize;
    let mut next_component = 0usize;
    for root in 0..count {
        if index_of[root] != usize::MAX {
            continue;
        }
        let mut work: Vec<(usize, Vec<usize>)> = Vec::new();
        index_of[root] = next_index;
        low[root] = next_index;
        next_index += 1;
        stack.push(root);
        on_stack[root] = true;
        work.push((root, edges[root].iter().copied().collect()));
        while let Some((node, pending)) = work.last_mut() {
            let node = *node;
            if let Some(next) = pending.pop() {
                if index_of[next] == usize::MAX {
                    index_of[next] = next_index;
                    low[next] = next_index;
                    next_index += 1;
                    stack.push(next);
                    on_stack[next] = true;
                    work.push((next, edges[next].iter().copied().collect()));
                } else if on_stack[next] {
                    low[node] = low[node].min(index_of[next]);
                }
                continue;
            }
            work.pop();
            if let Some((parent, _)) = work.last() {
                low[*parent] = low[*parent].min(low[node]);
            }
            if low[node] == index_of[node] {
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component[member] = next_component;
                    if member == node {
                        break;
                    }
                }
                next_component += 1;
            }
        }
    }
    names
        .into_iter()
        .enumerate()
        .map(|(at, name)| (name, component[at]))
        .collect()
}

/// The standing the parity stream records, from `component` (the parser's
/// [`unit_graph`]).
fn parity_standing(
    component: &std::collections::BTreeMap<String, usize>,
    function: &str,
    callee: &str,
) -> Standing {
    match (component.get(callee), component.get(function)) {
        (Some(_), _) if callee == function => Standing::Recursive,
        (Some(a), Some(b)) if a == b => Standing::Recursive,
        (Some(_), _) => Standing::Validated,
        (None, _) => Standing::Declared,
    }
}

/// Whether a `return` expression takes an address outside its calls' actuals:
/// rule 5j's scalar-result return premise (no unary `&`) for the value the
/// function hands back. An actual's `&` is the call's own substitution.
fn residual_takes_an_address(expression: &[Token]) -> bool {
    let mut index = 0;
    while index < expression.len() {
        let text = expression[index].text.as_str();
        if is_identifier(text)
            && expression
                .get(index + 1)
                .is_some_and(|next| next.text == "(")
            && let Some(close) = super::matching(expression, index + 1, "(", ")")
        {
            index = close + 1;
            continue;
        }
        if text == "&" {
            let unary = index == 0
                || matches!(
                    expression[index - 1].text.as_str(),
                    "(" | ","
                        | "="
                        | "+"
                        | "-"
                        | "*"
                        | "/"
                        | "%"
                        | "&"
                        | "|"
                        | "^"
                        | "!"
                        | "~"
                        | "<"
                        | ">"
                        | "?"
                        | ":"
                )
                || (expression[index - 1].text == ")"
                    && index >= 2
                    && is_cast_close(expression, index - 1));
            if unary {
                return true;
            }
        }
        index += 1;
    }
    false
}

/// Whether the `)` at `close` ends a cast `( TYPE )` rather than a group:
/// read conservatively, any parenthesised run of identifiers and `*` is a
/// type name.
fn is_cast_close(tokens: &[Token], close: usize) -> bool {
    let mut at = close;
    while at > 0 {
        at -= 1;
        match tokens[at].text.as_str() {
            // A call's own parentheses are no cast.
            "(" => return at + 1 < close && (at == 0 || !is_identifier(&tokens[at - 1].text)),
            "*" => {}
            text if is_identifier(text) => {}
            _ => return false,
        }
    }
    false
}

/// The classifier's K03 refusal.
pub(super) const K03_REFUSAL: &str = "call has an unmodeled formal address or borrowed expression";

/// Whether an actual holds an increment, assignment, compound assignment or
/// comma: a side effect C sequences before the call.
fn holds_effect(node: &serde_json::Value) -> bool {
    match kind(node) {
        "CompoundAssignOperator" => true,
        "UnaryOperator" if matches!(node["opcode"].as_str(), Some("++" | "--")) => true,
        "BinaryOperator" if matches!(node["opcode"].as_str(), Some("=" | ",")) => true,
        _ => children(node).iter().any(holds_effect),
    }
}
