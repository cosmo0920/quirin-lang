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

//! Card **CG17-LAYOUT-DERIVED-EXTENTS** (gateway rule 11u): the route-G
//! adapter of [`crate::layout_extents`]. Every hook is asked last, only for a
//! statement or `return` every older route refused, matched to one Clang node
//! by line and exact tokens; every consultation is a `QCORE_TYPED_PATH_PARITY`
//! row (`position: "layout-extents-…"`).
//!
//! * **A VLA declaration** whose decision is `admitted` (a builtin scalar
//!   element, a bound that only reads, every use an element access or a
//!   `sizeof` operand) is lowered to no custody event (a `ForgetScalar` of
//!   its name, a new object each time it runs): its bound's reads are
//!   scalar reads of named objects, its elements hold no identity, and no
//!   address of it leaves the block, so its lifetime's end at the block's exit
//!   ends nothing the flow tracks. Its three numeric obligations are recorded
//!   (parity), never discharged. Every other decision keeps the refusal.
//! * **A `container_of` value** in a declaration, an assignment or a `return`
//!   whose pointer operand is established as pointing at the member the
//!   offset names, of an object of the record the result points to, is the
//!   enclosing object's pointer `x` itself: the statement is lowered as the
//!   respelled `… = x;` / `return x;` by the older routes, and only if they
//!   admit it. Every other pointer keeps the refusal.
//! * **The census** (parity runs only): every VLA, flexible-array access,
//!   `container_of` and derived-address conversion of a function, with its
//!   decision.

use serde_json::{Value, json};

use super::{LegacyInstruction, Parser, Statement, Token, is_identifier};
use crate::elaboration::strip_parens;
use crate::layout_extents::{
    self, ContainerDecision, FlexExtent, FlexRefusal, VlaDecision, container_of, container_shape,
    member_provenance, obligations_of, vla_declaration,
};
use crate::typed_access_path::{StatementIndex, record_row};
use crate::typed_c_ast::{children, kind};

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

/// The value of a store node: a one-declarator declaration's initializer or
/// an assignment's right operand.
fn store_value(node: &Value) -> Option<&Value> {
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
            Some(initializer)
        }
        "BinaryOperator" if bare["opcode"].as_str() == Some("=") => children(bare).get(1),
        _ => None,
    }
}

impl Parser<'_> {
    fn layout_index(&self) -> Option<StatementIndex> {
        self.typed_positions_index().cloned()
    }

    fn record_layout(&self, position: &str, line: usize, fields: Value) {
        let mut row = json!({
            "function": self.typed_path_function,
            "line": line,
            "position": format!("layout-extents-{position}"),
        });
        if let (Some(row), Some(fields)) = (row.as_object_mut(), fields.as_object()) {
            row.extend(fields.clone());
        }
        record_row(row);
    }

    /// Hook (M12): a refused declaration of one variable-length array.
    pub(super) fn vla_declaration_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        if !matches!(instruction, LegacyInstruction::Unknown { .. })
            || !tokens.iter().any(|token| token.text == "[")
        {
            return None;
        }
        let index = self.layout_index()?;
        let statement_texts = texts(tokens);
        let node = Self::elaboration_node(&index, tokens[0].line, &statement_texts, &|node| {
            kind(node) == "DeclStmt"
        })?;
        let fact = vla_declaration(node, &index.function)?;
        self.record_layout(
            "vla",
            line,
            json!({
                "name": fact.name,
                "element": fact.element,
                "bound": fact.bound,
                "bound_class": format!("{:?}", fact.bound_class),
                "uses": fact.uses.iter().map(|u| format!("{u:?}")).collect::<Vec<_>>(),
                "decision": fact.decision.code(),
                "obligations": obligations_of(fact.decision)
                    .iter()
                    .map(|o| o.code())
                    .collect::<Vec<_>>(),
            }),
        );
        // A new object each time the declaration runs: any fact the flow kept
        // under its name is forgotten, as a scalar write's is.
        (fact.decision == VlaDecision::Admitted).then(|| {
            Statement::Instruction(
                LegacyInstruction::ForgetScalar {
                    destination: fact.name.clone(),
                },
                line,
            )
        })
    }

    /// The respelling of a `container_of` value as its enclosing object, or
    /// `None` with the decision recorded.
    fn container_base(&self, value: &Value, index: &StatementIndex, line: usize) -> Option<String> {
        let shape = container_shape(value, &|node| index.node_tokens(node))?;
        let provenance = member_provenance(shape.pointer, &index.function);
        // The offset's record must be the result's (Clang's spelling of both).
        let decision = if shape.offset_record != shape.target {
            ContainerDecision::RecordMismatch
        } else {
            container_of(&provenance, &shape.target, &shape.offset_path)
        };
        let reason = match &provenance {
            layout_extents::Provenance::Unestablished(reason) => *reason,
            layout_extents::Provenance::MemberOf { .. } => "",
        };
        self.record_layout(
            "container-of",
            line,
            json!({
                "target": shape.target,
                "offset_record": shape.offset_record,
                "offset_path": shape.offset_path,
                "decision": decision.code(),
                "reason": reason,
            }),
        );
        match decision {
            ContainerDecision::Enclosing((_, name)) if is_identifier(&name) => Some(name),
            _ => None,
        }
    }

    /// Hook (E18): a refused declaration or assignment whose whole value is
    /// a `container_of` expression.
    pub(super) fn container_of_statement(
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
        if !statement_texts[equal + 1..].contains(&"-") {
            return None;
        }
        let index = self.layout_index()?;
        let node = Self::elaboration_node(&index, tokens[0].line, &statement_texts, &|node| {
            store_value(node).is_some()
        })?;
        let value = store_value(node)?;
        let wanted = index.node_tokens(value)?;
        // The value is everything after the statement's `=`.
        if !statement_texts[equal + 1..]
            .iter()
            .copied()
            .eq(wanted.iter().map(String::as_str))
        {
            return None;
        }
        let base = self.container_base(value, &index, line)?;
        let mut synthetic = tokens[..=equal].to_vec();
        synthetic.push(Token {
            text: base,
            line: tokens[equal].line,
        });
        let saved = self.probe_state();
        let lowered = self.expression_statement(&synthetic, line, None);
        if refuses(&lowered) {
            self.restore_probe_state(&saved);
            self.record_layout(
                "container-of-statement",
                line,
                json!({"answer": "respelled-refused"}),
            );
            return None;
        }
        self.record_layout(
            "container-of-statement",
            line,
            json!({"answer": "respelled-admitted"}),
        );
        Some(lowered)
    }

    /// Hook (E18): a refused `return` whose whole value is a `container_of`
    /// expression.
    pub(super) fn container_of_return(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        let first = expression.first()?;
        if !expression.iter().any(|token| token.text == "-") {
            return None;
        }
        let index = self.layout_index()?;
        let mut wanted = vec!["return"];
        wanted.extend(texts(expression));
        let statement = Self::elaboration_node(&index, first.line, &wanted, &|node| {
            kind(node) == "ReturnStmt"
        })?;
        let [value] = children(statement) else {
            return None;
        };
        let base = self.container_base(value, &index, line)?;
        let respelled = vec![Token {
            text: base,
            line: first.line,
        }];
        let saved = self.probe_state();
        let lowered = self.return_statement(respelled, line);
        if super::return_refused(&lowered) || refuses(&lowered) {
            self.restore_probe_state(&saved);
            self.record_layout(
                "container-of-return",
                line,
                json!({"answer": "respelled-refused"}),
            );
            return None;
        }
        self.record_layout(
            "container-of-return",
            line,
            json!({"answer": "respelled-admitted"}),
        );
        Some(lowered)
    }

    /// Parity runs only: the function's layout census. Clang's numbers are
    /// asked once per function, only where a derived-address conversion or a
    /// flexible-array access needs them.
    pub(super) fn record_layout_census(&self) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let Some(index) = self.layout_index() else {
            return;
        };
        let function = &index.function;
        let line_of = |begin: Option<usize>| {
            begin.map_or(0, |offset| {
                index.source[..offset.min(index.source.len())]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    + 1
            })
        };
        for fact in layout_extents::vla_facts(function) {
            self.record_layout(
                "census",
                0,
                json!({
                    "form": "vla",
                    "name": fact.name,
                    "bound": fact.bound,
                    "decision": fact.decision.code(),
                }),
            );
        }
        let casts = layout_extents::derived_casts(function);
        let flexible = layout_extents::flexible_accesses(function);
        let mut queries = casts
            .iter()
            .flat_map(layout_extents::cast_queries)
            .collect::<Vec<_>>();
        for access in &flexible {
            queries.push(format!(
                "__builtin_offsetof({}, {})",
                access.record, access.member
            ));
            queries.push(format!("sizeof({})", access.element));
        }
        queries.sort();
        queries.dedup();
        let numbers = if queries.is_empty() {
            Default::default()
        } else {
            crate::typed_access_path::analysed_unit_constants(&queries)
        };
        let lookup = |query: &str| numbers.get(query).copied();
        for cast in &casts {
            let (required, base, offset, decision) = layout_extents::decide_cast(cast, &lookup);
            self.record_layout(
                "census",
                line_of(cast.begin),
                json!({
                    "form": "derived-address",
                    "target": cast.target,
                    "base": format!("{:?}", cast.base),
                    "required": required,
                    "base_alignment": base,
                    "offset": format!("{offset:?}"),
                    "decision": decision.code(),
                }),
            );
        }
        for access in &flexible {
            let offset = lookup(&format!(
                "__builtin_offsetof({}, {})",
                access.record, access.member
            ));
            let width = lookup(&format!("sizeof({})", access.element));
            let extent = match (offset, width) {
                (Some(o), Some(w)) => layout_extents::flexible_extent(
                    None,
                    u128::try_from(o).unwrap_or(0),
                    u128::try_from(w).unwrap_or(0),
                ),
                _ => FlexExtent::Refused(FlexRefusal::SizeNotEstablished),
            };
            self.record_layout(
                "census",
                line_of(access.begin),
                json!({
                    "form": "flexible-array",
                    "record": access.record,
                    "member": access.member,
                    "offset": offset,
                    "width": width,
                    "extent": format!("{extent:?}"),
                }),
            );
        }
        let mut containers = Vec::new();
        collect_containers(function, &index, &mut containers);
        for (begin, decision) in containers {
            self.record_layout(
                "census",
                line_of(begin),
                json!({"form": "container-of", "decision": decision}),
            );
        }
        let mut offsets = 0usize;
        count_offsetof(function, &mut offsets);
        if offsets > 0 {
            self.record_layout("census", 0, json!({"form": "offsetof", "count": offsets}));
        }
    }
}

fn collect_containers(
    node: &Value,
    index: &StatementIndex,
    out: &mut Vec<(Option<usize>, String)>,
) {
    if let Some(shape) = container_shape(node, &|n| index.node_tokens(n)) {
        let provenance = member_provenance(shape.pointer, &index.function);
        let decision = if shape.offset_record != shape.target {
            ContainerDecision::RecordMismatch
        } else {
            container_of(&provenance, &shape.target, &shape.offset_path)
        };
        let code = match (&decision, &provenance) {
            (ContainerDecision::Unestablished(_), layout_extents::Provenance::Unestablished(r)) => {
                format!("unestablished:{r}")
            }
            _ => decision.code().to_owned(),
        };
        out.push((
            crate::typed_access_path::offset(&node["range"]["begin"]),
            code,
        ));
        return;
    }
    for child in children(node) {
        collect_containers(child, index, out);
    }
}

fn count_offsetof(node: &Value, count: &mut usize) {
    if kind(node) == "OffsetOfExpr" {
        *count += 1;
    }
    for child in children(node) {
        count_offsetof(child, count);
    }
}
