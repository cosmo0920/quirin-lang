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

//! `quirin-audit measure chain` (card **QUIRIN-AUDIT-CONNECT**, Part A; user
//! decision of 2026-11-26): two or more measurement steps joined into one
//! **chained record** ([`CHAIN_SCHEMA`]), reconciled row by row.
//!
//! ```text
//! quirin-audit --output OUT measure chain \
//!     (--step RECORD[#BEFORE->AFTER] | --runs BEFORE_DIR AFTER_DIR)... \
//!     [--declared N=PREDICTION.md]...
//! ```
//!
//! A **step** is one comparison: a saved native-comparison record
//! (`native-comparison.json`, or `.json.gz` read through `gzip -dc`) and the
//! comparison it names (the only one when it has one), or two measured side
//! directories (`sides/NAME`, each with its `run.json`), compared here with
//! the measurement's own row comparison ([`crate::compare::rows`]). Nothing
//! is re-measured.
//!
//! * **Links** ([`link`]). Consecutive steps must share a side: the later
//!   step's before side is the earlier step's after side, with the same
//!   measurement tool, revision, analyzer binary, build commit, contract
//!   bundles, per-route inputs (compile-database digests, route digest,
//!   spelling, source date), link-resolution and rule switch, and the same
//!   contract-dependency graph (Part C) when either records one. A field a
//!   record does not carry is **unrecorded** and refuses the link exactly as
//!   a differing one does: a link is never bridged. A refused link splits the
//!   chain into **segments**; each is reconciled alone, and the chain is
//!   `refused`.
//! * **Completeness.** A route is complete in a segment only when every step
//!   compared it with status `complete` and no unmatched row, unmatched unit
//!   row or composition conflict. Otherwise the route's chain is
//!   `incomplete`, with every step's named cause.
//! * **Rows.** Every row any step lists (moved, content-changed, unmatched,
//!   unit moves and unit field differences) is reconciled across the
//!   segment: its verdict at every point, the steps that moved it, the net
//!   move, and its pattern (`moved`, `cancelled`, `cascade`,
//!   `evidence-only`). A move that cancels or cascades stays listed; nothing
//!   is summed. A row two steps list with different verdicts at their shared
//!   side refuses the route (`cx_incomplete_row_disagrees`).
//! * **Evidence.** Per step, the contract-bundle and dependency-graph
//!   changes, and per row its leaves, other fields and contract dependencies
//!   (declarations, summaries, null premises, withdrawn) before and after, so
//!   a row whose evidence moved without its verdict is listed.
//! * **Attribution.** A move is attributed to a `--declared N=FILE` block
//!   (the `measure-prediction` block of a card's prediction, for step N) or
//!   to the step record's own declared check; every other move is listed as
//!   unattributed, and every unattributed new `Safe` is flagged for review
//!   and fails the run. A dependency change (Part C) is listed beside a move
//!   it may explain, never as its attribution.
//!
//! Every refusal and condition is a named formula of `qcore_schemas::connect`
//! (the CX block of the C04.18.9 typed rendering, checked by the
//! premise-formula validator); the record names each by its id. The tool
//! proposes no verdict: it only joins and reports. The Lean model is
//! `QCore.MeasureConnect`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qcore_schemas::connect::formula;
use quirin_audit_plugin::{Request, Response, Severity};
use serde_json::{Value, json};

use crate::util::{read_json, sha_file, write_json};
use crate::{COMPARISON_SCHEMA, RUN_SCHEMA, declared, tool};

/// The chained record's schema.
pub const CHAIN_SCHEMA: &str = "quirin-measure-chain/1";

/// The chained record's file name in the output directory.
pub const CHAIN_FILE: &str = "chain.json";

/// Usage text.
pub const USAGE: &str = "usage: quirin-audit --output OUT measure chain \
(--step RECORD[#BEFORE->AFTER] | --runs BEFORE_DIR AFTER_DIR)... [--declared N=FILE]...\n\
two or more steps, in chain order";

/// One step as given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepSpec {
    /// A native-comparison record and, optionally, the comparison label.
    Record {
        path: String,
        comparison: Option<String>,
    },
    /// Two measured side directories.
    Runs { before: String, after: String },
}

/// The parsed flags of `measure chain`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChainOptions {
    pub steps: Vec<StepSpec>,
    /// `(step number, 1-based; prediction file)`.
    pub declared: Vec<(usize, String)>,
}

/// Parse the arguments after `measure chain`.
///
/// # Errors
///
/// An unknown or valueless flag, fewer than two steps, or a `--declared`
/// naming no step.
pub fn parse(arguments: &[String]) -> Result<ChainOptions, String> {
    let mut options = ChainOptions::default();
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        let value = |offset: usize| {
            arguments
                .get(index + offset)
                .cloned()
                .ok_or_else(|| format!("{flag} requires a value\n{USAGE}"))
        };
        match flag {
            "--step" => {
                let text = value(1)?;
                let (path, comparison) = match text.split_once('#') {
                    Some((path, label)) => {
                        if !label.contains("->") {
                            return Err(format!(
                                "--step `{text}`: the comparison is spelled BEFORE->AFTER"
                            ));
                        }
                        (path.to_owned(), Some(label.to_owned()))
                    }
                    None => (text, None),
                };
                options.steps.push(StepSpec::Record { path, comparison });
                index += 2;
            }
            "--runs" => {
                let (before, after) = (value(1)?, value(2)?);
                options.steps.push(StepSpec::Runs { before, after });
                index += 3;
            }
            "--declared" => {
                let text = value(1)?;
                let (number, file) = text
                    .split_once('=')
                    .ok_or_else(|| format!("--declared `{text}`: N=FILE"))?;
                let number: usize = number
                    .parse()
                    .ok()
                    .filter(|number| *number > 0)
                    .ok_or_else(|| format!("--declared `{text}`: N is a step number from 1"))?;
                if options.declared.iter().any(|(known, _)| *known == number) {
                    return Err(format!("--declared names step {number} twice"));
                }
                options.declared.push((number, file.to_owned()));
                index += 2;
            }
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        }
    }
    if options.steps.len() < 2 {
        return Err(format!("a chain joins two or more steps\n{USAGE}"));
    }
    for (number, _) in &options.declared {
        if *number > options.steps.len() {
            return Err(format!(
                "--declared names step {number}; the chain has {}",
                options.steps.len()
            ));
        }
    }
    Ok(options)
}

/// Read a JSON record, decompressing a `.gz` one with `gzip -dc`.
///
/// # Errors
///
/// An unreadable, undecompressible or malformed file.
pub fn read_record(path: &Path) -> Result<Value, String> {
    if path.extension().is_some_and(|extension| extension == "gz") {
        let output = std::process::Command::new("gzip")
            .arg("-dc")
            .arg(path)
            .output()
            .map_err(|error| format!("gzip -dc {}: {error}", path.display()))?;
        if !output.status.success() {
            return Err(format!(
                "gzip -dc {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let mut deserializer = serde_json::Deserializer::from_slice(&output.stdout);
        deserializer.disable_recursion_limit();
        let value = serde::Deserialize::deserialize(&mut deserializer)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        deserializer
            .end()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(value)
    } else {
        read_json(path)
    }
}

/// What a link compares of a side.
#[derive(Clone, Debug, PartialEq)]
pub struct Identity {
    /// The side's name in its record (or its directory).
    pub name: String,
    pub revision: Value,
    pub build_commit: Value,
    /// The analyzer binary's SHA-256.
    pub analyzer: Value,
    /// The contract bundles' identities (crate directory to digest).
    pub bundles: Value,
    /// Per route, the inputs record.
    pub inputs: BTreeMap<String, Value>,
    pub link_resolution: Value,
    pub rules_off: Value,
    /// Card QUIRIN-AUDIT-CONNECT Part C: the contract-dependency graph with
    /// its identities, when the side records one.
    pub dependency_graph: Value,
}

impl Identity {
    fn to_json(&self) -> Value {
        json!({
            "side": self.name,
            "revision": self.revision,
            "build_commit": self.build_commit,
            "analyzer_sha256": self.analyzer,
            "bundles": self.bundles,
            "inputs": self.inputs,
            "link_resolution": self.link_resolution,
            "rules_off": self.rules_off,
            "dependency_graph": self.dependency_graph,
        })
    }
}

/// The declared contract-dependency graph a report records
/// (`assumptions.contract_revision.contract_dependencies`, card
/// QUIRIN-AUDIT-CONNECT Part C), `null` when it records none.
#[must_use]
pub fn dependency_graph_of(document: &Value) -> Value {
    document["assumptions"]["contract_revision"]["contract_dependencies"].clone()
}

/// Record both sides' dependency graphs in a route comparison, only when a
/// side has one (so every other record is unchanged).
pub fn record_dependency_graphs(
    record: &mut Value,
    before: &crate::rows::Rows,
    after: &crate::rows::Rows,
) {
    let (left, right) = (
        dependency_graph_of(&before.document),
        dependency_graph_of(&after.document),
    );
    if !left.is_null() || !right.is_null() {
        record["dependency_graph"] = json!({"before": left, "after": right});
    }
}

/// A side's dependency graph from its route entries when its record has
/// none: per route, the entry's graph for that side.
fn graphs_of_routes(routes: &BTreeMap<String, Value>, side: &str) -> Value {
    let graphs: BTreeMap<&String, &Value> = routes
        .iter()
        .filter(|(_, entry)| !entry["dependency_graph"][side].is_null())
        .map(|(route, entry)| (route, &entry["dependency_graph"][side]))
        .collect();
    if graphs.is_empty() {
        Value::Null
    } else {
        json!(graphs)
    }
}

impl Step {
    /// Fill each side's dependency graph from the route entries when the
    /// side record carries none.
    fn with_route_graphs(mut self) -> Self {
        if self.before.dependency_graph.is_null() {
            self.before.dependency_graph = graphs_of_routes(&self.routes, "before");
        }
        if self.after.dependency_graph.is_null() {
            self.after.dependency_graph = graphs_of_routes(&self.routes, "after");
        }
        self
    }
}

/// One loaded step.
#[derive(Clone, Debug)]
pub struct Step {
    /// 1-based position in the chain.
    pub number: usize,
    pub source: Value,
    /// The measurement tool's identity (who compared the two sides).
    pub tool: Value,
    pub comparison: String,
    pub before: Identity,
    pub after: Identity,
    /// Route name to the comparison's route entry.
    pub routes: BTreeMap<String, Value>,
    /// The record's declared check of this comparison, when it has one.
    pub declared: Value,
}

fn rules_off_of(spec: &Value) -> Value {
    spec.as_str()
        .and_then(|text| text.split_once("--rules-off "))
        .map_or(Value::Null, |(_, ids)| json!(ids.trim()))
}

fn identity_of_record_side(name: &str, side: &Value) -> Identity {
    Identity {
        name: name.to_owned(),
        revision: side["revision"].clone(),
        build_commit: side["build_commit"].clone(),
        analyzer: side["binary"]["sha256"].clone(),
        bundles: side["bundles"].clone(),
        inputs: side["inputs"]
            .as_object()
            .map(|routes| {
                routes
                    .iter()
                    .map(|(route, inputs)| (route.clone(), inputs.clone()))
                    .collect()
            })
            .unwrap_or_default(),
        link_resolution: side["link_resolution"].clone(),
        rules_off: side
            .get("rules_off")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or_else(|| rules_off_of(&side["spec"])),
        dependency_graph: side
            .get("dependency_graph")
            .cloned()
            .unwrap_or(Value::Null),
    }
}

fn identity_of_run(name: &str, run: &Value) -> Identity {
    Identity {
        name: name.to_owned(),
        revision: run["revision"].clone(),
        build_commit: run["build_commit"].clone(),
        analyzer: run["binary"]["sha256"].clone(),
        bundles: run["bundles"].clone(),
        inputs: run["routes"]
            .as_object()
            .map(|routes| {
                routes
                    .iter()
                    .map(|(route, record)| (route.clone(), record["inputs"].clone()))
                    .collect()
            })
            .unwrap_or_default(),
        link_resolution: run["link_resolution"].clone(),
        rules_off: match &run["rules_off"] {
            Value::Array(ids) if !ids.is_empty() => json!(
                ids.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            _ => Value::Null,
        },
        dependency_graph: run
            .get("dependency_graph")
            .cloned()
            .unwrap_or(Value::Null),
    }
}

/// Load a step from a saved record.
///
/// # Errors
///
/// An unreadable record, another schema, an unknown or ambiguous comparison.
pub fn load_record_step(
    number: usize,
    path: &Path,
    comparison: Option<&str>,
) -> Result<Step, String> {
    let record = read_record(path)?;
    if record["schema"] != json!(COMPARISON_SCHEMA) {
        return Err(format!(
            "{}: schema {}, not {COMPARISON_SCHEMA}",
            path.display(),
            record["schema"]
        ));
    }
    let comparisons = record["comparisons"]
        .as_array()
        .ok_or_else(|| format!("{}: no comparisons", path.display()))?;
    let chosen = match comparison {
        Some(label) => comparisons
            .iter()
            .find(|entry| entry["comparison"] == json!(label))
            .ok_or_else(|| format!("{}: no comparison {label}", path.display()))?,
        None => match comparisons.as_slice() {
            [only] => only,
            _ => {
                return Err(format!(
                    "{}: {} comparisons; name one as {}#BEFORE->AFTER",
                    path.display(),
                    comparisons.len(),
                    path.display()
                ));
            }
        },
    };
    let label = chosen["comparison"].as_str().unwrap_or_default().to_owned();
    let (before, after) = (
        chosen["before"].as_str().unwrap_or_default(),
        chosen["after"].as_str().unwrap_or_default(),
    );
    let side = |name: &str| -> Result<Identity, String> {
        let side = record["sides"]
            .get(name)
            .ok_or_else(|| format!("{}: no side {name}", path.display()))?;
        Ok(identity_of_record_side(name, side))
    };
    let declared = if record["declared"]["comparison"] == json!(label) {
        record["declared"].clone()
    } else {
        Value::Null
    };
    Ok(Step {
        number,
        source: json!({"kind": "record", "path": path.display().to_string(),
                       "sha256": sha_file(path)?, "comparison": label,
                       "record_status": record["status"]}),
        tool: record["tool"].clone(),
        comparison: label,
        before: side(before)?,
        after: side(after)?,
        routes: chosen["routes"]
            .as_object()
            .map(|routes| {
                routes
                    .iter()
                    .map(|(route, entry)| (route.clone(), entry.clone()))
                    .collect()
            })
            .unwrap_or_default(),
        declared,
    }
    .with_route_graphs())
}

fn route_definition(run: &Value, route: &str) -> Result<crate::inputs::Route, String> {
    serde_json::from_value(run["routes"][route]["route_definition"].clone())
        .map_err(|error| format!("route {route}: no readable route definition: {error}"))
}

/// Load a step from two measured side directories, comparing them here.
///
/// # Errors
///
/// A directory without a `run.json` of [`RUN_SCHEMA`].
pub fn load_runs_step(number: usize, before: &Path, after: &Path) -> Result<Step, String> {
    let read_run = |directory: &Path| -> Result<Value, String> {
        let run = read_json(&directory.join("run.json"))?;
        if run["schema"] != json!(RUN_SCHEMA) {
            return Err(format!(
                "{}/run.json: schema {}, not {RUN_SCHEMA}",
                directory.display(),
                run["schema"]
            ));
        }
        Ok(run)
    };
    let (left, right) = (read_run(before)?, read_run(after)?);
    let name = |directory: &Path, run: &Value| {
        run["side"].as_str().map_or_else(
            || directory.display().to_string(),
            std::borrow::ToOwned::to_owned,
        )
    };
    let routes: BTreeSet<String> = [&left, &right]
        .iter()
        .flat_map(|run| {
            run["routes"]
                .as_object()
                .map(|routes| routes.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .collect();
    let mut entries = BTreeMap::new();
    for route in routes {
        let entry = (|| -> Result<Value, String> {
            for (directory, run) in [(before, &left), (after, &right)] {
                if run["routes"][&route]["exit"] != json!(0) {
                    return Err(format!(
                        "side {} did not complete route {route} (exit {})",
                        directory.display(),
                        run["routes"][&route]["exit"]
                    ));
                }
            }
            let definition = route_definition(&left, &route)?;
            if route_definition(&right, &route)?.digest() != definition.digest() {
                return Err(format!("the two sides define route {route} differently"));
            }
            let report = definition.report();
            let left_rows = crate::rows::read(&before.join(&route).join(report), false)?;
            let right_rows = crate::rows::read(&after.join(&route).join(report), false)?;
            let (mut record, _) = crate::compare::rows(&left_rows, &right_rows);
            record["status_deltas"] = json!(crate::compare::deltas(&left_rows, &right_rows));
            record_dependency_graphs(&mut record, &left_rows, &right_rows);
            record["composition"] = match (&left_rows.contribution, &right_rows.contribution) {
                (Some(left), Some(right)) => {
                    let (_, union) = crate::compare::union(left, right);
                    if union["conflicts"].as_array().is_some_and(|c| !c.is_empty()) {
                        record["status"] = json!("incomplete");
                        if let Some(reasons) = record["incomplete_because"].as_array_mut() {
                            reasons.push(json!(
                                "composition-conflict in the union of the two sides (named)"
                            ));
                        }
                    }
                    union
                }
                _ => json!({"union": "not formed: a side has no component sidecar"}),
            };
            Ok(record)
        })()
        .unwrap_or_else(|why| json!({"status": "not-compared", "why": why}));
        entries.insert(route, entry);
    }
    let (before_name, after_name) = (name(before, &left), name(after, &right));
    Ok(Step {
        number,
        source: json!({"kind": "runs",
            "before": {"path": before.display().to_string(),
                       "run_json_sha256": sha_file(&before.join("run.json"))?},
            "after": {"path": after.display().to_string(),
                      "run_json_sha256": sha_file(&after.join("run.json"))?},
            "comparison": format!("{before_name}->{after_name}")}),
        tool: tool(),
        comparison: format!("{before_name}->{after_name}"),
        before: identity_of_run(&before_name, &left),
        after: identity_of_run(&after_name, &right),
        routes: entries,
        declared: Value::Null,
    }
    .with_route_graphs())
}

/// One refused condition of a link.
fn refusal(id: &str, field: &str, earlier: &Value, later: &Value) -> Value {
    json!({"formula": formula(id).map(|(id, _)| id), "field": field,
           "earlier_after": earlier, "later_before": later})
}

/// Check the link between `earlier` (its after side) and `later` (its before
/// side): every refused condition, named by its formula. Empty means joined
/// (`cx_joined_is_every_identity`, `cx_every_identity_is_joined`).
#[must_use]
pub fn link(earlier: &Step, later: &Step) -> Vec<Value> {
    let (a, b) = (&earlier.after, &later.before);
    let mut refused = Vec::new();
    let required = |id: &str, field: &str, x: &Value, y: &Value, out: &mut Vec<Value>| {
        if x.is_null() || y.is_null() {
            out.push(refusal("cx_refuse_unrecorded", field, x, y));
        } else if x != y {
            out.push(refusal(id, field, x, y));
        }
    };
    let tool_sha = |step: &Step| step.tool["sha256"].clone();
    required(
        "cx_refuse_tool",
        "tool.sha256",
        &tool_sha(earlier),
        &tool_sha(later),
        &mut refused,
    );
    required(
        "cx_refuse_revision",
        "revision",
        &a.revision,
        &b.revision,
        &mut refused,
    );
    required(
        "cx_refuse_analyzer",
        "analyzer_sha256",
        &a.analyzer,
        &b.analyzer,
        &mut refused,
    );
    required(
        "cx_refuse_bundles",
        "bundles",
        &a.bundles,
        &b.bundles,
        &mut refused,
    );
    // Optional fields: equal (both may be absent) or the link is refused.
    for (id, field, x, y) in [
        (
            "cx_refuse_build_commit",
            "build_commit",
            &a.build_commit,
            &b.build_commit,
        ),
        (
            "cx_refuse_link_resolution",
            "link_resolution",
            &a.link_resolution,
            &b.link_resolution,
        ),
        ("cx_refuse_rules_off", "rules_off", &a.rules_off, &b.rules_off),
        (
            "cx_refuse_dependency_graph",
            "dependency_graph",
            &a.dependency_graph,
            &b.dependency_graph,
        ),
    ] {
        let normal = |value: &Value| match value {
            Value::Bool(false) => Value::Null,
            other => other.clone(),
        };
        if normal(x) != normal(y) {
            refused.push(refusal(id, field, x, y));
        }
    }
    // Inputs, per route both steps compare.
    let shared: BTreeSet<&String> = earlier
        .routes
        .keys()
        .filter(|route| later.routes.contains_key(*route))
        .collect();
    if shared.is_empty() {
        refused.push(refusal(
            "cx_refuse_no_shared_route",
            "routes",
            &json!(earlier.routes.keys().collect::<Vec<_>>()),
            &json!(later.routes.keys().collect::<Vec<_>>()),
        ));
    }
    for route in shared {
        let (x, y) = (
            a.inputs.get(route).cloned().unwrap_or(Value::Null),
            b.inputs.get(route).cloned().unwrap_or(Value::Null),
        );
        required(
            "cx_refuse_inputs",
            &format!("inputs.{route}"),
            &x,
            &y,
            &mut refused,
        );
    }
    refused
}

/// Why a step's route entry is not complete, each cause named by its formula.
fn incomplete_causes(entry: Option<&Value>) -> Vec<Value> {
    let Some(entry) = entry else {
        return vec![json!({"formula": "cx_incomplete_not_measured",
                           "why": "the step does not compare the route"})];
    };
    let mut causes = Vec::new();
    if entry["status"] != json!("complete") {
        causes.push(json!({"formula": "cx_incomplete_status", "status": entry["status"],
                           "why": entry.get("why").or_else(|| entry.get("incomplete_because"))}));
    }
    let nonempty = |value: &Value| value.as_array().is_some_and(|items| !items.is_empty());
    if nonempty(&entry["unmatched"]) {
        causes.push(json!({"formula": "cx_incomplete_unmatched",
                           "rows": entry["unmatched"].as_array().map_or(0, Vec::len)}));
    }
    if nonempty(&entry["unmatched_units"]) {
        causes.push(json!({"formula": "cx_incomplete_unmatched_units",
                           "rows": entry["unmatched_units"].as_array().map_or(0, Vec::len)}));
    }
    if nonempty(&entry["conflicts"]["before"]) || nonempty(&entry["conflicts"]["after"]) {
        causes.push(json!({"formula": "cx_incomplete_conflicts",
                           "conflicts": entry["conflicts"]}));
    }
    causes
}

/// What one step says of one row.
#[derive(Clone, Debug, Default)]
struct Event {
    before: Option<String>,
    after: Option<String>,
    evidence: Option<Value>,
}

/// The kind of a row key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Function,
    Unit,
}

fn dependency_diff(entry: &Value) -> Value {
    let Some(dependencies) = entry.get("contract_dependencies") else {
        return Value::Null;
    };
    let (before, after) = (&dependencies["before"], &dependencies["after"]);
    let mut out = serde_json::Map::new();
    let keys: BTreeSet<&String> = before
        .as_object()
        .into_iter()
        .flatten()
        .chain(after.as_object().into_iter().flatten())
        .map(|(key, _)| key)
        .collect();
    for key in keys {
        let set = |side: &Value| -> BTreeSet<String> {
            match &side[key] {
                Value::Array(items) => items.iter().map(Value::to_string).collect(),
                Value::Object(map) => map.iter().map(|(k, v)| format!("{k}={v}")).collect(),
                Value::Null => BTreeSet::new(),
                other => BTreeSet::from([other.to_string()]),
            }
        };
        let (left, right) = (set(before), set(after));
        if left != right {
            out.insert(
                key.clone(),
                json!({"added": right.difference(&left).collect::<Vec<_>>(),
                       "removed": left.difference(&right).collect::<Vec<_>>()}),
            );
        }
    }
    Value::Object(out)
}

fn evidence_of(entry: &Value) -> Value {
    json!({
        "leaves_removed": entry["leaves_removed"],
        "leaves_added": entry["leaves_added"],
        "other_fields_changed": entry["other_fields_changed"],
        "contract_dependencies": dependency_diff(entry),
        "contract_dependencies_after": entry["contract_dependencies"]["after"],
    })
}

fn pair(value: &Value) -> Option<(String, String)> {
    let items = value.as_array()?;
    match items.as_slice() {
        [before, after] => Some((before.as_str()?.to_owned(), after.as_str()?.to_owned())),
        _ => None,
    }
}

/// Every row a step's route entry lists, with what it says.
fn events(entry: &Value) -> BTreeMap<(Kind, String), Event> {
    let mut out = BTreeMap::new();
    for list in ["moves", "content_changes"] {
        for row in entry[list].as_array().into_iter().flatten() {
            let Some(key) = row["row"].as_str() else {
                continue;
            };
            let (before, after) = pair(&row["status"]).unzip();
            out.insert(
                (Kind::Function, key.to_owned()),
                Event {
                    before,
                    after,
                    evidence: Some(evidence_of(row)),
                },
            );
        }
    }
    for row in entry["unmatched"].as_array().into_iter().flatten() {
        let Some(key) = row["row"].as_str() else {
            continue;
        };
        let status = row["status"].as_str().map(str::to_owned);
        let absent = Some("absent".to_owned());
        let (before, after) = if row["only"] == json!("before") {
            (status, absent)
        } else {
            (absent, status)
        };
        out.insert(
            (Kind::Function, key.to_owned()),
            Event {
                before,
                after,
                evidence: None,
            },
        );
    }
    for row in entry["unit_moves"].as_array().into_iter().flatten() {
        let Some(key) = row["row"].as_str() else {
            continue;
        };
        let (before, after) = pair(&row["verdict"]).unzip();
        out.insert(
            (Kind::Unit, key.to_owned()),
            Event {
                before,
                after,
                evidence: None,
            },
        );
    }
    for (key, fields) in entry["unit_field_differences"]
        .as_object()
        .into_iter()
        .flatten()
    {
        let event = out.entry((Kind::Unit, key.clone())).or_default();
        event.evidence = Some(json!({"unit_fields_changed": fields}));
    }
    for row in entry["unmatched_units"].as_array().into_iter().flatten() {
        let Some(key) = row["row"].as_str() else {
            continue;
        };
        let event = out.entry((Kind::Unit, key.to_owned())).or_default();
        if row["only"] == json!("before") {
            event.after = Some("absent".into());
        } else {
            event.before = Some("absent".into());
        }
    }
    out
}

/// `(library, file, unit row, property, profile)` and, for a function row,
/// `function#occurrence`.
fn split_key(kind: Kind, key: &str) -> Option<(Vec<&str>, Option<&str>)> {
    let parts: Vec<&str> = key.split("::").collect();
    match (kind, parts.len()) {
        (Kind::Unit, 5) => Some((parts, None)),
        (Kind::Function, 6) => Some((parts[..5].to_vec(), Some(parts[5]))),
        _ => None,
    }
}

/// Unmatched rows of one step that name the same library, file and function
/// under another key: a rekeyed (renamed) row, reported as such.
fn rekeyed(entry: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for (list, kind) in [("unmatched", Kind::Function), ("unmatched_units", Kind::Unit)] {
        let rows: Vec<(&str, &str)> = entry[list]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| Some((row["row"].as_str()?, row["only"].as_str()?)))
            .collect();
        let identity = |key: &str| -> Option<(String, String, String)> {
            let (unit, function) = split_key(kind, key)?;
            let function = function.map_or("", |f| f.split('#').next().unwrap_or(f));
            Some((unit[0].to_owned(), unit[1].to_owned(), function.to_owned()))
        };
        for (gone, _) in rows.iter().filter(|(_, only)| *only == "before") {
            for (new, _) in rows.iter().filter(|(_, only)| *only == "after") {
                if identity(gone).is_some() && identity(gone) == identity(new) {
                    out.push(json!({"kind": if kind == Kind::Unit { "unit" } else { "function" },
                                    "before_key": gone, "after_key": new,
                                    "formula": "cx_incomplete_unmatched"}));
                }
            }
        }
    }
    out
}

/// A row's reconciliation across a segment.
fn reconcile(
    steps: &[&Step],
    route: &str,
    kind: Kind,
    key: &str,
    per_step: &[Option<&Event>],
    compared: &[bool],
) -> (Value, bool) {
    let points = steps.len() + 1;
    // A step *lists* the row's verdicts when it states both sides
    // (`cx_listed`); the verdict at a side is the after verdict of the
    // nearest listing on its left, else the before verdict of the nearest
    // listing on its right (`QCore.MeasureConnect.recon`): an unlisted step
    // keeps the verdict across it (`cx_unlisted_is_unchanged`).
    let listing: Vec<Option<(String, String)>> = per_step
        .iter()
        .map(|event| {
            event.and_then(|event| Some((event.before.clone()?, event.after.clone()?)))
        })
        .collect();
    let verdicts: Vec<Option<String>> = (0..points)
        .map(|point| {
            listing[..point]
                .iter()
                .rev()
                .find_map(|listed| listed.as_ref().map(|(_, after)| after.clone()))
                .or_else(|| {
                    listing[point..]
                        .iter()
                        .find_map(|listed| listed.as_ref().map(|(before, _)| before.clone()))
                })
        })
        .collect();
    // Two consecutive listings that disagree at the side between them
    // (`cx_disagrees`) make the route incomplete.
    let mut disagreements = Vec::new();
    let mut previous: Option<(usize, &String)> = None;
    for (position, listed) in listing.iter().enumerate() {
        if let Some((before, after)) = listed {
            if let Some((earlier, earlier_after)) = previous
                && earlier_after != before
            {
                disagreements.push(json!({
                    "steps": [steps[earlier].number, steps[position].number],
                    "verdicts": [earlier_after, before],
                    "formula": "cx_incomplete_row_disagrees"}));
            }
            previous = Some((position, after));
        }
    }
    let _ = compared;
    let moved_in: Vec<usize> = (0..steps.len())
        .filter(|&position| {
            per_step[position].is_some_and(|event| {
                event.before.is_some() && event.after.is_some() && event.before != event.after
            })
        })
        .map(|position| steps[position].number)
        .collect();
    let evidence: Vec<Value> = per_step
        .iter()
        .enumerate()
        .filter_map(|(position, event)| {
            let evidence = event.as_ref()?.evidence.as_ref()?;
            Some(json!({"step": steps[position].number, "evidence": evidence}))
        })
        .collect();
    let (first, last) = (verdicts[0].clone(), verdicts[points - 1].clone());
    let pattern = match moved_in.len() {
        0 => "evidence-only",
        1 => "moved",
        _ if first == last => "cancelled",
        _ => "cascade",
    };
    let new_safe: Vec<usize> = (0..steps.len())
        .filter(|&position| {
            verdicts[position + 1].as_deref() == Some("Safe")
                && verdicts[position].as_deref() != Some("Safe")
                && moved_in.contains(&steps[position].number)
        })
        .map(|position| steps[position].number)
        .collect();
    let disagrees = !disagreements.is_empty();
    (
        json!({
            "row": key,
            "route": route,
            "kind": if kind == Kind::Unit { "unit" } else { "function" },
            "verdicts": verdicts,
            "moved_in": moved_in,
            "net": [first, last],
            "pattern": pattern,
            "new_safe_in": new_safe,
            "evidence": evidence,
            "disagreements": disagreements,
        }),
        disagrees,
    )
}

/// A declaration's attribution source for one step.
struct Attribution {
    label: String,
    sha256: String,
    declaration: Option<declared::Declaration>,
    /// The record's own check (`declared.routes`), when that is the source.
    record_check: Option<Value>,
}

impl Attribution {
    /// Whether this source attributes the function row's move in `route`.
    fn covers(&self, route: &str, key: &str, before: &str, after: &str) -> bool {
        if let Some(declaration) = &self.declaration {
            let Some(declared_route) = declaration.get(route) else {
                return false;
            };
            let Some((unit, Some(function))) = split_key(Kind::Function, key) else {
                return false;
            };
            let function = function.split('#').next().unwrap_or(function);
            return declared_route.moves.iter().any(|movement| {
                declared::selects(&movement.selector, unit[0], unit[1], function)
                    && (movement.before == "*" || movement.before == before)
                    && (movement.after == "*" || movement.after == after)
            });
        }
        if let Some(check) = &self.record_check {
            let route_check = &check["routes"][route];
            if route_check["declared"] != json!(true) {
                return false;
            }
            return !route_check["unpredicted"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|row| row["row"] == json!(key));
        }
        false
    }
}

fn diff_maps(before: &Value, after: &Value) -> Value {
    let empty = serde_json::Map::new();
    let (left, right) = (
        before.as_object().unwrap_or(&empty),
        after.as_object().unwrap_or(&empty),
    );
    let keys: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
    let changed: BTreeMap<&String, Value> = keys
        .into_iter()
        .filter(|key| left.get(*key) != right.get(*key))
        .map(|key| (key, json!({"before": left.get(key), "after": right.get(key)})))
        .collect();
    json!(changed)
}

/// The dependency entries of a recorded graph whose identity changed, and the
/// declarations they name (dependent and target).
fn dependency_changes(before: &Value, after: &Value) -> (Value, BTreeSet<String>) {
    // A side's graph is one record, or one per route.
    let entries = |graph: &Value| -> BTreeMap<String, Value> {
        let lists: Vec<&Value> = if graph.get("dependencies").is_some() {
            vec![&graph["dependencies"]]
        } else {
            graph
                .as_object()
                .into_iter()
                .flatten()
                .map(|(_, route)| &route["dependencies"])
                .collect()
        };
        lists
            .into_iter()
            .filter_map(Value::as_array)
            .flatten()
            .map(|entry| (entry["id"].as_str().unwrap_or_default().to_owned(), entry.clone()))
            .collect()
    };
    let (left, right) = (entries(before), entries(after));
    let mut names = BTreeSet::new();
    let mut changes = Vec::new();
    for id in left.keys().chain(right.keys()).collect::<BTreeSet<_>>() {
        let (x, y) = (left.get(id), right.get(id));
        if x != y {
            for entry in [x, y].into_iter().flatten() {
                for field in ["dependent", "target_function"] {
                    if let Some(name) = entry[field].as_str() {
                        names.insert(name.to_owned());
                    }
                }
            }
            changes.push(json!({"id": id, "before": x, "after": y}));
        }
    }
    (json!(changes), names)
}

/// Build the chained record of `steps` with the attribution sources given.
///
/// # Errors
///
/// An unreadable or malformed prediction file.
#[allow(clippy::too_many_lines)]
pub fn chain(steps: &[Step], predictions: &[(usize, PathBuf)]) -> Result<Value, String> {
    let mut attributions: BTreeMap<usize, Vec<Attribution>> = BTreeMap::new();
    for (number, path) in predictions {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        attributions.entry(*number).or_default().push(Attribution {
            label: path.display().to_string(),
            sha256: sha_file(path)?,
            declaration: Some(
                declared::parse(&text).map_err(|error| format!("{}: {error}", path.display()))?,
            ),
            record_check: None,
        });
    }
    for step in steps {
        if !step.declared.is_null() {
            attributions.entry(step.number).or_default().push(Attribution {
                label: step.declared["path"]
                    .as_str()
                    .unwrap_or("(the record's declared check)")
                    .to_owned(),
                sha256: step.declared["sha256"].as_str().unwrap_or_default().to_owned(),
                declaration: None,
                record_check: Some(step.declared.clone()),
            });
        }
    }
    // Links, and the segments they leave.
    let mut links = Vec::new();
    let mut segments: Vec<Vec<&Step>> = vec![Vec::new()];
    for (position, step) in steps.iter().enumerate() {
        if position > 0 {
            let refused = link(&steps[position - 1], step);
            let joined = refused.is_empty();
            links.push(json!({
                "between": [steps[position - 1].number, step.number],
                "joined": joined,
                "refused_because": refused,
            }));
            if !joined {
                segments.push(Vec::new());
            }
        }
        segments.last_mut().expect("one segment").push(step);
    }
    let links_joined = links.iter().all(|entry| entry["joined"] == json!(true));
    let mut segment_records = Vec::new();
    let mut unattributed_moves = Vec::new();
    let mut unattributed_new_safe = Vec::new();
    let mut every_route_complete = true;
    for segment in &segments {
        let routes: BTreeSet<&String> = segment.iter().flat_map(|step| step.routes.keys()).collect();
        let mut route_records = BTreeMap::new();
        let step_changes: Vec<(usize, BTreeSet<String>, Value)> = segment
            .iter()
            .map(|step| {
                let (changes, names) = dependency_changes(
                    &step.before.dependency_graph,
                    &step.after.dependency_graph,
                );
                (step.number, names, changes)
            })
            .collect();
        for route in routes {
            let entries: Vec<Option<&Value>> =
                segment.iter().map(|step| step.routes.get(route)).collect();
            let mut causes = Vec::new();
            for (step, entry) in segment.iter().zip(&entries) {
                for cause in incomplete_causes(*entry) {
                    causes.push(json!({"step": step.number, "cause": cause}));
                }
            }
            let compared: Vec<bool> = entries
                .iter()
                .map(|entry| entry.is_some_and(|entry| entry["status"] != json!("not-compared")))
                .collect();
            let per_step: Vec<BTreeMap<(Kind, String), Event>> = entries
                .iter()
                .map(|entry| entry.map(events).unwrap_or_default())
                .collect();
            let keys: BTreeSet<&(Kind, String)> =
                per_step.iter().flat_map(BTreeMap::keys).collect();
            let mut rows = Vec::new();
            let mut counts: BTreeMap<&'static str, u64> = BTreeMap::new();
            for (kind, key) in keys {
                let events: Vec<Option<&Event>> = per_step
                    .iter()
                    .map(|events| events.get(&(*kind, key.clone())))
                    .collect();
                let (mut row, disagrees) =
                    reconcile(segment, route, *kind, key, &events, &compared);
                if disagrees {
                    causes.push(json!({"row": key,
                        "cause": {"formula": "cx_incomplete_row_disagrees"}}));
                }
                // Attribution, step by step.
                let mut attributed = Vec::new();
                let mut unattributed = Vec::new();
                let declared_by = |position: usize| -> Vec<Value> {
                    let step = segment[position];
                    let Some(event) = events[position] else {
                        return Vec::new();
                    };
                    let (Some(before), Some(after)) = (&event.before, &event.after) else {
                        return Vec::new();
                    };
                    attributions
                        .get(&step.number)
                        .into_iter()
                        .flatten()
                        .filter(|source| {
                            *kind == Kind::Function && source.covers(route, key, before, after)
                        })
                        .map(|source| json!({"step": step.number, "declared": source.label,
                                             "sha256": source.sha256}))
                        .collect()
                };
                for (position, step) in segment.iter().enumerate() {
                    if !row["moved_in"]
                        .as_array()
                        .is_some_and(|moved| moved.contains(&json!(step.number)))
                    {
                        continue;
                    }
                    let mut sources = declared_by(position);
                    if *kind == Kind::Unit {
                        // A unit row's move is derived from its function rows:
                        // attributed when every function row of the unit that
                        // moved in the step is attributed.
                        let prefix = format!("{key}::");
                        let function_moves: Vec<&String> = per_step[position]
                            .iter()
                            .filter(|((k, function_key), event)| {
                                *k == Kind::Function
                                    && function_key.starts_with(&prefix)
                                    && event.before.is_some()
                                    && event.before != event.after
                            })
                            .map(|((_, function_key), _)| function_key)
                            .collect();
                        let all = !function_moves.is_empty()
                            && function_moves.iter().all(|function_key| {
                                let event = &per_step[position][&(Kind::Function, (*function_key).clone())];
                                attributions.get(&step.number).into_iter().flatten().any(|source| {
                                    source.covers(
                                        route,
                                        function_key,
                                        event.before.as_deref().unwrap_or_default(),
                                        event.after.as_deref().unwrap_or_default(),
                                    )
                                })
                            });
                        if all {
                            sources.push(json!({"step": step.number,
                                "derived_from_function_rows": function_moves}));
                        }
                    }
                    let candidates: Vec<&String> = step_changes[position]
                        .1
                        .iter()
                        .filter(|name| {
                            row["evidence"].as_array().into_iter().flatten().any(|evidence| {
                                evidence["step"] == json!(step.number)
                                    && evidence["evidence"]["contract_dependencies_after"]
                                        ["declarations"]
                                        .as_array()
                                        .is_some_and(|names| names.contains(&json!(name)))
                            })
                        })
                        .collect();
                    if !candidates.is_empty() {
                        row["dependency_changes"] = json!({"step": step.number,
                            "formula": "cx_dependency_change_listed",
                            "declarations": candidates});
                    }
                    if sources.is_empty() {
                        unattributed.push(step.number);
                    } else {
                        attributed.extend(sources);
                    }
                }
                let review: Vec<&Value> = row["new_safe_in"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|step| unattributed.iter().any(|number| json!(number) == **step))
                    .collect();
                let review = json!(review);
                row["attributed"] = json!(attributed);
                row["unattributed_in"] = json!(unattributed);
                row["review"] = json!(review.as_array().is_some_and(|r| !r.is_empty()));
                if !unattributed.is_empty() {
                    unattributed_moves.push(json!({"route": route, "row": key,
                        "steps": unattributed}));
                }
                if row["review"] == json!(true) {
                    unattributed_new_safe.push(json!({"route": route, "row": key,
                        "steps": review, "formula": "cx_unattributed_new_safe_is_reviewed"}));
                }
                *counts
                    .entry(match row["pattern"].as_str() {
                        Some("moved") => "moved",
                        Some("cancelled") => "cancelled",
                        Some("cascade") => "cascade",
                        _ => "evidence_only",
                    })
                    .or_default() += 1;
                rows.push(row);
            }
            let rekeys: Vec<Value> = segment
                .iter()
                .zip(&entries)
                .flat_map(|(step, entry)| {
                    entry.map(rekeyed).unwrap_or_default().into_iter().map(
                        move |mut rekey| {
                            rekey["step"] = json!(step.number);
                            rekey
                        },
                    )
                })
                .collect();
            let complete = causes.is_empty();
            every_route_complete &= complete;
            let totals = |entry: Option<&Value>, side: &str| {
                entry.map_or(Value::Null, |entry| entry["totals"][side]["function_status"].clone())
            };
            let net_delta = {
                let (first, last) = (
                    totals(entries[0], "before"),
                    totals(entries[entries.len() - 1], "after"),
                );
                let statuses: BTreeSet<&String> = first
                    .as_object()
                    .into_iter()
                    .flatten()
                    .chain(last.as_object().into_iter().flatten())
                    .map(|(status, _)| status)
                    .collect();
                statuses
                    .into_iter()
                    .map(|status| {
                        let count = |totals: &Value| totals[status].as_i64().unwrap_or(0);
                        (status.clone(), count(&last) - count(&first))
                    })
                    .collect::<BTreeMap<_, _>>()
            };
            route_records.insert(
                route.clone(),
                json!({
                    "status": if complete { "complete" } else { "incomplete" },
                    "incomplete_because": causes,
                    "per_step": segment.iter().zip(&entries).map(|(step, entry)| json!({
                        "step": step.number,
                        "status": entry.map_or(json!("not-measured"), |e| e["status"].clone()),
                        "function_rows": entry.map_or(Value::Null, |e| e["function_rows"].clone()),
                        "status_deltas": entry.map_or(Value::Null, |e| e["status_deltas"].clone()),
                        "composition_conflicts": entry.map_or(Value::Null,
                            |e| e["composition"]["conflicts"].clone()),
                    })).collect::<Vec<_>>(),
                    "net_status_delta": if complete { json!(net_delta) } else {
                        json!("not derived: the route's chain is incomplete") },
                    "counts": counts,
                    "rekeyed": rekeys,
                    "rows": rows,
                }),
            );
        }
        segment_records.push(json!({
            "steps": segment.iter().map(|step| step.number).collect::<Vec<_>>(),
            "points": std::iter::once(segment[0].before.to_json())
                .chain(segment.iter().map(|step| step.after.to_json()))
                .collect::<Vec<_>>(),
            "contract_changes": segment.iter().zip(&step_changes).map(|(step, changes)| json!({
                "step": step.number,
                "bundles": diff_maps(&step.before.bundles, &step.after.bundles),
                "dependency_graph": changes.2,
            })).collect::<Vec<_>>(),
            "routes": route_records,
        }));
    }
    let status = if !links_joined {
        "refused"
    } else if every_route_complete {
        "complete"
    } else {
        "incomplete"
    };
    Ok(json!({
        "schema": CHAIN_SCHEMA,
        "tool": tool(),
        "status": status,
        "steps": steps.iter().map(|step| json!({
            "step": step.number,
            "source": step.source,
            "tool": step.tool,
            "comparison": step.comparison,
            "before": step.before.to_json(),
            "after": step.after.to_json(),
            "declared": step.declared.get("path").cloned().unwrap_or(Value::Null),
        })).collect::<Vec<_>>(),
        "attribution_sources": attributions.iter().map(|(number, sources)| json!({
            "step": number,
            "sources": sources.iter().map(|s| json!({"declared": s.label, "sha256": s.sha256,
                "kind": if s.declaration.is_some() { "prediction" } else { "record-declared" }}))
                .collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "links": links,
        "segments": segment_records,
        "unattributed_moves": unattributed_moves,
        "unattributed_new_safe": unattributed_new_safe,
        "formulas": qcore_schemas::connect::FORMULAS.iter()
            .filter(|(id, _)| id.starts_with("cx_refuse") || id.starts_with("cx_incomplete")
                || id.starts_with("cx_joined") || id.starts_with("cx_every_identity")
                || id.starts_with("cx_unattributed") || id.starts_with("cx_net")
                || id.starts_with("cx_moved") || id.starts_with("cx_unlisted")
                || id.starts_with("cx_dependency_change"))
            .map(|(id, text)| (*id, *text))
            .collect::<BTreeMap<_, _>>(),
    }))
}

/// Answer `measure chain`.
///
/// # Errors
///
/// A malformed argument or an unloadable step.
pub fn answer(request: &Request, arguments: &[String]) -> Result<Response, String> {
    let options = parse(arguments)?;
    let output = request
        .output_directory()
        .canonicalize()
        .map_err(|error| format!("{}: {error}", request.output_directory().display()))?;
    let mut steps = Vec::new();
    for (position, spec) in options.steps.iter().enumerate() {
        let number = position + 1;
        steps.push(match spec {
            StepSpec::Record { path, comparison } => load_record_step(
                number,
                &request.resolve_argument(path),
                comparison.as_deref(),
            )?,
            StepSpec::Runs { before, after } => load_runs_step(
                number,
                &request.resolve_argument(before),
                &request.resolve_argument(after),
            )?,
        });
    }
    let predictions: Vec<(usize, PathBuf)> = options
        .declared
        .iter()
        .map(|(number, path)| (*number, request.resolve_argument(path)))
        .collect();
    let record = chain(&steps, &predictions)?;
    let path = output.join(CHAIN_FILE);
    write_json(&path, &record)?;
    let mut response = Response::completed();
    response.push_artifact(&output, Path::new(CHAIN_FILE), "measure-chain")?;
    for entry in record["links"].as_array().into_iter().flatten() {
        if entry["joined"] != json!(true) {
            response.push_diagnostic(
                Severity::Warning,
                "chain-link-refused",
                format!(
                    "steps {}: {}",
                    entry["between"],
                    entry["refused_because"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|r| format!("{} ({})", r["formula"], r["field"]))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
    }
    let review = record["unattributed_new_safe"]
        .as_array()
        .map_or(0, Vec::len);
    if record["status"] != json!("complete") {
        return Ok(Response::failed(format!(
            "the chain is {}: every refused link and incomplete route is named in {}",
            record["status"].as_str().unwrap_or_default(),
            path.display()
        )));
    }
    if review > 0 {
        return Ok(Response::failed(format!(
            "{review} unattributed new Safe row(s) are flagged for review in {}; none is accepted",
            path.display()
        )));
    }
    Ok(response)
}
