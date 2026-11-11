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

//! Card W45-FB-BACKLOG-CUSTODY: the checked-specification marker
//! `requires resource_body_checked;` (contract spec §C06.3).
//!
//! The loader admits the marker as a contract-level requirement with no
//! argument, at most once, and records nothing new in the environment: the
//! marked declaration is an ordinary entry of the function map, and which
//! declarations carry the marker is rederived from the retained texts
//! (`declared_checked_specifications`), exactly as the variadic tail and the
//! literal premise are. So an environment serializes byte for byte as before,
//! and every consumer that knows nothing of the marker applies the
//! declaration as it applies any other — the gateway is the consumer that
//! withdraws a specification its body does not establish.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{
    BODY_CHECKED_MARKER, checked_specifications, declared_checked_specifications,
    load_resource_contract_sources,
};

const SPEC: &str = "module contract;

extern fn w_make(chunk: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_body_checked;
    ensures normal resource_borrowed(chunk);
    ensures normal resource_return_owned(nullable);
};

extern fn w_free(w: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(w, nonnull);
    ensures normal resource_return_none();
};
";

fn sources(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("w.qcore".to_owned(), text.to_owned())])
}

#[test]
fn the_marker_is_a_requirement_the_loader_admits_and_does_not_store() {
    assert_eq!(BODY_CHECKED_MARKER, "resource_body_checked");
    let marked = load_resource_contract_sources(&sources(SPEC)).unwrap();
    let bare = load_resource_contract_sources(&sources(
        &SPEC.replace("    requires resource_body_checked;\n", ""),
    ))
    .unwrap();
    // The function map is the same map: the marker changes no mode.
    assert_eq!(marked.functions(), bare.functions());
    assert_eq!(
        declared_checked_specifications(&marked).unwrap(),
        ["w_make".to_owned()].into_iter().collect()
    );
    assert!(declared_checked_specifications(&bare).unwrap().is_empty());
}

#[test]
fn a_duplicate_marker_rejects_the_whole_module() {
    let doubled = SPEC.replace(
        "    requires resource_body_checked;\n",
        "    requires resource_body_checked;\n    requires resource_body_checked;\n",
    );
    let error = load_resource_contract_sources(&sources(&doubled)).unwrap_err();
    assert!(error.contains("duplicate requirement"), "{error}");
    assert!(checked_specifications(&sources(&doubled)).is_err());
}

#[test]
fn the_marker_takes_no_argument_and_is_not_a_clause() {
    for broken in [
        SPEC.replace(
            "requires resource_body_checked;",
            "requires resource_body_checked(chunk);",
        ),
        SPEC.replace(
            "    requires resource_body_checked;\n",
            "    ensures normal resource_body_checked();\n",
        ),
    ] {
        assert!(
            load_resource_contract_sources(&sources(&broken)).is_err(),
            "{broken}"
        );
    }
}

#[test]
fn an_environment_with_no_retained_text_declares_no_specification() {
    let marked = load_resource_contract_sources(&sources(SPEC)).unwrap();
    let mut bare = qcore_schemas::contract::ResourceContracts::default();
    for (name, contract) in marked.functions() {
        bare.insert_contract(name, contract.clone());
    }
    assert!(declared_checked_specifications(&bare).unwrap().is_empty());
}

#[test]
fn the_marker_composes_with_the_revision_and_review_markers() {
    let bare = load_resource_contract_sources(&sources(SPEC)).unwrap();
    // Card W45-CONTRACT-REVISION-MODE's revision marker, before or after: the
    // declaration loads the same modes and stays a specification.
    for revision in [
        "    requires resource_revision(flexible);\n    requires resource_body_checked;\n",
        "    requires resource_body_checked;\n    requires resource_revision(pinned);\n",
    ] {
        let text = SPEC.replace("    requires resource_body_checked;\n", revision);
        let loaded = load_resource_contract_sources(&sources(&text)).unwrap();
        assert!(!loaded.declared_revisions().is_empty(), "{text}");
        assert_eq!(
            loaded
                .contract("w_make")
                .map(|contract| &contract.parameters),
            bare.contract("w_make").map(|contract| &contract.parameters),
            "{text}"
        );
        assert_eq!(
            checked_specifications(&sources(&text)).unwrap(),
            ["w_make".to_owned()].into_iter().collect(),
            "{text}"
        );
    }
    // A review marker withholds the declaration: it is still read as marked,
    // but it is not in the function map, so no consumer applies it.
    let held = SPEC.replace(
        "    requires resource_body_checked;\n",
        "    requires resource_body_checked;\n    requires resource_review_pending(\"body unread\");\n",
    );
    let loaded = load_resource_contract_sources(&sources(&held)).unwrap();
    assert!(loaded.contract("w_make").is_none());
    assert!(
        declared_checked_specifications(&loaded)
            .unwrap()
            .contains("w_make")
    );
    // No marker may wrap it.
    for wrapped in [
        "    ensures normal resource_revision(resource_body_checked, pinned);\n",
        "    ensures normal resource_review_pending(resource_body_checked, \"r\");\n",
    ] {
        let text = SPEC.replace("    requires resource_body_checked;\n", wrapped);
        assert!(
            load_resource_contract_sources(&sources(&text)).is_err(),
            "{text}"
        );
    }
}
