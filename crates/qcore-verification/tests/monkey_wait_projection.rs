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
const WRAPPER_CONTRACT: &str = include_str!("../../../contract/qcore/monkey/wrapper_paths.qcore");
const COMPOSITION_CONTRACT: &str = include_str!("../../../contract/qcore/monkey/composition.qcore");

fn contracts() -> MonkeyWaitContracts {
    MonkeyWaitContracts::parse(WRAPPER_CONTRACT, COMPOSITION_CONTRACT).unwrap()
}

fn wrapper_source() -> String {
    format!(
        "{WRAPPERS}\n{}",
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
    import_source_functions_from_profile(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        MONKEY_SOURCE_REVISION,
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
            summary(&callbacks, WAIT_CALLBACK),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ],
        &contracts(),
    )
    .unwrap()
}

#[test]
fn exact_imported_source_derives_the_bounded_native_obligations() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let wrappers = wrapper_source();
    let projection = project(CALLBACKS, &wrappers);
    assert_eq!(projection.source_revision(), MONKEY_SOURCE_REVISION);
    assert_eq!(projection.source_summary_digests().len(), 5);
    assert_eq!(projection.selected_path_indices()[WAIT_CALLBACK], 0);
    assert_eq!(
        projection.selected_path_indices()["_mk_event_loop_create"],
        3
    );
    assert_eq!(projection.selected_path_indices()["_mk_event_wait_2"], 2);
    assert_eq!(projection.required_observations().len(), 11);
    assert_eq!(projection.contract_digests().len(), 2);
    assert_eq!(
        projection.callback_semantics(),
        WaitCallbackSemantics::BorrowedNoEffectNoTasks
    );
    assert_eq!(
        projection.required_observations()[1],
        WaitObservationRequirement::EventCreateSuccess {
            callback: WAIT_CALLBACK.into(),
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
            summary(&callbacks, WAIT_CALLBACK),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ],
        &contracts(),
    );
    assert!(matches!(
        result,
        Err(WaitSourceProjectionError::SourceShapeMismatch(name)) if name == WAIT_CALLBACK
    ));

    let changed_wrappers = wrapper_source.replacen("event_free(timeout_event);", "", 1);
    let callbacks = import(CALLBACKS, ImportRole::Callback);
    let wrappers = import(&changed_wrappers, ImportRole::Wrapper);
    let result = project_monkey_timed_wait_source(
        [
            summary(&callbacks, WAIT_CALLBACK),
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
            summary(&callbacks, WAIT_CALLBACK),
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
            summary(&callbacks, WAIT_CALLBACK),
            summary(&callbacks, WAIT_CALLBACK),
        ],
        &contracts(),
    );
    assert_eq!(
        duplicate,
        Err(WaitSourceProjectionError::DuplicateSummary(
            WAIT_CALLBACK.into()
        ))
    );

    let foreign = project_monkey_timed_wait_source(
        [
            summary(&callbacks, WAIT_CALLBACK),
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
            summary(&callbacks, WAIT_CALLBACK),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ],
        &contracts(),
    )
    .unwrap();
    assert_eq!(projection.source_revision(), MONKEY_SOURCE_REVISION);
    assert_eq!(projection.source_summary_digests().len(), 5);
}

#[test]
fn missing_or_substituted_contract_semantics_fail_closed() {
    let missing = WRAPPER_CONTRACT.replace(
        "    ensures normal timed_wait_calls_event_free_after_dispatch_and_returns_dispatch_result;\n",
        "",
    );
    assert_eq!(
        MonkeyWaitContracts::parse(&missing, COMPOSITION_CONTRACT),
        Err(WaitSourceProjectionError::ContractClauseMismatch(
            "_mk_event_wait_2".into()
        ))
    );

    let substituted = COMPOSITION_CONTRACT.replace(
        "callback_has_no_resource_effect",
        "callback_may_release_resources",
    );
    assert_eq!(
        MonkeyWaitContracts::parse(WRAPPER_CONTRACT, &substituted),
        Err(WaitSourceProjectionError::ContractClauseMismatch(
            WAIT_CALLBACK.into()
        ))
    );

    let owned_callback = COMPOSITION_CONTRACT.replace(
        "callback_argument: borrow<'callback, const, MonkeyCallbackArgument> use one",
        "callback_argument: own<MonkeyCallbackArgument> use one",
    );
    assert_eq!(
        MonkeyWaitContracts::parse(WRAPPER_CONTRACT, &owned_callback),
        Err(WaitSourceProjectionError::ContractHeaderMismatch(
            WAIT_CALLBACK.into()
        ))
    );

    let effectful_callback = COMPOSITION_CONTRACT.replacen(
        "extern fn cb_wait_2_timeout(\n    callback_argument: borrow<'callback, const, MonkeyCallbackArgument> use one\n) -> unit\ncontract {",
        "extern fn cb_wait_2_timeout(\n    callback_argument: borrow<'callback, const, MonkeyCallbackArgument> use one\n) -> unit\neffects { io }\ncontract {",
        1,
    );
    assert_eq!(
        MonkeyWaitContracts::parse(WRAPPER_CONTRACT, &effectful_callback),
        Err(WaitSourceProjectionError::ContractHeaderMismatch(
            WAIT_CALLBACK.into()
        ))
    );
}

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
                summary(&callbacks, WAIT_CALLBACK),
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
