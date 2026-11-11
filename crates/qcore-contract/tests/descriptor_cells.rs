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

//! Card MK-EVENT-DESCRIPTOR-QUANT (contract specification §C04.6.2, grammar
//! G05d, gateway rule 38; USER DECISION 2026-11-01, "I approve the changes of
//! D1-D5."): descriptor cells read total-or-reject, the shorthand and its
//! lower level loading to one canonical set, and the four restated Monkey
//! declarations.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{check_source_assumptions, load_resource_contract_sources};
use qcore_schemas::callback::CallbackMember;
use qcore_schemas::contract::{ResourceContracts, ResourceKind, ResourceParameterMode};
use qcore_schemas::descriptor_cell::{
    CellPlace, CellState, CellValue, DescriptorCells, MemberEquality, WitnessGuard,
};

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "cells.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

fn cells(text: &str, name: &str) -> DescriptorCells {
    let contracts = load(text).unwrap_or_else(|error| panic!("{error}"));
    assert!(contracts.contract(name).is_none(), "{name} withheld");
    contracts
        .retaining_contract(name)
        .expect("retaining")
        .descriptor_cells
        .clone()
}

fn fd() -> CallbackMember {
    CallbackMember {
        tag: "mk_event".into(),
        path: "fd".into(),
    }
}

const TIMER_HEAD: &str = "
extern fn create(loop: ptr<mut, u8>, data: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(loop);
    ensures normal resource_borrowed(data);
    ensures normal resource_retained_by(data, loop);
";

const TIMER_SHORTHAND: &str = "
    quant live #data: mk_event.fd |-> #fresh(descriptor) when #result != -1;
    quant live #data: mk_event.fd |-> {#stale(descriptor), #kept} when #result == -1;
";

const TIMER_LOWER: &str = "
    ensures normal resource_member_kind(data, mk_event.fd, descriptor);
    ensures normal resource_on_return(ne, -1, resource_member_out(data, mk_event.fd));
    ensures normal resource_on_return(eq, -1, resource_member_stale_or_kept(data, mk_event.fd));
";

fn timer(body: &str) -> String {
    format!("{TIMER_HEAD}{body}    ensures normal resource_return_none();\n}};\n")
}

#[test]
fn the_timer_creation_reads_identically_at_both_levels() {
    let shorthand = cells(&timer(TIMER_SHORTHAND), "create");
    let lower = cells(&timer(TIMER_LOWER), "create");
    let both = cells(&timer(&format!("{TIMER_SHORTHAND}{TIMER_LOWER}")), "create");
    assert_eq!(shorthand, lower);
    assert_eq!(shorthand, both);
    let member = CellPlace::Member {
        parameter: 1,
        member: fd(),
    };
    assert_eq!(
        shorthand.kinds,
        vec![(member.clone(), ResourceKind::Descriptor)]
    );
    assert_eq!(
        shorthand.states,
        vec![
            CellState {
                place: member.clone(),
                value: CellValue::Fresh,
                guard: Some(WitnessGuard::NotSentinel),
            },
            CellState {
                place: member,
                value: CellValue::StaleOrKept,
                guard: Some(WitnessGuard::Sentinel),
            },
        ]
    );
}

const CHANNEL_HEAD: &str = "
extern fn channel(loop: ptr<mut, u8>, r_fd: ptr<mut, u8>, w_fd: ptr<mut, u8>, data: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(loop);
    ensures normal resource_borrowed(data);
    ensures normal resource_retained_by(data, loop);
";

#[test]
fn the_channel_slots_read_identically_at_both_levels() {
    let shorthand = format!(
        "{CHANNEL_HEAD}
    ensures normal resource_borrowed(r_fd);
    ensures normal resource_borrowed(w_fd);
    quant live #r_fd: #pointee |-> #fresh(descriptor) when #result == 0;
    quant live #w_fd: #pointee |-> #fresh(descriptor) when #result == 0;
    quant live #data: mk_event.fd |-> #same(r_fd) when #result == 0;
    quant live #data: mk_event.fd |-> {{#stale(descriptor), #kept}} when #result != 0;
    ensures normal resource_return_none();
}};"
    );
    let lower = format!(
        "{CHANNEL_HEAD}
    ensures normal resource_on_return(eq, 0, resource_out(r_fd, nonnull));
    ensures normal resource_on_return(ne, 0, resource_borrowed(r_fd));
    ensures normal resource_kind(r_fd, descriptor);
    ensures normal resource_on_return(eq, 0, resource_out(w_fd, nonnull));
    ensures normal resource_on_return(ne, 0, resource_borrowed(w_fd));
    ensures normal resource_kind(w_fd, descriptor);
    ensures normal resource_member_kind(data, mk_event.fd, descriptor);
    ensures normal resource_on_return(eq, 0, resource_member_alias(data, mk_event.fd, r_fd));
    ensures normal resource_on_return(ne, 0, resource_member_stale_or_kept(data, mk_event.fd));
    ensures normal resource_return_none();
}};"
    );
    let read = cells(&shorthand, "channel");
    assert_eq!(read, cells(&lower, "channel"));
    assert!(read.states.contains(&CellState {
        place: CellPlace::Slot { parameter: 1 },
        value: CellValue::Fresh,
        guard: Some(WitnessGuard::Zero),
    }));
    assert!(read.states.contains(&CellState {
        place: CellPlace::Member {
            parameter: 3,
            member: fd(),
        },
        value: CellValue::Same { slot: 1 },
        guard: Some(WitnessGuard::Zero),
    }));
    let contracts = load(&lower).expect("loads");
    let declaration = contracts.retaining_contract("channel").expect("retaining");
    assert_eq!(
        declaration.contract.parameters[1],
        ResourceParameterMode::Borrowed
    );
    assert!(declaration.contract.parameter_kinds.is_empty());
}

const TEARDOWN: &str = "
extern fn teardown(loop: ptr<mut, u8>, r_fd: i32, w_fd: i32, data: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(loop);
    ensures normal resource_borrowed(data);
    ensures normal resource_kind(r_fd, descriptor);
    ensures normal resource_kind(w_fd, descriptor);
    ensures normal resource_released(r_fd, valid) when #data: mk_event.fd == #r_fd;
    ensures normal resource_released(w_fd, valid) when #data: mk_event.fd == #r_fd;
    quant live #loop: #holds(data) |-> #absent when #data: mk_event.fd == #r_fd;
    ensures normal resource_return_none();
};";

#[test]
fn the_channel_teardown_reads_its_one_guard() {
    let read = cells(TEARDOWN, "teardown");
    assert_eq!(read.guarded_releases, vec![1, 2]);
    assert!(read.guarded_removal);
    assert_eq!(
        read.guard,
        Some(MemberEquality {
            parameter: 3,
            member: fd(),
            against: 1,
        })
    );
    let contracts = load(TEARDOWN).expect("loads");
    let declaration = contracts.retaining_contract("teardown").expect("retaining");
    assert_eq!(declaration.removals.len(), 1);
    assert_eq!(
        declaration.contract.parameters[1],
        ResourceParameterMode::Borrowed
    );
}

#[test]
fn every_violation_rejects_the_whole_module() {
    let cases: Vec<(String, &str)> = vec![
        // A creation needs its witness.
        (
            timer("    quant live #data: mk_event.fd |-> #fresh(descriptor);\n"),
            "quant-live-cell-guard",
        ),
        // Two witness pairs.
        (
            timer(
                "    quant live #data: mk_event.fd |-> #fresh(descriptor) when #result != -1;\n    quant live #data: mk_event.fd |-> {#stale(descriptor), #kept} when #result != 0;\n",
            ),
            "quant-live-cell-guard",
        ),
        // Two states in one class.
        (
            timer(
                "    quant live #data: mk_event.fd |-> #fresh(descriptor) when #result != -1;\n    quant live #data: mk_event.fd |-> {#stale(descriptor), #kept} when #result != -1;\n",
            ),
            "quant-live-cell-duplicate",
        ),
        // A disagreeing overlap of the two levels.
        (
            timer(&format!(
                "{TIMER_SHORTHAND}    ensures normal resource_on_return(ne, -1, resource_member_stale_or_kept(data, mk_event.fd));\n"
            )),
            "quant-live-cell-duplicate",
        ),
        // A kind with no state.
        (
            timer(&format!(
                "{TIMER_SHORTHAND}    ensures normal resource_member_kind(data, mk_event.other, descriptor);\n"
            )),
            "quant-live-cell-kind",
        ),
        // A binder that is not borrowed.
        (
            "extern fn f(data: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_released(data, nonnull); quant live #data: mk_event.fd |-> #stale(descriptor); ensures normal resource_return_none(); };".into(),
            "quant-live-cell-binder",
        ),
        // `#same` of a slot with no creation.
        (
            format!(
                "{CHANNEL_HEAD}    ensures normal resource_borrowed(r_fd);\n    ensures normal resource_borrowed(w_fd);\n    quant live #data: mk_event.fd |-> #same(r_fd) when #result == 0;\n    ensures normal resource_return_none();\n}};"
            ),
            "quant-live-cell-value",
        ),
        // A witness over a result the declaration owns.
        (
            "extern fn f(data: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(data); quant live #data: mk_event.fd |-> #fresh(descriptor) when #result != -1; ensures normal resource_return_owned(sentinel); ensures normal resource_return_kind(descriptor); };".into(),
            "quant-live-cell-guard",
        ),
        // A guarded release without its kind.
        (
            TEARDOWN.replace("    ensures normal resource_kind(w_fd, descriptor);\n", ""),
            "quant-live-cell-equality",
        ),
        // Two different guards.
        (
            TEARDOWN.replace(
                "resource_released(w_fd, valid) when #data: mk_event.fd == #r_fd",
                "resource_released(w_fd, valid) when #data: mk_event.fd == #w_fd",
            ),
            "quant-live-cell-equality",
        ),
        // A guard against a formal the declaration does not release.
        (
            TEARDOWN.replace(
                "    ensures normal resource_released(r_fd, valid) when #data: mk_event.fd == #r_fd;\n",
                "    ensures normal resource_borrowed(r_fd);\n",
            ).replace("    ensures normal resource_kind(r_fd, descriptor);\n", ""),
            "quant-live-cell-equality",
        ),
        // A reserved keyword as a parameter.
        (
            "extern fn f(fresh: ptr<mut, u8>, data: ptr<mut, u8>) -> i32 contract { requires resource_contract_v1; ensures normal resource_borrowed(fresh); ensures normal resource_borrowed(data); quant live #data: mk_event.fd |-> #stale(descriptor); ensures normal resource_return_none(); };".into(),
            "reserved",
        ),
    ];
    for (text, diagnostic) in cases {
        let error = load(&text).expect_err(&text);
        assert!(error.contains(diagnostic), "{text}\n{error}");
    }
}

#[test]
fn the_restated_monkey_declarations_load() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../providers");
    let module = "qcore-provider-libevent/contracts/monkey/event_loop.qv";
    let text = std::fs::read_to_string(format!("{root}/{module}")).expect("module");
    assert!(!text.contains("resource_premise(mk_event_descriptors"));
    let contracts = load_resource_contract_sources(&BTreeMap::from([(module.to_owned(), text)]))
        .unwrap_or_else(|error| panic!("{error}"));
    check_source_assumptions(&contracts).expect("reloads");
    for name in [
        "mk_event_timeout_create",
        "mk_event_timeout_destroy",
        "mk_event_channel_create",
        "mk_event_channel_destroy",
    ] {
        let declaration = contracts.retaining_contract(name).expect(name);
        assert!(!declaration.descriptor_cells.is_empty(), "{name}");
    }
    let destroy = contracts
        .retaining_contract("mk_event_timeout_destroy")
        .expect("destroy");
    assert_eq!(
        destroy.descriptor_cells.states,
        vec![CellState {
            place: CellPlace::Member {
                parameter: 1,
                member: fd(),
            },
            value: CellValue::Stale,
            guard: None,
        }]
    );
    assert_eq!(destroy.removals.len(), 1);
    assert!(
        contracts
            .retaining_contract("mk_event_channel_destroy")
            .expect("channel destroy")
            .descriptor_cells
            .guarded_removal
    );
}
