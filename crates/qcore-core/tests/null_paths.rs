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

//! Card NULL-TRIAGE-UNRESOLVED-PATHS: the Rust decisions of
//! `qcore_core::null_paths` against the shared vectors of
//! `proof/QCore/NullTriagePaths.lean` (`exploreVectors`, `concludeVectors`,
//! `armsVectors`), read from the Lean text so neither side can change alone,
//! plus the Rust-only controls.

use qcore_core::null_paths::{
    Explored, Reading, conclude, counter_lower_bound, explore, relation_arms, removes,
};
use qcore_core::null_triage::{FindingClass, Triage};

const LEAN: &str = include_str!("../../../proof/QCore/NullTriagePaths.lean");

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

const EXPLORE_VECTORS: [(&str, bool, &str); 7] = [
    ("d1", true, "taken"),
    ("d1", false, "infeasible"),
    ("d0", false, "taken"),
    ("exact", true, "taken"),
    ("exact", false, "taken"),
    ("inexact", true, "taken-inexact"),
    ("inexact", false, "taken-inexact"),
];

const CONCLUDE_VECTORS: [(bool, bool, bool, &str); 5] = [
    (true, true, false, "finding"),
    (true, false, true, "finding"),
    (false, true, true, "unresolved"),
    (false, false, false, "unresolved"),
    (false, false, true, "infeasible"),
];

const ARMS_VECTORS: [(i128, i128, &str, i128, bool, bool); 8] = [
    (0, 10, "<", 0, false, true),
    (-5, 10, "<", 0, true, true),
    (3, 3, "==", 3, true, false),
    (3, 3, "!=", 3, false, true),
    (0, 10, "==", 11, false, true),
    (0, 10, ">", 10, false, true),
    (0, 10, "<=", 10, true, false),
    (1, 10, ">=", 1, true, false),
];

fn reading(name: &str) -> Reading {
    match name {
        "d1" => Reading::Decided(true),
        "d0" => Reading::Decided(false),
        "exact" => Reading::Exact,
        _ => Reading::Inexact,
    }
}

#[test]
fn the_vectors_are_the_leans() {
    let rows = EXPLORE_VECTORS
        .iter()
        .map(|(r, a, o)| format!("(\"{r}\", {a}, \"{o}\")"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def exploreVectors : List (String × Bool × String) := ["),
        rows
    );
    let rows = CONCLUDE_VECTORS
        .iter()
        .map(|(e, i, c, o)| format!("({e}, {i}, {c}, \"{o}\")"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def concludeVectors : List (Bool × Bool × Bool × String) := ["),
        rows
    );
    let rows = ARMS_VECTORS
        .iter()
        .map(|(lo, hi, op, c, t, f)| format!("({lo}, {hi}, \"{op}\", {c}, {t}, {f})"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def armsVectors : List (Int × Int × String × Int × Bool × Bool) := ["),
        rows
    );
}

#[test]
fn the_rust_decisions_agree_with_the_vectors() {
    for (r, arm, out) in EXPLORE_VECTORS {
        assert_eq!(explore(reading(r), arm).as_str(), out, "{r} {arm}");
    }
    for (exact_reach, inexact_reach, complete, out) in CONCLUDE_VECTORS {
        let explored = Explored {
            exact_reach,
            inexact_reach,
            complete,
        };
        assert_eq!(conclude(explored), out, "{explored:?}");
    }
    // The Lean vectors state the arms over the interval `[lo, hi]` itself:
    // a difference `x - y` with `y` the single value 0 is that interval.
    for (lo, hi, op, c, t, f) in ARMS_VECTORS {
        assert_eq!(
            relation_arms((lo, hi), (0, 0), false, op, c),
            Some((t, f)),
            "{lo} {hi} {op} {c}"
        );
        assert_eq!(
            relation_arms((lo, hi), (0, 0), true, op, c),
            Some((t, f)),
            "{lo} {hi} {op} {c} (sum)"
        );
    }
}

#[test]
fn a_difference_reads_both_intervals() {
    // x in [0, 255], y in [1001, 2000]: x - y < 0 always.
    assert_eq!(
        relation_arms((0, 255), (1001, 2000), false, ">", 0),
        Some((false, true))
    );
    // x in [0, 10], y in [0, 10]: both arms of x < y.
    assert_eq!(
        relation_arms((0, 10), (0, 10), false, "<", 0),
        Some((true, true))
    );
    // a + b > 0 with a, b in [0, 5]: both.
    assert_eq!(
        relation_arms((0, 5), (0, 5), true, ">", 0),
        Some((true, true))
    );
    // An empty interval or an unread operator is no reading.
    assert_eq!(relation_arms((1, 0), (0, 0), false, "<", 0), None);
    assert_eq!(relation_arms((0, 1), (0, 0), false, "&", 0), None);
}

#[test]
fn a_counter_keeps_its_least_start() {
    assert_eq!(counter_lower_bound(&[0], &[1]), Some(0));
    assert_eq!(counter_lower_bound(&[3, 1], &[2, 1]), Some(1));
    assert_eq!(counter_lower_bound(&[0], &[1, -1]), None);
    assert_eq!(counter_lower_bound(&[], &[1]), None);
}

#[test]
fn only_an_infeasible_triage_removes_its_candidate() {
    assert!(removes(&Triage::Infeasible));
    assert!(!removes(&Triage::Unresolved));
    assert!(!removes(&Triage::Indeterminate));
    assert!(!removes(&Triage::Finding(FindingClass::NormalPath)));
}
