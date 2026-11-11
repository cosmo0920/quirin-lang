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

//! Card **CG21-CALL-EXITS-AND-BOUNDARIES** (gateway rule 11o): the route-G
//! hooks of [`crate::call_exits`].
//!
//! * [`Parser::call_exit_boundary`], asked **first** for every expression or
//!   declaration statement: inline assembly, SEH and a `cleanup` attribute
//!   are an `Unknown` with the boundary's name and unbounded writes. It is a
//!   soundness repair, not a fallback: an older route could otherwise admit
//!   the statement without the attribute's scope-exit call or the assembly's
//!   effects.
//! * [`Parser::call_exit_statement`], asked right after the statement's
//!   classification: a `ContractCall` statement whose callee the environment
//!   declares `resource_no_return()` is [`Statement::PathEnd`] when this
//!   body's Clang AST resolves the callee to a `noreturn` declaration, and an
//!   `Unknown` otherwise. Such contracts are new with the card, so the hook
//!   replaces no older lowering. Every consultation is a parity row
//!   (`position: "call-exit-statement"`); in parity runs an uncontracted
//!   whole-call statement is also resolved, for the census only (`position:
//!   "call-exit-census"`), and its answer changes nothing.

use serde_json::json;

use super::{LegacyInstruction, Parser, RefusedWrites, Statement, Token};
use crate::call_exits::{self, Decision, Resolution};
use crate::typed_access_path::{parity_enabled, record_row};

const UNCONTRACTED: &str = "call to uncontracted function `";

impl Parser<'_> {
    /// Hook (asked first): the statement's named boundary, if it holds one.
    pub(super) fn call_exit_boundary(&self, tokens: &[Token], line: usize) -> Option<Statement> {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let boundary = call_exits::statement_boundary(&texts)?;
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": texts,
            "position": "call-exit-boundary",
            "boundary": format!("{boundary:?}"),
            "new_admitted": false,
            "new_refusal": boundary.description(),
        }));
        Some(Statement::Instruction(
            LegacyInstruction::Unknown {
                description: boundary.description().to_owned(),
                writes: RefusedWrites::Unbounded,
            },
            line,
        ))
    }

    /// Hook: `goto *E;` (the tokens after `goto`), a named boundary with
    /// unbounded writes. The node keeps the next statement as its successor,
    /// which is not C's edge set; the refusal makes the function `Unknown`,
    /// so no claim rests on it.
    pub(super) fn computed_goto(&self, tokens: &[Token], line: usize) -> Statement {
        let boundary = call_exits::Boundary::ComputedGoto;
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": tokens.iter().map(|token| token.text.as_str()).collect::<Vec<_>>(),
            "position": "call-exit-boundary",
            "boundary": format!("{boundary:?}"),
            "new_admitted": false,
            "new_refusal": boundary.description(),
        }));
        Statement::Instruction(
            LegacyInstruction::Unknown {
                description: boundary.description().to_owned(),
                writes: RefusedWrites::Unbounded,
            },
            line,
        )
    }

    /// Hook: a declared no-return call statement, lowered as its path end or
    /// refused. `None` keeps `instruction`.
    pub(super) fn call_exit_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let (function, contracted) = match instruction {
            LegacyInstruction::ContractCall {
                function,
                destination: None,
                ..
            } => (function.as_str(), true),
            LegacyInstruction::Unknown { description, .. } if parity_enabled() => {
                // Census only: the uncontracted whole-call statement.
                let callee = description
                    .strip_prefix(UNCONTRACTED)?
                    .strip_suffix('`')?
                    .to_owned();
                if whole_call(tokens) == Some(callee.as_str()) {
                    self.call_exit_census(&callee, line, tokens);
                }
                return None;
            }
            _ => return None,
        };
        let declared = self
            .declared_no_return
            .get_or_init(|| call_exits::declared_no_return(self.contracts))
            .contains(function);
        if !declared {
            return None;
        }
        let resolution = self.call_exit_resolution(function);
        let decision = call_exits::decide(contracted, declared, resolution);
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": tokens.iter().map(|token| token.text.as_str()).collect::<Vec<_>>(),
            "position": "call-exit-statement",
            "callee": function,
            "declared_no_return": declared,
            "resolution": format!("{resolution:?}"),
            "decision": format!("{decision:?}"),
            "new_admitted": decision == Decision::PathEnd,
        }));
        match decision {
            Decision::PathEnd => Some(Statement::PathEnd(instruction.clone(), line)),
            // The writes an uncontracted call statement of the same tokens
            // would refuse: only the wording differs from that refusal.
            Decision::Refused => Some(Statement::Instruction(
                LegacyInstruction::Unknown {
                    description: call_exits::unestablished_description(function, resolution),
                    writes: self.refused_writes(
                        &tokens
                            .iter()
                            .map(|token| token.text.as_str())
                            .collect::<Vec<_>>(),
                    ),
                },
                line,
            )),
            Decision::Continue => None,
        }
    }

    /// This body's Clang resolution of `callee`; unresolved without an AST.
    /// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (rule 13s): read from the
    /// function's table (one walk per function), which answers every name as
    /// `call_exits::resolve` does.
    pub(super) fn call_exit_resolution(&self, callee: &str) -> Resolution {
        self.typed_positions_index()
            .map_or(Resolution::Unresolved, |index| {
                index.call_resolution(callee)
            })
    }

    /// Parity runs only: what the AST says of an uncontracted call statement.
    fn call_exit_census(&self, callee: &str, line: usize, tokens: &[Token]) {
        let resolution = self.call_exit_resolution(callee);
        record_row(json!({
            "function": self.typed_path_function,
            "line": line,
            "texts": tokens.iter().map(|token| token.text.as_str()).collect::<Vec<_>>(),
            "position": "call-exit-census",
            "callee": callee,
            "resolution": format!("{resolution:?}"),
            "new_admitted": false,
        }));
    }
}

/// The callee of a statement that is exactly one direct call, optionally cast
/// to `void`: `NAME ( … )` or `( void ) NAME ( … )`.
fn whole_call(tokens: &[Token]) -> Option<&str> {
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let start = match texts.as_slice() {
        ["(", "void", ")", ..] => 3,
        _ => 0,
    };
    if texts.get(start + 1) != Some(&"(") {
        return None;
    }
    let close = super::matching(tokens, start + 1, "(", ")")?;
    (close + 1 == tokens.len()).then(|| tokens[start].text.as_str())
}
