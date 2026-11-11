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

//! Card **CG02-ELABORATION-CORE** (gateway rule 10s): the elaborator of
//! [`crate::elaboration`] connected to the custody gateway (route G).
//!
//! Every hook here is a **fallback**: it is consulted only where every
//! existing route of its position refused, so an answer an older route gives
//! is never decided by different code (hard invariant 3), and a declined
//! answer leaves the refusal byte for byte (the classifier's site and element
//! counters are restored). Each consultation is recorded in the
//! `QCORE_TYPED_PATH_PARITY` stream with `position: "elaboration-…"`.
//!
//! * **Statements** (`expression_statement`'s last step). The statement is
//!   matched to one Clang node by its line and exact tokens and elaborated;
//!   the whole graph must pass the order model. It is then lowered in
//!   sequenced **pieces**: every operand of a top-level comma, and the inner
//!   assignment of a `NAME = (NAME2 = …)` chain, is a statement of its own,
//!   decided first by the existing statement lowering (so a call keeps its one
//!   contract application) and only then by the elaborator's own events. A
//!   declaration or assignment whose value is a comma sequences the leading
//!   operands and then declares or assigns the last one.
//! * **Events.** A piece the existing lowering refuses is lowered from its
//!   events in the graph's order, which an admitted graph makes one of its
//!   permitted serialisations with the same outcome: a scalar object write is
//!   `ForgetScalar`, a path read or write is the typed access-path reader's
//!   `IndirectScalarAccess` (its dereference obligations checked by `transfer`
//!   exactly as for W46's statements), a scalar object read is nothing. A call
//!   outside a piece, a pointer-valued store, a write to an object the gateway
//!   does not read as a scalar local, a protected formal or a tracked
//!   descriptor refuse the whole statement.
//! * **Returns.** Leading comma operands are pieces; the last operand is
//!   handed to the existing `return` lowering. An assignment `NAME = E` is
//!   the piece `NAME = E` then `return NAME`. Otherwise a scalar value of a
//!   scalar-result function is its events then a `NoOp` exit: nothing escapes.
//! * **Conditions.** A call-free controlling expression holding one of this
//!   card's value forms and no connective is its events, then a branch that
//!   refines nothing. A pointer-valued condition stays refused (its null
//!   refinement is the older routes').

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::condition_desugar::{ConditionPlan, PlanNode, Position, Target};
use super::{
    LegacyInstruction, Parser, RefusedWrites, Statement, Token, contains_braces, is_identifier,
};
use crate::elaboration::{
    self, Context, Elaboration, Event, Operand, Place, Refusal, strip_parens,
};
use crate::function_reading_reuse::LineTree;
use crate::typed_access_path::{StatementIndex, StatementPlan, is_pointer, is_scalar, record_row};
use crate::typed_c_ast::{children, kind};

/// The G context: the typed access-path reader answers places, the function's
/// AST answers aliasing.
struct GContext<'a> {
    index: &'a StatementIndex,
    never_bound: &'a dyn Fn(&str) -> bool,
    /// Declaration ids of the function's automatic locals and formals, and
    /// those whose address a `&` takes somewhere in it; read on first use.
    aliasing: std::cell::OnceCell<(BTreeSet<String>, BTreeSet<String>)>,
    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the contracts call
    /// footprints are read from, where the statement route lowers calls.
    calls: Option<super::elaborator_calls_route::CallContracts<'a>>,
}

impl Context for GContext<'_> {
    type Facts = StatementPlan;

    fn scalar(&self, node: &Value) -> bool {
        is_scalar(node)
            || self.index.enums.is_enum(node)
            || crate::typed_position_shapes::anonymous_enum_member(&self.index.enums, node)
    }

    fn pointer(&self, node: &Value) -> bool {
        is_pointer(node)
    }

    fn place(&self, node: &Value, store: bool) -> Result<StatementPlan, Refusal> {
        let mut reader = self.index.reader(self.never_bound);
        reader
            .path(node, store, false)
            .ok_or_else(|| Refusal::Place("the typed access-path reader refused it".into()))?;
        Ok(reader.plan)
    }

    fn unaliased(&self, decl: &str) -> bool {
        let (automatic, taken) = self.aliasing.get_or_init(|| aliasing(&self.index.function));
        automatic.contains(decl) && !taken.contains(decl)
    }

    fn shared_bitfield_unit(&self, node: &Value, one: &str, other: &str) -> Option<u32> {
        crate::union_bitfield::shared_unit_refusal(
            node,
            &self.index.records,
            one,
            other,
            &crate::typed_access_path::record_layout_dump,
        )
    }

    /// Card CG18-FOOTPRINT-CONNECTION (rule 11k): two members of one struct
    /// object, decided by the core footprint model from Clang's layout.
    fn disjoint_places(&self, node: &Value, one: &str, other: &str) -> bool {
        crate::footprint_connection::members_disjoint(
            node,
            &self.index.records,
            one,
            other,
            &crate::typed_access_path::record_layout_dump,
        )
    }

    /// Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b): a pointer read of one
    /// automatic local or formal the custody closure proves never bound to a
    /// resource identity. Every other pointer value (a path, a call, a
    /// static or file-scope object) may carry one.
    fn untracked_pointer(&self, node: &Value) -> bool {
        let operand = elaboration::pointer_operand(node);
        if kind(operand) != "DeclRefExpr" {
            return false;
        }
        let reference = &operand["referencedDecl"];
        let (Some(id), Some(name)) = (reference["id"].as_str(), reference["name"].as_str()) else {
            return false;
        };
        let (automatic, _) = self.aliasing.get_or_init(|| aliasing(&self.index.function));
        automatic.contains(id) && (self.never_bound)(name)
    }

    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the call's contract
    /// footprint, read as rule 10y reads a call of its tree.
    fn call_footprint(&self, call: &Value) -> Option<elaboration::Footprint> {
        let calls = self.calls?;
        let (automatic, taken) = self.aliasing.get_or_init(|| aliasing(&self.index.function));
        let facts = super::elaborator_calls_route::ElaboratorFacts {
            contracts: calls.contracts,
            tails: calls.tails,
            fields: &self.index.fields,
            automatic,
            taken,
        };
        crate::nested_call_effects::call_footprint(call, &facts).ok()
    }
}

/// Walk a function's AST once for its automatic objects and taken addresses.
/// Card CG05-NESTED-CALL-EFFECTS (rule 10y) reads the same facts.
///
/// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n) corrects what takes an
/// address: besides `&x`, the address of a member or element of an object
/// (`&s.m`, `&a[i]`) and an array's decay to a pointer (`p = a`, `f(a)`) take
/// the address of the object the place is rooted at, through `.` members,
/// array subscripts and parentheses. A decay that is only indexed (`a[i]`)
/// takes none. Only an aggregate's set changes: a scalar or pointer object's
/// address is taken by `&x` alone, as before.
pub(super) fn aliasing(function: &Value) -> (BTreeSet<String>, BTreeSet<String>) {
    /// The object a place is rooted at in its own storage.
    fn root(node: &Value) -> Option<&str> {
        let node = strip_parens(node);
        match kind(node) {
            "DeclRefExpr" => node["referencedDecl"]["id"].as_str(),
            "MemberExpr" if node["isArrow"].as_bool() != Some(true) => {
                root(children(node).first()?)
            }
            "ArraySubscriptExpr" => {
                let base = strip_parens(children(node).first()?);
                (kind(base) == "ImplicitCastExpr"
                    && base["castKind"].as_str() == Some("ArrayToPointerDecay"))
                .then(|| root(children(base).first()?))
                .flatten()
            }
            _ => None,
        }
    }
    fn walk(node: &Value, automatic: &mut BTreeSet<String>, taken: &mut BTreeSet<String>) {
        match kind(node) {
            "ParmVarDecl" | "VarDecl" if node["storageClass"].as_str().is_none() => {
                if let Some(id) = node["id"].as_str() {
                    automatic.insert(id.to_owned());
                }
            }
            "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                if let Some(id) = children(node).first().and_then(root) {
                    taken.insert(id.to_owned());
                }
            }
            "ImplicitCastExpr" if node["castKind"].as_str() == Some("ArrayToPointerDecay") => {
                if let Some(id) = children(node).first().and_then(root) {
                    taken.insert(id.to_owned());
                }
            }
            "ArraySubscriptExpr" => {
                // The base's decay is only indexed: its own operand is walked,
                // the decay itself takes no address.
                let mut parts = children(node).iter();
                if let Some(base) = parts.next() {
                    let bare = strip_parens(base);
                    if kind(bare) == "ImplicitCastExpr"
                        && bare["castKind"].as_str() == Some("ArrayToPointerDecay")
                    {
                        for child in children(bare) {
                            walk(child, automatic, taken);
                        }
                    } else {
                        walk(base, automatic, taken);
                    }
                }
                for child in parts {
                    walk(child, automatic, taken);
                }
                return;
            }
            _ => {}
        }
        for child in children(node) {
            walk(child, automatic, taken);
        }
    }
    let mut automatic = BTreeSet::new();
    let mut taken = BTreeSet::new();
    walk(function, &mut automatic, &mut taken);
    (automatic, taken)
}

/// Whether a node holds one of this card's value forms, or (card CG08, rule
/// 11b) a cast or type query `cast_representation::form` names.
fn holds_value_form(node: &Value) -> bool {
    elaboration::value_form(node).is_some()
        || crate::cast_representation::form(node).is_some()
        || children(node).iter().any(holds_value_form)
}

/// Whether a statement holds one of this card's forms: a compound
/// assignment, an increment, a comma, or an assignment *used as a value*. A
/// statement's own top-level `=` is a plain store, whose refusals belong to
/// the older routes and to other cards (a type query is CG08's).
fn statement_holds_value_form(node: &Value) -> bool {
    let bare = strip_parens(node);
    if kind(bare) == "BinaryOperator" && bare["opcode"].as_str() == Some("=") {
        return children(bare).iter().any(holds_value_form);
    }
    holds_value_form(bare)
}

/// The operands of a (possibly parenthesised) comma chain, in order.
fn comma_operands(node: &Value) -> Vec<&Value> {
    let bare = strip_parens(node);
    if kind(bare) == "BinaryOperator"
        && bare["opcode"].as_str() == Some(",")
        && let [left, right] = children(bare)
    {
        let mut operands = comma_operands(left);
        operands.extend(comma_operands(right));
        return operands;
    }
    vec![bare]
}

fn named_object(node: &Value) -> Option<(&str, &str)> {
    let bare = strip_parens(node);
    (kind(bare) == "DeclRefExpr"
        && matches!(
            bare["referencedDecl"]["kind"].as_str(),
            Some("VarDecl" | "ParmVarDecl")
        ))
    .then(|| {
        Some((
            bare["referencedDecl"]["id"].as_str()?,
            bare["referencedDecl"]["name"].as_str()?,
        ))
    })
    .flatten()
}

/// One sequenced step of a lowered statement.
enum Step<'a> {
    /// A sub-expression statement.
    Piece(&'a Value),
    /// `DECLARATION = VALUE` with the declaration's own tokens before `=`.
    Declare(&'a Value, &'a Value),
    /// `LEFT = VALUE`.
    Assign(&'a Value, &'a Value),
}

/// Where the elaborator is asked.
#[derive(Clone, Copy)]
enum Site {
    Statement,
    Return,
    Condition,
}

impl Site {
    fn label(self) -> &'static str {
        match self {
            Site::Statement => "elaboration-statement",
            Site::Return => "elaboration-return",
            Site::Condition => "elaboration-condition",
        }
    }
}

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

impl Parser<'_> {
    /// The function's Clang index, shared (it owns only `Arc`s), so the
    /// lowering below may call back into the statement walker.
    fn elaboration_index(&self) -> Option<StatementIndex> {
        self.typed_positions_index().cloned()
    }

    /// The unique outermost node on `line` whose tokens are `wanted` and whose
    /// kind `accept` takes. A match's own sub-nodes (implicit wrappers) are the
    /// same text and are not visited; two separate matches are ambiguous and
    /// answer nothing.
    pub(super) fn elaboration_node<'i>(
        index: &'i StatementIndex,
        line: usize,
        wanted: &[&str],
        accept: &dyn Fn(&Value) -> bool,
    ) -> Option<&'i Value> {
        #[allow(
            clippy::too_many_arguments,
            reason = "a recursive walk's fixed context"
        )]
        fn walk<'i>(
            index: &StatementIndex,
            node: &'i Value,
            // Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rule 13t): `node`'s
            // own begin and end line (`index.node_line`, `node_end_line`),
            // read from the function's line tree, built once, in the same
            // child order, so this walk visits exactly the nodes it visited
            // when it computed them.
            lines: &LineTree,
            line: usize,
            wanted: &[&str],
            accept: &dyn Fn(&Value) -> bool,
            stripped: bool,
            out: &mut Vec<&'i Value>,
        ) {
            // A subtree whose known range lies wholly before or after the
            // line holds no candidate.
            if lines.begin.is_some_and(|begin| begin > line)
                || lines.end.is_some_and(|end| end < line)
            {
                return;
            }
            if accept(node)
                && lines.begin == Some(line)
                && index.node_tokens_once(node).is_some_and(|found| {
                    if stripped {
                        let texts = found.iter().map(String::as_str).collect::<Vec<_>>();
                        super::parenthesised_callee::strip_callee_parentheses(&texts)
                            .iter()
                            .map(String::as_str)
                            .eq(wanted.iter().copied())
                    } else {
                        found.iter().map(String::as_str).eq(wanted.iter().copied())
                    }
                })
            {
                out.push(node);
                return;
            }
            let subtrees = lines.children(node, &|node| index.node_line(node), &|node| {
                index.node_end_line(node)
            });
            for (child, lines) in children(node).iter().zip(subtrees) {
                walk(index, child, lines, line, wanted, accept, stripped, out);
            }
        }
        let lines = index.line_tree();
        let mut found = Vec::new();
        walk(
            index,
            &index.function,
            lines,
            line,
            wanted,
            accept,
            false,
            &mut found,
        );
        // Card CG05-ELABORATOR-CALL-FOOTPRINTS: the token stream spells a
        // parenthesised callee `(f)(x)` as `f(x)` (W45-PARENTHESISED-CALLEE's
        // rule 1), Clang's node keeps the parentheses; with no exact match,
        // the node is matched with its callee parentheses stripped the same
        // way, so both spellings are decided alike.
        if found.is_empty() {
            walk(
                index,
                &index.function,
                lines,
                line,
                wanted,
                accept,
                true,
                &mut found,
            );
        }
        match found.as_slice() {
            [node] => Some(node),
            _ => None,
        }
    }

    fn elaboration_context<'i>(
        index: &'i StatementIndex,
        never_bound: &'i dyn Fn(&str) -> bool,
    ) -> GContext<'i> {
        GContext {
            index,
            never_bound,
            aliasing: std::cell::OnceCell::new(),
            calls: None,
        }
    }

    /// Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the G context with
    /// the call footprints in force (the statement route only).
    fn elaboration_context_with_calls<'i>(
        &'i self,
        index: &'i StatementIndex,
        never_bound: &'i dyn Fn(&str) -> bool,
    ) -> GContext<'i> {
        GContext {
            calls: Some(self.call_contracts()),
            ..Self::elaboration_context(index, never_bound)
        }
    }

    /// Premises every site shares before any AST is read.
    fn elaboration_declines(&self, tokens: &[Token]) -> bool {
        let texts = texts(tokens);
        tokens.is_empty()
            || contains_braces(tokens)
            || self.opaque_callee(tokens)
            || texts
                .iter()
                .any(|text| is_identifier(text) && self.descriptor_names.contains(text))
            || (!self.list_frames.is_empty() && self.list_iteration_refusal(&texts).is_some())
    }

    fn record_elaboration(
        &self,
        site: Site,
        line: usize,
        tokens: &[Token],
        old: &str,
        refusal: Option<&Refusal>,
    ) {
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts(tokens),
            "position": site.label(),
            "old_route": "old_refusal",
            "old_refusal": old,
            "old_admitted": false,
            "new_admitted": refusal.is_none(),
            "new_refusal": refusal.map(Refusal::reason),
            "disagreement": refusal.is_none(),
        }));
    }

    /// Hook: a statement every existing route refused (`old` is its
    /// refusal). `None` keeps the refusal byte for byte.
    pub(super) fn elaborated_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        old: &str,
    ) -> Option<Statement> {
        if self.elaboration_declines(tokens) {
            return None;
        }
        let index = self.elaboration_index()?;
        let saved = self.probe_state();
        let outcome = self.elaborate_statement(&index, tokens, line);
        self.record_elaboration(Site::Statement, line, tokens, old, outcome.as_ref().err());
        match outcome {
            Ok(statements) => Some(Statement::Block(statements)),
            Err(_) => {
                self.restore_probe_state(&saved);
                None
            }
        }
    }

    fn elaborate_statement(
        &mut self,
        index: &StatementIndex,
        tokens: &[Token],
        line: usize,
    ) -> Result<Vec<Statement>, Refusal> {
        let wanted = texts(tokens);
        let node = Self::elaboration_node(index, tokens[0].line, &wanted, &|node| {
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
        })
        .ok_or(Refusal::Unlisted("no unique Clang node for the statement"))?;
        if !statement_holds_value_form(node) {
            return Err(Refusal::Unlisted("no value form of this card"));
        }
        // The whole statement passes the order model before any piece is
        // lowered.
        {
            let never_bound = |name: &str| self.joined_never_bound(name);
            let context = self.elaboration_context_with_calls(index, &never_bound);
            elaboration::elaborate(node, &context)?;
        }
        let steps = self.steps(node)?;
        let mut statements = Vec::new();
        for step in &steps {
            statements.extend(self.lower_step(index, step, tokens, line, true)?);
        }
        Ok(statements)
    }

    /// The sequenced steps of a statement node.
    fn steps<'n>(&self, node: &'n Value) -> Result<Vec<Step<'n>>, Refusal> {
        let bare = strip_parens(node);
        if kind(bare) == "DeclStmt" {
            let [declaration] = children(bare) else {
                return Err(Refusal::Declaration("not exactly one declarator"));
            };
            let name = declaration["name"].as_str().unwrap_or_default();
            if self.formal_parameters.contains(name) {
                return Err(Refusal::Declaration("may shadow a formal"));
            }
            let [initializer] = children(declaration) else {
                return Err(Refusal::Declaration("no single initializer"));
            };
            let operands = comma_operands(initializer);
            let Some((last, leading)) = operands.split_last() else {
                return Err(Refusal::Declaration("no initializer"));
            };
            if leading.is_empty() {
                // Card CHAINED-ASSIGNMENT-INNER-DESTINATION (rule 5i,
                // amended): `T NAME = (NAME2 = VALUE)` is the inner
                // assignment, then `T NAME = NAME2` (C11 6.5.16p3), exactly as
                // an assignment's chain below.
                let inner = strip_parens(last);
                if kind(inner) == "BinaryOperator"
                    && inner["opcode"].as_str() == Some("=")
                    && let [inner_name, _] = children(inner)
                    && let Some((inner_id, _)) = named_object(inner_name)
                    && declaration["id"].as_str().is_some_and(|id| id != inner_id)
                {
                    let mut steps = self.steps(inner)?;
                    steps.push(Step::Declare(bare, inner_name));
                    return Ok(steps);
                }
                return Ok(vec![Step::Piece(bare)]);
            }
            let mut steps = Vec::new();
            for operand in leading {
                steps.extend(self.steps(operand)?);
            }
            steps.push(Step::Declare(bare, last));
            return Ok(steps);
        }
        let operands = comma_operands(bare);
        if operands.len() > 1 {
            let mut steps = Vec::new();
            for operand in operands {
                steps.extend(self.steps(operand)?);
            }
            return Ok(steps);
        }
        if kind(bare) == "BinaryOperator"
            && bare["opcode"].as_str() == Some("=")
            && let [left, right] = children(bare)
        {
            let operands = comma_operands(right);
            if let Some((last, leading)) = operands.split_last()
                && !leading.is_empty()
            {
                let mut steps = Vec::new();
                for operand in leading {
                    steps.extend(self.steps(operand)?);
                }
                steps.push(Step::Assign(left, last));
                return Ok(steps);
            }
            // `LEFT = (NAME = VALUE)`: the inner assignment, then `LEFT = NAME`.
            // The two objects are distinct, so their unsequenced stores commute.
            let inner = strip_parens(right);
            if kind(inner) == "BinaryOperator"
                && inner["opcode"].as_str() == Some("=")
                && let [name, _] = children(inner)
                && let Some((inner_id, _)) = named_object(name)
                && named_object(left).is_some_and(|(outer_id, _)| outer_id != inner_id)
            {
                let mut steps = self.steps(inner)?;
                steps.push(Step::Assign(left, name));
                return Ok(steps);
            }
        }
        Ok(vec![Step::Piece(bare)])
    }

    fn node_texts(index: &StatementIndex, node: &Value) -> Result<Vec<String>, Refusal> {
        index
            .node_tokens(node)
            .ok_or(Refusal::Unlisted("node without tokens"))
    }

    /// The tokens of `wanted` as they stand in `statement` (their own lines),
    /// or synthesised on the statement's first line when they are not one run
    /// of it.
    fn step_tokens(statement: &[Token], wanted: &[String], line: usize) -> Vec<Token> {
        if !wanted.is_empty() {
            for start in 0..statement.len() {
                let end = start + wanted.len();
                if end <= statement.len()
                    && statement[start..end]
                        .iter()
                        .zip(wanted)
                        .all(|(token, text)| &token.text == text)
                {
                    return statement[start..end].to_vec();
                }
            }
        }
        let at = statement.first().map_or(line, |token| token.line);
        wanted
            .iter()
            .map(|text| Token {
                text: text.clone(),
                line: at,
            })
            .collect()
    }

    /// Lower one step. `refused` says the enclosing statement's own tokens
    /// were already refused by the statement lowering, so a step spelling all
    /// of them goes straight to its events.
    fn lower_step(
        &mut self,
        index: &StatementIndex,
        step: &Step<'_>,
        statement: &[Token],
        line: usize,
        refused: bool,
    ) -> Result<Vec<Statement>, Refusal> {
        let (wanted, synthetic) = match step {
            Step::Piece(node) => (Self::node_texts(index, node)?, (*node).clone()),
            Step::Declare(declaration, value) => {
                // The declaration's own tokens up to its top-level `=`.
                let whole = Self::node_texts(index, declaration)?;
                let equal = whole
                    .iter()
                    .position(|text| text == "=")
                    .ok_or(Refusal::Declaration("no initializer"))?;
                let mut wanted = whole[..=equal].to_vec();
                wanted.extend(Self::node_texts(index, value)?);
                let mut synthetic = (*declaration).clone();
                synthetic["inner"][0]["inner"] = json!([(*value).clone()]);
                (wanted, synthetic)
            }
            Step::Assign(left, value) => {
                let mut wanted = Self::node_texts(index, left)?;
                wanted.push("=".to_owned());
                wanted.extend(Self::node_texts(index, value)?);
                let value = if kind(strip_parens(value)) == "DeclRefExpr" {
                    // A named object used as a value is read.
                    json!({
                        "kind": "ImplicitCastExpr",
                        "castKind": "LValueToRValue",
                        "type": value["type"].clone(),
                        "inner": [(*value).clone()],
                    })
                } else {
                    (*value).clone()
                };
                let synthetic = json!({
                    "kind": "BinaryOperator",
                    "opcode": "=",
                    "type": left["type"].clone(),
                    "inner": [(*left).clone(), value],
                });
                (wanted, synthetic)
            }
        };
        let tokens = Self::step_tokens(statement, &wanted, line);
        let whole = refused && texts(&tokens) == texts(statement);
        if !whole {
            let saved = self.probe_state();
            let lowered = self.expression_statement(&tokens, line, None);
            if !refuses(&lowered) {
                return Ok(vec![lowered]);
            }
            self.restore_probe_state(&saved);
        }
        // A piece that holds none of this card's forms keeps the older
        // routes' refusal (a plain store's refusal is theirs).
        if matches!(step, Step::Piece(_)) && !statement_holds_value_form(&synthetic) {
            return Err(Refusal::Unlisted(
                "a sequenced piece outside this card's forms",
            ));
        }
        // Card CG03-CALL-SUMMARY-INSTANTIATION (rule 10u): a piece that is
        // one direct call whose actuals carry effects C sequences before it.
        if let Step::Piece(node) = step {
            let elaborated = {
                let never_bound = |name: &str| self.joined_never_bound(name);
                let context = self.elaboration_context_with_calls(index, &never_bound);
                elaboration::elaborate(node, &context)?
            };
            if let Ok(Some(instructions)) =
                self.effectful_call_piece(index, node, &elaborated, line)
            {
                return Ok(instructions
                    .into_iter()
                    .map(|instruction| Statement::Instruction(instruction, line))
                    .collect());
            }
        }
        // Card CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): a piece holding
        // calls, each with its contract footprint, through the one
        // application point.
        let elaborated = {
            let never_bound = |name: &str| self.joined_never_bound(name);
            let context = self.elaboration_context_with_calls(index, &never_bound);
            elaboration::elaborate(&synthetic, &context)?
        };
        if let Some(instructions) = self.footprint_call_events(&elaborated, index, line)? {
            return Ok(instructions
                .into_iter()
                .map(|instruction| Statement::Instruction(instruction, line))
                .collect());
        }
        let instructions = self.event_instructions(index, &synthetic, Site::Statement)?;
        Ok(instructions
            .into_iter()
            .map(|instruction| Statement::Instruction(instruction, line))
            .collect())
    }

    /// Elaborate `node` and lower its events in the graph's order.
    fn event_instructions(
        &self,
        index: &StatementIndex,
        node: &Value,
        site: Site,
    ) -> Result<Vec<LegacyInstruction>, Refusal> {
        let statement_never_bound = |name: &str| self.joined_never_bound(name);
        let position_never_bound = |name: &str| self.position_never_bound(name);
        let never_bound: &dyn Fn(&str) -> bool = match site {
            Site::Statement => &statement_never_bound,
            Site::Return | Site::Condition => &position_never_bound,
        };
        let context = Self::elaboration_context(index, never_bound);
        let elaborated = elaboration::elaborate(node, &context)?;
        self.lower_events(&elaborated)
    }

    /// Card CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (rule 10z): the events of
    /// one edge's statement node (its value regions substituted by the arm C
    /// evaluates on that edge), lowered as a refused piece's are.
    pub(super) fn leaf_events(
        &self,
        index: &StatementIndex,
        node: &Value,
    ) -> Result<Vec<LegacyInstruction>, Refusal> {
        self.event_instructions(index, node, Site::Statement)
    }

    fn lower_events(
        &self,
        elaborated: &Elaboration<StatementPlan>,
    ) -> Result<Vec<LegacyInstruction>, Refusal> {
        let mut instructions = Vec::new();
        for id in 0..elaborated.graph.events.len() {
            if let Some(instruction) = self.lower_event(elaborated, id)? {
                instructions.push(instruction);
            }
        }
        Ok(instructions)
    }

    /// One event's instruction, `None` for an event that lowers to nothing.
    /// Card CG03-CALL-SUMMARY-INSTANTIATION lowers the events before a
    /// sequenced call through this same function.
    pub(super) fn lower_event(
        &self,
        elaborated: &Elaboration<StatementPlan>,
        id: usize,
    ) -> Result<Option<LegacyInstruction>, Refusal> {
        let event = &elaborated.graph.events[id];
        match event {
            Event::Read(Place::Object { .. }) => Ok(None),
            // A read inside a path's address: the typed reader's plan of
            // that path carries its base and per-link obligations.
            Event::Read(Place::Path { .. }) if elaborated.address_reads.contains(&id) => Ok(None),
            Event::Write {
                place: Place::Object { name, .. },
                pointer: false,
            } => {
                if self.protected_formals.contains(name)
                    || !crate::typed_field_access::is_scalar_initializer(
                        name,
                        &["1"],
                        &self.typed_model,
                        &self.typed_types,
                    )
                {
                    return Err(Refusal::Unlisted(
                        "write to an object the gateway does not read as a scalar local",
                    ));
                }
                Ok(Some(LegacyInstruction::ForgetScalar {
                    destination: name.clone(),
                }))
            }
            Event::Read(Place::Path { .. })
            | Event::Write {
                place: Place::Path { .. },
                pointer: false,
            } => {
                let plan = elaborated
                    .facts
                    .get(&id)
                    .ok_or(Refusal::Unlisted("path event without facts"))?;
                let write = matches!(event, Event::Write { .. });
                Ok(Some(self.path_instruction(plan, write)))
            }
            Event::Write { pointer: true, .. } => Err(elaboration::refused("pointer store")),
            Event::Call { callee } => {
                let effectful = elaborated.actuals.get(&id).is_some_and(|events| {
                    events
                        .iter()
                        .any(|&event| !matches!(elaborated.graph.events[event], Event::Read(_)))
                });
                Err(if effectful {
                    Refusal::EffectfulActual(callee.clone())
                } else {
                    Refusal::UnorderedCall(callee.clone())
                })
            }
            _ => Err(Refusal::Unlisted("event outside this card's lowering")),
        }
    }

    /// The typed access-path reader's instruction for one path event, as
    /// `typed_path_statement` lowers a W46 statement.
    fn path_instruction(&self, plan: &StatementPlan, write: bool) -> LegacyInstruction {
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: plan
                .scalar
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.scalar.bases.clone(),
            destination: None,
            frame: (write && plan.scalar.writes_through_pointer)
                .then(|| RefusedWrites::Sparing(self.frame_protected.clone())),
            chains: plan.chains.clone(),
            borrowed_destination: false,
            readback: None,
        }
    }

    /// Hook: a `return` every existing route refused. `None` keeps the
    /// refusal.
    pub(super) fn elaborated_return(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        if self.elaboration_declines(expression) {
            return None;
        }
        let index = self.elaboration_index()?;
        let saved = self.probe_state();
        let outcome = self.elaborate_return(&index, expression, line);
        self.record_elaboration(
            Site::Return,
            line,
            expression,
            "unmodeled call, aggregate or indirect access in return expression",
            outcome.as_ref().err(),
        );
        match outcome {
            Ok(statements) => Some(Statement::Block(statements)),
            Err(_) => {
                self.restore_probe_state(&saved);
                None
            }
        }
    }

    fn elaborate_return(
        &mut self,
        index: &StatementIndex,
        expression: &[Token],
        line: usize,
    ) -> Result<Vec<Statement>, Refusal> {
        let mut wanted = vec!["return"];
        wanted.extend(texts(expression));
        let statement = Self::elaboration_node(index, expression[0].line, &wanted, &|node| {
            kind(node) == "ReturnStmt"
        })
        .ok_or(Refusal::Unlisted("no unique Clang node for the return"))?;
        let [node] = children(statement) else {
            return Err(Refusal::Unlisted("return without one value"));
        };
        if !holds_value_form(node) {
            return Err(Refusal::Unlisted("no value form of this card"));
        }
        let result = {
            let never_bound = |name: &str| self.position_never_bound(name);
            let context = Self::elaboration_context(index, &never_bound);
            elaboration::elaborate(node, &context)?.result
        };
        let operands = comma_operands(node);
        let Some((&last, leading)) = operands.split_last() else {
            return Err(Refusal::Unlisted("empty return"));
        };
        let mut statements = Vec::new();
        for operand in leading {
            for step in self.steps(operand)? {
                statements.extend(self.lower_step(index, &step, expression, line, false)?);
            }
        }
        // `return NAME = VALUE`: the assignment, then `return NAME`.
        if kind(last) == "BinaryOperator"
            && last["opcode"].as_str() == Some("=")
            && let [left, _] = children(last)
            && let Some((_, name)) = named_object(left)
        {
            for step in self.steps(last)? {
                statements.extend(self.lower_step(index, &step, expression, line, false)?);
            }
            let name = Self::step_tokens(expression, &[name.to_owned()], line);
            let returned = self.return_statement(name, line);
            if super::return_refused(&returned) {
                return Err(Refusal::Unlisted("the assigned object's return is refused"));
            }
            statements.push(returned);
            return Ok(statements);
        }
        if !leading.is_empty() {
            let wanted = Self::node_texts(index, last)?;
            let tokens = Self::step_tokens(expression, &wanted, line);
            let saved = self.probe_state();
            let returned = self.return_statement(tokens, line);
            if !super::return_refused(&returned) {
                statements.push(returned);
                return Ok(statements);
            }
            self.restore_probe_state(&saved);
        }
        // A scalar value's events, then an exit that escapes nothing.
        if !matches!(result, Operand::Value { pointer: false, .. })
            || !self.return_facts.scalar_result
        {
            return Err(Refusal::Unlisted("a returned value that is not a scalar"));
        }
        let instructions = self.event_instructions(index, last, Site::Return)?;
        statements.extend(
            instructions
                .into_iter()
                .map(|instruction| Statement::Instruction(instruction, line)),
        );
        statements.push(Statement::Return {
            line,
            writes: self.expression_writes(expression),
            expression_supported: true,
            returned_place: None,
            returned_evaluation: None,
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        });
        Ok(statements)
    }

    /// Hook: a call-free controlling expression every existing route refused.
    /// `None` keeps the refusal.
    pub(super) fn elaborated_condition(
        &mut self,
        condition: &[Token],
        line: usize,
        position: Position,
    ) -> Option<Box<ConditionPlan>> {
        if self.elaboration_declines(condition)
            || super::cast_representation_route::calls_outside_type_queries(condition)
        {
            return None;
        }
        let index = self.elaboration_index()?;
        let outcome = self.elaborate_condition(&index, condition);
        self.record_elaboration(
            Site::Condition,
            line,
            condition,
            position.refusal(),
            outcome.as_ref().err(),
        );
        outcome.ok().map(Box::new)
    }

    /// Card CG04-CONDITION-ACQUISITION (rule 10x): one call-free leaf of a
    /// desugared condition, decided as the hook above decides a whole
    /// call-free condition, under the same premises. Its events in order; the
    /// caller adds the leaf's branch, which refines nothing.
    pub(super) fn elaborated_leaf(&self, leaf: &[Token]) -> Option<Vec<LegacyInstruction>> {
        if self.elaboration_declines(leaf)
            || super::cast_representation_route::calls_outside_type_queries(leaf)
        {
            return None;
        }
        let index = self.elaboration_index()?;
        let mut plan = self.elaborate_condition(&index, leaf).ok()?;
        // The last node is the branch that refines nothing.
        plan.nodes.pop();
        Some(
            plan.nodes
                .into_iter()
                .map(|node| node.instruction)
                .collect(),
        )
    }

    fn elaborate_condition(
        &self,
        index: &StatementIndex,
        condition: &[Token],
    ) -> Result<ConditionPlan, Refusal> {
        let wanted = texts(condition);
        let node = Self::elaboration_node(index, condition[0].line, &wanted, &|node| {
            !kind(node).ends_with("Stmt") && !kind(node).ends_with("Decl")
        })
        .ok_or(Refusal::Unlisted("no unique Clang node for the condition"))?;
        if !holds_value_form(node) {
            return Err(Refusal::Unlisted("no value form of this card"));
        }
        let never_bound = |name: &str| self.position_never_bound(name);
        let context = Self::elaboration_context(index, &never_bound);
        let elaborated = elaboration::elaborate(node, &context)?;
        if !matches!(elaborated.result, Operand::Value { pointer: false, .. }) {
            return Err(Refusal::Unlisted("a condition that is not a scalar value"));
        }
        let instructions = self.lower_events(&elaborated)?;
        // The events in order, then one branch that refines nothing.
        let mut nodes = instructions
            .into_iter()
            .enumerate()
            .map(|(at, instruction)| PlanNode {
                instruction,
                successors: vec![Target::Node(at + 1)],
            })
            .collect::<Vec<_>>();
        nodes.push(PlanNode {
            instruction: LegacyInstruction::NoOp,
            successors: vec![Target::True, Target::False],
        });
        Ok(ConditionPlan { nodes, entry: 0 })
    }
}

impl Position {
    /// The refusal the position's lowering pushes (recorded beside the
    /// elaborator's answer).
    fn refusal(self) -> &'static str {
        match self {
            Position::Branch => "unmodeled call, aggregate or indirect access in branch condition",
            Position::Loop => "unmodeled call, aggregate or indirect access in loop condition",
            Position::ForLoop => {
                "unmodeled call, aggregate or indirect access in for-loop condition"
            }
            Position::DoWhile => {
                "unmodeled call, aggregate or indirect access in do-while condition"
            }
            Position::Switch => "unmodeled call, aggregate or indirect access in switch expression",
        }
    }
}

/// Whether a lowered statement is a refusal (anywhere inside a block).
fn refuses(statement: &Statement) -> bool {
    match statement {
        Statement::Instruction(LegacyInstruction::Unknown { .. }, _) => true,
        Statement::Block(statements) => statements.iter().any(refuses),
        _ => false,
    }
}
