//! V06 / R07 probe (b): the i128 observation literal parser in `qcore-contract`.
//!
//! This binary is EXTERNAL to the workspace. It reaches the predicate parser
//! only through the public, loadable route
//! `qcore_contract::contract_observation::check_contract_observation`, which
//! parses a real `.qcore` extern contract and evaluates each clause. The
//! private `evaluate` entry point is not used here, so every row below is
//! evidence about the shipped public boundary, not an internal shortcut.
//!
//! Row outcomes are classified into three disjoint buckets:
//!   CHECKED   - the predicate parsed and evaluated to true
//!   VIOLATION - the predicate parsed and evaluated to false (accepted spelling)
//!   REJECTED  - parse/range/overflow refusal (fail-closed)

use qcore_contract::contract_observation::{ObservationVerdict, check_contract_observation};
use std::collections::BTreeMap;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Outcome {
    Checked,
    Violation,
    Rejected,
}

fn classify(predicate: &str, values: &BTreeMap<String, i128>) -> (Outcome, String) {
    let source =
        format!("extern fn observed() -> unit contract {{ ensures normal {predicate}; }};");
    match check_contract_observation(&source, "observed", values) {
        Ok(ObservationVerdict::Checked { .. }) => (Outcome::Checked, String::new()),
        Ok(ObservationVerdict::OutsidePreconditions { predicate }) => {
            (Outcome::Violation, format!("outside preconditions: {predicate}"))
        }
        Err(message) if message.contains("observed contract violation") => {
            (Outcome::Violation, message)
        }
        Err(message) => (Outcome::Rejected, message),
    }
}

fn main() {
    let max = "170141183460469231731687303715884105727";
    let min = "-170141183460469231731687303715884105728";
    let values: BTreeMap<String, i128> = BTreeMap::from([
        ("minimum".to_owned(), i128::MIN),
        ("maximum".to_owned(), i128::MAX),
    ]);

    // (id, expected outcome, predicate)
    let rows: Vec<(&str, Outcome, String)> = vec![
        // --- extrema accept ---
        ("B01", Outcome::Checked, format!("maximum == {max}")),
        ("B02", Outcome::Checked, format!("minimum == {min}")),
        // --- extrema +/- 1 reject ---
        ("B03", Outcome::Rejected, "0 == 170141183460469231731687303715884105728".into()),
        ("B04", Outcome::Rejected, "0 == -170141183460469231731687303715884105729".into()),
        // --- zero and negative zero accept ---
        ("B05", Outcome::Checked, "0 == 0".into()),
        ("B06", Outcome::Checked, "-0 == 0".into()),
        ("B07", Outcome::Checked, "-0 == -0".into()),
        // --- sign spellings ---
        ("B08", Outcome::Checked, format!("- 170141183460469231731687303715884105728 == minimum")),
        ("B09", Outcome::Rejected, "--170141183460469231731687303715884105728 == 0".into()),
        ("B10", Outcome::Checked, "-(-170141183460469231731687303715884105727) == maximum".into()),
        ("B11", Outcome::Rejected, "-(-170141183460469231731687303715884105728) == 0".into()),
        ("B12", Outcome::Rejected, "+170141183460469231731687303715884105727 == maximum".into()),
        // --- leading zeros ---
        ("B13", Outcome::Checked, "000170141183460469231731687303715884105727 == maximum".into()),
        ("B14", Outcome::Rejected, "000170141183460469231731687303715884105728 == 0".into()),
        ("B15", Outcome::Checked, "-000170141183460469231731687303715884105728 == minimum".into()),
        ("B16", Outcome::Rejected, "-000170141183460469231731687303715884105729 == 0".into()),
        ("B17", Outcome::Checked, "0000 == 0".into()),
        // --- constant arithmetic with a supplied minimum-valued fact ---
        ("B18", Outcome::Rejected, "-minimum == maximum".into()),
        ("B19", Outcome::Rejected, "minimum - 1 == 0".into()),
        ("B20", Outcome::Rejected, "0 - minimum == 0".into()),
        ("B21", Outcome::Rejected, "minimum * -1 == 0".into()),
        ("B22", Outcome::Rejected, "minimum + minimum == 0".into()),
        ("B23", Outcome::Checked, format!("minimum + 1 == -170141183460469231731687303715884105727")),
        ("B24", Outcome::Checked, "minimum - minimum == 0".into()),
        ("B25", Outcome::Checked, "-(minimum + 1) == maximum".into()),
        // --- constant arithmetic at the maximum ---
        ("B26", Outcome::Rejected, "maximum + 1 == 0".into()),
        ("B27", Outcome::Rejected, "maximum * 2 == 0".into()),
        ("B28", Outcome::Rejected, "maximum + maximum == 0".into()),
        ("B29", Outcome::Checked, "maximum - maximum == 0".into()),
        ("B30", Outcome::Checked, "-maximum == minimum + 1".into()),
        ("B31", Outcome::Rejected, format!("{max} * {max} == 0")),
        ("B32", Outcome::Rejected, format!("{min} - 1 == 0")),
        ("B33", Outcome::Rejected, format!("{max} + 1 == 0")),
        // --- unsupported operators stay unsupported (no widening) ---
        ("B34", Outcome::Rejected, "minimum / -1 == 0".into()),
        ("B35", Outcome::Rejected, "minimum % -1 == 0".into()),
        // --- ordering / comparison at the extrema ---
        ("B36", Outcome::Checked, "minimum < minimum + 1".into()),
        ("B37", Outcome::Checked, format!("minimum <= {min}")),
        ("B38", Outcome::Checked, format!("maximum >= {max}")),
        ("B39", Outcome::Checked, "minimum != 0".into()),
        ("B40", Outcome::Checked, "minimum < maximum".into()),
        // --- i64 boundary is not an i128 parser boundary ---
        ("B41", Outcome::Checked, "9223372036854775808 == 9223372036854775808".into()),
        ("B42", Outcome::Checked, "-9223372036854775809 < 0".into()),
        // --- a parsed but false extremum predicate is a violation, not acceptance ---
        ("B43", Outcome::Violation, format!("minimum == {max}")),
        ("B44", Outcome::Violation, "maximum == 0".into()),
        // --- non-decimal / separator spellings reject ---
        ("B45", Outcome::Rejected, "0x7fffffff == 0".into()),
        ("B46", Outcome::Rejected, "170_141 == 0".into()),
        ("B47", Outcome::Rejected, "1e3 == 0".into()),
    ];

    let mut mismatches = 0usize;
    println!(
        "{:<5} {:<10} {:<10} {:<9} {}",
        "ID", "EXPECT", "ACTUAL", "RESULT", "PREDICATE / MESSAGE"
    );
    for (id, expect, predicate) in &rows {
        let (actual, message) = classify(predicate, &values);
        let verdict = if actual == *expect {
            "MATCH"
        } else {
            mismatches += 1;
            "MISMATCH"
        };
        let mut detail = predicate.clone();
        if !message.is_empty() {
            detail.push_str("  <= ");
            detail.push_str(&message);
        }
        detail.truncate(150);
        println!(
            "{id:<5} {:<10} {:<10} {verdict:<9} {detail}",
            format!("{expect:?}"),
            format!("{actual:?}")
        );
    }

    // Facts-side boundary: the minimum-valued fact itself, supplied as data.
    let fact_rows: [(&str, i128, Outcome); 4] = [
        ("F01", i128::MIN, Outcome::Checked),
        ("F02", i128::MIN + 1, Outcome::Violation),
        ("F03", i128::MAX, Outcome::Violation),
        ("F04", 0, Outcome::Violation),
    ];
    for (id, fact, expect) in fact_rows {
        let supplied = BTreeMap::from([("minimum".to_owned(), fact)]);
        let (actual, message) = classify(&format!("minimum == {min}"), &supplied);
        let verdict = if actual == expect {
            "MATCH"
        } else {
            mismatches += 1;
            "MISMATCH"
        };
        println!(
            "{id:<5} {:<10} {:<10} {verdict:<9} supplied minimum = {fact}  {message}",
            format!("{expect:?}"),
            format!("{actual:?}")
        );
    }

    // A missing observation is an error, not a skip, even at the extrema.
    let (actual, message) = classify(&format!("minimum == {min}"), &BTreeMap::new());
    let verdict = if actual == Outcome::Rejected {
        "MATCH"
    } else {
        mismatches += 1;
        "MISMATCH"
    };
    println!(
        "{:<5} {:<10} {:<10} {verdict:<9} no facts supplied  {message}",
        "F05",
        "Rejected",
        format!("{actual:?}")
    );

    println!("\nrows = {}, mismatches = {mismatches}", rows.len() + 5);
    if mismatches > 0 {
        std::process::exit(1);
    }
}
