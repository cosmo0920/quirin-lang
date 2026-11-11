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

//! Unit tests of card CG07's statement-expression reading (gateway rule 11g)
//! and the mechanical binding to `proof/QCore/StatementExpressions.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/StatementExpressions.lean");

fn block(start: &str) -> String {
    let body = LEAN.split(start).nth(1).expect(start);
    body[..body.find("\n]").unwrap()].to_owned()
}

fn booleans(line: &str) -> Vec<bool> {
    line.trim()
        .trim_end_matches(',')
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(", ")
        .map(|field| field == "true")
        .collect()
}

#[test]
fn cg07_admission_table() {
    for (position, hygienic, automatic_, uses, taken, last, admitted) in ADMISSION_VECTORS {
        assert_eq!(
            admit(position, hygienic, automatic_, uses, taken, last),
            admitted
        );
    }
}

/// The mechanical binding: `admissionVectors` = `ADMISSION_VECTORS` and
/// `positionTable` = `POSITIONS`.
#[test]
fn cg07_lean_vectors_match() {
    let rows =
        block("def admissionVectors : List (Bool × Bool × Bool × Bool × Bool × Bool × Bool) := [")
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('('))
            .map(booleans)
            .collect::<Vec<_>>();
    let rust = ADMISSION_VECTORS
        .iter()
        .map(|(a, b, c, d, e, f, g)| vec![*a, *b, *c, *d, *e, *f, *g])
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
    let positions = block("def positionTable : List (String × Bool) := [")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let name = line.split('"').nth(1).unwrap().to_owned();
            (name, line.contains("true"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        positions,
        POSITIONS
            .iter()
            .map(|(name, admitted)| ((*name).to_owned(), *admitted))
            .collect::<Vec<_>>()
    );
    for (name, admitted) in POSITIONS {
        assert_eq!(position_admitted(name), admitted);
    }
    assert!(!position_admitted("an unlisted position"));
}

mod ast {
    use super::super::*;
    use crate::typed_c_ast::{children, kind};

    fn function(source: &str) -> Value {
        crate::typed_c_ast::function_ast(
            source,
            &crate::legacy_c::C11TranslationUnit {
                source,
                ..Default::default()
            },
            "subject",
        )
        .unwrap_or_else(|error| panic!("Clang rejected the fixture: {error}\n{source}"))
    }

    fn statement_expressions(node: &Value, out: &mut Vec<Value>) {
        if kind(node) == "StmtExpr" {
            out.push(node.clone());
        }
        for child in children(node) {
            statement_expressions(child, out);
        }
    }

    fn first(node: &Value) -> Option<&Value> {
        if kind(node) == "StmtExpr" {
            return Some(node);
        }
        children(node).iter().find_map(first)
    }

    fn decide_first(source: &str, position: Position) -> Result<(), Refusal> {
        let function = function(source);
        let body = Body::read(&function);
        let node = first(&function).expect("a statement expression");
        decide(&body, node, position)
    }

    #[test]
    fn escape_and_value_premises() {
        if !crate::test_support::clang_available() {
            return;
        }
        assert_eq!(
            decide_first(
                "int *subject(void) { int *p = ({ int t = 1; &t; }); return p; }",
                Position::Initializer
            ),
            Err(Refusal::Escape("t".into()))
        );
        assert_eq!(
            decide_first(
                "int *subject(void) { int *p = ({ int a[2]; a; }); return p; }",
                Position::Initializer
            ),
            Err(Refusal::Escape("a".into()))
        );
        // The same block with its value dropped escapes nothing.
        assert_eq!(
            decide_first(
                "void subject(void) { ({ int t = 1; &t; }); }",
                Position::Discarded
            ),
            Ok(())
        );
        assert_eq!(
            decide_first(
                "int subject(int a) { int x = ({ int t = a; t; }); return x; }",
                Position::Initializer
            ),
            Ok(())
        );
        // A block whose last item is no expression has no value to use.
        assert_eq!(
            decide_first(
                "void subject(int a) { ({ if (a) a = 1; }); }",
                Position::Initializer
            ),
            Err(Refusal::NoValue)
        );
        assert_eq!(
            decide_first(
                "void subject(int a) { ({ if (a) a = 1; }); }",
                Position::Discarded
            ),
            Ok(())
        );
        assert_eq!(
            decide_first(
                "int subject(void) { int x = ({ static int s; s; }); return x; }",
                Position::Initializer
            ),
            Err(Refusal::Storage("s".into()))
        );
    }

    #[test]
    fn hygiene_premises() {
        if !crate::test_support::clang_available() {
            return;
        }
        // An inner name that is also a formal, an outer local or a global.
        for source in [
            "int subject(int t) { int x = ({ int t = 1; t; }); return x; }",
            "int subject(void) { int t = 2; int x = ({ int t = 1; t; }); return x + t; }",
            "int t; int subject(void) { int x = ({ int t = 1; t; }); return x + t; }",
            "int subject(void) { int x = ({ int t = 1; ({ int t = 2; t; }) + t; }); return x; }",
        ] {
            assert_eq!(
                decide_first(source, Position::Initializer),
                Err(Refusal::Shadow("t".into())),
                "{source}"
            );
        }
        // Two disjoint statement expressions may reuse a name.
        let source = "int subject(int a) { int x = ({ int t = a; t; }); int y = ({ int t = x; t; }); return y; }";
        let function = function(source);
        let body = Body::read(&function);
        let mut all = Vec::new();
        statement_expressions(&function, &mut all);
        assert_eq!(all.len(), 2);
        let mut found = Vec::new();
        fn collect<'v>(node: &'v Value, out: &mut Vec<&'v Value>) {
            if kind(node) == "StmtExpr" {
                out.push(node);
            }
            for child in children(node) {
                collect(child, out);
            }
        }
        collect(&function, &mut found);
        for node in found {
            assert_eq!(decide(&body, node, Position::Initializer), Ok(()));
        }
    }
}
