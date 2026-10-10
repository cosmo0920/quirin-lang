//! Card **NULL-MEMBER-INVARIANT-SCOPE** (gateway rules 14x–15a): the run
//! fold's part for stable members and function-scope entries, called from
//! [`crate::null_fold::fold`].
//!
//! * Each **stable member** (`null_member_stable(TAG, FIELD)`) is checked
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
}

/// The environment's member, stable and scope declarations.
#[must_use]
pub fn declarations(environment: &BoundsContracts) -> Declarations {
    let mut out = Declarations::default();
    for (name, contract) in environment.null_declarations() {
        for entry in &contract.clauses {
            match &entry.clause {
                BoundsClause::NullMember { tag, field } => {
                    out.members
                        .insert(format!("{tag}.{field}"), (name.clone(), contract.held()));
                }
                BoundsClause::NullStable { tag, field } => {
                    out.stable
                        .insert(format!("{tag}.{field}"), (name.clone(), contract.held()));
                }
                BoundsClause::NullScope { .. } => {
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
pub fn tally(evidence: &[Value], key: &str, withdrawn: &BTreeSet<String>) -> BTreeMap<String, Tally> {
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
        stable_section.insert(
            member.clone(),
            json!({
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
            }),
        );
    }
    let mut scope_section = serde_json::Map::new();
    for (function, (entries, held)) in &declarations.scopes {
        scope_section.insert(
            function.clone(),
            json!({
                "entries": entries,
                "standing": if *held { "held" } else { "in-force: every caller's actual is a call-precondition check" },
            }),
        );
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
            "module contract;\nextern fn make(k: i32) -> ptr<mut, u8>\ncontract {\n    requires bounds_contract_v1;\n    requires null_member_stable(S, next);\n    requires null_member_nonnull(T, owner);\n};\n".to_owned(),
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
        let withdrawn = withdrawals(std::slice::from_ref(&fresh), &declarations, &BTreeSet::new());
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
}
