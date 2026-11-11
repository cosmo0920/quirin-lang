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

//! Card W45-CONTRACT-REVISION-MODE: revision modes of resource-v1 contracts
//! (contract spec §C06.2).
//!
//! `requires resource_revision(pinned|flexible);` marks a contract and
//! `ensures normal resource_revision(CLAUSE, pinned|flexible);` one clause.
//! Pinned is the default. These tests pin the round trip, the refusals that
//! keep loading total-or-reject, the non-inheritance of a mode (§C15), the
//! byte identity of every environment without a marker, the recheck of a
//! withheld pin mismatch, and the contract-text digest a lock is taken for.
//! Every module is an inline text, so the corpus pins do not move.

use std::collections::BTreeMap;

use qcore_contract::contract_inclusion::{
    ContractLoadPath, PublishedContractCrate, derive_contract_namespace,
};
use qcore_contract::contract_loader::{
    PublishedModuleRef, check_source_assumptions, load_published_resource_contracts,
    load_resource_contract_sources,
};
use qcore_contract::contract_revision::declaration_digests;
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_revision::{
    PinMismatchReason, RevisionMarker, RevisionMode, RevisionScope,
};

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

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), text.to_owned())]))
}

/// `BARE` with `k_close`'s requirements followed by `marker`.
fn contract_marked(marker: &str) -> String {
    BARE.replace(
        "    ensures normal resource_released(h, nonnull);",
        &format!("    {marker}\n    ensures normal resource_released(h, nonnull);"),
    )
}

fn replaced(clause: &str, replacement: &str) -> String {
    assert!(BARE.contains(clause), "{clause}");
    BARE.replace(clause, replacement)
}

fn refused(text: &str) -> String {
    load(text).expect_err("the whole module must be refused")
}

#[test]
fn the_default_is_pinned_and_records_nothing() {
    let contracts = load(BARE).unwrap();
    assert_eq!(contracts.revision_mode("k_close"), RevisionMode::Pinned);
    assert_eq!(contracts.revision_mode("k_open"), RevisionMode::Pinned);
    assert!(contracts.declared_revisions().is_empty());
    let json = serde_json::to_string(&contracts).unwrap();
    assert!(!json.contains("revision"), "{json}");
    assert!(!json.contains("pin_mismatch"), "{json}");
}

#[test]
fn a_contract_level_marker_round_trips_and_changes_no_mode_of_custody() {
    let bare = load(BARE).unwrap();
    let flexible = load(&contract_marked("requires resource_revision(flexible);")).unwrap();
    assert_eq!(flexible.revision_mode("k_close"), RevisionMode::Flexible);
    assert_eq!(flexible.revision_mode("k_open"), RevisionMode::Pinned);
    assert_eq!(flexible.functions(), bare.functions());
    let declared = &flexible.declared_revisions()["k_close"];
    assert_eq!(
        declared.markers,
        [RevisionMarker {
            scope: RevisionScope::Contract,
            mode: RevisionMode::Flexible
        }]
    );
    let pinned = load(&contract_marked("requires resource_revision(pinned);")).unwrap();
    assert_eq!(pinned.revision_mode("k_close"), RevisionMode::Pinned);
    assert_eq!(pinned.functions(), bare.functions());
    // The serialized environment round trips.
    let json = serde_json::to_string(&flexible).unwrap();
    let back: ResourceContracts = serde_json::from_str(&json).unwrap();
    assert_eq!(back, flexible);
    check_source_assumptions(&flexible).unwrap();
}

#[test]
fn one_pinned_clause_pins_a_flexible_contract() {
    let text = contract_marked("requires resource_revision(flexible);").replace(
        "    ensures normal resource_released(h, nonnull);",
        "    ensures normal resource_revision(resource_released(h, nonnull), pinned);",
    );
    let contracts = load(&text).unwrap();
    assert_eq!(contracts.revision_mode("k_close"), RevisionMode::Pinned);
    assert_eq!(contracts.functions(), load(BARE).unwrap().functions());
    assert_eq!(
        contracts.declared_revisions()["k_close"].markers[1],
        RevisionMarker {
            scope: RevisionScope::Clause {
                clause: "ensures normal resource_released(h, nonnull)".into()
            },
            mode: RevisionMode::Pinned
        }
    );
}

#[test]
fn a_flexible_clause_leaves_a_pinned_contract_pinned() {
    let contracts = load(&replaced(
        "    ensures normal resource_borrowed(n);",
        "    ensures normal resource_revision(resource_borrowed(n), flexible);",
    ))
    .unwrap();
    assert_eq!(contracts.revision_mode("k_close"), RevisionMode::Pinned);
    assert_eq!(contracts.declared_revisions()["k_close"].markers.len(), 1);
}

#[test]
fn a_duplicate_or_conflicting_mode_refuses_the_module() {
    let twice = contract_marked(
        "requires resource_revision(flexible);\n    requires resource_revision(flexible);",
    );
    assert!(refused(&twice).contains("duplicate revision marker"));
    let conflict = contract_marked(
        "requires resource_revision(flexible);\n    requires resource_revision(pinned);",
    );
    assert!(refused(&conflict).contains("conflicting revision markers"));
    let restated = contract_marked("requires resource_revision(flexible);").replace(
        "    ensures normal resource_borrowed(n);",
        "    ensures normal resource_revision(resource_borrowed(n), flexible);",
    );
    assert!(refused(&restated).contains("already makes every clause `flexible`"));
    let nested = replaced(
        "    ensures normal resource_borrowed(n);",
        "    ensures normal resource_revision(resource_revision(resource_borrowed(n), pinned), \
         flexible);",
    );
    assert!(refused(&nested).contains("cannot wrap another marker"));
}

#[test]
fn a_mode_on_an_unsupported_item_refuses_the_module() {
    for (clause, replacement, expected) in [
        (
            "    requires resource_contract_v1;\n    ensures normal resource_released",
            "    requires resource_revision(resource_contract_v1, pinned);\n    ensures normal \
             resource_released",
            "malformed revision marker",
        ),
        (
            "    requires resource_contract_v1;\n    ensures normal resource_released",
            "    requires resource_contract_v1;\n    requires \
             resource_revision(resource_literal_argument(n, 0), pinned);\n    ensures normal \
             resource_released",
            "take the contract's mode",
        ),
        (
            "    ensures normal resource_borrowed(n);",
            "    ensures normal resource_revision(flexible);",
            "a contract-level revision marker is a requirement",
        ),
        (
            "    ensures normal resource_borrowed(n);",
            "    ensures normal resource_revision(resource_borrowed(n), loose);",
            "malformed revision marker",
        ),
        (
            "    ensures normal resource_borrowed(n);",
            "    ensures normal resource_revision();",
            "malformed revision marker",
        ),
        (
            "    ensures normal resource_borrowed(n);",
            "    ensures normal resource_revision(resource_borrowed(n));",
            "malformed revision marker",
        ),
        (
            "    ensures normal resource_borrowed(n);",
            "    ensures normal resource_revision(resource_review_pending(resource_borrowed(n), \
             \"why\"), pinned);",
            "cannot wrap another marker",
        ),
        (
            "    ensures normal resource_borrowed(n);",
            "    ensures normal resource_review_pending(resource_revision(resource_borrowed(n), \
             pinned), \"why\");",
            "cannot wrap another marker",
        ),
    ] {
        let error = refused(&replaced(clause, replacement));
        assert!(error.contains(expected), "{replacement}: {error}");
    }
}

#[test]
fn removing_the_markers_restores_the_environment_byte_for_byte() {
    let marked = contract_marked("requires resource_revision(flexible);");
    let unmarked = marked.replace("    requires resource_revision(flexible);\n", "");
    assert_eq!(unmarked, BARE);
    assert_eq!(
        serde_json::to_string(&load(&unmarked).unwrap()).unwrap(),
        serde_json::to_string(&load(BARE).unwrap()).unwrap()
    );
}

#[test]
fn a_withheld_pin_rechecks_and_a_forged_one_refuses() {
    let mut contracts = load(BARE).unwrap();
    contracts
        .withhold_for_pin_mismatch(
            "k_close",
            "k.qcore".into(),
            PinMismatchReason::SourceChanged,
            Some("contracts.lock".into()),
            "changed".into(),
        )
        .unwrap();
    assert!(contracts.contract("k_close").is_none());
    assert_eq!(
        contracts.pin_mismatch()["k_close"].contract,
        *load(BARE).unwrap().contract("k_close").unwrap()
    );
    check_source_assumptions(&contracts).unwrap();
    // A second withholding of the same name, or of an unknown one, refuses.
    assert!(
        contracts
            .clone()
            .withhold_for_pin_mismatch(
                "k_close",
                "k.qcore".into(),
                PinMismatchReason::NoLock,
                None,
                String::new()
            )
            .is_err()
    );
    // A flexible contract has no pin to mismatch.
    let mut flexible = load(&contract_marked("requires resource_revision(flexible);")).unwrap();
    let error = flexible
        .withhold_for_pin_mismatch(
            "k_close",
            "k.qcore".into(),
            PinMismatchReason::NoLock,
            None,
            String::new(),
        )
        .unwrap_err();
    assert!(error.contains("flexible"), "{error}");
    // A forged record: the withheld contract differs from its source.
    let mut value = serde_json::to_value(&contracts).unwrap();
    value["pin_mismatch"]["k_close"]["contract"]["parameters"][0] = serde_json::json!("borrowed");
    let forged: ResourceContracts = serde_json::from_value(value).unwrap();
    assert!(check_source_assumptions(&forged).is_err());
    // A forged record: a contract both in force and withheld.
    let mut value = serde_json::to_value(&contracts).unwrap();
    value["pin_mismatch"]["k_open"] = value["pin_mismatch"]["k_close"].clone();
    let forged: ResourceContracts = serde_json::from_value(value).unwrap();
    assert!(check_source_assumptions(&forged).is_err());
}

#[test]
fn a_contract_digest_ignores_layout_and_sees_every_token() {
    let base = declaration_digests(&BTreeMap::from([("k.qcore".into(), BARE.to_owned())])).unwrap();
    let relaid = BARE.replace(
        "    ensures normal resource_borrowed(n);",
        "    // a comment\n    ensures   normal resource_borrowed( n );",
    );
    let same = declaration_digests(&BTreeMap::from([("k.qcore".into(), relaid)])).unwrap();
    assert_eq!(base, same);
    let changed = declaration_digests(&BTreeMap::from([(
        "k.qcore".into(),
        replaced(
            "resource_released(h, nonnull)",
            "resource_released(h, nullable)",
        ),
    )]))
    .unwrap();
    assert_eq!(base["k_open"], changed["k_open"]);
    assert_ne!(base["k_close"], changed["k_close"]);
    assert_eq!(base["k_close"].0, "k.qcore");
}

const LIBC: &str = "quirin-provider-libc";
const APP: &str = "quirin-provider-app";

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

const FLEXIBLE_PARENT: &str = "module contract;

extern fn free(ptr: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_revision(flexible);
    ensures normal resource_released(ptr, nullable);
    ensures normal resource_return_none();
};
";

const CHILD: &str = "module contract;
#include \"quirin-provider-libc:libc/heap.qcore\";

extern fn app_free(p: ptr<mut, u8>) -> unit
contract inherit libc::free;
";

#[test]
fn a_mode_is_not_inherited() {
    let load_path = ContractLoadPath::new(vec![
        krate(LIBC, &[], &[("libc/heap.qcore", FLEXIBLE_PARENT)]),
        krate(APP, &[LIBC], &[("app/heap.qcore", CHILD)]),
    ])
    .unwrap();
    let contracts = load_published_resource_contracts(
        &load_path,
        &[
            PublishedModuleRef::new(LIBC, "libc/heap.qcore"),
            PublishedModuleRef::new(APP, "app/heap.qcore"),
        ],
    )
    .unwrap();
    assert_eq!(contracts.revision_mode("free"), RevisionMode::Flexible);
    // The child names a different function whose source was never reviewed
    // under the parent's mode: it is pinned by default.
    assert_eq!(contracts.revision_mode("app_free"), RevisionMode::Pinned);
    assert!(!contracts.declared_revisions().contains_key("app_free"));
    let elaborated = &contracts.source_assumptions()["quirin-provider-app:app/heap.qcore"];
    assert!(!elaborated.contains("resource_revision"), "{elaborated}");
    // The custody meaning is inherited unchanged.
    assert_eq!(
        contracts.contract("app_free").unwrap(),
        contracts.contract("free").unwrap()
    );
    check_source_assumptions(&contracts).unwrap();
}
