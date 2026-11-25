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

//! Card **CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION** (gateway rule 89): a
//! call statement whose first actual opens with `*` is not a function-pointer
//! declarator.
//!
//! The classifier's declaration-likeness test (`classify_simple_base` in
//! `legacy_classify.rs`) reads a statement prefix `IDENT ( * …` as the
//! declarator `T (*NAME)…`, so the shadow guard refused `q_observe(*out);`
//! and `rd_list_add(*toppars_add_list, …);` as "local declaration may shadow
//! a formal resource identity". The tokens cannot tell the two apart; the
//! function's Clang AST can. This module asks the statement's own node
//! (matched by line and exact tokens, several told apart by occurrence, as
//! every typed reader is) for its kind and drops only that one disjunct of
//! the test when the node is a `CallExpr`: a call expression declares
//! nothing, so it cannot shadow anything (C11 6.7p1, 6.8.3p2). A `DeclStmt`,
//! any other kind, or no unique node keeps the disjunct, so every real
//! declarator (`T (*out);`, `T (*out)[4];`) keeps the refusal, and every
//! other disjunct of the test is untouched. The admitted call then reaches
//! the classifier's normal call reading unchanged: an uncontracted callee,
//! a dereferenced actual the access gate refuses, or a call through a
//! pointer each keeps its own refusal. Premise: faithful Clang extraction of
//! the statement's node kind (trust boundary 28). The Lean model is
//! `QCore.CallNotDeclarator`.

use serde_json::Value;

use super::{Parser, Token};
use crate::typed_c_ast::kind;

/// What the statement's Clang node says, as far as this rule asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reading {
    /// No unique node matched the statement's line and tokens.
    Unmatched,
    /// A `DeclStmt`.
    Declaration,
    /// A `CallExpr`.
    Call,
    /// Any other node kind.
    Other,
}

#[cfg(test)]
impl Reading {
    pub(crate) const ALL: [Self; 4] = [Self::Unmatched, Self::Declaration, Self::Call, Self::Other];
}

/// The reading of one matched node (`None`: no unique node).
pub(crate) fn reading(node: Option<&Value>) -> Reading {
    match node.map(kind) {
        None => Reading::Unmatched,
        Some("DeclStmt") => Reading::Declaration,
        Some("CallExpr") => Reading::Call,
        Some(_) => Reading::Other,
    }
}

/// The `IDENT ( *` disjunct of the declaration-likeness test with this rule:
/// `lexical` is the token test's answer. Only a `CallExpr` drops it.
pub(crate) fn declarator_shaped(lexical: bool, reading: Reading) -> bool {
    lexical && reading != Reading::Call
}

/// The whole declaration-likeness test: a keyword specifier first, an
/// `IDENT IDENT` or `IDENT *` prefix, or the `IDENT ( *` prefix this rule
/// reads. Used only by the decision table the Lean vectors pin; the
/// classifier spells the same disjunction inline.
#[cfg(test)]
pub(crate) fn declaration_like(
    specifier: bool,
    identifier_pair: bool,
    identifier_star: bool,
    paren_star: bool,
    reading: Reading,
) -> bool {
    specifier || identifier_pair || identifier_star || declarator_shaped(paren_star, reading)
}

impl Parser<'_> {
    /// Rule 89's hook: whether the statement `tokens`, whose prefix the token
    /// test read as `IDENT ( *`, is still declarator-shaped once Clang's AST
    /// is asked. Called only when that disjunct holds.
    pub(super) fn paren_star_declarator(&self, tokens: &[Token]) -> bool {
        let Some(line) = tokens.first().map(|token| token.line) else {
            return true;
        };
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let node = self.typed_positions_index().and_then(|index| {
            index.statement_node(&texts, line, &|| {
                crate::typed_place::occurrence(self.tokens, self.typed_place_origin.get(), tokens)
            })
        });
        let reading = reading(node);
        let declarator = declarator_shaped(true, reading);
        if crate::typed_access_path::parity_enabled() {
            crate::typed_access_path::record_row(serde_json::json!({
                "function": self.typed_path_function,
                "line": line,
                "texts": texts,
                "position": "call-not-declarator",
                "rule": "89",
                "reading": format!("{reading:?}"),
                "admitted": !declarator,
            }));
        }
        declarator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The decision table `QCore.CallNotDeclarator` proves, entry for entry:
    /// the four lexical disjuncts, then each reading.
    #[test]
    fn declaration_like_vectors_are_the_lean_vectors() {
        let mut actual = Vec::new();
        for specifier in [false, true] {
            for identifier_pair in [false, true] {
                for identifier_star in [false, true] {
                    for paren_star in [false, true] {
                        for reading in Reading::ALL {
                            actual.push(declaration_like(
                                specifier,
                                identifier_pair,
                                identifier_star,
                                paren_star,
                                reading,
                            ));
                        }
                    }
                }
            }
        }
        let rendered = actual
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let lean = include_str!("../../../proof/QCore/CallNotDeclarator.lean");
        assert!(
            lean.contains(&format!("def vectors : List Bool := [{rendered}]")),
            "the Lean vectors differ from the Rust decision: [{rendered}]"
        );
        // Exactly one entry changes from the token test: the paren-star
        // disjunct alone, read as a call.
        let lexical = |index: usize| {
            let bits = index / 4;
            bits != 0
        };
        let changed = actual
            .iter()
            .enumerate()
            .filter(|(index, value)| lexical(*index) != **value)
            .count();
        assert_eq!(changed, 1);
    }

    #[test]
    fn only_a_call_expression_drops_the_disjunct() {
        let call = serde_json::json!({ "kind": "CallExpr", "inner": [] });
        let declaration = serde_json::json!({ "kind": "DeclStmt", "inner": [] });
        let other = serde_json::json!({ "kind": "BinaryOperator", "inner": [] });
        assert_eq!(reading(Some(&call)), Reading::Call);
        assert_eq!(reading(Some(&declaration)), Reading::Declaration);
        assert_eq!(reading(Some(&other)), Reading::Other);
        assert_eq!(reading(None), Reading::Unmatched);
        assert!(!declarator_shaped(true, Reading::Call));
        for kept in [Reading::Declaration, Reading::Other, Reading::Unmatched] {
            assert!(declarator_shaped(true, kept));
        }
        for any in Reading::ALL {
            assert!(!declarator_shaped(false, any));
        }
    }
}
