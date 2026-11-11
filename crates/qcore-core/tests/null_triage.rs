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

//! Card NULL-CANDIDATE-TRIAGE: the Rust decisions of
//! `qcore_core::null_triage` against the shared vectors of
//! `proof/QCore/NullTriage.lean` (`admitVectors`, `memberVectors`,
//! `resultVectors`), read from the Lean text so neither side can change
//! alone, plus the Rust-only controls.

use qcore_core::null_deref::{NullVerdict, Outcome};
use qcore_core::null_triage::{
    Admission, Condition, FindingClass, ResultEvidence, admit, demote, member_standing,
    result_standing, triaged_verdict, with_travelled_checks,
};

const LEAN: &str = include_str!("../../../proof/QCore/NullTriage.lean");

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

const ADMIT_VECTORS: [(&str, bool, &str); 8] = [
    ("d1", true, "taken"),
    ("d1", false, "infeasible"),
    ("d0", false, "taken"),
    ("d0", true, "infeasible"),
    ("free", true, "taken"),
    ("free", false, "taken"),
    ("corr", true, "unresolved"),
    ("corr", false, "unresolved"),
];

const MEMBER_VECTORS: [(usize, usize, &str); 4] = [
    (0, 0, "declared-assumption"),
    (3, 0, "partly-validated"),
    (3, 1, "withdrawn"),
    (0, 1, "withdrawn"),
];

const RESULT_VECTORS: [(&[&str], &str); 6] = [
    (&[], "declared-assumption"),
    (&["v"], "validated"),
    (&["v", "v"], "validated"),
    (&["v", "u"], "declared-assumption"),
    (&["v", "r"], "withdrawn"),
    (&["u"], "declared-assumption"),
];

fn condition(name: &str) -> Condition {
    match name {
        "d1" => Condition::Decided(true),
        "d0" => Condition::Decided(false),
        "free" => Condition::Free,
        _ => Condition::Correlated,
    }
}

fn evidence(name: &str) -> ResultEvidence {
    match name {
        "v" => ResultEvidence::Validated,
        "u" => ResultEvidence::Unvalidated,
        _ => ResultEvidence::Refuted,
    }
}

#[test]
fn the_vectors_are_the_leans() {
    let admit_rows = ADMIT_VECTORS
        .iter()
        .map(|(c, a, o)| format!("(\"{c}\", {a}, \"{o}\")"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def admitVectors : List (String × Bool × String) := ["),
        admit_rows
    );
    let member_rows = MEMBER_VECTORS
        .iter()
        .map(|(e, f, s)| format!("({e}, {f}, \"{s}\")"))
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def memberVectors : List (Nat × Nat × String) := ["),
        member_rows
    );
    let result_rows = RESULT_VECTORS
        .iter()
        .map(|(us, s)| {
            format!(
                "([{}], \"{s}\")",
                us.iter()
                    .map(|u| format!("\"{u}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lean_rows("def resultVectors : List (List String × String) := ["),
        result_rows
    );
}

#[test]
fn the_rust_decides_every_vector() {
    for (c, a, o) in ADMIT_VECTORS {
        assert_eq!(admit(condition(c), a).as_str(), o, "{c} {a}");
    }
    for (e, f, s) in MEMBER_VECTORS {
        assert_eq!(member_standing(e, f), s, "{e} {f}");
    }
    for (us, s) in RESULT_VECTORS {
        let units = us.iter().map(|u| evidence(u)).collect::<Vec<_>>();
        assert_eq!(result_standing(&units), s, "{us:?}");
    }
}

#[test]
fn triage_withdrawal_and_travel_never_make_a_row_better() {
    for base in [
        NullVerdict::Safe,
        NullVerdict::Pending,
        NullVerdict::Unknown,
        NullVerdict::Unsafe,
    ] {
        assert_eq!(triaged_verdict(base, 0), base);
        assert_eq!(triaged_verdict(base, 2), NullVerdict::Unsafe);
        assert!(demote(base, true) >= NullVerdict::Unknown);
        assert_eq!(demote(base, false), base);
        assert!(with_travelled_checks(base, &[Outcome::Candidate(Default::default())]) >= base);
        assert_eq!(
            with_travelled_checks(base, &[Outcome::Proved(Default::default())]),
            base
        );
    }
    assert_eq!(admit(Condition::Correlated, true), Admission::Unresolved);
    assert_eq!(
        FindingClass::AllocationFailure.as_str(),
        "null-on-allocation-failure"
    );
    assert_eq!(FindingClass::NormalPath.as_str(), "null-on-normal-path");
}
