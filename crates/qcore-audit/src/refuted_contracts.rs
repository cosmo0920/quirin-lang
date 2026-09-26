//! The refutation table, its two phases and its report section (card
//! **W45-REFUTED-CONTRACT-WITHDRAWAL**, contract spec §C06.4).
//!
//! # Why two phases
//!
//! A declared contract is refuted by its own body only where the defining unit
//! is analysed, and its callers may sit in any unit. Every audit route
//! therefore analyses its units twice at most:
//!
//! 1. **Phase one** analyses every unit under the loaded environment. Each
//!    unit refutes only what its own bodies contradict, holds those calls
//!    itself, and records each refutation on the defining function's row
//!    (`contract_refutations`). The table is the union over every row
//!    ([`table`]), a set: its value is independent of the unit order, of
//!    `--jobs` and of how units are grouped.
//! 2. **Phase two** runs only when the table is not empty. Every unit is
//!    analysed again with the table recorded in the environment
//!    (`ResourceContracts::set_refuted_by_body`), so every call site of a
//!    refuted declaration outside its own body is held, in every unit. Its
//!    rows are the published ones.
//!
//! With an empty table phase two would repeat phase one exactly (the hold is
//! the identity), so it is not run, and a run with no refutation costs one
//! pass, as before the card.
//!
//! # The interface a unit-composing checkpoint needs
//!
//! A phase-one unit result depends on its unit and the environment alone; a
//! phase-two result also on the table. A checkpoint that composes per-group
//! results (W45-AUDIT-CHECKPOINTS) must therefore (a) carry each unit's
//! `contract_refutations` rows, from which [`table`] recomputes the table by
//! union, and (b) join [`qcore_schemas::contract_refutation::table_digest`] of
//! the table it was analysed under to its identity, refusing to compose
//! phase-two results under different digests. The empty table has a digest,
//! so a phase-one result is keyed as "analysed under the empty table".

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::legacy_c::refuted_contract::{CONTRACT_REFUTED_BY_BODY, held_call};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_refutation::{
    ContractRefutation, RefutationKind, RefutationTable, insert_refutation, table_digest,
};
use serde_json::{Value, json};

use crate::report::AuditReport;

/// What the section means, carried in the section itself.
pub const MEANING: &str = "declared contracts of functions whose bodies this run analysed, checked against those bodies (contract spec C06.4): a declaration is refuted when a counted-user clause of the function's own row is refuted, or the validated summary of the body contradicts a declared custody duty (a refinement inside one duty is not a refutation); a refuted declaration stays in force for its own body, whose row keeps the refutation, and every other call site is held with reason code `contract-refuted-by-body`, which is Unknown, never Pending or Safe; the table is the union of every unit's refutations from a first pass under the loaded contracts, and a second pass applies it to every unit when it is not empty; `held_functions` counts analysed functions with at least one recorded held call, `held_call_sites` the held-call refusals recorded, a lower bound on textual call sites";

/// The refutation table of a finished report: the union of every function
/// row's `contract_refutations`, keyed by the function and named by its unit.
#[must_use]
pub fn table(units: &[Value]) -> RefutationTable {
    let mut table = RefutationTable::new();
    for unit in units {
        let file = unit["file"].as_str().unwrap_or_default();
        for function in unit["functions"].as_array().into_iter().flatten() {
            let name = function["function"].as_str().unwrap_or_default();
            for refutation in function["contract_refutations"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let kind = match refutation["kind"].as_str() {
                    Some("custody_duty") => RefutationKind::CustodyDuty,
                    _ => RefutationKind::CountedUserClause,
                };
                insert_refutation(
                    &mut table,
                    name,
                    ContractRefutation {
                        unit: file.to_owned(),
                        kind,
                        clause: refutation["clause"].as_str().unwrap_or_default().to_owned(),
                        origin: refutation["origin"].as_str().map(ToOwned::to_owned),
                        reason: refutation["reason"].as_str().unwrap_or_default().to_owned(),
                    },
                );
            }
        }
    }
    table
}

/// The outcome of [`run_two_phases`].
pub struct TwoPhases<R, E> {
    /// The published results, in task order: phase two's when it ran.
    pub results: Vec<R>,
    /// The error that stopped the published phase, at its sequential position.
    pub error: Option<E>,
    /// The table phase two applied; empty when only phase one ran.
    pub applied: RefutationTable,
}

/// Run one route's per-unit phase once under the empty table and, when it
/// completed and found a refutation, once more under that table.
///
/// `phase` analyses every unit under the table it is given and returns the
/// results in task order, stopped at the first error with that error; the
/// caller prints and folds the published results afterwards, so a phase that
/// is repeated prints nothing twice. `table_of` reads the table from results.
pub fn run_two_phases<R, E>(
    mut phase: impl FnMut(&RefutationTable) -> (Vec<R>, Option<E>),
    table_of: impl Fn(&[R]) -> RefutationTable,
) -> TwoPhases<R, E> {
    let empty = RefutationTable::new();
    let (results, error) = phase(&empty);
    if error.is_some() {
        return TwoPhases {
            results,
            error,
            applied: empty,
        };
    }
    let found = table_of(&results);
    if found.is_empty() {
        return TwoPhases {
            results,
            error,
            applied: empty,
        };
    }
    let (results, error) = phase(&found);
    TwoPhases {
        results,
        error,
        applied: found,
    }
}

/// `contracts` with `table` recorded, for phase two.
#[must_use]
pub fn with_table(contracts: &ResourceContracts, table: &RefutationTable) -> ResourceContracts {
    let mut contracts = contracts.clone();
    contracts.set_refuted_by_body(table);
    contracts
}

/// The `refuted_contracts` section for one finished report.
#[must_use]
pub fn section(applied: &RefutationTable, report: &AuditReport) -> Value {
    let mut call_sites = BTreeMap::<String, usize>::new();
    let mut functions = BTreeMap::<String, BTreeSet<(String, String, String)>>::new();
    for unit in report.unit_rows() {
        let library = unit["library"].as_str().unwrap_or_default();
        let file = unit["file"].as_str().unwrap_or_default();
        for function in unit["functions"].as_array().into_iter().flatten() {
            let name = function["function"].as_str().unwrap_or_default();
            for diagnostic in function["diagnostics"].as_array().into_iter().flatten() {
                let Some(callee) = diagnostic["description"].as_str().and_then(held_call) else {
                    continue;
                };
                *call_sites.entry(callee.to_owned()).or_default() += 1;
                functions.entry(callee.to_owned()).or_default().insert((
                    library.to_owned(),
                    file.to_owned(),
                    name.to_owned(),
                ));
            }
        }
    }
    // The published run's own refutations: phase one's when only it ran, a
    // subset of the applied table after phase two (a held call can only make
    // a refutation undecided, never create one).
    let published = table(report.unit_rows());
    let names = applied
        .keys()
        .chain(published.keys())
        .chain(call_sites.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let entries = names
        .iter()
        .map(|name| {
            json!({
                "function": name,
                "applied": applied.contains_key(name),
                "refutations": applied.get(name).or_else(|| published.get(name)),
                "held_functions": functions.get(name).map_or(0, BTreeSet::len),
                "held_call_sites": call_sites.get(name).copied().unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();
    let held_functions = functions.values().flatten().collect::<BTreeSet<_>>().len();
    json!({
        "meaning": MEANING,
        "reason_code": CONTRACT_REFUTED_BY_BODY,
        "phases": if applied.is_empty() { 1 } else { 2 },
        "table_sha256": table_digest(applied),
        "refuted_declarations": applied.len(),
        "published_refutations_sha256": table_digest(&published),
        "held_functions": held_functions,
        "held_call_sites": call_sites.values().sum::<usize>(),
        "entries": entries,
    })
}

/// The one-line console tally of a [`section`].
#[must_use]
pub fn tally(section: &Value) -> String {
    let count = |key: &str| section[key].as_u64().unwrap_or_default();
    format!(
        "refuted contracts: {} declaration(s) refuted by their own bodies, {} phase(s); {} \
         function(s) and {} call site(s) held Unknown ({CONTRACT_REFUTED_BY_BODY})",
        count("refuted_declarations"),
        count("phases"),
        count("held_functions"),
        count("held_call_sites"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(file: &str, function: &str, clause: &str) -> Value {
        json!({
            "library": "l", "file": file,
            "functions": [{
                "function": function, "status": "Unsafe", "diagnostics": [],
                "contract_refutations": [{
                    "kind": "counted_user_clause", "clause": clause, "line": 3,
                    "origin": format!("{file}:3"), "reason": "r",
                }],
            }],
        })
    }

    #[test]
    fn the_table_is_a_set_over_units_and_groups() {
        let a = unit("a.c", "f", "resource_user_exit(t, 0, added)");
        let b = unit("b.c", "g", "resource_user_exit(t, -1, restored)");
        // Two variants of one file record the same refutation once.
        let whole = table(&[a.clone(), b.clone(), a.clone()]);
        let reversed = table(&[b.clone(), a.clone()]);
        assert_eq!(whole, reversed);
        assert_eq!(table_digest(&whole), table_digest(&reversed));
        assert_eq!(whole["f"].len(), 1);
        // Groups composed by union equal the whole run.
        let grouped = qcore_schemas::contract_refutation::union(
            table(std::slice::from_ref(&a)),
            &table(std::slice::from_ref(&b)),
        );
        assert_eq!(grouped, whole);
    }

    #[test]
    fn phase_two_runs_only_on_a_refutation_and_applies_phase_one_table() {
        let mut seen = Vec::new();
        let outcome = run_two_phases(
            |table: &RefutationTable| {
                seen.push(table.len());
                (vec![unit("a.c", "f", "c")], None::<String>)
            },
            table,
        );
        assert_eq!(seen, vec![0, 1]);
        assert!(outcome.applied.contains_key("f"));
        let mut seen = Vec::new();
        let outcome = run_two_phases(
            |table: &RefutationTable| {
                seen.push(table.len());
                (
                    vec![json!({"file": "a.c", "functions": []})],
                    None::<String>,
                )
            },
            table,
        );
        assert_eq!(seen, vec![0]);
        assert!(outcome.applied.is_empty());
    }

    #[test]
    fn a_stopped_phase_one_is_published_as_it_stopped() {
        let mut runs = 0;
        let outcome = run_two_phases(
            |_: &RefutationTable| {
                runs += 1;
                (vec![unit("a.c", "f", "c")], Some("first error".to_owned()))
            },
            table,
        );
        assert_eq!(runs, 1);
        assert_eq!(outcome.error.as_deref(), Some("first error"));
    }

    #[test]
    fn the_empty_table_has_a_digest() {
        assert_eq!(table_digest(&RefutationTable::new()).len(), 64);
    }
}
