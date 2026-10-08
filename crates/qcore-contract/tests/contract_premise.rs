//! Card **VERDICT-CONTRACT-DEPENDENCIES**: the named premise
//! `requires resource_premise(NAME, "STATEMENT");` (contract spec §C04.18).
//!
//! The loader admits it as a contract-level requirement, total-or-reject, and
//! records nothing new in the environment: the function map is the map the
//! declaration has without it, and the premises are rederived from the
//! retained texts (`declared_premises`), with each literal-argument premise
//! read as one as well. The clause is part of the declaration's text, so its
//! contract digest (and therefore its pin) covers it.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{
    DeclaredPremise, LITERAL_PREMISE_NAME, PREMISE_CLAUSE, PremiseOrigin, declaration_premises,
    declared_premises, load_resource_contract_sources,
};

const MODULE: &str = "module contract;

extern fn q_wait(q: ptr<mut, u8>, timeout: i32) -> i32
contract {
    requires resource_contract_v1;
    requires resource_premise(served_queue, \"a callback the call runs takes over no reference the caller holds on q\");
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(timeout);
    ensures normal resource_return_none();
};

extern fn q_destroy(q: ptr<mut, u8>, do_free: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(do_free, 1);
    ensures normal resource_released(q, nonnull);
    ensures normal resource_borrowed(do_free);
    ensures normal resource_return_none();
};
";

fn sources(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("q.qv".to_owned(), text.to_owned())])
}

#[test]
fn a_premise_is_admitted_and_changes_no_contract() {
    assert_eq!(PREMISE_CLAUSE, "resource_premise");
    let with = load_resource_contract_sources(&sources(MODULE)).unwrap();
    let without = load_resource_contract_sources(&sources(&MODULE.replace(
        "    requires resource_premise(served_queue, \"a callback the call runs takes over no reference the caller holds on q\");\n",
        "",
    )))
    .unwrap();
    assert_eq!(with.functions(), without.functions());
    let premises = declared_premises(&with).unwrap();
    assert_eq!(
        premises["q_wait"],
        vec![DeclaredPremise {
            name: "served_queue".to_owned(),
            statement: "a callback the call runs takes over no reference the caller holds on q"
                .to_owned(),
            origin: PremiseOrigin::Declared,
        }]
    );
    // A literal-argument premise is reported as a premise too.
    assert_eq!(
        premises["q_destroy"],
        vec![DeclaredPremise {
            name: LITERAL_PREMISE_NAME.to_owned(),
            statement: "do_free = 1".to_owned(),
            origin: PremiseOrigin::LiteralArgument,
        }]
    );
    assert!(!declared_premises(&without).unwrap().contains_key("q_wait"));
}

#[test]
fn two_premises_of_different_names_are_kept_in_clause_order() {
    let text = MODULE.replace(
        "    ensures normal resource_borrowed(q);\n    ensures normal resource_borrowed(timeout);",
        "    requires resource_premise(second, \"another\");\n    ensures normal resource_borrowed(q);\n    ensures normal resource_borrowed(timeout);",
    );
    let premises = declaration_premises(&sources(&text)).unwrap();
    let names = premises["q_wait"]
        .iter()
        .map(|premise| premise.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["served_queue", "second"]);
}

#[test]
fn every_malformed_premise_rejects_the_module() {
    let good = "requires resource_premise(served_queue, \"a callback the call runs takes over no reference the caller holds on q\");";
    for bad in [
        // A name that is not a lowercase identifier.
        "requires resource_premise(Served, \"x\");",
        "requires resource_premise(9queue, \"x\");",
        "requires resource_premise(served-queue, \"x\");",
        // A statement that is not one plain, non-blank string.
        "requires resource_premise(served_queue, \"  \");",
        "requires resource_premise(served_queue, \"a\\nb\");",
        "requires resource_premise(served_queue, x);",
        "requires resource_premise(served_queue);",
        "requires resource_premise(served_queue, \"x\", \"y\");",
        // A second premise of the same name.
        "requires resource_premise(served_queue, \"x\"); requires resource_premise(served_queue, \"y\");",
        // Not a postcondition.
        "ensures normal resource_premise(served_queue, \"x\");",
    ] {
        let text = MODULE.replace(good, bad);
        assert_ne!(text, MODULE);
        assert!(
            load_resource_contract_sources(&sources(&text)).is_err(),
            "{bad} must reject the module"
        );
        assert!(declaration_premises(&sources(&text)).is_err(), "{bad}");
    }
}

#[test]
fn an_environment_without_sources_declares_no_premise() {
    let contracts = qcore_schemas::contract::ResourceContracts::new([(
        "q_wait",
        qcore_schemas::contract::ResourceApiKind::Neutral,
    )]);
    assert!(declared_premises(&contracts).unwrap().is_empty());
}
