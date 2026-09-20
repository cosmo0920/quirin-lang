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
const EVIDENCE: &[u8] =
    include_bytes!("../../../docs/reports/cycle18-wait-failures/failure-observations.json");

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

fn import(source: &str, role: ImportRole) -> Vec<SourceResult> {
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
        .unwrap()
}

fn projection() -> MonkeyWaitSourceProjection {
    let callbacks = import(CALLBACKS, ImportRole::Callback);
    let wrappers_source = wrapper_source();
    let wrappers = import(&wrappers_source, ImportRole::Wrapper);
    let contracts = MonkeyWaitContracts::parse(WRAPPER_CONTRACT, COMPOSITION_CONTRACT).unwrap();
    project_monkey_timed_wait_source(
        [
            summary(&callbacks, wait_callback()),
            summary(&wrappers, "_mk_event_loop_create"),
            summary(&wrappers, "_mk_event_wait_with_flags"),
            summary(&wrappers, "_mk_event_wait_2"),
            summary(&wrappers, "_mk_event_loop_destroy"),
        ],
        &contracts,
    )
    .unwrap()
}

#[test]
fn complete_failure_inventory_separates_native_violations_from_unknowns() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    assert_eq!(projection.source_path_counts()["_mk_event_loop_create"], 4);
    assert_eq!(projection.source_path_counts()["_mk_event_wait_2"], 3);
    let checked = check_wait_failure_paths(&projection, EVIDENCE).unwrap();
    assert_eq!(checked.len(), 8);
    assert_eq!(
        checked
            .iter()
            .filter(|row| row.outcome == WaitFailureOutcome::Unsafe)
            .count(),
        3
    );
    assert_eq!(
        checked
            .iter()
            .filter(|row| row.outcome == WaitFailureOutcome::Unknown)
            .count(),
        5
    );
    assert_eq!(
        checked
            .iter()
            .filter(|row| row.evidence_class == WaitFailureEvidenceClass::InstrumentedSourceHarness)
            .count(),
        6
    );
    assert_eq!(
        checked
            .iter()
            .filter(|row| row.evidence_class == WaitFailureEvidenceClass::SourceModel)
            .count(),
        2
    );
    let registration = checked
        .iter()
        .find(|row| row.scenario == "timer_registration_failure")
        .unwrap();
    assert_eq!(registration.outcome, WaitFailureOutcome::Unknown);
    assert!(registration.reason.contains("liveness"));
    let success = checked
        .iter()
        .find(|row| row.scenario == "success_observation")
        .unwrap();
    assert_eq!(success.outcome, WaitFailureOutcome::Unknown);
}

#[test]
fn incomplete_or_mutated_native_evidence_fails_closed() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let projection = projection();
    let baseline: serde_json::Value = serde_json::from_slice(EVIDENCE).unwrap();
    let mutations: Vec<Box<dyn Fn(&mut serde_json::Value)>> = vec![
        Box::new(|value| {
            value.as_array_mut().unwrap().pop();
        }),
        Box::new(|value| value[2]["base_freed"] = true.into()),
        Box::new(|value| value[3]["event_add_argument_nonnull"] = true.into()),
        Box::new(|value| value[4]["timeout_activated"] = true.into()),
        Box::new(|value| {
            value[5]
                .as_object_mut()
                .unwrap()
                .remove("timeout_activated");
        }),
        Box::new(|value| value[0]["source_revision"] = "changed".into()),
        Box::new(|value| value[1]["trace"][2] = "changed".into()),
    ];
    for mutate in mutations {
        let mut changed = baseline.clone();
        mutate(&mut changed);
        assert!(
            check_wait_failure_paths(&projection, &serde_json::to_vec(&changed).unwrap()).is_err()
        );
    }
}
