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

//! Card **NULL-MEMBER-INVARIANT-SCOPE** (gateway rules 14x–14z): the run
//! fold's part for stable members and function-scope entries, called from
//! [`crate::null_fold::fold`].
//!
//! * Each **stable member** (`nonnull_member(TAG, FIELD) where quant #scope [[stable]]`) is checked
//!   against every store the analysed units made to it: an assignment or a
//!   whole-object write of an existing object is examined (one not proved
//!   withdraws it); an initializer list writes a fresh object and is counted
//!   apart, exempt. Its standing is
//!   [`qcore_c_gateway::null_deref::null_member_scope::stable_standing`].
//! * **Withdrawal cascades.** A store whose proof rests on a declaration the
//!   run withdraws (a stable member, a declared member, a declared result) is
//!   a store not proved, for rule 14t's declared members as well; the fold
//!   iterates to the least set closed under that ([`withdrawals`]).
//! * **Scope entries** are listed with their hold; their obligations are the
//!   callers' checks in the rows.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::null_deref::null_member_scope::stable_standing;
use qcore_c_gateway::null_deref::null_triage::member_standing;
use qcore_contract::bounds_contract::{BoundsClause, BoundsContracts};
use serde_json::{Value, json};

/// The declarations the fold reads, by name.
#[derive(Default)]
pub struct Declarations {
    /// `TAG.FIELD` -> (host, held), rule 14t's declared members.
    pub members: BTreeMap<String, (String, bool)>,
    /// `TAG.FIELD` -> (host, held), the stable members.
    pub stable: BTreeMap<String, (String, bool)>,
    /// function -> (entries as written, held).
    pub scopes: BTreeMap<String, (Vec<String>, bool)>,
    /// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the named premises of each
    /// premised member fact, by its row dependency (`stable:TAG.FIELD`,
    /// `member:TAG.FIELD` since card NONNULL-MEMBER-UNIFIED-QUALIFIER, or a
    /// scope declaration's function name).
    pub premises: BTreeMap<String, Vec<(String, String)>>,
}

/// The environment's member, stable and scope declarations.
#[must_use]
pub fn declarations(environment: &BoundsContracts) -> Declarations {
    let mut out = Declarations {
        premises: qcore_c_gateway::null_deref::member_fact_premises(environment),
        ..Declarations::default()
    };
    for (name, contract) in environment.null_declarations() {
        for entry in &contract.clauses {
            match &entry.clause {
                BoundsClause::NonnullMember { tag, field } => {
                    out.members
                        .insert(format!("{tag}.{field}"), (name.clone(), contract.held()));
                }
                BoundsClause::NonnullMemberStable { tag, field } => {
                    out.stable
                        .insert(format!("{tag}.{field}"), (name.clone(), contract.held()));
                }
                BoundsClause::NonnullMemberScope { .. } => {
                    let scope = out
                        .scopes
                        .entry(name.clone())
                        .or_insert_with(|| (Vec::new(), contract.held()));
                    scope.0.push(entry.text.clone());
                }
                _ => {}
            }
        }
    }
    out
}

/// One declaration's stores, as the fold counts them.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    /// Stores to existing objects (every member store; a stable member's
    /// assignments and whole-object writes).
    pub examined: usize,
    /// Examined stores not proved non-null, or proved only by a withdrawn
    /// declaration.
    pub failed: usize,
    /// A stable member's initializer lists (fresh objects).
    pub fresh: usize,
    /// A stable member's whole-object writes.
    pub whole_object: usize,
    /// Up to 25 failures, for the reader.
    pub failures: Vec<Value>,
}

fn rests_on(store: &Value, withdrawn: &BTreeSet<String>) -> bool {
    store["relies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|d| withdrawn.contains(d))
}

/// Count the stores of `key` (`member_stores` or `stable_stores`) under the
/// withdrawn set.
#[must_use]
pub fn tally(
    evidence: &[Value],
    key: &str,
    withdrawn: &BTreeSet<String>,
) -> BTreeMap<String, Tally> {
    let mut out: BTreeMap<String, Tally> = BTreeMap::new();
    for unit in evidence {
        for store in unit[key].as_array().into_iter().flatten() {
            let member = store["member"].as_str().unwrap_or("").to_owned();
            let entry = out.entry(member).or_default();
            let kind = store["kind"].as_str().unwrap_or("assignment");
            if kind == "initializer" && key == "stable_stores" {
                entry.fresh += 1;
                continue;
            }
            if kind == "whole-object" {
                entry.whole_object += 1;
            }
            entry.examined += 1;
            let outcome = store["outcome"].as_str().unwrap_or("");
            let relied = rests_on(store, withdrawn);
            if !matches!(outcome, "proved" | "moved" | "excluded") || relied {
                entry.failed += 1;
                if entry.failures.len() < 25 {
                    let mut failure = json!({
                        "library": unit["library"],
                        "file": unit["file"],
                        "function": store["function"],
                        "line": store["line"],
                        "origin": store["origin"],
                        "outcome": if relied { "proved-by-a-withdrawn-declaration" } else { outcome },
                        "sources": store["sources"],
                    });
                    if key == "stable_stores" {
                        failure["kind"] = json!(kind);
                    }
                    entry.failures.push(failure);
                }
            }
        }
    }
    out
}

/// The least withdrawn set containing `base` (the refuted results) and
/// closed under: a declared or stable member with an examined store not
/// proved (or proved only by a withdrawn declaration) is withdrawn.
#[must_use]
pub fn withdrawals(
    evidence: &[Value],
    declarations: &Declarations,
    base: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut withdrawn = base.clone();
    loop {
        let mut next = base.clone();
        let members = tally(evidence, "member_stores", &withdrawn);
        for (member, (_, held)) in &declarations.members {
            let (examined, failed) = members
                .get(member)
                .map_or((0, 0), |t| (t.examined, t.failed));
            if !held && member_standing(examined, failed) == "withdrawn" {
                next.insert(format!("member:{member}"));
            }
        }
        let stable = tally(evidence, "stable_stores", &withdrawn);
        for (member, (_, held)) in &declarations.stable {
            let (examined, failed) = stable
                .get(member)
                .map_or((0, 0), |t| (t.examined, t.failed));
            if !held && stable_standing(examined, failed) == "withdrawn" {
                next.insert(format!("stable:{member}"));
            }
        }
        if next == withdrawn {
            return withdrawn;
        }
        withdrawn = next;
    }
}

/// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: a declaration record's named
/// premises (`premises`, absent when it states none, so an unpremised
/// record keeps its bytes). Card NONNULL-MEMBER-UNIFIED-QUALIFIER: rule 14t's
/// `member_declarations` records list them too.
pub(crate) fn premise_list(record: &mut Value, premises: Option<&Vec<(String, String)>>) {
    if let Some(premises) = premises.filter(|p| !p.is_empty()) {
        record["premises"] = json!(
            premises
                .iter()
                .map(|(name, statement)| json!({"name": name, "statement": statement}))
                .collect::<Vec<_>>()
        );
    }
}

/// The fold's `stable_declarations` and `scope_declarations` sections.
#[must_use]
pub fn sections(
    evidence: &[Value],
    declarations: &Declarations,
    withdrawn: &BTreeSet<String>,
) -> (Value, Value) {
    let stable = tally(evidence, "stable_stores", withdrawn);
    let mut stable_section = serde_json::Map::new();
    for (member, (host, held)) in &declarations.stable {
        let t = stable.get(member).cloned().unwrap_or_default();
        let standing = if *held {
            "held"
        } else {
            stable_standing(t.examined, t.failed)
        };
        let mut record = json!({
            "host": host,
            "standing": standing,
            "clauses": {
                "stores": if *held { "held" } else if t.failed > 0 { "withdrawn" } else if t.examined == 0 { "unexamined" } else { "validated" },
                "fresh-objects": "exempt: an initializer list writes an object no use held before it",
                "whole-object": "declared-assumption",
            },
            "stores_examined": t.examined,
            "stores_not_proved": t.failed,
            "fresh_object_stores": t.fresh,
            "whole_object_stores": t.whole_object,
            "failures": t.failures,
        });
        premise_list(
            &mut record,
            declarations.premises.get(&format!("stable:{member}")),
        );
        stable_section.insert(member.clone(), record);
    }
    let mut scope_section = serde_json::Map::new();
    for (function, (entries, held)) in &declarations.scopes {
        let mut record = json!({
            "entries": entries,
            "standing": if *held { "held" } else { "in-force: every caller's actual is a call-precondition check" },
        });
        premise_list(&mut record, declarations.premises.get(function));
        scope_section.insert(function.clone(), record);
    }
    (Value::Object(stable_section), Value::Object(scope_section))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_contract::bounds_contract::load_bounds_contract_sources;

    fn environment() -> BoundsContracts {
        load_bounds_contract_sources(&BTreeMap::from([(
            "m.qv".to_owned(),
            "module contract;\nextern fn make(k: i32) -> ptr<mut, u8>\ncontract {\n    requires bounds_contract_v1;\n    requires nonnull_member(S, next) where quant #scope [[stable]];\n    requires nonnull_member(T, owner);\n};\n".to_owned(),
        )]))
        .expect("loads")
    }

    fn row(checks: Value, deps: Value) -> Value {
        json!({"library": "l", "file": "a.c", "function": "f", "custody": "Unknown",
               "null": {"function": "f", "verdict": "Safe", "reasons": [],
                        "scope": "unconditional", "checks": checks,
                        "contract_dependencies": deps},
               "combined": "Unknown"})
    }

    #[test]
    fn an_initializer_is_exempt_and_an_unproved_store_withdraws() {
        let fresh = json!({"library": "l", "file": "a.c", "member_stores": [], "external_calls": [], "definitions": [],
            "stable_stores": [{"member": "S.next", "function": "g", "line": 1, "kind": "initializer", "outcome": "violation"},
                              {"member": "S.next", "function": "g", "line": 2, "kind": "assignment", "outcome": "proved"}]});
        let declarations = declarations(&environment());
        let withdrawn = withdrawals(
            std::slice::from_ref(&fresh),
            &declarations,
            &BTreeSet::new(),
        );
        assert!(withdrawn.is_empty(), "{withdrawn:?}");
        let (stable, _) = sections(&[fresh], &declarations, &withdrawn);
        assert_eq!(stable["S.next"]["standing"], "partly-validated");
        assert_eq!(stable["S.next"]["fresh_object_stores"], 1);
        let unproved = json!({"library": "l", "file": "a.c", "member_stores": [], "external_calls": [], "definitions": [],
            "stable_stores": [{"member": "S.next", "function": "g", "line": 2, "kind": "assignment", "outcome": "candidate"}]});
        let withdrawn = withdrawals(&[unproved], &declarations, &BTreeSet::new());
        assert!(withdrawn.contains("stable:S.next"));
    }

    #[test]
    fn withdrawal_cascades_through_a_store_proved_by_a_withdrawn_declaration() {
        // T.owner's only store is proved by a fact the withdrawn stable
        // member kept: it fails, and T.owner is withdrawn with it.
        let unit = json!({"library": "l", "file": "a.c", "external_calls": [], "definitions": [],
            "stable_stores": [{"member": "S.next", "function": "g", "line": 2, "kind": "whole-object", "outcome": "candidate"}],
            "member_stores": [{"member": "T.owner", "function": "h", "line": 3, "outcome": "proved", "relies": ["stable:S.next"]}]});
        let declarations = declarations(&environment());
        let withdrawn = withdrawals(std::slice::from_ref(&unit), &declarations, &BTreeSet::new());
        assert!(withdrawn.contains("stable:S.next"));
        assert!(withdrawn.contains("member:T.owner"), "{withdrawn:?}");
        // The fold demotes a row that rested on the stable member.
        let mut rows = vec![row(
            json!([{"id": 0, "line": 4, "subject": "s->next", "outcome": "moved", "proofs": ["moved:0", "result:stable:S.next"]}]),
            json!(["stable:S.next"]),
        )];
        let section = crate::null_fold::fold(&mut rows, &[unit], &environment());
        assert_eq!(rows[0]["null"]["verdict"], "Unknown");
        assert_eq!(rows[0]["null"]["checks"][0]["outcome"], "candidate");
        assert_eq!(
            section["stable_declarations"]["S.next"]["standing"],
            "withdrawn"
        );
        assert_eq!(
            section["member_declarations"]["T.owner"]["standing"],
            "withdrawn"
        );
    }

    /// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: a premised stable member's
    /// record lists its premise, a row whose dependencies travel adds to
    /// lists it as the analysis would, and the premise changes no verdict.
    #[test]
    fn a_premised_member_fact_is_listed_on_the_record_and_on_a_travelled_row() {
        let premised = load_bounds_contract_sources(&BTreeMap::from([(
            "m.qv".to_owned(),
            "module contract;\nextern fn make(k: i32) -> ptr<mut, u8>\ncontract {\n    requires bounds_contract_v1;\n    requires nonnull_member(S, next) where quant #scope [[stable]];\n    requires resource_premise(no_plugin, \"no code outside the run stores S.next\");\n};\n".to_owned(),
        )]))
        .expect("loads");
        let caller = json!({"library": "l", "file": "a.c", "member_stores": [], "stable_stores": [],
            "external_calls": [], "definitions": []});
        let home = json!({"library": "l", "file": "b.c", "member_stores": [], "stable_stores": [],
            "external_calls": [], "definitions": [{"function": "mk", "external": true, "parameters": 0,
                "inferred": [], "returns_non_null": true, "returns_null_on_a_path": false,
                "dependencies": ["stable:S.next"]}]});
        let travelled = || {
            vec![row(
                json!([{"id": 0, "line": 4, "subject": "p", "outcome": "candidate", "sources": ["uncontracted-result:mk"]}]),
                json!([]),
            )]
        };
        let mut rows = travelled();
        let section = crate::null_fold::fold(&mut rows, &[caller.clone(), home.clone()], &premised);
        assert_eq!(rows[0]["null"]["checks"][0]["outcome"], "proved");
        assert_eq!(rows[0]["null"]["premises"][0]["name"], "no_plugin");
        assert_eq!(
            rows[0]["null"]["premises"][0]["through"],
            json!(["stable:S.next"])
        );
        assert!(
            rows[0]["null"]["contract_dependencies"]
                .as_array()
                .unwrap()
                .contains(&json!("premise:no_plugin"))
        );
        assert_eq!(
            section["stable_declarations"]["S.next"]["premises"][0]["name"],
            "no_plugin"
        );
        // Without the premise: the same verdict, no premise listed.
        let mut plain = travelled();
        let plain_section = crate::null_fold::fold(&mut plain, &[caller, home], &environment());
        assert_eq!(plain[0]["null"]["verdict"], rows[0]["null"]["verdict"]);
        assert!(plain[0]["null"].get("premises").is_none());
        assert!(
            plain_section["stable_declarations"]["S.next"]
                .get("premises")
                .is_none()
        );
    }
    /// Card NONNULL-MEMBER-UNIFIED-QUALIFIER (USER DECISION 2026-10-16, "Yes,
    /// all kinds"): a premised plain member's `member_declarations` record
    /// lists its premise, and so does a row whose travelled dependencies name
    /// `member:TAG.FIELD`; the verdict is the unpremised one.
    #[test]
    fn a_premised_plain_member_is_listed_on_the_record_and_on_a_travelled_row() {
        let declaration = |premise: &str| {
            load_bounds_contract_sources(&BTreeMap::from([(
                "m.qv".to_owned(),
                format!("module contract;\nextern fn make(k: i32) -> ptr<mut, u8>\ncontract {{\n    requires bounds_contract_v1;\n    requires nonnull_member(T, owner);\n    {premise}\n}};\n"),
            )]))
            .expect("loads")
        };
        let premised = declaration(
            "requires resource_premise(no_plugin, \"no code outside the run stores T.owner\");",
        );
        let unpremised = declaration("");
        let caller = json!({"library": "l", "file": "a.c", "member_stores": [], "stable_stores": [],
            "external_calls": [], "definitions": []});
        let home = json!({"library": "l", "file": "b.c", "member_stores": [], "stable_stores": [],
            "external_calls": [], "definitions": [{"function": "mk", "external": true, "parameters": 0,
                "inferred": [], "returns_non_null": true, "returns_null_on_a_path": false,
                "dependencies": ["member:T.owner"]}]});
        let travelled = || {
            vec![row(
                json!([{"id": 0, "line": 4, "subject": "p", "outcome": "candidate", "sources": ["uncontracted-result:mk"]}]),
                json!([]),
            )]
        };
        let mut rows = travelled();
        let section = crate::null_fold::fold(&mut rows, &[caller.clone(), home.clone()], &premised);
        assert_eq!(rows[0]["null"]["checks"][0]["outcome"], "proved");
        assert_eq!(rows[0]["null"]["premises"][0]["name"], "no_plugin");
        assert_eq!(
            rows[0]["null"]["premises"][0]["through"],
            json!(["member:T.owner"])
        );
        assert!(
            rows[0]["null"]["contract_dependencies"]
                .as_array()
                .unwrap()
                .contains(&json!("premise:no_plugin"))
        );
        assert_eq!(
            section["member_declarations"]["T.owner"]["premises"][0]["name"],
            "no_plugin"
        );
        let mut plain = travelled();
        let plain_section = crate::null_fold::fold(&mut plain, &[caller, home], &unpremised);
        assert_eq!(plain[0]["null"]["verdict"], rows[0]["null"]["verdict"]);
        assert!(plain[0]["null"].get("premises").is_none());
        assert!(
            plain_section["member_declarations"]["T.owner"]
                .get("premises")
                .is_none()
        );
    }
}
