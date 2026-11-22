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

//! Card NULLABLE-GUARDED-RELEASE: the settled-exit withdrawal decision and
//! its mechanical binding to `proof/QCore/NullableGuardedRelease.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/NullableGuardedRelease.lean");

fn lean_withdraw_rows() -> Vec<(bool, bool, bool)> {
    let table = LEAN
        .split("def withdrawVectors : List (Bool × Bool × Bool) := [")
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
fn the_withdraw_vectors_hold_and_match_the_lean_table() {
    for &(settled_unresolved, unknown_leaf, expected) in &WITHDRAW_VECTORS {
        assert_eq!(withdraws(settled_unresolved, unknown_leaf), expected);
    }
    assert_eq!(lean_withdraw_rows(), WITHDRAW_VECTORS.to_vec());
}

/// A report is withdrawn only beside an `Unknown` leaf, so no function can
/// move towards `Safe` or `Pending` by this rule.
#[test]
fn a_withdrawal_always_has_an_unknown_leaf() {
    for settled_unresolved in [false, true] {
        assert!(!withdraws(settled_unresolved, false));
    }
}

fn cfg() -> LegacyCfg {
    LegacyCfg {
        function: "w".to_owned(),
        entry: 0,
        nodes: Vec::new(),
        publications: Default::default(),
        conditional_names: BTreeSet::new(),
        tie_names: BTreeSet::new(),
        traversal: Default::default(),
        cell_tests: Default::default(),
        cell_frames: Default::default(),
        thread_local_reads: Default::default(),
    }
}

fn refusal() -> LegacyDiagnostic {
    LegacyDiagnostic {
        function: "w".to_owned(),
        line: 4,
        kind: LegacyDiagnosticKind::UnknownAlias {
            variable: "p".to_owned(),
        },
    }
}

fn leak() -> LegacyDiagnostic {
    LegacyDiagnostic {
        function: "w".to_owned(),
        line: 9,
        kind: LegacyDiagnosticKind::Leak {
            acquisition_line: 2,
        },
    }
}

/// The settled exit's own facts decide a report held on any iterate: the
/// iterate claim of a site the settled exit marks unresolved (the Metadata
/// shape: a refused release's marker joined after the null edge reached the
/// exit) is withdrawn beside the refusal; an unmarked site's claim is
/// published; a marked site's claim with no `Unknown` leaf is published.
#[test]
fn settle_reads_the_settled_exit() {
    let settled = |marked: bool| {
        let mut state = AbstractState::default();
        state.resources.insert(1, super::super::LIVE);
        if marked {
            state.unresolved_effect_sites.insert(1);
        }
        vec![Some(state)]
    };
    for (marked, leaf, published) in [
        (true, true, false),
        (false, true, true),
        (true, false, true),
        (false, false, true),
    ] {
        let mut held = Held::default();
        held.hold(9, 2, 1, 0);
        // A second iterate's report of the same identity changes nothing.
        held.hold(9, 2, 1, 0);
        let mut diagnostics = BTreeSet::new();
        if leaf {
            diagnostics.insert(refusal());
        }
        held.settle(&cfg(), &settled(marked), &mut diagnostics);
        assert_eq!(diagnostics.contains(&leak()), published, "{marked} {leaf}");
    }
}

/// One identity the settled exit does not mark publishes the key even when
/// another identity reporting the same key is withdrawn.
#[test]
fn any_unwithdrawn_identity_publishes_the_key() {
    let mut state = AbstractState::default();
    state.unresolved_effect_sites.insert(1);
    let mut held = Held::default();
    held.hold(9, 2, 1, 0);
    held.hold(9, 2, 5, 0);
    let mut diagnostics = BTreeSet::from([refusal()]);
    held.settle(&cfg(), &[Some(state)], &mut diagnostics);
    assert!(diagnostics.contains(&leak()));
}
