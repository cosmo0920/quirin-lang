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

//! Card **K6-DIRECT-POSITION-ACCESS** (gateway rule 81): the admission and
//! refusal conditions the rule text and the report state, read by the
//! premise-formula reader (`qcore_frontend::premise_formula`).
//!
//! Each statement is a formula of the C04.18.9 language: the syntax step
//! (`parse_tokens`) accepts it and the canonical printing is stable. The
//! sort step (`check`) cannot accept it: the closed `VOCABULARY` holds
//! contract-premise terms only and no term of a gateway reading
//! (`#exempt`, `#binds`, `#place`, ...), and no `TRUE`/`FALSE` constant is
//! registered. This test pins that boundary: every rejection is an unknown
//! term or constant, never a syntax error. Registering a gateway vocabulary
//! block is follow-up K6-RULE-FORMULA-VOCABULARY.

use qcore_frontend::lexer::{TokenKind, lex};
use qcore_frontend::premise_formula::{canonical, check, parse_tokens};

/// The rule 81 statements, as `docs/agents/rules/81.md` states them.
const STATEMENTS: &[&str] = &[
    "#exempt(#a) == TRUE -> #binds(#a) == #binds(#plain(#a))",
    "#exempt(#a) == TRUE -> (#opens_group(#a) == TRUE && #name_arrow(#a) == TRUE && #selection_chain(#a) == TRUE && #group_arrow(#a) == TRUE && #outer_address(#a) == FALSE)",
    "(#opens_group(#a) == TRUE && #name_arrow(#a) == TRUE && #selection_chain(#a) == TRUE && #group_arrow(#a) == TRUE && #outer_address(#a) == FALSE) -> #exempt(#a) == TRUE",
    "#outer_address(#a) == TRUE -> #binds(#a) == TRUE",
    "#name_arrow(#a) == FALSE -> #binds(#a) == TRUE",
    "#arrow_over_member_address(#e) == TRUE -> (#place(#e) == #place(#plain(#e)) && #root(#e) == #root(#plain(#e)))",
];

fn tokens(source: &str) -> Vec<TokenKind> {
    lex(source)
        .expect("the statement lexes")
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect()
}

#[test]
fn every_statement_is_a_formula_of_the_language() {
    for statement in STATEMENTS {
        let raw =
            parse_tokens(&tokens(statement)).unwrap_or_else(|error| panic!("{statement}: {error}"));
        let printed = canonical(&raw);
        let again = parse_tokens(&tokens(&printed)).expect("the canonical printing reparses");
        assert_eq!(canonical(&again), printed, "{statement}");
    }
}

#[test]
fn the_sort_step_rejects_only_unregistered_terms() {
    for statement in STATEMENTS {
        let raw = parse_tokens(&tokens(statement)).expect("parses");
        let error = check(&raw).expect_err("no gateway term is registered");
        assert!(
            error.contains("unknown term") || error.contains("constant"),
            "{statement}: {error}"
        );
    }
}
