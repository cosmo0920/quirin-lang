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

//! Card NULL-SAFETY-PROPERTY: the Rust decisions of
//! `qcore_core::null_deref` against the shared vectors of
//! `proof/QCore/NullDeref.lean` (`joinVectors`, `checkVectors`,
//! `verdictVectors`, `firstVectors`, `inferredVectors`), read from the Lean
//! text so neither side can change alone, plus the Rust-only controls.

use std::collections::BTreeSet;

use qcore_core::null_deref::{
    First, NullVerdict, Nullness, Outcome, Proof, Request, Source, after_dereference, check,
    clause_standing, compose, inferred, null_verdict, offset, premise_name, refine,
};

const LEAN: &str = include_str!("../../../proof/QCore/NullDeref.lean");

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

fn of_class(class: u8) -> Nullness {
    match class {
        0 => Nullness::proved(Proof::Tested(1)),
        1 => Nullness::Null,
        _ => Nullness::maybe(Source::Formal("p".into())),
    }
}

fn class_of(value: &Nullness) -> u8 {
    match value {
        Nullness::NonNull(_) => 0,
        Nullness::Null => 1,
        Nullness::Maybe(_) => 2,
    }
}

const JOIN_VECTORS: [(u8, u8, u8); 9] = [
    (0, 0, 0),
    (0, 1, 2),
    (0, 2, 2),
    (1, 0, 2),
    (1, 1, 1),
    (1, 2, 2),
    (2, 0, 2),
    (2, 1, 2),
    (2, 2, 2),
];

const CHECK_VECTORS: [(u8, bool, &str); 6] = [
    (0, true, "proved"),
    (0, false, "proved"),
    (1, true, "violation"),
    (1, false, "violation-reachability-unresolved"),
    (2, true, "candidate"),
    (2, false, "candidate"),
];

const VERDICT_VECTORS: [(&[&str], bool, &str); 8] = [
    (&[], false, "Safe"),
    (&["proved", "moved", "excluded"], false, "Safe"),
    (&["proved"], true, "Unknown"),
    (&["proved", "candidate"], false, "Unknown"),
    (&["held"], false, "Pending"),
    (&["held", "candidate"], false, "Unknown"),
    (&["violation-reachability-unresolved"], false, "Unknown"),
    (&["candidate", "violation"], false, "Unsafe"),
];

const FIRST_VECTORS: [(&[&str], &str); 5] = [
    (&[], ".pending"),
    (&["deref"], ".dereferenced"),
    (&["deref", "test"], ".dereferenced"),
    (&["test", "deref"], ".other"),
    (&["test"], ".other"),
];

const INFERRED_VECTORS: [(&[&str], bool); 4] = [
    (&[], false),
    (&[".dereferenced"], true),
    (&[".dereferenced", ".other"], false),
    (&[".pending"], false),
];

fn outcome(name: &str) -> Outcome {
    match name {
        "proved" => Outcome::Proved(BTreeSet::from([Proof::Address])),
        "moved" => Outcome::Moved(BTreeSet::from([Proof::Moved(0)])),
        "held" => Outcome::Held("clause".into()),
        "candidate" => Outcome::Candidate(BTreeSet::from([Source::NullLiteral])),
        "violation" => Outcome::Violation { reachable: true },
        "violation-reachability-unresolved" => Outcome::Violation { reachable: false },
        _ => Outcome::Excluded,
    }
}

fn first(name: &str) -> First {
    match name {
        ".dereferenced" => First::Dereferenced,
        ".other" => First::Other,
        _ => First::Pending,
    }
}

fn first_name(value: First) -> &'static str {
    match value {
        First::Pending => ".pending",
        First::Dereferenced => ".dereferenced",
        First::Other => ".other",
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

#[test]
fn the_vectors_are_the_lean_vectors() {
    assert_eq!(
        lean_rows("def joinVectors : List (Nat × Nat × Nat) := ["),
        JOIN_VECTORS
            .iter()
            .map(|(a, b, j)| format!("({a}, {b}, {j})"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        lean_rows("def checkVectors : List (Nat × Bool × String) := ["),
        CHECK_VECTORS
            .iter()
            .map(|(c, r, o)| format!("({c}, {r}, \"{o}\")"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        lean_rows("def verdictVectors : List (List String × Bool × String) := ["),
        VERDICT_VECTORS
            .iter()
            .map(|(os, r, v)| format!("({}, {r}, \"{v}\")", quoted(os)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        lean_rows("def firstVectors : List (List String × First) := ["),
        FIRST_VECTORS
            .iter()
            .map(|(es, f)| format!("({}, {f})", quoted(es)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        lean_rows("def inferredVectors : List (List First × Bool) := ["),
        INFERRED_VECTORS
            .iter()
            .map(|(es, i)| format!("([{}], {i})", es.join(", ")))
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_vectors_hold_for_the_rust_decisions() {
    for (a, b, joined) in JOIN_VECTORS {
        assert_eq!(class_of(&of_class(a).join(&of_class(b))), joined, "{a} {b}");
    }
    for (class, reachable, name) in CHECK_VECTORS {
        assert_eq!(check(&of_class(class), reachable).class(), name);
    }
    for (outcomes, reasons, verdict) in VERDICT_VECTORS {
        let outcomes = outcomes.iter().map(|o| outcome(o)).collect::<Vec<_>>();
        let reasons = if reasons {
            vec!["null-unsupported-form: X".to_owned()]
        } else {
            Vec::new()
        };
        assert_eq!(null_verdict(&outcomes, &reasons).0.as_str(), verdict);
    }
    for (events, expected) in FIRST_VECTORS {
        let value = events.iter().fold(First::Pending, |f, e| {
            if *e == "deref" {
                f.dereference()
            } else {
                f.test()
            }
        });
        assert_eq!(first_name(value), expected, "{events:?}");
    }
    for (exits, expected) in INFERRED_VECTORS {
        let exits = exits.iter().map(|e| first(e)).collect::<BTreeSet<_>>();
        assert_eq!(inferred(&exits), expected, "{exits:?}");
    }
}

#[test]
fn the_dominating_dereference_moves_the_question_until_reassignment() {
    let maybe = Nullness::maybe(Source::MemoryField("s->next".into()));
    let after = after_dereference(&maybe, 7);
    assert_eq!(after, Nullness::proved(Proof::Moved(7)));
    assert_eq!(check(&after, true).class(), "moved");
    // An offset keeps the moved question; a null offset is no value.
    assert_eq!(offset(&after), after);
    assert_eq!(
        offset(&Nullness::Null),
        Nullness::maybe(Source::NullLiteral)
    );
    // A proved pointer keeps its proofs through a dereference.
    let tested = Nullness::proved(Proof::Tested(3));
    assert_eq!(after_dereference(&tested, 9), tested);
}

#[test]
fn a_guard_proves_its_arm_and_contradicts_the_other() {
    let maybe = Nullness::maybe(Source::Formal("p".into()));
    assert_eq!(
        refine(&maybe, true, Proof::Tested(4)),
        Some(Nullness::proved(Proof::Tested(4)))
    );
    assert_eq!(
        refine(&maybe, false, Proof::Tested(4)),
        Some(Nullness::Null)
    );
    let proved = Nullness::proved(Proof::Address);
    assert_eq!(refine(&proved, false, Proof::Tested(4)), None);
    assert_eq!(refine(&Nullness::Null, true, Proof::Tested(4)), None);
}

#[test]
fn the_composition_is_safe_only_when_the_null_row_is() {
    let both = Request {
        custody: true,
        null: true,
    };
    assert_eq!(
        compose(both, Some(NullVerdict::Safe), Some(NullVerdict::Unknown)),
        NullVerdict::Unknown
    );
    assert_eq!(
        compose(both, Some(NullVerdict::Safe), Some(NullVerdict::Safe)),
        NullVerdict::Safe
    );
    assert_eq!(
        compose(both, Some(NullVerdict::Safe), None),
        NullVerdict::Unknown
    );
    assert_eq!(premise_name("peek", 380, "iv"), "null-deref:peek:380:iv");
    let tested_first = BTreeSet::from([First::Other]);
    assert_eq!(clause_standing(&tested_first), "stronger-than-necessary");
    assert_eq!(
        clause_standing(&BTreeSet::from([First::Dereferenced])),
        "necessary"
    );
}
