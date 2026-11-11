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

//! Card W43-ABI-ENUM-AND-CONST-PP: guarded retention clauses (contract
//! specification §C04.14.1, grammar G05, gateway rule 12w), read
//! total-or-reject.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{check_source_assumptions, load_resource_contract_sources};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::retained_by::{GuardRelation, Retention, RetentionGuard, RetentionHolder};

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "guarded.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

fn declaration(clause: &str) -> String {
    format!(
        "extern fn set(holder: ptr<mut, u8>, kind: u32, value: ptr<const, u8>, delta: i8, flag: bool) -> i32
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(holder);
    ensures normal resource_borrowed(kind);
    ensures normal resource_borrowed(value);
    ensures normal resource_borrowed(delta);
    ensures normal resource_borrowed(flag);
    ensures normal {clause};
    ensures normal resource_return_none();
}};"
    )
}

#[test]
fn a_guarded_retention_loads_with_its_guard_and_range() {
    let contracts = load(&declaration(
        "resource_retained_by(value, holder) when kind == 2",
    ))
    .expect("loads");
    let set = contracts.retaining_contract("set").expect("retaining");
    assert_eq!(
        set.retentions,
        [Retention {
            retained: 2,
            holder: RetentionHolder::Parameter { index: 0 },
            guard: Some(RetentionGuard {
                parameter: 1,
                relation: GuardRelation::Eq,
                constant: 2,
                low: 0,
                high: i128::from(u32::MAX),
            }),
        }]
    );
    check_source_assumptions(&contracts).expect("the retained texts reload to the same tables");
    for (clause, relation, constant) in [
        (
            "resource_retained_by(value, holder) when kind<3",
            GuardRelation::Lt,
            3,
        ),
        (
            "resource_retained_by(value, holder) when delta >= -4",
            GuardRelation::Ge,
            -4,
        ),
        (
            "resource_retained_by(value, holder) when delta <= 0",
            GuardRelation::Le,
            0,
        ),
        (
            "resource_retained_by(value, global) when kind > 7",
            GuardRelation::Gt,
            7,
        ),
    ] {
        let contracts = load(&declaration(clause)).expect(clause);
        let guard = contracts
            .retaining_contract("set")
            .expect("retaining")
            .retentions[0]
            .guard
            .expect("guarded");
        assert_eq!(
            (guard.relation, guard.constant),
            (relation, constant),
            "{clause}"
        );
    }
}

#[test]
fn an_unguarded_retention_serialises_as_before() {
    let contracts = load(&declaration("resource_retained_by(value, holder)")).expect("loads");
    let set = contracts.retaining_contract("set").expect("retaining");
    let json = serde_json::to_string(&set.retentions[0]).expect("serialises");
    assert!(!json.contains("guard"), "{json}");
}

#[test]
fn every_malformed_guard_rejects_the_whole_module() {
    for clause in [
        // Only a retention takes a guard.
        "resource_holder_reset(holder) when kind == 2",
        // `!=` is not in the operator set; neither is a second guard.
        "resource_retained_by(value, holder) when kind != 2",
        "resource_retained_by(value, holder) when kind == 2 when kind == 3",
        "resource_retained_by(value, holder) when kind == 2 && kind == 3",
        // The literal is canonical: no leading zero, no `-0`, no suffix, no
        // other base, no name.
        "resource_retained_by(value, holder) when kind == 02",
        "resource_retained_by(value, holder) when delta == -0",
        "resource_retained_by(value, holder) when kind == 2u",
        "resource_retained_by(value, holder) when kind == 0x2",
        "resource_retained_by(value, holder) when kind == PTR",
        // An integer parameter the clause does not name.
        "resource_retained_by(value, holder) when holder == 0",
        "resource_retained_by(value, holder) when flag == 1",
        "resource_retained_by(value, holder) when nowhere == 1",
        // In range, and partitioning it.
        "resource_retained_by(value, holder) when kind == -1",
        "resource_retained_by(value, holder) when delta == 128",
        "resource_retained_by(value, holder) when kind >= 0",
        "resource_retained_by(value, holder) when kind < 0",
        "resource_retained_by(value, holder) when delta <= 127",
        "resource_retained_by(value, holder) when delta > 127",
        // `when` is the guard's word and nothing else's.
        "resource_retained_by(value, holder) when",
        "resource_retained_by(value, holder) when kind",
        "resource_retained_by(value, holder) kind == 2",
    ] {
        assert!(load(&declaration(clause)).is_err(), "{clause}");
    }
}
