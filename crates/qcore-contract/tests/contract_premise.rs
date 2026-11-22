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
//! `requires resource_premise(NAME, FORMULA);` (contract spec §C04.18; its
//! string statement retired by card SP5-GLOBAL-PROSE-PREMISE-BAN, `QF001`).
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
    requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});
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
        "    requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});\n",
        "",
    )))
    .unwrap();
    assert_eq!(with.functions(), without.functions());
    let premises = declared_premises(&with).unwrap();
    assert_eq!(
        premises["q_wait"],
        vec![DeclaredPremise {
            name: "served_queue".to_owned(),
            statement: "#op_callbacks |-> #takes(#held_on(#served)) == {}".to_owned(),
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
        "    requires resource_premise(second, #false);\n    ensures normal resource_borrowed(q);\n    ensures normal resource_borrowed(timeout);",
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
    let good = "requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});";
    for bad in [
        // A name that is not a lowercase identifier.
        "requires resource_premise(Served, #false);",
        "requires resource_premise(9queue, #false);",
        "requires resource_premise(served-queue, #false);",
        // A statement that is not a formula.
        "requires resource_premise(served_queue, x);",
        "requires resource_premise(served_queue);",
        "requires resource_premise(served_queue, #false, #false);",
        // A second premise of the same name.
        "requires resource_premise(served_queue, #false); requires resource_premise(served_queue, #false);",
        // Not a postcondition.
        "ensures normal resource_premise(served_queue, #false);",
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

/// Card SP1-SYMBOLIC-OPENSSL-PREMISES (C04.18.9): a premise may state a
/// formula. It changes no contract either, and its statement, which every
/// rendering prints, is the formula's canonical printing.
#[test]
fn a_formula_premise_is_admitted_with_its_canonical_statement() {
    let served = "    requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});\n";
    let formula = "    requires resource_premise(served_queue,\n        quant all #f in #stated:\n        #f |-> #inert);\n";
    let text = MODULE.replace(served, formula);
    assert_ne!(text, MODULE);
    let with = load_resource_contract_sources(&sources(&text)).unwrap();
    let without = load_resource_contract_sources(&sources(&MODULE.replace(served, ""))).unwrap();
    assert_eq!(with.functions(), without.functions());
    assert_eq!(
        declared_premises(&with).unwrap()["q_wait"],
        vec![DeclaredPremise {
            name: "served_queue".to_owned(),
            statement: "quant all #f in #stated: #f |-> #inert".to_owned(),
            origin: PremiseOrigin::Declared,
        }]
    );
    // The same formula spelled with other spacing and parentheses is the
    // same premise.
    let respelled = MODULE.replace(
        served,
        "    requires resource_premise(served_queue, (quant all #f in #stated : (#f |-> #inert)));\n",
    );
    assert_eq!(
        declaration_premises(&sources(&respelled)).unwrap()["q_wait"],
        declared_premises(&with).unwrap()["q_wait"]
    );
    // A set difference prints `\`: the premise's erasure (its legacy
    // hypothesis atom) holds the canonical formula, backslash included.
    let difference = MODULE.replace(
        served,
        "    requires resource_premise(served_queue, #slots(#args) \\ #stated |-> #inert);\n",
    );
    let loaded = load_resource_contract_sources(&sources(&difference)).unwrap();
    assert_eq!(
        declared_premises(&loaded).unwrap()["q_wait"][0].statement,
        "#slots(#args) \\ #stated |-> #inert"
    );
}

/// Fail-closed: a malformed formula, an unknown term, an ill-sorted
/// application or an unbound variable rejects the whole module, and so does
/// a formula quoted as a string; the premise is never dropped.
#[test]
fn every_malformed_formula_rejects_the_module() {
    let good = "requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});";
    for bad in [
        "requires resource_premise(served_queue, #unknown |-> #inert);",
        "requires resource_premise(served_queue, #freed |-> #inert);",
        "requires resource_premise(served_queue, #x |-> #inert);",
        "requires resource_premise(served_queue, quant all #o in #created: #ex_data(#class(#x)) |-> #inert);",
        "requires resource_premise(served_queue, #stated);",
        "requires resource_premise(served_queue, #stated |-> #inert &&);",
        "requires resource_premise(served_queue, #args | #created \\ #freed);",
        "requires resource_premise(served_queue, #slots(#args) - #stated |-> #inert);",
        "requires resource_premise(served_queue, #ex_data(NOT_A_CLASS) |-> #inert);",
        "requires resource_premise(served_queue, \"#stated |-> #inert\");",
        "requires resource_premise(served_queue, #stated |-> #inert); requires resource_premise(served_queue, #stated |-> #inert);",
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

/// Card SP5-GLOBAL-PROSE-PREMISE-BAN (C04.18): the retired string statement
/// rejects the whole module by name (`QF001`), whatever the string, on the
/// resource loader, the premise readers and the structural parser alike;
/// the premise is never dropped and the string never read.
#[test]
fn a_string_statement_rejects_the_module_by_name() {
    let good = "requires resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {});";
    for bad in [
        "requires resource_premise(served_queue, \"a callback the call runs takes over no reference the caller holds on q\");",
        "requires resource_premise(served_queue, \"#op_callbacks |-> #takes(#held_on(#served)) == {}\");",
        "requires resource_premise(served_queue, \"  \");",
        "requires resource_premise(served_queue, \"x\", \"y\");",
        "requires resource_premise(served_queue, #false && \"x\");",
    ] {
        let text = MODULE.replace(good, bad);
        assert_ne!(text, MODULE);
        for refusal in [
            load_resource_contract_sources(&sources(&text))
                .err()
                .map(|e| e.to_string()),
            declaration_premises(&sources(&text)).err(),
            qcore_frontend::frontend::parse_source_program(&text)
                .err()
                .map(|e| e.to_string()),
        ] {
            let refusal = refusal.unwrap_or_else(|| panic!("{bad} must reject the module"));
            assert!(
                refusal.contains(qcore_frontend::premise_formula::PROSE_PREMISE),
                "{bad}: {refusal}"
            );
        }
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
    requires resource_premise(no_plugin, quant all #f in #loaded_registered: #false);
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
                statement: "quant all #f in #loaded_registered: #false".into(),
                origin: PremiseOrigin::Declared,
            }]
        )])
    );
    // The model module stays outside the resource vocabulary.
    assert!(load_resource_contract_sources(&sources(MODEL)).is_err());
    let good = "requires resource_premise(no_plugin, quant all #f in #loaded_registered: #false);";
    // Without the premise, nothing is read.
    let plain = MODEL.replace(&format!("    {good}\n"), "");
    assert_ne!(plain, MODEL);
    assert!(module_premises(&plain).expect("reads").is_empty());
    for bad in [
        "requires resource_premise(No_plugin, #false);",
        "requires resource_premise(no_plugin, x);",
        "requires resource_premise(no_plugin, \"no library outside the run registers a callback\");",
        "requires resource_premise(no_plugin);",
        "requires resource_premise(no_plugin, #false, #false);",
        "ensures normal resource_premise(no_plugin, #false);",
        "quant normal resource_premise(no_plugin, #false);",
        "requires resource_premise(no_plugin, #false); requires resource_premise(no_plugin, #false);",
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

#[path = "../../../tests/support/legacy_prose_premise.rs"]
mod legacy_prose_premise;
#[path = "../../../tests/support/tracked_files.rs"]
mod tracked_files;

/// Card SP5-GLOBAL-PROSE-PREMISE-BAN: the legacy reader of a retired string
/// premise (`tests/support/legacy_prose_premise.rs`) is test code only.
/// Every tracked Rust file that names it lies under a `tests` directory (a
/// test target or the shared test support), so no crate's source, and no
/// production load route, can reach it.
#[test]
fn the_legacy_prose_reader_is_confined_to_test_code() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut naming = Vec::new();
    for path in tracked_files::tracked_files(&root) {
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        if !text.contains("legacy_prose_premise") {
            continue;
        }
        let relative = path.strip_prefix(&root).unwrap_or(&path).to_owned();
        assert!(
            relative.components().any(|c| c.as_os_str() == "tests"),
            "{} names the legacy prose reader outside test code",
            relative.display()
        );
        naming.push(relative);
    }
    assert!(
        !naming.is_empty(),
        "the census found no test reading the legacy reader"
    );
}

/// Card SP5-GLOBAL-PROSE-PREMISE-BAN: every formula the legacy reader writes
/// is well-formed and is its own canonical printing, so a lifted text states
/// exactly the statement the converted module states; every prose it reads
/// is refused by the format.
#[test]
fn every_recorded_statement_lifts_to_a_canonical_formula() {
    let statements = legacy_prose_premise::statements();
    assert!(!statements.is_empty());
    for (name, prose, formula) in statements {
        let read = qcore_frontend::premise_formula::read_source(&formula)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(read.canonical, formula, "{name}");
        let clause = format!("resource_premise({name}, \"{prose}\")");
        let tokens = qcore_frontend::lexer::lex(&clause)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .filter(|kind| *kind != qcore_frontend::lexer::TokenKind::Eof)
            .collect::<Vec<_>>();
        assert!(
            qcore_frontend::premise_formula::premise_clause(&tokens)
                .is_err_and(|e| e.starts_with(qcore_frontend::premise_formula::PROSE_PREMISE)),
            "{name}"
        );
        assert_eq!(
            legacy_prose_premise::lift(&clause),
            format!("resource_premise({name}, {formula})")
        );
    }
}
