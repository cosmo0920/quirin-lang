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

//! Card W45-CONTRACT-DEFERRED-REVIEW — a declared contract, or one of its
//! clauses, awaiting human review holds every call site that would apply it,
//! under the dedicated `awaiting-human-review` reason
//! (`docs/reports/w45-contract-deferred-review/README.md`). Since card
//! W45-USER-CONTRACT-PLUGINS a function whose only unresolved diagnostics are
//! held calls is `Pending` (never `Safe`, below `Unknown`); it was `Unknown`.
//!
//! **The family is held out.** A `k_*` API unrelated to every production
//! library, declared inline: no fixture file and no production contract moves.
//! Every `Safe` below is `Safe` for a synthetic declared custody projection
//! under the premises the fixture names, and every `Unsafe` is provisional.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult,
    analyze_legacy_c_translation_unit, analyze_unrestricted_c11, check_legacy_safety_certificate,
    held_call, hold_description,
};
use qcore_contract::contract_inclusion::{
    ContractLoadPath, PublishedContractCrate, derive_contract_namespace,
};
use qcore_contract::contract_loader::{
    PublishedModuleRef, load_published_resource_contracts, load_resource_contract_sources,
};
use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

const BARE: &str = "module contract;

extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "void *k_open(void);\nvoid k_close(void *h, int n);\n";

/// Open, test, close: `Safe` under the bare contracts.
const BALANCED: &str = "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); }";
/// Open and forget: a provisional leak under the bare contracts.
const LEAKED: &str = "void w(void) { void *h; h = k_open(); if (!h) { return; } }";
/// Close twice: a provisional double release under the bare contracts.
const TWICE: &str =
    "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); k_close(h, 0); }";

fn load(text: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), text.to_owned())]))
        .expect("the fixture module loads")
}

/// `BARE` with `k_close`'s whole contract deferred.
fn close_deferred() -> String {
    BARE.replace(
        "    ensures normal resource_released(h, nonnull);",
        "    requires resource_review_pending(\"is the handle freed on every path?\");\n    \
         ensures normal resource_released(h, nonnull);",
    )
}

/// `BARE` with one clause wrapped in a clause-level marker.
fn clause_deferred(clause: &str) -> String {
    let marked = format!(
        "ensures normal resource_review_pending({}, \"under review\");",
        clause
            .strip_prefix("ensures normal ")
            .and_then(|rest| rest.strip_suffix(';'))
            .expect("an ensures clause")
    );
    assert!(BARE.contains(clause), "{clause}");
    BARE.replace(clause, &marked)
}

fn analyse(body: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{PRELUDE}{body}"), "w", contracts)
        .expect("the fixture tokenizes")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .expect("the function is analysed")
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

fn held(summary: &LegacyFunctionSummary) -> Vec<String> {
    unsupported(summary)
        .iter()
        .filter_map(|description| held_call(description).map(str::to_owned))
        .collect()
}

#[test]
fn safe_with_the_contract_and_unknown_awaiting_review_without_it() {
    let bare = analyse(BALANCED, &load(BARE));
    assert_eq!(
        function(&bare, "w").status,
        AnalysisStatus::Safe,
        "the control decides"
    );
    // The whole contract deferred.
    let deferred = analyse(BALANCED, &load(&close_deferred()));
    let w = function(&deferred, "w");
    assert_eq!(w.status, AnalysisStatus::Pending);
    assert_eq!(held(w), ["k_close"]);
    assert!(unsupported(w).contains(&hold_description("k_close")));
    // Only the load-bearing clause deferred.
    let clause = analyse(
        BALANCED,
        &load(&clause_deferred(
            "ensures normal resource_released(h, nonnull);",
        )),
    );
    assert_eq!(function(&clause, "w").status, AnalysisStatus::Pending);
    assert_eq!(held(function(&clause, "w")), ["k_close"]);
}

/// A deferred clause is never dropped: deferring even a clause that is not
/// load-bearing here holds the call, because dropping it would let the
/// remaining clauses decide the call on their own.
#[test]
fn a_deferred_clause_is_never_silently_dropped() {
    for clause in [
        "ensures normal resource_borrowed(n);",
        "ensures normal resource_return_none();",
    ] {
        let summary = analyse(BALANCED, &load(&clause_deferred(clause)));
        let w = function(&summary, "w");
        assert_eq!(w.status, AnalysisStatus::Pending, "{clause}");
        assert_eq!(held(w), ["k_close"], "{clause}");
    }
}

/// A would-be provisional `Unsafe` through a deferred contract is never
/// alleged. Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL corrects its
/// verdict from `Pending` to `Unknown`: accepting the declaration would allege
/// the violation, so no human act alone stands between the row and `Safe`.
/// `BALANCED`, whose accepted declaration decides it `Safe`, stays `Pending`.
#[test]
fn a_deferred_contract_accuses_nothing() {
    // The deferred release: the double release the bare contract alleges, and
    // the leak that ignoring the release would allege.
    let bare = load(BARE);
    let deferred = load(&close_deferred());
    assert_eq!(
        function(&analyse(TWICE, &bare), "w").status,
        AnalysisStatus::Unsafe
    );
    for (body, status) in [
        (TWICE, AnalysisStatus::Unknown),
        (BALANCED, AnalysisStatus::Pending),
    ] {
        let summary = analyse(body, &deferred);
        let w = function(&summary, "w");
        assert_eq!(w.status, status, "{body}");
        assert!(!held(w).is_empty(), "{body}");
        assert!(
            w.diagnostics.iter().all(|diagnostic| matches!(
                diagnostic.kind,
                LegacyDiagnosticKind::Unsupported { .. }
                    | LegacyDiagnosticKind::UnknownAlias { .. }
            )),
            "no violation is alleged through a held call: {:?}",
            w.diagnostics
        );
    }
    // The deferred acquisition: the leak the bare contract alleges.
    assert_eq!(
        function(&analyse(LEAKED, &bare), "w").status,
        AnalysisStatus::Unsafe
    );
    let open_deferred = load(&BARE.replace(
        "    ensures normal resource_return_owned(nullable);",
        "    ensures normal resource_review_pending(resource_return_owned(nullable), \"why\");",
    ));
    let summary = analyse(LEAKED, &open_deferred);
    assert_eq!(function(&summary, "w").status, AnalysisStatus::Unknown);
    assert_eq!(held(function(&summary, "w")), ["k_open"]);
    assert!(
        function(&summary, "w")
            .diagnostics
            .iter()
            .all(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Unsupported { .. })),
        "the leak is not alleged: {:?}",
        function(&summary, "w").diagnostics
    );
}

#[test]
fn removing_the_marker_restores_the_original_verdict_byte_for_byte() {
    let bare = load(BARE);
    let restored = load(&close_deferred().replace(
        "    requires resource_review_pending(\"is the handle freed on every path?\");\n",
        "",
    ));
    assert_eq!(restored, bare);
    for body in [BALANCED, LEAKED, TWICE] {
        let before = format!("{:?}", analyse(body, &bare));
        let after = format!("{:?}", analyse(body, &restored));
        assert_eq!(before, after, "{body}");
    }
}

/// A body defined in the unit under a pending name is analysed as an
/// undeclared definition, and its callers are held whatever that body proves.
#[test]
fn a_defined_callee_does_not_lift_the_hold_from_its_callers() {
    let source = format!(
        "{PRELUDE}void free(void *p);\n\
         void k_close(void *h, int n) {{ free(h); }}\n{BALANCED}"
    );
    let contracts = load_resource_contract_sources(&BTreeMap::from([
        ("k.qcore".to_owned(), close_deferred()),
        (
            "libc.qcore".to_owned(),
            "module contract;\n\nextern fn free(p: ptr<mut, u8>) -> unit\ncontract {\n    \
             requires resource_contract_v1;\n    ensures normal resource_released(p, nullable);\n    \
             ensures normal resource_return_none();\n};\n"
                .to_owned(),
        ),
    ]))
    .unwrap();
    let summary = analyze_legacy_c_translation_unit(&source, "w", &contracts).unwrap();
    assert_eq!(function(&summary, "w").status, AnalysisStatus::Pending);
    assert_eq!(held(function(&summary, "w")), ["k_close"]);
}

// ---------------------------------------------------------------------------
// Inheritance (contract spec §C15)
// ---------------------------------------------------------------------------

fn krate(name: &str, dependencies: &[&str], modules: &[(&str, &str)]) -> PublishedContractCrate {
    PublishedContractCrate {
        name: name.to_owned(),
        namespace: derive_contract_namespace(name).expect("fixture crate names derive"),
        version: "0.1.0".to_owned(),
        dependencies: dependencies.iter().map(|d| (*d).to_owned()).collect(),
        modules: modules
            .iter()
            .map(|(module, text)| ((*module).to_owned(), (*text).to_owned()))
            .collect(),
        published: modules
            .iter()
            .map(|(module, _)| (*module).to_owned())
            .collect(),
    }
}

#[test]
fn a_child_inheriting_a_deferred_clause_is_held_at_its_own_call_sites() {
    let parent = clause_deferred("ensures normal resource_released(h, nonnull);");
    let child = "module contract;\n#include \"quirin-provider-k:k/api.qcore\";\n\n\
                 extern fn app_close(handle: ptr<mut, u8>, flags: i32) -> unit\n\
                 contract inherit k::k_close;\n";
    let load_path = ContractLoadPath::new(vec![
        krate("quirin-provider-k", &[], &[("k/api.qcore", &parent)]),
        krate(
            "quirin-provider-app",
            &["quirin-provider-k"],
            &[("app/api.qcore", child)],
        ),
    ])
    .unwrap();
    let contracts = load_published_resource_contracts(
        &load_path,
        &[
            PublishedModuleRef::new("quirin-provider-k", "k/api.qcore"),
            PublishedModuleRef::new("quirin-provider-app", "app/api.qcore"),
        ],
    )
    .unwrap();
    assert!(contracts.pending_review().contains_key("app_close"));
    let source = "void *k_open(void);\nvoid app_close(void *h, int n);\n\
                  void w(void) { void *h; h = k_open(); if (!h) { return; } app_close(h, 0); }";
    let summary = analyze_legacy_c_translation_unit(source, "w", &contracts).unwrap();
    assert_eq!(function(&summary, "w").status, AnalysisStatus::Pending);
    assert_eq!(held(function(&summary, "w")), ["app_close"]);
}

// ---------------------------------------------------------------------------
// The production entry point: the unit reason, its code and the certificate
// ---------------------------------------------------------------------------

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

#[test]
fn the_unit_reason_names_the_held_calls_under_their_own_code() {
    if !clang_available() {
        eprintln!("skipping Clang-dependent W45-CONTRACT-DEFERRED-REVIEW test");
        return;
    }
    let source = format!("{PRELUDE}{BALANCED}");
    let unit = C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    match analyze_unrestricted_c11(&unit, "w", &load(&close_deferred())).unwrap() {
        result @ UnrestrictedLegacyResult::Unknown { .. } => {
            // Card W45-USER-CONTRACT-PLUGINS: the unit is `Pending`.
            assert_eq!(result.verdict(), qcore_schemas::verdict::Verdict::Pending);
            let UnrestrictedLegacyResult::Unknown { reason, .. } = result else {
                unreachable!()
            };
            assert!(
                reason.ends_with(", 1 call(s) held awaiting human review"),
                "{reason}"
            );
            let category = ReasonCategory::classify(&reason).unwrap();
            assert_eq!(category, ReasonCategory::AwaitingHumanReview);
            assert_eq!(category.subject(), ReasonSubject::Analyser);
            assert!(!category.may_support_unsafe());
        }
        other => panic!("a held call must not decide the unit: {other:?}"),
    }
    // The bare control certifies, and so does a unit that calls nothing
    // pending under an environment that holds a pending contract; that
    // certificate's recheck reproduces the pending map from the retained text.
    assert!(matches!(
        analyze_unrestricted_c11(&unit, "w", &load(BARE)).unwrap(),
        UnrestrictedLegacyResult::Safe(_)
    ));
    let deferred = load(&close_deferred());
    let neutral = format!("{PRELUDE}void w(void) {{ return; }}");
    let certificate = match analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &neutral,
            ..Default::default()
        },
        "w",
        &deferred,
    )
    .unwrap()
    {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("a unit that calls nothing pending decides: {other:?}"),
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    let mut forged = certificate.clone();
    forged.contracts.set_pending_review(BTreeMap::new());
    assert!(check_legacy_safety_certificate(&forged).is_err());
}

/// Card W45-PARENTHESISED-CALLEE: `(k_close)(h, 0)` calls `k_close`, so it is
/// held exactly as `k_close(h, 0)` is. Before that card the parenthesised
/// callee was invisible to the hold rewrite and to the classifier, the
/// statement lowered to nothing and the handle was reported leaked
/// (provisional `Unsafe`).
#[test]
fn a_parenthesised_call_of_a_pending_callee_is_held() {
    let parenthesised = BALANCED.replace("k_close(h, 0)", "(k_close)(h, 0)");
    assert_ne!(parenthesised, BALANCED);
    // Under the bare contracts the parenthesised release is the release.
    let bare = analyse(&parenthesised, &load(BARE));
    assert_eq!(function(&bare, "w").status, AnalysisStatus::Safe);
    assert_eq!(bare, analyse(BALANCED, &load(BARE)));
    // Deferred, it is held under the review reason, never a leak.
    let deferred = analyse(&parenthesised, &load(&close_deferred()));
    let w = function(&deferred, "w");
    assert_eq!(w.status, AnalysisStatus::Pending);
    assert_eq!(held(w), ["k_close"]);
    assert!(
        !w.diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind, LegacyDiagnosticKind::Leak { .. })),
        "{:?}",
        w.diagnostics
    );
    assert_eq!(deferred, analyse(BALANCED, &load(&close_deferred())));
}
