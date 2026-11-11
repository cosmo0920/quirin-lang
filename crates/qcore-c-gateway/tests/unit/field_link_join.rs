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

//! Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: the link join, the escape step
//! and the exit floor, and their mechanical binding to
//! `proof/QCore/FieldLinkJoin.lean`.

use super::*;
use crate::legacy_c::{ESCAPED, LIVE};

const LEAN: &str = include_str!("../../../../proof/QCore/FieldLinkJoin.lean");

/// The rows of the Lean list whose definition line is `header`, each split on
/// commas, constructors without their dot and guillemets.
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
                .map(|cell| cell.trim().trim_start_matches('.').replace(['«', '»'], ""))
                .collect()
        })
        .collect()
}

fn link(code: &str) -> Link {
    match code {
        "absent" => Link::Absent,
        "must" => Link::Must,
        "partial" => Link::Partial,
        other => panic!("unknown link {other}"),
    }
}

fn status(tracked: bool, live: bool, released: bool, escaped: bool, contained: bool) -> Option<u8> {
    tracked.then(|| {
        [
            (live, LIVE),
            (released, RELEASED_BIT),
            (escaped, ESCAPED),
            (contained, CONTAINED_BIT),
        ]
        .into_iter()
        .filter(|(set, _)| *set)
        .fold(0, |bits, (_, bit)| bits | bit)
    })
}

#[test]
fn the_link_vectors_hold() {
    for &(one, other, joined) in LINK_VECTORS {
        assert_eq!(join_link(link(one), link(other)).code(), joined);
    }
}

#[test]
fn b08_lean_link_vectors_match_the_rust_table() {
    let rows = lean_rows("def linkVectors : List (Link × Link × Link) := [");
    assert_eq!(rows.len(), LINK_VECTORS.len());
    for (row, (one, other, joined)) in rows.iter().zip(LINK_VECTORS) {
        assert_eq!(row, &[*one, *other, *joined]);
    }
}

#[test]
fn the_escape_vectors_hold() {
    for &((partial, tracked, live, released, escaped, contained), step) in ESCAPE_VECTORS {
        assert_eq!(
            escape_step(partial, status(tracked, live, released, escaped, contained)).code(),
            step
        );
    }
}

#[test]
fn b08_lean_escape_vectors_match_the_rust_table() {
    let rows =
        lean_rows("def escapeVectors : List (Bool × Bool × Bool × Bool × Bool × Bool × Step) := [");
    assert_eq!(rows.len(), ESCAPE_VECTORS.len());
    for (row, ((a, b, c, d, e, f), step)) in rows.iter().zip(ESCAPE_VECTORS) {
        assert_eq!(
            row,
            &[
                a.to_string(),
                b.to_string(),
                c.to_string(),
                d.to_string(),
                e.to_string(),
                f.to_string(),
                (*step).to_owned()
            ]
        );
    }
}

/// Every one of the 64 inputs: a possibly-live child is escaped or refused,
/// never kept or withdrawn; it is escaped only exactly live on every path;
/// a partial link never lets a possibly-live child escape.
#[test]
fn the_escape_step_is_total_and_never_drops_a_possibly_live_child() {
    for bits in 0u8..64 {
        let bit = |index: u8| bits & (1 << index) != 0;
        let partial = bit(0);
        let tracked = bit(1);
        let child = status(tracked, bit(2), bit(3), bit(4), bit(5));
        let step = escape_step(partial, child);
        let live = child.is_some_and(|bits| bits & LIVE != 0);
        if live {
            assert!(
                matches!(step, EscapeStep::Escape | EscapeStep::Refuse),
                "{bits:#b}"
            );
        }
        if step == EscapeStep::Escape {
            assert!(!partial && child == Some(LIVE), "{bits:#b}");
        }
        if partial && live {
            assert_eq!(step, EscapeStep::Refuse, "{bits:#b}");
        }
    }
}

#[test]
fn the_exit_vectors_hold_and_match_lean() {
    for &(withheld, modeled, refuses) in EXIT_VECTORS {
        assert_eq!(exit_refuses(withheld, modeled), refuses);
    }
    let rows = lean_rows("def exitVectors : List (Bool × Bool × Bool) := [");
    assert_eq!(rows.len(), EXIT_VECTORS.len());
    for (row, (a, b, c)) in rows.iter().zip(EXIT_VECTORS) {
        assert_eq!(row, &[a.to_string(), b.to_string(), c.to_string()]);
    }
}
