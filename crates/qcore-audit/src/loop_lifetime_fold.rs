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

//! S3 post-analysis demotion. Body summaries carry exact outgoing effects;
//! this fold cannot promote a row or treat absent run evidence as discharge.
use qcore_c_gateway::{
    loop_lifetime_summary::{self, Standing},
    member_run_facts::RunMemberFacts,
};
use qcore_schemas::contract::ResourceContracts;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) fn fold(
    rows: &mut [Value],
    contracts: &ResourceContracts,
    run: Option<&RunMemberFacts>,
    diagnostic_counts: &mut BTreeMap<String, usize>,
) -> (Value, BTreeSet<String>) {
    let active = contracts.lifetime_declarations().any(|(_, d)| {
        d.lifetime.as_ref().is_some_and(|l| {
            !l.registered.is_empty()
                || l.blocks.iter().any(|b| {
                    b.clauses.iter().any(|c| {
                        matches!(c, qcore_schemas::loop_lifetime::LocalClause::Registered(_))
                    })
                })
        })
    });
    if !active {
        return (Value::Null, BTreeSet::new());
    }
    let evidence = run.map(|r| loop_lifetime_summary::run_evidence(r, contracts));
    let withdrawn = evidence
        .as_ref()
        .map(|e| {
            e.protocols
                .iter()
                .filter(|p| matches!(p.standing, Standing::Withdrawn { .. }))
                .map(|p| p.host.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let known_roots = evidence
        .as_ref()
        .map(|e| {
            e.summaries
                .iter()
                .flat_map(|(unit, summaries)| {
                    summaries.keys().map(move |name| format!("{unit}:{name}"))
                })
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let mut demoted = Vec::new();
    let mut keys = BTreeSet::new();
    for row in rows {
        let library = row["library"].as_str().unwrap_or("").to_owned();
        let file = row["file"].as_str().unwrap_or("").to_owned();
        let mut count = 0usize;
        if let Some(functions) = row["functions"].as_array_mut() {
            for function in functions {
                let deps = &function["contract_dependencies"];
                let roots = deps[crate::contract_dependencies::LIFETIME_ROOTS_KEY]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>();
                if roots.is_empty() {
                    continue;
                }
                let missing_or_refused = match &evidence {
                    None => true,
                    Some(e) => roots
                        .iter()
                        .any(|root| e.refused.contains(*root) || !known_roots.contains(*root)),
                };
                let contradicted = deps[crate::contract_dependencies::LIFETIME_EFFECTS_KEY]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|effect| {
                        effect["host"]
                            .as_str()
                            .is_some_and(|host| withdrawn.contains(host))
                    });
                if !(missing_or_refused || contradicted)
                    || !matches!(function["status"].as_str(), Some("Safe" | "Pending"))
                {
                    continue;
                }
                function["status"] = json!("Unknown");
                let name = function["function"].as_str().unwrap_or("").to_owned();
                if let Some(ds) = function["diagnostics"].as_array_mut() {
                    ds.push(json!({"line":0,"origin":Value::Null,"category":"unsupported","description":"lifetime-precondition-undischarged: run root, caller effect, retained home or protocol standing is unresolved"}));
                }
                count += 1;
                if let Some(scope) = function["lifetime_scope"].as_str() {
                    keys.insert(scope.to_owned());
                }
                demoted.push(json!({"library":library,"file":file,"function":name}));
            }
        }
        if count > 0 {
            *diagnostic_counts.entry("unsupported".into()).or_default() += count;
            let old = row["diagnostic_counts"]["unsupported"]
                .as_u64()
                .unwrap_or(0);
            row["diagnostic_counts"]["unsupported"] = json!(old + count as u64);
            if matches!(row["status"].as_str(), Some("Safe" | "Pending")) {
                row["status"] = json!("Unknown");
            }
        }
    }
    (json!({"evidence": evidence,"demoted":demoted}), keys)
}

/// Each property row carries its body's exact source/input/configuration scope.
/// Duplicate occurrences remain distinct rows; unrelated configurations never
/// inherit a demotion through a (library,file,function) name-only key.
pub(crate) fn demote_properties(
    null: &mut [Value],
    bounds: &mut [Value],
    scopes: &BTreeSet<String>,
) {
    for row in null {
        if row["lifetime_scope"]
            .as_str()
            .is_some_and(|scope| scopes.contains(scope))
        {
            let key = (
                row["library"].as_str().unwrap_or("").to_owned(),
                row["file"].as_str().unwrap_or("").to_owned(),
                row["function"].as_str().unwrap_or("").to_owned(),
            );
            crate::callback_member_fold::demote_property_rows(
                std::slice::from_mut(row),
                &mut [],
                &BTreeSet::from([key]),
            );
        }
    }
    for row in bounds {
        if row["lifetime_scope"]
            .as_str()
            .is_some_and(|scope| scopes.contains(scope))
        {
            let key = (
                row["library"].as_str().unwrap_or("").to_owned(),
                row["file"].as_str().unwrap_or("").to_owned(),
                row["function"].as_str().unwrap_or("").to_owned(),
            );
            crate::callback_member_fold::demote_property_rows(
                &mut [],
                std::slice::from_mut(row),
                &BTreeSet::from([key]),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconciliation_preserves_configurations_and_duplicate_occurrences() {
        let row = |scope: &str| json!({"library":"fb","file":"same.c","function":"f","lifetime_scope":scope,"custody":"Safe","null":{"verdict":"Safe"},"bounds":{"verdict":"Unsafe"},"combined":"Safe"});
        let mut null = vec![
            row("source:config-a:f"),
            row("source:config-b:f"),
            row("source:config-a:f"),
        ];
        let mut bounds = null.clone();
        demote_properties(
            &mut null,
            &mut bounds,
            &BTreeSet::from(["source:config-a:f".into()]),
        );
        assert_eq!(
            null.iter()
                .map(|r| r["custody"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["Unknown", "Safe", "Unknown"]
        );
        assert_eq!(null[0]["combined"], "Unknown");
        assert_eq!(bounds[0]["combined"], "Unsafe");
        assert_eq!(bounds[1]["custody"], "Safe");
    }
}
