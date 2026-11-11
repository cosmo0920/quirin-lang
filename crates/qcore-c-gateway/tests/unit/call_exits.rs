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

//! Unit tests of card CG21's call-exit reading (gateway rule 11o) and the
//! mechanical binding to `proof/QCore/CallExits.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/CallExits.lean");

fn block(start: &str) -> String {
    let body = LEAN.split(start).nth(1).expect(start);
    body[..body.find("\n]").unwrap()].to_owned()
}

#[test]
fn cg21_decision_table() {
    for (contracted, declared, resolution, decision) in DECISION_VECTORS {
        assert_eq!(decide(contracted, declared, resolution), decision);
    }
    // Every combination is listed once.
    let combinations = DECISION_VECTORS
        .iter()
        .map(|(c, d, r, _)| format!("{c}{d}{r:?}"))
        .collect::<BTreeSet<_>>();
    assert_eq!(combinations.len(), DECISION_VECTORS.len());
}

fn lean_resolution(word: &str) -> Resolution {
    match word {
        ".noReturn" => Resolution::NoReturn,
        ".returns" => Resolution::Returns,
        ".unresolved" => Resolution::Unresolved,
        other => panic!("{other}"),
    }
}

fn lean_decision(word: &str) -> Decision {
    match word {
        ".continue" => Decision::Continue,
        ".pathEnd" => Decision::PathEnd,
        ".refused" => Decision::Refused,
        other => panic!("{other}"),
    }
}

/// The mechanical binding: `decisionVectors` = `DECISION_VECTORS` and
/// `boundaryTable` = `BOUNDARIES`.
#[test]
fn cg21_lean_vectors_match() {
    let rows = block("def decisionVectors : List (Bool × Bool × Resolution × Decision) := [")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let fields = line
                .trim_end_matches(',')
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(", ")
                .collect::<Vec<_>>();
            (
                fields[0] == "true",
                fields[1] == "true",
                lean_resolution(fields[2]),
                lean_decision(fields[3]),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, DECISION_VECTORS.to_vec());
    let boundaries = block("def boundaryTable : List (Boundary × Bool) := [")
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let name = line
                .trim_start_matches("(.")
                .split(',')
                .next()
                .unwrap()
                .to_owned();
            (name, line.contains("true"))
        })
        .collect::<Vec<_>>();
    let rust = BOUNDARIES
        .iter()
        .map(|(boundary, admitted)| {
            let name = match boundary {
                Boundary::InlineAssembly => "inlineAssembly",
                Boundary::AssemblyGoto => "assemblyGoto",
                Boundary::StructuredExceptions => "structuredExceptions",
                Boundary::CleanupAttribute => "cleanupAttribute",
                Boundary::ComputedGoto => "computedGoto",
                Boundary::LabelAddress => "labelAddress",
            };
            (name.to_owned(), *admitted)
        })
        .collect::<Vec<_>>();
    assert_eq!(boundaries, rust);
}

fn read(text: &str) -> Option<Boundary> {
    let texts = text.split(' ').collect::<Vec<_>>();
    statement_boundary(&texts)
}

#[test]
fn cg21_statement_boundaries() {
    assert_eq!(
        read("__asm__ volatile ( \"\" : : : \"memory\" )"),
        Some(Boundary::InlineAssembly)
    );
    assert_eq!(read("asm ( \"nop\" )"), Some(Boundary::InlineAssembly));
    assert_eq!(
        read("__asm__ goto ( \"\" : : : : out )"),
        Some(Boundary::AssemblyGoto)
    );
    assert_eq!(
        read("asm volatile goto ( \"\" : : : : out )"),
        Some(Boundary::AssemblyGoto)
    );
    assert_eq!(
        read("register int r __asm__ ( \"eax\" ) = 0"),
        Some(Boundary::InlineAssembly)
    );
    assert_eq!(read("__try { }"), Some(Boundary::StructuredExceptions));
    assert_eq!(
        read("__attribute__ ( ( cleanup ( g ) ) ) int x = 0"),
        Some(Boundary::CleanupAttribute)
    );
    assert_eq!(
        read("__attribute__ ( ( unused , __cleanup__ ( g ) ) ) void * p = q_alloc ( )"),
        Some(Boundary::CleanupAttribute)
    );
    // Not a boundary: a name `cleanup` outside an attribute list, and an
    // attribute list without one.
    assert_eq!(read("cleanup ( p )"), None);
    assert_eq!(
        read("__attribute__ ( ( unused ) ) int x = cleanup ( 1 )"),
        None
    );
    assert_eq!(read("x = 1"), None);
    // The token stream spells `&&` as two tokens.
    assert_eq!(read("void * t = & & l"), Some(Boundary::LabelAddress));
    assert_eq!(
        read("t = c ? & & l1 : & & l2"),
        Some(Boundary::LabelAddress)
    );
    // A logical `&&` follows an operand.
    assert_eq!(read("x = a & & b"), None);
    assert_eq!(read("ok = ( a ) & & b"), None);
    assert_eq!(read("ok = a & & b & & c"), None);
    // `a && &b`: an address operand after a logical `&&`.
    assert_eq!(read("ok = a & & & b"), None);
}

#[test]
fn cg21_undeclarable_calls_are_named() {
    assert_eq!(
        undeclarable_call("longjmp").as_deref(),
        Some("`longjmp` is a non-local exit, which no resource contract or route models")
    );
    assert!(
        undeclarable_call("_setjmp")
            .unwrap()
            .contains("returns-twice")
    );
    assert!(
        undeclarable_call("__builtin_choose_expr")
            .unwrap()
            .contains("ChooseExpr")
    );
    assert_eq!(undeclarable_call("__builtin_expect"), None);
    assert_eq!(undeclarable_call("abort"), None);
}

mod ast {
    use super::super::*;
    use serde_json::json;

    fn reference(name: &str, id: &str, qual_type: &str) -> Value {
        json!({"kind": "DeclRefExpr", "referencedDecl": {
            "id": id, "kind": "FunctionDecl", "name": name, "type": {"qualType": qual_type}}})
    }

    fn body(nodes: Vec<Value>) -> Value {
        json!({"kind": "FunctionDecl", "name": "subject", "inner": [
            {"kind": "CompoundStmt", "inner": nodes}]})
    }

    #[test]
    fn cg21_resolution_reads_the_declaration() {
        let none = BTreeSet::new();
        let gnu = body(vec![reference(
            "die",
            "0x1",
            "void (void) __attribute__((noreturn))",
        )]);
        assert_eq!(resolve(&gnu, "die", &none), Resolution::NoReturn);
        // `_Noreturn`: the attribute on the declaration, by id.
        let c11 = body(vec![reference("die", "0x2", "void (void)")]);
        assert_eq!(resolve(&c11, "die", &none), Resolution::Returns);
        let ids = BTreeSet::from(["0x2".to_owned()]);
        assert_eq!(resolve(&c11, "die", &ids), Resolution::NoReturn);
        // A returned function pointer's attribute is not read as the
        // function's.
        let pointer = body(vec![reference(
            "make",
            "0x3",
            "void (*(void))(void) __attribute__((noreturn))",
        )]);
        assert_eq!(resolve(&pointer, "make", &none), Resolution::Returns);
        // Disagreeing references, a shadowing object and no reference refuse.
        let mixed = body(vec![
            reference("die", "0x4", "void (void)"),
            reference("die", "0x5", "void (void) __attribute__((noreturn))"),
        ]);
        assert_eq!(resolve(&mixed, "die", &none), Resolution::Unresolved);
        let shadowed = body(vec![
            json!({"kind": "VarDecl", "name": "die"}),
            reference("die", "0x6", "void (void) __attribute__((noreturn))"),
        ]);
        assert_eq!(resolve(&shadowed, "die", &none), Resolution::Unresolved);
        assert_eq!(resolve(&gnu, "other", &none), Resolution::Unresolved);
        // The name alone establishes nothing.
        let named = body(vec![reference("abort", "0x7", "void (void)")]);
        assert_eq!(resolve(&named, "abort", &none), Resolution::Returns);
        // A block-scope declaration carrying the attribute.
        let local = body(vec![
            json!({"kind": "FunctionDecl", "id": "0x8", "name": "die",
                   "inner": [{"kind": "C11NoReturnAttr"}]}),
            reference("die", "0x8", "void (void)"),
        ]);
        assert_eq!(resolve(&local, "die", &none), Resolution::NoReturn);
    }
}
