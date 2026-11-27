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

//! Card SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF: the blocking-leaf condition
//! and its mechanical binding to `proof/QCore/SettledLeakJoined.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/SettledLeakJoined.lean");

/// Rule 84 decides a marked report beside an `Unknown` or a `Pending` leaf,
/// as `QCore.SettledLeakJoined.blocking` states; a mark beside neither is
/// left to rule 74c.
#[test]
fn a_blocking_leaf_is_an_unknown_or_a_pending_leaf() {
    assert!(LEAN.contains(
        "def blocking (unknownLeaf pendingLeaf : Bool) : Bool := unknownLeaf || pendingLeaf"
    ));
    for unknown_leaf in [false, true] {
        for pending_leaf in [false, true] {
            assert_eq!(
                blocking(unknown_leaf, pending_leaf),
                unknown_leaf || pending_leaf
            );
        }
    }
}

fn leaf(kind: LegacyDiagnosticKind) -> LegacyDiagnostic {
    LegacyDiagnostic {
        function: "w".to_owned(),
        line: 1,
        kind,
    }
}

/// The leaves are read through `diagnostic_verdict`: a held call's refusal is
/// `Pending`, any other refusal `Unknown`, a violation neither.
#[test]
fn the_blocking_leaf_reads_the_verdict_of_each_leaf() {
    let held = leaf(LegacyDiagnosticKind::Unsupported {
        description: "call to `k_close` whose declared contract awaits human review \
                      (awaiting-human-review)"
            .to_owned(),
    });
    let refused = leaf(LegacyDiagnosticKind::Unsupported {
        description: "call to uncontracted function `u_keep`".to_owned(),
    });
    let leak = leaf(LegacyDiagnosticKind::Leak {
        acquisition_line: 1,
    });
    assert!(blocking_leaf(&BTreeSet::from([held])));
    assert!(blocking_leaf(&BTreeSet::from([refused])));
    assert!(!blocking_leaf(&BTreeSet::from([leak])));
    assert!(!blocking_leaf(&BTreeSet::new()));
}
