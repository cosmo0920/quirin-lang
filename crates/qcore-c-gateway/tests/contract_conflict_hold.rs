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

//! Card CONTRACT-SUPERSESSION (contract spec §C06.6): two modules declare one
//! function. The declaration with the greater pin stamp is in force and the
//! other is retained as superseded; when the stamps do not decide, no
//! declaration is in force and every call relying on the function is held
//! under `contract-conflict`, which is `Unknown`: never `Safe`, never a
//! provisional `Unsafe`, never `Pending`.
//!
//! **The family is held out.** A `k_*` API declared inline; no fixture file
//! and no production contract moves.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::conflict_hold::{CONTRACT_CONFLICT, held_call, hold_description};
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceContracts, analyze_legacy_c_translation_unit,
};
use qcore_contract::contract_loader::{
    check_source_assumptions, load_resource_contract_sources,
    load_resource_contract_sources_stamped,
};
use qcore_contract::contract_revision::module_declaration_digests;
use qcore_contract::contract_supersession::StampTable;
use qcore_schemas::contract_supersession::{ConflictReason, StampStanding};
use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

const OPEN: &str = "module contract;

extern fn k_open() -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};
";

/// `k_close` releases its handle.
const RELEASING: &str = "module contract;

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(h, nonnull);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
";

/// Another text of `k_close`: it only borrows its handle, so a caller that
/// opened one leaks it.
const BORROWING: &str = "module contract;

extern fn k_note(n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

extern fn k_close(h: ptr<mut, u8>, n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(h);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};
";

const PRELUDE: &str = "void *k_open(void);\nvoid k_close(void *h, int n);\nvoid k_note(int n);\n";
const BALANCED: &str = "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); }";
const TWICE: &str =
    "void w(void) { void *h; h = k_open(); if (!h) { return; } k_close(h, 0); k_close(h, 0); }";

fn sources() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("a/open.qcore".to_owned(), OPEN.to_owned()),
        ("a/close.qcore".to_owned(), RELEASING.to_owned()),
        ("b/close.qcore".to_owned(), BORROWING.to_owned()),
    ])
}

fn digest(module: &str, function: &str) -> String {
    let sources = sources();
    module_declaration_digests(module, &sources[module])
        .unwrap()
        .into_iter()
        .find(|declaration| declaration.name == function)
        .unwrap()
        .contract_sha256
}

/// Stamps for the two `k_close` declarations.
fn stamps(a: Option<&str>, b: Option<&str>) -> StampTable {
    let mut table = BTreeMap::new();
    for (module, stamp) in [("a/close.qcore", a), ("b/close.qcore", b)] {
        if let Some(stamp) = stamp {
            table.insert(
                (module.to_owned(), "k_close".to_owned()),
                (
                    digest(module, "k_close"),
                    StampStanding::Stamped(stamp.to_owned()),
                ),
            );
        }
    }
    StampTable(table)
}

fn analyse(body: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{PRELUDE}{body}"), "w", contracts)
        .expect("the fixture tokenizes")
}

fn w(summary: &LegacyProgramSummary) -> &LegacyFunctionSummary {
    summary.functions.get("w").expect("w is analysed")
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
fn the_newer_stamp_is_in_force_and_the_older_retained() {
    let contracts = load_resource_contract_sources_stamped(
        &sources(),
        &stamps(Some("2026-10-02T00:00:00Z"), Some("2026-10-01T00:00:00Z")),
    )
    .unwrap();
    check_source_assumptions(&contracts).unwrap();
    assert!(contracts.contract_conflict().is_empty());
    let superseded = &contracts.superseded()["k_close"];
    assert_eq!(superseded.len(), 1);
    assert_eq!(superseded[0].module, "b/close.qcore");
    assert_eq!(superseded[0].by_module, "a/close.qcore");
    assert_eq!(superseded[0].pinned_at, "2026-10-01T00:00:00Z");
    assert_eq!(superseded[0].by_pinned_at, "2026-10-02T00:00:00Z");
    assert!(superseded[0].text.starts_with("extern fn k_close"));
    // The superseded text is cut out of its module, every other byte kept.
    let kept = &contracts.source_assumptions()["b/close.qcore"];
    assert_eq!(kept.len(), BORROWING.len());
    assert_eq!(kept.lines().count(), BORROWING.lines().count());
    assert!(kept.contains("extern fn k_note") && !kept.contains("k_close"));
    // The releasing declaration decides: `Safe`, and the double release is a
    // provisional `Unsafe` exactly as with that declaration alone.
    assert_eq!(
        w(&analyse(BALANCED, &contracts)).status,
        AnalysisStatus::Safe
    );
    assert_eq!(
        w(&analyse(TWICE, &contracts)).status,
        AnalysisStatus::Unsafe
    );

    // The other order of stamps puts the borrowing text in force.
    let contracts = load_resource_contract_sources_stamped(
        &sources(),
        &stamps(Some("2026-10-01T00:00:00Z"), Some("2026-10-02T00:00:00Z")),
    )
    .unwrap();
    assert_eq!(contracts.superseded()["k_close"][0].module, "a/close.qcore");
    // The releasing module is left without a declaration and is dropped.
    assert!(!contracts.source_assumptions().contains_key("a/close.qcore"));
    assert_ne!(
        w(&analyse(BALANCED, &contracts)).status,
        AnalysisStatus::Safe
    );
}

#[test]
fn undecided_stamps_hold_every_call_as_unknown() {
    for (table, reason) in [
        (stamps(None, None), ConflictReason::Unpinned),
        (
            stamps(Some("2026-10-01T00:00:00Z"), None),
            ConflictReason::Unpinned,
        ),
        (
            stamps(Some("2026-10-01T00:00:00Z"), Some("2026-10-01T00:00:00Z")),
            ConflictReason::EqualStamps,
        ),
    ] {
        let contracts = load_resource_contract_sources_stamped(&sources(), &table).unwrap();
        check_source_assumptions(&contracts).unwrap();
        let conflict = &contracts.contract_conflict()["k_close"];
        assert_eq!(conflict.reason, reason);
        assert_eq!(conflict.declarations.len(), 2, "both texts are retained");
        assert!(contracts.contract("k_close").is_none());
        for body in [BALANCED, TWICE] {
            let summary = analyse(body, &contracts);
            let row = w(&summary);
            // Never Safe, never a provisional Unsafe, never Pending.
            assert_eq!(row.status, AnalysisStatus::Unknown, "{body}");
            assert!(held(row).iter().all(|callee| callee == "k_close"));
            assert!(!held(row).is_empty());
        }
    }
}

#[test]
fn the_unstamped_loader_withholds_every_duplicate() {
    let contracts = load_resource_contract_sources(&sources()).unwrap();
    assert_eq!(
        contracts.contract_conflict()["k_close"].reason,
        ConflictReason::Unpinned
    );
    assert!(contracts.contract("k_open").is_some());
    assert!(contracts.contract("k_note").is_some());
}

#[test]
fn byte_identical_duplicates_are_idempotent() {
    let sources = BTreeMap::from([
        ("a/close.qcore".to_owned(), RELEASING.to_owned()),
        ("b/close.qcore".to_owned(), RELEASING.to_owned()),
        ("a/open.qcore".to_owned(), OPEN.to_owned()),
    ]);
    let digest = module_declaration_digests("a/close.qcore", RELEASING).unwrap()[0]
        .contract_sha256
        .clone();
    let table = StampTable(BTreeMap::from([
        (
            ("a/close.qcore".to_owned(), "k_close".to_owned()),
            (
                digest.clone(),
                StampStanding::Stamped("2026-10-01T00:00:00Z".into()),
            ),
        ),
        (
            ("b/close.qcore".to_owned(), "k_close".to_owned()),
            (
                digest,
                StampStanding::Stamped("2026-10-01T00:00:00Z".into()),
            ),
        ),
    ]));
    let contracts = load_resource_contract_sources_stamped(&sources, &table).unwrap();
    assert!(contracts.contract_conflict().is_empty());
    assert_eq!(
        contracts.superseded()["k_close"][0].module,
        "b/close.qcore",
        "the least module key stays in force"
    );
    assert_eq!(
        w(&analyse(BALANCED, &contracts)).status,
        AnalysisStatus::Safe
    );
}

#[test]
fn the_refusal_classifies_as_a_contract_conflict() {
    let description = hold_description("k_close");
    assert!(description.contains(CONTRACT_CONFLICT));
    assert_eq!(held_call(&description), Some("k_close"));
    let category = ReasonCategory::classify(&description).unwrap();
    assert_eq!(category, ReasonCategory::ContractConflict);
    assert_eq!(category.as_str(), CONTRACT_CONFLICT);
    assert_eq!(category.subject(), ReasonSubject::Analyser);
    assert!(!category.may_support_unsafe());
    assert!(!category.is_human_resolvable());
}

#[test]
fn a_source_spelling_of_the_prefix_forges_no_conflict() {
    let contracts = load_resource_contract_sources(&sources()).unwrap();
    let body = "void w(void) { qcore_contract_conflict__k_close(0, 0); }";
    let summary = analyse(body, &contracts);
    let row = w(&summary);
    assert_eq!(row.status, AnalysisStatus::Unknown);
    assert!(held(row).is_empty(), "the source's own spelling is escaped");
}
