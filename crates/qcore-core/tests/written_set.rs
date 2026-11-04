//! Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION: the slot derivation's
//! combination vectors hold in Rust (the module's unit tests) and are, entry
//! for entry, the vectors of `proof/QCore/WrittenSetDerivation.lean`
//! (`combineVectors`, `resolveVectors`), whose theorems
//! (`combine_vectors_hold`, `resolve_vectors_hold`) check them against the
//! Lean implementation.

use qcore_core::result_set::Interval;
use qcore_core::written_set::{COMBINE_VECTORS, RESOLVE_VECTORS};

const LEAN: &str = include_str!("../../../proof/QCore/WrittenSetDerivation.lean");

fn intervals(set: &[Interval]) -> String {
    let parts = set
        .iter()
        .map(|(lo, hi)| format!("({lo}, {hi})"))
        .collect::<Vec<_>>();
    format!("[{}]", parts.join(", "))
}

/// The rows of a Lean `def NAME … := [ … ]` table, one per line.
fn lean_rows(name: &str) -> Vec<String> {
    let start = LEAN
        .find(&format!("def {name} "))
        .unwrap_or_else(|| panic!("no Lean table {name}"));
    let body = &LEAN[start..];
    let open = body.find(":= [").expect("table opener") + 4;
    let close = body[open..].find("\n]").expect("table closer") + open;
    body[open..close]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

#[test]
fn the_combine_vectors_are_the_lean_ones() {
    let rust = COMBINE_VECTORS
        .iter()
        .map(|(ends, class, outcome)| {
            let ends = ends
                .iter()
                .map(|(returns, slot)| {
                    let slot = slot
                        .map_or_else(|| "none".to_owned(), |s| format!("some {}", intervals(s)));
                    format!("({}, {slot})", intervals(returns))
                })
                .collect::<Vec<_>>()
                .join(", ");
            let outcome = match outcome {
                Ok(set) => format!(".ok {}", intervals(set)),
                Err("written-set-unwritten") => ".unwritten".to_owned(),
                Err("written-set-no-return") => ".noReturn".to_owned(),
                Err(other) => panic!("no Lean outcome for {other}"),
            };
            format!("([{ends}], {class}, {outcome})")
        })
        .collect::<Vec<_>>();
    assert_eq!(rust, lean_rows("combineVectors"));
}

#[test]
fn the_resolve_vectors_are_the_lean_ones() {
    let rust = RESOLVE_VECTORS
        .iter()
        .map(|(narrowed, results, class, written, resolution)| {
            let resolution = match resolution {
                None => ".infeasible".to_owned(),
                Some(None) => ".unknown".to_owned(),
                Some(Some(set)) => format!(".known {}", intervals(set)),
            };
            format!(
                "({}, {}, {class}, {}, {resolution})",
                intervals(narrowed),
                intervals(results),
                intervals(written)
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rust, lean_rows("resolveVectors"));
}
