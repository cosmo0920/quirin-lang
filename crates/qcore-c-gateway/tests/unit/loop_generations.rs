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
            previous_owes,
            previous_held,
            previous_unresolved,
            older_owes,
            older_held,
            older_unresolved,
        ),
        expected,
    ) in STEP_VECTORS
    {
        let step = decide(Earlier {
            present,
            previous_owes,
            previous_held,
            previous_unresolved,
            older_owes,
            older_held,
            older_unresolved,
        });
        assert_eq!(step.code(), expected);
    }
}

/// Every one of the 128 inputs: refused exactly when an owed obligation meets
/// a holder or an unresolved effect; fresh exactly when absent.
#[test]
fn the_decision_is_total_and_refuses_only_an_exposed_owed_summary() {
    for bits in 0u8..128 {
        let bit = |index: u8| bits & (1 << index) != 0;
        let earlier = Earlier {
            present: bit(0),
            previous_owes: bit(1),
            previous_held: bit(2),
            previous_unresolved: bit(3),
            older_owes: bit(4),
            older_held: bit(5),
            older_unresolved: bit(6),
        };
        let owed = earlier.previous_owes || earlier.older_owes;
        let exposed = earlier.previous_held
            || earlier.older_held
            || earlier.previous_unresolved
            || earlier.older_unresolved;
        let expected = if !earlier.present {
            Step::Fresh
        } else if owed && exposed {
            Step::ShiftRefused
        } else {
            Step::Shift
        };
        assert_eq!(decide(earlier), expected, "{earlier:?}");
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
