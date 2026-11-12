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

//! `quirin-audit compare` (card **W45-CONTRACT-REVISION-MODE**): the change
//! report between two audit runs — two revisions of the audited source, two
//! contract sets, or both.
//!
//! ```text
//! quirin-audit compare --before REPORT --after REPORT [--output FILE]
//! ```
//!
//! `REPORT` is a run's `resource-diagnostics.json`; its `.contracts.json`
//! sidecar is read beside it. The report lists, beyond verdict totals:
//!
//! * **sources** — selected units added and removed, and the units each run
//!   could not analyse because the source or its compile record was missing;
//! * **contracts** — contracts in force added, removed and changed (text
//!   digest, mode or status), from each run's `contract_revision` section;
//! * **pin mismatches** — each run's, and which are new and which resolved;
//! * **tip tracking** (card CONTRACT-PIN-TIP-MODE) — each run's tip entries
//!   with their standing, ref, resolved commit and notice, and the tips that
//!   moved between the runs;
//! * **assumptions** — flexible trusted contracts added and removed, and the
//!   standing libc assumption of each run;
//! * **obligations** — per function, the diagnostics (category and
//!   description; lines excluded, since lines move between revisions) that
//!   appeared and disappeared;
//! * **verdict moves** — per unit and per function, classified so that
//!   *violation detected* (a move to `Unsafe`) is never pooled with
//!   *coverage lost* (a move from `Safe` to `Pending` or `Unknown`), and a lost
//!   coverage names the reason codes of its refusals (`contract-pin-mismatch`
//!   among them). Card W45-USER-CONTRACT-PLUGINS: the four-value verdict. A
//!   move between `Unknown` and `Pending` is classed on its own
//!   (`unknown_to_pending`, `pending_to_unknown`), and each run's verdict
//!   tallies, `Pending` included, are reported beside the moves.
//!
//! Inventory only: it reads two finished runs and decides nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qcore_c_gateway::legacy_c::pin_hold;
use qcore_schemas::sha256_hex;
use serde_json::{Value, json};

/// Usage text of `quirin-audit compare`.
pub const USAGE: &str =
    "usage: quirin-audit compare --before REPORT --after REPORT [--output FILE]";

/// What the change report means, carried in the report itself.
pub const MEANING: &str = "change report between two audit runs (card W45-CONTRACT-REVISION-MODE; four-value verdict, card W45-USER-CONTRACT-PLUGINS): `violation_detected` lists functions that moved to Unsafe and `coverage_lost` functions that moved from Safe to Pending or Unknown; the two are never pooled; `unknown_to_pending` and `pending_to_unknown` are moves between the two non-Safe undecided values, and Pending is never Safe; and a provisional Unsafe is not a validated upstream defect; obligations are per-function diagnostics compared as (category, description) multisets without lines; contracts are compared by status, mode and normalised text digest from each run's contract_revision section";

struct Run {
    path: PathBuf,
    report: Value,
    sidecar: Value,
    report_sha256: String,
    sidecar_sha256: String,
}

impl Run {
    fn read(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let sidecar_path = path.with_extension("contracts.json");
        let sidecar_bytes = fs::read(&sidecar_path)
            .map_err(|error| format!("{}: {error}", sidecar_path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            report: serde_json::from_slice(&bytes)
                .map_err(|error| format!("{}: {error}", path.display()))?,
            sidecar: serde_json::from_slice(&sidecar_bytes)
                .map_err(|error| format!("{}: {error}", sidecar_path.display()))?,
            report_sha256: sha256_hex(&bytes),
            sidecar_sha256: sha256_hex(&sidecar_bytes),
        })
    }

    fn revision_section(&self) -> Result<&Value, String> {
        let section = &self.sidecar["contract_revision"];
        if section.is_object() {
            Ok(section)
        } else {
            Err(format!(
                "{}: the contracts sidecar has no contract_revision section",
                self.path.display()
            ))
        }
    }

    fn record(&self) -> Value {
        json!({
            "report": self.path,
            "report_sha256": self.report_sha256,
            "contracts_sha256": self.sidecar_sha256,
            "revision": self.report["fluent_bit_revision"].clone(),
            "source": self.sidecar["contract_revision"]["source"].clone(),
        })
    }
}

/// A unit's key: library, file and its occurrence among units of that file
/// (static and shared configurations stay separate).
type UnitKey = (String, String, usize);

fn units(report: &Value) -> BTreeMap<UnitKey, &Value> {
    let mut seen = BTreeMap::<(String, String), usize>::new();
    let mut units = BTreeMap::new();
    for unit in report["units"].as_array().into_iter().flatten() {
        let library = unit["library"].as_str().unwrap_or_default().to_owned();
        let file = unit["file"].as_str().unwrap_or_default().to_owned();
        let occurrence = seen.entry((library.clone(), file.clone())).or_default();
        units.insert((library, file, *occurrence), unit);
        *occurrence += 1;
    }
    units
}

fn key_json(key: &UnitKey) -> Value {
    json!({"library": key.0, "file": key.1, "configuration": key.2})
}

/// Whether a unit could not be analysed because its source or compile record
/// was absent (an environment reason, never a statement about the source).
fn missing(unit: &Value) -> bool {
    unit["reason_code"] == "environment-unavailable"
}

fn obligations(function: &Value) -> BTreeMap<(String, String), usize> {
    let mut multiset = BTreeMap::new();
    for diagnostic in function["diagnostics"].as_array().into_iter().flatten() {
        let key = (
            diagnostic["category"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            diagnostic["description"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        );
        *multiset.entry(key).or_default() += 1;
    }
    multiset
}

fn functions(unit: &Value) -> BTreeMap<String, &Value> {
    unit["functions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|function| Some((function["function"].as_str()?.to_owned(), function)))
        .collect()
}

/// The reason codes behind a function's refusals, as far as the change report
/// names them: a held call for a pin mismatch, and otherwise the diagnostic
/// categories.
fn refusal_reasons(function: &Value) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    // Card W45-USER-CONTRACT-PLUGINS: a checker plugin's link names its own
    // reason codes.
    for link in function["plugin_links"].as_array().into_iter().flatten() {
        for reason in link["reasons"].as_array().into_iter().flatten() {
            if let Some(code) = reason["code"].as_str() {
                reasons.insert(code.to_owned());
            }
        }
    }
    for diagnostic in function["diagnostics"].as_array().into_iter().flatten() {
        let description = diagnostic["description"].as_str().unwrap_or_default();
        if pin_hold::held_call(description).is_some() {
            reasons.insert(pin_hold::CONTRACT_PIN_MISMATCH.to_owned());
        } else if qcore_c_gateway::legacy_c::held_call(description).is_some() {
            reasons.insert("awaiting-human-review".to_owned());
        } else if qcore_c_gateway::legacy_c::refuted_contract::held_call(description).is_some() {
            // Card W45-REFUTED-CONTRACT-WITHDRAWAL.
            reasons.insert(
                qcore_c_gateway::legacy_c::refuted_contract::CONTRACT_REFUTED_BY_BODY.to_owned(),
            );
        } else if qcore_c_gateway::legacy_c::caller_frame_hold::held_call(description).is_some() {
            // Card W48-RK-LOG-CALLER-FRAME (rule 32a).
            reasons.insert(
                qcore_c_gateway::legacy_c::caller_frame_hold::CALLER_FRAME_SELF_RELIANCE.to_owned(),
            );
        } else if let Some(category) = diagnostic["category"].as_str() {
            reasons.insert(category.to_owned());
        }
    }
    reasons.into_iter().collect()
}

fn classify(before: Option<&str>, after: Option<&str>) -> &'static str {
    match (before, after) {
        (Some(from), Some(to)) if from == to => "unchanged",
        (_, Some("Unsafe")) => "violation_detected",
        (Some("Unsafe"), _) => "violation_withdrawn",
        (Some("Safe"), Some("Unknown" | "Pending")) => "coverage_lost",
        (Some("Unknown" | "Pending"), Some("Safe")) => "coverage_gained",
        (Some("Unknown"), Some("Pending")) => "unknown_to_pending",
        (Some("Pending"), Some("Unknown")) => "pending_to_unknown",
        (None, Some(_)) => "added",
        (Some(_), None) => "removed",
        _ => "other",
    }
}

/// Card W45-USER-CONTRACT-PLUGINS: a run's unit and function verdicts,
/// tallied over the four values (every other unit status under its own
/// spelling). `Pending` is counted apart from `Safe` and `Unknown`.
fn verdict_tallies(report: &Value) -> Value {
    let mut units = BTreeMap::<String, u64>::new();
    let mut functions = BTreeMap::<String, u64>::new();
    for verdict in qcore_schemas::verdict::Verdict::ALL {
        units.insert(verdict.as_str().to_owned(), 0);
        functions.insert(verdict.as_str().to_owned(), 0);
    }
    for unit in report["units"].as_array().into_iter().flatten() {
        if let Some(status) = unit["status"].as_str() {
            *units.entry(status.to_owned()).or_default() += 1;
        }
        for function in unit["functions"].as_array().into_iter().flatten() {
            if let Some(status) = function["status"].as_str() {
                *functions.entry(status.to_owned()).or_default() += 1;
            }
        }
    }
    json!({"units": units, "functions": functions})
}

fn set_difference(before: &BTreeSet<String>, after: &BTreeSet<String>) -> Value {
    json!({
        "added": after.difference(before).collect::<Vec<_>>(),
        "removed": before.difference(after).collect::<Vec<_>>(),
    })
}

/// The change report between two finished runs.
///
/// # Errors
///
/// A run that does not read, or whose sidecar has no `contract_revision`
/// section, refuses.
pub fn compare(before: &Path, after: &Path) -> Result<Value, String> {
    let (before, after) = (Run::read(before)?, Run::read(after)?);
    let (before_section, after_section) = (before.revision_section()?, after.revision_section()?);
    // Sources.
    let (before_units, after_units) = (units(&before.report), units(&after.report));
    let before_keys = before_units.keys().cloned().collect::<BTreeSet<_>>();
    let after_keys = after_units.keys().cloned().collect::<BTreeSet<_>>();
    let missing_of = |units: &BTreeMap<UnitKey, &Value>| {
        units
            .iter()
            .filter(|(_, unit)| missing(unit))
            .map(|(key, unit)| {
                let mut record = key_json(key);
                record["reason"] = unit["reason"].clone();
                record
            })
            .collect::<Vec<_>>()
    };
    // Contracts.
    let inventory = |section: &Value| {
        section["contracts"]
            .as_object()
            .map(|map| {
                map.iter()
                    .map(|(name, record)| (name.clone(), record.clone()))
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default()
    };
    let (before_contracts, after_contracts) = (inventory(before_section), inventory(after_section));
    let in_force = |record: &Value| {
        !matches!(
            record["status"].as_str(),
            Some("not-in-force" | "pending-review")
        )
    };
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    let names = before_contracts
        .keys()
        .chain(after_contracts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in &names {
        let (old, new) = (before_contracts.get(name), after_contracts.get(name));
        let old_in = old.is_some_and(in_force);
        let new_in = new.is_some_and(in_force);
        let entry = json!({"function": name, "before": old, "after": new});
        // A contract changes with its mode, status or text digest; the
        // module key names where a run loaded it from, which two runs may
        // spell differently for the same text.
        let identity = |record: Option<&Value>| {
            record.map(|r| {
                (
                    r["mode"].clone(),
                    r["status"].clone(),
                    r["contract_sha256"].clone(),
                )
            })
        };
        match (old_in, new_in) {
            (false, true) => added.push(entry),
            (true, false) => removed.push(entry),
            (true, true) if identity(old) != identity(new) => changed.push(entry),
            _ => {}
        }
    }
    // Pin mismatches.
    let mismatches = |section: &Value| {
        section["pin_mismatches"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| Some((m["function"].as_str()?.to_owned(), m.clone())))
            .collect::<BTreeMap<_, _>>()
    };
    let (before_pins, after_pins) = (mismatches(before_section), mismatches(after_section));
    let pin_names = |pins: &BTreeMap<String, Value>| pins.keys().cloned().collect::<BTreeSet<_>>();
    // Assumptions.
    let trusted = |section: &Value| {
        section["assumptions"]["flexible_trusted"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|record| record["function"].as_str().map(str::to_owned))
            .collect::<BTreeSet<_>>()
    };
    let standing = |section: &Value| {
        section["assumptions"]["standing"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|record| json!({"name": record["name"], "contracts": record["contracts"]}))
            .collect::<Vec<_>>()
    };
    // Card CONTRACT-PIN-TIP-MODE: tip entries, from the pinned records that
    // carry a tip.
    let tips = |section: &Value| {
        section["pinned_in_force"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(section["pin_mismatches"].as_array().into_iter().flatten())
            .filter(|record| record.get("tip").is_some())
            .filter_map(|record| {
                let tip = &record["tip"];
                Some((
                    record["function"].as_str()?.to_owned(),
                    json!({
                        "function": record["function"],
                        "in_force": record.get("reason").is_none(),
                        "reason": record.get("reason"),
                        "repository": tip["repository"]["name"],
                        "ref": tip["ref"],
                        "release_label": tip["release_label"],
                        "resolved_commit": tip["resolved_commit"],
                        "notice": record["notice"],
                    }),
                ))
            })
            .collect::<BTreeMap<_, _>>()
    };
    let (before_tips, after_tips) = (tips(before_section), tips(after_section));
    let tip_moved = before_tips
        .iter()
        .filter_map(|(name, old)| {
            let new = after_tips.get(name)?;
            (old["resolved_commit"] != new["resolved_commit"]).then(|| {
                json!({
                    "function": name,
                    "ref": new["ref"],
                    "before": old["resolved_commit"],
                    "after": new["resolved_commit"],
                })
            })
        })
        .collect::<Vec<_>>();
    // Obligations and verdict moves.
    let mut obligation_added = Vec::new();
    let mut obligation_removed = Vec::new();
    let mut unit_moves = Vec::new();
    let mut classes: BTreeMap<&'static str, Vec<Value>> = BTreeMap::new();
    for key in before_keys.union(&after_keys) {
        let (old, new) = (before_units.get(key), after_units.get(key));
        let old_status = old.and_then(|unit| unit["status"].as_str());
        let new_status = new.and_then(|unit| unit["status"].as_str());
        if old_status != new_status {
            let mut record = key_json(key);
            record["before"] = json!(old_status);
            record["after"] = json!(new_status);
            record["class"] = json!(classify(old_status, new_status));
            unit_moves.push(record);
        }
        let old_functions = old.map(|unit| functions(unit)).unwrap_or_default();
        let new_functions = new.map(|unit| functions(unit)).unwrap_or_default();
        let function_names = old_functions
            .keys()
            .chain(new_functions.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for name in function_names {
            let (old_function, new_function) = (old_functions.get(&name), new_functions.get(&name));
            let old_obligations = old_function.map(|f| obligations(f)).unwrap_or_default();
            let new_obligations = new_function.map(|f| obligations(f)).unwrap_or_default();
            for ((category, description), count) in &new_obligations {
                let before_count = old_obligations
                    .get(&(category.clone(), description.clone()))
                    .copied()
                    .unwrap_or(0);
                if *count > before_count {
                    let mut record = key_json(key);
                    record["function"] = json!(name);
                    record["category"] = json!(category);
                    record["description"] = json!(description);
                    record["count"] = json!(count - before_count);
                    obligation_added.push(record);
                }
            }
            for ((category, description), count) in &old_obligations {
                let after_count = new_obligations
                    .get(&(category.clone(), description.clone()))
                    .copied()
                    .unwrap_or(0);
                if *count > after_count {
                    let mut record = key_json(key);
                    record["function"] = json!(name);
                    record["category"] = json!(category);
                    record["description"] = json!(description);
                    record["count"] = json!(count - after_count);
                    obligation_removed.push(record);
                }
            }
            let old_status = old_function.and_then(|f| f["status"].as_str());
            let new_status = new_function.and_then(|f| f["status"].as_str());
            let class = classify(old_status, new_status);
            if class == "unchanged" {
                continue;
            }
            let mut record = key_json(key);
            record["function"] = json!(name);
            record["before"] = json!(old_status);
            record["after"] = json!(new_status);
            if let Some(function) = new_function
                && matches!(new_status, Some("Unknown" | "Pending"))
            {
                record["reasons"] = json!(refusal_reasons(function));
            }
            classes.entry(class).or_default().push(record);
        }
    }
    let count = |class: &str| classes.get(class).map_or(0, Vec::len);
    let counts = json!({
        "units_added": after_keys.difference(&before_keys).count(),
        "units_removed": before_keys.difference(&after_keys).count(),
        "contracts_added": added.len(),
        "contracts_removed": removed.len(),
        "contracts_changed": changed.len(),
        "pin_mismatches_before": before_pins.len(),
        "pin_mismatches_after": after_pins.len(),
        "obligations_added": obligation_added.iter().map(|r| r["count"].as_u64().unwrap_or(0)).sum::<u64>(),
        "obligations_removed": obligation_removed.iter().map(|r| r["count"].as_u64().unwrap_or(0)).sum::<u64>(),
        "violation_detected": count("violation_detected"),
        "violation_withdrawn": count("violation_withdrawn"),
        "coverage_lost": count("coverage_lost"),
        "coverage_gained": count("coverage_gained"),
        "unknown_to_pending": count("unknown_to_pending"),
        "pending_to_unknown": count("pending_to_unknown"),
        "functions_added": count("added"),
        "functions_removed": count("removed"),
        "other_moves": count("other"),
        "tip_tracked_before": before_tips.len(),
        "tip_tracked_after": after_tips.len(),
        "tip_moved": tip_moved.len(),
    });
    Ok(json!({
        "schema_version": 1,
        "meaning": MEANING,
        "before": before.record(),
        "after": after.record(),
        "counts": counts,
        "sources": {
            "units": {
                "added": after_keys.difference(&before_keys).map(key_json).collect::<Vec<_>>(),
                "removed": before_keys.difference(&after_keys).map(key_json).collect::<Vec<_>>(),
                "common": before_keys.intersection(&after_keys).count(),
            },
            "missing": {"before": missing_of(&before_units), "after": missing_of(&after_units)},
        },
        "contracts_in_force": {"added": added, "removed": removed, "changed": changed},
        "pin_mismatches": {
            "before": before_pins.values().collect::<Vec<_>>(),
            "after": after_pins.values().collect::<Vec<_>>(),
            "new": pin_names(&after_pins).difference(&pin_names(&before_pins)).collect::<Vec<_>>(),
            "resolved": pin_names(&before_pins).difference(&pin_names(&after_pins)).collect::<Vec<_>>(),
        },
        "tip_tracking": {
            "before": before_tips.values().collect::<Vec<_>>(),
            "after": after_tips.values().collect::<Vec<_>>(),
            "moved": tip_moved,
        },
        "assumptions": {
            "flexible_trusted": set_difference(&trusted(before_section), &trusted(after_section)),
            "standing": {"before": standing(before_section), "after": standing(after_section)},
        },
        "obligations": {"added": obligation_added, "removed": obligation_removed},
        "verdicts": {
            "before": verdict_tallies(&before.report),
            "after": verdict_tallies(&after.report),
        },
        "verdict_moves": {
            "units": unit_moves,
            "violation_detected": classes.remove("violation_detected").unwrap_or_default(),
            "violation_withdrawn": classes.remove("violation_withdrawn").unwrap_or_default(),
            "coverage_lost": classes.remove("coverage_lost").unwrap_or_default(),
            "coverage_gained": classes.remove("coverage_gained").unwrap_or_default(),
            "unknown_to_pending": classes.remove("unknown_to_pending").unwrap_or_default(),
            "pending_to_unknown": classes.remove("pending_to_unknown").unwrap_or_default(),
            "functions_added": classes.remove("added").unwrap_or_default(),
            "functions_removed": classes.remove("removed").unwrap_or_default(),
            "other": classes.remove("other").unwrap_or_default(),
        },
    }))
}

/// The console summary of a change report.
#[must_use]
pub fn summary(report: &Value) -> String {
    let count = |key: &str| report["counts"][key].as_u64().unwrap_or_default();
    format!(
        "compare: contracts in force +{} -{} ~{}; pin mismatches {} -> {}; obligations +{} -{}; \
         violation detected {}, violation withdrawn {}, coverage lost {}, coverage gained {}, \
         unknown to pending {}, pending to unknown {}; units +{} -{}",
        count("contracts_added"),
        count("contracts_removed"),
        count("contracts_changed"),
        count("pin_mismatches_before"),
        count("pin_mismatches_after"),
        count("obligations_added"),
        count("obligations_removed"),
        count("violation_detected"),
        count("violation_withdrawn"),
        count("coverage_lost"),
        count("coverage_gained"),
        count("unknown_to_pending"),
        count("pending_to_unknown"),
        count("units_added"),
        count("units_removed"),
    ) + &tip_summary(report)
}

/// Card CONTRACT-PIN-TIP-MODE: the tip clause and each run's notices, only
/// when either run tracked a tip.
fn tip_summary(report: &Value) -> String {
    let count = |key: &str| report["counts"][key].as_u64().unwrap_or_default();
    if count("tip_tracked_before") == 0 && count("tip_tracked_after") == 0 {
        return String::new();
    }
    let mut text = format!(
        "; tip-tracked contracts {} -> {} ({} tip(s) moved)",
        count("tip_tracked_before"),
        count("tip_tracked_after"),
        count("tip_moved")
    );
    for (run, key) in [("before", "before"), ("after", "after")] {
        for record in report["tip_tracking"][key].as_array().into_iter().flatten() {
            text.push_str(&format!(
                "\ncompare notice ({run}) [{}]: {}",
                record["notice"]["severity"].as_str().unwrap_or_default(),
                record["notice"]["message"].as_str().unwrap_or_default()
            ));
        }
    }
    text
}

/// Entry point of `quirin-audit compare`.
pub fn main(arguments: Vec<String>) -> ExitCode {
    let mut before = None;
    let mut after = None;
    let mut output = None;
    let mut iter = arguments.into_iter();
    while let Some(argument) = iter.next() {
        let slot = match argument.as_str() {
            "--before" => &mut before,
            "--after" => &mut after,
            "--output" => &mut output,
            _ => {
                eprintln!("unknown argument `{argument}`\n{USAGE}");
                return ExitCode::FAILURE;
            }
        };
        let Some(value) = iter.next() else {
            eprintln!("{argument} needs a value\n{USAGE}");
            return ExitCode::FAILURE;
        };
        *slot = Some(PathBuf::from(value));
    }
    let (Some(before), Some(after)) = (before, after) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let report = match compare(&before, &after) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("compare refused: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", summary(&report));
    let text = match serde_json::to_string_pretty(&report) {
        Ok(text) => text + "\n",
        Err(error) => {
            eprintln!("compare refused: {error}");
            return ExitCode::FAILURE;
        }
    };
    match output {
        Some(path) => {
            if let Err(error) = fs::write(&path, text) {
                eprintln!("compare refused: {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
        None => print!("{text}"),
    }
    ExitCode::SUCCESS
}
