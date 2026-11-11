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

//! Card **NULL-CANDIDATE-TRIAGE** (gateway rules 14t–14v): the null
//! property's fold over a whole run, after every unit is analysed and before
//! the null rows are written. No unit is analysed twice.
//!
//! * **Declared members** (rule 14t): every store some unit made to a
//!   declared `TAG.FIELD` is evidence; one not proved non-null withdraws the
//!   declaration ([`qcore_c_gateway::null_deref::null_triage::member_standing`]).
//! * **Declared results** (rule 14u): every analysed body that defines the
//!   function is evidence (rule 13g's fold,
//!   [`qcore_c_gateway::null_deref::null_triage::result_standing`]); one that returns null on
//!   a path withdraws it.
//! * **Withdrawal**: a check proved by a withdrawn declaration is a
//!   candidate again, and a row whose dependency set names one is never
//!   `Safe` or `Pending` ([`qcore_c_gateway::null_deref::null_triage::demote`]).
//! * **Cross-unit travel** (rule 14v): an external-linkage definition
//!   analysed in exactly one unit exports its inferred `quant null-deref`
//!   clauses and its non-null result. A caller in another unit gains a
//!   `call-precondition` check of each matching actual (judged where the
//!   walk recorded it), and a candidate whose every source is the untested
//!   result of such callees is proved. A clause or result never travels from
//!   an ambiguous definition (two units define the name), across a
//!   parameter-count mismatch, or from a summary resting on a withdrawn
//!   declaration.
//!
//! The custody report never reads any of this.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::null_deref::core_model::{
    AWAITING_REVIEW, CANDIDATE, NullVerdict, Outcome, Request, UNREACHED_VIOLATION, compose,
    null_verdict,
};
use qcore_c_gateway::null_deref::null_triage::{
    FINDING, ResultEvidence, WITHDRAWN, demote, member_standing, result_standing, triaged_verdict,
};
use qcore_contract::bounds_contract::{BoundsClause, BoundsContracts};
use serde_json::{Value, json};

/// What the fold section means, carried in it.
pub const MEANING: &str = "the run's fold of the null property (card NULL-CANDIDATE-TRIAGE, rules 14t-14v), after every unit and before the rows were written: each declared non-null member's standing (`withdrawn` when a store some analysed unit made is not proved non-null, `partly-validated` when every examined store is proved and the zero-initialised construction premise stays uncovered, `declared-assumption` when no analysed unit stores it); each declared non-null result's standing (rule 13g's fold over the analysed bodies); each declared integer result range's standing (card NULL-TRIAGE-UNRESOLVED-PATHS, rule 15q, the same fold over the bodies' range evidence); the rows a withdrawn declaration demoted; and cross-unit travel: the caller checks an external definition's inferred clauses added, and the candidates an external definition's non-null result proved. Nothing here is read by custody";

fn verdict_of(text: &str) -> NullVerdict {
    match text {
        "Safe" => NullVerdict::Safe,
        "Pending" => NullVerdict::Pending,
        "Unsafe" => NullVerdict::Unsafe,
        _ => NullVerdict::Unknown,
    }
}

fn custody_of(row: &Value) -> Option<NullVerdict> {
    row["custody"].as_str().map(verdict_of)
}

/// The outcome classes of a row's checks as the core's outcomes (a finding
/// is a candidate the triage validated).
fn outcomes_of(checks: &[Value]) -> (Vec<Outcome>, usize) {
    let mut findings = 0;
    let outcomes = checks
        .iter()
        .map(|check| match check["outcome"].as_str().unwrap_or("") {
            "proved" => Outcome::Proved(BTreeSet::new()),
            "moved" => Outcome::Moved(BTreeSet::new()),
            "excluded" => Outcome::Excluded,
            "held" => Outcome::Held(
                check["proofs"][0]
                    .as_str()
                    .unwrap_or("held")
                    .trim_start_matches("held:")
                    .to_owned(),
            ),
            "violation" => Outcome::Violation { reachable: true },
            "violation-reachability-unresolved" => Outcome::Violation { reachable: false },
            "finding" => {
                findings += 1;
                Outcome::Candidate(BTreeSet::new())
            }
            _ => Outcome::Candidate(BTreeSet::new()),
        })
        .collect();
    (outcomes, findings)
}

/// Recompute a row's null verdict and reasons from its checks and its
/// function-level reasons.
fn recompute(row: &mut Value, withdrawn: &BTreeSet<String>) {
    let null = &mut row["null"];
    let checks = null["checks"].as_array().cloned().unwrap_or_default();
    let (outcomes, findings) = outcomes_of(&checks);
    let function_reasons = null["reasons"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|r| {
            *r != CANDIDATE
                && *r != UNREACHED_VIOLATION
                && !r.starts_with(AWAITING_REVIEW)
                && !r.starts_with(FINDING)
                && !r.starts_with(WITHDRAWN)
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let (base, mut reasons) = null_verdict(&outcomes, &function_reasons);
    let mut verdict = triaged_verdict(base, findings);
    if findings > 0 {
        reasons.push(format!(
            "{FINDING}: {findings} validated path(s) to a NULL dereference (provisional; never an upstream defect)"
        ));
    }
    let relied = null["contract_dependencies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|d| withdrawn.contains(*d))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    verdict = demote(verdict, !relied.is_empty());
    for name in relied {
        reasons.push(format!(
            "{WITHDRAWN}: the row rests on `{name}`, which the run withdrew"
        ));
    }
    null["verdict"] = json!(verdict.as_str());
    null["reasons"] = json!(reasons);
    let combined = compose(
        Request {
            custody: true,
            null: true,
        },
        custody_of(row),
        Some(verdict),
    );
    row["combined"] = json!(combined.as_str());
}

/// What the null row says of the checks of `variable` at `line` (the JSON
/// form of the projection's premise standing; see `projection.rs`).
fn premise_standing(checks: &[Value], line: u64, variable: &str) -> String {
    let by_id = checks
        .iter()
        .filter_map(|c| c["id"].as_u64().map(|id| (id, c)))
        .collect::<BTreeMap<_, _>>();
    fn resolve(check: &Value, by_id: &BTreeMap<u64, &Value>, depth: usize) -> String {
        match check["outcome"].as_str().unwrap_or("?") {
            "proved" => "proved".into(),
            "moved" if depth < 64 => {
                let mut worst = "proved".to_owned();
                for proof in check["proofs"].as_array().into_iter().flatten() {
                    let proof = proof.as_str().unwrap_or("");
                    let resolved = match proof
                        .strip_prefix("moved:")
                        .and_then(|id| id.parse::<u64>().ok())
                    {
                        Some(origin) => match by_id.get(&origin) {
                            Some(o) if o["id"] != check["id"] => resolve(o, by_id, depth + 1),
                            _ => "unmatched".into(),
                        },
                        None => "proved".into(),
                    };
                    if resolved != "proved" {
                        worst = resolved;
                    }
                }
                worst
            }
            other => other.to_owned(),
        }
    }
    let names_it = |check: &Value| {
        let subject = check["subject"].as_str().unwrap_or("");
        subject == variable
            || subject
                .strip_prefix('&')
                .and_then(|rest| rest.strip_prefix(variable))
                .is_some_and(|rest| rest.starts_with("->") || rest.starts_with('['))
    };
    let judge = |selected: Vec<&Value>| {
        let mut standing = "proved".to_owned();
        for check in selected {
            let resolved = resolve(check, &by_id, 0);
            if resolved != "proved" {
                standing = resolved;
            }
        }
        standing
    };
    let exact = checks
        .iter()
        .filter(|c| c["line"].as_u64() == Some(line) && names_it(c))
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return judge(exact);
    }
    let nearby = checks
        .iter()
        .filter(|c| c["line"].as_u64().is_some_and(|l| l.abs_diff(line) <= 3) && names_it(c))
        .collect::<Vec<_>>();
    if nearby.is_empty() {
        "unmatched".into()
    } else {
        format!("nearby:{}", judge(nearby))
    }
}

fn restand_premises(row: &mut Value) {
    let checks = row["null"]["checks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(premises) = row["custody_null_premises"].as_array_mut() {
        for premise in premises {
            let line = premise["line"].as_u64().unwrap_or_default();
            let variable = premise["variable"].as_str().unwrap_or("").to_owned();
            premise["null_property"] = json!(premise_standing(&checks, line, &variable));
        }
    }
}

/// Fold the run: standings, withdrawals and cross-unit travel. Changes the
/// rows in place and returns the summary's `fold` section.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn fold(rows: &mut [Value], evidence: &[Value], environment: &BoundsContracts) -> Value {
    // ---- declarations
    let declarations = crate::null_member_scope_fold::declarations(environment);
    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the member facts' premises,
    // relisted on a row whose dependencies travel adds to.
    let premises = declarations.premises.clone();
    let mut results: BTreeMap<String, bool> = BTreeMap::new();
    let mut ranges: BTreeMap<String, bool> = BTreeMap::new();
    for (name, contract) in environment.null_declarations() {
        for entry in &contract.clauses {
            match &entry.clause {
                BoundsClause::NonnullResult => {
                    results.insert(name.clone(), contract.held());
                }
                // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15q).
                BoundsClause::NonnullResultRange { .. } => {
                    ranges.insert(name.clone(), contract.held());
                }
                _ => {}
            }
        }
    }
    let mut definitions: BTreeMap<String, Vec<(usize, &Value)>> = BTreeMap::new();
    let mut unit_defines: Vec<BTreeSet<String>> = Vec::new();
    for (index, unit) in evidence.iter().enumerate() {
        let mut names = BTreeSet::new();
        for definition in unit["definitions"].as_array().into_iter().flatten() {
            let name = definition["function"].as_str().unwrap_or("").to_owned();
            names.insert(name.clone());
            definitions
                .entry(name)
                .or_default()
                .push((index, definition));
        }
        unit_defines.push(names);
    }
    let mut refuted = BTreeSet::new();
    let mut result_section = serde_json::Map::new();
    for (name, held) in &results {
        let units = definitions
            .get(name)
            .into_iter()
            .flatten()
            .map(|(_, d)| {
                if d["returns_non_null"] == true {
                    ResultEvidence::Validated
                } else if d["returns_null_on_a_path"] == true {
                    ResultEvidence::Refuted
                } else {
                    ResultEvidence::Unvalidated
                }
            })
            .collect::<Vec<_>>();
        let standing = if *held {
            "held"
        } else {
            result_standing(&units)
        };
        if standing == "withdrawn" {
            refuted.insert(format!("result:{name}"));
        }
        result_section.insert(
            name.clone(),
            json!({
                "standing": standing,
                "bodies_examined": units.len(),
                "validated": units.iter().filter(|u| **u == ResultEvidence::Validated).count(),
                "refuted": units.iter().filter(|u| **u == ResultEvidence::Refuted).count(),
            }),
        );
    }
    // Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15q): each declared result
    // range, by rule 13g's fold over the bodies the units examined.
    let mut range_section = serde_json::Map::new();
    for (name, held) in &ranges {
        let units = definitions
            .get(name)
            .into_iter()
            .flatten()
            .filter_map(|(_, d)| match d["result_range"].as_str() {
                Some("validated") => Some(ResultEvidence::Validated),
                Some("refuted") => Some(ResultEvidence::Refuted),
                Some(_) => Some(ResultEvidence::Unvalidated),
                None => None,
            })
            .collect::<Vec<_>>();
        let standing = if *held {
            "held"
        } else {
            result_standing(&units)
        };
        if standing == "withdrawn" {
            refuted.insert(format!("range:{name}"));
        }
        range_section.insert(
            name.clone(),
            json!({
                "standing": standing,
                "bodies_examined": units.len(),
                "validated": units.iter().filter(|u| **u == ResultEvidence::Validated).count(),
                "refuted": units.iter().filter(|u| **u == ResultEvidence::Refuted).count(),
            }),
        );
    }
    // Card NULL-MEMBER-INVARIANT-SCOPE: withdrawal cascades through stores
    // proved only by a withdrawn declaration (members and stable members).
    let withdrawn = crate::null_member_scope_fold::withdrawals(evidence, &declarations, &refuted);
    let stores = crate::null_member_scope_fold::tally(evidence, "member_stores", &withdrawn);
    let mut member_section = serde_json::Map::new();
    for (member, (host, held)) in &declarations.members {
        let tally = stores.get(member).cloned().unwrap_or_default();
        let (examined, failed, failures) = (tally.examined, tally.failed, tally.failures);
        let standing = if *held {
            "held"
        } else {
            member_standing(examined, failed)
        };
        let mut record = json!({
            "host": host,
            "standing": standing,
            "clauses": {
                "stores": if *held { "held" } else if failed > 0 { "withdrawn" } else if examined == 0 { "unexamined" } else { "validated" },
                "construction": "declared-assumption",
            },
            "stores_examined": examined,
            "stores_not_proved": failed,
            "failures": failures,
        });
        // Card NONNULL-MEMBER-UNIFIED-QUALIFIER (USER DECISION 2026-10-16,
        // "Yes, all kinds"): a premised plain member's record lists them.
        crate::null_member_scope_fold::premise_list(
            &mut record,
            declarations.premises.get(&format!("member:{member}")),
        );
        member_section.insert(member.clone(), record);
    }
    let (stable_section, scope_section) =
        crate::null_member_scope_fold::sections(evidence, &declarations, &withdrawn);
    // ---- exports: external definitions analysed in exactly one unit
    let mut exports: BTreeMap<String, (usize, &Value)> = BTreeMap::new();
    let mut ambiguous = 0usize;
    for (name, defined) in &definitions {
        let external = defined
            .iter()
            .filter(|(_, d)| d["external"] == true)
            .collect::<Vec<_>>();
        let units = defined.iter().map(|(u, _)| *u).collect::<BTreeSet<_>>();
        match (external.as_slice(), units.len()) {
            ([single], 1) => {
                exports.insert(name.clone(), (single.0, single.1));
            }
            ([], _) => {}
            _ => ambiguous += 1,
        }
    }
    let rests_on_withdrawn = |definition: &Value| {
        definition["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|d| withdrawn.contains(d))
    };
    // Rows by (file, function) for each unit (the evidence and the rows share
    // library and file).
    let mut row_index: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        row_index.insert(
            (
                row["library"].as_str().unwrap_or("").to_owned(),
                row["file"].as_str().unwrap_or("").to_owned(),
                row["function"].as_str().unwrap_or("").to_owned(),
            ),
            index,
        );
    }
    let mut changed = BTreeSet::new();
    let mut travelled_checks: BTreeMap<String, usize> = BTreeMap::new();
    let mut travelled_rows = BTreeSet::new();
    let mut results_proved = 0usize;
    let mut result_rows = BTreeSet::new();
    let unit_of = |library: &str, file: &str| {
        evidence
            .iter()
            .position(|u| u["library"] == library && u["file"] == file)
    };
    // ---- result travel: a candidate whose every source is the untested
    // result of exported callees whose every return is non-null.
    for (index, row) in rows.iter_mut().enumerate() {
        let library = row["library"].as_str().unwrap_or("").to_owned();
        let file = row["file"].as_str().unwrap_or("").to_owned();
        let Some(unit) = unit_of(&library, &file) else {
            continue;
        };
        let mut added_dependencies = BTreeSet::new();
        let mut touched = false;
        for check in row["null"]["checks"].as_array_mut().into_iter().flatten() {
            if check["outcome"] != "candidate" {
                continue;
            }
            let sources = check["sources"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let callees = sources
                .iter()
                .map(|s| s.strip_prefix("uncontracted-result:"))
                .collect::<Option<Vec<_>>>();
            let Some(callees) = callees else {
                continue;
            };
            if callees.is_empty() {
                continue;
            }
            let proved = callees.iter().all(|callee| {
                !unit_defines[unit].contains(*callee)
                    && exports.get(*callee).is_some_and(|(home, d)| {
                        *home != unit && d["returns_non_null"] == true && !rests_on_withdrawn(d)
                    })
            });
            if !proved {
                continue;
            }
            for callee in &callees {
                added_dependencies.insert(format!("cross-unit-result:{callee}"));
                if let Some((_, d)) = exports.get(*callee) {
                    for dep in d["dependencies"].as_array().into_iter().flatten() {
                        if let Some(dep) = dep.as_str() {
                            added_dependencies.insert(dep.to_owned());
                        }
                    }
                }
            }
            check["outcome"] = json!("proved");
            check["proofs"] = json!(
                callees
                    .iter()
                    .map(|c| format!("result:cross-unit:{c}"))
                    .collect::<Vec<_>>()
            );
            if let Some(object) = check.as_object_mut() {
                object.remove("sources");
            }
            results_proved += 1;
            touched = true;
        }
        if touched {
            add_dependencies(row, &added_dependencies, &premises);
            changed.insert(index);
            result_rows.insert(index);
        }
    }
    // ---- clause travel: each exported inferred clause at a recorded actual.
    for (unit_index, unit) in evidence.iter().enumerate() {
        let library = unit["library"].as_str().unwrap_or("");
        let file = unit["file"].as_str().unwrap_or("");
        for call in unit["external_calls"].as_array().into_iter().flatten() {
            let callee = call["callee"].as_str().unwrap_or("");
            let Some((home, definition)) = exports.get(callee) else {
                continue;
            };
            if *home == unit_index
                || definition["parameters"].as_u64() != call["arguments"].as_u64()
            {
                continue;
            }
            let position = call["position"].as_u64().unwrap_or(u64::MAX);
            let Some(formal) = definition["inferred"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|pair| pair[0].as_u64() == Some(position))
                .and_then(|pair| pair[1].as_str())
            else {
                continue;
            };
            let function = call["function"].as_str().unwrap_or("").to_owned();
            let Some(&row_at) =
                row_index.get(&(library.to_owned(), file.to_owned(), function.clone()))
            else {
                continue;
            };
            let home_file = evidence[*home]["file"].as_str().unwrap_or("?");
            let row = &mut rows[row_at];
            let next = row["null"]["checks"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c["id"].as_u64())
                .max()
                .map_or(0, |m| m + 1);
            let outcome = call["outcome"].as_str().unwrap_or("candidate").to_owned();
            let mut check = json!({
                "id": next,
                "kind": "call-precondition",
                "line": call["line"],
                "subject": format!(
                    "{} at cross-unit inferred `{callee}` quant null-deref #{formal} ({home_file})",
                    call["subject"].as_str().unwrap_or("?")
                ),
                "outcome": outcome,
                "origin": call["origin"],
            });
            for key in ["sources", "proofs"] {
                if call[key].as_array().is_some_and(|a| !a.is_empty()) {
                    check[key] = call[key].clone();
                }
            }
            if let Some(checks) = row["null"]["checks"].as_array_mut() {
                checks.push(check);
            }
            add_dependencies(
                row,
                &BTreeSet::from([format!("cross-unit-inferred:{callee}")]),
                &premises,
            );
            *travelled_checks.entry(outcome).or_default() += 1;
            travelled_rows.insert(row_at);
            changed.insert(row_at);
        }
    }
    // ---- withdrawal: checks a withdrawn member or result proved.
    let mut demoted_checks = 0usize;
    for (index, row) in rows.iter_mut().enumerate() {
        let mut touched = false;
        for check in row["null"]["checks"].as_array_mut().into_iter().flatten() {
            if !matches!(check["outcome"].as_str(), Some("proved" | "moved")) {
                continue;
            }
            let withdrawn_proofs = check["proofs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter_map(|p| {
                    p.strip_prefix("result:member:")
                        .map(|m| format!("member:{m}"))
                        .or_else(|| {
                            p.strip_prefix("result:declared-result:")
                                .map(|r| format!("result:{r}"))
                        })
                        // Card NULL-MEMBER-INVARIANT-SCOPE: a fact a
                        // withdrawn stable member kept.
                        .or_else(|| {
                            p.strip_prefix("result:stable:")
                                .map(|m| format!("stable:{m}"))
                        })
                        // Card NULL-TRIAGE-UNRESOLVED-PATHS: a check a
                        // withdrawn result range proved infeasible.
                        .or_else(|| {
                            p.strip_prefix("result:declared-range:")
                                .map(|r| format!("range:{r}"))
                        })
                })
                .filter(|d| withdrawn.contains(d))
                .collect::<Vec<_>>();
            if withdrawn_proofs.is_empty() {
                continue;
            }
            check["outcome"] = json!("candidate");
            check["sources"] = json!(
                withdrawn_proofs
                    .iter()
                    .map(|d| format!("withdrawn:{d}"))
                    .collect::<Vec<_>>()
            );
            if let Some(object) = check.as_object_mut() {
                object.remove("proofs");
            }
            demoted_checks += 1;
            touched = true;
        }
        let relies = row["null"]["contract_dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(|d| withdrawn.contains(d));
        if touched || relies {
            changed.insert(index);
        }
    }
    let mut demoted_rows = 0usize;
    let mut moved = BTreeMap::<String, usize>::new();
    for index in &changed {
        let row = &mut rows[*index];
        let before = row["null"]["verdict"].as_str().unwrap_or("?").to_owned();
        recompute(row, &withdrawn);
        restand_premises(row);
        let after = row["null"]["verdict"].as_str().unwrap_or("?").to_owned();
        if before != after {
            *moved.entry(format!("{before} -> {after}")).or_default() += 1;
            if row["null"]["reasons"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|r| r.as_str().is_some_and(|r| r.starts_with(WITHDRAWN)))
            {
                demoted_rows += 1;
            }
        }
    }
    let mut section = json!({
        "meaning": MEANING,
        "member_declarations": member_section,
        "result_declarations": result_section,
        "result_range_declarations": range_section,
        "withdrawn": withdrawn,
        "withdrawal": {
            "checks_made_candidates": demoted_checks,
            "rows_demoted": demoted_rows,
        },
        "travel": {
            "exported_definitions": exports.len(),
            "ambiguous_external_definitions": ambiguous,
            "clause_checks_added_by_outcome": travelled_checks,
            "rows_with_an_added_clause_check": travelled_rows.len(),
            "candidates_proved_by_a_cross_unit_result": results_proved,
            "rows_with_such_a_proof": result_rows.len(),
        },
        "rows_changed": changed.len(),
        "verdict_moves": moved,
    });
    // Card NULL-MEMBER-INVARIANT-SCOPE: present only when declared, so an
    // environment without them writes the section it wrote before.
    if stable_section.as_object().is_some_and(|o| !o.is_empty()) {
        section["stable_declarations"] = stable_section;
    }
    if scope_section.as_object().is_some_and(|o| !o.is_empty()) {
        section["scope_declarations"] = scope_section;
    }
    section
}

fn add_dependencies(
    row: &mut Value,
    added: &BTreeSet<String>,
    premises: &BTreeMap<String, Vec<(String, String)>>,
) {
    let mut all = row["null"]["contract_dependencies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    all.extend(added.iter().cloned());
    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: a dependency travel added
    // may name a premised member fact; the row lists its premise as the
    // analysis would have (`null_deref::attach_premises`).
    let listed = qcore_c_gateway::null_deref::premises_of(all.iter().map(String::as_str), premises);
    for premise in &listed {
        all.insert(format!(
            "{}{}",
            qcore_c_gateway::null_deref::PREMISE_DEPENDENCY,
            premise.name
        ));
    }
    row["null"]["contract_dependencies"] = json!(all);
    if !listed.is_empty() {
        row["null"]["premises"] = json!(listed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(file: &str, function: &str, checks: Value, deps: Value, verdict: &str) -> Value {
        json!({"library": "l", "file": file, "function": function, "custody": "Unknown",
               "null": {"function": function, "verdict": verdict, "reasons": [],
                        "scope": "unconditional", "checks": checks,
                        "contract_dependencies": deps},
               "combined": "Unknown"})
    }

    #[test]
    fn a_cross_unit_result_proves_a_sole_source_candidate_only() {
        let mut rows = vec![
            row(
                "a.c",
                "f",
                json!([
                    {"id": 0, "line": 3, "subject": "p", "outcome": "candidate", "sources": ["uncontracted-result:g"]},
                ]),
                json!([]),
                "Unknown",
            ),
            row(
                "a.c",
                "h",
                json!([
                    {"id": 0, "line": 3, "subject": "p", "outcome": "candidate", "sources": ["uncontracted-result:g", "formal:q"]},
                ]),
                json!([]),
                "Unknown",
            ),
        ];
        let evidence = vec![
            json!({"library": "l", "file": "a.c", "member_stores": [], "external_calls": [],
                   "definitions": [{"function": "f", "external": true, "parameters": 0, "inferred": [], "returns_non_null": false, "returns_null_on_a_path": false, "dependencies": []},
                                   {"function": "h", "external": true, "parameters": 0, "inferred": [], "returns_non_null": false, "returns_null_on_a_path": false, "dependencies": []}]}),
            json!({"library": "l", "file": "b.c", "member_stores": [], "external_calls": [],
                   "definitions": [{"function": "g", "external": true, "parameters": 0, "inferred": [], "returns_non_null": true, "returns_null_on_a_path": false, "dependencies": []}]}),
        ];
        let section = fold(&mut rows, &evidence, &BoundsContracts::default());
        assert_eq!(rows[0]["null"]["verdict"], "Safe");
        assert_eq!(
            rows[0]["null"]["checks"][0]["proofs"][0],
            "result:cross-unit:g"
        );
        // A second source keeps the candidate.
        assert_eq!(rows[1]["null"]["verdict"], "Unknown");
        assert_eq!(
            section["travel"]["candidates_proved_by_a_cross_unit_result"],
            1
        );
    }

    #[test]
    fn clause_travel_adds_a_check_and_never_improves_a_row() {
        let mut rows = vec![row(
            "a.c",
            "f",
            json!([{"id": 0, "line": 2, "subject": "x", "outcome": "proved", "proofs": ["tested:1"]}]),
            json!([]),
            "Safe",
        )];
        let evidence = vec![
            json!({"library": "l", "file": "a.c", "member_stores": [], "definitions": [{"function": "f", "external": true, "parameters": 0, "inferred": [], "returns_non_null": false, "returns_null_on_a_path": false, "dependencies": []}],
                   "external_calls": [{"function": "f", "callee": "g", "position": 0, "arguments": 1, "line": 5, "subject": "p", "outcome": "candidate", "sources": ["formal:p"]}]}),
            json!({"library": "l", "file": "b.c", "member_stores": [], "external_calls": [],
                   "definitions": [{"function": "g", "external": true, "parameters": 1, "inferred": [[0, "q"]], "returns_non_null": false, "returns_null_on_a_path": false, "dependencies": []}]}),
        ];
        let section = fold(&mut rows, &evidence, &BoundsContracts::default());
        assert_eq!(rows[0]["null"]["verdict"], "Unknown");
        assert_eq!(rows[0]["null"]["checks"].as_array().unwrap().len(), 2);
        assert_eq!(
            section["travel"]["clause_checks_added_by_outcome"]["candidate"],
            1
        );
        assert_eq!(section["verdict_moves"]["Safe -> Unknown"], 1);
    }

    #[test]
    fn an_ambiguous_definition_exports_nothing() {
        let mut rows = vec![row(
            "a.c",
            "f",
            json!([
                {"id": 0, "line": 3, "subject": "p", "outcome": "candidate", "sources": ["uncontracted-result:g"]},
            ]),
            json!([]),
            "Unknown",
        )];
        let g = json!({"function": "g", "external": true, "parameters": 0, "inferred": [], "returns_non_null": true, "returns_null_on_a_path": false, "dependencies": []});
        let evidence = vec![
            json!({"library": "l", "file": "a.c", "member_stores": [], "external_calls": [], "definitions": []}),
            json!({"library": "l", "file": "b.c", "member_stores": [], "external_calls": [], "definitions": [g.clone()]}),
            json!({"library": "l", "file": "c.c", "member_stores": [], "external_calls": [], "definitions": [g]}),
        ];
        let section = fold(&mut rows, &evidence, &BoundsContracts::default());
        assert_eq!(rows[0]["null"]["verdict"], "Unknown");
        assert_eq!(section["travel"]["ambiguous_external_definitions"], 1);
    }
}
