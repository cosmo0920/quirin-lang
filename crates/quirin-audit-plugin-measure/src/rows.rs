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

//! A route's published rows, read through the composition interface.
//!
//! Card AUDIT-COMPOSITION writes every unit report beside a component sidecar
//! (`<report>.components.json`, [`ComponentSidecar`]) and renders the report's
//! unit rows from it. This module reads both, validates the sidecar, derives
//! every unit row's verdict with [`Contribution::derive`] (the join over its
//! components) and checks that each unit row of the report is exactly the row
//! the sidecar renders. A disagreement refuses the route: the tool never
//! compares a report its own components do not support.
//!
//! The keys are the components' [`RowKey`]s: `(library, file)`, the row
//! (`unit#n`, so a unit replayed under several configurations stays distinct),
//! the property and the profile. A function row is keyed by its unit row's key,
//! its function name and its occurrence within that unit row.
//!
//! A report written before AUDIT-COMPOSITION has no sidecar. Reading it is
//! allowed only on request (`--legacy-rows`), with the same keys built the
//! way `report_components::sidecar` builds them and the record saying so.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use qcore_schemas::composition::{ComponentSidecar, Contribution, RowKey, UnitRef};
use qcore_schemas::verdict::Verdict;
use serde_json::Value;

use crate::util::read_json;

/// The property every unit row decides (`projection::hypothesis_context`).
pub const PROPERTY: &str = "resource-custody.v1";

/// A unit row: its key, derived verdict and the row itself.
#[derive(Clone, Debug)]
pub struct UnitRow {
    pub key: RowKey,
    pub verdict: Verdict,
    pub conflicted: bool,
    pub row: Value,
}

/// A function row.
#[derive(Clone, Debug)]
pub struct FunctionRow {
    pub unit: usize,
    pub function: String,
    pub occurrence: usize,
    pub row: Value,
}

impl FunctionRow {
    #[must_use]
    pub fn status(&self) -> &str {
        self.row["status"].as_str().unwrap_or("(none)")
    }
}

/// A route's rows.
#[derive(Clone, Debug)]
pub struct Rows {
    /// `sidecar` or `report-units (no sidecar: pre-composition revision)`.
    pub source: String,
    pub units: Vec<UnitRow>,
    /// Keyed by [`function_key`].
    pub functions: BTreeMap<String, FunctionRow>,
    pub document: Value,
    /// Every distinct rule off-list the components' provenance records
    /// (empty for a report without a sidecar).
    pub rule_off_lists: BTreeSet<BTreeSet<String>>,
    /// The sidecar's contribution (`None` without a sidecar).
    pub contribution: Option<Contribution>,
}

/// The printable key of a unit row.
#[must_use]
pub fn unit_key(key: &RowKey) -> String {
    format!(
        "{}::{}::{}::{}::{}",
        key.unit.library, key.unit.file, key.row, key.property, key.profile
    )
}

/// The printable key of a function row.
#[must_use]
pub fn function_key(key: &RowKey, function: &str, occurrence: usize) -> String {
    format!("{}::{function}#{occurrence}", unit_key(key))
}

fn legacy_key(row: &Value, seen: &mut BTreeMap<(String, String), usize>) -> Result<RowKey, String> {
    let library = row["library"]
        .as_str()
        .ok_or("a unit row without a library")?;
    let file = row["file"].as_str().ok_or("a unit row without a file")?;
    let count = seen
        .entry((library.to_owned(), file.to_owned()))
        .or_insert(0);
    let key = RowKey {
        unit: UnitRef {
            library: library.into(),
            file: file.into(),
        },
        row: format!("unit#{count}"),
        property: PROPERTY.into(),
        profile: library.into(),
    };
    *count += 1;
    Ok(key)
}

/// A unit status as a verdict, as `checkpoint::unit_verdict` reads it: a
/// verdict spelling is itself, `data_only` is `Safe`, anything else `Unknown`.
#[must_use]
pub fn unit_verdict(status: &str) -> Verdict {
    match status.parse::<Verdict>() {
        Ok(verdict) => verdict,
        Err(_) if status == "data_only" => Verdict::Safe,
        Err(_) => Verdict::Unknown,
    }
}

/// Read a route's report and its sidecar.
///
/// # Errors
///
/// A missing or malformed report; a missing sidecar without `legacy`; a
/// sidecar that fails validation, does not match the report's row count, or
/// renders a row other than the report's.
pub fn read(report: &Path, legacy: bool) -> Result<Rows, String> {
    let mut document = read_json(report)?;
    let units = document
        .get_mut("units")
        .and_then(Value::as_array_mut)
        .map(std::mem::take)
        .ok_or_else(|| format!("{}: no `units` array", report.display()))?;
    let sidecar_path = report.with_extension("components.json");
    let mut rule_off_lists = BTreeSet::new();
    let mut contribution = None;
    let (source, keyed) = if sidecar_path.is_file() {
        let sidecar: ComponentSidecar = serde_json::from_value(read_json(&sidecar_path)?)
            .map_err(|error| format!("{}: {error}", sidecar_path.display()))?;
        sidecar
            .validate()
            .map_err(|error| format!("{}: {error}", sidecar_path.display()))?;
        if sidecar.order.len() != units.len() {
            return Err(format!(
                "{}: {} rows presented, the report has {}",
                sidecar_path.display(),
                sidecar.order.len(),
                units.len()
            ));
        }
        rule_off_lists = sidecar
            .contribution
            .components()
            .map(|component| component.provenance.rule_set.off.clone())
            .collect();
        let mut keyed = Vec::new();
        for (index, (key, row)) in sidecar.order.iter().zip(units).enumerate() {
            let derivation = sidecar
                .contribution
                .derive(key)
                .ok_or_else(|| format!("row {index}: presented without a component"))?;
            let conflicted = derivation.is_conflicted();
            if let ([only], false) = (derivation.components.as_slice(), conflicted)
                && only.detail != row
            {
                return Err(format!(
                    "{}: unit row {index} ({}) is not the row its component renders",
                    report.display(),
                    unit_key(key)
                ));
            }
            if conflicted && unit_verdict(row["status"].as_str().unwrap_or("")) < derivation.verdict
            {
                return Err(format!(
                    "{}: conflicted unit row {index} publishes a verdict below its derivation",
                    report.display()
                ));
            }
            keyed.push(UnitRow {
                key: key.clone(),
                verdict: derivation.verdict,
                conflicted,
                row,
            });
        }
        contribution = Some(sidecar.contribution);
        ("sidecar".to_owned(), keyed)
    } else if legacy {
        let mut seen = BTreeMap::new();
        let mut keyed = Vec::new();
        for row in units {
            let key = legacy_key(&row, &mut seen)?;
            let verdict = unit_verdict(row["status"].as_str().unwrap_or(""));
            keyed.push(UnitRow {
                key,
                verdict,
                conflicted: false,
                row,
            });
        }
        (
            "report-units (no component sidecar: a pre-composition revision, read on request)"
                .to_owned(),
            keyed,
        )
    } else {
        return Err(format!(
            "{} has no component sidecar {}; a report written before AUDIT-COMPOSITION is \
             read only with --legacy-rows",
            report.display(),
            sidecar_path.display()
        ));
    };
    let mut functions = BTreeMap::new();
    for (index, unit) in keyed.iter().enumerate() {
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for row in unit.row["functions"].as_array().into_iter().flatten() {
            let function = row["function"]
                .as_str()
                .ok_or_else(|| format!("{}: a function row without a name", unit_key(&unit.key)))?
                .to_owned();
            let occurrence = seen.entry(function.clone()).or_insert(0);
            let key = function_key(&unit.key, &function, *occurrence);
            functions.insert(
                key,
                FunctionRow {
                    unit: index,
                    function,
                    occurrence: *occurrence,
                    row: row.clone(),
                },
            );
            *occurrence += 1;
        }
    }
    Ok(Rows {
        source,
        units: keyed,
        functions,
        document,
        rule_off_lists,
        contribution,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_keys_count_repeated_units() {
        let mut seen = BTreeMap::new();
        let row = serde_json::json!({"library": "l", "file": "a.c"});
        assert_eq!(legacy_key(&row, &mut seen).unwrap().row, "unit#0");
        assert_eq!(legacy_key(&row, &mut seen).unwrap().row, "unit#1");
        let other = serde_json::json!({"library": "l", "file": "b.c"});
        assert_eq!(legacy_key(&other, &mut seen).unwrap().row, "unit#0");
    }

    #[test]
    fn unit_statuses_read_as_the_checkpoint_does() {
        assert_eq!(unit_verdict("Safe"), Verdict::Safe);
        assert_eq!(unit_verdict("data_only"), Verdict::Safe);
        assert_eq!(unit_verdict("gateway_failure"), Verdict::Unknown);
        assert_eq!(unit_verdict("Unsafe"), Verdict::Unsafe);
    }
}
