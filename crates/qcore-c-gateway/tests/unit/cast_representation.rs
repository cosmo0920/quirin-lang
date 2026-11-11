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

//! Unit tests of card CG08-CASTS-AND-TYPE-QUERIES (gateway rule 11b): the
//! cast table, the type-query operand classes, and the mechanical binding to
//! `proof/QCore/CastRepresentation.lean`.

use super::*;
use crate::legacy_c::C11TranslationUnit;
use crate::test_support::clang_available;

const LEAN: &str = include_str!("../../../../proof/QCore/CastRepresentation.lean");

/// The body of the Lean list `start … \n]`.
fn block(start: &str) -> String {
    let body = LEAN.split(start).nth(1).expect(start);
    body[..body.find("\n]").expect("list end")].to_owned()
}

/// The quoted strings of one Lean row, in order.
fn quoted(row: &str) -> Vec<&str> {
    row.split('"').skip(1).step_by(2).collect()
}

#[test]
fn variably_modified_reads_every_extent() {
    assert!(!variably_modified("int"));
    assert!(!variably_modified("int[4]"));
    assert!(!variably_modified("char[16][2]"));
    assert!(!variably_modified("int[]"));
    assert!(variably_modified("int[n]"));
    assert!(variably_modified("int (*)[n]"));
    assert!(variably_modified("int[4][n + 1]"));
    assert!(variably_modified("int[4"));
}

/// `castTable` = `CAST_TABLE`, row for row.
#[test]
fn cg08_lean_cast_table_matches_the_rust_table() {
    let rows = block("def castTable : List (String × Preservation × Provenance × Admission) := [")
        .lines()
        .filter(|line| line.trim_start().starts_with('('))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), CAST_TABLE.len());
    for (row, rust) in rows.iter().zip(CAST_TABLE) {
        let fields = row
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(',')
            .trim_end_matches(')')
            .splitn(4, ", ")
            .collect::<Vec<_>>();
        assert_eq!(quoted(fields[0]), vec![rust.kind], "{row}");
        let preservation = match fields[1] {
            ".preserving" => Preservation::Preserving,
            ".possiblyLossy" => Preservation::PossiblyLossy,
            other => panic!("{other}"),
        };
        let provenance = match fields[2] {
            ".kept" => Provenance::Kept,
            ".lost" => Provenance::Lost,
            ".notApplicable" => Provenance::NotApplicable,
            other => panic!("{other}"),
        };
        let admission = match fields[3].trim() {
            ".scalar" => Admission::Scalar,
            ".nullTest" => Admission::NullTest,
            ".pointerToInteger" => Admission::PointerToInteger,
            ".integerToPointer" => Admission::IntegerToPointer,
            ".pointerValue" => Admission::PointerValue,
            ".nullPointer" => Admission::NullPointer,
            ".discard" => Admission::Discard,
            refused => {
                let card = quoted(refused)[0];
                assert!(
                    matches!(rust.admission, Admission::Refused(owner) if owner == card),
                    "{row}"
                );
                rust.admission
            }
        };
        assert_eq!(
            (preservation, provenance, admission),
            (rust.preservation, rust.provenance, rust.admission),
            "{row}"
        );
    }
    // An unlisted kind has no row: the elaborator refuses it by name.
    for unlisted in [
        "LValueToRValue",
        "FunctionToPointerDecay",
        "VectorSplat",
        "FixedPointCast",
    ] {
        assert!(representation(unlisted).is_none(), "{unlisted}");
    }
}

/// The first `UnaryExprOrTypeTraitExpr` of a fixture's `subject`.
fn first_query(source: &str) -> Value {
    fn find(node: &Value) -> Option<&Value> {
        if kind(node) == "UnaryExprOrTypeTraitExpr" {
            return Some(node);
        }
        children(node).iter().find_map(find)
    }
    let function = crate::typed_c_ast::function_ast(
        source,
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
    )
    .unwrap_or_else(|error| panic!("Clang rejected the fixture: {error}\n{source}"));
    find(&function).expect("a type query").clone()
}

/// `queryVectors` through Clang and `query_operand`.
#[test]
fn cg08_lean_query_vectors_match_query_operand() {
    if !clang_available() {
        return;
    }
    let mut count = 0;
    for row in block("def queryVectors : List (String × QueryClass) := [")
        .split("\n  (\"")
        .skip(1)
    {
        let (source, rest) = row.split_once('"').expect("a source");
        let node = first_query(source);
        let class = match query_operand(&node) {
            QueryOperand::Unevaluated => ".unevaluated",
            QueryOperand::Evaluated(_) => ".evaluated",
            QueryOperand::Refused(_) => ".refused",
        };
        assert!(rest.contains(class), "{source}: {class}");
        count += 1;
    }
    assert_eq!(count, 8);
}

/// [`clang_constant`] over `prelude`, or a panic naming the expression.
fn evaluated(prelude: &str, expression: &str) -> i128 {
    let unit = C11TranslationUnit {
        source: prelude,
        ..Default::default()
    };
    clang_constant(prelude, &unit, expression)
        .unwrap_or_else(|error| panic!("Clang rejected {expression}: {error}"))
}

/// A non-constant expression (a variably modified `sizeof`) is no constant:
/// the evaluator refuses rather than guess.
#[test]
fn a_non_constant_query_has_no_clang_constant() {
    if !clang_available() {
        return;
    }
    let unit = C11TranslationUnit::default();
    assert!(clang_constant("int n;", &unit, "sizeof(int[n])").is_err());
    assert!(clang_constant("", &unit, "undeclared_name").is_err());
}

/// `constantVectors`: each value is Clang's evaluation (x86-64 Linux, where
/// the vectors were read); a consumer of a query's number takes exactly it.
#[test]
fn cg08_lean_constant_vectors_are_clangs_constants() {
    if !clang_available() || !cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        return;
    }
    let prelude = LEAN
        .split("def constantPrelude : String :=\n  \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("constantPrelude");
    let mut count = 0;
    for line in block("def constantVectors : List (String × Nat) := [").lines() {
        let line = line.trim();
        if !line.starts_with('(') {
            continue;
        }
        let expression = quoted(line)[0];
        let expected: i128 = line
            .rsplit(", ")
            .next()
            .unwrap()
            .trim_end_matches(',')
            .trim_end_matches(')')
            .parse()
            .unwrap();
        assert_eq!(evaluated(prelude, expression), expected, "{expression}");
        count += 1;
    }
    assert_eq!(count, 8);
}

#[test]
fn only_the_gate_kinds_open_a_consultation() {
    let cast = |kind: &str| serde_json::json!({"kind": "ImplicitCastExpr", "castKind": kind});
    for gate in GATE_KINDS {
        assert!(form(&cast(gate)).is_some(), "{gate}");
        assert!(representation(gate).is_some(), "{gate}");
    }
    for quiet in [
        "LValueToRValue",
        "NoOp",
        "BitCast",
        "IntegralCast",
        "NullToPointer",
    ] {
        assert!(form(&cast(quiet)).is_none(), "{quiet}");
    }
    assert!(form(&serde_json::json!({"kind": "CStyleCastExpr", "castKind": "BitCast"})).is_some());
    assert!(form(&serde_json::json!({"kind": "UnaryExprOrTypeTraitExpr"})).is_some());
}
