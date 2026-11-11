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

//! Card **NULL-SAFETY-PROPERTY** (gateway rules 14k–14p): the tally written
//! beside the null selection's rows (`QCORE_NULL_PROPERTY`). Evidence only:
//! it reads the rows the run recorded and decides nothing.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// What the summary means, carried in it.
pub const MEANING: &str = "the null-dereference property (c.null.deref.v1) of every projected function, reported separately from custody: `null` counts the null rows' verdicts (Safe split into unconditional and conditional on the function's own declared or inferred null-deref preconditions); `combined` joins the custody and null rows of a request naming both, a missing row counting Unknown; `checks` counts every judged dereference by outcome, and every candidate by source kind and by library; `custody_null_premises` counts the named null premises custody rows rest on (rule 14n) by what the null row says of each (`proved` when every check of the variable at that line is proved or moved to a proved check; `nearby:` when only checks within three lines name it, never counted as proved) and the custody Safe rows whose every own premise the null property proves. Unknown and Pending stay in every denominator";

fn bump(map: &mut BTreeMap<String, usize>, key: &str) {
    *map.entry(key.to_owned()).or_default() += 1;
}

/// The summary of the recorded null rows.
#[must_use]
pub fn summary(rows: &[Value]) -> Value {
    let mut verdicts = BTreeMap::new();
    let mut custody = BTreeMap::new();
    let mut combined = BTreeMap::new();
    let mut outcomes = BTreeMap::new();
    let mut sources = BTreeMap::new();
    let mut libraries = BTreeMap::new();
    let mut reasons = BTreeMap::new();
    let mut premises = BTreeMap::new();
    let mut inferred = 0usize;
    let mut inferred_functions = 0usize;
    let mut clause_standing = BTreeMap::new();
    let mut safe_with_premise = 0usize;
    let mut safe_premises_proved = 0usize;
    let mut candidate_functions = 0usize;
    let mut indirect_calls = 0u64;
    let mut indirect_functions = 0usize;
    for row in rows {
        let verdict = row["null"]["verdict"].as_str().unwrap_or("absent");
        let key = if verdict == "Safe" {
            if row["null"]["scope"] == "unconditional" {
                "Safe (unconditional)"
            } else {
                "Safe (conditional on its preconditions)"
            }
        } else {
            verdict
        };
        bump(&mut verdicts, key);
        bump(&mut custody, row["custody"].as_str().unwrap_or("absent"));
        bump(&mut combined, row["combined"].as_str().unwrap_or("absent"));
        let library = row["library"].as_str().unwrap_or("?");
        let mut has_candidate = false;
        for check in row["null"]["checks"].as_array().into_iter().flatten() {
            let outcome = check["outcome"].as_str().unwrap_or("?");
            bump(&mut outcomes, outcome);
            if outcome == "candidate" {
                has_candidate = true;
                bump(&mut libraries, library);
                for source in check["sources"].as_array().into_iter().flatten() {
                    let kind = source
                        .as_str()
                        .unwrap_or("?")
                        .split(':')
                        .next()
                        .unwrap_or("?");
                    bump(&mut sources, kind);
                }
            }
        }
        candidate_functions += usize::from(has_candidate);
        let indirect = row["null"]["indirect_calls"].as_u64().unwrap_or_default();
        indirect_calls += indirect;
        indirect_functions += usize::from(indirect > 0);
        let mut codes = std::collections::BTreeSet::new();
        for reason in row["null"]["reasons"].as_array().into_iter().flatten() {
            codes.insert(
                reason
                    .as_str()
                    .unwrap_or("")
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .to_owned(),
            );
        }
        for code in codes {
            bump(&mut reasons, &code);
        }
        let count = row["null"]["inferred_preconditions"]
            .as_array()
            .map_or(0, Vec::len);
        inferred += count;
        inferred_functions += usize::from(count > 0);
        for standing in row["null"]["clause_standing"]
            .as_array()
            .into_iter()
            .flatten()
        {
            bump(
                &mut clause_standing,
                standing["standing"].as_str().unwrap_or("?"),
            );
        }
        let own = row["custody_null_premises"].as_array();
        for premise in own.into_iter().flatten() {
            bump(
                &mut premises,
                premise["null_property"].as_str().unwrap_or("?"),
            );
        }
        if row["custody"] == "Safe"
            && let Some(own) = own
            && !own.is_empty()
        {
            safe_with_premise += 1;
            if own.iter().all(|p| p["null_property"] == "proved") {
                safe_premises_proved += 1;
            }
        }
    }
    json!({
        "schema_version": 1,
        "property": qcore_c_gateway::null_deref::NULL_PROPERTY,
        "meaning": MEANING,
        "functions": rows.len(),
        "null": verdicts,
        "custody": custody,
        "combined": combined,
        "functions_by_reason_code": reasons,
        "checks": {
            "by_outcome": outcomes,
            "candidates_by_source_kind": sources,
            "candidates_by_library": libraries,
            "functions_with_a_candidate": candidate_functions,
            "indirect_calls_not_inventoried": indirect_calls,
            "functions_with_an_indirect_call": indirect_functions,
        },
        "inference": {
            "inferred_preconditions": inferred,
            "functions_with_an_inferred_precondition": inferred_functions,
            "declared_clause_standing": clause_standing,
        },
        "custody_null_premises": {
            "by_null_standing": premises,
            "custody_safe_rows_with_an_own_premise": safe_with_premise,
            "of_which_every_premise_proved": safe_premises_proved,
        },
    })
}

/// The one-line console tally.
#[must_use]
pub fn line(summary: &Value) -> String {
    let null = &summary["null"];
    let count = |key: &str| null[key].as_u64().unwrap_or_default();
    format!(
        "null property: {} function(s): {} Safe unconditional, {} Safe conditional, {} Pending, {} Unknown, {} Unsafe; {} with a candidate; custody Safe rows with an own null premise {} ({} with every premise proved)",
        summary["functions"].as_u64().unwrap_or_default(),
        count("Safe (unconditional)"),
        count("Safe (conditional on its preconditions)"),
        count("Pending"),
        count("Unknown"),
        count("Unsafe"),
        summary["checks"]["functions_with_a_candidate"]
            .as_u64()
            .unwrap_or_default(),
        summary["custody_null_premises"]["custody_safe_rows_with_an_own_premise"]
            .as_u64()
            .unwrap_or_default(),
        summary["custody_null_premises"]["of_which_every_premise_proved"]
            .as_u64()
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tally_splits_safe_rows_and_counts_premises_conservatively() {
        let rows = vec![
            json!({"library": "l", "custody": "Safe", "combined": "Unknown",
                   "null": {"verdict": "Unknown", "scope": "unconditional", "reasons": ["null-deref-candidate"],
                            "checks": [{"outcome": "candidate", "sources": ["memory-field:s->next"]}]},
                   "custody_null_premises": [{"null_property": "candidate"}]}),
            json!({"library": "l", "custody": "Safe", "combined": "Safe",
                   "null": {"verdict": "Safe", "scope": "unconditional", "reasons": [],
                            "checks": [{"outcome": "proved"}]},
                   "custody_null_premises": [{"null_property": "proved"}]}),
            json!({"library": "l", "custody": "Safe", "combined": "Safe",
                   "null": {"verdict": "Safe", "scope": "conditional on f's null-deref preconditions: inferred quant null-deref #p",
                            "reasons": [], "checks": [], "inferred_preconditions": ["#p"]},
                   "custody_null_premises": [{"null_property": "nearby:proved"}]}),
        ];
        let summary = summary(&rows);
        assert_eq!(summary["null"]["Safe (unconditional)"], 1);
        assert_eq!(
            summary["null"]["Safe (conditional on its preconditions)"],
            1
        );
        assert_eq!(
            summary["checks"]["candidates_by_source_kind"]["memory-field"],
            1
        );
        assert_eq!(
            summary["custody_null_premises"]["custody_safe_rows_with_an_own_premise"],
            3
        );
        // A nearby proof is never counted as a discharged premise.
        assert_eq!(
            summary["custody_null_premises"]["of_which_every_premise_proved"],
            1
        );
        assert_eq!(summary["inference"]["inferred_preconditions"], 1);
        assert!(line(&summary).contains("1 Safe unconditional, 1 Safe conditional"));
    }
}
