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

//! Card **CAST-FORMALIZATION** (gateway rule 12h): the route-G adapter of
//! [`crate::cast_semantics`]. The hooks in `legacy_c.rs` naming the card are
//! thin; each consults the one conversion table and records its decision as
//! a `QCORE_TYPED_PATH_PARITY` row (`position: "cast-semantics-…"`).
//!
//! * **Census** (parity runs only, no verdict effect): every explicit cast of
//!   a function body, with its Clang kind, conversion class, position and the
//!   line of its enclosing statement (`position: "cast-semantics-census"`),
//!   which the card's census joins to the route's leaves.
//! * **H-U, value positions** (asked last in the statement lowering, only for
//!   a refusal every older route left standing). A cast whose value is used
//!   as a value (a call actual, a variadic tail actual, a pointer-step
//!   operand, an initializer or an assignment value) and whose class keeps
//!   its operand's identity (`byte-view`, `typed-view`, `punned-view`,
//!   `qualifier`, [`ConversionClass::preserves_identity`]) changes nothing a
//!   custody route reads: the uncast statement denotes the same objects. Such
//!   casts of a plain object name are removed and the uncast statement is
//!   lowered by the older routes, exactly as rule 11b(3) lowers a top-level
//!   pointer-destination cast, and kept only where they admit it. A `scalar`
//!   conversion of a declared scalar that carries no identity (not one of
//!   the function's descriptor names) is removed too, but only as a variadic
//!   tail actual: a tail actual is data the callee reads and no route derives
//!   a value fact from (rule 11w), whereas elsewhere a narrowing conversion's
//!   value would differ from its operand's (CG20). A cast at an access
//!   position (`((T *)p)->f`, `*(T *)p`, `((T *)p)[i]`) is never removed: its
//!   type is the access's (H-V, the typed access-path reader's view). The
//!   uncast statement matches no Clang node, so a declared variadic tail's
//!   widening (rule 11w's T7/T8 statement fallback) is applied here with the
//!   original statement's Clang node for the promoted-type check.

use serde_json::{Value, json};

use super::{LegacyInstruction, Parser, Statement, Token};
use crate::cast_semantics::{CastSite, ConversionClass, Position, explicit_casts};
use crate::typed_access_path::{qual, record_row};
use crate::typed_c_ast::{children, kind};

/// The tail refusal's spelling (rule 11w, `legacy_variadics.rs`).
const TAIL_PREFIX: &str = "arguments beyond the declared parameters of `";
const TAIL_SUFFIX: &str = "` are outside the resource projection";

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
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

/// The plain object name a cast converts: its operand through parentheses
/// and the implicit read, a `DeclRefExpr` of a variable or formal.
fn named_operand(cast: &Value) -> Option<&Value> {
    let operand = crate::typed_access_path::strip(children(cast).first()?)?;
    (kind(operand) == "DeclRefExpr"
        && matches!(
            operand["referencedDecl"]["kind"].as_str(),
            Some("VarDecl" | "ParmVarDecl")
        ))
    .then_some(operand)
}

/// The place a cast converts, for the soundness repair: a plain object name
/// or a member chain (`a->b.c`) over one, through parentheses and the
/// implicit read.
fn place_operand(cast: &Value) -> Option<&Value> {
    let operand = crate::typed_access_path::strip(children(cast).first()?)?;
    let mut cursor = operand;
    loop {
        match kind(cursor) {
            "MemberExpr" => cursor = crate::typed_access_path::strip(children(cursor).first()?)?,
            "DeclRefExpr" => {
                return matches!(
                    cursor["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                )
                .then_some(operand);
            }
            _ => return None,
        }
    }
}

/// Whether the soundness repair removes `site`: an identity-preserving
/// pointer conversion of a place at a call actual or a variadic tail actual.
fn hides_an_identity(site: &CastSite<'_>) -> bool {
    matches!(
        site.class,
        ConversionClass::ByteView
            | ConversionClass::TypedView
            | ConversionClass::PunnedView
            | ConversionClass::Qualifier
    ) && matches!(site.position, Position::CallActual | Position::VariadicTail)
        && place_operand(site.node).is_some()
        && !crate::typed_access_path::qual(site.node).is_some_and(|text| {
            let (_, qualifiers) = crate::cast_semantics::type_class(text);
            qualifiers.volatile || qualifiers.atomic
        })
}

/// Whether H-U removes `site`: the table's class at a value position, over a
/// plain object name (`descriptor` names the function's integer-carried
/// identities, whose scalar conversion would hide one).
pub(crate) fn removable(site: &CastSite<'_>, descriptor: &dyn Fn(&str) -> bool) -> bool {
    let Some(operand) = named_operand(site.node) else {
        return false;
    };
    let value_position = matches!(
        site.position,
        Position::CallActual
            | Position::VariadicTail
            | Position::PointerStep
            | Position::Initializer
            | Position::AssignmentValue
    );
    match site.class {
        ConversionClass::ByteView
        | ConversionClass::TypedView
        | ConversionClass::PunnedView
        | ConversionClass::Qualifier => {
            debug_assert!(site.class.preserves_identity());
            value_position
                && !crate::typed_access_path::qual(site.node).is_some_and(|text| {
                    let (_, qualifiers) = crate::cast_semantics::type_class(text);
                    qualifiers.volatile || qualifiers.atomic
                })
        }
        ConversionClass::Scalar => {
            site.position == Position::VariadicTail
                && qual(operand).is_some_and(|text| {
                    crate::cast_semantics::type_class(text).0
                        == crate::cast_semantics::TypeClass::Scalar
                })
                && operand["referencedDecl"]["name"]
                    .as_str()
                    .is_some_and(|name| !descriptor(name))
        }
        _ => false,
    }
}

/// The statement's tokens with every removable cast replaced by its operand's
/// tokens, matched to Clang's casts in source order (every cast, removable or
/// not, advances the match, so two casts with one spelling are told apart by
/// position). `None` when no cast is removable or a cast's tokens are not
/// found in order.
fn uncast_tokens(
    index: &crate::typed_access_path::StatementIndex,
    statement: &Value,
    tokens: &[Token],
    removable: &dyn Fn(&CastSite<'_>) -> bool,
) -> Option<(Vec<Token>, Vec<&'static str>)> {
    let mut out = tokens.to_vec();
    let mut cursor = 0usize;
    let mut removed = Vec::new();
    for site in explicit_casts(statement) {
        let run = index.node_tokens(site.node)?;
        let start = (cursor..out.len()).find(|&at| {
            out.len() >= at + run.len()
                && out[at..at + run.len()]
                    .iter()
                    .zip(&run)
                    .all(|(token, text)| &token.text == text)
        })?;
        if !removable(&site) {
            cursor = start + 1;
            continue;
        }
        let operand = index.node_tokens(children(site.node).first()?)?;
        let keep = run.len().checked_sub(operand.len())?;
        if !run[keep..].iter().eq(operand.iter()) {
            return None;
        }
        let replacement = out[start + keep..start + run.len()].to_vec();
        out.splice(start..start + run.len(), replacement.iter().cloned());
        cursor = start + replacement.len();
        removed.push(site.class.name());
    }
    (!removed.is_empty()).then_some((out, removed))
}

/// The call to `callee` a statement node is, or holds as its initializer or
/// assigned value (rule 11w's `statement_call`).
fn statement_call<'n>(node: &'n Value, callee: &str) -> Option<&'n Value> {
    let strip = |mut node: &'n Value| {
        while matches!(kind(node), "ParenExpr" | "ImplicitCastExpr") {
            match children(node).first() {
                Some(inner) => node = inner,
                None => break,
            }
        }
        node
    };
    let bare = strip(node);
    let call = match kind(bare) {
        "CallExpr" => bare,
        "DeclStmt" => {
            let [declaration] = children(bare) else {
                return None;
            };
            strip(children(declaration).first()?)
        }
        "BinaryOperator" if bare["opcode"].as_str() == Some("=") => strip(children(bare).get(1)?),
        _ => return None,
    };
    let function = children(call).first().map(strip)?;
    (kind(call) == "CallExpr" && function["referencedDecl"]["name"].as_str() == Some(callee))
        .then_some(call)
}

impl Parser<'_> {
    /// Parity runs only: the function's explicit-cast census.
    pub(super) fn record_cast_census(&self) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let Some(index) = self.typed_positions_index().cloned() else {
            return;
        };
        for site in explicit_casts(&index.function) {
            let operand = children(site.node).first();
            record_row(json!({
                "function": self.typed_path_function,
                "line": index.node_line(site.node),
                "statement_line": index.node_line(site.statement),
                "position": "cast-semantics-census",
                "cast_kind": site.node["castKind"],
                "class": site.class.name(),
                "cast_position": site.position.name(),
                "from": operand.and_then(qual),
                "to": qual(site.node),
                "texts": index.node_tokens(site.node).map(|mut tokens| {
                    tokens.truncate(24);
                    tokens
                }),
            }));
        }
    }

    fn record_uncast(&self, line: usize, tokens: &[Token], answer: &str, removed: &[&str]) {
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts(tokens),
            "position": "cast-semantics-value",
            "answer": answer,
            "removed": removed,
        }));
    }

    /// Hook H-U0 (rule 12h), a soundness repair asked first: the lexical
    /// classifier reads a call actual that is not a place as no place, so a
    /// `Borrowed` actual `(void *) p` was never checked against `p`'s state
    /// (`q_free((void *) t); q_observe((void *) t);` was `Safe`). An
    /// identity-preserving conversion of a place at a call actual is that
    /// place (`QCore.CastSemantics.view_preserves_identity`): a call the
    /// classifier admitted with such an actual is classified as its uncast
    /// statement instead, whose answer (an admission or a refusal) replaces
    /// the original's. `None` keeps the original classification.
    ///
    /// Card GAPO-RETURN-CORRECTNESS (rule 15x): the uncast statement is
    /// classified from `before`, the probe state the original classification
    /// started from, since its answer replaces the original's (before the
    /// card the original's sites stayed minted, leaving a gap).
    pub(super) fn uncast_call_actuals(
        &mut self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
        before: &super::probe_state::ParserProbeState,
    ) -> Option<LegacyInstruction> {
        if !matches!(instruction, LegacyInstruction::ContractCall { .. })
            || !tokens
                .windows(2)
                .any(|pair| pair[1].text == ")" && pair[0].text == "*")
        {
            return None;
        }
        let index = self.typed_positions_index().cloned()?;
        let wanted = texts(tokens);
        let statement = Self::elaboration_node(&index, tokens.first()?.line, &wanted, &|node| {
            matches!(kind(node), "DeclStmt" | "BinaryOperator" | "CallExpr")
        })?;
        let (synthetic, removed) =
            uncast_tokens(&index, statement, tokens, &|site| hides_an_identity(site))?;
        self.restore_probe_state(before);
        let repaired = self.classify_simple(&synthetic);
        let answer = if matches!(repaired, LegacyInstruction::ContractCall { .. }) {
            "uncast-actuals-admitted"
        } else {
            "uncast-actuals-refused"
        };
        self.record_uncast(tokens.first()?.line, tokens, answer, &removed);
        Some(repaired)
    }

    /// Card GAPO-RETURN-CORRECTNESS (rule 15z): hook H-U0 at the return
    /// position. `call` is the returned call as spelled (the whole
    /// expression, the call of a scalar operand, or the operand of a
    /// conversion) and `prefix` the synthetic destination its position
    /// classifies it under (W4's and W45-K's reserved place, or nothing for
    /// B01's scalar call). An admitted call with an identity-preserving cast
    /// of a place at an actual is classified again as its uncast spelling,
    /// from `before` (the probe state the original classification started
    /// from, rule 15x), and that answer replaces the original's, exactly as
    /// at a statement. `None` keeps the original (no such cast, or no unique
    /// Clang call node).
    pub(super) fn uncast_returned_call(
        &mut self,
        prefix: &[Token],
        call: &[Token],
        instruction: &LegacyInstruction,
        before: &super::probe_state::ParserProbeState,
    ) -> Option<LegacyInstruction> {
        if !matches!(instruction, LegacyInstruction::ContractCall { .. })
            || !call
                .windows(2)
                .any(|pair| pair[1].text == ")" && pair[0].text == "*")
        {
            return None;
        }
        let index = self.typed_positions_index().cloned()?;
        let wanted = texts(call);
        let node = Self::elaboration_node(&index, call.first()?.line, &wanted, &|node| {
            kind(node) == "CallExpr"
        })?;
        let (uncast, removed) = uncast_tokens(&index, node, call, &|site| hides_an_identity(site))?;
        let mut synthetic = prefix.to_vec();
        synthetic.extend(uncast);
        self.restore_probe_state(before);
        let repaired = self.classify_simple(&synthetic);
        let answer = if matches!(repaired, LegacyInstruction::ContractCall { .. }) {
            "uncast-return-actuals-admitted"
        } else {
            "uncast-return-actuals-refused"
        };
        self.record_uncast(call.first()?.line, call, answer, &removed);
        Some(repaired)
    }

    /// Card GAPO-RETURN-CORRECTNESS (rule 15z): whether the returned call
    /// `call` holds an identity-preserving cast of a place at an actual (the
    /// casts hook H-U0 removes), read from its Clang call node. Rule 12i's
    /// lowering, whose W44 re-classification hook H-U0 is not asked inside,
    /// declines such a call. Without a unique Clang node the answer is
    /// `true` when the spelling holds `* )`, so an unread cast declines too.
    pub(super) fn returned_call_hides_a_place(&self, call: &[Token]) -> bool {
        if !call
            .windows(2)
            .any(|pair| pair[1].text == ")" && pair[0].text == "*")
        {
            return false;
        }
        let Some(index) = self.typed_positions_index() else {
            return true;
        };
        let wanted = texts(call);
        let Some(line) = call.first().map(|token| token.line) else {
            return true;
        };
        let Some(node) =
            Self::elaboration_node(index, line, &wanted, &|node| kind(node) == "CallExpr")
        else {
            return true;
        };
        explicit_casts(node).iter().any(hides_an_identity)
    }

    /// Hook H-U (rule 12h): a statement every older route refused, lowered
    /// as its uncast statement where the table removes a cast at a value
    /// position. `None` keeps the refusal byte for byte.
    pub(super) fn uncast_value_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        if !matches!(instruction, LegacyInstruction::Unknown { .. })
            || !tokens.iter().any(|token| token.text == "(")
        {
            return None;
        }
        let index = self.typed_positions_index().cloned()?;
        let wanted = texts(tokens);
        let statement = Self::elaboration_node(&index, tokens.first()?.line, &wanted, &|node| {
            matches!(
                kind(node),
                "DeclStmt"
                    | "BinaryOperator"
                    | "CompoundAssignOperator"
                    | "CallExpr"
                    | "UnaryOperator"
                    | "CStyleCastExpr"
            )
        })?;
        let descriptors = &self.descriptor_names;
        let (synthetic, removed) = uncast_tokens(&index, statement, tokens, &|site| {
            removable(site, &|name| descriptors.contains(name))
        })?;
        let saved = self.probe_state();
        let lowered = self.expression_statement(&synthetic, line, None);
        if !refuses(&lowered) {
            self.record_uncast(line, tokens, "uncast-admitted", &removed);
            return Some(lowered);
        }
        self.restore_probe_state(&saved);
        if let Some(widened) = self.uncast_tail(statement, &synthetic, line) {
            self.record_uncast(line, tokens, "uncast-tail-admitted", &removed);
            return Some(widened);
        }
        self.restore_probe_state(&saved);
        self.record_uncast(line, tokens, "uncast-refused", &removed);
        None
    }

    /// Rule 11w's statement fallback (T7, T8) over the uncast statement:
    /// the callee's declared tail, the promoted type of every trailing actual
    /// read from the original statement's Clang node, and the classifier's
    /// widening over the uncast tokens.
    fn uncast_tail(
        &mut self,
        statement: &Value,
        synthetic: &[Token],
        line: usize,
    ) -> Option<Statement> {
        let LegacyInstruction::Unknown { description, .. } = self.classify_simple(synthetic) else {
            return None;
        };
        let callee = description
            .strip_prefix(TAIL_PREFIX)?
            .strip_suffix(TAIL_SUFFIX)?
            .to_owned();
        if !self.variadic_tails.contains(&callee) {
            return None;
        }
        let named = self.contracts.contract(&callee)?.parameters.len();
        let call = statement_call(statement, &callee)?;
        let promoted = children(call).get(1 + named..).is_some_and(|tail| {
            tail.iter()
                .all(crate::variadic_tail::promoted_scalar_or_pointer)
        });
        if !promoted {
            return None;
        }
        // Rule 12g's Clang readings of the original trailing arguments, for
        // the positions this hook did not uncast (a cast position's reading
        // refuses, and the uncast name is read by T8 instead).
        let borrowed = self.variadic_borrowed_tails.contains(&callee);
        let readings = self.tail_readings(call, named, borrowed);
        self.variadic_tail_widening.set(true);
        *self.variadic_tail_readings.borrow_mut() = Some(readings);
        let widened = self.classify_simple(synthetic);
        *self.variadic_tail_readings.borrow_mut() = None;
        self.variadic_tail_widening.set(false);
        if !matches!(widened, LegacyInstruction::ContractCall { .. }) {
            return None;
        }
        let synthetic_texts = texts(synthetic);
        let (call_index, _) = super::find_call(&synthetic_texts)?;
        let spans = super::call_argument_spans(&synthetic_texts, call_index)?;
        let borrowed_tail = self.variadic_borrowed_tails.contains(&callee);
        let addresses = spans
            .iter()
            .skip(named)
            .filter_map(|&(start, end)| {
                match crate::variadic_tail::widened_actual(
                    &synthetic_texts[start..end],
                    &self.typed_types,
                    &|name| self.descriptor_names.contains(name),
                    &self.protected_formals,
                    borrowed_tail,
                ) {
                    Some(crate::variadic_tail::WidenedTail::ScalarAddress(name)) => Some(name),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        let mut block = vec![Statement::Instruction(widened, line)];
        block.extend(addresses.into_iter().map(|destination| {
            Statement::Instruction(LegacyInstruction::ForgetScalar { destination }, line)
        }));
        Some(Statement::Block(block))
    }
}
