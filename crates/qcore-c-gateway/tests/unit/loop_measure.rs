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

//! Card W45-FB-ENGINE-PROGRESS-MEASURE: the loop-measure decision and its
//! mechanical binding to `proof/QCore/CountedLoopMeasure.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/CountedLoopMeasure.lean");

fn bools(text: &str) -> Vec<bool> {
    text.split(',')
        .map(str::trim)
        .filter(|cell| !cell.is_empty())
        .map(|cell| match cell {
            "true" => true,
            "false" => false,
            other => panic!("not a Bool: {other}"),
        })
        .collect()
}

/// The Lean `decideVectors` rows: gates, conjuncts, code.
fn lean_vectors() -> Vec<(Vec<bool>, Vec<Vec<bool>>, String)> {
    let table = LEAN
        .split("def decideVectors : List (List Bool × List (List Bool) × String) := [")
        .nth(1)
        .expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("(["))
        .map(|row| {
            let row = row.trim_end_matches(',');
            let row = &row[1..row.len() - 1];
            let gates_end = row.find(']').expect("gates");
            let gates = bools(&row[1..gates_end]);
            let rest = &row[gates_end + 1..];
            let open = rest.find('[').expect("conjuncts");
            let close = rest.rfind(']').expect("conjuncts end");
            let conjuncts = rest[open + 1..close]
                .split(']')
                .map(|cell| cell.trim_start_matches([',', ' ']).trim_start_matches('['))
                .filter(|cell| !cell.trim().is_empty())
                .map(bools)
                .collect();
            let code = rest[close + 1..]
                .trim_start_matches([',', ' '])
                .trim_matches('"')
                .to_owned();
            (gates, conjuncts, code)
        })
        .collect()
}

fn outcome(shape: &Shape) -> &'static str {
    decide(shape).map_or("admitted", Refusal::code)
}

#[test]
fn the_decide_vectors_hold_and_match_the_lean_table() {
    for (gates, conjuncts, code) in DECIDE_VECTORS {
        assert_eq!(
            outcome(&vector_shape(*gates, conjuncts)),
            *code,
            "{gates:?} {conjuncts:?}"
        );
    }
    let rust = DECIDE_VECTORS
        .iter()
        .map(|(gates, conjuncts, code)| {
            (
                gates.to_vec(),
                conjuncts.iter().map(|c| c.to_vec()).collect::<Vec<_>>(),
                (*code).to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(lean_vectors(), rust);
}

#[test]
fn every_refusal_code_is_the_lean_code() {
    let all = [
        Refusal::NoCounter,
        Refusal::StepNotIncrement,
        Refusal::CounterNotLocalInteger,
        Refusal::CounterAddressTaken,
        Refusal::OpaqueControl,
        Refusal::LabelInLoop,
        Refusal::CounterWrittenInLoop,
        Refusal::InitNotConstant,
        Refusal::TestNotLessThan,
        Refusal::ComparisonConverts,
        Refusal::CounterMayWrap,
        Refusal::BoundWrittenInLoop,
        Refusal::BoundAddressTaken,
        Refusal::BoundNotStable,
    ];
    for refusal in all {
        assert!(
            LEAN.contains(&format!("=> \"{}\"", refusal.code())),
            "{refusal:?} has no Lean code"
        );
    }
}

/// Every conjunct reading: an admitted kind needs a value-preserving
/// comparison and a bound within the counter's type (the premises
/// `an_admitted_conjunct_meets_its_premises` names).
#[test]
fn an_admitted_conjunct_meets_its_premises() {
    for bits in 0u8..128 {
        let flag = |n: u8| bits & (1 << n) != 0;
        let conjunct = Conjunct {
            preserving: flag(0),
            constant: flag(1),
            local: flag(2),
            written: flag(3),
            address_taken: flag(4),
            narrow: flag(5),
            fits_counter: flag(6),
        };
        if let Ok(kind) = conjunct_kind(&conjunct) {
            assert!(conjunct.preserving && conjunct.fits_counter, "{conjunct:?}");
            match kind {
                BoundKind::Constant => assert!(conjunct.constant),
                BoundKind::StableLocal => {
                    assert!(conjunct.local && !conjunct.written && !conjunct.address_taken);
                }
                BoundKind::TypeRange => assert!(conjunct.narrow),
            }
        }
    }
}

#[test]
fn array_extents_are_one_dimensional_constants() {
    assert_eq!(array_extent("uint64_t[1]"), Some(1));
    assert_eq!(array_extent("unsigned long[16]"), Some(16));
    assert_eq!(array_extent("int[2][3]"), None);
    assert_eq!(array_extent("int[]"), None);
    assert_eq!(array_extent("int[n]"), None);
    assert_eq!(array_extent("int"), None);
}
