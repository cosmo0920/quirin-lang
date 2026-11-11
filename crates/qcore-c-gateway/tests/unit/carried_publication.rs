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

//! Card MONKEY-VHOST-PROPERTY-CUSTODY: the publish and unlink steps of rule
//! 12d and their mechanical binding to `proof/QCore/CarriedPublication.lean`.

use super::*;
use crate::legacy_c::{CONTAINED, RELEASED};

const LEAN: &str = include_str!("../../../../proof/QCore/CarriedPublication.lean");

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

fn status(tracked: bool, live: bool, released: bool, escaped: bool, contained: bool) -> Option<u8> {
    tracked.then(|| {
        [
            (live, LIVE),
            (released, RELEASED),
            (escaped, ESCAPED),
            (contained, CONTAINED),
        ]
        .into_iter()
        .filter(|(set, _)| *set)
        .fold(0, |bits, (_, bit)| bits | bit)
    })
}

#[test]
fn the_publish_vectors_hold() {
    for &(
        (partial, tracked, live, released, escaped, contained, unresolved, published, owns),
        step,
    ) in PUBLISH_VECTORS
    {
        let facts = ChildFacts {
            partial,
            status: status(tracked, live, released, escaped, contained),
            unresolved,
            published,
            owns,
        };
        assert_eq!(publish_step(facts).code(), step, "{facts:?}");
    }
}

#[test]
fn the_unlink_vectors_hold() {
    for &((partial, tracked, live, released, escaped), step) in UNLINK_VECTORS {
        let bits = status(tracked, live, released, escaped, false);
        assert_eq!(
            unlink_step(partial, bits).code(),
            step,
            "{partial} {bits:?}"
        );
    }
}

/// Exactly one tracked status carries, and only without an encumbrance.
#[test]
fn only_an_exactly_live_unencumbered_child_is_carried() {
    for bits in 0u8..16 {
        for flags in 0u8..16 {
            let facts = ChildFacts {
                partial: flags & 1 != 0,
                status: Some(bits),
                unresolved: flags & 2 != 0,
                published: flags & 4 != 0,
                owns: flags & 8 != 0,
            };
            let carried = publish_step(facts) == PublishStep::Carry;
            assert_eq!(carried, bits == LIVE && flags == 0, "{facts:?}");
        }
    }
}

/// An untracked child is never carried.
#[test]
fn an_untracked_child_refuses() {
    for flags in 0u8..16 {
        let facts = ChildFacts {
            partial: flags & 1 != 0,
            status: None,
            unresolved: flags & 2 != 0,
            published: flags & 4 != 0,
            owns: flags & 8 != 0,
        };
        assert_eq!(publish_step(facts), PublishStep::Refuse, "{facts:?}");
    }
}

/// An unlink restores exactly a must-linked, exactly escaped child.
#[test]
fn only_an_exactly_escaped_must_linked_child_is_restored() {
    for bits in 0u8..16 {
        for partial in [false, true] {
            let restored = unlink_step(partial, Some(bits)) == UnlinkStep::Restore;
            assert_eq!(restored, bits == ESCAPED && !partial);
        }
    }
    assert_eq!(unlink_step(false, None), UnlinkStep::Keep);
}

fn bool_cell(cell: &str) -> bool {
    match cell {
        "true" => true,
        "false" => false,
        other => panic!("not a boolean: {other}"),
    }
}

#[test]
fn the_lean_publish_vectors_are_the_rust_table() {
    let rows = lean_rows("def publishVectors");
    assert_eq!(rows.len(), PUBLISH_VECTORS.len());
    for (row, &((a, b, c, d, e, f, g, h, i), step)) in rows.iter().zip(PUBLISH_VECTORS) {
        let cells: Vec<bool> = row[..9].iter().map(|cell| bool_cell(cell)).collect();
        assert_eq!(cells, [a, b, c, d, e, f, g, h, i]);
        assert_eq!(row[9], step);
    }
}

#[test]
fn the_lean_unlink_vectors_are_the_rust_table() {
    let rows = lean_rows("def unlinkVectors");
    assert_eq!(rows.len(), UNLINK_VECTORS.len());
    for (row, &((a, b, c, d, e), step)) in rows.iter().zip(UNLINK_VECTORS) {
        let cells: Vec<bool> = row[..5].iter().map(|cell| bool_cell(cell)).collect();
        assert_eq!(cells, [a, b, c, d, e]);
        assert_eq!(row[5], step);
    }
}
