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

//! Card NULL-DEREF-QTT-GROUNDING: the Rust decisions of
//! `qcore_core::null_deref` (`First`, `inferred`, `clause_standing`) against
//! the usage grades of `qcore_core::usage` (language specification §L07), on
//! the shared vectors of `proof/QCore/NullDerefGrade.lean` (`gradeVectors`),
//! read from the Lean text so neither side can change alone, and exhaustively
//! on short paths. No decision changes: this binds the grounding to the code.

use std::collections::BTreeSet;

use qcore_core::null_deref::{First, clause_standing, inferred};
use qcore_core::usage::Usage;

const LEAN: &str = include_str!("../../../proof/QCore/NullDerefGrade.lean");

/// The rows of the Lean list that opens with `header`, one per line.
fn lean_rows(header: &str) -> Vec<String> {
    let start = LEAN.find(header).expect("the Lean vector list") + header.len();
    let body = &LEAN[start..];
    let end = body.find("\n]").expect("the list's end");
    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

/// Exit `First`s, the joined grade's lower bound, `relevant`'s admission of
/// it, the standing: the Lean's `gradeVectors`.
const GRADE_VECTORS: [(&[&str], u64, bool, &str); 7] = [
    (&["dereferenced"], 1, true, "necessary"),
    (&["dereferenced", "dereferenced"], 1, true, "necessary"),
    (&["pending"], 0, false, "stronger-than-necessary"),
    (&["other"], 0, false, "stronger-than-necessary"),
    (
        &["dereferenced", "other"],
        0,
        false,
        "stronger-than-necessary",
    ),
    (
        &["dereferenced", "pending"],
        0,
        false,
        "stronger-than-necessary",
    ),
    (&["other", "pending"], 0, false, "stronger-than-necessary"),
];

fn first(name: &str) -> First {
    match name {
        "dereferenced" => First::Dereferenced,
        "other" => First::Other,
        _ => First::Pending,
    }
}

/// The Lean's `firstGrade`: a path that dereferenced first demands `one`.
fn first_grade(value: First) -> Usage {
    match value {
        First::Dereferenced => Usage::ONE,
        First::Pending | First::Other => Usage::ZERO,
    }
}

fn quoted(names: &[&str]) -> String {
    format!(
        "[{}]",
        names
            .iter()
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The least enclosing interval of the exits' grades (§L07's branch join).
fn joined(exits: &BTreeSet<First>) -> Option<Usage> {
    exits.iter().map(|f| first_grade(*f)).reduce(Usage::join)
}

#[test]
fn the_grade_vectors_are_the_lean_vectors() {
    assert_eq!(
        lean_rows("def gradeVectors : List (List String × Nat × Bool × String) := ["),
        GRADE_VECTORS
            .iter()
            .map(|(es, lower, admitted, name)| format!(
                "({}, {lower}, {admitted}, \"{name}\")",
                quoted(es)
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_grade_vectors_hold_for_the_rust_decisions() {
    for (exits, lower, admitted, name) in GRADE_VECTORS {
        let exits = exits.iter().map(|e| first(e)).collect::<BTreeSet<_>>();
        let grade = joined(&exits).expect("a non-empty exit set");
        assert_eq!(grade.lower(), lower, "{exits:?}");
        assert_eq!(Usage::RELEVANT.contains(grade), admitted, "{exits:?}");
        assert_eq!(inferred(&exits), admitted, "{exits:?}");
        assert_eq!(clause_standing(&exits), name, "{exits:?}");
    }
    // No terminating path: no grade, no clause.
    assert_eq!(joined(&BTreeSet::new()), None);
    assert!(!inferred(&BTreeSet::new()));
}

/// Every path of at most `length` events over `deref` / `test`.
fn paths(length: usize) -> Vec<Vec<bool>> {
    let mut out = vec![Vec::new()];
    let mut frontier = vec![Vec::new()];
    for _ in 0..length {
        let mut next = Vec::new();
        for path in &frontier {
            for event in [true, false] {
                let mut longer: Vec<bool> = path.clone();
                longer.push(event);
                next.push(longer);
            }
        }
        out.extend(next.iter().cloned());
        frontier = next;
    }
    out
}

/// The Lean's `pathGrade` on a path of `deref` (`true`) / `test` (`false`)
/// events: one per dereference before the first test, by §L07's sequential
/// addition.
fn path_grade(path: &[bool]) -> Usage {
    let mut grade = Usage::ZERO;
    for event in path {
        if !*event {
            break;
        }
        grade = grade.checked_add(Usage::ONE).expect("a short path");
    }
    grade
}

/// The walk's `First` transitions over the same events.
fn path_first(path: &[bool]) -> First {
    path.iter().fold(First::Pending, |f, event| {
        if *event { f.dereference() } else { f.test() }
    })
}

#[test]
fn a_path_requires_the_identity_exactly_when_it_dereferences_first() {
    for path in paths(5) {
        let grade = path_grade(&path);
        assert_eq!(
            Usage::RELEVANT.contains(grade),
            path_first(&path) == First::Dereferenced,
            "{path:?}"
        );
        assert_eq!(
            grade.lower() >= 1,
            first_grade(path_first(&path)).lower() >= 1,
            "{path:?}"
        );
    }
}

#[test]
fn the_grade_join_of_two_arms_is_the_walks_union() {
    let all = paths(3);
    for left in &all {
        for right in &all {
            let grade = path_grade(left).join(path_grade(right));
            let exits = BTreeSet::from([path_first(left), path_first(right)]);
            assert_eq!(
                Usage::RELEVANT.contains(grade),
                inferred(&exits),
                "{left:?} {right:?}"
            );
            assert_eq!(
                clause_standing(&exits) == "necessary",
                Usage::RELEVANT.contains(grade),
                "{left:?} {right:?}"
            );
        }
    }
}

#[test]
fn a_lower_bound_of_zero_is_admitted_by_many_and_refused_by_relevant() {
    for path in paths(4) {
        let grade = path_grade(&path);
        assert!(Usage::MANY.contains(grade), "{path:?}");
        if grade.lower() == 0 {
            assert!(!Usage::RELEVANT.contains(grade), "{path:?}");
        }
    }
}
