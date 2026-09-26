//! Card W45-FB-BACKLOG-CUSTODY: the checked-specification marker
//! `requires resource_body_checked;` (contract spec §C06.2).
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
        SPEC.replace("requires resource_body_checked;", "requires resource_body_checked(chunk);"),
        SPEC.replace(
            "    requires resource_body_checked;\n",
            "    ensures normal resource_body_checked();\n",
        ),
    ] {
        assert!(load_resource_contract_sources(&sources(&broken)).is_err(), "{broken}");
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
