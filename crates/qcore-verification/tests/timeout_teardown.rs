use std::path::PathBuf;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceResult, SourceSummary, import_source_functions_from_profile,
};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_schemas::profile::ImportRole;
use qcore_verification::monkey_wait_projection::monkey_source_revision;
use qcore_verification::timeout_teardown::*;

const CALLBACKS: &str = include_str!("../../../compiler/tests/fixtures/monkey/callbacks.c");
const LIBEVENT_EVENT_C: &[u8] = include_bytes!("../../../third_party/libevent/event.c");
const LIBEVENT_REVISION: &str = "d82464a277d0f42703702c4dfd9af6af38595a83";

fn imported(source: &str) -> Vec<SourceResult> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profile = load_source_set_profile(&root.join("contract/profiles"), "fluent-bit").unwrap();
    import_source_functions_from_profile(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        monkey_source_revision(),
        &profile,
        "monkey",
        ImportRole::Callback,
    )
    .unwrap()
}

fn summary(results: &[SourceResult]) -> &SourceSummary {
    results
        .iter()
        .find_map(|result| match result {
            SourceResult::Summary(summary) if summary.function() == "cb_timeout" => Some(summary),
            _ => None,
        })
        .expect("cb_timeout summary")
}

fn projection() -> TimeoutTeardownSourceProjection {
    let imported = imported(CALLBACKS);
    project_cb_timeout_source(
        summary(&imported),
        &TimeoutTeardownContracts::parse().unwrap(),
        LIBEVENT_REVISION,
        LIBEVENT_EVENT_C,
    )
    .unwrap()
}

fn evidence(projection: &TimeoutTeardownSourceProjection) -> TimeoutTeardownEvidence {
    TimeoutTeardownEvidence {
        source_summary_digest: projection.source_summary_digest().into(),
        libevent_event_c_digest: projection.libevent_event_c_digest().into(),
        contract_digest: projection.contract_digest().into(),
        path_index: 1,
        native_trace: projection.required_native_trace().to_vec(),
        base: 2,
        event_id: 4,
        event_generation: 1,
        dispatcher: 1,
        map_argument: 400,
        peer_descriptor: 7,
        callback_descriptor: 8,
        callback_close: CloseObservation::Success,
        peer_close: CloseObservation::Success,
    }
}

#[test]
fn sealed_source_and_pinned_libevent_derive_both_teardown_paths() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    assert_eq!(projection.teardown_path_indices(), &[0, 1]);
    assert_eq!(
        projection.required_native_trace(),
        [
            "callback_enter",
            "close_socket",
            "event_del",
            "event_free",
            "map_free",
            "callback_exit"
        ]
    );
}

#[test]
fn exact_native_evidence_discharges_distinct_custody() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    let certificate = check_timeout_teardown_evidence(&projection, &evidence(&projection)).unwrap();
    assert_eq!(certificate.freed_event.id, 4);
    assert_eq!(certificate.dropped_map_argument, 400);
    assert_eq!(certificate.closed_descriptors, vec![7, 8]);
}

#[test]
fn linux_failed_close_is_platform_qualified_and_retires_the_number() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    let mut evidence = evidence(&projection);
    evidence.callback_close = CloseObservation::Failure {
        target: "linux".into(),
        errno: "eintr".into(),
        record_locks: "none-held".into(),
    };
    assert!(check_timeout_teardown_evidence(&projection, &evidence).is_ok());
    evidence.callback_close = CloseObservation::Failure {
        target: "unpinned".into(),
        errno: "eintr".into(),
        record_locks: "none-held".into(),
    };
    assert!(check_timeout_teardown_evidence(&projection, &evidence).is_err());
}

#[test]
fn source_order_and_evidence_binding_are_load_bearing() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let changed = CALLBACKS.replace(
        "event_del(ev_map->event);\n        event_free(ev_map->event);",
        "event_free(ev_map->event);\n        event_del(ev_map->event);",
    );
    let changed_import = imported(&changed);
    assert!(
        project_cb_timeout_source(
            summary(&changed_import),
            &TimeoutTeardownContracts::parse().unwrap(),
            LIBEVENT_REVISION,
            LIBEVENT_EVENT_C
        )
        .is_err()
    );

    let projection = projection();
    let mut wrong = evidence(&projection);
    wrong.native_trace.swap(2, 3);
    assert!(check_timeout_teardown_evidence(&projection, &wrong).is_err());
    let mut same_descriptor = evidence(&projection);
    same_descriptor.peer_descriptor = same_descriptor.callback_descriptor;
    assert!(check_timeout_teardown_evidence(&projection, &same_descriptor).is_err());
}

#[test]
fn evidence_json_refuses_unknown_fields() {
    let value = r#"{"source_summary_digest":"x","libevent_event_c_digest":"x","contract_digest":"x","path_index":0,"native_trace":[],"base":1,"event_id":1,"event_generation":1,"dispatcher":1,"map_argument":1,"peer_descriptor":3,"callback_descriptor":4,"callback_close":{"result":"success"},"peer_close":{"result":"success"},"claimed_safe":true}"#;
    assert!(serde_json::from_str::<TimeoutTeardownEvidence>(value).is_err());
}
