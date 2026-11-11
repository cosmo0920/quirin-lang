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
    table_from_rows(units)
}

/// Union borrowed report rows without cloning their diagnostic payloads.
pub fn table_from_rows<'a>(units: impl IntoIterator<Item = &'a Value>) -> RefutationTable {
    let mut table = RefutationTable::new();
    for unit in units {
        absorb_refutations(&mut table, unit);
    }
    table
}

/// Add one unit row's `contract_refutations` to `table`.
fn absorb_refutations(table: &mut RefutationTable, unit: &Value) {
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
                table,
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
    phase: impl FnMut(&RefutationTable) -> (Vec<R>, Option<E>),
    table_of: impl Fn(&[R]) -> RefutationTable,
) -> TwoPhases<R, E> {
    run_two_phases_with_bounds(phase, table_of, |_| BTreeSet::new())
}

/// [`run_two_phases`] with the bounds property's own table (card
/// **CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS**, rule 14h): `bounds_of`
/// reads the bounds declarations phase one's bodies refute
/// ([`crate::bounds_evidence::refutation_table`]). Phase two runs when either
/// table is not empty; the bounds table is installed in the bounds
/// environment first ([`crate::bounds_selection::apply_refutation_table`]),
/// so every unit's bounds rows are analysed again with each refuted
/// declaration withdrawn from its callers. With both tables empty nothing runs
/// twice, as before.
pub fn run_two_phases_with_bounds<R, E>(
    mut phase: impl FnMut(&RefutationTable) -> (Vec<R>, Option<E>),
    table_of: impl Fn(&[R]) -> RefutationTable,
    bounds_of: impl Fn(&[R]) -> BTreeSet<String>,
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
    let bounds = bounds_of(&results);
    if found.is_empty() && bounds.is_empty() {
        return TwoPhases {
            results,
            error,
            applied: empty,
        };
    }
    crate::bounds_selection::apply_refutation_table(&bounds);
    // The first pass will never be published now. Release its potentially large
    // reports before allocating the second pass, rather than retaining both.
    drop(results);
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

/// What the `refuted_contracts` section reads of a run's rows: the held calls
/// per callee and the rows' own refutations. Rows are absorbed one unit at a
/// time, so a streaming composer (`crate::compose`) builds exactly what the
/// whole run builds from its report.
#[derive(Clone, Debug, Default)]
pub struct RowTally {
    call_sites: BTreeMap<String, usize>,
    functions: BTreeMap<String, BTreeSet<(String, String, String)>>,
    published: RefutationTable,
}

impl RowTally {
    /// Absorb one unit row (`library`, `file`, `functions`).
    pub fn absorb(&mut self, unit: &Value) {
        let library = unit["library"].as_str().unwrap_or_default();
        let file = unit["file"].as_str().unwrap_or_default();
        for function in unit["functions"].as_array().into_iter().flatten() {
            let name = function["function"].as_str().unwrap_or_default();
            for diagnostic in function["diagnostics"].as_array().into_iter().flatten() {
                let Some(callee) = diagnostic["description"].as_str().and_then(held_call) else {
                    continue;
                };
                *self.call_sites.entry(callee.to_owned()).or_default() += 1;
                self.functions
                    .entry(callee.to_owned())
                    .or_default()
                    .insert((library.to_owned(), file.to_owned(), name.to_owned()));
            }
        }
        absorb_refutations(&mut self.published, unit);
    }

    /// The rows' own refutations: phase one's table when only it ran, a
    /// subset of the applied table after phase two (a held call can only
    /// make a refutation undecided, never create one).
    #[must_use]
    pub fn published(&self) -> &RefutationTable {
        &self.published
    }

    /// The section, given the table the published phase applied.
    #[must_use]
    pub fn section(&self, applied: &RefutationTable) -> Value {
        let names = applied
            .keys()
            .chain(self.published.keys())
            .chain(self.call_sites.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let entries = names
            .iter()
            .map(|name| {
                json!({
                    "function": name,
                    "applied": applied.contains_key(name),
                    "refutations": applied.get(name).or_else(|| self.published.get(name)),
                    "held_functions": self.functions.get(name).map_or(0, BTreeSet::len),
                    "held_call_sites": self.call_sites.get(name).copied().unwrap_or(0),
                })
            })
            .collect::<Vec<_>>();
        let held_functions = self
            .functions
            .values()
            .flatten()
            .collect::<BTreeSet<_>>()
            .len();
        json!({
            "meaning": MEANING,
            "reason_code": CONTRACT_REFUTED_BY_BODY,
            "phases": if applied.is_empty() { 1 } else { 2 },
            "table_sha256": table_digest(applied),
            "refuted_declarations": applied.len(),
            "published_refutations_sha256": table_digest(&self.published),
            "held_functions": held_functions,
            "held_call_sites": self.call_sites.values().sum::<usize>(),
            "entries": entries,
        })
    }
}

/// The `refuted_contracts` section for one finished report.
#[must_use]
pub fn section(applied: &RefutationTable, report: &AuditReport) -> Value {
    let mut tally = RowTally::default();
    for unit in report.unit_rows() {
        tally.absorb(unit);
    }
    tally.section(applied)
}

/// The key under which a run records the table it applies in its
/// assumptions before any unit is analysed, so a checkpoint's run-wide inputs
/// carry it to the composer; the finished section replaces it.
pub const APPLIED_KEY: &str = "refuted_contracts";

/// The applied table a run recorded under [`APPLIED_KEY`], or the empty
/// table when it recorded none.
///
/// # Errors
///
/// Refuses a recorded table that does not decode.
pub fn applied_table(assumptions: &Value) -> Result<RefutationTable, String> {
    match assumptions
        .get(APPLIED_KEY)
        .and_then(|record| record.get("applied"))
    {
        None => Ok(RefutationTable::new()),
        Some(table) => serde_json::from_value(table.clone())
            .map_err(|error| format!("the recorded refutation table does not decode: {error}")),
    }
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
    fn discarded_results_are_released_before_phase_two_starts() {
        use std::cell::Cell;
        struct Retained<'a>(&'a Cell<usize>);
        impl Drop for Retained<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() - 1);
            }
        }
        let live = Cell::new(0);
        let mut runs = 0;
        let outcome = run_two_phases(
            |_| {
                assert_eq!(live.get(), 0, "previous pass still retains its payload");
                runs += 1;
                live.set(1);
                (vec![Retained(&live)], None::<String>)
            },
            |_| table(&[unit("a.c", "f", "c")]),
        );
        assert_eq!(runs, 2);
        assert_eq!(live.get(), 1, "published results must remain available");
        drop(outcome);
        assert_eq!(live.get(), 0);
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
