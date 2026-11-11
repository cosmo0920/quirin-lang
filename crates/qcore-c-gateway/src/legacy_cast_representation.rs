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

//! Card **CG08-CASTS-AND-TYPE-QUERIES** (gateway rule 11b): the route-G
//! adapter of [`crate::cast_representation`]. The hooks in `legacy_c.rs`
//! naming the card are thin; each is matched to one Clang node by line and
//! exact tokens, and every consultation is a `QCORE_TYPED_PATH_PARITY` row
//! (`position: "cast-representation-…"`).
//!
//! * **The null test is no escape (soundness repair).** C converts a pointer
//!   returned from, or stored into, a `_Bool` with `PointerToBoolean`: the
//!   result is the null test, and no pointer reaches the caller or the
//!   destination. The older `return` route escaped the pointer (so `bool
//!   f(void) { void *p = alloc(); return p; }` was Safe though `p` leaks), and
//!   the older statement route bound the destination to the pointer's
//!   identity (`bool b = p;`). A `return`, declaration or assignment whose
//!   value Clang converts so is asked of the elaborator **before** the older
//!   routes (their premise, that the value is the pointer, is false): its
//!   events, then an exit or a scalar write that carries nothing. Where the
//!   elaborator refuses, the older answer stands only when the pointer's root
//!   is an automatic object the custody closure proves never bound (escaping
//!   or binding nothing changes nothing); otherwise the position refuses.
//! * **Pointer-to-pointer casts at a pointer destination** (E15). A refused
//!   `T *NAME = (U *) VALUE;` or `NAME = (U *) VALUE;` whose explicit casts
//!   are all object-pointer `BitCast`/`NoOp` conversions (the same address,
//!   C11 6.3.2.3p7) is lowered as the uncast statement `… = VALUE;` by the
//!   older routes, exactly as the W44 cast return decides `return (T *) p;`
//!   as `return p;`. The cast adds no admission of its own.
//! * **A cast return the reader does not commit to** (`CAST-RETURN-
//!   UNCOMMITTED-OPERAND`). `return (INTEGER) h;` whose operand the lexical
//!   declaration reader does not commit to as a pointer is refused when
//!   Clang's cast kind is `PointerToIntegral` and `h` may carry an identity.
//! * **Unevaluated `sizeof` operands at the lexical gate** (E16,
//!   `W44-SIZEOF-OPERAND-RESIDUALS`). In a statement whose first call is an
//!   ordinary callee, a parenthesised `sizeof ( … )` whose every Clang node
//!   on its line is a query with an unevaluated operand joins W44's operand
//!   spans: the gate sees the marker, never a value (constraint 5a).
//! * **Type queries in conditions.** A parenthesised type-query operand is
//!   no call to the elaborator's call-free premise
//!   ([`calls_outside_type_queries`]); the elaborator decides the operand.

use serde_json::{Value, json};

use super::{LegacyInstruction, Parser, Statement, Token, is_identifier};
use crate::elaboration::{pointer_operand, strip_parens};
use crate::typed_access_path::{StatementIndex, is_pointer, record_row};
use crate::typed_c_ast::{children, kind};

/// The refusal of a `return` whose pointer value C converts to a truth value
/// and the elaborator does not read.
pub(crate) const POINTER_TEST_RETURN_REFUSAL: &str =
    "pointer converted to a truth value in return expression is not modeled (CG08)";

/// The refusal of a store whose pointer value C converts to a truth value and
/// the elaborator does not read.
pub(crate) const POINTER_TEST_STORE_REFUSAL: &str =
    "pointer converted to a truth value in a store is not modeled (CG08)";

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

/// Whether a value node is, at its top, C's pointer-to-truth-value
/// conversion (implicit or explicit), with its pointer operand.
fn pointer_test(value: &Value) -> Option<&Value> {
    let bare = strip_parens(value);
    (matches!(kind(bare), "ImplicitCastExpr" | "CStyleCastExpr")
        && bare["castKind"].as_str() == Some("PointerToBoolean"))
    .then(|| children(bare).first())
    .flatten()
}

/// The root object of a pointer value: through parentheses, implicit
/// conversions, member selection, subscripts and dereferences.
fn root(mut node: &Value) -> Option<&Value> {
    loop {
        node = strip_parens(node);
        match kind(node) {
            "ImplicitCastExpr" | "MemberExpr" | "ArraySubscriptExpr" => {
                node = children(node).first()?;
            }
            "UnaryOperator" if node["opcode"].as_str() == Some("*") => {
                node = children(node).first()?;
            }
            "DeclRefExpr" => return Some(node),
            _ => return None,
        }
    }
}

fn refuses(statement: &Statement) -> bool {
    match statement {
        Statement::Instruction(LegacyInstruction::Unknown { .. }, _) => true,
        Statement::Block(statements) => statements.iter().any(refuses),
        Statement::Return {
            returned_evaluation: Some(evaluation),
            ..
        } => matches!(**evaluation, LegacyInstruction::Unknown { .. }),
        _ => false,
    }
}

/// The value of a store node: a one-declarator declaration's initializer or
/// an assignment's right operand, with the destination's declared type text.
fn store_value(node: &Value) -> Option<(&Value, &Value)> {
    let bare = strip_parens(node);
    match kind(bare) {
        "DeclStmt" => {
            let [declaration] = children(bare) else {
                return None;
            };
            if kind(declaration) != "VarDecl" {
                return None;
            }
            let [initializer] = children(declaration) else {
                return None;
            };
            Some((declaration, initializer))
        }
        "BinaryOperator" if bare["opcode"].as_str() == Some("=") => {
            let [left, right] = children(bare) else {
                return None;
            };
            Some((left, right))
        }
        _ => None,
    }
}

impl Parser<'_> {
    fn cast_index(&self) -> Option<StatementIndex> {
        self.typed_positions_index().cloned()
    }

    fn record_cast(&self, position: &str, line: usize, tokens: &[Token], answer: &str) {
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts(tokens),
            "position": format!("cast-representation-{position}"),
            "answer": answer,
        }));
    }

    /// Whether a pointer value's root is an automatic object the custody
    /// closure proves never bound (`never_bound`), so escaping or binding it
    /// changes nothing.
    fn root_never_bound(&self, pointer: &Value, never_bound: &dyn Fn(&str) -> bool) -> bool {
        let Some(root) = root(pointer) else {
            return false;
        };
        let reference = &root["referencedDecl"];
        matches!(reference["kind"].as_str(), Some("VarDecl" | "ParmVarDecl"))
            && reference["name"].as_str().is_some_and(never_bound)
            && self.index_automatic(reference["id"].as_str().unwrap_or_default())
    }

    fn index_automatic(&self, id: &str) -> bool {
        let Some(index) = self.cast_index() else {
            return false;
        };
        let (automatic, _) = super::elaboration_route::aliasing(&index.function);
        automatic.contains(id)
    }

    /// Hook (soundness repair): a `return` whose value C converts from a
    /// pointer to a truth value. `None` leaves the older routes' answer.
    pub(super) fn pointer_test_return(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        if !expression.iter().any(|token| is_identifier(&token.text)) {
            return None;
        }
        let index = self.cast_index()?;
        let mut wanted = vec!["return"];
        wanted.extend(texts(expression));
        let statement = Self::elaboration_node(&index, expression[0].line, &wanted, &|node| {
            kind(node) == "ReturnStmt"
        })?;
        let [value] = children(statement) else {
            return None;
        };
        let pointer = pointer_test(value)?;
        if let Some(elaborated) = self.elaborated_return(expression, line) {
            self.record_cast("return", line, expression, "null-test-elaborated");
            return Some(elaborated);
        }
        if self.root_never_bound(pointer, &|name| self.position_never_bound(name)) {
            self.record_cast(
                "return",
                line,
                expression,
                "null-test-untracked-older-route",
            );
            return None;
        }
        self.record_cast("return", line, expression, "null-test-refused");
        let writes = self.expression_writes(expression);
        Some(Statement::Return {
            line,
            writes: writes.clone(),
            expression_supported: true,
            returned_place: None,
            returned_evaluation: Some(Box::new(LegacyInstruction::Unknown {
                description: POINTER_TEST_RETURN_REFUSAL.to_owned(),
                writes,
            })),
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        })
    }

    /// Hook (soundness repair): a declaration or assignment whose value C
    /// converts from a pointer to a truth value, asked before the older
    /// routes. `None` leaves them the statement.
    pub(super) fn pointer_test_store(
        &mut self,
        tokens: &[Token],
        line: usize,
    ) -> Option<Statement> {
        let statement_texts = texts(tokens);
        if !statement_texts.contains(&"=") {
            return None;
        }
        let index = self.cast_index()?;
        let node = Self::elaboration_node(&index, tokens[0].line, &statement_texts, &|node| {
            store_value(node).is_some()
        })?;
        let (_, value) = store_value(node)?;
        let pointer = pointer_test(value)?;
        if let Some(elaborated) =
            self.elaborated_statement(tokens, line, POINTER_TEST_STORE_REFUSAL)
        {
            self.record_cast("store", line, tokens, "null-test-elaborated");
            return Some(elaborated);
        }
        if self.root_never_bound(pointer, &|name| self.joined_never_bound(name)) {
            self.record_cast("store", line, tokens, "null-test-untracked-older-route");
            return None;
        }
        self.record_cast("store", line, tokens, "null-test-refused");
        Some(Statement::Instruction(
            LegacyInstruction::Unknown {
                description: POINTER_TEST_STORE_REFUSAL.to_owned(),
                writes: self.refused_writes(&statement_texts),
            },
            line,
        ))
    }

    /// Hook (E15): a refused statement storing an explicit object-pointer
    /// cast into a pointer destination, lowered as the uncast statement.
    pub(super) fn uncast_pointer_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        if !matches!(instruction, LegacyInstruction::Unknown { .. }) {
            return None;
        }
        let statement_texts = texts(tokens);
        let equal = statement_texts.iter().position(|text| *text == "=")?;
        if !statement_texts[equal + 1..].contains(&"(") {
            return None;
        }
        let index = self.cast_index()?;
        let node = Self::elaboration_node(&index, tokens[0].line, &statement_texts, &|node| {
            store_value(node).is_some()
        })?;
        let (destination, value) = store_value(node)?;
        if !is_pointer(destination) {
            return None;
        }
        let operand = uncast_operand(value)?;
        let wanted = index.node_tokens(operand)?;
        let value_tokens = run_of(tokens, equal + 1, &wanted)?;
        let mut synthetic = tokens[..=equal].to_vec();
        synthetic.extend(value_tokens);
        let saved = self.probe_state();
        let lowered = self.expression_statement(&synthetic, line, None);
        if refuses(&lowered) {
            self.restore_probe_state(&saved);
            self.record_cast("pointer-statement", line, tokens, "uncast-refused");
            return None;
        }
        self.record_cast("pointer-statement", line, tokens, "uncast-admitted");
        Some(lowered)
    }

    /// Hook (`CAST-RETURN-UNCOMMITTED-OPERAND`): whether a `return` the W44
    /// cast-return rule leaves to the pre-card parse converts a possibly
    /// tracked pointer to an integer, by Clang's cast kind.
    pub(super) fn pointer_integer_return(&self, expression: &[Token]) -> bool {
        let Some(first) = expression.first() else {
            return false;
        };
        let Some(index) = self.cast_index() else {
            return false;
        };
        let mut wanted = vec!["return"];
        wanted.extend(texts(expression));
        let Some(statement) = Self::elaboration_node(&index, first.line, &wanted, &|node| {
            kind(node) == "ReturnStmt"
        }) else {
            return false;
        };
        let [value] = children(statement) else {
            return false;
        };
        let mut node = strip_parens(value);
        while matches!(kind(node), "CStyleCastExpr" | "ImplicitCastExpr") {
            let Some(inner) = children(node).first() else {
                return false;
            };
            if node["castKind"].as_str() == Some("PointerToIntegral") {
                // Only a name: a cast of a call or an expression keeps the
                // older routes' answer (rule 5j reads a cast of a whole call).
                if kind(pointer_operand(inner)) != "DeclRefExpr" {
                    return false;
                }
                let refused =
                    !self.root_never_bound(inner, &|name| self.position_never_bound(name));
                self.record_cast(
                    "integer-return",
                    first.line,
                    expression,
                    if refused {
                        "pointer-to-integer-refused"
                    } else {
                        "pointer-to-integer-untracked"
                    },
                );
                return refused;
            }
            node = strip_parens(inner);
        }
        false
    }

    /// Parity runs only (`W45-CALLEE-CAST-LEXICAL-PREMISE`): cross-check the
    /// lexical cast/call reading of `( NAME ) (` against Clang's nodes. For
    /// every `CallExpr` whose callee is a parenthesised name and every
    /// `CStyleCastExpr` whose written type is one identifier over a
    /// parenthesised operand, one row: Clang's answer, and whether the
    /// function's normalised tokens on that line spell the call (`NAME (`,
    /// W45's rewrite) or keep the group (`( NAME ) (`, read as a cast).
    pub(super) fn record_callee_cast_census(&self, body_start: usize) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let Some(body_end) = super::matching(self.tokens, body_start, "{", "}") else {
            return;
        };
        let Some(index) = self.cast_index() else {
            return;
        };
        let body = &self.tokens[body_start..=body_end];
        let spelled = |line: usize, name: &str, group: bool| {
            let on_line = body
                .iter()
                .filter(|token| token.line == line)
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            if group {
                on_line
                    .windows(4)
                    .any(|window| window == ["(", name, ")", "("])
            } else {
                on_line
                    .windows(3)
                    .any(|window| window[1] == name && window[2] == "(" && window[0] != "(")
            }
        };
        let mut sites = Vec::new();
        collect_callee_cast_sites(&index, &index.function, &mut sites);
        for (line, name, clang) in sites {
            let as_call = spelled(line, &name, false);
            let as_group = spelled(line, &name, true);
            let lexical = match (as_call, as_group) {
                (true, false) => "call",
                (false, true) => "cast",
                (true, true) => "both",
                (false, false) => "absent",
            };
            record_row(json!({
                "function": self.typed_path_function,
                "line": line,
                "position": "cast-representation-callee-census",
                "name": name,
                "clang": clang,
                "lexical": lexical,
                "agree": lexical == clang,
            }));
        }
    }

    /// The `[start, end)` spans of the parenthesised `sizeof ( … )` operands
    /// of `tokens` whose every Clang node on their line is a query with an
    /// unevaluated operand (E16), outside `known`.
    pub(super) fn clang_unevaluated_spans(
        &self,
        tokens: &[Token],
        known: &[(usize, usize)],
    ) -> Vec<(usize, usize)> {
        let statement_texts = texts(tokens);
        let mut spans = Vec::new();
        let mut index = None;
        let mut cursor = 0;
        while cursor + 1 < statement_texts.len() {
            if statement_texts[cursor] != "sizeof"
                || statement_texts[cursor + 1] != "("
                || known
                    .iter()
                    .any(|(start, end)| cursor >= *start && cursor < *end)
            {
                cursor += 1;
                continue;
            }
            let Some(close) = super::matching(tokens, cursor + 1, "(", ")") else {
                break;
            };
            if index.is_none() {
                index = Some(self.cast_index());
            }
            let Some(Some(index)) = &index else {
                break;
            };
            let wanted = &statement_texts[cursor..=close];
            if unevaluated_on_line(index, tokens[cursor].line, wanted) {
                spans.push((cursor, close + 1));
                cursor = close + 1;
            } else {
                cursor += 1;
            }
        }
        spans
    }
}

/// The `( NAME ) (` sites Clang decides: a call through a parenthesised name
/// (`"call"`), or a cast to a one-identifier type name over a parenthesised
/// operand (`"cast"`), each with its line.
fn collect_callee_cast_sites(
    index: &StatementIndex,
    node: &Value,
    out: &mut Vec<(usize, String, &'static str)>,
) {
    match kind(node) {
        "CallExpr" => {
            if let Some(callee) = children(node).first() {
                let mut cursor = callee;
                let mut parenthesised = false;
                loop {
                    match kind(cursor) {
                        "ImplicitCastExpr" => {}
                        "ParenExpr" => parenthesised = true,
                        _ => break,
                    }
                    match children(cursor).first() {
                        Some(inner) => cursor = inner,
                        None => break,
                    }
                }
                if parenthesised
                    && kind(cursor) == "DeclRefExpr"
                    && let (Some(name), Some(line)) = (
                        cursor["referencedDecl"]["name"].as_str(),
                        index.node_line(node),
                    )
                {
                    out.push((line, name.to_owned(), "call"));
                }
            }
        }
        "CStyleCastExpr" => {
            if let (Some(written), Some(operand), Some(line)) = (
                node["type"]["qualType"].as_str(),
                children(node).first(),
                index.node_line(node),
            ) && !written.is_empty()
                && written
                    .bytes()
                    .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
                && !written.as_bytes()[0].is_ascii_digit()
                && kind(operand) == "ParenExpr"
            {
                out.push((line, written.to_owned(), "cast"));
            }
        }
        _ => {}
    }
    for child in children(node) {
        collect_callee_cast_sites(index, child, out);
    }
}

/// Whether `tokens` hold a call outside every parenthesised type-query
/// operand. The operand of a query is decided by the elaborator (an
/// unevaluated one calls nothing; an evaluated one holding a call refuses),
/// so a condition whose only "calls" are type operators is still call-free.
pub(super) fn calls_outside_type_queries(tokens: &[Token]) -> bool {
    let mut kept = Vec::with_capacity(tokens.len());
    let mut cursor = 0;
    while cursor < tokens.len() {
        if super::TYPE_OPERATORS.contains(&tokens[cursor].text.as_str())
            && tokens.get(cursor + 1).is_some_and(|next| next.text == "(")
            && let Some(close) = super::matching(tokens, cursor + 1, "(", ")")
        {
            kept.push(Token {
                text: "0".to_owned(),
                line: tokens[cursor].line,
            });
            cursor = close + 1;
            continue;
        }
        kept.push(tokens[cursor].clone());
        cursor += 1;
    }
    super::contains_call(&kept)
}

/// The operand under a value's explicit object-pointer casts: at least one
/// `CStyleCastExpr`, each (and each implicit conversion between them) a
/// `BitCast` or `NoOp` from an object pointer to an object pointer.
fn uncast_operand(value: &Value) -> Option<&Value> {
    let mut node = strip_parens(value);
    let mut explicit = false;
    while matches!(kind(node), "CStyleCastExpr" | "ImplicitCastExpr") {
        if !matches!(node["castKind"].as_str(), Some("BitCast" | "NoOp")) {
            break;
        }
        // Card CAST-FORMALIZATION (rule 12h): the uncast statement denotes
        // the same object only for a class that keeps the identity.
        if !crate::cast_semantics::classify_node(node)
            .is_some_and(crate::cast_semantics::ConversionClass::preserves_identity)
        {
            return None;
        }
        let inner = children(node).first()?;
        if !is_pointer(node) || !is_pointer(inner) {
            return None;
        }
        explicit |= kind(node) == "CStyleCastExpr";
        node = strip_parens(inner);
    }
    // An implicit read or conversion of the operand stays with it.
    explicit.then_some(node)
}

/// The run of `wanted` in `tokens` from `from` to the end: the value's own
/// tokens, which must be all that follows the `=`.
fn run_of(tokens: &[Token], from: usize, wanted: &[String]) -> Option<Vec<Token>> {
    let rest = tokens.get(from..)?;
    let start = rest.len().checked_sub(wanted.len())?;
    let run = &rest[start..];
    // Everything before the run is the cast groups and parentheses.
    if !run
        .iter()
        .zip(wanted)
        .all(|(token, text)| &token.text == text)
    {
        return None;
    }
    let before = &rest[..start];
    if !before.iter().all(|token| {
        matches!(token.text.as_str(), "(" | ")" | "*" | "const") || is_identifier(&token.text)
    }) {
        return None;
    }
    // Every `(` opened before the value is closed before it.
    let depth = before
        .iter()
        .fold(0i32, |depth, token| match token.text.as_str() {
            "(" => depth + 1,
            ")" => depth - 1,
            _ => depth,
        });
    (depth == 0).then(|| run.to_vec())
}

/// Whether at least one Clang `UnaryExprOrTypeTraitExpr` on `line` has
/// exactly the tokens `wanted`, and every one that does has an unevaluated
/// operand.
fn unevaluated_on_line(index: &StatementIndex, line: usize, wanted: &[&str]) -> bool {
    fn walk<'n>(
        index: &StatementIndex,
        node: &'n Value,
        line: usize,
        wanted: &[&str],
        out: &mut Vec<&'n Value>,
    ) {
        if index.node_line(node).is_some_and(|begin| begin > line)
            || index.node_end_line(node).is_some_and(|end| end < line)
        {
            return;
        }
        if kind(node) == "UnaryExprOrTypeTraitExpr"
            && index.node_line(node) == Some(line)
            && index
                .node_tokens(node)
                .is_some_and(|found| found.iter().map(String::as_str).eq(wanted.iter().copied()))
        {
            out.push(node);
        }
        for child in children(node) {
            walk(index, child, line, wanted, out);
        }
    }
    let mut found = Vec::new();
    walk(index, &index.function, line, wanted, &mut found);
    !found.is_empty()
        && found.iter().all(|node| {
            crate::cast_representation::query_operand(node)
                == crate::cast_representation::QueryOperand::Unevaluated
        })
}
