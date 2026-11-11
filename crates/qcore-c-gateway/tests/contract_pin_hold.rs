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

//! Card W45-CONTRACT-REVISION-MODE — a pinned contract the audit withheld for a
//! pin mismatch holds every call site that would apply it, under the dedicated
//! `contract-pin-mismatch` reason
//! (`docs/reports/w45-contract-revision-mode/README.md`). Since card
//! W45-USER-CONTRACT-PLUGINS a function whose only unresolved diagnostics are
//! held calls is `Pending`: a re-pin after review resolves it. It was `Unknown`.
//!
//! **The family is held out.** A `k_*` API unrelated to every production
//! library, declared inline: no fixture file and no production contract moves.
//! Every `Safe` below is `Safe` for a synthetic declared custody projection
//! under the premises the fixture names, and every `Unsafe` is provisional.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::pin_hold::{held_call, hold_description};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyFunctionSummary,
    LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult,
    analyze_legacy_c_translation_unit, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract_revision::PinMismatchReason;
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

/// The bare environment with `names` withheld for a pin mismatch, as the
/// audit's pin evaluation withholds them.
fn withheld(names: &[&str]) -> ResourceContracts {
    let mut contracts = load(BARE);
    for name in names {
        contracts
            .withhold_for_pin_mismatch(
                name,
                "k.qcore".into(),
                PinMismatchReason::SourceChanged,
                Some("contracts.lock".into()),
                "the audited source's sites differ from the lock".into(),
            )
            .unwrap();
    }
    contracts
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

fn held(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => {
                held_call(description).map(str::to_owned)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn safe_in_force_and_pending_for_a_pin_mismatch() {
    let bare = analyse(BALANCED, &load(BARE));
    assert_eq!(function(&bare, "w").status, AnalysisStatus::Safe);
    let summary = analyse(BALANCED, &withheld(&["k_close"]));
    let w = function(&summary, "w");
    assert_eq!(w.status, AnalysisStatus::Pending);
    assert_eq!(held(w), ["k_close"]);
    assert!(w.diagnostics.iter().any(|diagnostic| matches!(
        &diagnostic.kind,
        LegacyDiagnosticKind::Unsupported { description } if *description == hold_description("k_close")
    )));
}

/// A would-be provisional `Unsafe` through a withheld contract is never
/// alleged. Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL corrects its
/// verdict from `Pending` to `Unknown`: a re-pin would allege the violation,
/// so no human act alone stands between the row and `Safe`. `BALANCED`, which
/// the re-pinned declaration decides `Safe`, stays `Pending`.
#[test]
fn a_withheld_pin_accuses_nothing() {
    let bare = load(BARE);
    assert_eq!(
        function(&analyse(TWICE, &bare), "w").status,
        AnalysisStatus::Unsafe
    );
    assert_eq!(
        function(&analyse(LEAKED, &bare), "w").status,
        AnalysisStatus::Unsafe
    );
    for (body, names, callee, status) in [
        (TWICE, vec!["k_close"], "k_close", AnalysisStatus::Unknown),
        (
            BALANCED,
            vec!["k_close"],
            "k_close",
            AnalysisStatus::Pending,
        ),
        (LEAKED, vec!["k_open"], "k_open", AnalysisStatus::Unknown),
    ] {
        let summary = analyse(body, &withheld(&names));
        let w = function(&summary, "w");
        assert_eq!(w.status, status, "{body}");
        assert_eq!(held(w), [callee], "{body}");
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
}

/// Re-pinning — removing the mismatch — restores every verdict exactly.
#[test]
fn removing_the_mismatch_restores_the_verdicts_byte_for_byte() {
    let bare = load(BARE);
    let mut value = serde_json::to_value(withheld(&["k_close", "k_open"])).unwrap();
    // What a re-pin gives: the same environment with nothing withheld.
    let map = value.as_object_mut().unwrap();
    let withheld = map.remove("pin_mismatch").unwrap();
    for (name, record) in withheld.as_object().unwrap() {
        map["functions"][name] = record["contract"].clone();
    }
    let restored: ResourceContracts = serde_json::from_value(value).unwrap();
    assert_eq!(restored, bare);
    for body in [BALANCED, LEAKED, TWICE] {
        assert_eq!(
            format!("{:?}", analyse(body, &bare)),
            format!("{:?}", analyse(body, &restored)),
            "{body}"
        );
    }
}

/// A body defined in the unit under a withheld name is analysed as an
/// undeclared definition, and its callers are held whatever that body proves.
#[test]
fn a_defined_callee_does_not_lift_the_hold_from_its_callers() {
    let source = format!(
        "{PRELUDE}void free(void *p);\n\
         void k_close(void *h, int n) {{ free(h); }}\n{BALANCED}"
    );
    let mut contracts = load_resource_contract_sources(&BTreeMap::from([
        ("k.qcore".to_owned(), BARE.to_owned()),
        (
            "libc.qcore".to_owned(),
            "module contract;\n\nextern fn free(p: ptr<mut, u8>) -> unit\ncontract {\n    \
             requires resource_contract_v1;\n    ensures normal resource_released(p, nullable);\n    \
             ensures normal resource_return_none();\n};\n"
                .to_owned(),
        ),
    ]))
    .unwrap();
    contracts
        .withhold_for_pin_mismatch(
            "k_close",
            "k.qcore".into(),
            PinMismatchReason::SourceChanged,
            None,
            String::new(),
        )
        .unwrap();
    let summary = analyze_legacy_c_translation_unit(&source, "w", &contracts).unwrap();
    assert_eq!(function(&summary, "w").status, AnalysisStatus::Pending);
    assert_eq!(held(function(&summary, "w")), ["k_close"]);
}

#[test]
fn the_unit_reason_names_the_held_calls_under_their_own_code() {
    if qcore_c_gateway::clang_command().is_err() {
        eprintln!("skipping Clang-dependent W45-CONTRACT-REVISION-MODE test");
        return;
    }
    let source = format!("{PRELUDE}{BALANCED}");
    let unit = C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    match analyze_unrestricted_c11(&unit, "w", &withheld(&["k_close"])).unwrap() {
        result @ UnrestrictedLegacyResult::Unknown { .. } => {
            // Card W45-USER-CONTRACT-PLUGINS: the unit is `Pending`.
            assert_eq!(result.verdict(), qcore_schemas::verdict::Verdict::Pending);
            let UnrestrictedLegacyResult::Unknown { reason, .. } = result else {
                unreachable!()
            };
            assert!(
                reason.ends_with(", 1 call(s) held for a contract pin mismatch"),
                "{reason}"
            );
            let category = ReasonCategory::classify(&reason).unwrap();
            assert_eq!(category, ReasonCategory::ContractPinMismatch);
            assert_eq!(category.as_str(), "contract-pin-mismatch");
            assert_eq!(category.subject(), ReasonSubject::Analyser);
            assert!(!category.may_support_unsafe());
        }
        other => panic!("a held call must not decide the unit: {other:?}"),
    }
    // A unit that calls nothing withheld certifies under an environment that
    // withholds a contract, and its certificate's recheck reproduces the
    // withheld map; a certificate whose map was dropped is refused.
    let neutral = format!("{PRELUDE}void w(void) {{ return; }}");
    let certificate = match analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &neutral,
            ..Default::default()
        },
        "w",
        &withheld(&["k_close"]),
    )
    .unwrap()
    {
        UnrestrictedLegacyResult::Safe(certificate) => certificate,
        other => panic!("a unit that calls nothing withheld decides: {other:?}"),
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    let mut forged = certificate.clone();
    let mut value = serde_json::to_value(&forged.contracts).unwrap();
    value.as_object_mut().unwrap().remove("pin_mismatch");
    forged.contracts = serde_json::from_value(value).unwrap();
    assert!(check_legacy_safety_certificate(&forged).is_err());
}
