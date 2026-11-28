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

//! Card SETTLED-LEAK-MARK-LEAFLESS: the decision and the reason, and their
//! mechanical binding to `proof/QCore/SettledLeakLeafless.lean`.

use qcore_schemas::verdict::Verdict;

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/SettledLeakLeafless.lean");

/// The Lean table, parsed row by row.
fn lean_vectors() -> Vec<[bool; 7]> {
    let start = LEAN
        .find("def outcomeVectors")
        .expect("the Lean table is defined");
    let body = &LEAN[start..];
    let body = &body[body.find(":= [").expect("the table opens") + 4..];
    let body = &body[..body.find(']').expect("the table closes")];
    body.lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let cells = line
                .trim_start_matches('(')
                .trim_end_matches(',')
                .trim_end_matches(')')
                .split(',')
                .map(|cell| match cell.trim() {
                    "true" => true,
                    "false" => false,
                    other => panic!("not a Boolean: {other}"),
                })
                .collect::<Vec<_>>();
            cells.try_into().expect("seven cells")
        })
        .collect()
}

/// `OUTCOME_VECTORS` is `QCore.SettledLeakLeafless.outcomeVectors`, and the
/// Rust decision reproduces every row.
#[test]
fn the_outcome_vectors_are_the_lean_table() {
    let lean = lean_vectors();
    assert_eq!(lean.len(), OUTCOME_VECTORS.len());
    for (row, lean) in OUTCOME_VECTORS.iter().zip(lean) {
        let (blocking, marked, claimable, witnessed, decided, leak, reason) = *row;
        assert_eq!(
            [
                blocking, marked, claimable, witnessed, decided, leak, reason
            ],
            lean
        );
        assert_eq!(leafless(blocking, marked, claimable), decided, "{row:?}");
        let (got_leak, got_reason) = if decided {
            outcome(witnessed)
        } else {
            (false, false)
        };
        assert_eq!((got_leak, got_reason), (leak, reason), "{row:?}");
    }
    assert!(LEAN.contains(
        "def leafless (blockingLeaf marked claimable : Bool) : Bool := !blockingLeaf && marked && claimable"
    ));
    assert!(
        LEAN.contains("def outcome (witnessed : Bool) : Bool × Bool := (witnessed, !witnessed)")
    );
}

/// The reason is an `Unknown` leaf (`QCore.SettledLeakLeafless.reasonLeaf`):
/// not a held call's `Pending`, never a violation.
#[test]
fn the_reason_is_an_unknown_leaf() {
    let text = description(12);
    assert!(text.contains(MARK_WITHOUT_REFUSAL), "{text}");
    assert!(text.contains("line 12"), "{text}");
    assert!(LEAN.contains("def reasonLeaf : Leaf := ⟨.unknown, \"mark-without-refusal\"⟩"));
    assert_eq!(
        super::super::verdict::diagnostic_verdict(&LegacyDiagnosticKind::Unsupported {
            description: text,
        }),
        Verdict::Unknown
    );
}
