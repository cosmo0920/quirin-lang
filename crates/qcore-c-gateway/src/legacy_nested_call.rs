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

//! Card **CG05-NESTED-CALL-EFFECTS** (gateway rule 10y): the nested-call
//! tree of [`crate::nested_call_effects`] connected to the custody gateway
//! (route G), through card CG03's one application point.
//!
//! # The consumers
//!
//! Each is asked last, only where the older routes left card O04-B09B10's
//! nested-call refusal (`nested call `…` inside a modeled call is outside the
//! resource projection`), the classifier's `resource call with a formal
//! embedded in an unmodeled expression` (a statement) or the `return`
//! position's refusal, so every call an
//! older route admits (card B04's identity siblings, O04-TRACE's variadic
//! tails, CG03's return operand) keeps its older answer.
//!
//! * **A statement** ([`Parser::nested_call_statement`], the last step of
//!   `expression_statement`): `CALL(…)`, `NAME = CALL(…)` or a one-declarator
//!   declaration initialised by `CALL(…)`, whose actuals hold calls; or
//!   `NAME = VALUE` / a declaration initialised by `VALUE` for a scalar `NAME`
//!   and a scalar `VALUE` holding calls (`psize = strlen(a) + strlen(b)`).
//! * **A `return`** ([`Parser::nested_call_return`], after every older return
//!   route): `return CALL(…)` whose actuals hold calls, desugared as card
//!   C-GATEWAY-RETURN-CALL-W4 desugars `return f(a);` (the reserved return
//!   place, use `Returned`); or, in a function whose definition declares a
//!   scalar result, `return VALUE;` for a scalar `VALUE` holding calls, whose
//!   exit escapes nothing.
//!
//! The statement or expression is matched to one Clang node by line and exact
//! tokens. Its call tree is read and order-checked
//! ([`crate::nested_call_effects::read`] / `read_data`); then every call is
//! instantiated by [`Parser::instantiate_call`], inside-out, in the graph's
//! event order:
//!
//! * a **bound** call as `__qcore_nested_value_N = g(…)`, use `Passed`;
//! * a call inside a **data** value as `g(…)`, use `Passed` when its value is
//!   the whole actual and `Tested` when an operator consumes it;
//! * the outer call with each bound actual spelled as its reserved place and
//!   each data actual as `0` (its reads and calls were lowered before it).
//!
//! Each application must apply exactly the contract the footprint was read
//! from. A statement's reserved places are cleared after it: an owned value the
//! outer call did not take stays owned by the body and is reported where any
//! leaked value is. Every consultation is a parity row
//! (`position: "nested-call"`).

use serde_json::{Value, json};

use super::return_call::{self, RETURN_RESULT_PLACE};
use super::{LegacyInstruction, Parser, Statement, Token, contains_braces, is_identifier};
use crate::call_instantiation::ResultUse;
use crate::nested_call_effects::{self, Actual, Call, CallFacts, Facts, NESTED_VALUE_PREFIX};
use crate::typed_access_path::{FieldFacts, StatementIndex, is_pointer, record_row};
use crate::typed_c_ast::{children, kind};

/// Card O04-B09B10's refusal, the only statement refusal these consumers are
/// asked after.
pub(super) const NESTED_REFUSAL_HEAD: &str = "nested call `";
pub(super) const NESTED_REFUSAL_TAIL: &str =
    "` inside a modeled call is outside the resource projection";

/// The classifier's refusal of a contracted call inside a larger expression
/// that mentions a protected name (CG00's `formal-in-unmodeled-expression`,
/// gap entry E11): the statement consumer is asked after it too.
pub(super) const FORMAL_EMBEDDED: &str =
    "resource call with a formal embedded in an unmodeled expression";

/// Whether a refusal is card O04-B09B10's nested-call refusal.
pub(super) fn is_nested_refusal(description: &str) -> bool {
    description.starts_with(NESTED_REFUSAL_HEAD) && description.ends_with(NESTED_REFUSAL_TAIL)
}

pub(super) struct GFacts<'a> {
    contracts: &'a qcore_schemas::contract::ResourceContracts,
    tails: &'a std::collections::BTreeSet<String>,
    index: &'a StatementIndex,
    automatic: std::collections::BTreeSet<String>,
    taken: std::collections::BTreeSet<String>,
}

impl Facts for GFacts<'_> {
    fn contract(&self, callee: &str) -> Option<CallFacts> {
        let contract = self.contracts.contract(callee)?;
        Some(CallFacts {
            modes: contract.parameters.clone(),
            result: contract.result,
            tail: self.tails.contains(callee),
            container: contract.has_container_effect(),
        })
    }

    fn unaliased(&self, decl: &str) -> bool {
        self.automatic.contains(decl) && !self.taken.contains(decl)
    }

    fn fields(&self) -> &std::collections::BTreeMap<String, Option<FieldFacts>> {
        &self.index.fields
    }
}

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

/// The call a value is, through parentheses, value-preserving pointer
/// conversions and a selection's selected association.
fn through(value: &Value) -> Option<&Value> {
    nested_call_effects::whole_call(value)
}

/// What a statement node is, as these consumers read it.
enum Shape<'a> {
    /// A call whose value is discarded or stored into `destination`.
    Call {
        call: &'a Value,
        destination: Option<String>,
        use_: ResultUse,
    },
    /// A scalar value holding calls stored into the scalar object `name`.
    Data { value: &'a Value, name: String },
}

fn shape(node: &Value) -> Option<Shape<'_>> {
    let bare = crate::elaboration::strip_parens(node);
    let (value, name) = match kind(bare) {
        "CallExpr" => {
            return Some(Shape::Call {
                call: bare,
                destination: None,
                use_: ResultUse::Discarded,
            });
        }
        "BinaryOperator" if bare["opcode"].as_str() == Some("=") => {
            let [left, right] = children(bare) else {
                return None;
            };
            let left = crate::elaboration::strip_parens(left);
            if kind(left) != "DeclRefExpr"
                || !matches!(
                    left["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                )
            {
                return None;
            }
            (right, left["referencedDecl"]["name"].as_str()?.to_owned())
        }
        "DeclStmt" => {
            let [declaration] = children(bare) else {
                return None;
            };
            if kind(declaration) != "VarDecl" || declaration["storageClass"].as_str().is_some() {
                return None;
            }
            let [initializer] = children(declaration) else {
                return None;
            };
            (initializer, declaration["name"].as_str()?.to_owned())
        }
        _ => return None,
    };
    match through(value) {
        Some(call) => Some(Shape::Call {
            call,
            destination: Some(name),
            use_: ResultUse::Stored,
        }),
        None if !is_pointer(value) => Some(Shape::Data { value, name }),
        None => None,
    }
}

/// The unique Clang node on the statement's line whose tokens are the
/// statement's. When none is, and the only difference the analysed tokens can
/// carry is card W44-C11-GENERIC-SELECTION's invariant rewriting of a
/// `_Generic` selection to its core, the unique node of an accepted kind on
/// that line whose own tokens hold `_Generic`; [`locate`] then checks every
/// call and plain actual against the rewritten tokens.
fn statement_node<'i>(
    index: &'i StatementIndex,
    tokens: &[Token],
    accept: &dyn Fn(&Value) -> bool,
) -> Option<&'i Value> {
    let words = texts(tokens);
    let line = tokens.first()?.line;
    if let Some(node) = Parser::elaboration_node(index, line, &words, accept) {
        return Some(node);
    }
    if words.contains(&"_Generic") {
        return None;
    }
    fn walk<'i>(
        index: &StatementIndex,
        node: &'i Value,
        line: usize,
        accept: &dyn Fn(&Value) -> bool,
        out: &mut Vec<&'i Value>,
    ) {
        if index.node_line(node).is_some_and(|begin| begin > line)
            || index.node_end_line(node).is_some_and(|end| end < line)
        {
            return;
        }
        if kind(node) == "CompoundStmt" {
            for child in children(node) {
                if accept(child)
                    && index.node_line(child) == Some(line)
                    && index
                        .node_tokens(child)
                        .is_some_and(|found| found.iter().any(|text| text == "_Generic"))
                {
                    out.push(child);
                }
            }
        }
        for child in children(node) {
            walk(index, child, line, accept, out);
        }
    }
    let mut found = Vec::new();
    walk(index, &index.function, line, accept, &mut found);
    // Several statements of the line may hold a selection: keep the one whose
    // destination and outer callee are the statement's own.
    let destination = words
        .iter()
        .position(|word| *word == "=")
        .and_then(|equal| equal.checked_sub(1))
        .map(|name| words[name]);
    let callee = top_level_calls(&words, 0, words.len())
        .first()
        .map(|&at| words[at]);
    found.retain(|node| match shape(node) {
        Some(Shape::Call {
            call,
            destination: written,
            ..
        }) => {
            written.as_deref() == destination
                && crate::elaboration::direct_callee(children(call).first().unwrap_or(&Value::Null))
                    == callee
        }
        _ => false,
    });
    match found.as_slice() {
        [node] => Some(node),
        _ => None,
    }
}

/// The index of the `)` closing the `(` at `open`.
fn closing(words: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, word) in words.iter().enumerate().skip(open) {
        match *word {
            "(" => depth += 1,
            ")" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// The callee indices of the calls in `words[start..end]` that no other
/// call's argument list holds, in source order; a type operator's operand is
/// skipped (it is not evaluated).
fn top_level_calls(words: &[&str], start: usize, end: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut at = start;
    while at < end {
        if is_identifier(words[at]) && words.get(at + 1) == Some(&"(") && at + 1 < end {
            if !super::TYPE_OPERATORS.contains(&words[at]) {
                out.push(at);
            }
            match closing(words, at + 1) {
                Some(close) => at = close + 1,
                None => return Vec::new(),
            }
            continue;
        }
        at += 1;
    }
    out
}

/// Locate `call` at the callee index `at` of `words` and every call of its
/// actuals, checking Clang's tree against the statement's own tokens: the
/// callee, the arity, each plain actual's tokens, and the calls of each bound
/// or data actual. Any difference refuses.
fn locate(
    words: &[&str],
    index: &StatementIndex,
    call: &Call<'_>,
    at: usize,
    out: &mut std::collections::BTreeMap<usize, usize>,
) -> Result<(), String> {
    if words.get(at) != Some(&call.callee.as_str()) || words.get(at + 1) != Some(&"(") {
        return Err(format!(
            "call `{}` is not spelled where Clang has it",
            call.callee
        ));
    }
    let spans = super::call_argument_spans(words, at)
        .ok_or_else(|| "an unreadable argument list".to_owned())?;
    if spans.len() != call.actuals.len() {
        return Err(format!("call `{}`: the spelled arity differs", call.callee));
    }
    for (actual, &(start, end)) in call.actuals.iter().zip(&spans) {
        match actual {
            Actual::Plain(node) => {
                let clang = index
                    .node_tokens(node)
                    .ok_or_else(|| "an actual without tokens".to_owned())?;
                if !clang
                    .iter()
                    .map(String::as_str)
                    .eq(words[start..end].iter().copied())
                {
                    return Err("an actual's tokens differ from Clang's".to_owned());
                }
            }
            Actual::Bound(inner) => match top_level_calls(words, start, end).as_slice() {
                [inner_at] => locate(words, index, inner, *inner_at, out)?,
                _ => return Err("a bound actual is not one spelled call".to_owned()),
            },
            Actual::Data { calls } => {
                let starts = top_level_calls(words, start, end);
                if starts.len() != calls.len() {
                    return Err("a data actual's calls are not the spelled ones".to_owned());
                }
                for (inner, inner_at) in calls.iter().zip(starts) {
                    locate(words, index, inner, inner_at, out)?;
                }
            }
        }
    }
    out.insert(call.event, at);
    Ok(())
}

/// One consultation's outcome: the instructions, the tree's depth and its
/// calls with their uses.
type Lowered = (Vec<LegacyInstruction>, usize, Vec<String>);

impl Parser<'_> {
    /// The premises every consumer shares before any AST is read.
    fn nested_call_declines(&self, tokens: &[Token]) -> bool {
        let words = texts(tokens);
        tokens.is_empty()
            || contains_braces(tokens)
            || self.opaque_callee(tokens)
            || words
                .iter()
                .any(|text| is_identifier(text) && self.descriptor_names.contains(text))
            || (!self.list_frames.is_empty() && self.list_iteration_refusal(&words).is_some())
            || self
                .tokens
                .iter()
                .any(|token| token.text.starts_with(NESTED_VALUE_PREFIX))
    }

    pub(super) fn nested_facts<'i>(&'i self, index: &'i StatementIndex) -> GFacts<'i> {
        let (automatic, taken) = super::elaboration_route::aliasing(&index.function);
        GFacts {
            contracts: self.contracts,
            tails: &self.variadic_tails,
            index,
            automatic,
            taken,
        }
    }

    fn record_nested(
        &self,
        consumer: &str,
        line: usize,
        tokens: &[Token],
        old: &str,
        outcome: Result<(usize, &[String]), &String>,
    ) {
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts(tokens),
            "position": "nested-call",
            "consumer": consumer,
            "old_route": "old_refusal",
            "old_refusal": old,
            "old_admitted": false,
            "new_admitted": outcome.is_ok(),
            "new_refusal": outcome.err(),
            "depth": outcome.ok().map(|(depth, _)| depth),
            "calls": outcome.ok().map(|(_, calls)| calls),
            "disagreement": outcome.is_ok(),
        }));
    }

    /// Hook (rule 10y): a statement the classifier refused with the nested-call
    /// refusal. `None` keeps the refusal byte for byte.
    pub(super) fn nested_call_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Vec<LegacyInstruction>> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        if !(is_nested_refusal(description) || description == FORMAL_EMBEDDED)
            || self.nested_call_declines(tokens)
        {
            return None;
        }
        let index = self.typed_positions_index()?.clone();
        let saved = self.probe_state();
        let outcome = self.lower_nested_statement(&index, tokens, line);
        self.record_nested(
            "statement",
            line,
            tokens,
            description,
            outcome
                .as_ref()
                .map(|(_, depth, calls)| (*depth, calls.as_slice())),
        );
        match outcome {
            Ok((instructions, ..)) => Some(instructions),
            Err(_) => {
                self.restore_probe_state(&saved);
                None
            }
        }
    }

    fn lower_nested_statement(
        &mut self,
        index: &StatementIndex,
        tokens: &[Token],
        line: usize,
    ) -> Result<Lowered, String> {
        let accept =
            |node: &Value| matches!(kind(node), "CallExpr" | "BinaryOperator" | "DeclStmt");
        let edge;
        let node = match statement_node(index, tokens, &accept) {
            Some(node) => node,
            // Rule 11m: on a value-connective edge, the edge's own node.
            None => {
                edge = crate::value_connective::edge_root()
                    .filter(|root| accept(root))
                    .ok_or_else(|| "no unique Clang node for the statement".to_owned())?;
                &edge
            }
        };
        let facts = self.nested_facts(index);
        match shape(node).ok_or_else(|| "not a call or a scalar value holding calls".to_owned())? {
            Shape::Call {
                call,
                destination,
                use_,
            } => {
                let reading =
                    nested_call_effects::read(call, &facts, use_).map_err(|r| r.reason())?;
                let (start, end) =
                    Self::call_run(&reading.root.callee, tokens, destination.is_some())?;
                if end != tokens.len() {
                    return Err("tokens follow the call".to_owned());
                }
                let prefix = tokens[..start].to_vec();

                let mut lowered = self.lower_tree(
                    index,
                    &reading.root.post_order(),
                    &reading.places,
                    Some((&reading.root, prefix, destination, start)),
                    tokens,
                    line,
                )?;
                // The reserved places end with the statement.
                for place in reading.places.values() {
                    lowered.0.push(LegacyInstruction::Clear {
                        destination: place.clone(),
                    });
                }
                lowered.1 = reading.root.depth();
                Ok(lowered)
            }
            Shape::Data { value, name } => {
                let reading =
                    nested_call_effects::read_data(value, &facts).map_err(|r| r.reason())?;
                // The value is everything after the destination's `=`.
                let start = texts(tokens)
                    .iter()
                    .position(|word| *word == "=")
                    .ok_or_else(|| "a store without `=`".to_owned())?
                    + 1;
                let mut lowered = self.lower_tree(
                    index,
                    &reading.post_order(),
                    &reading.places,
                    None,
                    tokens,
                    line,
                )?;
                for place in reading.places.values() {
                    lowered.0.push(LegacyInstruction::Clear {
                        destination: place.clone(),
                    });
                }
                // The scalar store, as card CG02's elaborator lowers a scalar
                // local's write: its old facts are forgotten.
                let _ = start;
                if self.protected_formals.contains(&name)
                    || !crate::typed_field_access::is_scalar_initializer(
                        &name,
                        &["1"],
                        &self.typed_model,
                        &self.typed_types,
                    )
                {
                    return Err(
                        "a store to an object the gateway does not read as a scalar local"
                            .to_owned(),
                    );
                }
                lowered
                    .0
                    .push(LegacyInstruction::ForgetScalar { destination: name });
                lowered.1 = reading.calls.iter().map(Call::depth).max().unwrap_or(0);
                Ok(lowered)
            }
        }
    }

    /// The run of `tokens` the outer call spells: the only call the statement
    /// holds at its top level after its destination's `=` (from the start when
    /// there is none), closing the statement.
    fn call_run(
        callee: &str,
        tokens: &[Token],
        destination: bool,
    ) -> Result<(usize, usize), String> {
        let words = texts(tokens);
        let from = if destination {
            words
                .iter()
                .position(|word| *word == "=")
                .ok_or_else(|| "a destination without `=`".to_owned())?
                + 1
        } else {
            0
        };
        match top_level_calls(&words, from, words.len()).as_slice() {
            [at] if words[*at] == callee => {
                let close = closing(&words, at + 1).ok_or_else(|| "an unclosed call".to_owned())?;
                Ok((*at, close + 1))
            }
            _ => Err("the call is not the statement's one outer call".to_owned()),
        }
    }

    /// Instantiate every call of `order` inside-out. `root` is the outer call
    /// with the statement's prefix and destination; every other call is bound
    /// to its reserved place or read as data.
    fn lower_tree(
        &mut self,
        index: &StatementIndex,
        order: &[&Call<'_>],
        places: &std::collections::BTreeMap<usize, String>,
        root: Option<(&Call<'_>, Vec<Token>, Option<String>, usize)>,
        tokens: &[Token],
        line: usize,
    ) -> Result<Lowered, String> {
        // Every call of the tree, located in the statement's own tokens and
        // checked against Clang: its callee, its arity and each plain actual.
        let words = texts(tokens);
        let mut positions = std::collections::BTreeMap::new();
        let roots = match &root {
            Some((outer, _, _, at)) => vec![(*outer, *at)],
            None => {
                let starts = top_level_calls(&words, 0, words.len());
                let outermost = order
                    .iter()
                    .filter(|call| {
                        !order.iter().any(|other| {
                            other.event != call.event
                                && other
                                    .post_order()
                                    .iter()
                                    .any(|inner| inner.event == call.event)
                        })
                    })
                    .copied()
                    .collect::<Vec<_>>();
                if starts.len() != outermost.len() {
                    return Err("the value's calls are not the statement's".to_owned());
                }
                outermost.into_iter().zip(starts).collect()
            }
        };
        for (call, at) in roots {
            locate(&words, index, call, at, &mut positions)?;
        }
        let mut instructions = Vec::new();
        let mut calls = Vec::new();
        for call in order {
            let spelled = Self::nested_call_tokens(places, &positions, call, tokens, line)?;
            let (statement, destination) = match &root {
                Some((outer, prefix, destination, _)) if std::ptr::eq(*call, *outer) => {
                    let mut whole = prefix.clone();
                    whole.extend(spelled);
                    (whole, destination.clone())
                }
                _ => match places.get(&call.event) {
                    Some(place) => {
                        let mut whole = vec![
                            Token {
                                text: place.clone(),
                                line,
                            },
                            Token {
                                text: "=".to_owned(),
                                line,
                            },
                        ];
                        whole.extend(spelled);
                        (whole, Some(place.clone()))
                    }
                    None => (spelled, None),
                },
            };
            let use_ = call.use_.unwrap_or(ResultUse::Passed);
            let sequence = self
                .instantiate_call(&call.callee, &statement, destination.as_deref(), use_)
                .map_err(|refusal| format!("call `{}`: {refusal:?}", call.callee))?;
            // The contract applied is the one the footprint was read from.
            match sequence.last() {
                Some(LegacyInstruction::ContractCall { contract, .. })
                    if contract.parameters == call.facts.modes
                        && contract.result == call.facts.result => {}
                _ => {
                    return Err(format!(
                        "call `{}`: the contract applied is not the one read",
                        call.callee
                    ));
                }
            }
            calls.push(format!("{}:{}", call.callee, use_.code()));
            instructions.extend(sequence);
        }
        Ok((instructions, 0, calls))
    }

    /// The tokens of one call of the tree, each bound actual spelled as its
    /// reserved place and each data actual as `0`; a plain actual is the
    /// statement's own run, which [`locate`] checked against Clang.
    fn nested_call_tokens(
        places: &std::collections::BTreeMap<usize, String>,
        positions: &std::collections::BTreeMap<usize, usize>,
        call: &Call<'_>,
        statement: &[Token],
        line: usize,
    ) -> Result<Vec<Token>, String> {
        let at = |text: &str| Token {
            text: text.to_owned(),
            line,
        };
        let words = texts(statement);
        let callee = *positions
            .get(&call.event)
            .ok_or_else(|| "a call not located in the statement".to_owned())?;
        let spans = super::call_argument_spans(&words, callee)
            .ok_or_else(|| "an unreadable argument list".to_owned())?;
        let mut out = vec![at(&call.callee), at("(")];
        for (position, (actual, &(start, end))) in call.actuals.iter().zip(&spans).enumerate() {
            if position > 0 {
                out.push(at(","));
            }
            match actual {
                Actual::Plain(_) => out.extend_from_slice(&statement[start..end]),
                Actual::Bound(inner) => {
                    let place = places
                        .get(&inner.event)
                        .ok_or_else(|| "a bound call without its place".to_owned())?;
                    out.push(at(place));
                }
                Actual::Data { .. } => out.push(at("0")),
            }
        }
        out.push(at(")"));
        Ok(out)
    }

    /// Hook (rule 10y): a `return` the older routes refused, or lowered to the
    /// nested-call refusal. `None` keeps the statement.
    pub(super) fn nested_call_return(
        &mut self,
        expression: &[Token],
        line: usize,
        statement: &Statement,
    ) -> Option<Statement> {
        let old = match statement {
            Statement::Return {
                returned_evaluation: Some(evaluation),
                ..
            } => match evaluation.as_ref() {
                LegacyInstruction::Unknown { description, .. }
                    if is_nested_refusal(description) =>
                {
                    description.clone()
                }
                _ => return None,
            },
            Statement::Return {
                expression_supported: false,
                returned_evaluation: None,
                ..
            } => "unmodeled call, aggregate or indirect access in return expression".to_owned(),
            _ => return None,
        };
        if self.nested_call_declines(expression)
            || !return_call::place_is_unspelled(self.tokens, RETURN_RESULT_PLACE)
        {
            return None;
        }
        let index = self.typed_positions_index()?.clone();
        let saved = self.probe_state();
        let outcome = self.lower_nested_return(&index, expression, line);
        self.record_nested(
            "return",
            line,
            expression,
            &old,
            outcome
                .as_ref()
                .map(|((_, depth, calls), _)| (*depth, calls.as_slice())),
        );
        match outcome {
            Ok(((instructions, ..), escapes)) => {
                let mut instructions = instructions;
                // A whole call into the reserved place is the return's
                // evaluation, exactly as card C-GATEWAY-RETURN-CALL-W4's.
                let evaluation = if escapes {
                    instructions.pop().map(Box::new)
                } else {
                    None
                };
                let mut statements = instructions
                    .into_iter()
                    .map(|instruction| Statement::Instruction(instruction, line))
                    .collect::<Vec<_>>();
                statements.push(Statement::Return {
                    line,
                    writes: self.expression_writes(expression),
                    expression_supported: evaluation.is_none(),
                    returned_place: None,
                    returned_evaluation: evaluation,
                    returned_evaluation_escapes: escapes,
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

    /// The return's lowering, and whether its exit escapes the reserved return
    /// place (a whole call) or nothing (a scalar value).
    fn lower_nested_return(
        &mut self,
        index: &StatementIndex,
        expression: &[Token],
        line: usize,
    ) -> Result<(Lowered, bool), String> {
        if expression.is_empty() {
            return Err("an empty return".to_owned());
        }
        let accept = |node: &Value| {
            !matches!(
                kind(node),
                "ReturnStmt"
                    | "CompoundStmt"
                    | "IfStmt"
                    | "WhileStmt"
                    | "DoStmt"
                    | "ForStmt"
                    | "SwitchStmt"
                    | "CaseStmt"
                    | "DefaultStmt"
                    | "LabelStmt"
            )
        };
        let edge;
        let node = match statement_node(index, expression, &accept) {
            Some(node) => node,
            // Rule 11m: on a value-connective edge, the edge's own node.
            None => {
                edge = crate::value_connective::edge_root()
                    .filter(|root| accept(root))
                    .ok_or_else(|| "no unique Clang node for the return value".to_owned())?;
                &edge
            }
        };
        let facts = self.nested_facts(index);
        if let Some(call) = through(node) {
            let reading = nested_call_effects::read(call, &facts, ResultUse::Returned)
                .map_err(|r| r.reason())?;
            let (start, end) = Self::call_run(&reading.root.callee, expression, false)?;
            if start != 0 || end != expression.len() {
                return Err("the call is not the whole return value".to_owned());
            }
            let prefix = return_call::assignment_prefix(line).to_vec();
            let mut lowered = self.lower_tree(
                index,
                &reading.root.post_order(),
                &reading.places,
                Some((
                    &reading.root,
                    prefix,
                    Some(RETURN_RESULT_PLACE.to_owned()),
                    start,
                )),
                expression,
                line,
            )?;
            lowered.1 = reading.root.depth();
            return Ok((lowered, true));
        }
        if !self.return_facts.scalar_result || is_pointer(node) {
            return Err("a value holding calls outside a scalar-result function".to_owned());
        }
        let reading = nested_call_effects::read_data(node, &facts).map_err(|r| r.reason())?;
        let mut lowered = self.lower_tree(
            index,
            &reading.post_order(),
            &reading.places,
            None,
            expression,
            line,
        )?;
        for place in reading.places.values() {
            lowered.0.push(LegacyInstruction::Clear {
                destination: place.clone(),
            });
        }
        lowered.1 = reading.calls.iter().map(Call::depth).max().unwrap_or(0);
        Ok((lowered, false))
    }
}
