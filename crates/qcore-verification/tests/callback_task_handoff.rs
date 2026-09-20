use std::path::PathBuf;
use std::process::Command;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceResult, SourceSummary, import_source_functions_from_profile,
};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_schemas::profile::ImportRole;
use qcore_verification::callback_task_handoff::{
    CallbackTaskHandoffError, CallbackTaskObservation, CallbackTaskObservationAction as Action,
    CallbackTaskObservationBundle, check_callback_task_observations,
    check_taskful_callback_observations, project_callback_task_source,
    project_taskful_callback_source,
};
use qcore_verification::monkey_wait_projection::monkey_source_revision;

const CALLBACKS: &str = include_str!("../../../compiler/tests/fixtures/monkey/callbacks.c");
const TASKFUL_SOURCE: &str =
    include_str!("../../../compiler/tests/fixtures/cycle18_task_handoff.c");
const CB_EVENT_NATIVE: &str = include_str!(
    "../../../docs/reports/cycle18-priority1-integration/payload-native/observations.jsonl"
);
const CB_TIMEOUT_NATIVE: &str = include_str!(
    "../../../docs/reports/cycle18-priority1-integration/timeout-native/monkey_observation.jsonl"
);
const CB_WAIT_NATIVE: &str =
    include_str!("../../../docs/reports/cycle18-observed-integration/native/observations.jsonl");

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

fn summaries(results: &[SourceResult]) -> Vec<SourceSummary> {
    results
        .iter()
        .filter_map(|result| match result {
            SourceResult::Summary(summary) => Some(summary.clone()),
            SourceResult::Unknown { .. } => None,
        })
        .collect()
}

fn taskful_imported(source: &str, role: ImportRole) -> Vec<SourceSummary> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profile =
        load_source_set_profile(&root.join("contract/profiles"), "cycle18-task-handoff").unwrap();
    import_source_functions_from_profile(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "229578d8ad617803bed3f02bcc1f0f59251d560d",
        &profile,
        "fixture",
        role,
    )
    .unwrap()
    .into_iter()
    .map(|result| match result {
        SourceResult::Summary(summary) => summary,
        SourceResult::Unknown { function, reason } => {
            panic!("taskful source `{function}` refused: {reason}")
        }
    })
    .collect()
}

fn observations() -> Vec<CallbackTaskObservation> {
    ["cb_event", "cb_timeout", "cb_wait_2_timeout"]
        .into_iter()
        .enumerate()
        .flat_map(|(index, callback)| {
            let base = (index as u64) * 2;
            [
                CallbackTaskObservation {
                    schema_version: 1,
                    sequence: base + 1,
                    thread: None,
                    order: None,
                    action: Action::CallbackEnter {
                        callback: callback.into(),
                        argument: index as u32 + 10,
                    },
                },
                CallbackTaskObservation {
                    schema_version: 1,
                    sequence: base + 2,
                    thread: None,
                    order: None,
                    action: Action::CallbackExit {
                        callback: callback.into(),
                    },
                },
            ]
        })
        .collect()
}

fn bundle(records: Vec<CallbackTaskObservation>) -> CallbackTaskObservationBundle {
    CallbackTaskObservationBundle {
        schema_version: 1,
        producer: "test-complete-callback-task-observer".into(),
        capabilities: vec![
            "callback_boundaries".into(),
            "task_lifecycle".into(),
            "deferred_lifecycle".into(),
        ],
        records,
    }
}

#[test]
fn all_three_callbacks_and_eight_paths_have_declared_no_task_effects() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let imported = imported(CALLBACKS);
    let projection = project_callback_task_source(&summaries(&imported)).unwrap();
    assert_eq!(
        projection.callbacks(),
        ["cb_event", "cb_timeout", "cb_wait_2_timeout"]
    );
    assert_eq!(projection.total_paths(), 8);
}

#[test]
fn bounded_no_task_observation_rechecks_the_generic_kernel() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let imported = imported(CALLBACKS);
    let projection = project_callback_task_source(&summaries(&imported)).unwrap();
    let certificate =
        check_callback_task_observations(&projection, &bundle(observations())).unwrap();
    assert!(certificate.no_task_or_deferred_action_observed);
    assert_eq!(certificate.callbacks_observed.len(), 3);
}

#[test]
fn observed_task_action_and_incomplete_windows_refuse() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let imported = imported(CALLBACKS);
    let projection = project_callback_task_source(&summaries(&imported)).unwrap();
    let mut task = observations();
    task.insert(
        1,
        CallbackTaskObservation {
            schema_version: 1,
            sequence: 1,
            thread: None,
            order: None,
            action: Action::TaskSpawn {
                callback: "cb_event".into(),
                task: 7,
            },
        },
    );
    // Renumber to isolate the semantic refusal from ordering.
    for (index, row) in task.iter_mut().enumerate() {
        row.sequence = index as u64 + 1;
    }
    assert!(matches!(
        check_callback_task_observations(&projection, &bundle(task)),
        Err(CallbackTaskHandoffError::Observation(_))
    ));
    let mut incomplete = observations();
    incomplete.pop();
    assert!(check_callback_task_observations(&projection, &bundle(incomplete)).is_err());
}

#[test]
fn observation_without_task_completeness_capability_refuses() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let imported = imported(CALLBACKS);
    let projection = project_callback_task_source(&summaries(&imported)).unwrap();
    let mut evidence = bundle(observations());
    evidence
        .capabilities
        .retain(|name| name != "task_lifecycle");
    assert!(check_callback_task_observations(&projection, &evidence).is_err());
}

#[test]
fn retained_native_instances_cover_callbacks_but_carry_no_task_observer_capability() {
    let mut callbacks = std::collections::BTreeSet::new();
    for line in [CB_EVENT_NATIVE, CB_TIMEOUT_NATIVE, CB_WAIT_NATIVE]
        .into_iter()
        .flat_map(str::lines)
    {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        if let Some(callback) = value
            .get("action")
            .and_then(|action| action.get("callback"))
            .and_then(serde_json::Value::as_str)
        {
            callbacks.insert(callback.to_owned());
        }
        assert!(value.get("observer_capabilities").is_none());
    }
    assert_eq!(
        callbacks,
        ["cb_event", "cb_timeout", "cb_wait_2_timeout"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}

#[test]
fn observed_call_must_belong_to_that_callbacks_source_paths() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let imported = imported(CALLBACKS);
    let projection = project_callback_task_source(&summaries(&imported)).unwrap();
    let mut records = observations();
    records.insert(
        1,
        CallbackTaskObservation {
            schema_version: 1,
            sequence: 1,
            thread: None,
            order: None,
            action: Action::ExternalCall {
                callback: "cb_event".into(),
                function: "send".into(),
            },
        },
    );
    for (index, row) in records.iter_mut().enumerate() {
        row.sequence = index as u64 + 1;
    }
    assert!(check_callback_task_observations(&projection, &bundle(records)).is_err());
}

#[test]
fn unsupported_transitive_callee_effect_fails_closed() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let changed = CALLBACKS
        .replace(
            "void mk_mem_free(void *);",
            "void mk_mem_free(void *);\nvoid task_spawn(void);",
        )
        .replace("perror(\"write\");", "task_spawn();");
    let imported = imported(&changed);
    assert!(project_callback_task_source(&summaries(&imported)).is_err());
}

#[test]
fn observation_json_refuses_unknown_fields() {
    let value = serde_json::json!({
        "schema_version": 1,
        "sequence": 1,
        "action": {"action": "callback_exit", "callback": "cb_event", "safe": true}
    });
    assert!(serde_json::from_value::<CallbackTaskObservation>(value).is_err());
}

#[test]
fn taskful_source_and_native_pthread_run_match_exactly() {
    if qcore_c_gateway::clang_command().is_err() || !cfg!(target_family = "unix") {
        return;
    }
    let callback = taskful_imported(TASKFUL_SOURCE, ImportRole::Callback);
    let after = taskful_imported(TASKFUL_SOURCE, ImportRole::Wrapper);
    let projection = project_taskful_callback_source(&callback[0], &after[0]).unwrap();
    assert_eq!(projection.operations().len(), 12);

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("compiler/tests/fixtures/cycle18_task_handoff.c");
    let binary =
        std::env::temp_dir().join(format!("qcore-cycle18-task-handoff-{}", std::process::id()));
    let mut compiler = qcore_c_gateway::clang_command().unwrap();
    let compiled = compiler
        .args([
            "-std=c11",
            "-pthread",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-Wno-unused-parameter",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&binary).output().unwrap();
    let _ = std::fs::remove_file(&binary);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let observed: CallbackTaskObservationBundle = serde_json::from_slice(&run.stdout).unwrap();
    let certificate = check_taskful_callback_observations(&projection, &observed).unwrap();
    assert_eq!(certificate.observation_count, 12);
    assert!(!certificate.no_task_or_deferred_action_observed);
}

#[test]
fn taskful_source_identity_arguments_are_load_bearing() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let changed = TASKFUL_SOURCE.replacen("qcore_task_join(1, 10);", "qcore_task_join(1, 11);", 1);
    let callback = taskful_imported(&changed, ImportRole::Callback);
    let after = taskful_imported(&changed, ImportRole::Wrapper);
    assert!(project_taskful_callback_source(&callback[0], &after[0]).is_err());
}
