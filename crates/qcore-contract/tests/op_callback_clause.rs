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

//! Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the `supplied_owned` actual, the
//! member invocation and the type-switched release (contract specification
//! §C04.19.1-§C04.19.3) of resource vocabulary version 1, read
//! total-or-reject (grammar G05), in the spellings of the user's decision of
//! 2026-10-14.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{check_source_assumptions, load_resource_contract_sources};
use qcore_schemas::arm_release::{ArmClause, ArmEffect, ArmGuard, Discriminant};
use qcore_schemas::callback::{CallbackActual, CallbackClause, CallbackMember, MemberObject};
use qcore_schemas::contract::ResourceContracts;

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "op_callbacks.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

const DECLARATIONS: &str = "
extern fn serve(q: ptr<mut, u8>, cb: fnptr, opaque: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_callback_invoked(cb, _, supplied_owned, opaque);
    ensures normal resource_callback_member_invoked(supplied_owned, op_s.rko_op_cb, _, _, supplied_owned);
    ensures normal resource_return_none();
};

extern fn op_destroy(rko: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_discriminant(rko, rko_type, 1610612736);
    ensures normal resource_released(rko, nonnull);
    ensures normal resource_callback_member_invoked(rko, op_s.rko_u.leaders.cb, _, _, rko);
    ensures normal resource_arm_released(rko, rko_u.offset_commit.partitions) when rko_type == 6;
    ensures normal resource_arm_released(rko, rko_u.offset_commit.reason) when rko_type == 6;
    ensures normal resource_arm_empty(rko) when rko_type == 2;
    ensures normal resource_arm_empty(rko) otherwise;
    ensures normal resource_return_none();
};
";

#[test]
fn the_clauses_load_withheld() {
    let contracts = load(DECLARATIONS).expect("loads");
    assert!(contracts.contract("serve").is_none());
    assert!(contracts.contract("op_destroy").is_none());
    let serve = contracts.retaining_contract("serve").expect("withheld");
    assert_eq!(
        serve.callbacks,
        [
            CallbackClause::Invoked {
                callback: 1,
                actuals: vec![
                    CallbackActual::Supplied,
                    CallbackActual::SuppliedOwned,
                    CallbackActual::Parameter { index: 2 }
                ],
            },
            CallbackClause::MemberInvoked {
                object: MemberObject::SuppliedOwned,
                member: CallbackMember {
                    tag: "op_s".into(),
                    path: "rko_op_cb".into()
                },
                actuals: vec![
                    CallbackActual::Supplied,
                    CallbackActual::Supplied,
                    CallbackActual::SuppliedOwned
                ],
            },
        ]
    );
    let destroy = contracts
        .retaining_contract("op_destroy")
        .expect("withheld");
    assert_eq!(
        destroy.discriminant,
        Some(Discriminant {
            parameter: 0,
            field: "rko_type".into(),
            mask: 1_610_612_736
        })
    );
    assert_eq!(
        destroy.arms,
        [
            ArmClause {
                guard: ArmGuard::Equals { value: 6 },
                effect: ArmEffect::Released {
                    path: "rko_u.offset_commit.partitions".into()
                },
            },
            ArmClause {
                guard: ArmGuard::Equals { value: 6 },
                effect: ArmEffect::Released {
                    path: "rko_u.offset_commit.reason".into()
                },
            },
            ArmClause {
                guard: ArmGuard::Equals { value: 2 },
                effect: ArmEffect::Empty,
            },
            ArmClause {
                guard: ArmGuard::Otherwise,
                effect: ArmEffect::Empty,
            },
        ]
    );
    assert!(matches!(
        &destroy.callbacks[..],
        [CallbackClause::MemberInvoked {
            object: MemberObject::Parameter { index: 0 },
            member,
            ..
        }] if member.spelling() == "op_s.rko_u.leaders.cb"
    ));
    check_source_assumptions(&contracts).expect("the retained texts reload");
}

fn declaration(parameters: &str, clauses: &str) -> String {
    format!(
        "extern fn k({parameters}) -> unit\ncontract {{\n    requires resource_contract_v1;\n{clauses}    ensures normal resource_return_none();\n}};\n"
    )
}

#[test]
fn every_misplaced_clause_rejects_the_module() {
    let invoking = "f: fnptr, q: ptr<mut, u8>";
    let invoked =
        "    ensures normal resource_borrowed(f);\n    ensures normal resource_borrowed(q);\n";
    let released = "r: ptr<mut, u8>";
    let releasing = "    ensures normal resource_released(r, nonnull);\n";
    let cases = [
        // `supplied_owned` in a table dispatch.
        (
            "t: ptr<mut, u8>",
            "    ensures normal resource_borrowed(t);\n    ensures normal resource_callback_table(t, supplied_owned);\n".to_owned(),
        ),
        // Two `supplied_owned` in one list.
        (
            invoking,
            format!("{invoked}    ensures normal resource_callback_invoked(f, supplied_owned, supplied_owned);\n"),
        ),
        // A parameter named `supplied_owned`.
        (
            "f: fnptr, supplied_owned: ptr<mut, u8>",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_borrowed(supplied_owned);\n    ensures normal resource_callback_invoked(f, supplied_owned);\n".to_owned(),
        ),
        // A member with no record.
        (
            invoking,
            format!("{invoked}    ensures normal resource_callback_member_invoked(q, rko_op_cb, _);\n"),
        ),
        // A member of an object that is not a pointer.
        (
            "n: i32",
            "    ensures normal resource_borrowed(n);\n    ensures normal resource_callback_member_invoked(n, op_s.cb, _);\n".to_owned(),
        ),
        // A member of the supplied object not invoked with it.
        (
            invoking,
            format!("{invoked}    ensures normal resource_callback_member_invoked(supplied_owned, op_s.cb, _);\n"),
        ),
        // The same member invoked twice.
        (
            invoking,
            format!("{invoked}    ensures normal resource_callback_member_invoked(q, op_s.cb, _);\n    ensures normal resource_callback_member_invoked(q, op_s.cb, q);\n"),
        ),
        // An arm with no discriminant.
        (
            released,
            format!("{releasing}    ensures normal resource_arm_empty(r) otherwise;\n"),
        ),
        // A discriminant with no arm.
        (
            released,
            format!("    requires resource_discriminant(r, type, 0);\n{releasing}"),
        ),
        // An arm with no guard.
        (
            released,
            format!("    requires resource_discriminant(r, type, 0);\n{releasing}    ensures normal resource_arm_empty(r);\n"),
        ),
        // An arm value the mask clears.
        (
            released,
            format!("    requires resource_discriminant(r, type, 4);\n{releasing}    ensures normal resource_arm_empty(r) when type == 6;\n    ensures normal resource_arm_empty(r) otherwise;\n"),
        ),
        // A guard on another member.
        (
            released,
            format!("    requires resource_discriminant(r, type, 0);\n{releasing}    ensures normal resource_arm_empty(r) when kind == 1;\n    ensures normal resource_arm_empty(r) otherwise;\n"),
        ),
        // A guard that is not `==`.
        (
            released,
            format!("    requires resource_discriminant(r, type, 0);\n{releasing}    ensures normal resource_arm_empty(r) when type >= 1;\n    ensures normal resource_arm_empty(r) otherwise;\n"),
        ),
        // A name, not a decimal.
        (
            released,
            format!("    requires resource_discriminant(r, type, 0);\n{releasing}    ensures normal resource_arm_empty(r) when type == OP_FETCH;\n    ensures normal resource_arm_empty(r) otherwise;\n"),
        ),
        // An empty arm beside a release for one guard.
        (
            released,
            format!("    requires resource_discriminant(r, type, 0);\n{releasing}    ensures normal resource_arm_empty(r) when type == 1;\n    ensures normal resource_arm_released(r, u.p) when type == 1;\n    ensures normal resource_arm_empty(r) otherwise;\n"),
        ),
        // A discriminated object the declaration does not release.
        (
            "r: ptr<mut, u8>",
            "    requires resource_discriminant(r, type, 0);\n    ensures normal resource_borrowed(r);\n    ensures normal resource_arm_empty(r) otherwise;\n".to_owned(),
        ),
        // A duplicate arm release.
        (
            released,
            format!("    requires resource_discriminant(r, type, 0);\n{releasing}    ensures normal resource_arm_released(r, u.p) when type == 1;\n    ensures normal resource_arm_released(r, u.p) when type == 1;\n    ensures normal resource_arm_empty(r) otherwise;\n"),
        ),
    ];
    for (parameters, clauses) in cases {
        let text = declaration(parameters, &clauses);
        assert!(load(&text).is_err(), "accepted:\n{text}");
    }
}
