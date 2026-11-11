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

//! Card SUP07-REFUSED-TEST-ACCUSATION: rule 12e's edge step and its
//! mechanical binding to `proof/QCore/RefusedTestAccusation.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/RefusedTestAccusation.lean");

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
fn the_edge_vectors_hold() {
    for &((refused, absent, tied, targeted), step) in EDGE_VECTORS {
        assert_eq!(
            edge_step(refused, absent, tied, targeted).code(),
            step,
            "{refused} {absent} {tied} {targeted}"
        );
    }
}

#[test]
fn the_edge_vectors_are_the_lean_table() {
    let rows = lean_rows("def edgeVectors : List EdgeVector := [");
    let rust = EDGE_VECTORS
        .iter()
        .map(|&((refused, absent, tied, targeted), step)| {
            vec![
                refused.to_string(),
                absent.to_string(),
                tied.to_string(),
                targeted.to_string(),
                step.to_owned(),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn the_vectors_cover_every_combination() {
    let mut seen = BTreeSet::new();
    for &(inputs, _) in EDGE_VECTORS {
        assert!(seen.insert(inputs), "duplicate row {inputs:?}");
    }
    assert_eq!(seen.len(), 16);
}

fn null_test(source: &str) -> LegacyInstruction {
    LegacyInstruction::BranchOnNull {
        source: source.to_owned(),
        true_is_null: true,
    }
}

#[test]
fn only_the_null_edge_of_a_null_test_is_absent() {
    assert_eq!(absent_edge_source(&null_test("temp"), 0), Some("temp"));
    assert_eq!(absent_edge_source(&null_test("temp"), 1), None);
    let inverted = LegacyInstruction::BranchOnNull {
        source: "temp".to_owned(),
        true_is_null: false,
    };
    assert_eq!(absent_edge_source(&inverted, 0), None);
    assert_eq!(absent_edge_source(&inverted, 1), Some("temp"));
    assert_eq!(absent_edge_source(&LegacyInstruction::NoOp, 0), None);
}

#[test]
fn only_tied_names_are_read() {
    let joined = BTreeMap::from([
        (1, BTreeSet::from(["temp".to_owned()])),
        (2, BTreeSet::from(["other".to_owned()])),
        (3, BTreeSet::new()),
    ]);
    assert_eq!(tied_sites(&joined, "temp"), BTreeSet::from([1]));
    assert!(tied_sites(&joined, "absent").is_empty());
}

#[test]
fn only_a_member_address_through_a_name_is_read_as_one() {
    let read = |text: &str| {
        let body = text.split_whitespace().collect::<Vec<_>>();
        let (read, any) = member_addresses_read(&body);
        (read.join(" "), any)
    };
    assert_eq!(
        read("e = & conn -> event ;"),
        ("e = + conn -> event ;".to_owned(), true)
    );
    assert_eq!(
        read("f ( & ( conn ) -> s ) ;"),
        ("f ( + ( conn ) -> s ) ;".to_owned(), true)
    );
    // The tokenizer's spelling of `->`.
    assert_eq!(
        read("e = & conn - > event ;"),
        ("e = + conn - > event ;".to_owned(), true)
    );
    assert_eq!(
        read("f ( & ( conn -> s ) ) ;"),
        ("f ( + ( conn -> s ) ) ;".to_owned(), true)
    );
    // The name's own address, a member of the object itself, an element and
    // a dereference keep their `&`.
    for kept in [
        "f ( & conn ) ;",
        "f ( & conn . s ) ;",
        "f ( & conn [ 0 ] ) ;",
        "f ( & * conn ) ;",
        "f ( & ( conn ) ) ;",
        "f ( ( & conn ) -> s ) ;",
    ] {
        assert_eq!(read(kept), (kept.to_owned(), false), "{kept}");
    }
}
