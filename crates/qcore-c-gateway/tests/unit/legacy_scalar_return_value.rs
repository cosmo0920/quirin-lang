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

//! Card W48-BOTTOM-UP-CALLEE-BODIES: rule 12f's decision and its mechanical
//! binding to `proof/QCore/ScalarReturnValue.lean`.

use std::collections::BTreeSet;

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/ScalarReturnValue.lean");

/// The rows of the Lean list whose definition line is `header`, each split on
/// commas, without parentheses and constructor dots.
fn lean_rows(header: &str) -> Vec<Vec<String>> {
    let table = LEAN.split(header).nth(1).expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|row| {
            row.trim_end_matches(',')
                .replace(['(', ')'], "")
                .split(',')
                .map(|cell| cell.trim().trim_start_matches('.').to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn the_decision_vectors_hold() {
    for &((refused, scalar_result, hidden, walker), decision) in DECISION_VECTORS {
        assert_eq!(
            decide(refused, scalar_result, hidden, walker).code(),
            decision,
            "{refused} {scalar_result} {hidden} {walker}"
        );
    }
}

#[test]
fn the_decision_vectors_are_the_lean_table() {
    let rows = lean_rows("def decisionVectors : List DecisionVector := [");
    let rust = DECISION_VECTORS
        .iter()
        .map(|&((refused, scalar_result, hidden, walker), decision)| {
            vec![
                refused.to_string(),
                scalar_result.to_string(),
                hidden.to_string(),
                walker.to_string(),
                decision.to_owned(),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn the_vectors_cover_every_combination() {
    let mut seen = BTreeSet::new();
    for &(inputs, _) in DECISION_VECTORS {
        assert!(seen.insert(inputs), "duplicate row {inputs:?}");
    }
    assert_eq!(seen.len(), 16);
}
