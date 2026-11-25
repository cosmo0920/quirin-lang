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

//! Card **NAME-KEYED-HOLDER-SHADOWING** (gateway rule 86): a formal-keyed
//! reading of rule 12t is one declaration.
//!
//! Rule 12t reads a retained actual `NAME` as a caller's object, an interior
//! `&NAME->f` as that object's member, and a path holder `NAME->f` as
//! storage the caller supplied, and it keys a removal on either. Each reading
//! starts from `CallerFormals::contains`, a token census of the formal's
//! spelling, which cannot tell the formal from a block-scope local that
//! shadows it. Rule 74o closed the same shape for frame holders with
//! `one_automatic_object` and `every_reference_is`, which read `VarDecl`s
//! only. This is their analogue for a formal: the name has exactly one
//! declaration in the function, its `ParmVarDecl`, and every `DeclRefExpr`
//! of the name designates it.

use serde_json::Value;

use super::Parser;
use crate::typed_c_ast::{children, kind};

/// Whether `name` in `function` (a Clang `FunctionDecl`) has exactly one
/// declaration, a direct `ParmVarDecl` of the function, and every
/// `DeclRefExpr` naming `name` refers to it. A shadowing local (a `VarDecl`
/// of the name in any block or for-init), a second `ParmVarDecl` (a
/// block-scope prototype), or a reference to any other declaration (a
/// file-scope object, an enumeration constant, a function) refuses.
pub(crate) fn every_reference_is_the_formal(function: &Value, name: &str) -> bool {
    fn walk<'a>(
        node: &'a Value,
        name: &str,
        declarations: &mut usize,
        references: &mut Vec<Option<&'a str>>,
    ) {
        if matches!(
            kind(node),
            "VarDecl" | "ParmVarDecl" | "FunctionDecl" | "EnumConstantDecl" | "TypedefDecl"
        ) && node["name"].as_str() == Some(name)
        {
            *declarations += 1;
        }
        if kind(node) == "DeclRefExpr" && node["referencedDecl"]["name"].as_str() == Some(name) {
            references.push(node["referencedDecl"]["id"].as_str());
        }
        for child in children(node) {
            walk(child, name, declarations, references);
        }
    }
    let parameters = children(function)
        .iter()
        .filter(|node| kind(node) == "ParmVarDecl" && node["name"].as_str() == Some(name))
        .collect::<Vec<_>>();
    let [parameter] = parameters.as_slice() else {
        return false;
    };
    let Some(id) = parameter["id"].as_str().filter(|id| !id.is_empty()) else {
        return false;
    };
    let mut declarations = 0usize;
    let mut references = Vec::new();
    for child in children(function) {
        walk(child, name, &mut declarations, &mut references);
    }
    declarations == 1 && references.iter().all(|reference| *reference == Some(id))
}

impl Parser<'_> {
    /// Rule 12t's caller formal under rule 86: an unrebound formal by the
    /// token census (`CallerFormals::contains`) whose name is one declaration
    /// ([`every_reference_is_the_formal`]). Without Clang's reading of the
    /// function, no name is one.
    pub(super) fn one_declaration_formal(&self, name: &str) -> bool {
        self.caller_formals.contains(name)
            && self
                .typed_positions_index()
                .is_some_and(|index| every_reference_is_the_formal(&index.function, name))
    }
}

#[cfg(test)]
mod tests {
    use super::every_reference_is_the_formal;
    use serde_json::{Value, json};

    fn parameter(id: &str, name: &str) -> Value {
        json!({"kind": "ParmVarDecl", "id": id, "name": name})
    }

    fn variable(id: &str, name: &str) -> Value {
        json!({"kind": "VarDecl", "id": id, "name": name})
    }

    fn reference(id: &str, name: &str) -> Value {
        json!({"kind": "DeclRefExpr", "referencedDecl": {"id": id, "name": name}})
    }

    fn function(parameters: Vec<Value>, body: Vec<Value>) -> Value {
        let mut inner = parameters;
        inner.push(json!({"kind": "CompoundStmt", "inner": body}));
        json!({"kind": "FunctionDecl", "name": "subject", "inner": inner})
    }

    #[test]
    fn an_unshadowed_formal_is_one_declaration() {
        let f = function(
            vec![parameter("0x1", "src"), parameter("0x2", "s")],
            vec![reference("0x1", "src"), reference("0x2", "s")],
        );
        assert!(every_reference_is_the_formal(&f, "src"));
        assert!(every_reference_is_the_formal(&f, "s"));
        // A formal the body never names is one declaration too.
        let unnamed = function(vec![parameter("0x1", "src")], vec![]);
        assert!(every_reference_is_the_formal(&unnamed, "src"));
    }

    #[test]
    fn a_shadowing_local_refuses_even_unreferenced() {
        let used = function(
            vec![parameter("0x1", "src")],
            vec![json!({"kind": "CompoundStmt", "inner": [
                {"kind": "DeclStmt", "inner": [variable("0x9", "src")]},
                reference("0x9", "src"),
            ]})],
        );
        assert!(!every_reference_is_the_formal(&used, "src"));
        let unused = function(
            vec![parameter("0x1", "src")],
            vec![
                json!({"kind": "DeclStmt", "inner": [variable("0x9", "src")]}),
                reference("0x1", "src"),
            ],
        );
        assert!(!every_reference_is_the_formal(&unused, "src"));
    }

    #[test]
    fn a_reference_to_another_declaration_refuses() {
        // A block-scope enumeration constant, or a file-scope object the
        // reference reaches although no local declares it.
        let constant = function(
            vec![parameter("0x1", "src")],
            vec![
                json!({"kind": "EnumDecl", "inner": [{"kind": "EnumConstantDecl", "id": "0x7", "name": "src"}]}),
                reference("0x7", "src"),
            ],
        );
        assert!(!every_reference_is_the_formal(&constant, "src"));
        let foreign = function(vec![parameter("0x1", "src")], vec![reference("0x5", "src")]);
        assert!(!every_reference_is_the_formal(&foreign, "src"));
        let unidentified = function(
            vec![parameter("0x1", "src")],
            vec![json!({"kind": "DeclRefExpr", "referencedDecl": {"name": "src"}})],
        );
        assert!(!every_reference_is_the_formal(&unidentified, "src"));
    }

    #[test]
    fn a_name_that_is_not_one_formal_refuses() {
        // No formal of the name, a local of the name, and a block-scope
        // prototype naming a second parameter.
        let local = function(
            vec![parameter("0x1", "s")],
            vec![
                json!({"kind": "DeclStmt", "inner": [variable("0x9", "src")]}),
                reference("0x9", "src"),
            ],
        );
        assert!(!every_reference_is_the_formal(&local, "src"));
        let prototype = function(
            vec![parameter("0x1", "src")],
            vec![json!({"kind": "DeclStmt", "inner": [
                {"kind": "FunctionDecl", "name": "helper", "inner": [parameter("0x8", "src")]},
            ]})],
        );
        assert!(!every_reference_is_the_formal(&prototype, "src"));
        let anonymous = function(vec![json!({"kind": "ParmVarDecl", "name": "src"})], vec![]);
        assert!(!every_reference_is_the_formal(&anonymous, "src"));
    }
}
