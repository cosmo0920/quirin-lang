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

//! Card CONTRACT-SUPERSESSION (contract spec §C06.6): the Rust `decide`
//! against the vectors `proof/QCore/ContractSupersession.lean` decides
//! (`vectors_hold`), parsed out of the Lean file so the two cannot drift, and
//! the decision's set laws on generated candidate lists.

use qcore_schemas::contract_supersession::{
    Candidate, ConflictReason, Decision, StampStanding, decide,
};

const LEAN: &str = include_str!("../../../proof/QCore/ContractSupersession.lean");

/// A Lean candidate `⟨module, text, stamp⟩` as a Rust candidate: the module
/// key `mN`, the text digest `tN` and a stamp on day `N` of 2026-10, or an
/// unusable (missing) stamp for `none`.
fn candidate(module: u32, text: u32, stamp: Option<u32>) -> Candidate {
    Candidate {
        module: format!("m{module}"),
        contract_sha256: format!("t{text}"),
        stamp: stamp.map_or(StampStanding::Missing, |day| {
            StampStanding::Stamped(format!("2026-10-{day:02}T00:00:00Z"))
        }),
    }
}

/// The vectors of the Lean file: `(candidates, Some(text) | None)`.
fn lean_vectors() -> Vec<(Vec<Candidate>, Option<String>)> {
    let body = LEAN
        .split("def vectors : List Vector := [")
        .nth(1)
        .and_then(|rest| rest.split("\n]").next())
        .expect("the Lean vector list");
    let mut vectors = Vec::new();
    for line in body.lines().map(str::trim) {
        if !line.starts_with("⟨[") {
            continue;
        }
        let (cands, expected) = line
            .trim_start_matches("⟨[")
            .split_once("], ")
            .expect("a vector line");
        let mut list = Vec::new();
        for cand in cands.split("⟩, ⟨") {
            let fields = cand
                .trim_matches(|c| c == '⟨' || c == '⟩')
                .split(", ")
                .collect::<Vec<_>>();
            assert_eq!(fields.len(), 3, "{cand}");
            let stamp = match fields[2] {
                "none" => None,
                some => Some(some.trim_start_matches("some ").parse().unwrap()),
            };
            list.push(candidate(
                fields[0].parse().unwrap(),
                fields[1].parse().unwrap(),
                stamp,
            ));
        }
        let expected = expected.trim_end_matches("⟩,").trim_end_matches('⟩');
        let expected = if expected == ".conflict" {
            None
        } else {
            Some(format!(
                "t{}",
                expected.trim_start_matches(".inForce ").trim()
            ))
        };
        vectors.push((list, expected));
    }
    vectors
}

#[test]
fn the_rust_decision_agrees_with_the_lean_vectors() {
    let vectors = lean_vectors();
    assert_eq!(vectors.len(), 8, "every Lean vector is parsed");
    for (candidates, expected) in vectors {
        let decided = match decide(&candidates) {
            Decision::InForce(winner) => Some(winner.contract_sha256),
            Decision::Conflict(_) => None,
        };
        assert_eq!(decided, expected, "{candidates:?}");
    }
}

/// Every permutation and duplication of a candidate list decides the same.
#[test]
fn the_decision_reads_only_the_set() {
    let stamps = [None, Some(1), Some(2), Some(3)];
    let mut lists = Vec::new();
    for a in stamps {
        for b in stamps {
            for c in stamps {
                for texts in [(1, 2, 3), (1, 1, 2), (1, 1, 1)] {
                    lists.push(vec![
                        candidate(1, texts.0, a),
                        candidate(2, texts.1, b),
                        candidate(3, texts.2, c),
                    ]);
                }
            }
        }
    }
    for list in lists {
        let expected = decide(&list);
        let orders = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        for order in orders {
            let permuted = order.iter().map(|i| list[*i].clone()).collect::<Vec<_>>();
            assert_eq!(decide(&permuted), expected, "{permuted:?}");
            let mut doubled = permuted.clone();
            doubled.push(permuted[0].clone());
            assert_eq!(decide(&doubled), expected, "{doubled:?}");
        }
        // Union is associative and commutative because the decision is a
        // function of the set: splitting the list anywhere changes nothing.
        let (left, right) = list.split_at(1);
        let union = right.iter().chain(left).cloned().collect::<Vec<_>>();
        assert_eq!(decide(&union), expected);
        // Never optimistic: an unusable stamp is always a conflict.
        if list
            .iter()
            .any(|c| !matches!(c.stamp, StampStanding::Stamped(_)))
        {
            assert!(matches!(expected, Decision::Conflict(_)));
        }
    }
}

#[test]
fn the_conflict_reason_is_a_function_of_the_set() {
    let malformed = Candidate {
        stamp: StampStanding::Malformed("x".into()),
        ..candidate(1, 1, None)
    };
    let missing = candidate(2, 2, None);
    let stale = Candidate {
        stamp: StampStanding::Stale,
        ..candidate(3, 3, None)
    };
    for list in [
        vec![malformed.clone(), missing.clone(), stale.clone()],
        vec![stale.clone(), missing.clone(), malformed.clone()],
    ] {
        assert_eq!(
            decide(&list),
            Decision::Conflict(ConflictReason::MalformedStamp)
        );
    }
    assert_eq!(
        decide(&[stale.clone(), missing]),
        Decision::Conflict(ConflictReason::MissingStamp)
    );
    assert_eq!(
        decide(&[stale, candidate(4, 4, Some(1))]),
        Decision::Conflict(ConflictReason::StaleStamp)
    );
    assert_eq!(decide(&[]), Decision::Conflict(ConflictReason::Unpinned));
}
