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

//! Card SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN: the publication decision and
//! its mechanical binding to `proof/QCore/SettledLeakJoined.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/SettledLeakJoined.lean");

fn lean_publish_rows() -> Vec<(bool, bool, bool)> {
    let table = LEAN
        .split("def publishVectors : List (Bool × Bool × Bool) := [")
        .nth(1)
        .expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|row| {
            let cells = row
                .trim_end_matches(',')
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(str::trim)
                .map(|cell| match cell {
                    "true" => true,
                    "false" => false,
                    other => panic!("not a Bool: {other}"),
                })
                .collect::<Vec<_>>();
            (cells[0], cells[1], cells[2])
        })
        .collect()
}

#[test]
fn the_publish_vectors_hold_and_match_the_lean_table() {
    for &(withdrawn, witnessed, expected) in &PUBLISH_VECTORS {
        assert_eq!(publishes(withdrawn, witnessed), expected);
    }
    assert_eq!(lean_publish_rows(), PUBLISH_VECTORS.to_vec());
}

/// A report rule 74c (A) does not withdraw is published whatever the
/// witness; a withdrawn one only with a witness.
#[test]
fn a_withdrawn_report_needs_a_witness() {
    assert!(publishes(false, false));
    assert!(!publishes(true, false));
    assert!(publishes(true, true));
}
