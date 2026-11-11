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

use super::super::monkey_source::import_monkey_callbacks;
use super::*;
use crate::legacy_c::C11TranslationUnit;
use serde_json::{Value as Json, json};

const SOURCE: &str = include_str!("../fixtures/monkey/callbacks.c");
const OBSERVATIONS: &str = include_str!("../fixtures/monkey/binding/observations.jsonl");
const BINDINGS: &str = include_str!("../fixtures/monkey/binding/bindings.jsonl");
const CONTRACT: &str =
    include_str!("../../../../providers/qcore-provider-libevent/contracts/monkey/composition.qv");

fn import(source: &str) -> Vec<MonkeySourceResult> {
    import_monkey_callbacks(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap()
}
fn rows() -> Vec<Json> {
    BINDINGS
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn encode(rows: &[Json]) -> String {
    rows.iter()
        .map(Json::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
fn check(source: &str, bindings: &str, contract: &str) -> Result<MonkeyBindingReport, String> {
    check_monkey_bindings(&import(source), OBSERVATIONS, bindings, contract)
}
fn checked(report: &MonkeyBindingReport) -> usize {
    report.callbacks.iter().map(|r| r.checked_executions).sum()
}
fn callback(rows: &mut [Json], event: u64) -> &mut Json {
    rows.iter_mut()
        .find(|row| row["kind"] == "callback" && row["event"] == event)
        .unwrap()
}

#[test]
fn native_callbacks_bind_source_paths_and_qcore_predicates() {
    if !crate::test_support::clang_available() {
        return;
    }
    let report = check(SOURCE, BINDINGS, CONTRACT).unwrap();
    assert_eq!(checked(&report), 3, "{report:?}");
    assert_eq!(report.callbacks[0].unknown_paths.len(), 3);
    assert_eq!(report.callbacks[1].unknown_paths.len(), 2);
    assert!(report.callbacks[2].unknown_paths.is_empty());
    assert_eq!(
        report
            .callbacks
            .iter()
            .map(|r| r.checked_postconditions)
            .sum::<usize>(),
        6
    );
    assert!(!report.static_certificate);
    assert!(!report.unknown.is_empty());
}

#[test]
fn rejects_missing_duplicate_stale_and_cross_event_bindings() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = import(SOURCE);
    for mutation in 0..7 {
        let mut rows = rows();
        match mutation {
            0 => {
                rows.remove(0);
            }
            1 => {
                rows.push(rows[0].clone());
            }
            2 => {
                callback(&mut rows, 1)["argument"] = json!(8);
            }
            3 => {
                callback(&mut rows, 1)["fd"] = json!(99);
            }
            4 => {
                callback(&mut rows, 1)["exit"] = json!(44);
            }
            5 => {
                callback(&mut rows, 1)["schema_version"] = json!(2);
            }
            6 => {
                callback(&mut rows, 1)["untrusted_extra"] = json!(true);
            }
            _ => unreachable!(),
        }
        assert!(
            check_monkey_bindings(&source, OBSERVATIONS, &encode(&rows), CONTRACT).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn changed_memory_guards_and_calls_do_not_check() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = import(SOURCE);
    for mutation in 0..7 {
        let mut rows = rows();
        match mutation {
            0 => callback(&mut rows, 1)["flags"] = json!(4),
            1 => callback(&mut rows, 1)["after"][0]["value"] = json!(0),
            2 => callback(&mut rows, 1)["before"][4]["value"] = json!(-1),
            3 => callback(&mut rows, 4)["calls"][0]["arguments"][1]["value"] = json!(2),
            4 => callback(&mut rows, 4)["calls"][1]["memory_value"] = json!(103),
            5 => callback(&mut rows, 4)["calls"]
                .as_array_mut()
                .unwrap()
                .swap(5, 6),
            6 => {
                callback(&mut rows, 4)["calls"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            _ => unreachable!(),
        }
        let result = check_monkey_bindings(&source, OBSERVATIONS, &encode(&rows), CONTRACT);
        assert!(
            result.is_err() || checked(&result.unwrap()) < 3,
            "mutation {mutation}"
        );
    }
}

#[test]
fn runtime_effects_cannot_be_omitted_rebound_or_reused() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = import(SOURCE);
    for mutation in 0..4 {
        let mut rows = rows();
        let calls = callback(&mut rows, 4)["calls"].as_array_mut().unwrap();
        match mutation {
            0 => {
                calls.remove(3);
            }
            1 => calls[3]["sequence"] = json!(40),
            2 => calls[4]["arguments"][0] = json!(17),
            3 => calls[0]["arguments"][0] = json!(17),
            _ => unreachable!(),
        }
        assert!(check_monkey_bindings(&source, OBSERVATIONS, &encode(&rows), CONTRACT).is_err());
    }
}

#[test]
fn source_and_contract_mutations_change_verdicts() {
    if !crate::test_support::clang_available() {
        return;
    }
    for source in [
        SOURCE.replace("fired->data = event", "fired->data = map"),
        SOURCE.replace("fired_count++", "fired_count += 2"),
        SOURCE.replace("uint64_t val = 1", "uint64_t val = 2"),
        SOURCE.replace(
            "mk_mem_free(ev_map);",
            "mk_mem_free(ev_map); mk_mem_free(ev_map);",
        ),
    ] {
        let report = check(&source, BINDINGS, CONTRACT).unwrap();
        assert!(checked(&report) < 3, "{report:?}");
    }
    let contract = CONTRACT.replace(
        "ensures normal fired_event_array_is_written",
        "ensures normal not fired_event_array_is_written",
    );
    assert_eq!(checked(&check(SOURCE, BINDINGS, &contract).unwrap()), 2);
    let contract = CONTRACT.replace("callback_has_no_resource_effect", "unsupported_predicate");
    assert_eq!(checked(&check(SOURCE, BINDINGS, &contract).unwrap()), 2);
}

#[test]
fn source_unknown_is_reported_per_callback_without_promotion() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = SOURCE.replace("ctx->fired_count++;", "while (1) { ctx->fired_count++; }");
    let report = check(&source, BINDINGS, CONTRACT).unwrap();
    assert_eq!(checked(&report), 2);
    assert!(!report.callbacks[0].unknown_reasons.is_empty());
    assert!(!report.static_certificate);
}

#[test]
fn native_binding_rules_are_executed_and_mutations_fail_closed() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = import(SOURCE);
    let contracts = CALLBACK_BINDING_CONTRACTS.replace("\r\n", "\n");
    for name in [
        "binding_symbol",
        "binding_call_arguments",
        "binding_call_interval",
        "binding_runtime_effect",
        "binding_scope",
        "binding_memory",
        "binding_execution",
        "binding_select_path",
        "binding_send",
        "binding_errno_location",
        "binding_perror",
        "binding_evutil_closesocket",
        "binding_event_del",
        "binding_event_free",
        "binding_mk_mem_free",
    ] {
        let marker = format!("extern fn {name}() -> unit\ncontract {{");
        for predicate in ["false", "unknown_native_predicate"] {
            let rules = contracts.replace(
                &marker,
                &format!("{marker}\n    ensures normal {predicate};"),
            );
            assert_ne!(rules, contracts);
            match check_monkey_bindings_with_rules(
                &source,
                OBSERVATIONS,
                BINDINGS,
                CONTRACT,
                &rules,
            ) {
                Err(error) => assert!(error.contains(predicate), "{name}: {error}"),
                Ok(report) => {
                    assert!(checked(&report) < 3, "{name}: {report:?}");
                    assert!(!report.static_certificate);
                    assert!(
                        report
                            .callbacks
                            .iter()
                            .flat_map(|row| &row.unknown_reasons)
                            .any(|reason| reason.contains(predicate)),
                        "{name}: {report:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn expanded_native_run_covers_all_paths_and_labels_injected_failure() {
    if !crate::test_support::clang_available() {
        return;
    }
    let observations = include_str!("../fixtures/monkey/binding/all_paths/observations.jsonl");
    let bindings = include_str!("../fixtures/monkey/binding/all_paths/bindings.jsonl");
    let source = import(SOURCE);
    let report = check_monkey_bindings(&source, observations, bindings, CONTRACT).unwrap();
    assert_eq!(checked(&report), 8, "{report:?}");
    assert!(
        report
            .callbacks
            .iter()
            .all(|row| row.unknown_paths.is_empty() && row.unknown_reasons.is_empty())
    );
    assert_eq!(
        report
            .callbacks
            .iter()
            .map(|row| row.checked_postconditions)
            .sum::<usize>(),
        18
    );
    let timeout = &report.callbacks[1];
    assert_eq!(timeout.native_paths, BTreeSet::from([0, 2]));
    assert_eq!(timeout.injected_paths, BTreeSet::from([1]));
    assert_eq!(timeout.injected_executions, 1);
    assert!(!report.static_certificate);
    assert!(!report.unknown.is_empty());

    let coverage = account_monkey_callbacks(&source, Some(&report)).unwrap();
    assert_eq!(coverage.selected_callbacks, 3);
    assert_eq!(coverage.accounted_callbacks, 3);
    assert_eq!(coverage.selected_paths, 8);
    assert_eq!(coverage.checked_paths, 8);
    assert!(coverage.callbacks.iter().all(|row| {
        row.missing_paths.is_empty()
            && matches!(
                &row.cycle18_composition,
                Cycle18CallbackStatus::Unknown { reason }
                    if reason.contains("no source-to-Cycle18 EventProgram")
            )
    }));

    let mut rows: Vec<Json> = bindings
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let cleanup = rows
        .iter_mut()
        .filter_map(|row| row["calls"].as_array_mut())
        .flatten()
        .find(|call| call["function"] == "event_free")
        .unwrap();
    cleanup["origin"] = json!("injected_failure");
    assert!(check_monkey_bindings(&source, observations, &encode(&rows), CONTRACT).is_err());
    assert!(
        check_monkey_bindings(
            &source,
            observations,
            &bindings.replace("injected_failure", "unknown_provenance"),
            CONTRACT
        )
        .is_err()
    );
}

#[test]
fn selected_callbacks_are_accounted_without_the_optional_native_directory() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = import(SOURCE);
    let coverage = account_monkey_callbacks(&source, None).unwrap();
    assert_eq!(coverage.selected_callbacks, coverage.accounted_callbacks);
    assert_eq!(coverage.selected_paths, 8);
    assert_eq!(coverage.checked_paths, 0);
    assert!(coverage.callbacks.iter().all(|row| {
        row.selected_paths == Some(row.missing_paths.len())
            && row.checked_paths.is_empty()
            && matches!(
                &row.cycle18_composition,
                Cycle18CallbackStatus::Unknown { reason }
                    if reason.contains("was not supplied")
            )
    }));
}
