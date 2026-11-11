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

/// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Add
/// premise support"): a Quirin model contract (a `resource type` module no
/// resource view selects) states a named premise in the contract format
/// itself; `module_premises` reads it, and a malformed, repeated or
/// postcondition premise rejects the module at parse, in any vocabulary.
const MODEL: &str = "module contract;

resource type Owner {
    repr: ptr<mut, u8>;
    footprint: { queues = 1 };
    destructor: owner_destroy noexcept;
    copy: forbidden;
    move: allowed;
}

extern fn owner_destroy(
    queue: own<Owner> use one
) -> unit
contract {
    requires resource_premise(no_plugin, \"no library outside the run registers a callback\");
    requires queue_is_live;
    ensures normal queue_is_discharged;
    quant normal live_queues == old(live_queues) - 1;
};
";

#[test]
fn a_model_contract_states_a_premise_total_or_reject() {
    use qcore_contract::contract_loader::module_premises;
    let premises = module_premises(MODEL).expect("the model contract reads");
    assert_eq!(
        premises,
        BTreeMap::from([(
            "owner_destroy".to_owned(),
            vec![DeclaredPremise {
                name: "no_plugin".into(),
                statement: "no library outside the run registers a callback".into(),
                origin: PremiseOrigin::Declared,
            }]
        )])
    );
    // The model module stays outside the resource vocabulary.
    assert!(load_resource_contract_sources(&sources(MODEL)).is_err());
    let good = "requires resource_premise(no_plugin, \"no library outside the run registers a callback\");";
    // Without the premise, nothing is read.
    let plain = MODEL.replace(&format!("    {good}\n"), "");
    assert_ne!(plain, MODEL);
    assert!(module_premises(&plain).expect("reads").is_empty());
    for bad in [
        "requires resource_premise(No_plugin, \"x\");",
        "requires resource_premise(no_plugin, x);",
        "requires resource_premise(no_plugin, \"  \");",
        "requires resource_premise(no_plugin);",
        "requires resource_premise(no_plugin, \"x\", \"y\");",
        "ensures normal resource_premise(no_plugin, \"x\");",
        "quant normal resource_premise(no_plugin, \"x\");",
        "requires resource_premise(no_plugin, \"x\"); requires resource_premise(no_plugin, \"y\");",
    ] {
        let text = MODEL.replace(good, bad);
        assert_ne!(text, MODEL);
        assert!(
            module_premises(&text).is_err(),
            "{bad} must reject the module"
        );
        assert!(
            qcore_frontend::frontend::parse_source_program(&text).is_err(),
            "{bad}: the contract format itself rejects it"
        );
    }
}
