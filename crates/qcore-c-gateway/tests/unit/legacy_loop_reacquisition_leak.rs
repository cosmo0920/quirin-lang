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

//! Card LOOP-REACQUISITION-LEAK-ACCUSATION: the withdrawal decision and its
//! mechanical binding to `proof/QCore/LoopReacquisitionLeak.lean`.

use super::*;
use crate::loop_generations::{older, previous};

const LEAN: &str = include_str!("../../../../proof/QCore/LoopReacquisitionLeak.lean");

fn lean_withdraw_rows() -> Vec<(usize, bool, bool, bool)> {
    let table = LEAN
        .split("def withdrawVectors : List (Nat × Bool × Bool × Bool) := [")
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
                .collect::<Vec<_>>();
            let flag = |cell: &str| match cell {
                "true" => true,
                "false" => false,
                other => panic!("not a Bool: {other}"),
            };
            (
                cells[0].parse().expect("an identity"),
                flag(cells[1]),
                flag(cells[2]),
                flag(cells[3]),
            )
        })
        .collect()
}

#[test]
fn the_withdraw_vectors_hold_and_match_the_lean_table() {
    for &(identity, refused, reported, expected) in WITHDRAW_VECTORS {
        assert_eq!(
            withdraws(identity, refused, reported),
            expected,
            "{identity:#x} {refused} {reported}"
        );
    }
    assert_eq!(lean_withdraw_rows(), WITHDRAW_VECTORS.to_vec());
}

/// Every input: a current generation is never withdrawn, an earlier one
/// exactly when refused and reported.
#[test]
fn only_an_earlier_generation_of_a_refused_reported_site_is_withdrawn() {
    for site in [0, 7, 12_345] {
        for refused in [false, true] {
            for reported in [false, true] {
                assert!(!withdraws(site, refused, reported));
                assert!(!Deferred::defers(site));
                for earlier in [previous(site), older(site)] {
                    assert!(Deferred::defers(earlier));
                    assert_eq!(withdraws(earlier, refused, reported), refused && reported);
                }
            }
        }
    }
}
