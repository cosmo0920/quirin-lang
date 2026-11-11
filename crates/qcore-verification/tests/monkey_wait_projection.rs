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

#![cfg_attr(windows, allow(dead_code, unused_imports))]

use std::path::PathBuf;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceResult, SourceSummary, import_source_functions_from_profile,
};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_schemas::profile::ImportRole;
use qcore_verification::monkey_wait_projection::*;

const CALLBACKS: &str = include_str!("../../../compiler/tests/fixtures/monkey/callbacks.c");
const WRAPPERS: &str = include_str!("../../../compiler/tests/fixtures/monkey/wrappers.c");
const WRAPPER_CONTRACT: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/wrapper_paths.qv");
const COMPOSITION_CONTRACT: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/composition.qv");

fn contracts() -> MonkeyWaitContracts {
    MonkeyWaitContracts::parse(WRAPPER_CONTRACT, COMPOSITION_CONTRACT).unwrap()
}

fn wrapper_source() -> String {
    let wrappers = WRAPPERS.replace("\r\n", "\n");
    format!(
        "{wrappers}\n{}",
        r#"
int event_base_loop(struct event_base *, int);
int _mk_event_wait_with_flags(struct mk_event_loop *loop, int flags)
{
    struct mk_event_ctx *ctx = loop->data;
    ctx->fired_count = 0;
    event_base_loop(ctx->base, flags);
    loop->n_events = ctx->fired_count;
    return loop->n_events;
}
"#
    )
}

fn profile() -> qcore_contract::profile_loader::LoadedProfile {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    load_source_set_profile(&root.join("contract/profiles"), "fluent-bit").unwrap()
}

fn import(source: &str, role: ImportRole) -> Vec<SourceResult> {
    let source = source.replace("\r\n", "\n");
    import_source_functions_from_profile(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        monkey_source_revision(),
        &profile(),
        "monkey",
        role,
    )
    .unwrap()
}

fn summary<'a>(results: &'a [SourceResult], name: &str) -> &'a SourceSummary {
    results
        .iter()
        .find_map(|result| match result {
            SourceResult::Summary(summary) if summary.function() == name => Some(summary),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing imported summary {name}: {results:?}"))
}

fn project(callback_source: &str, wrapper_source: &str) -> MonkeyWaitSourceProjection {
    let callbacks = import(callback_source, ImportRole::Callback);
    let wrappers = import(wrapper_source, ImportRole::Wrapper);
    project_monkey_timed_wait_source(
        [
            summary(&callbacks, wait_callback()),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ],
        &contracts(),
    )
    .unwrap()
}

#[cfg(windows)]
#[test]
fn windows_llp64_summary_set_stays_unpromoted() {
    let wrappers_source = wrapper_source();
    let callbacks = import(CALLBACKS, ImportRole::Callback);
    let wrappers = import(&wrappers_source, ImportRole::Wrapper);
    assert!(matches!(
        project_monkey_timed_wait_source(
            [
                summary(&callbacks, wait_callback()),
                summary(&wrappers, "_mk_event_loop_create"),
                summary(&wrappers, "_mk_event_wait_with_flags"),
                summary(&wrappers, "_mk_event_wait_2"),
                summary(&wrappers, "_mk_event_loop_destroy"),
            ],
            &contracts(),
        ),
        Err(WaitSourceProjectionError::SourceArtifactMismatch(_))
    ));
}

#[cfg(not(windows))]
#[test]
fn exact_imported_source_derives_the_bounded_native_obligations() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let wrappers = wrapper_source();
    let projection = project(CALLBACKS, &wrappers);
    assert_eq!(projection.source_revision(), monkey_source_revision());
    assert_eq!(projection.source_summary_digests().len(), 5);
    assert_eq!(projection.selected_path_indices()[wait_callback()], 0);
    assert_eq!(
        projection.selected_path_indices()["_mk_event_loop_create"],
        3
    );
    assert_eq!(projection.selected_path_indices()["_mk_event_wait_2"], 2);
    assert_eq!(projection.required_observations().len(), 11);
    assert_eq!(projection.contract_digests().len(), 3);
    assert!(
        projection
            .contract_digests()
            .contains_key("monkey/wait_source_policy.qv")
    );
    assert_eq!(
        projection.callback_semantics(),
        WaitCallbackSemantics::BorrowedNoEffectNoTasks
    );
    assert_eq!(
        projection.required_observations()[1],
        WaitObservationRequirement::EventCreateSuccess {
            callback: wait_callback().into(),
            flags: 1,
            descriptor_absent: true,
            priority_required: true,
        }
    );
    assert_eq!(
        projection.required_observations()[4],
        WaitObservationRequirement::TimeoutActivation
    );
    assert!(matches!(
        projection.required_observations().last(),
        Some(WaitObservationRequirement::BaseFree)
    ));
}

#[cfg(not(windows))]
#[test]
fn body_and_lifecycle_substitutions_do_not_project() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let changed_callback = CALLBACKS.replace(
        "static void cb_wait_2_timeout(evutil_socket_t fd, short flags, void *data)\n{\n    return;\n}",
        "static void cb_wait_2_timeout(evutil_socket_t fd, short flags, void *data)\n{\n    perror(\"changed\");\n    return;\n}",
    );
    let callbacks = import(&changed_callback, ImportRole::Callback);
    let wrapper_source = wrapper_source();
    let wrappers = import(&wrapper_source, ImportRole::Wrapper);
    let result = project_monkey_timed_wait_source(
        [
            summary(&callbacks, wait_callback()),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ],
        &contracts(),
    );
    assert!(matches!(
        result,
        Err(WaitSourceProjectionError::SourceShapeMismatch(name)) if name == wait_callback()
    ));

    let changed_wrappers = wrapper_source.replacen("event_free(timeout_event);", "", 1);
    let callbacks = import(CALLBACKS, ImportRole::Callback);
    let wrappers = import(&changed_wrappers, ImportRole::Wrapper);
    let result = project_monkey_timed_wait_source(
        [
            summary(&callbacks, wait_callback()),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ],
        &contracts(),
    );
    assert!(matches!(
        result,
        Err(WaitSourceProjectionError::SourceShapeMismatch(name)) if name == "_mk_event_wait_2"
    ));
}

#[test]
fn incomplete_or_foreign_summary_sets_fail_closed() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let callbacks = import(CALLBACKS, ImportRole::Callback);
    let wrapper_source = wrapper_source();
    let wrappers = import(&wrapper_source, ImportRole::Wrapper);
    let incomplete = project_monkey_timed_wait_source(
        [
            summary(&callbacks, wait_callback()),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
        ],
        &contracts(),
    );
    assert_eq!(
        incomplete,
        Err(WaitSourceProjectionError::MissingSummary(
            "_mk_event_loop_destroy".into()
        ))
    );

    let duplicate = project_monkey_timed_wait_source(
        [
            summary(&callbacks, wait_callback()),
            summary(&callbacks, wait_callback()),
        ],
        &contracts(),
    );
    assert_eq!(
        duplicate,
        Err(WaitSourceProjectionError::DuplicateSummary(
            wait_callback().into()
        ))
    );

    let foreign = project_monkey_timed_wait_source(
        [
            summary(&callbacks, wait_callback()),
            summary(&callbacks, "cb_event"),
        ],
        &contracts(),
    );
    assert_eq!(
        foreign,
        Err(WaitSourceProjectionError::UnexpectedSummary(
            "cb_event".into()
        ))
    );
}

#[test]
#[ignore = "requires QCORE_MONKEY_PREPROCESSED from the pinned compile command"]
fn configured_pinned_translation_unit_matches_the_fixture_projection() {
    let path = std::env::var("QCORE_MONKEY_PREPROCESSED")
        .expect("QCORE_MONKEY_PREPROCESSED must name the pinned preprocessed translation unit");
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let source = std::fs::read_to_string(path).unwrap();
    let callbacks = import(&source, ImportRole::Callback);
    let wrappers = import(&source, ImportRole::Wrapper);
    let projection = project_monkey_timed_wait_source(
        [
            summary(&callbacks, wait_callback()),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ],
        &contracts(),
    )
    .unwrap();
    assert_eq!(projection.source_revision(), monkey_source_revision());
    assert_eq!(projection.source_summary_digests().len(), 5);
}

#[test]
fn missing_or_substituted_contract_semantics_fail_closed() {
    let wrapper_contract = WRAPPER_CONTRACT.replace("\r\n", "\n");
    let composition_contract = COMPOSITION_CONTRACT.replace("\r\n", "\n");
    let missing = wrapper_contract.replace(
        "    ensures normal timed_wait_calls_event_free_after_dispatch_and_returns_dispatch_result;\n",
        "",
    );
    assert_eq!(
        MonkeyWaitContracts::parse(&missing, &composition_contract),
        Err(WaitSourceProjectionError::ContractClauseMismatch(
            "_mk_event_wait_2".into()
        ))
    );

    let substituted = composition_contract.replace(
        "callback_has_no_resource_effect",
        "callback_may_release_resources",
    );
    assert_eq!(
        MonkeyWaitContracts::parse(&wrapper_contract, &substituted),
        Err(WaitSourceProjectionError::ContractClauseMismatch(
            wait_callback().into()
        ))
    );

    let owned_callback = composition_contract.replace(
        "callback_argument: borrow<'callback, const, MonkeyCallbackArgument> use one",
        "callback_argument: own<MonkeyCallbackArgument> use one",
    );
    assert_eq!(
        MonkeyWaitContracts::parse(&wrapper_contract, &owned_callback),
        Err(WaitSourceProjectionError::ContractClauseMismatch(
            wait_callback().into()
        ))
    );

    let effectful_callback = composition_contract.replacen(
        "extern fn cb_wait_2_timeout(\n    callback_argument: borrow<'callback, const, MonkeyCallbackArgument> use one\n) -> unit\ncontract {",
        "extern fn cb_wait_2_timeout(\n    callback_argument: borrow<'callback, const, MonkeyCallbackArgument> use one\n) -> unit\neffects { io }\ncontract {",
        1,
    );
    assert_eq!(
        MonkeyWaitContracts::parse(&wrapper_contract, &effectful_callback),
        Err(WaitSourceProjectionError::ContractClauseMismatch(
            wait_callback().into()
        ))
    );
}

#[cfg(not(windows))]
#[test]
fn same_count_operand_field_and_guard_substitutions_fail_closed() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let mutations = [
        ("event_new descriptor", "ctx->base, -1,", "ctx->base, -2,"),
        (
            "wait result field",
            "loop->n_events = ctx->fired_count;",
            "loop->n_events = ctx->queue_size;",
        ),
        ("timeout guard", "timeout == -1", "timeout == -2"),
    ];
    for (label, from, to) in mutations {
        let wrapper_source = wrapper_source().replacen(from, to, 1);
        let callbacks = import(CALLBACKS, ImportRole::Callback);
        let wrappers = import(&wrapper_source, ImportRole::Wrapper);
        let result = project_monkey_timed_wait_source(
            [
                summary(&callbacks, wait_callback()),
                summary(&wrappers, "_mk_event_loop_create"),
                summary(&wrappers, "_mk_event_wait_with_flags"),
                summary(&wrappers, "_mk_event_wait_2"),
                summary(&wrappers, "_mk_event_loop_destroy"),
            ],
            &contracts(),
        );
        assert!(
            matches!(
                result,
                Err(WaitSourceProjectionError::SourceArtifactMismatch(_))
            ),
            "{label} unexpectedly projected: {result:?}"
        );
    }
}
