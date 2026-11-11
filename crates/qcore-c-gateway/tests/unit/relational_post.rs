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

//! Card CG23-RELATIONAL-POSTCONDITIONS (rule 11r): the decision tables and
//! their mechanical binding to `proof/QCore/RelationalPost.lean`.

use super::*;
use qcore_schemas::contract::{
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};

/// The route-G token spelling: punctuation one character per token.
fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        if word.chars().all(|ch| ch.is_ascii_punctuation()) {
            out.extend(word.chars().map(String::from));
        } else {
            out.push(word.to_owned());
        }
    }
    out
}

fn op(code: &str) -> RelationOp {
    RelationOp::from_spelling(code).expect("an operator code")
}

fn disposition(code: &str) -> ResourceFunctionContract {
    let mode = match code {
        "a" => ResourceParameterMode::Consumed { nullable: false },
        "b" => ResourceParameterMode::Owned { nullable: false },
        other => panic!("unknown disposition {other}"),
    };
    ResourceFunctionContract::new([mode], ResourceReturnMode::None)
}

fn classes(code: &str) -> RelationalContract {
    let class = |op_code: &str, k: i64, d: &str| ExitClass {
        relation: ReturnRelation::new(op(op_code), k),
        contract: disposition(d),
    };
    let (classes, origin) = match code {
        "eq0/ne0" => (
            vec![class("eq", 0, "a"), class("ne", 0, "b")],
            RelationalOrigin::Declared,
        ),
        "lt0/ge0" => (
            vec![class("lt", 0, "a"), class("ge", 0, "b")],
            RelationalOrigin::Declared,
        ),
        "eq0/eqm1" => (
            vec![class("eq", 0, "a"), class("eq", -1, "b")],
            RelationalOrigin::InferredValidated,
        ),
        other => panic!("unknown classes {other}"),
    };
    RelationalContract {
        origin,
        classes,
        withdrawn: None,
    }
}

#[test]
fn the_meets_vectors_hold() {
    for (left, a, right, b, expected) in MEETS_VECTORS {
        let left = ReturnRelation::new(op(left), *a);
        let right = ReturnRelation::new(op(right), *b);
        assert_eq!(left.meets(right), *expected, "{left:?} {right:?}");
        assert_eq!(
            right.meets(left),
            *expected,
            "symmetric: {left:?} {right:?}"
        );
    }
}

#[test]
fn the_select_vectors_hold() {
    for (code, edge_op, k, expected) in SELECT_VECTORS {
        let relational = classes(code);
        let selected = relational.select(ReturnRelation::new(op(edge_op), *k));
        match *expected {
            "none" => assert_eq!(selected, None, "{code} {edge_op} {k}"),
            d => assert_eq!(selected, Some(&disposition(d)), "{code} {edge_op} {k}"),
        }
    }
}

#[test]
fn the_declared_vector_classes_partition_and_the_inferred_ones_do_not_claim_totality() {
    assert!(classes("eq0/ne0").disjoint() && classes("eq0/ne0").total());
    assert!(classes("lt0/ge0").disjoint() && classes("lt0/ge0").total());
    assert!(classes("eq0/eqm1").disjoint() && !classes("eq0/eqm1").total());
    assert_eq!(classes("eq0/ne0").join(), None);
}

#[test]
fn the_test_vectors_hold() {
    for (condition, edge_op, k) in TEST_VECTORS {
        let owned = tokens(condition);
        let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
        let test = read_test(&texts);
        match *edge_op {
            "-" => assert_eq!(test, None, "{condition}"),
            code => {
                let test = test.unwrap_or_else(|| panic!("{condition} is read"));
                assert_eq!(
                    test.when_true,
                    ReturnRelation::new(op(code), *k),
                    "{condition}"
                );
                assert_eq!(texts[test.callee], "f", "{condition}");
                assert_eq!(texts[test.close], ")", "{condition}");
                assert_eq!(
                    test.assigned.map(|index| texts[index]),
                    condition.contains(" r = ").then_some("r"),
                    "{condition}"
                );
            }
        }
    }
}

#[test]
fn a_withdrawn_disposition_selects_nothing() {
    let mut relational = classes("eq0/ne0");
    relational.withdrawn = Some("contradicted".to_owned());
    for code in ["eq", "ne", "lt", "ge"] {
        assert_eq!(relational.select(ReturnRelation::new(op(code), 0)), None);
    }
    assert_eq!(relational.join(), None);
}

#[test]
fn class_names_are_reserved() {
    assert!(class_name(1, "take").starts_with(CLASS_PREFIX));
    assert!(untested_description("take", None).contains(UNTESTED));
    assert!(untested_description("take", Some("x")).contains(WITHDRAWN));
}

const LEAN: &str = include_str!("../../../../proof/QCore/RelationalPost.lean");

fn lean_rows(header: &str) -> Vec<Vec<String>> {
    let table = LEAN.split(header).nth(1).expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|row| {
            row.trim_end_matches(',')
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(|cell| {
                    cell.trim()
                        .trim_start_matches('.')
                        .replace(['(', ')', ' '], "")
                })
                .collect()
        })
        .collect()
}

#[test]
fn cg23_lean_meets_vectors_match_the_rust_table() {
    let rows = lean_rows("def meetsVectors : List (Op × Int × Op × Int × Bool) := [");
    let rust = MEETS_VECTORS
        .iter()
        .map(|(a, k, b, l, m)| {
            vec![
                (*a).to_owned(),
                k.to_string(),
                (*b).to_owned(),
                l.to_string(),
                m.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn cg23_lean_select_vectors_match_the_rust_table() {
    let rows = lean_rows("def selectVectors : List (Fixture × Op × Int × Option Disp) := [");
    let rust = SELECT_VECTORS
        .iter()
        .map(|(classes, edge, k, selected)| {
            let fixture = match *classes {
                "eq0/ne0" => "eqZeroNeZero",
                "lt0/ge0" => "ltZeroGeZero",
                "eq0/eqm1" => "eqZeroEqMinusOne",
                other => panic!("{other}"),
            };
            let selected = match *selected {
                "none" => "none".to_owned(),
                d => format!("some.{d}"),
            };
            vec![
                fixture.to_owned(),
                (*edge).to_owned(),
                k.to_string(),
                selected,
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}
