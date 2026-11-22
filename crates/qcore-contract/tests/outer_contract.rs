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

//! Card OUTER-CONTRACT (contract specification C04.20, grammar G05g, gateway
//! rule 31): the outer contract v1 loader is total-or-reject, and the other
//! vocabularies refuse its modules.
//!
//! Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS: a definition's statement is a
//! C04.18.9 formula; the string statement is refused by name (`QF004`) on
//! every route; the two shipped statements are the recorded formulas, and
//! the Fluent Bit statement's right conjunct is the librdkafka statement.

use std::collections::BTreeMap;

use qcore_contract::outer_contract::{
    EnvelopeLine, GuardOp, MemberEnvelope, load_outer_contract_sources,
};

const FLUENT_BIT: &str =
    include_str!("../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/outer.qv");

fn load(text: &str) -> Result<qcore_contract::outer_contract::OuterContracts, String> {
    load_outer_contract_sources(&BTreeMap::from([(
        "app:outer.qv".to_owned(),
        text.to_owned(),
    )]))
}

const DEFINITION: &str = r#"module contract;
extern fn load_plugin(path: ptr<mut, u8>, config: ptr<mut, u8>) -> i32
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] app_plugin, #outer_code |-> #stores_none(#flb_core_owned);
    quant #outer where #config in #loaded [[runtime]] when #result == 0;
    quant #outer #loaded: app_instance.p |-> #kept;
    quant #outer #loaded: app_collector.cb |-> #registered;
    quant #outer #loaded: #premise app_closed;
};
"#;

#[test]
fn the_fluent_bit_module_loads() {
    let contracts = load(FLUENT_BIT).expect("the Fluent Bit outer module loads");
    assert_eq!(
        contracts.loaders.keys().collect::<Vec<_>>(),
        ["flb_plugin_load", "flb_plugin_proxy_create"]
    );
    let contract = &contracts.contracts["flb_external_plugin"];
    assert_eq!(contract.defined_by, "flb_plugin_load");
    assert_eq!(contract.loaders.len(), 2);
    let keys = contract
        .envelope
        .iter()
        .map(EnvelopeLine::key)
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            "flb_input_instance.p",
            "flb_output_instance.p",
            "flb_filter_instance.p",
            "flb_processor_instance.p",
            "rd_kafka_conf_s.interceptors.on_conf_set",
            "rd_kafka_s.rk_conf.interceptors.on_conf_set",
            "rd_kafka_conf_s.interceptors.on_conf_dup",
            "rd_kafka_s.rk_conf.interceptors.on_conf_dup",
            "rd_kafka_conf_s.interceptors.on_conf_destroy",
            "rd_kafka_s.rk_conf.interceptors.on_conf_destroy",
            "rd_kafka_conf_s.interceptors.on_new",
            "rd_kafka_s.rk_conf.interceptors.on_new",
            "rd_kafka_conf_s.interceptors.on_destroy",
            "rd_kafka_s.rk_conf.interceptors.on_destroy",
            "rd_kafka_conf_s.interceptors.on_send",
            "rd_kafka_s.rk_conf.interceptors.on_send",
            "rd_kafka_conf_s.interceptors.on_acknowledgement",
            "rd_kafka_s.rk_conf.interceptors.on_acknowledgement",
            "rd_kafka_conf_s.interceptors.on_consume",
            "rd_kafka_s.rk_conf.interceptors.on_consume",
            "rd_kafka_conf_s.interceptors.on_commit",
            "rd_kafka_s.rk_conf.interceptors.on_commit",
            "rd_kafka_conf_s.interceptors.on_request_sent",
            "rd_kafka_s.rk_conf.interceptors.on_request_sent",
            "rd_kafka_conf_s.interceptors.on_response_received",
            "rd_kafka_s.rk_conf.interceptors.on_response_received",
            "rd_kafka_conf_s.interceptors.on_thread_start",
            "rd_kafka_s.rk_conf.interceptors.on_thread_start",
            "rd_kafka_conf_s.interceptors.on_thread_exit",
            "rd_kafka_s.rk_conf.interceptors.on_thread_exit",
            "rd_kafka_conf_s.interceptors.on_broker_state_change",
            "rd_kafka_s.rk_conf.interceptors.on_broker_state_change",
            "rd_kafka_conf_s.background_event_cb",
            "rd_kafka_conf_s.closesocket_cb",
            "rd_kafka_conf_s.connect_cb",
            "rd_kafka_conf_s.offset_commit_cb",
            "rd_kafka_conf_s.open_cb",
            "rd_kafka_conf_s.resolve_cb",
            "rd_kafka_conf_s.socket_cb",
            "rd_kafka_conf_s.ssl.cert_verify_cb",
            "rd_kafka_conf_s.consume_cb",
            "rd_kafka_conf_s.dr_cb",
            "rd_kafka_conf_s.dr_msg_cb",
            "rd_kafka_conf_s.error_cb",
            "rd_kafka_conf_s.log_cb",
            "rd_kafka_conf_s.rebalance_cb",
            "rd_kafka_conf_s.sasl.oauthbearer.token_refresh_cb",
            "rd_kafka_conf_s.stats_cb",
            "rd_kafka_conf_s.throttle_cb",
            "rd_kafka_s.rk_conf.background_event_cb",
            "rd_kafka_s.rk_conf.closesocket_cb",
            "rd_kafka_s.rk_conf.connect_cb",
            "rd_kafka_s.rk_conf.offset_commit_cb",
            "rd_kafka_s.rk_conf.open_cb",
            "rd_kafka_s.rk_conf.resolve_cb",
            "rd_kafka_s.rk_conf.socket_cb",
            "rd_kafka_s.rk_conf.ssl.cert_verify_cb",
            "rd_kafka_s.rk_conf.consume_cb",
            "rd_kafka_s.rk_conf.dr_cb",
            "rd_kafka_s.rk_conf.dr_msg_cb",
            "rd_kafka_s.rk_conf.error_cb",
            "rd_kafka_s.rk_conf.log_cb",
            "rd_kafka_s.rk_conf.rebalance_cb",
            "rd_kafka_s.rk_conf.sasl.oauthbearer.token_refresh_cb",
            "rd_kafka_s.rk_conf.stats_cb",
            "rd_kafka_s.rk_conf.throttle_cb",
            "rd_kafka_op_s.rko_op_cb",
            "rd_kafka_op_s.rko_type",
            "rd_kafka_broker_s.rkb_rk",
            "rd_kafka_topic_s.rkt_rk",
            "rd_kafka_topic_s.rkt_topic",
            "rd_kafka_toppar_s.rktp_rkt",
            "rd_kafka_cgrp_s.rkcg_rk",
            "rd_kafka_transport_s.rktrans_rkb",
            "premise:rk_plugin_callback_closure",
            "premise:assignor_callback",
            "premise:broker_monitor_callback",
            "premise:conf_property_writer",
            "premise:share_ack_batches_nonempty",
            "premise:served_queue",
            "premise:cjson_list_invariant",
            "premise:cjson_allocator_hooks"
        ]
    );
    let load = &contracts.loaders["flb_plugin_load"];
    assert_eq!(load.runtime[0].binder, "config");
    assert_eq!(load.runtime[0].index, 2);
    assert_eq!(
        load.runtime[0].guard.as_ref().map(|g| (g.op, g.value)),
        Some((GuardOp::Equal, 0))
    );
    let proxy = &contracts.loaders["flb_plugin_proxy_create"];
    assert!(proxy.statement.is_none() && proxy.envelope.is_empty());
    assert!(proxy.runtime[0].guard.is_none());
    assert!(contracts.in_force("flb_external_plugin"));
    let identity = contracts.identity("flb_external_plugin").expect("identity");
    assert!(identity.starts_with("flb_external_plugin@") && identity.len() == 20 + 64);
}

#[test]
fn every_form_reads_and_the_digest_covers_each_line() {
    let contracts = load(DEFINITION).expect("loads");
    let contract = &contracts.contracts["app_plugin"];
    assert_eq!(
        contract.envelope,
        vec![
            EnvelopeLine::Member {
                tag: "app_instance".into(),
                path: "p".into(),
                envelope: MemberEnvelope::Kept
            },
            EnvelopeLine::Member {
                tag: "app_collector".into(),
                path: "cb".into(),
                envelope: MemberEnvelope::Registered
            },
            EnvelopeLine::Premise {
                name: "app_closed".into()
            },
        ]
    );
    let before = contracts.digest("app_plugin").unwrap();
    let changed = load(&DEFINITION.replace("app_instance.p |-> #kept", "app_instance.q |-> #kept"))
        .expect("loads");
    assert_ne!(changed.digest("app_plugin").unwrap(), before);
    let mut held = contracts.clone();
    held.hold_for_pin_mismatch("load_plugin", "contract-changed", "test")
        .unwrap();
    assert!(!held.in_force("app_plugin"));
}

#[test]
fn a_citation_needs_a_definition_and_a_definition_is_unique() {
    let citation = r#"module contract;
extern fn other_loader(config: ptr<mut, u8>) -> unit
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] app_plugin;
    quant #outer where #config in #loaded [[runtime]];
};
"#;
    let error = load(citation).unwrap_err();
    assert!(error.contains("outer-contract-undefined"), "{error}");
    let both = load_outer_contract_sources(&BTreeMap::from([
        ("app:a.qv".to_owned(), DEFINITION.to_owned()),
        ("app:b.qv".to_owned(), citation.to_owned()),
    ]))
    .expect("a definition and a citation load together");
    assert_eq!(both.contracts["app_plugin"].loaders.len(), 2);
    let twice = load_outer_contract_sources(&BTreeMap::from([
        ("app:a.qv".to_owned(), DEFINITION.to_owned()),
        (
            "app:b.qv".to_owned(),
            DEFINITION.replace("load_plugin", "load_again"),
        ),
    ]))
    .unwrap_err();
    assert!(twice.contains("outer-contract-defined-twice"), "{twice}");
}

#[test]
fn malformed_forms_reject_the_module_with_a_named_diagnostic() {
    let cases: [(&str, &str, &str); 17] = [
        ("requires outer_contract_v1;\n", "", "exactly one"),
        (
            "    quant #outer where #config in #loaded [[runtime]] when #result == 0;\n",
            "",
            "outer-runtime-missing",
        ),
        (
            "#loaded [[runtime]]",
            "loaded [[runtime]]",
            "outer-loaded-keyword",
        ),
        ("#config in", "#path2 in", "outer-binder-parameter"),
        ("[[runtime]] when", "[[stable]] when", "outer-runtime-shape"),
        ("|-> #kept", "|-> kept", "outer-envelope-keyword"),
        ("|-> #kept", "|-> #moved", "outer-envelope-value"),
        (
            "app_instance.p |->",
            "app_instance |->",
            "outer-envelope-member",
        ),
        (
            "app_plugin, #outer_code",
            "App_Plugin, #outer_code",
            "outer-contract-shape",
        ),
        // Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS: a formula the
        // judgment refuses rejects the module (at parse, every route).
        (
            "#stores_none(#flb_core_owned)",
            "#stores_none(#outer_code)",
            "premise-formula: ill-sorted",
        ),
        // The retired string statement, refused by name.
        (
            "#outer_code |-> #stores_none(#flb_core_owned)",
            "\"a loaded plugin stores into no member the core owns\"",
            "QF004",
        ),
        (
            "    quant #outer #loaded: #premise app_closed;\n",
            "    quant #outer #loaded: #premise app_closed;\n    quant #outer #loaded: #premise app_closed;\n",
            "outer-envelope-repeated",
        ),
        (
            "    requires outer_contract_v1;\n",
            "    requires outer_contract_v1;\n    requires resource_contract_v1;\n",
            "outer-clause-foreign",
        ),
        (
            "    requires outer_contract_v1;\n",
            "    requires outer_contract_v1;\n    ensures normal resource_borrowed(path);\n",
            "outer-clause-foreign",
        ),
        (
            "    quant #outer [[contract]] app_plugin, #outer_code |-> #stores_none(#flb_core_owned);\n",
            "",
            "outer-contract-missing",
        ),
        ("#outer where", "#inner where", "exit selector"),
        ("-> i32", "-> ptr<mut, u8>", "outer-guard-result"),
    ];
    for (from, to, expected) in cases {
        let text = DEFINITION.replacen(from, to, 1);
        assert_ne!(text, DEFINITION, "case `{from}` changed nothing");
        let error = load(&text).expect_err(expected);
        assert!(error.contains(expected), "case `{from}` -> `{to}`: {error}");
    }
    let citation_envelope = r#"module contract;
extern fn other_loader(config: ptr<mut, u8>) -> unit
contract {
    requires outer_contract_v1;
    quant #outer [[contract]] app_plugin;
    quant #outer where #config in #loaded [[runtime]];
    quant #outer #loaded: app_instance.p |-> #kept;
};
"#;
    let error = load(citation_envelope).unwrap_err();
    assert!(error.contains("outer-envelope-citation"), "{error}");
    let scalar_binder = DEFINITION.replace("config: ptr<mut, u8>", "config: i32");
    let error = load(&scalar_binder).unwrap_err();
    assert!(error.contains("outer-binder-pointer"), "{error}");
}

#[test]
fn the_resource_and_bounds_loaders_refuse_an_outer_module() {
    let sources = BTreeMap::from([("fluent_bit:outer.qv".to_owned(), FLUENT_BIT.to_owned())]);
    assert!(qcore_contract::bounds_contract::load_bounds_contract_sources(&sources).is_err());
    assert!(qcore_contract::contract_loader::load_resource_contract_sources(&sources).is_err());
}

const LIBRDKAFKA: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/outer.qv");
const MIGRATION: &str =
    include_str!("../../../docs/reports/sp8-outer-contract-symbolic-statements/migration.json");

fn recorded() -> Vec<(String, String, String)> {
    let record: serde_json::Value = serde_json::from_str(MIGRATION).unwrap();
    record["outer_statements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let field = |k: &str| s[k].as_str().unwrap().to_owned();
            (field("name"), field("prose"), field("formula"))
        })
        .collect()
}

/// Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS: each shipped definition
/// states exactly its recorded formula, printed canonically; the Fluent Bit
/// statement is its own conjunct `&&` the librdkafka statement (the relation
/// `QCore.PremiseFormula.Outer.fb_entails_rk` proves), and both cite the
/// registered candidate inventory.
#[test]
fn the_shipped_statements_are_the_recorded_formulas() {
    use qcore_frontend::premise_formula::{Op, Raw, read_source};
    let librdkafka = load_outer_contract_sources(&BTreeMap::from([(
        "librdkafka:outer.qv".to_owned(),
        LIBRDKAFKA.to_owned(),
    )]))
    .unwrap();
    let fluent_bit = load(FLUENT_BIT).unwrap();
    let statements = BTreeMap::from([
        (
            "rk_plugin_library".to_owned(),
            librdkafka.contracts["rk_plugin_library"].statement.clone(),
        ),
        (
            "flb_external_plugin".to_owned(),
            fluent_bit.contracts["flb_external_plugin"]
                .statement
                .clone(),
        ),
    ]);
    let recorded = recorded();
    assert_eq!(recorded.len(), 2);
    for (name, _, formula) in &recorded {
        assert_eq!(&statements[name], formula, "{name}");
        assert_eq!(&read_source(formula).unwrap().canonical, formula, "{name}");
        assert_eq!(
            qcore_contract::outer_contract::candidate_inventories(formula)
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![
                qcore_contract::outer_contract::candidate_digest(
                    qcore_contract::outer_contract::CANDIDATE_INVENTORIES[0]
                )
                .unwrap()
            ],
            "{name}"
        );
    }
    let rk = read_source(&statements["rk_plugin_library"]).unwrap().raw;
    let fb = read_source(&statements["flb_external_plugin"]).unwrap().raw;
    let Raw::Bin(Op::Conj, _, right) = fb else {
        panic!("the Fluent Bit statement is a conjunction");
    };
    assert_eq!(*right, rk);
}

/// Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS: the retired string statement
/// is refused by name (`QF004`) on the structural parser every route reads,
/// by the outer loader, and by the resource and bounds loaders; each
/// recorded sentence is refused as it stood.
#[test]
fn a_string_statement_is_refused_on_every_route() {
    for (name, prose, formula) in recorded() {
        let module = if name == "rk_plugin_library" {
            LIBRDKAFKA
        } else {
            FLUENT_BIT
        };
        let retired = module.replace(&formula, &format!("\"{prose}\""));
        assert_ne!(retired, module, "{name}");
        let parse = qcore_frontend::frontend::parse_source_program(&retired).unwrap_err();
        assert!(parse.to_string().contains("QF004"), "{name}: {parse}");
        let sources = BTreeMap::from([("p:outer.qv".to_owned(), retired)]);
        let outer = load_outer_contract_sources(&sources).unwrap_err();
        assert!(outer.contains("QF004"), "{name}: {outer}");
        let resource =
            qcore_contract::contract_loader::load_resource_contract_sources(&sources).unwrap_err();
        assert!(resource.to_string().contains("QF004"), "{name}: {resource}");
        let bounds =
            qcore_contract::bounds_contract::load_bounds_contract_sources(&sources).unwrap_err();
        assert!(bounds.to_string().contains("QF004"), "{name}: {bounds}");
    }
    for statement in ["\"   \"", "#outer_code |-> #balanced && \"and more\""] {
        let text = DEFINITION.replace("#outer_code |-> #stores_none(#flb_core_owned)", statement);
        let error = load(&text).unwrap_err();
        assert!(error.contains("QF004"), "{statement}: {error}");
    }
}

#[path = "../../../tests/support/legacy_prose_premise.rs"]
mod legacy_prose_premise;

/// Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS: a frozen text stating a
/// retired outer statement is read only through the test-side legacy reader,
/// which writes each recorded sentence as its formula and nothing else: the
/// lifted text is the current module, byte for byte, and an unrecorded
/// sentence stays a string the loader refuses.
#[test]
fn the_legacy_reader_lifts_exactly_the_recorded_statements() {
    assert_eq!(legacy_prose_premise::outer_statements(), recorded());
    for (name, prose, formula) in recorded() {
        let module = if name == "rk_plugin_library" {
            LIBRDKAFKA
        } else {
            FLUENT_BIT
        };
        let retired = module.replace(&formula, &format!("\"{prose}\""));
        assert_eq!(legacy_prose_premise::lift(&retired), module, "{name}");
    }
    let unrecorded = DEFINITION.replace(
        "#outer_code |-> #stores_none(#flb_core_owned)",
        "\"a loaded plugin stores into no member the core owns\"",
    );
    let lifted = legacy_prose_premise::lift(&unrecorded);
    assert_eq!(lifted, unrecorded);
    assert!(load(&lifted).unwrap_err().contains("QF004"));
}
