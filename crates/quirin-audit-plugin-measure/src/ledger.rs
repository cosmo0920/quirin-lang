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

//! Per-rule contribution ledgers (card **CONTRIBUTION-LEDGERS**, plan seam A,
//! "Per-rule contribution ledgers"): the native-comparison record's
//! `contribution` slot.
//!
//! A ledger attributes the function rows one binary's rule switch moves on one
//! evaluation profile. It is measured from a single binary as TIP against
//! TIP-with-rules-off (`QCORE_GATEWAY_RULES_OFF`), never from two revisions,
//! so a sibling's landing cannot move it. The schema, its checks, the
//! repository layout and the profile consistency check are owned by
//! `scripts/contribution_ledgers.py` (`quirin-contribution-ledger/1`); this
//! module only builds the ledger bodies from the comparisons, and the Python
//! converter stamps the record digest and the profile baseline when it writes
//! them into the repository.
//!
//! **Attribution** (plan §7: ledgers compose only if interactions are named
//! rows). `--rules-off A` measures `tip-off -> tip`; every moved row is rule
//! A's, and the ledger says its interactions with other rules were not
//! measured. `--rules-off A,B --per-rule` also measures `tip-off-A -> tip` and
//! `tip-off-B -> tip`. A row is attributed to rule A alone exactly when the
//! switch of A alone moves it, no other single switch moves it, and the joint
//! switch moves it the same way. Every other row moved by any of these
//! comparisons is an **interaction row**, named under the rules whose single
//! switches move it (or all switched rules when no single switch, or only
//! one with a different joint move, does), and never attributed to a rule
//! alone. `--rules-off A,B` without `--per-rule` attributes every row to the
//! interaction `A+B`: the rules are not separated.
//!
//! **Identity.** The group's base is the joint-off side's status totals; its
//! rule ledgers' deltas plus its interaction ledgers' joint deltas re-derive
//! the TIP side's totals exactly, because every row the joint comparison
//! moves is in exactly one of them. The builder checks this and refuses a
//! group that does not add up (it never repairs a total); the profile's
//! published total is still re-derived from a real run at landing and checked
//! against these ledgers by `contribution_ledgers.py consistency`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

/// The ledger schema (`scripts/contribution_ledgers.py` is its authority).
pub const LEDGER_SCHEMA: &str = "quirin-contribution-ledger/1";

/// How every ledger this module builds was measured.
pub const SOURCE: &str = "measured-by-switch";

/// One measurement's rule switches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Switch {
    /// The side measured with and without the rules.
    pub tip: String,
    /// The joint-off side (every listed rule off).
    pub joint: String,
    /// The switched rule ids, in [`rule_order`].
    pub rules: Vec<String>,
    /// `(rule id, side)` of each single-rule-off side (`--per-rule`).
    pub per_rule: Vec<(String, String)>,
}

impl Switch {
    /// The comparison label of `side -> tip`.
    #[must_use]
    pub fn label(&self, side: &str) -> String {
        format!("{side}->{}", self.tip)
    }

    /// Every switched side with its off-list, the joint side first.
    #[must_use]
    pub fn sides(&self) -> Vec<(String, Vec<String>)> {
        let mut sides = vec![(self.joint.clone(), self.rules.clone())];
        sides.extend(
            self.per_rule
                .iter()
                .map(|(id, side)| (side.clone(), vec![id.clone()])),
        );
        sides
    }
}

/// The order of rule ids: numeric prefix, then the rest (`32 < 32a < 33 <
/// 100`). The registry's consultation order replaces it once
/// `crates/gateway-rules/registry.toml` exists (GATEWAY-RULE-API).
#[must_use]
pub fn rule_order(id: &str) -> (u64, String) {
    let digits: String = id.chars().take_while(char::is_ascii_digit).collect();
    (
        digits.parse().unwrap_or(u64::MAX),
        id[digits.len()..].to_owned(),
    )
}

/// The key of an interaction among `rules`: the ids in [`rule_order`],
/// joined by `+`.
#[must_use]
pub fn interaction_key(rules: &BTreeSet<String>) -> String {
    let mut ids: Vec<&String> = rules.iter().collect();
    ids.sort_by_key(|id| rule_order(id));
    ids.into_iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("+")
}

/// A moved row of one comparison: status before and after, and its entry.
#[derive(Clone, Debug)]
struct Moved {
    before: String,
    after: String,
    entry: Value,
}

fn moved_rows(route: &Value) -> BTreeMap<String, Moved> {
    let mut rows = BTreeMap::new();
    for entry in route["moves"].as_array().into_iter().flatten() {
        let (Some(row), Some(before), Some(after)) = (
            entry["row"].as_str(),
            entry["status"][0].as_str(),
            entry["status"][1].as_str(),
        ) else {
            continue;
        };
        rows.insert(
            row.to_owned(),
            Moved {
                before: before.into(),
                after: after.into(),
                entry: entry.clone(),
            },
        );
    }
    for entry in route["unmatched"].as_array().into_iter().flatten() {
        let (Some(row), Some(status)) = (entry["row"].as_str(), entry["status"].as_str()) else {
            continue;
        };
        let (before, after) = if entry["only"] == "before" {
            (status, "absent")
        } else {
            ("absent", status)
        };
        rows.insert(
            row.to_owned(),
            Moved {
                before: before.into(),
                after: after.into(),
                entry: entry.clone(),
            },
        );
    }
    rows
}

fn content_rows(route: &Value) -> BTreeMap<String, Value> {
    route["content_changes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| Some((entry["row"].as_str()?.to_owned(), entry.clone())))
        .collect()
}

fn function_status(route: &Value, side: &str) -> BTreeMap<String, i64> {
    route["totals"][side]["function_status"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(status, count)| (status.clone(), count.as_i64().unwrap_or(0)))
        .collect()
}

fn add_move(delta: &mut BTreeMap<String, i64>, before: &str, after: &str) {
    if before == after {
        return;
    }
    if before != "absent" {
        *delta.entry(before.to_owned()).or_insert(0) -= 1;
    }
    if after != "absent" {
        *delta.entry(after.to_owned()).or_insert(0) += 1;
    }
}

fn nonzero(delta: BTreeMap<String, i64>) -> BTreeMap<String, i64> {
    delta.into_iter().filter(|(_, count)| *count != 0).collect()
}

fn leaves(entry: &Value, field: &str) -> Value {
    entry.get(field).cloned().unwrap_or_else(|| json!([]))
}

/// A ledger's distinct named premises, or `None` when a row's premises could
/// not be resolved.
fn premise_names(rows: &[Value]) -> Option<Vec<String>> {
    let mut names = BTreeSet::new();
    for row in rows {
        let premises = &row["premises"];
        if premises.is_null() || premises.get("unresolved").is_some() {
            return None;
        }
        for listed in premises["declarations"].as_object().into_iter().flatten() {
            for name in listed.1.as_array().into_iter().flatten() {
                if let Some(name) = name.as_str() {
                    names.insert(name.to_owned());
                }
            }
        }
    }
    Some(names.into_iter().collect())
}

/// The rows one ledger holds, before they are written.
#[derive(Default)]
struct Bucket {
    moved: Vec<Value>,
    delta: BTreeMap<String, i64>,
    traded: Vec<Value>,
    unknown: Vec<Value>,
}

impl Bucket {
    fn content(&mut self, row: &str, entry: &Value) {
        let status = entry["status"][1].as_str().unwrap_or_default();
        let (removed, added) = (
            leaves(entry, "leaves_removed"),
            leaves(entry, "leaves_added"),
        );
        let traded = removed.as_array().is_some_and(|items| !items.is_empty())
            || added.as_array().is_some_and(|items| !items.is_empty());
        if traded {
            self.traded.push(json!({"row": row, "status": status,
                "leaves_removed": removed, "leaves_added": added}));
        }
        if status == "Unknown" {
            self.unknown.push(json!({"row": row,
                "why": entry.get("after_leaves").cloned().unwrap_or(Value::Null)}));
        }
    }
}

fn moved_entry(row: &str, moved: &Moved) -> Value {
    json!({
        "row": row,
        "before": moved.before,
        "after": moved.after,
        "premises": moved.entry.get("premises").cloned().unwrap_or(Value::Null),
        "leaves_removed": leaves(&moved.entry, "leaves_removed"),
        "leaves_added": leaves(&moved.entry, "leaves_added"),
    })
}

/// The comparison entry of `label` for `route`, or why there is none.
fn entry<'a>(comparisons: &'a [Value], label: &str, route: &str) -> Result<&'a Value, String> {
    let found = comparisons
        .iter()
        .find(|comparison| comparison["comparison"] == label)
        .map(|comparison| &comparison["routes"][route])
        .filter(|entry| !entry.is_null())
        .ok_or_else(|| format!("comparison {label} has no route {route}"))?;
    if found["status"] == "not-compared" {
        return Err(format!(
            "comparison {label} route {route} was not compared ({})",
            found["why"]
        ));
    }
    Ok(found)
}

/// A single-rule comparison: the rule, its label, its moved rows and its
/// content-changed rows.
type Single = (
    String,
    String,
    BTreeMap<String, Moved>,
    BTreeMap<String, Value>,
);

/// The ledgers of one route.
///
/// # Errors
///
/// A switched comparison missing or not compared, or a group whose ledgers do
/// not re-derive the TIP side's totals.
#[allow(clippy::too_many_lines)]
pub fn route_ledgers(
    switch: &Switch,
    comparisons: &[Value],
    route: &str,
    profile: &str,
    measurement: &Value,
) -> Result<Vec<Value>, String> {
    let joint_label = switch.label(&switch.joint);
    let joint = entry(comparisons, &joint_label, route)?;
    let joint_moves = moved_rows(joint);
    let mut singles: Vec<Single> = Vec::new();
    if switch.rules.len() == 1 {
        singles.push((
            switch.rules[0].clone(),
            joint_label.clone(),
            joint_moves.clone(),
            content_rows(joint),
        ));
    } else {
        for (id, side) in &switch.per_rule {
            let label = switch.label(side);
            let single = entry(comparisons, &label, route)?;
            singles.push((id.clone(), label, moved_rows(single), content_rows(single)));
        }
    }
    let all: BTreeSet<String> = switch.rules.iter().cloned().collect();
    // Partition every moved row.
    let mut rules: BTreeMap<String, Bucket> = singles
        .iter()
        .map(|(id, ..)| (id.clone(), Bucket::default()))
        .collect();
    let mut interactions: BTreeMap<String, (BTreeSet<String>, Bucket)> = BTreeMap::new();
    let mut keys: BTreeSet<&String> = joint_moves.keys().collect();
    for (_, _, moves, _) in &singles {
        keys.extend(moves.keys());
    }
    for row in keys {
        let movers: Vec<&Single> = singles
            .iter()
            .filter(|(_, _, moves, _)| moves.contains_key(row))
            .collect();
        let jointly = joint_moves.get(row);
        if let ([(id, _, moves, _)], Some(jointly)) = (movers.as_slice(), jointly) {
            let single = &moves[row];
            if single.before == jointly.before && single.after == jointly.after {
                let bucket = rules.get_mut(id).expect("known rule");
                bucket.moved.push(moved_entry(row, single));
                add_move(&mut bucket.delta, &single.before, &single.after);
                continue;
            }
        }
        let named: BTreeSet<String> = if movers.len() >= 2 {
            movers.iter().map(|(id, ..)| id.clone()).collect()
        } else {
            all.clone()
        };
        let key = interaction_key(&named);
        let (_, bucket) = interactions
            .entry(key)
            .or_insert_with(|| (named, Bucket::default()));
        let shown = jointly.or_else(|| movers.first().map(|(_, _, moves, _)| &moves[row]));
        let mut moved = moved_entry(row, shown.expect("moved somewhere"));
        moved["joint"] = jointly.map_or(
            Value::Null,
            |m| json!({"before": m.before, "after": m.after}),
        );
        moved["per_rule"] = movers
            .iter()
            .map(|(id, _, moves, _)| {
                let m = &moves[row];
                (id.clone(), json!({"before": m.before, "after": m.after}))
            })
            .collect::<serde_json::Map<_, _>>()
            .into();
        if let Some(jointly) = jointly {
            add_move(&mut bucket.delta, &jointly.before, &jointly.after);
        }
        bucket.moved.push(moved);
    }
    // Rows whose content (not status) the switches change, partitioned the
    // same way: leaves traded, and the rows that stay Unknown.
    let joint_content = content_rows(joint);
    let mut content_keys: BTreeSet<&String> = joint_content.keys().collect();
    for (_, _, _, content) in &singles {
        content_keys.extend(content.keys());
    }
    for row in content_keys {
        let changers: Vec<&String> = singles
            .iter()
            .filter(|(_, _, _, content)| content.contains_key(row))
            .map(|(id, ..)| id)
            .collect();
        if let [id] = changers.as_slice() {
            let single = singles
                .iter()
                .find(|(known, ..)| known == *id)
                .expect("known");
            rules
                .get_mut(*id)
                .expect("known rule")
                .content(row, &single.3[row]);
            continue;
        }
        let named: BTreeSet<String> = if changers.len() >= 2 {
            changers.into_iter().cloned().collect()
        } else {
            all.clone()
        };
        let shown = joint_content
            .get(row)
            .or_else(|| {
                singles
                    .iter()
                    .find_map(|(_, _, _, content)| content.get(row))
            })
            .expect("changed somewhere");
        interactions
            .entry(interaction_key(&named))
            .or_insert_with(|| (named, Bucket::default()))
            .1
            .content(row, shown);
    }
    // The identity: base + every ledger's delta = TIP.
    let base = function_status(joint, "before");
    let on = function_status(joint, "after");
    let mut derived = base.clone();
    for bucket in rules.values().chain(interactions.values().map(|(_, b)| b)) {
        for (status, count) in &bucket.delta {
            *derived.entry(status.clone()).or_insert(0) += count;
        }
    }
    if nonzero(derived.clone()) != nonzero(on.clone()) {
        return Err(format!(
            "route {route}: the base {base:?} plus the ledgers' deltas gives {derived:?}, not the \
             TIP totals {on:?}; a group that does not add up is refused, never repaired"
        ));
    }
    let interaction_keys: Vec<String> = interactions.keys().cloned().collect();
    let tip_figures = joint["figures"]
        .get("after")
        .cloned()
        .unwrap_or(Value::Null);
    let group = json!({
        "rules": switch.rules,
        "per_rule": !switch.per_rule.is_empty(),
        "interactions": interaction_keys,
        "joint_comparison": joint_label,
        "base": base,
        "on": on,
        "tip_figures": tip_figures,
        "interactions_measured": switch.rules.len() > 1 && !switch.per_rule.is_empty(),
    });
    let ledger = |kind: &str,
                  key: &str,
                  ids: Vec<String>,
                  comparison: &str,
                  off: Value,
                  bucket: Bucket| {
        let premises = premise_names(&bucket.moved);
        let mut measured = measurement.clone();
        measured["comparison"] = json!(comparison);
        json!({
            "schema": LEDGER_SCHEMA,
            "kind": kind,
            "key": key,
            "rules": ids,
            "profile": profile,
            "route": route,
            "source": SOURCE,
            "measurement": measured,
            "group": group,
            "totals": {"off": off, "on": on},
            "delta": nonzero(bucket.delta),
            "rows_moved": bucket.moved,
            "premises": premises,
            "leaves_traded": {
                "removed": bucket.traded.iter()
                    .map(|row| row["leaves_removed"].as_array().map_or(0, Vec::len)).sum::<usize>(),
                "added": bucket.traded.iter()
                    .map(|row| row["leaves_added"].as_array().map_or(0, Vec::len)).sum::<usize>(),
                "rows": bucket.traded,
            },
            "rows_unknown": bucket.unknown,
        })
    };
    let mut ledgers = Vec::new();
    let mut by_id: BTreeMap<String, (String, BTreeMap<String, i64>)> = BTreeMap::new();
    for (id, label, ..) in &singles {
        let single = entry(comparisons, label, route)?;
        by_id.insert(
            id.clone(),
            (label.clone(), function_status(single, "before")),
        );
    }
    // Without --per-rule, two or more rules are not separated: every row is
    // the interaction of all of them, and no rule ledger is written.
    let separated = switch.rules.len() == 1 || !switch.per_rule.is_empty();
    if separated {
        for (id, bucket) in rules {
            let (label, off) = by_id.remove(&id).expect("known");
            ledgers.push(ledger(
                "rule",
                &id,
                vec![id.clone()],
                &label,
                json!(off),
                bucket,
            ));
        }
    }
    for (key, (named, bucket)) in interactions {
        let mut ids: Vec<String> = named.into_iter().collect();
        ids.sort_by_key(|id| rule_order(id));
        ledgers.push(ledger(
            "interaction",
            &key,
            ids,
            &joint_label,
            json!(base),
            bucket,
        ));
    }
    Ok(ledgers)
}

/// The record's `contribution` slot.
///
/// # Errors
///
/// Two routes of one measurement on the same profile (a ledger is per rule
/// and profile), or a route whose ledgers cannot be built.
pub fn slot(
    switch: Option<&Switch>,
    comparisons: &[Value],
    routes: &[(String, String)],
    measurement: &Value,
) -> Result<Value, String> {
    let Some(switch) = switch else {
        return Ok(json!({"schema": LEDGER_SCHEMA, "rules": [], "ledgers": []}));
    };
    let mut seen = BTreeSet::new();
    for (route, profile) in routes {
        if !seen.insert(profile) {
            return Err(format!(
                "route {route}: profile {profile} is measured by two routes; a contribution \
                 ledger is per rule and profile"
            ));
        }
    }
    let mut ledgers = Vec::new();
    for (route, profile) in routes {
        ledgers.extend(route_ledgers(
            switch,
            comparisons,
            route,
            profile,
            measurement,
        )?);
    }
    Ok(json!({
        "schema": LEDGER_SCHEMA,
        "rules": switch.rules,
        "per_rule": !switch.per_rule.is_empty(),
        "comparisons": switch.sides().iter().map(|(side, off)| json!({
            "comparison": switch.label(side), "rules_off": off})).collect::<Vec<_>>(),
        "ledgers": ledgers,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(
        moves: &[(&str, &str, &str)],
        content: &[(&str, &str)],
        before: Value,
        after: Value,
    ) -> Value {
        json!({
            "status": "complete",
            "totals": {"before": {"function_status": before}, "after": {"function_status": after}},
            "moves": moves.iter().map(|(row, b, a)| json!({"row": row, "status": [b, a],
                "leaves_removed": [{"description": "leaf"}], "leaves_added": [],
                "premises": {"declarations": {"d": ["p1"]}}})).collect::<Vec<_>>(),
            "unmatched": [],
            "content_changes": content.iter().map(|(row, s)| json!({"row": row, "status": [s, s],
                "leaves_removed": [{"description": "x"}], "leaves_added": [{"description": "y"}],
                "after_leaves": [{"description": "y"}]})).collect::<Vec<_>>(),
        })
    }

    fn comparison(label: &str, route: Value) -> Value {
        json!({"comparison": label, "routes": {"r": route}})
    }

    fn switch(rules: &[&str], per_rule: bool) -> Switch {
        Switch {
            tip: "tip".into(),
            joint: "tip-off".into(),
            rules: rules.iter().map(|id| (*id).to_owned()).collect(),
            per_rule: if per_rule {
                rules
                    .iter()
                    .map(|id| ((*id).to_owned(), format!("tip-off-{id}")))
                    .collect()
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn rule_order_is_numeric_then_suffix() {
        let mut ids = vec!["33", "100", "32a", "32", "9"];
        ids.sort_by_key(|id| rule_order(id));
        assert_eq!(ids, ["9", "32", "32a", "33", "100"]);
        let set: BTreeSet<String> = ["35".to_owned(), "33".to_owned()].into();
        assert_eq!(interaction_key(&set), "33+35");
    }

    #[test]
    fn one_rule_owns_every_row_its_switch_moves() {
        let comparisons = [comparison(
            "tip-off->tip",
            route(
                &[("f", "Unknown", "Safe"), ("g", "Unknown", "Safe")],
                &[("h", "Unknown")],
                json!({"Safe": 1, "Unknown": 3}),
                json!({"Safe": 3, "Unknown": 1}),
            ),
        )];
        let ledgers =
            route_ledgers(&switch(&["33"], false), &comparisons, "r", "p", &json!({})).unwrap();
        assert_eq!(ledgers.len(), 1);
        let ledger = &ledgers[0];
        assert_eq!(ledger["kind"], "rule");
        assert_eq!(ledger["delta"], json!({"Safe": 2, "Unknown": -2}));
        assert_eq!(ledger["rows_moved"].as_array().unwrap().len(), 2);
        assert_eq!(ledger["premises"], json!(["p1"]));
        assert_eq!(ledger["rows_unknown"][0]["row"], "h");
        assert_eq!(ledger["leaves_traded"]["removed"], 1);
        assert_eq!(ledger["group"]["interactions_measured"], false);
    }

    #[test]
    fn rows_two_rules_move_together_are_named_interactions() {
        // f: rule 33 alone; g: needs both (each single switch moves it);
        // h: either suffices (only the joint switch moves it).
        let totals_on = json!({"Safe": 4, "Unknown": 0});
        let comparisons = [
            comparison(
                "tip-off->tip",
                route(
                    &[
                        ("f", "Unknown", "Safe"),
                        ("g", "Unknown", "Safe"),
                        ("h", "Unknown", "Safe"),
                    ],
                    &[],
                    json!({"Safe": 1, "Unknown": 3}),
                    totals_on.clone(),
                ),
            ),
            comparison(
                "tip-off-33->tip",
                route(
                    &[("f", "Unknown", "Safe"), ("g", "Unknown", "Safe")],
                    &[],
                    json!({"Safe": 2, "Unknown": 2}),
                    totals_on.clone(),
                ),
            ),
            comparison(
                "tip-off-35->tip",
                route(
                    &[("g", "Unknown", "Safe")],
                    &[],
                    json!({"Safe": 3, "Unknown": 1}),
                    totals_on,
                ),
            ),
        ];
        let ledgers = route_ledgers(
            &switch(&["35", "33"], true),
            &comparisons,
            "r",
            "p",
            &json!({}),
        )
        .unwrap();
        let by_key: BTreeMap<&str, &Value> = ledgers
            .iter()
            .map(|ledger| (ledger["key"].as_str().unwrap(), ledger))
            .collect();
        assert_eq!(
            by_key.keys().copied().collect::<Vec<_>>(),
            ["33", "33+35", "35"]
        );
        assert_eq!(by_key["33"]["rows_moved"][0]["row"], "f");
        assert_eq!(
            by_key["35"]["rows_moved"],
            json!([]),
            "g is not rule 35's alone"
        );
        let interaction = by_key["33+35"];
        assert_eq!(interaction["kind"], "interaction");
        let rows: Vec<&str> = interaction["rows_moved"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["row"].as_str().unwrap())
            .collect();
        assert_eq!(rows, ["g", "h"]);
        assert_eq!(interaction["delta"], json!({"Safe": 2, "Unknown": -2}));
        assert_eq!(interaction["rows_moved"][1]["per_rule"], json!({}));
        assert_eq!(interaction["group"]["interactions"], json!(["33+35"]));
    }

    #[test]
    fn a_group_that_does_not_add_up_is_refused() {
        let comparisons = [comparison(
            "tip-off->tip",
            route(
                &[("f", "Unknown", "Safe")],
                &[],
                json!({"Safe": 1, "Unknown": 3}),
                json!({"Safe": 3, "Unknown": 1}),
            ),
        )];
        let error =
            route_ledgers(&switch(&["33"], false), &comparisons, "r", "p", &json!({})).unwrap_err();
        assert!(error.contains("never repaired"), "{error}");
    }

    #[test]
    fn unseparated_rules_are_one_interaction() {
        let comparisons = [comparison(
            "tip-off->tip",
            route(
                &[("f", "Unknown", "Safe")],
                &[],
                json!({"Safe": 1, "Unknown": 1}),
                json!({"Safe": 2}),
            ),
        )];
        let ledgers = route_ledgers(
            &switch(&["33", "35"], false),
            &comparisons,
            "r",
            "p",
            &json!({}),
        )
        .unwrap();
        assert_eq!(ledgers.len(), 1);
        assert_eq!(ledgers[0]["key"], "33+35");
    }

    #[test]
    fn two_routes_on_one_profile_are_refused() {
        let routes = [
            ("a".to_owned(), "p".to_owned()),
            ("b".to_owned(), "p".to_owned()),
        ];
        let error = slot(Some(&switch(&["33"], false)), &[], &routes, &json!({})).unwrap_err();
        assert!(error.contains("per rule and profile"), "{error}");
        assert_eq!(
            slot(None, &[], &routes, &json!({})).unwrap()["rules"],
            json!([])
        );
    }
}
