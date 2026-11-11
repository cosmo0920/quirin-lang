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

//! Card **CG23-RELATIONAL-POSTCONDITIONS** (gateway rule 11r): exit-specific
//! summary dispositions (Clang gap entry K11) and the body check of a declared
//! return-code disposition.
//!
//! # The exit-specific route
//!
//! Consulted only after every ordinary route published nothing for a function
//! defined in the unit, and only when its definition is `int NAME(…)` (with at
//! most `static`, `inline` or `extern` beside `int`) and **every** `return` of
//! its body returns a plain integer literal, with at least two distinct
//! values. For each returned value `v` the body is re-read with every other
//! `return` replaced by the divergent statement `for (;;);`, which removes
//! exactly the paths ending there and leaves every state on every other path
//! as it was; the **ordinary** inference and validation then run unchanged on
//! that body. The class `eq v` takes the contract validated there, which
//! therefore holds on every exit returning `v`. An exit that falls off the end
//! of the body (no `return`) belongs to every per-value body, so every class
//! is validated on it too.
//!
//! The classes are published as an inferred [`RelationalContract`], whose
//! returned values are exactly the literals (the body returns no other). When
//! every class states the same contract nothing is published: that is no
//! exit-specific relation, and the ordinary route already refused it.
//!
//! # The body check
//!
//! A declared disposition of a function the unit defines is checked against
//! the body's own summary: an ordinary one must equal every declared class's
//! contract, and an exit-specific one must equal, for each returned value,
//! the contract of every declared class containing it. A contradiction
//! withdraws the declaration ([`RelationalContract::withdrawn`]), so every call
//! site refuses. A body that yields no summary leaves the declaration a
//! declared assumption, as an ordinary declaration is (rule 8).

use std::collections::BTreeSet;

use qcore_schemas::contract::{ResourceContracts, ResourceFunctionContract, ResourceReturnMode};

use crate::legacy_c::{Token, matching};
use crate::relational_post::{
    ExitClass, RelationOp, RelationalContract, RelationalOrigin, ReturnRelation, integer_constant,
};

/// The declaration specifiers an exit-specific summary admits beside `int`.
const SPECIFIERS: &[&str] = &["static", "inline", "extern", "__inline", "__inline__"];

/// The definition of `function`: the name's index and the body's braces.
/// (Shared with rule 12s, `legacy_relational_forwarding.rs`.)
pub(super) fn definition(tokens: &[Token], function: &str) -> Option<(usize, usize, usize)> {
    for (index, token) in tokens.iter().enumerate() {
        if token.text != function || tokens.get(index + 1).map(|t| t.text.as_str()) != Some("(") {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")")?;
        if tokens.get(close + 1).map(|t| t.text.as_str()) == Some("{") {
            let end = matching(tokens, close + 1, "{", "}")?;
            return Some((index, close + 1, end));
        }
    }
    None
}

/// Whether the definition at `name` is declared `int NAME(…)`.
fn int_result(tokens: &[Token], name: usize) -> bool {
    let start = tokens[..name]
        .iter()
        .rposition(|token| matches!(token.text.as_str(), ";" | "}" | "{"))
        .map_or(0, |index| index + 1);
    let specifiers = &tokens[start..name];
    specifiers
        .iter()
        .filter(|token| token.text == "int")
        .count()
        == 1
        && specifiers
            .iter()
            .all(|token| token.text == "int" || SPECIFIERS.contains(&token.text.as_str()))
}

/// Every `return` of the body `(open, close)`: its index, the index of its
/// `;`, and the literal it returns. `None` when one returns anything else.
fn literal_returns(
    tokens: &[Token],
    open: usize,
    close: usize,
) -> Option<Vec<(usize, usize, i64)>> {
    let mut returns = Vec::new();
    let mut index = open;
    while index < close {
        if tokens[index].text == "return" {
            let end = (index..close).find(|at| tokens[*at].text == ";")?;
            let texts = tokens[index + 1..end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>();
            returns.push((index, end, integer_constant(&texts)?));
            index = end;
        }
        index += 1;
    }
    Some(returns)
}

/// `tokens` with every listed `return …;` replaced by `for (;;);`.
fn diverge_returns(tokens: &[Token], returns: &[(usize, usize, i64)], keep: i64) -> Vec<Token> {
    let mut out = Vec::with_capacity(tokens.len() + returns.len() * 3);
    let mut copied = 0usize;
    for &(start, end, value) in returns {
        if value == keep {
            continue;
        }
        out.extend_from_slice(&tokens[copied..start]);
        let line = tokens[start].line;
        for text in ["for", "(", ";", ";", ")", ";"] {
            out.push(Token {
                text: text.to_owned(),
                line,
            });
        }
        copied = end + 1;
    }
    out.extend_from_slice(&tokens[copied..]);
    out
}

/// The exit-specific summary of `function`, or `None`. `infer` is the
/// ordinary inference and validation, run unchanged on each per-value body,
/// then card OUT-PARAMETER-BODY-SUMMARY's unwritten class (rule 11r(6)); it
/// returns the class contract with the slots the class leaves unwritten.
pub(super) fn infer_exit_specific(
    tokens: &[Token],
    function: &str,
    environment: &ResourceContracts,
    infer: impl Fn(
        &[Token],
        &str,
        &ResourceContracts,
    ) -> Option<(ResourceFunctionContract, BTreeSet<usize>)>,
) -> Option<RelationalContract> {
    let (name, open, close) = definition(tokens, function)?;
    if !int_result(tokens, name) {
        return None;
    }
    let returns = literal_returns(tokens, open, close)?;
    let mut values = returns
        .iter()
        .map(|(_, _, value)| *value)
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    if values.len() < 2 {
        return None;
    }
    let mut classes = Vec::with_capacity(values.len());
    let mut unwritten = Vec::with_capacity(values.len());
    for value in values {
        let body = diverge_returns(tokens, &returns, value);
        let (contract, quiet) = infer(&body, function, environment)?;
        if contract.result != ResourceReturnMode::None {
            return None;
        }
        classes.push(ExitClass {
            relation: ReturnRelation::new(RelationOp::Eq, value),
            contract,
        });
        unwritten.push(quiet);
    }
    // Rule 11r(6): a slot a class leaves unwritten is a relation only where
    // another class writes it; it is then exactly the slot the consumer reads
    // as unwritten (`Borrowed` here, `Out` there), and nothing else is.
    for (index, quiet) in unwritten.iter().enumerate() {
        if !quiet.is_empty() && *quiet != super::unwritten_class::unwritten_slots(&classes, index) {
            return None;
        }
    }
    Some(RelationalContract {
        origin: RelationalOrigin::InferredValidated,
        classes,
        withdrawn: None,
    })
}

/// What the body of a function with a declared disposition says about it.
pub(super) enum BodySummary {
    Ordinary(ResourceFunctionContract),
    ExitSpecific(RelationalContract),
    /// Card CALLEE-RETENTION-CLASS1 (rule 12s): the classes a forwarding body
    /// takes from its callee, each validated on its restricted body.
    Forwarded(RelationalContract),
}

/// The reason `declared` is contradicted by `body`, if it is.
pub(super) fn contradiction(declared: &RelationalContract, body: &BodySummary) -> Option<String> {
    match body {
        BodySummary::Ordinary(contract) => declared
            .classes
            .iter()
            .find(|class| class.contract.custody_projection() != *contract)
            .map(|class| {
                format!(
                    "the body's summary holds on every exit and differs from the `{} {}` class",
                    class.relation.op.spelling(),
                    class.relation.constant
                )
            }),
        BodySummary::ExitSpecific(body) => {
            for exit in &body.classes {
                let value = i128::from(exit.relation.constant);
                for class in &declared.classes {
                    if class.relation.holds(value)
                        && class.contract.custody_projection() != exit.contract
                    {
                        return Some(format!(
                            "the body's exits returning {} differ from the `{} {}` class",
                            exit.relation.constant,
                            class.relation.op.spelling(),
                            class.relation.constant
                        ));
                    }
                }
            }
            None
        }
        // Card CALLEE-RETENTION-CLASS1 (rule 12s): a forwarded class and a
        // declared class that share a returned value must agree.
        BodySummary::Forwarded(body) => {
            for exit in &body.classes {
                for class in &declared.classes {
                    if class.relation.meets(exit.relation)
                        && class.contract.custody_projection() != exit.contract
                    {
                        return Some(format!(
                            "the body's forwarded `{} {}` class differs from the `{} {}` class",
                            exit.relation.op.spelling(),
                            exit.relation.constant,
                            class.relation.op.spelling(),
                            class.relation.constant
                        ));
                    }
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(text: &str) -> Vec<Token> {
        text.split_whitespace()
            .map(|word| Token {
                text: word.to_owned(),
                line: 1,
            })
            .collect()
    }

    #[test]
    fn reads_literal_returns_of_an_int_definition() {
        let tokens = stream(
            "static int rel ( void * p ) { if ( c ) { q_free ( p ) ; return 0 ; } return - 1 ; }",
        );
        let (name, open, close) = definition(&tokens, "rel").unwrap();
        assert!(int_result(&tokens, name));
        let returns = literal_returns(&tokens, open, close).unwrap();
        assert_eq!(
            returns.iter().map(|(_, _, v)| *v).collect::<Vec<_>>(),
            [0, -1]
        );
        let body = diverge_returns(&tokens, &returns, 0);
        let text = body
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            text.contains("return 0 ;") && text.contains("for ( ; ; ) ;"),
            "{text}"
        );
        assert!(!text.contains("return - 1"), "{text}");
    }

    #[test]
    fn refuses_other_definitions() {
        for (text, name) in [
            ("void * rel ( void * p ) { return 0 ; }", "rel"),
            ("unsigned int rel ( void * p ) { return 0 ; }", "rel"),
        ] {
            let tokens = stream(text);
            let (index, _, _) = definition(&tokens, name).unwrap();
            assert!(!int_result(&tokens, index), "{text}");
        }
        let tokens = stream("int rel ( void * p ) { return x ; }");
        let (_, open, close) = definition(&tokens, "rel").unwrap();
        assert!(literal_returns(&tokens, open, close).is_none());
        let tokens = stream("int rel ( void * p ) { return ; }");
        let (_, open, close) = definition(&tokens, "rel").unwrap();
        assert!(literal_returns(&tokens, open, close).is_none());
    }
}
