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

//! Card **CG14-AGGREGATE-VALUES-AND-INITIALIZERS** (gateway rule 11f): the
//! readers of [`crate::aggregate_value`] connected to the custody gateway
//! (route G).
//!
//! One **fallback**, asked last in `expression_statement`, only for a
//! statement every older route refused with the braced-initializer, the
//! indirect-access or the unrecognised-destination refusal. The statement is
//! matched to one Clang node by its line and exact tokens (as rule 10s
//! matches), and read as one of four forms:
//!
//! * a **declaration** `T l = INIT;` of one automatic record or array local:
//!   a braced list (M09), a whole-record copy (M08) or, for a character
//!   array, a string literal (M11);
//! * an **assignment** `P = Q` of record type (M08);
//! * a **call** `CALL(…)` or `NAME = CALL(…)` whose actuals hold compound
//!   literals (M10), each a whole actual at a `Borrowed` formal.
//!
//! An admitted declaration or copy is the typed reads and stores its plan
//! names (`IndirectScalarAccess`, W46's frame for a store through `*`/`[]`),
//! or `NoOp` when it reads no memory. An admitted call is the call instantiated
//! once (rule 10u) with each literal respelled `0`: it carries no identity.
//! Anything declined keeps the older refusal byte for byte (the site and
//! element counters are restored). Every consultation is a parity row
//! (`position: "aggregate-value"`).

use serde_json::{Value, json};

use super::{LegacyInstruction, Parser, RefusedWrites, Statement, Token, is_identifier};
use crate::aggregate_value::{
    self, LiteralUse, Refusal, TypeClass, decide_literal, holds_literal, literal_actual, read_copy,
    read_initializer, string_extent,
};
use crate::call_instantiation::ResultUse;
use crate::typed_access_path::{StatementIndex, StatementPlan, qual};
use crate::typed_c_ast::{children, kind};

/// The older refusals this fallback is asked after.
const REFUSALS: [&str; 3] = [
    "braced initializer, expression, or declaration is not modeled",
    "indirect memory access is outside the resource projection",
    "unrecognized assignment destination",
];

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

impl Parser<'_> {
    /// The hook: the lowering of a refused aggregate statement, or `None`.
    pub(super) fn aggregate_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        if !REFUSALS.contains(&description.as_str()) || tokens.is_empty() {
            return None;
        }
        // Card W45-PARENTHESISED-CALLEE: an opaque callee keeps its refusal.
        if self.opaque_callee(tokens) {
            return None;
        }
        let index = self.typed_positions_index()?.clone();
        let wanted = texts(tokens);
        let node = Self::elaboration_node(&index, tokens[0].line, &wanted, &|node| {
            matches!(kind(node), "DeclStmt" | "CallExpr")
                || (kind(node) == "BinaryOperator" && node["opcode"].as_str() == Some("="))
        })?;
        let function = self.typed_path_function.clone();
        let (form, outcome, detail) = match kind(node) {
            "DeclStmt" => {
                let (form, outcome, detail) = self.aggregate_declaration(&index, node);
                let outcome = outcome.map(|(plan, guarded)| {
                    let lowered = self.aggregate_lowering(plan, line);
                    match guarded {
                        Some(root) => guarded_initializer(root, lowered, instruction, line),
                        None => lowered,
                    }
                });
                (form, outcome, detail)
            }
            "BinaryOperator" if !holds_literal(node) => {
                let (outcome, detail) = self.aggregate_assignment(&index, node);
                (
                    "copy",
                    outcome.map(|plan| self.aggregate_lowering(plan, line)),
                    detail,
                )
            }
            _ if holds_literal(node) => {
                let (outcome, detail) = self.literal_call(&index, node, tokens, line);
                ("compound-literal", outcome, detail)
            }
            _ => return None,
        };
        aggregate_value::record(
            &function,
            line,
            &wanted,
            form,
            &outcome.as_ref().map(|_| ()).map_err(Clone::clone),
            detail,
        );
        outcome.ok()
    }

    fn aggregate_reader_never_bound(&self) -> impl Fn(&str) -> bool + '_ {
        |name| self.joined_never_bound(name)
    }

    /// `T l = INIT;`: one automatic record or array local. An admitted plan
    /// carries the local's name when its type can hold a pointer: the flow
    /// then guards it ([`guarded_initializer`]).
    #[allow(clippy::type_complexity)]
    fn aggregate_declaration(
        &self,
        index: &StatementIndex,
        node: &Value,
    ) -> (
        &'static str,
        Result<(StatementPlan, Option<String>), Refusal>,
        Value,
    ) {
        // One declarator; a record defined inline in the declaration
        // (`const union { … } one = { 1 };`) is its type, not a declarator.
        let declarators = children(node)
            .iter()
            .filter(|child| kind(child) != "RecordDecl")
            .collect::<Vec<_>>();
        let [declaration] = declarators.as_slice() else {
            return (
                "initializer",
                Err(Refusal::Declaration("several declarators")),
                Value::Null,
            );
        };
        if kind(declaration) != "VarDecl" || !declaration["storageClass"].is_null() {
            return (
                "initializer",
                Err(Refusal::Declaration("not an automatic object")),
                Value::Null,
            );
        }
        let Some(name) = declaration["name"].as_str() else {
            return ("initializer", Err(Refusal::Node), Value::Null);
        };
        let [init] = children(declaration) else {
            return (
                "initializer",
                Err(Refusal::Declaration("no single initializer")),
                Value::Null,
            );
        };
        let facts = &index.aggregates;
        // An array, or an array of function pointers (`void (*[N])(void)`); a
        // pointer to an array (`int (*)[N]`) is not one.
        let is_array = qual(declaration)
            .is_some_and(|text| text.trim().ends_with(']') || text.contains("(*["));
        // For an array, the element's class: whether the local can hold a
        // pointer.
        let mut class = facts.class_of(declaration);
        // An unnamed record defined inline in this declaration is its type.
        if let [inline] = children(node)
            .iter()
            .filter(|child| kind(child) == "RecordDecl")
            .collect::<Vec<_>>()
            .as_slice()
            && class == TypeClass::Unread
            && !is_array
            && qual(declaration)
                .is_some_and(|text| text.contains("(unnamed") || text.contains("(anonymous"))
            && let Some(id) = inline["id"].as_str()
        {
            class = facts.record_class_of(id);
        }
        if !is_array && !matches!(class, TypeClass::CustodyFree | TypeClass::PointerBearing) {
            return (
                "initializer",
                Err(Refusal::Declaration("not a record or array")),
                json!({ "class": class.code() }),
            );
        }
        // A local that can hold a pointer: the flow refuses the declaration
        // where a place rooted at it is bound, and forgets such places.
        let guard = class.may_hold_identity().then(|| name.to_owned());
        let never_bound = self.aggregate_reader_never_bound();
        let mut reader = index.reader(&never_bound);
        // M11: a character array initialised by a string literal.
        if kind(init) == "StringLiteral" {
            let Some(extent) = string_extent(declaration, init) else {
                return (
                    "string-initializer",
                    Err(Refusal::InitializerForm(
                        "string literal of a non-character array",
                    )),
                    Value::Null,
                );
            };
            return (
                "string-initializer",
                Ok((reader.plan, None)),
                json!({
                    "array_bytes": extent.array_bytes,
                    "literal_bytes": extent.literal_bytes,
                    "terminated": extent.terminated(),
                    "bounds_claimed": false,
                }),
            );
        }
        if kind(init) == "InitListExpr" {
            let mut leaves = Vec::new();
            let outcome = read_initializer(&mut reader, facts, init, &mut leaves);
            let detail = json!({
                "class": class.code(),
                "guarded": guard.is_some(),
                "leaves": leaves.iter().map(|leaf| leaf.code()).collect::<Vec<_>>(),
            });
            return (
                "initializer",
                outcome.map(|()| (reader.plan, guard)),
                detail,
            );
        }
        // M08: `T l = Q;`, a whole-record copy into the new local.
        if is_array {
            return (
                "initializer",
                Err(Refusal::InitializerForm(
                    "array initialised by an expression",
                )),
                Value::Null,
            );
        }
        let outcome = read_copy(&mut reader, facts, None, init);
        (
            "copy",
            outcome.map(|_| (reader.plan, None)),
            json!({ "class": class.code() }),
        )
    }

    /// `P = Q` of record type.
    fn aggregate_assignment(
        &self,
        index: &StatementIndex,
        node: &Value,
    ) -> (Result<StatementPlan, Refusal>, Value) {
        let [destination, source] = children(node) else {
            return (Err(Refusal::Node), Value::Null);
        };
        let facts = &index.aggregates;
        let class = facts.class_of(node);
        if matches!(class, TypeClass::Scalar | TypeClass::Pointer) {
            return (
                Err(Refusal::Declaration("not a record")),
                json!({ "class": class.code() }),
            );
        }
        let never_bound = self.aggregate_reader_never_bound();
        let mut reader = index.reader(&never_bound);
        let outcome = read_copy(&mut reader, facts, Some(destination), source);
        (
            outcome.map(|_| reader.plan),
            json!({
                "class": class.code(),
                "destination": aggregate_value::side(destination, Some(&index.function)).code(),
                "source": aggregate_value::side(source, Some(&index.function)).code(),
            }),
        )
    }

    /// An admitted plan: its typed reads and stores, or `NoOp`.
    fn aggregate_lowering(&self, plan: StatementPlan, line: usize) -> Statement {
        if plan.accesses == 0 {
            return Statement::Instruction(LegacyInstruction::NoOp, line);
        }
        Statement::Instruction(
            LegacyInstruction::IndirectScalarAccess {
                typed_alternative: None,
                alias_reads: plan
                    .scalar
                    .record_bases
                    .iter()
                    .filter_map(|base| self.read_alias(Some(base)))
                    .collect(),
                bases: plan.scalar.bases,
                destination: None,
                frame: plan
                    .scalar
                    .writes_through_pointer
                    .then(|| RefusedWrites::Sparing(self.frame_protected.clone())),
                chains: plan.chains,
                borrowed_destination: false,
                // Card CG12-VOID-VIEW-READBACK (rule 11j): no readback here.
                readback: None,
            },
            line,
        )
    }

    /// `CALL(…)` or `NAME = CALL(…)` holding compound literals: each a whole
    /// actual at a `Borrowed` formal, with an identity-free initializer that
    /// reads no memory; the call is instantiated with each respelled `0`.
    fn literal_call(
        &mut self,
        index: &StatementIndex,
        node: &Value,
        tokens: &[Token],
        line: usize,
    ) -> (Result<Statement, Refusal>, Value) {
        let texts = texts(tokens);
        let (call, destination) = match kind(node) {
            "CallExpr" => (node, None),
            _ => match (children(node), texts.as_slice()) {
                ([_, value], [name, "=", ..]) if is_identifier(name) => {
                    (crate::elaboration::strip_parens(value), Some(*name))
                }
                _ => return (Err(Refusal::LiteralPosition), Value::Null),
            },
        };
        let call_index = if destination.is_some() { 2 } else { 0 };
        if kind(call) != "CallExpr"
            || texts.get(call_index + 1) != Some(&"(")
            || !is_identifier(texts[call_index])
            || super::matching(tokens, call_index + 1, "(", ")") != Some(tokens.len() - 1)
        {
            return (Err(Refusal::LiteralPosition), Value::Null);
        }
        let callee = texts[call_index].to_owned();
        let Some(contract) = self.contracts.contract(&callee).cloned() else {
            return (
                Err(Refusal::UncontractedCallee),
                json!({ "callee": callee }),
            );
        };
        let actuals = children(call).get(1..).unwrap_or_default();
        let Some(spans) = argument_spans(&texts, call_index) else {
            return (Err(Refusal::Node), Value::Null);
        };
        if spans.len() != actuals.len() || actuals.len() != contract.parameters.len() {
            return (Err(Refusal::Node), json!({ "callee": callee }));
        }
        let facts = &index.aggregates;
        let mut literals = Vec::new();
        let mut rows = Vec::new();
        for (formal, actual) in actuals.iter().enumerate() {
            let never_bound = |name: &str| self.joined_never_bound(name);
            let Some(literal) = literal_actual(actual) else {
                if holds_literal(actual) {
                    // A literal inside an actual: not a whole actual.
                    let refusal = decide_literal(false, false, LiteralUse::Elsewhere)
                        .expect_err("a literal elsewhere is refused");
                    return (Err(refusal), json!({ "callee": callee, "formal": formal }));
                }
                continue;
            };
            let use_ = if contract.parameters[formal]
                == qcore_schemas::contract::ResourceParameterMode::Borrowed
            {
                LiteralUse::Borrowed
            } else {
                LiteralUse::CustodyTaking
            };
            let mut reader = index.reader(&never_bound);
            let mut leaves = Vec::new();
            let identity_free = children(literal).first().is_some_and(|init| {
                read_initializer(&mut reader, facts, init, &mut leaves).is_ok()
            });
            let decision = decide_literal(identity_free, reader.plan.accesses > 0, use_);
            rows.push(json!({
                "formal": formal,
                "use": use_.code(),
                "leaves": leaves.iter().map(|leaf| leaf.code()).collect::<Vec<_>>(),
            }));
            if let Err(refusal) = decision {
                let refusal = match (refusal, leaves.iter().find(|leaf| !leaf.admitted())) {
                    (Refusal::Leaf(_), Some(leaf)) => Refusal::Leaf(*leaf),
                    (refusal, _) => refusal,
                };
                return (Err(refusal), json!({ "callee": callee, "literals": rows }));
            }
            literals.push(formal);
        }
        if literals.is_empty() {
            return (Err(Refusal::LiteralPosition), json!({ "callee": callee }));
        }
        // The statement with each literal actual respelled `0`.
        let mut rewritten = tokens[..call_index + 2].to_vec();
        for (position, &(start, end)) in spans.iter().enumerate() {
            if position > 0 {
                rewritten.push(tokens[start - 1].clone());
            }
            if literals.contains(&position) {
                rewritten.push(Token {
                    text: "0".to_owned(),
                    line: tokens[start].line,
                });
            } else {
                rewritten.extend_from_slice(&tokens[start..end]);
            }
        }
        rewritten.push(tokens[tokens.len() - 1].clone());
        let detail = json!({ "callee": callee, "literals": rows });
        if super::contains_braces(&rewritten) {
            return (Err(Refusal::LiteralPosition), detail);
        }
        let saved = self.probe_state();
        let use_ = if destination.is_some() {
            ResultUse::Stored
        } else {
            ResultUse::Discarded
        };
        match self.instantiate_call(&callee, &rewritten, destination, use_) {
            Ok(sequence) => (
                Ok(Statement::Block(
                    sequence
                        .into_iter()
                        .map(|instruction| Statement::Instruction(instruction, line))
                        .collect(),
                )),
                detail,
            ),
            Err(refusal) => {
                self.restore_probe_state(&saved);
                (Err(Refusal::Instantiation(format!("{refusal:?}"))), detail)
            }
        }
    }
}

/// The flow guard of an initializer of a local that can hold a pointer: rule
/// 10v's flow, reused (`LegacyInstruction::PointerStore`, owner `None`, the
/// local a record object of this frame). The declaration is exactly the
/// older refusal where a place rooted at the local is bound with a target or
/// unknown (a stale binding the new value would contradict); otherwise it is
/// its typed reads, after which every place rooted at the local is forgotten.
/// Its pointer members are the null constant, a string literal or a function,
/// all neutral, so the store's value class is `null`. As for every admitted
/// pointer store, the body then publishes no derived summary.
fn guarded_initializer(
    root: String,
    lowered: Statement,
    fallback: &LegacyInstruction,
    line: usize,
) -> Statement {
    let access = match lowered {
        Statement::Instruction(access, _) => access,
        other => return other,
    };
    Statement::Instruction(
        LegacyInstruction::PointerStore {
            store: crate::pointer_store::PointerStore {
                place: format!("{root} (initializer)"),
                root,
                owner: None,
                value: "null",
                name: None,
            },
            access: Box::new(access),
            fallback: Box::new(fallback.clone()),
        },
        line,
    )
}

/// The half-open token spans of one call's arguments, like
/// [`super::call_argument_spans`] but also nesting braces: a compound
/// literal's list holds commas.
fn argument_spans(tokens: &[&str], call_index: usize) -> Option<Vec<(usize, usize)>> {
    let mut arguments = Vec::new();
    let mut start = call_index + 2;
    let mut depth = 0usize;
    for (cursor, token) in tokens.iter().enumerate().skip(start) {
        match *token {
            "(" | "[" | "{" => depth += 1,
            ")" if depth == 0 => {
                if cursor > start {
                    arguments.push((start, cursor));
                }
                return Some(arguments);
            }
            ")" | "]" | "}" => depth = depth.checked_sub(1)?,
            "," if depth == 0 => {
                arguments.push((start, cursor));
                start = cursor + 1;
            }
            _ => {}
        }
    }
    None
}
