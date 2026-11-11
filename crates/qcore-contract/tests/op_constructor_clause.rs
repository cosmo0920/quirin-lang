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

//! Card OP-CONSTRUCTOR-DECLARATIONS: a constructor's result state (contract
//! specification §C04.19.4) of resource vocabulary version 1, read
//! total-or-reject (grammar G05), in the spellings of the user's decisions of
//! 2026-10-14: the four-operand `resource_discriminant_set`, the member
//! registration `resource_callback_member_registered`, and the `quant` state
//! clause `quant live #result: TAG.FIELD |-> #absent;` with its two levels
//! (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: `#absent`, the value keyword).

use std::collections::BTreeMap;

use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_frontend::lexer::MAPS_TO;
use qcore_schemas::callback::CallbackMember;
use qcore_schemas::constructed_state::{
    ConstructedState, DiscriminantSet, MemberState, MemberValue,
};
use qcore_schemas::contract::ResourceContracts;

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "op_constructors.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

fn op_cb() -> CallbackMember {
    CallbackMember {
        tag: "rd_kafka_op_s".to_owned(),
        path: "rko_op_cb".to_owned(),
    }
}

fn op_type() -> CallbackMember {
    CallbackMember {
        tag: "rd_kafka_op_s".to_owned(),
        path: "rko_type".to_owned(),
    }
}

/// `rd_kafka_op_new0` with its result state written as `STATE`.
fn new0(state: &str) -> String {
    format!(
        "
extern fn op_new0(source: ptr<const, u8>, type: i32) -> ptr<mut, u8>
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(source);
    ensures normal resource_borrowed(type);
    ensures normal resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 0);
{state}
    ensures normal resource_return_owned(nonnull);
}};
"
    )
}

const NEW_CB: &str = "
extern fn op_new_cb(rk: ptr<mut, u8>, type: i32, cb: fnptr) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(type);
    ensures normal resource_borrowed(cb);
    ensures normal resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 536870912);
    ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, cb);
    ensures normal resource_return_owned(nonnull);
};
";

fn expected_new0() -> ConstructedState {
    ConstructedState {
        live: true,
        discriminant: Some(DiscriminantSet {
            member: op_type(),
            parameter: 1,
            flags: 0,
        }),
        members: vec![MemberState {
            member: op_cb(),
            value: MemberValue::Absent,
        }],
    }
}

#[test]
fn the_clauses_load_withheld_with_their_state() {
    let shorthand = new0(&format!(
        "    quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} #absent;"
    ));
    let contracts = load(&format!("{shorthand}{NEW_CB}")).expect("loads");
    assert!(contracts.contract("op_new0").is_none());
    assert!(contracts.contract("op_new_cb").is_none());
    let declared = contracts.retaining_contract("op_new0").expect("withheld");
    assert_eq!(declared.constructed, expected_new0());
    let declared = contracts.retaining_contract("op_new_cb").expect("withheld");
    assert_eq!(
        declared.constructed,
        ConstructedState {
            live: false,
            discriminant: Some(DiscriminantSet {
                member: op_type(),
                parameter: 1,
                flags: 536_870_912,
            }),
            members: vec![MemberState {
                member: op_cb(),
                value: MemberValue::Callback { callback: 2 },
            }],
        }
    );
}

/// Rule 14e's two levels read identically: the shorthand, its two parts, and
/// both together (an agreeing overlap merges).
#[test]
fn both_levels_read_identically() {
    let shorthand = format!("    quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} #absent;");
    let parts = "    quant live #result;
    ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, #absent);";
    for state in [
        shorthand.clone(),
        parts.to_owned(),
        format!("{shorthand}\n{parts}"),
    ] {
        let contracts = load(&new0(&state)).expect("loads");
        assert_eq!(
            contracts
                .retaining_contract("op_new0")
                .expect("withheld")
                .constructed,
            expected_new0(),
            "{state}"
        );
    }
}

#[test]
fn every_misuse_rejects_the_module() {
    let rejects = |state: &str, needle: &str| {
        let error = load(&new0(state)).expect_err(state);
        assert!(error.contains(needle), "{state}: {error}");
    };
    // A bare `->` names the maps-to token.
    rejects(
        "    quant live #result: rd_kafka_op_s.rko_op_cb -> #absent;",
        "quant-live-maps-to",
    );
    rejects(
        "    quant live #result: rd_kafka_op_s.rko_op_cb -> #absent;",
        MAPS_TO,
    );
    // C member access on the left.
    rejects(
        &format!("    quant live #result: result->rko_op_cb {MAPS_TO} #absent;"),
        "quant-live-maps-to",
    );
    // An undotted member, another value, a parameter, a second value.
    rejects(
        &format!("    quant live #result: rko_op_cb {MAPS_TO} #absent;"),
        "quant-live-member",
    );
    rejects(
        &format!("    quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} NULL;"),
        "quant-live-value",
    );
    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (D1): the right of `|->` is
    // `absent` or a function designator, never a parameter of the
    // declaration (`source`); a name that is no parameter (`cb` here) is a
    // function designator and loads.
    rejects(
        &format!("    quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} source;"),
        "quant-live-value",
    );
    assert!(
        load(&new0(&format!(
            "    quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} cb;"
        )))
        .is_ok()
    );
    rejects("    quant live #source;", "quant-live-result-shape");
    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the bare `absent` rejects at
    // both levels, naming the keyword `#absent`.
    rejects(
        &format!("    quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} absent;"),
        "#absent",
    );
    rejects(
        "    ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, absent);",
        "#absent",
    );
    // Two states of one member.
    let two = NEW_CB.replace(
        "    ensures normal resource_return_owned(nonnull);",
        &format!(
            "    quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} #absent;
    ensures normal resource_return_owned(nonnull);"
        ),
    );
    assert!(load(&two).expect_err("two states").contains("two states"));
    // A registered callback that is not a function pointer.
    rejects(
        "    ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, source);",
        "`fnptr`",
    );
    // Another `quant` form in resource v1 still rejects.
    rejects("    quant deref #source;", "quantitative clause");
    // A discriminant on a non-integer parameter, or with a name for FLAGS.
    let wrong = new0("").replace(
        "resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 0)",
        "resource_discriminant_set(result, rd_kafka_op_s.rko_type, source, 0)",
    );
    assert!(
        load(&wrong)
            .expect_err("a pointer")
            .contains("integer parameter")
    );
    let wrong = new0("").replace(
        "resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 0)",
        "resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, RD_KAFKA_OP_CB)",
    );
    assert!(
        load(&wrong)
            .expect_err("a name")
            .contains("canonical decimal")
    );
    // An unbacked result: a nullable owned result is not live.
    let nullable = new0(&format!(
        "    quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} #absent;"
    ))
    .replace(
        "resource_return_owned(nonnull)",
        "resource_return_owned(nullable)",
    );
    assert!(
        load(&nullable)
            .expect_err("unbacked")
            .contains("quant-live-unbacked")
    );
}
