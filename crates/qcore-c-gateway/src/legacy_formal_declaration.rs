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

/// Every declaration of `name` in `function` (counting `VarDecl`,
/// `ParmVarDecl`, `FunctionDecl`, `EnumConstantDecl` and `TypedefDecl` of the
/// name at any depth, the function's own parameters included) with its id,
/// and the id every `DeclRefExpr` naming `name` refers to (`None` where Clang
/// gives none).
type Census<'a> = (Vec<(&'a str, &'a Value)>, Vec<Option<&'a str>>);

fn census<'a>(function: &'a Value, name: &str) -> Census<'a> {
    fn walk<'a>(
        node: &'a Value,
        name: &str,
        declarations: &mut Vec<(&'a str, &'a Value)>,
        references: &mut Vec<Option<&'a str>>,
    ) {
        if matches!(
            kind(node),
            "VarDecl" | "ParmVarDecl" | "FunctionDecl" | "EnumConstantDecl" | "TypedefDecl"
        ) && node["name"].as_str() == Some(name)
        {
            declarations.push((node["id"].as_str().unwrap_or(""), node));
        }
        if kind(node) == "DeclRefExpr" && node["referencedDecl"]["name"].as_str() == Some(name) {
            references.push(node["referencedDecl"]["id"].as_str());
        }
        for child in children(node) {
            walk(child, name, declarations, references);
        }
    }
    let mut declarations = Vec::new();
    let mut references = Vec::new();
    for child in children(function) {
        walk(child, name, &mut declarations, &mut references);
    }
    (declarations, references)
}

/// Whether `name` in `function` (a Clang `FunctionDecl`) has exactly one
/// declaration, a direct `ParmVarDecl` of the function, and every
/// `DeclRefExpr` naming `name` refers to it. A shadowing local (a `VarDecl`
/// of the name in any block or for-init), a second `ParmVarDecl` (a
/// block-scope prototype), or a reference to any other declaration (a
/// file-scope object, an enumeration constant, a function) refuses.
pub(crate) fn every_reference_is_the_formal(function: &Value, name: &str) -> bool {
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
    let (declarations, references) = census(function, name);
    declarations.len() == 1 && references.iter().all(|reference| *reference == Some(id))
}

/// Card JUMPED-SHADOW-FORMAL-READINGS (rule 86): the same reading for a
/// local, such as a container-publication derived local (`L = P->F`, read as
/// memory the caller supplied): exactly one declaration of the name in the
/// function, a `VarDecl` with an id, and every `DeclRefExpr` naming the name
/// refers to it. A second declaration of the name (a local that shadows it,
/// jumped over or not) or a reference elsewhere refuses.
pub(crate) fn every_reference_is_the_local(function: &Value, name: &str) -> bool {
    let (declarations, references) = census(function, name);
    let [(id, declaration)] = declarations.as_slice() else {
        return false;
    };
    kind(declaration) == "VarDecl"
        && !id.is_empty()
        && references.iter().all(|reference| *reference == Some(*id))
}

/// Card JUMPED-SHADOW-FORMAL-READINGS (rule 86): the named formals of
/// `function`, by Clang's reading, that are not one declaration
/// ([`every_reference_is_the_formal`]): a local of the same name anywhere in
/// the body, whatever its spelling (an opening `__attribute__`, `_Alignas`
/// or `__typeof__`, a parenthesized declarator, an enumeration constant), and
/// whether or not control reaches its declaration.
pub(crate) fn shadowed_formals(function: &Value) -> Vec<String> {
    children(function)
        .iter()
        .filter(|node| kind(node) == "ParmVarDecl")
        .filter_map(|node| node["name"].as_str())
        .filter(|name| !name.is_empty() && !every_reference_is_the_formal(function, name))
        .map(str::to_owned)
        .collect()
}

/// Card JUMPED-SHADOW-FORMAL-READINGS (rule 86): the counted-user pass's
/// stable formals of `function` (its own token census), kept only where the
/// name is one declaration ([`every_reference_is_the_formal`]) in Clang's
/// reading of the function. Without that reading, none is kept: every
/// operation is then unattributed and refuses.
pub(crate) fn one_declaration_formals(
    function: &str,
    stable: std::collections::BTreeSet<String>,
) -> std::collections::BTreeSet<String> {
    let Some(index) = crate::typed_access_path::StatementIndex::read(function) else {
        return std::collections::BTreeSet::new();
    };
    stable
        .into_iter()
        .filter(|name| every_reference_is_the_formal(&index.function, name))
        .collect()
}

/// The tokens that let control jump over a declaration.
const JUMPS: [&str; 2] = ["switch", "goto"];

/// The tokens that open a declaration the classifier's lexical guard does
/// not recognize.
const UNGUARDED: [&str; 14] = [
    "__attribute__",
    "__attribute",
    "_Alignas",
    "alignas",
    "__typeof__",
    "__typeof",
    "typeof",
    "typeof_unqual",
    "__typeof_unqual__",
    "__auto_type",
    "enum",
    "typedef",
    "_Static_assert",
    "static_assert",
];

/// The shadow guard's refusal, shared with the classifier's lexical guard.
pub(crate) const SHADOW_REFUSAL: &str = "local declaration may shadow a formal resource identity";

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

    /// Without Clang's reading of the function, why the body may hide a
    /// shadow of a formal from the classifier's lexical guard, if it may: a
    /// `switch` or `goto` in a body where that guard refused a declaration
    /// (`guarded`; the refusal may lie on no path), a token opening a
    /// declaration the guard does not recognize ([`UNGUARDED`]), or a
    /// statement opening `T ( P` with `P` a formal and `T` a name some
    /// `typedef` of the unit declares (a parenthesized declarator, which the
    /// guard reads as a call). An over-approximation: a body with none of
    /// them declares a formal's name only in a form the lexical guard refuses
    /// where it stands, on the paths through the declaration.
    fn lexically_unguarded(&self, guarded: bool) -> Option<String> {
        let start = self.owned_out_body_start;
        let end = super::matching(self.tokens, start, "{", "}")?;
        let texts = self.tokens[start + 1..end]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if let Some(word) = texts.iter().find(|text| UNGUARDED.contains(text)) {
            return Some(format!("the body spells `{word}`"));
        }
        if guarded && let Some(word) = texts.iter().find(|text| JUMPS.contains(text)) {
            return Some(format!(
                "a declaration the lexical guard refused may be jumped over (`{word}`)"
            ));
        }
        let typedef_names = self.typedef_names();
        (0..texts.len()).find_map(|at| {
            let opens = at == 0 || matches!(texts[at - 1], ";" | "{" | "}" | ":");
            let name = texts[at];
            let mut next = at + 1;
            if !opens || !typedef_names.contains(name) || texts.get(next) != Some(&"(") {
                return None;
            }
            while texts.get(next) == Some(&"(") {
                next += 1;
            }
            let formal = texts.get(next)?;
            self.formal_parameters
                .contains(*formal)
                .then(|| format!("`{name} ( {formal}` may declare `{formal}`"))
        })
    }

    /// Every name a `typedef` statement of the unit mentions: a superset of
    /// the unit's typedef names, read from the tokens.
    fn typedef_names(&self) -> std::collections::BTreeSet<&str> {
        let mut names = std::collections::BTreeSet::new();
        let mut inside = false;
        let mut depth = 0usize;
        for token in self.tokens {
            match token.text.as_str() {
                "typedef" => inside = true,
                "{" | "(" | "[" => depth += 1,
                "}" | ")" | "]" => depth = depth.saturating_sub(1),
                ";" if depth == 0 => inside = false,
                text if inside => {
                    names.insert(text);
                }
                _ => {}
            }
        }
        names
    }

    /// Card JUMPED-SHADOW-FORMAL-READINGS (rule 86), the hook before the
    /// body is parsed. Narrows [`super::container_publication::CallerFormals`]
    /// by declaration ([`every_reference_is_the_formal`],
    /// [`every_reference_is_the_local`]), so that every consumer of it
    /// (container publication and its foreign unlinks, member custody
    /// attribution, slot moves, rule 38's descriptor-cell `formal` closure,
    /// rule 12t) reads a formal or a derived local only where its name is one
    /// declaration. Without Clang's reading of the function (a unit Clang
    /// does not accept), no derived local is read and a formal keeps the
    /// census reading, guarded by [`Self::shadow_refusal`].
    pub(super) fn read_formals_by_declaration(&mut self) -> ShadowReading {
        let function = self
            .typed_positions_index()
            .map(|index| std::sync::Arc::clone(&index.function));
        let caller_formals = std::mem::take(&mut self.caller_formals);
        let Some(function) = function else {
            self.caller_formals = caller_formals.by_declaration(&|_| true, &|_| false);
            return ShadowReading::Unread;
        };
        self.caller_formals = caller_formals.by_declaration(
            &|name| every_reference_is_the_formal(&function, name),
            &|name| every_reference_is_the_local(&function, name),
        );
        ShadowReading::Read(shadowed_formals(&function))
    }

    /// Card JUMPED-SHADOW-FORMAL-READINGS (rule 86), the hook after the body
    /// is parsed: the shadow guard's refusal for the function's entry, on
    /// every path. A formal that is not one declaration
    /// ([`shadowed_formals`]) refuses, so a declaration control jumps over,
    /// and one whatever its spelling, refuses the formal's entry custody and
    /// every name-keyed reading of it. Without Clang's reading, the body
    /// refuses where it may hide a shadow from the lexical guard
    /// ([`Self::lexically_unguarded`]).
    pub(super) fn shadow_refusal(&self, reading: ShadowReading) -> Option<String> {
        match reading {
            ShadowReading::Read(shadowed) => (!shadowed.is_empty()).then(|| {
                format!(
                    "{SHADOW_REFUSAL}: `{}` is not one declaration",
                    shadowed.join("`, `")
                )
            }),
            ShadowReading::Unread => self
                .lexically_unguarded(self.shadow_guard_refused.get())
                .map(|why| {
                    format!("{SHADOW_REFUSAL}: the function's declarations are not read, and {why}")
                }),
        }
    }
}

/// What [`Parser::read_formals_by_declaration`] read.
pub(super) enum ShadowReading {
    /// Clang's reading: the formals that are not one declaration.
    Read(Vec<String>),
    /// No Clang reading of the function.
    Unread,
}

#[cfg(test)]
mod tests {
    use super::{every_reference_is_the_formal, every_reference_is_the_local, shadowed_formals};
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

    #[test]
    fn a_local_is_one_declaration_only_unshadowed() {
        let own = function(
            vec![parameter("0x1", "holder")],
            vec![
                json!({"kind": "DeclStmt", "inner": [variable("0x9", "h")]}),
                reference("0x9", "h"),
            ],
        );
        assert!(every_reference_is_the_local(&own, "h"));
        // A jumped or inner-block local of the same name, used or not.
        let shadowed = function(
            vec![parameter("0x1", "holder")],
            vec![
                json!({"kind": "DeclStmt", "inner": [variable("0x9", "h")]}),
                json!({"kind": "SwitchStmt", "inner": [
                    {"kind": "DeclStmt", "inner": [variable("0xa", "h")]},
                    reference("0xa", "h"),
                ]}),
            ],
        );
        assert!(!every_reference_is_the_local(&shadowed, "h"));
        // A formal is not a local, and a reference elsewhere refuses.
        assert!(!every_reference_is_the_local(&own, "holder"));
        let foreign = function(
            vec![],
            vec![
                json!({"kind": "DeclStmt", "inner": [variable("0x9", "h")]}),
                reference("0x5", "h"),
            ],
        );
        assert!(!every_reference_is_the_local(&foreign, "h"));
    }

    #[test]
    fn the_shadowed_formals_are_read_by_declaration() {
        let f = function(
            vec![
                parameter("0x1", "src"),
                parameter("0x2", "s"),
                parameter("0x3", ""),
            ],
            vec![
                json!({"kind": "EnumDecl", "inner": [{"kind": "EnumConstantDecl", "id": "0x7", "name": "src"}]}),
                reference("0x2", "s"),
            ],
        );
        assert_eq!(shadowed_formals(&f), vec!["src".to_owned()]);
    }
}
