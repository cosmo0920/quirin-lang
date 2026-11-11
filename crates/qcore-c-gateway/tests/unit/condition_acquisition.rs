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

//! Card CG04-CONDITION-ACQUISITION (rule 10x): the decision tables and their
//! mechanical binding to `proof/QCore/ConditionAcquisition.lean`.

use super::*;

fn split(code: &str) -> Option<Split> {
    match code {
        "null" => Some(Split::Null),
        "sentinel" => Some(Split::Sentinel),
        "none" => None,
        other => panic!("unknown split {other}"),
    }
}

fn result(code: &str) -> ResultClass {
    match code {
        "none" => ResultClass::None,
        "derived" => ResultClass::Derived,
        "owned" => ResultClass::Owned,
        "reallocated" => ResultClass::Reallocated,
        other => panic!("unknown result {other}"),
    }
}

#[test]
fn the_use_vectors_hold() {
    for (bound, split_code, expected) in USE_VECTORS {
        assert_eq!(
            leaf_use(*bound, split(split_code)).code(),
            *expected,
            "{bound} {split_code}"
        );
    }
}

#[test]
fn the_carrier_vectors_hold() {
    for (result_code, integer, split_code, expected) in CARRIER_VECTORS {
        let split = split(split_code).expect("a carrier row names a split");
        let decided = match carrier(result(result_code), *integer, split) {
            Ok(()) => "accept",
            Err(refusal) => refusal.code(),
        };
        assert_eq!(decided, *expected, "{result_code} {integer} {split_code}");
        assert_eq!(split.code(), *split_code);
    }
}

#[test]
fn the_leaf_vectors_hold() {
    for (call, in_operand, increment, connective, expected) in LEAF_VECTORS {
        let features = LeafFeatures {
            call: *call,
            call_in_type_operand: *in_operand,
            increment: *increment,
            connective: *connective,
        };
        let decided = match leaf_route(features) {
            Ok(route) => route.code(),
            Err(refusal) => refusal.code(),
        };
        assert_eq!(decided, *expected, "{features:?}");
    }
}

/// Every combination of features: a leaf reaches the calls route only with a
/// call and no refused feature, the call-free route only with no call
/// anywhere (`QCore.ConditionAcquisition.only_listed_leaves_are_admitted`).
#[test]
fn only_listed_leaves_are_admitted() {
    for bits in 0u8..16 {
        let features = LeafFeatures {
            call: bits & 1 != 0,
            call_in_type_operand: bits & 2 != 0,
            increment: bits & 4 != 0,
            connective: bits & 8 != 0,
        };
        let route = leaf_route(features);
        assert_eq!(
            route == Ok(LeafRoute::Calls),
            features.call
                && !features.call_in_type_operand
                && !features.increment
                && !features.connective,
            "{features:?}"
        );
        assert_eq!(
            route == Ok(LeafRoute::CallFree),
            !features.call && !features.call_in_type_operand,
            "{features:?}"
        );
    }
}

/// A Lean constructor name as the Rust code spells it: `.testedSplit` is
/// `tested-split`.
fn lean_code(name: &str) -> String {
    let name = name.trim().trim_start_matches('.');
    let mut code = String::new();
    for character in name.chars() {
        if character.is_ascii_uppercase() {
            code.push('-');
            code.push(character.to_ascii_lowercase());
        } else {
            code.push(character);
        }
    }
    code
}

fn lean_rows(header: &str) -> Vec<Vec<String>> {
    let table = LEAN.split(header).nth(1).expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|row| {
            row.trim_end_matches(',')
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(lean_code)
                .collect()
        })
        .collect()
}

const LEAN: &str = include_str!("../../../../proof/QCore/ConditionAcquisition.lean");

#[test]
fn cg04_lean_use_vectors_match_the_rust_table() {
    let rows = lean_rows("def useVectors : List (Bool × Test × Use) := [");
    assert_eq!(rows.len(), USE_VECTORS.len());
    for (row, (bound, split, use_)) in rows.iter().zip(USE_VECTORS) {
        assert_eq!(
            row,
            &[bound.to_string(), (*split).to_owned(), (*use_).to_owned()]
        );
    }
}

#[test]
fn cg04_lean_carrier_vectors_match_the_rust_table() {
    let rows =
        lean_rows("def carrierVectors : List (ResultClass × Bool × Split × CarrierDecision) := [");
    assert_eq!(rows.len(), CARRIER_VECTORS.len());
    for (row, (result, integer, split, decision)) in rows.iter().zip(CARRIER_VECTORS) {
        assert_eq!(
            row,
            &[
                (*result).to_owned(),
                integer.to_string(),
                (*split).to_owned(),
                (*decision).to_owned()
            ]
        );
    }
}

#[test]
fn cg04_lean_leaf_vectors_match_the_rust_table() {
    let rows = lean_rows("def leafVectors : List (Bool × Bool × Bool × Bool × LeafDecision) := [");
    assert_eq!(rows.len(), LEAF_VECTORS.len());
    for (row, (call, in_operand, increment, connective, decision)) in rows.iter().zip(LEAF_VECTORS)
    {
        assert_eq!(
            row,
            &[
                call.to_string(),
                in_operand.to_string(),
                increment.to_string(),
                connective.to_string(),
                (*decision).to_owned()
            ]
        );
    }
}

#[test]
fn the_placement_vectors_hold() {
    for (custody, on_cycle, generations, expected) in PLACEMENT_VECTORS {
        let decided = match placement(*custody, *on_cycle, *generations) {
            Ok(()) => "accept",
            Err(refusal) => refusal.code(),
        };
        assert_eq!(decided, *expected, "{custody} {on_cycle} {generations}");
    }
}

/// Card CG04-LOOP-SITE-GENERATIONS lifted the guard: the flow indexes its
/// sites by generation, so no production consultation takes the refusal arm.
#[test]
fn the_flow_indexes_generations_so_the_guard_is_lifted() {
    const { assert!(crate::loop_generations::SITES_ARE_GENERATION_INDEXED) };
    assert_eq!(
        placement(
            true,
            true,
            crate::loop_generations::SITES_ARE_GENERATION_INDEXED
        ),
        Ok(())
    );
}

#[test]
fn cg04_lean_placement_vectors_match_the_rust_table() {
    let rows =
        lean_rows("def placementVectors : List (Bool × Bool × Bool × PlacementDecision) := [");
    assert_eq!(rows.len(), PLACEMENT_VECTORS.len());
    for (row, (custody, on_cycle, generations, decision)) in rows.iter().zip(PLACEMENT_VECTORS) {
        assert_eq!(
            row,
            &[
                custody.to_string(),
                on_cycle.to_string(),
                generations.to_string(),
                (*decision).to_owned()
            ]
        );
    }
}
