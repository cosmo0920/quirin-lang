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

//! The row-by-row comparison of two sides of one route, and the byte
//! comparison of their report artifacts.
//!
//! Rows are matched by their component keys ([`crate::rows`]). For each
//! matched function row the comparison says whether it is byte-identical,
//! differs only in **environment digests** (digests over the contract
//! environment or the analyzer build that move on every row whenever the
//! sides' sources or build commits differ: the copied
//! `native_compare.py`'s list), changes content without changing status, or
//! **moves** (its status changes). A row present on one side only is
//! **unmatched**: it is named, stays in that side's denominator, and makes the
//! comparison incomplete. Totals are always counted from each side's own
//! report, never derived from the moves.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use qcore_schemas::composition::Contribution;
use serde_json::{Value, json};

use crate::declared::MeasuredMove;
use crate::rows::{Rows, unit_key};
use crate::util::{sha_file, tree_files};

/// Fields holding digests over the whole contract environment or build.
pub const ENVIRONMENT_DIGESTS: [&str; 5] = [
    "assumptions",
    "build_identity",
    "environment_sha256",
    "resource_contracts_sha256",
    "id",
];

/// The evidence-version prefix whose suffix is the analyzer build commit.
pub const EVIDENCE: &str = "contract-hypothesis-graph.v1:";

/// Records and logs a route directory holds beside its reports: they name
/// their side's paths, binary and times by design and are not report
/// artifacts.
#[must_use]
pub fn is_record(relative: &str) -> bool {
    let name = Path::new(relative)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    matches!(
        name,
        "invocation.json"
            | "console.log"
            | "stderr.log"
            | "audit.log"
            | "resource-diagnostics.invocation.json"
    ) || relative == "corpus/run.json"
        || name.ends_with(".exit")
        || name.ends_with(".slot.json")
}

/// A row with environment digests blanked, the build commit dropped from
/// every evidence version, and reading-site evidence compared as a multiset.
#[must_use]
pub fn normalize(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut out = serde_json::Map::new();
            for (key, item) in object {
                let normalized = if ENVIRONMENT_DIGESTS.contains(&key.as_str()) && item.is_string()
                {
                    json!("ENV")
                } else if key == "evidence_version"
                    && item.as_str().is_some_and(|text| text.starts_with(EVIDENCE))
                {
                    json!(format!("{EVIDENCE}BUILD"))
                } else {
                    normalize(item)
                };
                out.insert(key.clone(), normalized);
            }
            if let Some(Value::Array(items)) = out.get_mut("reading_site_evidence") {
                items.sort_by_key(Value::to_string);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(normalize).collect()),
        other => other.clone(),
    }
}

fn leaves(row: &Value) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for leaf in row["diagnostics"].as_array().into_iter().flatten() {
        let mut leaf = leaf.clone();
        if let Some(object) = leaf.as_object_mut() {
            object.remove("line");
        }
        *counts.entry(normalize(&leaf).to_string()).or_insert(0) += 1;
    }
    counts
}

fn multiset_minus(left: &BTreeMap<String, usize>, right: &BTreeMap<String, usize>) -> Vec<Value> {
    let mut out = Vec::new();
    for (leaf, count) in left {
        let other = right.get(leaf).copied().unwrap_or(0);
        for _ in other..*count {
            out.push(serde_json::from_str(leaf).unwrap_or(Value::Null));
        }
    }
    out
}

fn changed_fields(before: &Value, after: &Value, skip: &[&str]) -> Vec<String> {
    let empty = serde_json::Map::new();
    let left = before.as_object().unwrap_or(&empty);
    let right = after.as_object().unwrap_or(&empty);
    left.keys()
        .chain(right.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| !skip.contains(&key.as_str()) && left.get(*key) != right.get(*key))
        .cloned()
        .collect()
}

fn tally<'a>(statuses: impl Iterator<Item = &'a str>) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for status in statuses {
        *counts.entry(status.to_owned()).or_insert(0) += 1;
    }
    counts
}

/// The per-route totals of one side, counted from its report.
#[must_use]
pub fn totals(rows: &Rows) -> Value {
    json!({
        "function_rows": rows.functions.len(),
        "function_status": tally(rows.functions.values().map(crate::rows::FunctionRow::status)),
        "unit_rows": rows.units.len(),
        "unit_verdict": tally(rows.units.iter().map(|unit| unit.verdict.as_str())),
        "conflicted_unit_rows": rows.units.iter().filter(|unit| unit.conflicted).count(),
        "diagnostic_counts": rows.document.get("diagnostic_counts").cloned().unwrap_or(Value::Null),
        "row_source": rows.source,
    })
}

/// The comparison of one route, and the moves the declared check reads.
#[must_use]
pub fn rows(before: &Rows, after: &Rows) -> (Value, Vec<MeasuredMove>) {
    let mut identical = 0_u64;
    let mut environment_only = 0_u64;
    let mut moves = Vec::new();
    let mut content = Vec::new();
    let mut unmatched = Vec::new();
    let mut measured = Vec::new();
    let keys: BTreeSet<&String> = before
        .functions
        .keys()
        .chain(after.functions.keys())
        .collect();
    for key in keys {
        let (left, right) = (before.functions.get(key), after.functions.get(key));
        let (library, file, function) = {
            let (rows, row) = match (left, right) {
                (Some(row), _) => (before, row),
                (None, Some(row)) => (after, row),
                (None, None) => continue,
            };
            let unit = &rows.units[row.unit].key.unit;
            (
                unit.library.clone(),
                unit.file.clone(),
                row.function.clone(),
            )
        };
        let status = |row: Option<&crate::rows::FunctionRow>| {
            row.map_or_else(|| "absent".to_owned(), |row| row.status().to_owned())
        };
        let (status_before, status_after) = (status(left), status(right));
        let (Some(left), Some(right)) = (left, right) else {
            unmatched.push(json!({"row": key, "only": if left.is_some() { "before" } else { "after" },
                                  "status": if left.is_some() { &status_before } else { &status_after }}));
            measured.push(MeasuredMove {
                key: key.clone(),
                library,
                file,
                function,
                before: status_before,
                after: status_after,
            });
            continue;
        };
        if left.row == right.row {
            identical += 1;
            continue;
        }
        let (normal_left, normal_right) = (normalize(&left.row), normalize(&right.row));
        if normal_left == normal_right {
            environment_only += 1;
            continue;
        }
        let (leaves_left, leaves_right) = (leaves(&left.row), leaves(&right.row));
        let mut entry = json!({
            "row": key,
            "status": [status_before, status_after],
            "leaves_removed": multiset_minus(&leaves_left, &leaves_right),
            "leaves_added": multiset_minus(&leaves_right, &leaves_left),
            "other_fields_changed": changed_fields(&normal_left, &normal_right,
                &["status", "diagnostics", "contract_dependencies"]),
        });
        if normal_left["contract_dependencies"] != normal_right["contract_dependencies"] {
            entry["contract_dependencies"] = json!({
                "before": left.row["contract_dependencies"],
                "after": right.row["contract_dependencies"],
            });
        }
        if status_before == status_after {
            content.push(entry);
        } else {
            measured.push(MeasuredMove {
                key: key.clone(),
                library,
                file,
                function,
                before: status_before,
                after: status_after,
            });
            moves.push(entry);
        }
    }
    let mut unit_moves = Vec::new();
    let mut unit_fields = BTreeMap::new();
    let mut unit_environment_only = 0_u64;
    let index = |rows: &Rows| -> BTreeMap<String, usize> {
        rows.units
            .iter()
            .enumerate()
            .map(|(position, unit)| (unit_key(&unit.key), position))
            .collect()
    };
    let (units_before, units_after) = (index(before), index(after));
    let mut unmatched_units = Vec::new();
    for key in units_before
        .keys()
        .chain(units_after.keys())
        .collect::<BTreeSet<_>>()
    {
        match (units_before.get(key), units_after.get(key)) {
            (Some(&left), Some(&right)) => {
                let (left, right) = (&before.units[left], &after.units[right]);
                if left.verdict != right.verdict || left.conflicted != right.conflicted {
                    unit_moves.push(json!({"row": key,
                        "verdict": [left.verdict.as_str(), right.verdict.as_str()],
                        "conflicted": [left.conflicted, right.conflicted]}));
                }
                let strip = |row: &Value| {
                    let mut row = row.clone();
                    if let Some(object) = row.as_object_mut() {
                        object.remove("functions");
                    }
                    row
                };
                let (raw_left, raw_right) = (strip(&left.row), strip(&right.row));
                if raw_left != raw_right {
                    let fields = changed_fields(&normalize(&raw_left), &normalize(&raw_right), &[]);
                    if fields.is_empty() {
                        unit_environment_only += 1;
                    } else {
                        unit_fields.insert(key.clone(), fields);
                    }
                }
            }
            (left, _) => unmatched_units.push(json!({"row": key,
                "only": if left.is_some() { "before" } else { "after" }})),
        }
    }
    let conflicts = |rows: &Rows| -> Vec<String> {
        rows.units
            .iter()
            .filter(|unit| unit.conflicted)
            .map(|unit| unit_key(&unit.key))
            .collect()
    };
    let mut incomplete = Vec::new();
    if !unmatched.is_empty() || !unmatched_units.is_empty() {
        incomplete.push("unmatched rows (named; each stays in its side's denominator)");
    }
    if !conflicts(before).is_empty() || !conflicts(after).is_empty() {
        incomplete.push("composition-conflict rows (named; each derives at least Unknown)");
    }
    let record = json!({
        "status": if incomplete.is_empty() { "complete" } else { "incomplete" },
        "incomplete_because": incomplete,
        "totals": {"before": totals(before), "after": totals(after)},
        "function_rows": {
            "identical": identical,
            "environment_only": environment_only,
            "content_changed": content.len(),
            "moved": moves.len(),
            "unmatched": unmatched.len(),
        },
        "moves": moves,
        "content_changes": content,
        "unmatched": unmatched,
        "unit_moves": unit_moves,
        "unit_field_differences": unit_fields,
        "unit_rows_environment_only": unit_environment_only,
        "unmatched_units": unmatched_units,
        "conflicts": {"before": conflicts(before), "after": conflicts(after)},
        "environment_digest_fields": ENVIRONMENT_DIGESTS,
    });
    (record, measured)
}

/// The status-total deltas of a route (after minus before), counted from the
/// two reports.
#[must_use]
pub fn deltas(before: &Rows, after: &Rows) -> BTreeMap<String, i64> {
    let count = |rows: &Rows| {
        tally(
            rows.functions
                .values()
                .map(crate::rows::FunctionRow::status),
        )
    };
    let (left, right) = (count(before), count(after));
    left.keys()
        .chain(right.keys())
        .map(|status| {
            let difference = i64::try_from(right.get(status).copied().unwrap_or(0)).unwrap_or(0)
                - i64::try_from(left.get(status).copied().unwrap_or(0)).unwrap_or(0);
            (status.clone(), difference)
        })
        .collect()
}

fn lifetime_keys(document: &Value) -> BTreeSet<String> {
    document["callback_members"]["lifetime_protocols"]["evidence"]["summaries"]
        .as_object()
        .map(|summaries| summaries.keys().cloned().collect())
        .unwrap_or_default()
}

/// The one clock-dependent key of a contracts sidecar
/// (`resource-diagnostics.contracts.json`). Fluent Bit's
/// `lib/monkey/mk_bin/monkey.c` expands `__DATE__`/`__TIME__`, and the
/// lifetime-source summary key is a digest over the preprocessed text, so that
/// one key differs between any two runs not in the same second (ledger entries
/// LIFETIME-SOURCE-IDENTITY-TIME-MACROS, PREPASS-CLOCK-DEPENDENT-KEY).
///
/// The file is accepted only when exactly one lifetime-source summary key
/// differs, neither key's text occurs in the other file, and replacing every
/// occurrence of the first side's key by the second's turns the first file
/// into the second **as JSON and as a multiset of lines** (the key's place in
/// its sorted map moves with its text): the check of card AUDIT-COMPOSITION's
/// `compare_identity.py`, ported. Returns the named substitution, or why the
/// file is not that.
pub fn time_key(before: &[u8], after: &[u8]) -> Result<Value, Value> {
    let parse = |bytes: &[u8]| serde_json::from_slice::<Value>(bytes);
    let (Ok(left), Ok(right)) = (parse(before), parse(after)) else {
        return Err(json!({"why": "not JSON"}));
    };
    let (keys_left, keys_right) = (lifetime_keys(&left), lifetime_keys(&right));
    let only_left: Vec<&String> = keys_left.difference(&keys_right).collect();
    let only_right: Vec<&String> = keys_right.difference(&keys_left).collect();
    let ([old], [new]) = (only_left.as_slice(), only_right.as_slice()) else {
        return Err(json!({"lifetime_keys_only_before": only_left,
                          "lifetime_keys_only_after": only_right}));
    };
    let occurs = |haystack: &[u8], needle: &str| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle.as_bytes())
    };
    if occurs(after, old) || occurs(before, new) {
        return Err(json!({"collision": [old, new]}));
    }
    let swapped = String::from_utf8_lossy(before).replace(old.as_str(), new);
    let json_equal = serde_json::from_str::<Value>(&swapped).is_ok_and(|value| value == right);
    let mut lines_left: Vec<&str> = swapped.lines().collect();
    let after_text = String::from_utf8_lossy(after);
    let mut lines_right: Vec<&str> = after_text.lines().collect();
    lines_left.sort_unstable();
    lines_right.sort_unstable();
    let lines_equal = lines_left == lines_right;
    if !(json_equal && lines_equal) {
        return Err(
            json!({"pair": [old, new], "json_equal_after_swap": json_equal,
                          "lines_equal_after_swap": lines_equal}),
        );
    }
    Ok(json!({
        "substitution": "lifetime-source-time-key",
        "before_key": old,
        "after_key": new,
        "occurrences": String::from_utf8_lossy(before).matches(old.as_str()).count(),
        "json_equal_after_swap": true,
        "lines_equal_after_swap": true,
    }))
}

/// Compare every report artifact two route directories share, byte for byte;
/// records and logs are listed, not compared.
///
/// # Errors
///
/// An unreadable directory or file.
pub fn artifacts(before: &Path, after: &Path) -> Result<Value, String> {
    let list = |root: &Path| -> Result<BTreeSet<String>, String> {
        Ok(tree_files(root, &[])?
            .into_iter()
            .map(|path| path.display().to_string())
            .collect())
    };
    let (left, right) = (list(before)?, list(after)?);
    let mut identical = Vec::new();
    let mut substituted = BTreeMap::new();
    let mut differing = Vec::new();
    let mut records = Vec::new();
    for name in left.intersection(&right) {
        if is_record(name) {
            records.push(name.clone());
            continue;
        }
        let (a, b) = (sha_file(&before.join(name))?, sha_file(&after.join(name))?);
        if a == b {
            identical.push(json!({"file": name, "sha256": a}));
            continue;
        }
        if name.ends_with(".contracts.json") {
            let read = |path: &Path| {
                std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
            };
            match time_key(&read(&before.join(name))?, &read(&after.join(name))?) {
                Ok(swap) => {
                    substituted.insert(name.clone(), swap);
                }
                Err(why) => differing.push(json!({"file": name, "why": why})),
            }
            continue;
        }
        differing.push(json!({"file": name, "before": a, "after": b}));
    }
    let only = |a: &BTreeSet<String>, b: &BTreeSet<String>| -> Vec<String> {
        a.difference(b)
            .filter(|name| !is_record(name))
            .cloned()
            .collect()
    };
    let only_before = only(&left, &right);
    let only_after = only(&right, &left);
    let identical_all = differing.is_empty() && only_before.is_empty() && only_after.is_empty();
    Ok(json!({
        "byte_identical_except_named_substitutions": identical_all,
        "identical": identical,
        "identical_after_named_substitution": substituted,
        "differing": differing,
        "only_before": only_before,
        "only_after": only_after,
        "records_not_compared": records,
    }))
}

/// The two sides' contributions composed by `⊕` ([`Contribution::union`],
/// seam G): the union keeps every component of both sides, so a row both
/// sides produced under one provenance key with different bytes is a
/// `composition-conflict` (the same producer reporting two results for one
/// row: a reproducibility failure), named here with the row's derivation over
/// the union. Sides from different producers never conflict; their rows'
/// changes are the moves above.
#[must_use]
pub fn union(before: &Contribution, after: &Contribution) -> (Contribution, Value) {
    let composed = before.union(after);
    let conflicts: Vec<Value> = composed
        .conflicts()
        .map(|key| {
            let derived = composed.derive(&key.row);
            json!({
                "row": unit_key(&key.row),
                "analyzer_revision": key.provenance.analyzer_revision,
                "derived_verdict": derived.as_ref().map(|derived| derived.verdict.as_str()),
                "components": derived.as_ref().map_or(0, |derived| derived.components.len()),
            })
        })
        .collect();
    let record = json!({
        "components": {"before": before.len(), "after": after.len(), "union": composed.len()},
        "rows": composed.rows().len(),
        "conflicts": conflicts,
    });
    (composed, record)
}

/// The named premises a TIP row rests on, as the rule-32 report's
/// `premises.py` resolved them: the row's
/// `contract_dependencies.hypothesis_dependencies` (declaration to hypothesis
/// atoms) through the route's contract sidecar
/// (`<report>.contracts.json`: `contract_dependencies.declarations.<name>.
/// hypothesis_graph.premises.*`, each premise's `children` and
/// `erasure.name`). A row without hypothesis dependencies rests on no named
/// premise (`{"declarations": {}}`); without a sidecar the declarations are
/// named and the premises stay `unresolved`, never guessed.
#[must_use]
pub fn premises(row: &Value, contracts: Option<&Value>) -> Value {
    let Some(dependencies) = row["contract_dependencies"]["hypothesis_dependencies"].as_object()
    else {
        return json!({"declarations": {}});
    };
    let Some(contracts) = contracts else {
        return json!({"declarations": dependencies.keys().collect::<Vec<_>>(),
                      "unresolved": "no contract sidecar beside the report"});
    };
    let declarations = &contracts["contract_dependencies"]["declarations"];
    let resolved: serde_json::Map<String, Value> = dependencies
        .iter()
        .map(|(name, atoms)| {
            let atoms: BTreeSet<&str> = atoms
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let names: BTreeSet<&str> = declarations[name]["hypothesis_graph"]["premises"]
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(_, premise)| {
                    premise["children"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .any(|child| atoms.contains(child))
                })
                .filter_map(|(_, premise)| premise["erasure"]["name"].as_str())
                .collect();
            (name.clone(), json!(names))
        })
        .collect();
    json!({"declarations": resolved})
}

/// Annotate a rule-switched comparison for its contribution ledgers
/// ([`crate::ledger`]): every moved, unmatched-after and content-changed row
/// gets the named premises its TIP row rests on, and a row Unknown on the TIP
/// side gets its remaining reason leaves (`after_leaves`), the "why" of a row
/// that stays Unknown.
pub fn annotate(record: &mut Value, after: &Rows, contracts: Option<&Value>) {
    for field in ["moves", "content_changes", "unmatched"] {
        for entry in record[field].as_array_mut().into_iter().flatten() {
            let Some(row) = entry["row"]
                .as_str()
                .and_then(|key| after.functions.get(key))
            else {
                continue;
            };
            entry["premises"] = premises(&row.row, contracts);
            if row.status() == "Unknown" {
                let mut remaining = Vec::new();
                for (leaf, count) in leaves(&row.row) {
                    for _ in 0..count {
                        remaining.push(serde_json::from_str(&leaf).unwrap_or(Value::Null));
                    }
                }
                entry["after_leaves"] = Value::Array(remaining);
            }
        }
    }
}

/// Rows moved in more than one comparison: interactions, named row by row
/// with every comparison that moved them, never summed into any total (plan
/// §7).
#[must_use]
pub fn interactions(moved: &BTreeMap<String, BTreeMap<String, (String, String)>>) -> Value {
    let named: Vec<Value> = moved
        .iter()
        .filter(|(_, comparisons)| comparisons.len() > 1)
        .map(|(row, comparisons)| {
            json!({"row": row, "moved_in": comparisons.iter()
                .map(|(comparison, (before, after))| json!({"comparison": comparison,
                                                             "before": before, "after": after}))
                .collect::<Vec<_>>()})
        })
        .collect();
    Value::Array(named)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_blanks_environment_digests_only() {
        let row = json!({"status": "Safe", "id": "abc", "x": {"assumptions": "d1",
            "evidence_version": "contract-hypothesis-graph.v1:abc123"},
            "reading_site_evidence": [{"b": 2}, {"a": 1}]});
        let other = json!({"status": "Safe", "id": "xyz", "x": {"assumptions": "d2",
            "evidence_version": "contract-hypothesis-graph.v1:def456"},
            "reading_site_evidence": [{"a": 1}, {"b": 2}]});
        assert_eq!(normalize(&row), normalize(&other));
        let moved = json!({"status": "Unknown", "id": "xyz"});
        assert_ne!(normalize(&row), normalize(&moved));
    }

    #[test]
    fn records_are_not_artifacts() {
        assert!(is_record("invocation.json"));
        assert!(is_record("corpus/run.json"));
        assert!(is_record("corpus/audit.log"));
        assert!(is_record("corpus.exit"));
        assert!(!is_record("corpus/resource-diagnostics.json"));
        assert!(!is_record("corpus-null.jsonl"));
    }

    fn contracts(key: &str, other: &str) -> String {
        format!(
            "{{\"callback_members\": {{\"lifetime_protocols\": {{\"evidence\": {{\"summaries\": \
             {{\"{key}\": {{}}, \"{other}\": {{\"x\": 1}}}}}}}}}}, \"prepass\": [\"{key}\"]}}\n"
        )
    }

    #[test]
    fn exactly_one_swapped_time_key_is_accepted() {
        let before = contracts("aaa1", "kept");
        let after = contracts("bbb2", "kept");
        let swap = time_key(before.as_bytes(), after.as_bytes()).unwrap();
        assert_eq!(swap["before_key"], "aaa1");
        assert_eq!(swap["occurrences"], 2);
        // Two keys differing is not the named substitution.
        let two = contracts("bbb2", "changed");
        assert!(time_key(before.as_bytes(), two.as_bytes()).is_err());
        // A value change beside the swapped key is refused.
        let changed = after.replace("\"x\": 1", "\"x\": 2");
        let refused = time_key(before.as_bytes(), changed.as_bytes()).unwrap_err();
        assert_eq!(refused["json_equal_after_swap"], false);
    }

    #[test]
    fn premises_resolve_through_the_contract_sidecar() {
        let row = json!({"contract_dependencies": {"hypothesis_dependencies":
            {"rd_kafka_log0": ["a1", "a2"]}}});
        let contracts = json!({"contract_dependencies": {"declarations": {"rd_kafka_log0":
            {"hypothesis_graph": {"premises": {
                "0": {"children": ["a1"], "erasure": {"name": "rk_plugin_callback_closure"}},
                "1": {"children": ["zz"], "erasure": {"name": "unrelated"}}}}}}}});
        assert_eq!(
            premises(&row, Some(&contracts)),
            json!({"declarations": {"rd_kafka_log0": ["rk_plugin_callback_closure"]}})
        );
        assert!(premises(&row, None).get("unresolved").is_some());
        assert_eq!(
            premises(&json!({"contract_dependencies": []}), None),
            json!({"declarations": {}})
        );
    }

    #[test]
    fn interactions_are_rows_moved_in_two_comparisons() {
        let mut moved = BTreeMap::new();
        moved.insert(
            "r1".to_owned(),
            BTreeMap::from([
                (
                    "base->tip".to_owned(),
                    ("Unknown".to_owned(), "Safe".to_owned()),
                ),
                (
                    "off->tip".to_owned(),
                    ("Unknown".to_owned(), "Safe".to_owned()),
                ),
            ]),
        );
        moved.insert(
            "r2".to_owned(),
            BTreeMap::from([(
                "base->tip".to_owned(),
                ("Unknown".to_owned(), "Safe".to_owned()),
            )]),
        );
        let named = interactions(&moved);
        assert_eq!(named.as_array().unwrap().len(), 1);
        assert_eq!(named[0]["row"], "r1");
    }
}
