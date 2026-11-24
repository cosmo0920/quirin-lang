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

//! Card **LATER-DECLARATOR-INITIALISER-UNREAD** (gateway rule 74k): the
//! statement walker's hook for [`crate::later_declarator`].
//!
//! Asked for an expression or declaration statement before the statement's
//! own lowering. When the lexical reader splits the statement into at least
//! two init-declarators, the statement is exactly one Clang `DeclStmt` (one
//! unique node at its line with its exact tokens) whose children are the
//! split's `VarDecl`s by name and in order, and [`decide_all`] admits every
//! declarator, the statement is lowered as the block of its declarations
//! written one per statement: each initialised declarator through the
//! unchanged statement lowering of its own tokens (`specifiers declarator =
//! initialiser`), each uninitialised one as no event (rule 38h's reading of
//! it alone). Otherwise `None`: the old reading, refusals included.

use serde_json::{Value, json};

use super::{LegacyInstruction, Parser, Statement, Token};
use crate::later_declarator::{self, Facts, Lowering, decide_all};
use crate::typed_access_path::record_row;
use crate::typed_c_ast::{children, kind};

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

/// Whether `declaration` is an automatic object named `name`.
fn automatic_object(declaration: &Value, name: &str) -> bool {
    kind(declaration) == "VarDecl"
        && declaration["name"].as_str() == Some(name)
        && declaration.get("storageClass").is_none()
        && declaration.get("tls").is_none()
}

impl Parser<'_> {
    fn later_declarator_record(&self, line: usize, tokens: &[Token], answer: &str) {
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts(tokens),
            "position": "later-declarator",
            "rule": "74k",
            "answer": answer,
        }));
    }

    /// The hook. `None` keeps the statement on its old lowering.
    pub(super) fn later_declarator_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
    ) -> Option<Statement> {
        let statement_texts = texts(tokens);
        let split = later_declarator::split(&statement_texts)?;
        let index = self.typed_positions_index()?.clone();
        let node =
            Self::elaboration_node(&index, tokens.first()?.line, &statement_texts, &|node| {
                kind(node) == "DeclStmt"
            });
        let Some(node) = node else {
            self.later_declarator_record(line, tokens, "unmatched");
            return None;
        };
        let declarations = children(node);
        if declarations.len() != split.pieces.len() {
            self.later_declarator_record(line, tokens, "declarator-count");
            return None;
        }
        let facts = split
            .pieces
            .iter()
            .zip(declarations)
            .map(|(piece, declaration)| {
                let value = piece
                    .initialiser
                    .as_ref()
                    .map(|range| &statement_texts[range.clone()]);
                let ast_initialised = !children(declaration).is_empty();
                // Rule 38h's facts for the declaration written alone.
                let alone = json!({ "kind": "DeclStmt", "inner": [declaration] });
                Facts {
                    automatic_object: automatic_object(declaration, &piece.name)
                        && ast_initialised == value.is_some(),
                    initialised: value.is_some(),
                    evaluates_nothing: value.is_none()
                        && crate::typed_declaration::facts(&alone).all(),
                    pointer: piece.stars > 0,
                    null_constant: value.is_some_and(super::is_null_constant),
                    spells_a_cast: value.is_some_and(later_declarator::spells_a_cast),
                    numeric_literal: value.is_some_and(later_declarator::numeric_literal),
                }
            })
            .collect::<Vec<_>>();
        let Some(lowerings) = decide_all(&facts) else {
            self.later_declarator_record(line, tokens, "refused");
            return None;
        };
        self.later_declarator_record(line, tokens, "split");
        let pieces = split
            .statements()
            .into_iter()
            .zip(lowerings)
            .map(|(piece, lowering)| match lowering {
                Lowering::NoEvent => Statement::Instruction(LegacyInstruction::NoOp, line),
                Lowering::Alone => {
                    let piece = piece
                        .into_iter()
                        .map(|at| tokens[at].clone())
                        .collect::<Vec<_>>();
                    self.expression_statement(&piece, line, None)
                }
            })
            .collect();
        Some(Statement::Block(pieces))
    }
}
