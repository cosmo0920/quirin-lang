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

//! Card **MEMBER-ACCESS-PROJECTION** (gateway rule 38h): an uninitialized
//! declaration read from the Clang AST where the lexical gate cannot see that
//! it is one.
//!
//! The lexical indirect-access gate of
//! [`crate::legacy_c::Parser`]'s statement classifier admits a declaration
//! with no `=` only when its first token is a keyword specifier
//! (`DECLARATION_SPECIFIERS`) and every bracket bound is a decimal literal
//! (`constant_array_declaration`): such a declaration allocates automatic
//! storage, evaluates nothing and lowers to `NoOp`. The same declaration
//! spelled with a typedef specifier (`rd_segment_t *seg, *tmp;`,
//! `uint32_t op[32];`) or a constant-expression bound (`char buf[1024 * 3];`)
//! reaches the token test instead, which reads the `*` after a comma as a
//! dereference and a bracket as a subscript, and refuses it as an indirect
//! memory access. The census of card MEMBER-ACCESS-PROJECTION counted 141 such
//! leaves on the 046accf6 `current` run.
//!
//! Clang has the answer. This module decides, from the function's own AST
//! node for the statement (matched by line and exact tokens, several told
//! apart by occurrence, as every typed reader is), whether the statement is
//! **exactly one declaration statement of automatic objects, none
//! initialized and none variably modified** ([`DeclarationFacts::all`]).
//! Only then does the gate take its keyword-specifier branch's answer for the
//! statement: admitted, and lowered by the classifier's unchanged fall-through
//! to `NoOp`, exactly what the keyword spelling of the same declaration gets.
//! The rule is asked only where the old gate refuses ([`refuses`]), so every
//! statement the gate admitted keeps its admission and every other refusal
//! stays byte for byte.
//!
//! Premises: faithful Clang extraction of the statement's declarations, their
//! storage classes and their types (trust boundary 28); C11 6.7.9p10 and
//! 6.2.4p6: an object with automatic storage and no initializer has an
//! indeterminate value and its declaration evaluates nothing unless its type
//! is variably modified (6.7.6.2p2, 6.8p3), so no memory is read or written
//! and no identity is created. The Lean model is `QCore.TypedDeclarationReading`.

use serde_json::Value;

use crate::typed_c_ast::{children, kind};

/// What the AST says about one statement, field by field, so the decision
/// table and its Lean vectors can name each premise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DeclarationFacts {
    /// The node is a `DeclStmt`.
    pub(crate) declaration_statement: bool,
    /// It declares at least one object and every declaration in it is a
    /// `VarDecl` (no function, typedef, record or enumeration declaration).
    pub(crate) only_objects: bool,
    /// No declared object has an initializer (no `init` and no inner node).
    pub(crate) none_initialized: bool,
    /// No declared object carries a storage class (`static`, `extern`,
    /// `register`, thread-local): every one is automatic.
    pub(crate) automatic: bool,
    /// Every array extent in every declared type, as written and desugared,
    /// is a decimal constant: nothing is variably modified.
    pub(crate) constant_types: bool,
}

impl DeclarationFacts {
    /// The conjunction the rule admits on.
    pub(crate) const fn all(self) -> bool {
        self.declaration_statement
            && self.only_objects
            && self.none_initialized
            && self.automatic
            && self.constant_types
    }
}

/// Whether every bracket group of a Clang type spelling holds only decimal
/// digits. A variable-length array prints its size expression (`char[n]`);
/// a constant array prints its evaluated size (`char[3072]`).
fn constant_extents(spelling: &str) -> bool {
    let mut rest = spelling;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else {
            return false;
        };
        let bound = &rest[open + 1..open + close];
        if bound.is_empty() || !bound.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        rest = &rest[open + close + 1..];
    }
    !rest.contains(']')
}

/// The facts of one statement node.
pub(crate) fn facts(node: &Value) -> DeclarationFacts {
    let declarations = children(node);
    let objects = declarations
        .iter()
        .all(|declaration| kind(declaration) == "VarDecl");
    DeclarationFacts {
        declaration_statement: kind(node) == "DeclStmt",
        only_objects: !declarations.is_empty() && objects,
        none_initialized: declarations.iter().all(|declaration| {
            declaration.get("init").is_none() && children(declaration).is_empty()
        }),
        automatic: declarations.iter().all(|declaration| {
            declaration.get("storageClass").is_none() && declaration.get("tls").is_none()
        }),
        constant_types: declarations.iter().all(|declaration| {
            let spelled = declaration["type"]["qualType"].as_str();
            let desugared = declaration["type"]["desugaredQualType"].as_str();
            spelled.is_some_and(constant_extents) && desugared.is_none_or(constant_extents)
        }),
    }
}

/// The gate's answer with this rule: `old` is the lexical gate's refusal,
/// `declaration_like` its own declaration-prefix test, and `facts` the AST's
/// answer for the statement (`None` when no unique node matched). The rule
/// only ever turns a refusal into an admission, and only on every fact.
pub(crate) fn refuses(old: bool, declaration_like: bool, facts: Option<DeclarationFacts>) -> bool {
    old && !(declaration_like && facts.is_some_and(DeclarationFacts::all))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_facts() -> Vec<DeclarationFacts> {
        let mut out = Vec::new();
        for declaration_statement in [false, true] {
            for only_objects in [false, true] {
                for none_initialized in [false, true] {
                    for automatic in [false, true] {
                        for constant_types in [false, true] {
                            out.push(DeclarationFacts {
                                declaration_statement,
                                only_objects,
                                none_initialized,
                                automatic,
                                constant_types,
                            });
                        }
                    }
                }
            }
        }
        out
    }

    /// The decision table `QCore.TypedDeclarationReading` proves, entry for
    /// entry: `old`, `declaration_like`, then no facts or each of the 32.
    #[test]
    fn refusal_vectors_are_the_lean_vectors() {
        let mut actual = Vec::new();
        for old in [false, true] {
            for declaration_like in [false, true] {
                actual.push(refuses(old, declaration_like, None));
                for facts in every_facts() {
                    actual.push(refuses(old, declaration_like, Some(facts)));
                }
            }
        }
        let rendered = actual
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let lean = include_str!("../../../proof/QCore/TypedDeclarationReading.lean");
        assert!(
            lean.contains(&format!("def vectors : List Bool := [{rendered}]")),
            "the Lean vectors differ from the Rust decision: [{rendered}]"
        );
        // Old admissions stay; only the all-facts entries of an old refusal
        // of a declaration-like statement become admissions.
        assert_eq!(actual.iter().filter(|refused| !**refused).count(), 66 + 1);
    }

    fn statement(declarations: Value) -> Value {
        serde_json::json!({ "kind": "DeclStmt", "inner": declarations })
    }

    /// The facts over Clang's own JSON shapes (clang 21 `-ast-dump=json`).
    #[test]
    fn facts_read_clang_declarations() {
        let typedef_pair = statement(serde_json::json!([
            { "kind": "VarDecl", "name": "a", "type": { "qualType": "m_t *" } },
            { "kind": "VarDecl", "name": "b", "type": { "qualType": "m_t *" } },
        ]));
        assert!(facts(&typedef_pair).all());
        let constant = statement(serde_json::json!([
            { "kind": "VarDecl", "name": "buf", "type": { "qualType": "char[3072]" } },
        ]));
        assert!(facts(&constant).all());
        let initialized = statement(serde_json::json!([
            { "kind": "VarDecl", "name": "k", "type": { "qualType": "int" }, "init": "c",
              "inner": [{ "kind": "IntegerLiteral", "value": "2" }] },
            { "kind": "VarDecl", "name": "j", "type": { "qualType": "int" } },
        ]));
        assert!(!facts(&initialized).none_initialized);
        let fixed = statement(serde_json::json!([
            { "kind": "VarDecl", "name": "s", "type": { "qualType": "int" }, "storageClass": "static" },
        ]));
        assert!(!facts(&fixed).automatic);
        let vla = statement(serde_json::json!([
            { "kind": "VarDecl", "name": "v", "type": { "qualType": "char[n]" } },
        ]));
        assert!(!facts(&vla).constant_types);
        let aliased_vla = statement(serde_json::json!([
            { "kind": "VarDecl", "name": "t", "type": { "qualType": "tv", "desugaredQualType": "char[n]" } },
        ]));
        assert!(!facts(&aliased_vla).constant_types);
        let typedef = statement(serde_json::json!([
            { "kind": "TypedefDecl", "name": "tv", "type": { "qualType": "char[4]" } },
        ]));
        assert!(!facts(&typedef).only_objects);
        assert!(!facts(&statement(serde_json::json!([]))).only_objects);
        let expression =
            serde_json::json!({ "kind": "BinaryOperator", "opcode": "*", "inner": [] });
        assert!(!facts(&expression).declaration_statement);
    }

    #[test]
    fn extents_are_constant_only_when_every_bound_is_digits() {
        assert!(constant_extents("m_t *"));
        assert!(constant_extents("u32[32]"));
        assert!(constant_extents("char[3072]"));
        assert!(constant_extents("int[2][4]"));
        assert!(!constant_extents("char[n]"));
        assert!(!constant_extents("char[]"));
        assert!(!constant_extents("char (*)[n]"));
        assert!(!constant_extents("char[4"));
    }
}
