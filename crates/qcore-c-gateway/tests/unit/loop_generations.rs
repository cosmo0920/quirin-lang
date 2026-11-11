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

//! Card CG04-LOOP-SITE-GENERATIONS: the decision table and the identity
//! encoding, and their mechanical binding to
//! `proof/QCore/LoopSiteGenerations.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/LoopSiteGenerations.lean");

/// The rows of the Lean list whose definition line is `header`, each split on
/// commas, with a constructor `.shiftRefused` spelled `shift-refused`.
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
                .map(|cell| {
                    let cell = cell.trim().trim_start_matches('.');
                    let mut code = String::new();
                    for character in cell.chars() {
                        if character.is_ascii_uppercase() {
                            code.push('-');
                            code.push(character.to_ascii_lowercase());
                        } else {
                            code.push(character);
                        }
                    }
                    code
                })
                .collect()
        })
        .collect()
}

#[test]
fn the_step_vectors_hold() {
    for &(
        (
            present,
            previous_live,
            previous_held,
            previous_unresolved,
            older_live,
            older_held,
            older_unresolved,
        ),
        expected,
    ) in STEP_VECTORS
    {
        let step = decide(Earlier {
            present,
            previous_live,
            previous_held,
            previous_unresolved,
            older_live,
            older_held,
            older_unresolved,
        });
        assert_eq!(step.code(), expected);
    }
}

/// Every one of the 128 inputs: refused exactly when a possibly-live summary
/// meets a holder, or one possibly-live side meets the other's unresolved
/// effect; fresh exactly when absent; and monotone in every input.
#[test]
fn the_decision_is_total_monotone_and_refuses_only_an_exposed_live_summary() {
    let earlier = |bits: u8| {
        let bit = |index: u8| bits & (1 << index) != 0;
        Earlier {
            present: bit(0),
            previous_live: bit(1),
            previous_held: bit(2),
            previous_unresolved: bit(3),
            older_live: bit(4),
            older_held: bit(5),
            older_unresolved: bit(6),
        }
    };
    for bits in 0u8..128 {
        let e = earlier(bits);
        let exposed = ((e.previous_live || e.older_live) && (e.previous_held || e.older_held))
            || (e.previous_live && e.older_unresolved)
            || (e.older_live && e.previous_unresolved);
        let expected = if !e.present {
            Step::Fresh
        } else if exposed {
            Step::ShiftRefused
        } else {
            Step::Shift
        };
        assert_eq!(decide(e), expected, "{e:?}");
        // Monotone: adding any input bit never withdraws a refusal.
        for more in 0u8..7 {
            let grown = earlier(bits | (1 << more));
            if decide(e) == Step::ShiftRefused {
                assert_eq!(decide(grown), Step::ShiftRefused, "{e:?} -> {grown:?}");
            }
        }
    }
}

#[test]
fn cg04_lean_step_vectors_match_the_rust_table() {
    let rows = lean_rows(
        "def stepVectors : List (Bool × Bool × Bool × Bool × Bool × Bool × Bool × Step) := [",
    );
    assert_eq!(rows.len(), STEP_VECTORS.len());
    for (row, ((a, b, c, d, e, f, g), step)) in rows.iter().zip(STEP_VECTORS) {
        assert_eq!(
            row,
            &[
                a.to_string(),
                b.to_string(),
                c.to_string(),
                d.to_string(),
                e.to_string(),
                f.to_string(),
                g.to_string(),
                (*step).to_owned()
            ]
        );
    }
}

#[test]
fn the_identity_vectors_hold() {
    for &(identity, site, code) in IDENTITY_VECTORS {
        assert_eq!(base(identity), site);
        assert_eq!(generation_code(generation(identity)), code);
        // The Lean model's arithmetic reading of the same tags.
        assert_eq!(identity % PREVIOUS_TAG, site);
        assert_eq!(identity / PREVIOUS_TAG, usize::from(code));
    }
}

#[test]
fn cg04_lean_identity_vectors_match_the_rust_table() {
    let rows = lean_rows("def identityVectors : List (Nat × Nat × Nat) := [");
    assert_eq!(rows.len(), IDENTITY_VECTORS.len());
    for (row, (identity, site, code)) in rows.iter().zip(IDENTITY_VECTORS) {
        assert_eq!(
            row,
            &[identity.to_string(), site.to_string(), code.to_string()]
        );
    }
    assert!(LEAN.contains(&format!("def tag : Nat := {PREVIOUS_TAG}")));
    assert_eq!(OLDER_TAG, 2 * PREVIOUS_TAG);
}

#[test]
fn the_three_generations_of_a_site_are_distinct_and_name_it() {
    for site in [0usize, 1, 7, 4_095, PREVIOUS_TAG - 1] {
        assert!(indexable(site));
        let identities = [site, previous(site), older(site)];
        for identity in identities {
            assert_eq!(base(identity), site);
        }
        assert_eq!(generation(site), Generation::Current);
        assert_eq!(generation(previous(site)), Generation::Previous);
        assert_eq!(generation(older(site)), Generation::Older);
        assert!(is_summary(older(site)));
        assert!(!is_summary(previous(site)) && !is_summary(site));
    }
    assert!(!indexable(PREVIOUS_TAG));
}
