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

use super::*;
use crate::contract_loader::load_resource_contract_sources;
use serde_json::{Value, json};

const UNISTD: &str =
    include_str!("../../../../providers/quirin-provider-libc/contracts/libc/unistd.qv");
const SOCKET: &str =
    include_str!("../../../../providers/quirin-provider-libc/contracts/libc/socket.qv");
const STDIO: &str =
    include_str!("../../../../providers/quirin-provider-libc/contracts/libc/stdio.qv");
const DIRENT: &str =
    include_str!("../../../../providers/quirin-provider-libc/contracts/libc/dirent.qv");

fn landed() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/unistd.qv".to_string(), UNISTD.to_string()),
        ("libc/socket.qv".to_string(), SOCKET.to_string()),
        ("libc/stdio.qv".to_string(), STDIO.to_string()),
        ("libc/dirent.qv".to_string(), DIRENT.to_string()),
    ]))
    .expect("the landed libc descriptor modules load")
}

fn load(text: &str) -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "libc/test.qcore".to_string(),
        text.to_string(),
    )]))
    .expect("test contract loads")
}

fn operand(role: &str, kind: &str, id: i64, generation: u32) -> Value {
    json!({"role": role, "kind": kind, "id": id, "generation": generation})
}

fn row(call: &str, result: i64, errno: i64, operands: Vec<Value>) -> Value {
    json!({"schema_version":1,"sequence":0,"pid":7,"thread":7,"call":call,
           "site":{"module":"target","offset":64},"args":[],"result":result,
           "errno":errno,"injected":false,"operands":operands})
}

fn render(rows: Vec<Value>) -> String {
    let mut all = vec![row(
        "start",
        0,
        0,
        vec![
            operand("inherited", "fd", 0, 1),
            operand("inherited", "fd", 1, 1),
            operand("inherited", "fd", 2, 1),
        ],
    )];
    all.extend(rows);
    all.push(row("exit", 0, 0, vec![]));
    all.into_iter()
        .enumerate()
        .map(|(index, mut row)| {
            row["sequence"] = json!(index + 1);
            format!("{row}\n")
        })
        .collect()
}

fn bind(rows: Vec<Value>) -> BindingReport {
    bind_descriptor_contracts(&render(rows), &landed()).unwrap()
}

/// Every landed descriptor, stream and directory contract, on the executions
/// it states, including failures and a borrowed `fileno`.
fn faithful() -> Vec<Value> {
    vec![
        row("open", 3, 0, vec![operand("acquired", "fd", 3, 1)]),
        row("open", -1, 2, vec![]),
        row("read", 4, 0, vec![operand("used", "fd", 3, 1)]),
        row("close", 0, 0, vec![operand("released", "fd", 3, 1)]),
        row("socket", 3, 0, vec![operand("acquired", "fd", 3, 2)]),
        row("socket", -1, 97, vec![]),
        row("close", 0, 0, vec![operand("released", "fd", 3, 2)]),
        row(
            "fopen",
            4096,
            0,
            vec![
                operand("acquired", "file", 4096, 1),
                operand("derived", "fd", 3, 3),
            ],
        ),
        row(
            "fileno",
            3,
            0,
            vec![
                operand("used", "file", 4096, 1),
                operand("related", "fd", 3, 3),
            ],
        ),
        row(
            "fclose",
            0,
            0,
            vec![
                operand("released", "file", 4096, 1),
                operand("released_child", "fd", 3, 3),
            ],
        ),
        row("fopen", 0, 2, vec![]),
        row(
            "opendir",
            8192,
            0,
            vec![
                operand("acquired", "dir", 8192, 1),
                operand("derived", "fd", 3, 4),
            ],
        ),
        row(
            "closedir",
            0,
            0,
            vec![
                operand("released", "dir", 8192, 1),
                operand("released_child", "fd", 3, 4),
            ],
        ),
        row("opendir", 0, 13, vec![]),
    ]
}

#[test]
fn faithful_executions_contradict_no_landed_contract() {
    let report = bind(faithful());
    assert_eq!(report.contradictions, 0, "{:#?}", report.bound);
    for callee in [
        "open", "close", "socket", "fopen", "fileno", "fclose", "opendir", "closedir",
    ] {
        assert!(report.bound.contains_key(callee), "{callee} bound");
    }
    assert_eq!(report.bound["open"].successes, 1);
    assert_eq!(report.bound["open"].failures, 1);
    assert_eq!(report.bound["close"].executions, 2);
    assert!(report.bound["close"].caller_observations.is_empty());
    assert_eq!(
        report.bound["fopen"].facts["owned_result_derives_its_descriptor"],
        1
    );
    assert_eq!(
        report.bound["fclose"].facts["release_ends_owned_descriptor"],
        1
    );
    assert!(report.bound["read"].contradictions == 0);
    assert!(!report.static_certificate);
}

#[test]
fn a_success_that_acquires_the_wrong_kind_or_identity_contradicts() {
    for wrong in [
        row("open", 3, 0, vec![operand("acquired", "file", 3, 1)]),
        row("open", 3, 0, vec![operand("acquired", "fd", 4, 1)]),
        row("open", 3, 0, vec![]),
        row("fopen", 4096, 0, vec![operand("acquired", "dir", 4096, 1)]),
    ] {
        let report = bind(vec![wrong]);
        assert_eq!(report.contradictions, 1, "{:#?}", report.bound);
    }
}

#[test]
fn a_failure_that_acquires_or_a_result_that_is_neither_sentinel_nor_valid_contradicts() {
    let report = bind(vec![row(
        "socket",
        -1,
        24,
        vec![operand("acquired", "fd", 5, 1)],
    )]);
    assert_eq!(report.contradictions, 1);
    let report = bind(vec![row(
        "open",
        -2,
        0,
        vec![operand("acquired", "fd", -2, 1)],
    )]);
    assert_eq!(report.contradictions, 1);
}

#[test]
fn an_owned_result_that_was_already_live_contradicts_freshness() {
    let report = bind(vec![
        row("open", 3, 0, vec![operand("acquired", "fd", 3, 1)]),
        row("open", 3, 0, vec![operand("acquired", "fd", 3, 2)]),
    ]);
    assert_eq!(report.bound["open"].contradictions, 1);
}

#[test]
fn a_neutral_contract_that_acquires_or_releases_contradicts() {
    let report = bind(vec![row(
        "fileno",
        3,
        0,
        vec![operand("acquired", "fd", 3, 1)],
    )]);
    assert_eq!(report.bound["fileno"].contradictions, 1);
    let report = bind(vec![row(
        "read",
        0,
        0,
        vec![operand("released", "fd", 1, 1)],
    )]);
    assert_eq!(report.bound["read"].contradictions, 1);
}

#[test]
fn a_release_of_another_kind_contradicts() {
    let report = bind(vec![
        row("fopen", 4096, 0, vec![operand("acquired", "file", 4096, 1)]),
        row("closedir", 0, 0, vec![operand("released", "file", 4096, 1)]),
    ]);
    assert_eq!(report.bound["closedir"].contradictions, 1);
}

#[test]
fn a_caller_precondition_break_is_a_caller_observation_not_a_contradiction() {
    let report = bind(vec![
        row("close", -1, 9, vec![operand("released", "fd", -1, 0)]),
        row("open", 3, 0, vec![operand("acquired", "fd", 3, 1)]),
        row("close", 0, 0, vec![operand("released", "fd", 3, 1)]),
        row("close", -1, 9, vec![operand("released", "fd", 3, 1)]),
        row(
            "fopen",
            4096,
            0,
            vec![
                operand("acquired", "file", 4096, 1),
                operand("derived", "fd", 3, 2),
            ],
        ),
        row("close", 0, 0, vec![operand("released", "fd", 3, 2)]),
    ]);
    let close = &report.bound["close"];
    assert_eq!(close.contradictions, 0);
    assert_eq!(close.caller_observations["invalid_argument"], 1);
    assert_eq!(close.caller_observations["not_live"], 1);
    assert_eq!(close.caller_observations["owned_by_another_resource"], 1);
}

#[test]
fn a_contract_that_forbids_failure_is_contradicted_by_a_failing_execution() {
    // The landed contract allows `-1`; a hypothetical `valid` result would not.
    let never_fails = load(
        "module contract;\nextern fn open(file: ptr<const, u8>, oflag: i32) -> i32\ncontract {\n    \
         requires resource_contract_v1;\n    ensures normal resource_borrowed(file);\n    \
         ensures normal resource_borrowed(oflag);\n    ensures normal resource_variadic_data_tail();\n    \
         ensures normal resource_return_owned(valid);\n    ensures normal resource_return_kind(descriptor);\n};\n",
    );
    let report =
        bind_descriptor_contracts(&render(vec![row("open", -1, 2, vec![])]), &never_fails).unwrap();
    assert_eq!(report.contradictions, 1);
    let report =
        bind_descriptor_contracts(&render(vec![row("open", -1, 2, vec![])]), &landed()).unwrap();
    assert_eq!(report.contradictions, 0);
}

#[test]
fn an_unbindable_contract_shape_fails_closed() {
    let heap = load(
        "module contract;\nextern fn open(file: ptr<const, u8>) -> ptr<mut, u8>\ncontract {\n    \
         requires resource_contract_v1;\n    ensures normal resource_borrowed(file);\n    \
         ensures normal resource_return_owned(nullable);\n};\n",
    );
    assert!(bind_descriptor_contracts(&render(vec![row("open", 0, 2, vec![])]), &heap).is_err());
}

#[test]
fn injected_failures_are_counted_not_bound_and_undeclared_calls_are_profiled() {
    let mut injected = row("socket", -1, 24, vec![]);
    injected["injected"] = json!(true);
    let mut dup = row("fcntl", 0, 0, vec![operand("used", "fd", 1, 1)]);
    dup["args"] = json!([1, 4]);
    let report = bind(vec![
        injected,
        row(
            "dlopen",
            4096,
            0,
            vec![operand("acquired", "dl_handle", 4096, 1)],
        ),
        row(
            "dlopen",
            4096,
            0,
            vec![operand("acquired", "dl_handle", 4096, 1)],
        ),
        dup,
    ]);
    assert_eq!(report.bound["socket"].injected, 1);
    assert_eq!(report.bound["socket"].failures, 0);
    let dlopen = &report.unbound["dlopen"];
    assert_eq!(dlopen.reacquired_while_live, 1);
    assert_eq!(dlopen.acquired_equals_result, 2);
    assert_eq!(report.unbound["fcntl"].argument_1_values[&4], 1);
}

/// Card W46-EXTERNAL-CALLEE-CONTRACTS: `fcntl` is declared at `F_SETFD` (`2`)
/// only. An execution at that command binds to the declaration; an execution
/// at any other command, one that creates a descriptor (`F_DUPFD`, `0`) among
/// them, and one whose command was not recorded, is of an undeclared callee
/// and is profiled, never bound (contract spec C04.5).
#[test]
fn a_literal_premise_binds_only_the_executions_it_covers() {
    let mut setfd = row("fcntl", 0, 0, vec![operand("used", "fd", 3, 1)]);
    setfd["args"] = json!([3, 2, 1]);
    let mut dupfd = row("fcntl", 4, 0, vec![operand("acquired", "fd", 4, 1)]);
    dupfd["args"] = json!([3, 0, 0]);
    let unrecorded = row("fcntl", 0, 0, vec![operand("used", "fd", 3, 1)]);
    let report = bind(vec![
        row("open", 3, 0, vec![operand("acquired", "fd", 3, 1)]),
        setfd,
        dupfd,
        unrecorded,
        row("close", 0, 0, vec![operand("released", "fd", 3, 1)]),
    ]);
    assert_eq!(report.bound["fcntl"].executions, 1);
    assert_eq!(report.bound["fcntl"].contradictions, 0);
    assert_eq!(report.contradictions, 0);
    assert_eq!(report.unbound["fcntl"].executions, 2);
    assert_eq!(report.unbound["fcntl"].argument_1_values[&0], 1);
    assert_eq!(report.unbound["fcntl"].acquisitions["Fd"], 1);
}

/// Pinned real executions (Phase A measurement) bound to the landed contracts.
#[test]
fn pinned_real_runs_contradict_no_landed_contract() {
    let contracts = landed();
    let engine = bind_descriptor_contracts(
        include_str!("../fixtures/descriptor/engine_dummy_null.jsonl"),
        &contracts,
    )
    .unwrap();
    assert_eq!(engine.contradictions, 0, "{:#?}", engine.bound);
    // The five double closes of that run are the caller's, not the contract's.
    assert_eq!(engine.bound["close"].caller_observations["not_live"], 5);
    let probe = bind_descriptor_contracts(
        include_str!("../fixtures/descriptor/probe_libc.jsonl"),
        &contracts,
    )
    .unwrap();
    assert_eq!(probe.contradictions, 0, "{:#?}", probe.bound);
    assert!(probe.bound["fopen"].failures >= 1);
    assert!(probe.bound["opendir"].failures >= 1);
    assert_eq!(probe.unbound["dlopen"].reacquired_while_live, 1);
}

/// Phase B measurement: the actual `ctr_random_get` with descriptor 0 free.
/// `open` returns 0, which the landed `open` contract admits as a valid
/// descriptor, and the function's `fd > 0` guard leaves it live at exit.
#[test]
fn pinned_ctr_random_get_run_binds_a_zero_descriptor_and_leaks_it() {
    let records = include_str!("../fixtures/descriptor/ctraces_random_stdin_closed.jsonl");
    let binding = bind_descriptor_contracts(records, &landed()).unwrap();
    assert_eq!(binding.contradictions, 0, "{:#?}", binding.bound);
    assert_eq!(binding.bound["open"].successes, 1);
    assert_eq!(binding.bound["open"].zero_results, 1);
    assert!(!binding.bound.contains_key("close"));
    let policy = include_str!(
        "../../../../providers/quirin-provider-shared/contracts/library/descriptor_native_observation.qv"
    );
    let replay =
        crate::descriptor_observation::check_descriptor_observations(records, policy).unwrap();
    let leaked: Vec<_> = replay
        .live_at_exit
        .iter()
        .filter(|live| !live.inherited)
        .collect();
    assert_eq!(leaked.len(), 1);
    assert_eq!(
        (leaked[0].kind, leaked[0].id, leaked[0].origin_call.as_str()),
        (Kind::Fd, 0, "open")
    );
}
