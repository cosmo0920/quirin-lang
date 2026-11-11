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

//! Card CELL-GUARD-RESULT-RANGE-CHECKED: the result-set vectors hold in Rust
//! and are, entry for entry, the vectors of `proof/QCore/ResultSet.lean`
//! (`setVectors`, `memberVectors`, `classVectors`), whose theorems
//! (`set_vectors_hold`, `member_vectors_hold`, `class_vectors_hold`) check
//! them against the Lean model.

use qcore_core::result_set::{
    Bound, CLASS_VECTORS, Class, Cmp, INT, MEMBER_VECTORS, ResultSet, SET_VECTORS, SetExpr,
    comparison_set, narrowing_never_undecides, parse_text, test_class,
};

const LEAN: &str = include_str!("../../../proof/QCore/ResultSet.lean");

fn lean_int(value: i128) -> String {
    if value < 0 {
        format!("({value})")
    } else {
        value.to_string()
    }
}

fn lean_bound(bound: Bound) -> String {
    match bound {
        Bound::Literal(value) => format!("(.lit {})", lean_int(value)),
        Bound::Min => ".min".into(),
        Bound::Max => ".max".into(),
    }
}

fn lean_expr(expr: &SetExpr) -> String {
    match expr {
        SetExpr::Value(bound) => format!(".val {}", lean_bound(*bound)),
        SetExpr::Interval(lo, hi) => format!(".ival {} {}", lean_bound(*lo), lean_bound(*hi)),
        SetExpr::Or(a, b) => format!(".or ({}) ({})", lean_expr(a), lean_expr(b)),
        SetExpr::And(a, b) => format!(".and ({}) ({})", lean_expr(a), lean_expr(b)),
        SetExpr::Xor(a, b) => format!(".xor ({}) ({})", lean_expr(a), lean_expr(b)),
    }
}

fn lean_intervals(set: &ResultSet) -> String {
    let parts = set
        .intervals()
        .iter()
        .map(|(lo, hi)| format!("({lo}, {hi})"))
        .collect::<Vec<_>>();
    format!("[{}]", parts.join(", "))
}

fn lean_cmp(cmp: Cmp) -> &'static str {
    match cmp {
        Cmp::Eq => ".eq",
        Cmp::Ne => ".ne",
        Cmp::Lt => ".lt",
        Cmp::Le => ".le",
        Cmp::Gt => ".gt",
        Cmp::Ge => ".ge",
    }
}

fn lean_class(class: Class) -> &'static str {
    match class {
        Class::Holds => ".holds",
        Class::Fails => ".fails",
        Class::Undecided => ".undecided",
    }
}

/// The rows of a Lean `def NAME … := [ … ]` table, one per line.
fn lean_rows(name: &str) -> Vec<String> {
    let start = LEAN
        .find(&format!("def {name} "))
        .unwrap_or_else(|| panic!("no Lean table {name}"));
    let body = &LEAN[start..];
    let open = body.find(":= [").expect("table opener") + 4;
    let close = body[open..].find("\n]").expect("table closer") + open;
    body[open..close]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

#[test]
fn set_vectors_hold_and_match_lean() {
    let mut rows = Vec::new();
    for (written, expected) in SET_VECTORS {
        let expr = parse_text(written).unwrap_or_else(|e| panic!("{written}: {e}"));
        match expr.eval(INT) {
            Ok(set) => {
                assert_eq!(set.canonical(), *expected, "{written}");
                rows.push(format!(
                    "({}, some {})",
                    lean_expr(&expr),
                    lean_intervals(&set)
                ));
            }
            Err(error) => {
                assert!(
                    error.to_string().starts_with(expected),
                    "{written}: {error} (expected {expected})"
                );
                rows.push(format!("({}, none)", lean_expr(&expr)));
            }
        }
    }
    assert_eq!(rows, lean_rows("setVectors"));
}

#[test]
fn member_vectors_hold_and_match_lean() {
    let mut rows = Vec::new();
    for (written, value, member) in MEMBER_VECTORS {
        let expr = parse_text(written).unwrap();
        let set = expr.eval(INT).unwrap();
        assert_eq!(set.contains(*value), *member, "{written} {value}");
        rows.push(format!("({}, {}, {member})", lean_expr(&expr), value));
    }
    assert_eq!(rows, lean_rows("memberVectors"));
}

#[test]
fn class_vectors_hold_and_match_lean() {
    let mut rows = Vec::new();
    for (within, cmp, k, class, expected) in CLASS_VECTORS {
        let (known, known_lean) = if within.is_empty() {
            (ResultSet::full(INT), "none".to_owned())
        } else {
            let expr = parse_text(within).unwrap();
            (
                expr.eval(INT).unwrap(),
                format!("some ({})", lean_expr(&expr)),
            )
        };
        let class_expr = parse_text(class).unwrap();
        let class_set = class_expr.eval(INT).unwrap();
        let test = comparison_set(INT, *cmp, *k);
        assert_eq!(
            test_class(&known, &test, &class_set),
            *expected,
            "{within} {cmp:?} {k} {class}"
        );
        rows.push(format!(
            "({known_lean}, {}, {k}, {}, {})",
            lean_cmp(*cmp),
            lean_expr(&class_expr),
            lean_class(*expected)
        ));
    }
    assert_eq!(rows, lean_rows("classVectors"));
}

/// A narrower known set never takes a decision away (Lean
/// `narrowing_keeps_holds`, `narrowing_keeps_fails`), over every vector's
/// test and class and a family of sets.
#[test]
fn narrowing_never_takes_a_decision_away() {
    let sets = [
        "[MIN, MAX]",
        "-1 | [0, MAX]",
        "-1 | 0",
        "0",
        "-1",
        "[-5, 5]",
    ]
    .map(|text| parse_text(text).unwrap().eval(INT).unwrap());
    for (_, cmp, k, class, _) in CLASS_VECTORS {
        let class = parse_text(class).unwrap().eval(INT).unwrap();
        let test = comparison_set(INT, *cmp, *k);
        for within in &sets {
            for narrower in &sets {
                assert!(narrowing_never_undecides(within, narrower, &test, &class));
            }
        }
    }
}

/// The parser follows C's precedence (`&` over `^` over `|`), left
/// associative, and refuses anything outside the grammar.
#[test]
fn precedence_and_refusals() {
    let a = parse_text("1 | 2 ^ 3 & 4").unwrap();
    assert_eq!(a.to_string(), "1 | 2 ^ 3 & 4");
    assert_eq!(
        lean_expr(&a),
        ".or (.val (.lit 1)) (.xor (.val (.lit 2)) (.and (.val (.lit 3)) (.val (.lit 4))))"
    );
    let b = parse_text("(1 | 2) & 2").unwrap();
    assert_eq!(b.to_string(), "(1 | 2) & 2");
    for refused in [
        "", "1 |", "[1, 2", "[1 2]", "01", "-0", "x", "1 || 2", "1 ∪ 2", "(1", "1)",
    ] {
        assert!(parse_text(refused).is_err(), "{refused}");
    }
}
