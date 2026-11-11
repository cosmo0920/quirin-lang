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

//! The concrete reason of every review hold, on the row it holds (card
//! **W47-REVIEW-HOLD-CONCRETE-REASON**; gateway rule 10i as amended, contract
//! specification §C06.1).
//!
//! A call to a declaration awaiting human review refuses its statement with
//! the generic ``call to `NAME` whose declared contract awaits human review
//! (awaiting-human-review)``, and the declaration's own reasons were published
//! only in the run-wide `pending_human_review` section. A `Pending` row
//! claims that a named human act alone stands between it and `Safe`
//! (§L21.2), so the row itself names the act: this module adds one member,
//! `review_holds`, to every published function row that records at least one
//! held-call refusal. It lists, for every held declaration in name order,
//! each of its review markers in clause order: the declaration, its module,
//! the marker's scope and clause, the reason text verbatim, the owner, the
//! category ([`ReviewDisposition`]: whether the marker's author states that
//! the text must be rewritten before it can be accepted) and the reason code.
//!
//! Additive only: the member never changes a status, a diagnostic or a count,
//! and a row with no held call is byte-identical. The held callees are read
//! only from the exact refusal the gateway mints
//! (`qcore_c_gateway::legacy_c::held_call`), whose prefix no source can spell
//! (rule 10i). Fail-closed: a held callee the environment does not withhold,
//! or withholds with no marker, is an error, so a row is never published
//! holding a call whose reason is unknown.

use std::collections::BTreeSet;

use qcore_c_gateway::legacy_c::{AWAITING_HUMAN_REVIEW, held_call};
use qcore_schemas::contract::{ResourceContracts, ReviewScope};
use serde_json::{Value, json};

/// The function-row member this module adds.
pub const MEMBER: &str = "review_holds";

/// The held callees a function row records, in name order.
#[must_use]
pub fn held_callees(function_row: &Value) -> BTreeSet<String> {
    function_row["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|diagnostic| diagnostic["description"].as_str().and_then(held_call))
        .map(str::to_owned)
        .collect()
}

/// The `review_holds` value of one function row: `None` when it holds no
/// call, an error when a held callee has no withheld declaration or no
/// marker.
pub fn row_review_holds(
    function_row: &Value,
    contracts: &ResourceContracts,
) -> Result<Option<Value>, String> {
    let callees = held_callees(function_row);
    if callees.is_empty() {
        return Ok(None);
    }
    let mut entries = Vec::new();
    for callee in &callees {
        let pending = contracts.pending_review().get(callee).ok_or_else(|| {
            format!(
                "function `{}` holds a call to `{callee}` for review, but the contract \
                 environment withholds no declaration of that name: its reason is unknown",
                function_row["function"].as_str().unwrap_or_default()
            )
        })?;
        if pending.deferrals.is_empty() {
            return Err(format!(
                "the declaration of `{callee}` is withheld for review with no marker: its reason \
                 is unknown"
            ));
        }
        for deferral in &pending.deferrals {
            let (scope, clause) = match &deferral.scope {
                ReviewScope::Contract => ("contract", None),
                ReviewScope::Clause { clause } => ("clause", Some(clause.as_str())),
            };
            entries.push(json!({
                "declaration": callee,
                "module": pending.module,
                "scope": scope,
                "clause": clause,
                "reason": deferral.reason,
                "owner": deferral.owner,
                "category": deferral.disposition.as_str(),
                "reason_code": AWAITING_HUMAN_REVIEW,
            }));
        }
    }
    Ok(Some(Value::Array(entries)))
}

/// Add `review_holds` to every function row of one unit row that holds a
/// call. Nothing else in the row changes.
pub fn annotate_unit(unit_row: &mut Value, contracts: &ResourceContracts) -> Result<(), String> {
    let Some(functions) = unit_row["functions"].as_array_mut() else {
        return Ok(());
    };
    for function in functions {
        if let Some(holds) = row_review_holds(function, contracts)? {
            function[MEMBER] = holds;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use qcore_c_gateway::legacy_c::hold_description;
    use qcore_contract::contract_loader::load_resource_contract_sources;
    use serde_json::json;

    use super::{annotate_unit, row_review_holds};

    const CONTRACTS: &str = "module contract;

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_review_pending(resource_released(h, nonnull), \"frees h?\", \"cosmo\");
    ensures normal resource_review_pending(resource_borrowed(n), \"retained, not borrowed\", rewrite_before_acceptance);
    ensures normal resource_return_none();
};

extern fn a_flush(h: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_review_pending(\"is h flushed on every path?\");
    ensures normal resource_borrowed(h);
    ensures normal resource_return_none();
};
";

    fn contracts() -> qcore_schemas::contract::ResourceContracts {
        load_resource_contract_sources(&BTreeMap::from([(
            "k.qcore".to_owned(),
            CONTRACTS.to_owned(),
        )]))
        .unwrap()
    }

    fn held(callee: &str) -> serde_json::Value {
        json!({"category": "unsupported", "description": hold_description(callee), "line": 3})
    }

    #[test]
    fn two_holds_on_one_row_list_every_marker_in_a_deterministic_order() {
        // Recorded in the opposite order, and one callee twice.
        let row = json!({"function": "w", "status": "Pending",
            "diagnostics": [held("k_close"), held("a_flush"), held("k_close")]});
        let holds = row_review_holds(&row, &contracts()).unwrap().unwrap();
        let holds = holds.as_array().unwrap();
        assert_eq!(holds.len(), 3);
        assert_eq!(holds[0]["declaration"], "a_flush");
        assert_eq!(holds[0]["scope"], "contract");
        assert_eq!(holds[0]["clause"], serde_json::Value::Null);
        assert_eq!(holds[0]["reason"], "is h flushed on every path?");
        assert_eq!(holds[0]["category"], "acceptable-as-written-after-review");
        assert_eq!(holds[1]["declaration"], "k_close");
        assert_eq!(
            holds[1]["clause"],
            "ensures normal resource_released(h, nonnull)"
        );
        assert_eq!(holds[1]["reason"], "frees h?");
        assert_eq!(holds[1]["owner"], "cosmo");
        assert_eq!(holds[1]["category"], "acceptable-as-written-after-review");
        assert_eq!(holds[2]["clause"], "ensures normal resource_borrowed(n)");
        assert_eq!(holds[2]["reason"], "retained, not borrowed");
        assert_eq!(holds[2]["category"], "must-be-rewritten-before-acceptance");
        for hold in holds {
            assert_eq!(hold["module"], "k.qcore");
            assert_eq!(hold["reason_code"], "awaiting-human-review");
        }
    }

    #[test]
    fn a_row_without_a_held_call_is_unchanged() {
        let mut unit = json!({"functions": [
            {"function": "v", "status": "Unknown", "diagnostics": [
                {"category": "unsupported", "description": "call to uncontracted function `k_close`", "line": 1}]},
            {"function": "q", "status": "Safe", "diagnostics": []},
        ]});
        let before = unit.clone();
        annotate_unit(&mut unit, &contracts()).unwrap();
        assert_eq!(unit, before);
    }

    #[test]
    fn annotation_adds_one_member_and_changes_nothing_else() {
        let row = json!({"function": "w", "status": "Pending", "diagnostics": [held("a_flush")]});
        let mut unit = json!({"status": "Pending", "functions": [row.clone()]});
        annotate_unit(&mut unit, &contracts()).unwrap();
        let mut annotated = unit["functions"][0].clone();
        assert!(annotated["review_holds"].is_array());
        annotated.as_object_mut().unwrap().remove("review_holds");
        assert_eq!(annotated, row);
        assert_eq!(unit["status"], "Pending");
    }

    #[test]
    fn a_held_callee_the_environment_does_not_withhold_is_refused() {
        let row = json!({"function": "w", "status": "Pending", "diagnostics": [held("k_open")]});
        let error = row_review_holds(&row, &contracts()).unwrap_err();
        assert!(error.contains("its reason is unknown"), "{error}");
        let mut unit = json!({"functions": [row]});
        assert!(annotate_unit(&mut unit, &contracts()).is_err());
    }

    #[test]
    fn a_description_merely_resembling_a_hold_is_not_one() {
        let row = json!({"function": "w", "status": "Unknown", "diagnostics": [
            {"category": "unsupported",
             "description": format!("{} (and more)", hold_description("k_close")), "line": 1}]});
        assert_eq!(row_review_holds(&row, &contracts()).unwrap(), None);
    }
}
