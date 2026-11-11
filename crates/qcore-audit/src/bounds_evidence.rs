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

//! Body evidence for bounds declarations on the audit routes, its per-run
//! standing, the bounds `Safe` roll-up and the bounds refutation table (card
//! **CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS**, gateway rules 14f–14h,
//! contract specification C09.1 in C06.4's and C06.5's shape).
//!
//! Every unit's bounds analysis checks, in its first phase, the body of each
//! declaration the unit defines — a `static inline` definition in every unit
//! that includes its header, or a main-source definition — under that
//! declaration ([`qcore_c_gateway::index_extent::DeclarationBodyEvidence`]).
//! Header definitions have no row on the audit routes (C09.1: the routes
//! report each unit's main-source definitions), so this is where their check
//! is published: as **evidence for the declaration**, never as a verdict row.
//!
//! * [`unit_record`] keeps one unit's evidence (with its library and file);
//!   the report folds the units in unit order.
//! * [`section`] folds every unit's evidence per declaration
//!   ([`qcore_core::bounds_body_standing::standing`], rules 13g–13j's shape:
//!   `validated` only where every examining unit validated it and every clause
//!   is covered in every one), lists every declaration of the environment
//!   (one no unit examined is a declared assumption), and rolls up the bounds
//!   `Safe` rows by the declarations they could have consulted (each row's
//!   `contract_dependencies`, an over-approximation).
//! * [`refutation_table`] is the union of every unit's refuting bodies: the
//!   routes' second bounds phase (C06.4's two phases) withdraws each from every
//!   unit's callers through `BoundsContracts::refuted`.
//!
//! No verdict reads any of it. The Lean model is `QCore.BoundsBodyStanding`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::index_extent::BoundsUnitReport;
use qcore_c_gateway::index_extent::bounds_body_standing::{
    self as standing, Rollup, Standing, UnitOutcome,
};
use qcore_contract::bounds_contract::BoundsContracts;
use serde_json::{Value, json};

/// What the section means, carried in the section itself.
pub const MEANING: &str = "body evidence for bounds declarations (card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS, gateway rules 14f-14h): each unit's first bounds phase checks the body of every declaration the unit defines (a static inline header definition in every unit that includes it, or a main-source definition) under that declaration; header definitions have no row on the audit routes, so the check is published here as evidence for the declaration, never as a verdict row. A body whose own row is Safe and does not refute covers its quant checks and preconditions; a frame clause is covered only by a positive frame check of its parameter; a derived relation, a return extent, bounds_preserves_extents() and a global extent are covered by nothing. A declaration is validated only where every unit that examined a body validated it and every clause is covered in every one; partly-validated when every examining unit validated it and some clauses are covered everywhere; held when held by a review marker or a pin; a declared-assumption otherwise (a refuting body, an unvalidated body or no examined body). The Safe roll-up classes each bounds Safe row by the declarations its analysis could have consulted (its own name and every called declared name, an over-approximation). No verdict reads any of it";

/// One unit's evidence, for the report.
#[must_use]
pub fn unit_record(library: &str, file: &str, report: &BoundsUnitReport) -> Option<Value> {
    (!report.declaration_bodies.is_empty()).then(|| {
        json!({
            "library": library,
            "file": file,
            "declarations": report.declaration_bodies,
        })
    })
}

/// The declarations some unit's body refutes (the bounds refutation table).
#[must_use]
pub fn refutation_table(units: &[Value]) -> BTreeSet<String> {
    let mut table = BTreeSet::new();
    for unit in units {
        for evidence in unit["declarations"].as_array().into_iter().flatten() {
            if evidence["refutes"].as_bool() == Some(true)
                && let Some(name) = evidence["function"].as_str()
            {
                table.insert(name.to_owned());
            }
        }
    }
    table
}

fn outcome_of(evidence: &Value) -> UnitOutcome {
    match evidence["outcome"].as_str() {
        Some("held") => UnitOutcome::Held,
        Some("failed") => UnitOutcome::Failed,
        Some("validated") => UnitOutcome::Validated(
            evidence["covered"]
                .as_object()
                .map(|covered| covered.keys().cloned().collect())
                .unwrap_or_default(),
        ),
        _ => UnitOutcome::Unvalidated,
    }
}

/// The section of one run (module documentation): `units` are the unit
/// records in unit order, `rows` the bounds rows, `contracts` the installed
/// environment.
#[must_use]
pub fn section(units: &[Value], rows: &[Value], contracts: &BoundsContracts) -> Value {
    let mut examined: BTreeMap<String, Vec<(UnitOutcome, Value)>> = BTreeMap::new();
    for unit in units {
        for evidence in unit["declarations"].as_array().into_iter().flatten() {
            let Some(name) = evidence["function"].as_str() else {
                continue;
            };
            examined.entry(name.to_owned()).or_default().push((
                outcome_of(evidence),
                json!({
                    "library": unit["library"],
                    "file": unit["file"],
                    "outcome": evidence["outcome"],
                    "body_verdict": evidence["body_verdict"],
                    "covered": evidence["covered"],
                }),
            ));
        }
    }
    let mut standings = BTreeMap::new();
    let mut declarations = serde_json::Map::new();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (name, contract) in &contracts.functions {
        let clauses = contract
            .clauses
            .iter()
            .map(|e| e.text.clone())
            .collect::<BTreeSet<_>>();
        let units = examined.get(name).cloned().unwrap_or_default();
        let outcomes = units.iter().map(|(o, _)| o.clone()).collect::<Vec<_>>();
        let found = standing::standing(contract.held(), &clauses, &outcomes);
        let covered_everywhere = clauses
            .iter()
            .filter(|clause| {
                !outcomes.is_empty()
                    && outcomes.iter().all(|outcome| {
                        matches!(outcome, UnitOutcome::Validated(covered) if covered.contains(*clause))
                    })
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        *counts.entry(found.as_str()).or_default() += 1;
        let mut record = json!({
            "module": contract.module,
            "contract_sha256": contract.text_sha256,
            "body_standing": found.as_str(),
            "clauses": clauses.iter().map(|clause| {
                (clause.clone(), json!(if matches!(found, Standing::Held) {
                    "held"
                } else if covered_everywhere.contains(clause) {
                    "validated"
                } else {
                    "uncovered"
                }))
            }).collect::<serde_json::Map<_, _>>(),
            "units_examined": units.len(),
            "units_validated": outcomes.iter().filter(|o| matches!(o, UnitOutcome::Validated(_))).count(),
            "units_failed": outcomes.iter().filter(|o| matches!(o, UnitOutcome::Failed)).count(),
            "units": units.iter().map(|(_, record)| record.clone()).collect::<Vec<_>>(),
        });
        if let Standing::PartlyValidated(uncovered) = &found {
            record["uncovered_clauses"] = json!(uncovered);
        }
        declarations.insert(name.clone(), record);
        standings.insert(name.clone(), found);
    }
    let mut rollup: BTreeMap<&str, usize> = [
        Rollup::NoDeclaration,
        Rollup::OnlyValidated,
        Rollup::PartlyValidated,
        Rollup::DeclaredAssumption,
    ]
    .into_iter()
    .map(|class| (class.as_str(), 0))
    .collect();
    for row in rows {
        if row["bounds"]["verdict"].as_str() != Some("Safe") {
            continue;
        }
        let dependencies = row["bounds"]["contract_dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| d.as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        *rollup
            .entry(standing::rollup_class(&dependencies, &standings).as_str())
            .or_default() += 1;
    }
    json!({
        "meaning": MEANING,
        "counts": {
            "declarations": contracts.functions.len(),
            "validated": counts.get("validated").copied().unwrap_or_default(),
            "partly_validated": counts.get("partly-validated").copied().unwrap_or_default(),
            "declared_assumption": counts.get("declared-assumption").copied().unwrap_or_default(),
            "held": counts.get("held").copied().unwrap_or_default(),
            "units_with_evidence": units.len(),
        },
        "declarations": declarations,
        "safe_rollup": rollup,
    })
}

/// The one-line console tally of a [`section`].
#[must_use]
pub fn tally(section: &Value) -> String {
    let count = |key: &str| section["counts"][key].as_u64().unwrap_or_default();
    let rolled = |key: &str| section["safe_rollup"][key].as_u64().unwrap_or_default();
    format!(
        "bounds body evidence: {} declaration(s): {} validated, {} partly validated, {} declared assumption(s), {} held; bounds Safe rows: {} on no declaration, {} only on validated, {} on a partly validated, {} on a declared assumption",
        count("declarations"),
        count("validated"),
        count("partly_validated"),
        count("declared_assumption"),
        count("held"),
        rolled(Rollup::NoDeclaration.as_str()),
        rolled(Rollup::OnlyValidated.as_str()),
        rolled(Rollup::PartlyValidated.as_str()),
        rolled(Rollup::DeclaredAssumption.as_str()),
    )
}
