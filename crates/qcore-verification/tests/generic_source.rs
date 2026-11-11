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

//! Provider-free entry points: actual typed C import followed by trusted replay.
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use qcore_verification::cleanup_outcome::{
    CleanupField, CleanupProfile, ResetValue, check_cleanup_outcomes,
};
use qcore_verification::counter_contract::check_counter_contract;
use qcore_verification::field_destructor::check_field_destructor;
use qcore_verification::list_unlink_contract::check_list_unlink;
const SOURCE: &str = include_str!("fixtures/generic_source/sample.c");
const COUNTER: &str = include_str!("fixtures/generic_source/counter.qv");
const DESTRUCTOR: &str = include_str!("fixtures/generic_source/destructor.qv");
const UNLINK: &str = include_str!("fixtures/generic_source/unlink.qv");
const CLEANUP: &str = include_str!("fixtures/generic_source/cleanup.qv");
fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "invalid explicit Clang: {reason}"
            );
            eprintln!("skipping Clang-dependent source test: {reason}");
            false
        }
    }
}
fn import(source: &str, function: &str) -> SourceSummary {
    let result = import_source_functions(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "declared-sample-revision",
        &[function],
        &SourceImportProfile {
            records: &[
                "sample_state",
                "sample_owner",
                "sample_payload",
                "sample_link",
                "sample_handle",
            ],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap()
    .remove(0);
    let SourceResult::Summary(summary) = result else {
        panic!("{result:?}");
    };
    summary
}
#[test]
fn counter_contract_needs_no_compiler_or_provider_adapter() {
    if !clang_available() {
        return;
    }
    let source = import(SOURCE, "sample_decrement");
    let certificate =
        check_counter_contract(&source, COUNTER, "struct sample_state *", "SampleState").unwrap();
    assert_eq!(certificate.field(), "remaining");
    assert_eq!(certificate.instantiate(1, 255).unwrap(), 0);
    assert!(certificate.instantiate(0, 255).is_err());
    let changed = import(
        &SOURCE.replace("state->remaining--;", "state->unrelated--;"),
        "sample_decrement",
    );
    assert!(
        check_counter_contract(&changed, COUNTER, "struct sample_state *", "SampleState").is_err()
    );
    assert!(
        check_counter_contract(
            &source,
            &COUNTER.replace("delta == -1", "false"),
            "struct sample_state *",
            "SampleState"
        )
        .is_err()
    );
}
#[test]
fn nested_field_release_preserves_root_and_order_without_provider_types() {
    if !clang_available() {
        return;
    }
    let source = import(SOURCE, "sample_dispose");
    let certificate =
        check_field_destructor(&source, DESTRUCTOR, "struct sample_owner *", "SampleOwner")
            .unwrap();
    assert_eq!(certificate.cases().len(), 3);
    assert_eq!(
        certificate.cases()[2].released_in_order,
        ["payload__data", "payload"]
    );
    let changed = import(
        &SOURCE.replace("\r\n", "\n").replace(
            "free(payload->data);\n    free(payload);",
            "free(payload);\n    free(payload->data);",
        ),
        "sample_dispose",
    );
    assert!(
        check_field_destructor(&changed, DESTRUCTOR, "struct sample_owner *", "SampleOwner")
            .is_err()
    );
    assert!(
        check_field_destructor(
            &source,
            &DESTRUCTOR.replace("live__payload == 0", "false"),
            "struct sample_owner *",
            "SampleOwner"
        )
        .is_err()
    );
}
#[test]
fn list_replay_checks_both_neighbor_alias_cases_and_helper_identity() {
    if !clang_available() {
        return;
    }
    let source = import(SOURCE, "sample_unlink");
    let helper = import(SOURCE, "sample_link");
    let certificate = check_list_unlink(
        &source,
        &helper,
        UNLINK,
        "struct sample_link *",
        "SampleLink",
    )
    .unwrap();
    assert_eq!(certificate.cases().len(), 2);
    assert!(!certificate.cases()[0].neighbors_alias);
    assert!(certificate.cases()[1].neighbors_alias);
    let changed = import(
        &SOURCE.replace("next->prev = prev;", "next->prev = next;"),
        "sample_link",
    );
    assert!(
        check_list_unlink(
            &source,
            &changed,
            UNLINK,
            "struct sample_link *",
            "SampleLink"
        )
        .is_err()
    );
    assert!(
        check_list_unlink(
            &source,
            &source,
            UNLINK,
            "struct sample_link *",
            "SampleLink"
        )
        .is_err()
    );
}
#[test]
fn configured_cleanup_retains_failure_state_instead_of_assuming_discharge() {
    if !clang_available() {
        return;
    }
    let profile = CleanupProfile {
        function: "sample_cleanup".into(),
        root_type: "struct sample_handle *".into(),
        syscall: "sample_release".into(),
        syscall_signature: "int (int)".into(),
        presence_field: "handle".into(),
        fields: vec![CleanupField {
            name: "handle".into(),
            c_type: "int".into(),
            reset: ResetValue::Integer(-1),
        }],
        argument_fields: vec!["handle".into()],
        policy: "sample_cleanup_policy".into(),
        errno_function: "sample_errno".into(),
        report_function: "sample_report".into(),
    };
    let source = import(SOURCE, "sample_cleanup");
    let certificate = check_cleanup_outcomes(&source, &profile, CLEANUP).unwrap();
    assert_eq!(certificate.cases().len(), 4);
    let failure = &certificate.cases()[2];
    assert_eq!(failure.syscall_result, Some(-1));
    assert_eq!(failure.returned, -1);
    assert!(failure.fields_preserved);
    assert!(failure.field_writes.is_empty());
    assert!(certificate.cases()[3].fields_reset);
    let changed = import(
        &SOURCE.replace(
            "sample_report(*sample_errno()",
            "state->handle = -1; sample_report(*sample_errno()",
        ),
        "sample_cleanup",
    );
    assert!(check_cleanup_outcomes(&changed, &profile, CLEANUP).is_err());
}
