//! The `pending_human_review` report section and its console tally (card
//! **W45-CONTRACT-DEFERRED-REVIEW**, contract spec §C06).
//!
//! A declared contract, or one clause of it, marked
//! `resource_review_pending` is withheld by the contract loader, and every
//! call site that would have applied it is held under the reason code
//! `awaiting-human-review`: a function whose only unresolved diagnostics are
//! held calls is `Pending` (card W45-USER-CONTRACT-PLUGINS; `Unknown` before). This module makes the pending reviews
//! visible and countable: one entry per marker, with its declaration, the
//! module it is written in and that module's retained-text digest, its reason
//! and owner, and how many analysed functions and call sites it holds.
//!
//! Inventory only: it reads a finished report and the environment and never
//! changes a verdict. A held call is recognised by the exact refusal the
//! gateway writes (`qcore_c_gateway::legacy_c::held_call`), never by a
//! looser match, so no other refusal can be counted as a held call. Every run
//! publishes the section, with explicit zeros when nothing is pending, so a
//! zero is a statement rather than an absence.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::legacy_c::{AWAITING_HUMAN_REVIEW, held_call};
use qcore_schemas::contract::{ResourceContracts, ReviewScope};
use qcore_schemas::sha256_hex;
use serde_json::{Value, json};

use crate::report::AuditReport;

/// What the section means, carried in the section itself.
pub const MEANING: &str = "declared contracts, or clauses of them, marked `resource_review_pending` (contract spec C06): each is loaded and validated but withheld, discharges nothing and accuses nothing, and every call site that would apply it is held with reason code `awaiting-human-review`, and a function whose only unresolved diagnostics are held calls is Pending, never Safe; a clause-level marker holds exactly the call sites a contract-level one does, because a contract is applied as one conjunction; `held_functions` counts analysed functions with at least one recorded held call of the declaration, and `held_call_sites` counts the held-call refusals the analysis recorded; the analysis of a path stops at its first refused statement, so a later call site on the same path records no refusal of its own and the count is a lower bound on textual call sites";

/// The `pending_human_review` section for one finished report.
#[must_use]
pub fn section(contracts: &ResourceContracts, report: &AuditReport) -> Value {
    // (callee) -> held call sites, and the distinct (library, file, function)
    // rows holding at least one of them.
    let mut call_sites = BTreeMap::<String, usize>::new();
    let mut functions = BTreeMap::<String, BTreeSet<(String, String, String)>>::new();
    for unit in report.unit_rows() {
        let library = unit["library"].as_str().unwrap_or_default();
        let file = unit["file"].as_str().unwrap_or_default();
        for function in unit["functions"].as_array().into_iter().flatten() {
            let name = function["function"].as_str().unwrap_or_default();
            for diagnostic in function["diagnostics"].as_array().into_iter().flatten() {
                let Some(callee) = diagnostic["description"].as_str().and_then(held_call) else {
                    continue;
                };
                *call_sites.entry(callee.to_owned()).or_default() += 1;
                functions.entry(callee.to_owned()).or_default().insert((
                    library.to_owned(),
                    file.to_owned(),
                    name.to_owned(),
                ));
            }
        }
    }
    let mut entries = Vec::new();
    let mut markers = 0usize;
    for (function, pending) in contracts.pending_review() {
        let digest = contracts
            .source_assumptions()
            .get(&pending.module)
            .map(|text| sha256_hex(text.as_bytes()));
        let held_functions = functions.get(function).map_or(0, BTreeSet::len);
        let held_call_sites = call_sites.get(function).copied().unwrap_or(0);
        for deferral in &pending.deferrals {
            markers += 1;
            let (scope, clause) = match &deferral.scope {
                ReviewScope::Contract => ("contract", Value::Null),
                ReviewScope::Clause { clause } => ("clause", json!(clause)),
            };
            entries.push(json!({
                "function": function,
                "scope": scope,
                "clause": clause,
                "reason": deferral.reason,
                "owner": deferral.owner,
                "module": pending.module,
                "module_sha256": digest,
                "held_functions": held_functions,
                "held_call_sites": held_call_sites,
            }));
        }
    }
    let held_functions = functions.values().flatten().collect::<BTreeSet<_>>().len();
    json!({
        "meaning": MEANING,
        "reason_code": AWAITING_HUMAN_REVIEW,
        "declarations": contracts.pending_review().len(),
        "markers": markers,
        "held_functions": held_functions,
        "held_call_sites": call_sites.values().sum::<usize>(),
        "entries": entries,
    })
}

/// The one-line console tally of a [`section`].
#[must_use]
pub fn tally(section: &Value) -> String {
    let count = |key: &str| section[key].as_u64().unwrap_or_default();
    format!(
        "pending human review: {} declaration(s), {} marker(s); {} function(s) and {} call \
         site(s) held Pending ({AWAITING_HUMAN_REVIEW})",
        count("declarations"),
        count("markers"),
        count("held_functions"),
        count("held_call_sites"),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use qcore_c_gateway::legacy_c::{
        C11TranslationUnit, UnrestrictedLegacyResult, analyze_unrestricted_c11, hold_description,
    };
    use qcore_contract::contract_loader::load_resource_contract_sources;
    use serde_json::json;

    use super::{section, tally};
    use crate::report::AuditReport;

    const CONTRACTS: &str = "module contract;

extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_review_pending(resource_released(h, nonnull), \"why\", \"cosmo\");
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
";

    #[test]
    fn an_empty_environment_publishes_explicit_zeros() {
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "k.qcore".to_owned(),
            CONTRACTS.replace(
                "resource_review_pending(resource_released(h, nonnull), \"why\", \"cosmo\")",
                "resource_released(h, nonnull)",
            ),
        )]))
        .unwrap();
        let value = section(&contracts, &AuditReport::new("r"));
        assert_eq!(value["declarations"], 0);
        assert_eq!(value["markers"], 0);
        assert_eq!(value["held_call_sites"], 0);
        assert_eq!(value["entries"], json!([]));
        assert_eq!(
            tally(&value),
            "pending human review: 0 declaration(s), 0 marker(s); 0 function(s) and 0 call \
             site(s) held Pending (awaiting-human-review)"
        );
    }

    #[test]
    fn every_marker_is_listed_with_its_digest_and_what_it_holds() {
        if qcore_c_gateway::clang_command().is_err() {
            eprintln!("skipping Clang-dependent pending-review report test");
            return;
        }
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "k.qcore".to_owned(),
            CONTRACTS.to_owned(),
        )]))
        .unwrap();
        let source = "void *k_open(void);\nvoid k_close(void *h, int n);\n\
                      void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); \
                      k_close(h, 1); }\n\
                      void v(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); }";
        let result = analyze_unrestricted_c11(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "w",
            &contracts,
        );
        assert!(matches!(
            result,
            Ok(UnrestrictedLegacyResult::Unknown { .. })
        ));
        let mut report = AuditReport::new("r");
        report.record("k", "k.c", &["v".into(), "w".into()], &result, None);
        let value = section(&contracts, &report);
        assert_eq!(value["declarations"], 1);
        assert_eq!(value["markers"], 1);
        assert_eq!(value["held_functions"], 2);
        // `w`'s second call to `k_close` follows its first refused statement,
        // where the analysis of that path stops, so it records no refusal of
        // its own: the count is of recorded refusals, as the meaning says.
        assert_eq!(value["held_call_sites"], 2);
        let entry = &value["entries"][0];
        assert_eq!(entry["function"], "k_close");
        assert_eq!(entry["scope"], "clause");
        assert_eq!(
            entry["clause"],
            "ensures normal resource_released(h, nonnull)"
        );
        assert_eq!(entry["reason"], "why");
        assert_eq!(entry["owner"], "cosmo");
        assert_eq!(entry["module"], "k.qcore");
        assert_eq!(
            entry["module_sha256"],
            qcore_schemas::sha256_hex(CONTRACTS.as_bytes())
        );
        assert_eq!(entry["held_call_sites"], 2);
        // The unit row carries the dedicated reason code where unit reasons
        // are recorded.
        let rows = report.unit_rows();
        assert_eq!(rows[0]["reason_code"], "awaiting-human-review");
        assert!(
            rows[0]["functions"][1]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["description"] == hold_description("k_close"))
        );
    }
}
