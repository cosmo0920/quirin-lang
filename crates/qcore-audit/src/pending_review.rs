//! The `pending_human_review` report section and its console tally (card
//! **W45-CONTRACT-DEFERRED-REVIEW**, contract spec §C06).
//!
//! A declared contract, or one clause of it, marked
//! `resource_review_pending` is withheld by the contract loader, and every
//! call site that would have applied it is held `Unknown` under the reason
//! code `awaiting-human-review`. This module makes the pending reviews
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
pub const MEANING: &str = "declared contracts, or clauses of them, marked `resource_review_pending` (contract spec C06): each is loaded and validated but withheld, discharges nothing and accuses nothing, and every call site that would apply it is Unknown with reason code `awaiting-human-review`; a clause-level marker holds exactly the call sites a contract-level one does, because a contract is applied as one conjunction; `held_functions` counts analysed functions with at least one recorded held call of the declaration, and `held_call_sites` counts the held-call refusals the analysis recorded; the analysis of a path stops at its first refused statement, so a later call site on the same path records no refusal of its own and the count is a lower bound on textual call sites";

/// The `pending_human_review` section for one finished report.
#[must_use]
pub fn section(contracts: &ResourceContracts, report: &AuditReport) -> Value {
    render(
        &markers(contracts),
        &HeldCalls::from_rows(report.unit_rows()),
    )
}

/// The held-call refusals recorded in a set of unit rows: per callee, the call
/// sites held and the distinct `(library, file, function)` rows holding at
/// least one of them.
///
/// Card **W45-AUDIT-CHECKPOINTS**: the tally is a per-row fold, so the tally of
/// a whole report is the [`HeldCalls::absorb`] of its groups' tallies (sums of
/// call sites, unions of function rows). A per-group checkpoint records its
/// group's tally, and composition merges them into exactly the whole run's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeldCalls {
    pub call_sites: BTreeMap<String, usize>,
    pub functions: BTreeMap<String, BTreeSet<(String, String, String)>>,
}

impl HeldCalls {
    /// The tally of `rows`.
    #[must_use]
    pub fn from_rows(rows: &[Value]) -> Self {
        let mut held = Self::default();
        for unit in rows {
            let library = unit["library"].as_str().unwrap_or_default();
            let file = unit["file"].as_str().unwrap_or_default();
            for function in unit["functions"].as_array().into_iter().flatten() {
                let name = function["function"].as_str().unwrap_or_default();
                for diagnostic in function["diagnostics"].as_array().into_iter().flatten() {
                    let Some(callee) = diagnostic["description"].as_str().and_then(held_call)
                    else {
                        continue;
                    };
                    *held.call_sites.entry(callee.to_owned()).or_default() += 1;
                    held.functions
                        .entry(callee.to_owned())
                        .or_default()
                        .insert((library.to_owned(), file.to_owned(), name.to_owned()));
                }
            }
        }
        held
    }

    /// Add `other`'s call sites and unite its function rows.
    pub fn absorb(&mut self, other: Self) {
        for (callee, count) in other.call_sites {
            *self.call_sites.entry(callee).or_default() += count;
        }
        for (callee, rows) in other.functions {
            self.functions.entry(callee).or_default().extend(rows);
        }
    }
}

/// One review marker as the section publishes it, without its held counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingMarker {
    pub function: String,
    /// `contract` or `clause`.
    pub scope: String,
    pub clause: Option<String>,
    pub reason: String,
    pub owner: Option<String>,
    pub module: String,
    pub module_sha256: Option<String>,
}

/// The markers of one contract environment, in publication order, and the
/// number of declarations carrying them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PendingMarkers {
    pub declarations: usize,
    pub markers: Vec<PendingMarker>,
}

/// The markers `contracts` declares: a function of the environment alone.
#[must_use]
pub fn markers(contracts: &ResourceContracts) -> PendingMarkers {
    let mut markers = Vec::new();
    for (function, pending) in contracts.pending_review() {
        let digest = contracts
            .source_assumptions()
            .get(&pending.module)
            .map(|text| sha256_hex(text.as_bytes()));
        for deferral in &pending.deferrals {
            let (scope, clause) = match &deferral.scope {
                ReviewScope::Contract => ("contract", None),
                ReviewScope::Clause { clause } => ("clause", Some(clause.clone())),
            };
            markers.push(PendingMarker {
                function: function.clone(),
                scope: scope.to_owned(),
                clause,
                reason: deferral.reason.clone(),
                owner: deferral.owner.clone(),
                module: pending.module.clone(),
                module_sha256: digest.clone(),
            });
        }
    }
    PendingMarkers {
        declarations: contracts.pending_review().len(),
        markers,
    }
}

/// The section for `markers` over the held calls `held`.
#[must_use]
pub fn render(markers: &PendingMarkers, held: &HeldCalls) -> Value {
    let entries = markers
        .markers
        .iter()
        .map(|marker| {
            json!({
                "function": marker.function,
                "scope": marker.scope,
                "clause": marker.clause,
                "reason": marker.reason,
                "owner": marker.owner,
                "module": marker.module,
                "module_sha256": marker.module_sha256,
                "held_functions": held.functions.get(&marker.function).map_or(0, BTreeSet::len),
                "held_call_sites": held.call_sites.get(&marker.function).copied().unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();
    let held_functions = held
        .functions
        .values()
        .flatten()
        .collect::<BTreeSet<_>>()
        .len();
    json!({
        "meaning": MEANING,
        "reason_code": AWAITING_HUMAN_REVIEW,
        "declarations": markers.declarations,
        "markers": markers.markers.len(),
        "held_functions": held_functions,
        "held_call_sites": held.call_sites.values().sum::<usize>(),
        "entries": entries,
    })
}

/// The one-line console tally of a [`section`].
#[must_use]
pub fn tally(section: &Value) -> String {
    let count = |key: &str| section[key].as_u64().unwrap_or_default();
    format!(
        "pending human review: {} declaration(s), {} marker(s); {} function(s) and {} call \
         site(s) held Unknown ({AWAITING_HUMAN_REVIEW})",
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
             site(s) held Unknown (awaiting-human-review)"
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
