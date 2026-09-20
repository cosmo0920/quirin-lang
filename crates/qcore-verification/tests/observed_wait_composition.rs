use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceResult, SourceSummary, import_source_functions_from_profile,
};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_schemas::{profile::ImportRole, sha256_hex};
use qcore_verification::event_composition_context::*;
use qcore_verification::monkey_wait_projection::*;
use qcore_verification::observed_wait_composition::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

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

fn fixture() -> (Vec<u8>, Vec<u8>) {
    let identity = |id: u64| json!({"id": id, "generation": 1});
    let actions = vec![
        json!({"kind":"base_create","base":identity(10),"backend":"epoll"}),
        json!({"kind":"event_new","base":identity(10),"event":identity(0x1_0000_0014),
            "argument":identity(30),"callback":"cb_wait_2_timeout","flags":1,
            "priority":0,"result":0}),
        json!({"kind":"event_add","event":identity(0x1_0000_0014),"result":0}),
        json!({"kind":"dispatch_begin","base":identity(10)}),
        json!({"kind":"activate","event":identity(0x1_0000_0014),"reason":"timeout"}),
        json!({"kind":"callback_enter","event":identity(0x1_0000_0014),"argument":identity(30),
            "callback":"cb_wait_2_timeout","reason":"timeout"}),
        json!({"kind":"callback_exit","event":identity(0x1_0000_0014),"argument":identity(30),
            "callback":"cb_wait_2_timeout"}),
        json!({"kind":"dispatch_end","base":identity(10),"result":0}),
        json!({"kind":"event_free","event":identity(0x1_0000_0014)}),
        json!({"kind":"argument_end","argument":identity(30)}),
        json!({"kind":"base_free","base":identity(10)}),
    ];
    let mut observations = actions
        .into_iter()
        .enumerate()
        .map(|(index, action)| {
            serde_json::to_string(&json!({"schema_version":1,"run":1,"buffer":2,
            "local_sequence":index + 1,"dispatcher":3,"action":action}))
            .unwrap()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes();
    observations.push(b'\n');
    let digest = sha256_hex(&observations);
    let source_digest = "a".repeat(64);
    let bindings = serde_json::to_vec(&json!({
        "schema_version":1,
        "profile":"cycle18.timed_wait.single_dispatcher.v1",
        "observations":{"path":"observations.jsonl","sha256":digest,"records":11},
        "context":{"run":1,"buffer":2,"dispatcher":3,"base":identity(10),
            "event":identity(0x1_0000_0014),"argument":identity(30),"callback":"cb_wait_2_timeout",
            "backend":"epoll","flags":1,"priority":0},
        "source_apis":[
            {"api":"mk_event_wait_2","path":"mk_core/mk_event.c","sha256":source_digest},
            {"api":"_mk_event_wait_2","path":"mk_core/mk_event_libevent.c","sha256":source_digest},
            {"api":"cb_wait_2_timeout","path":"mk_core/mk_event_libevent.c","sha256":source_digest}
        ]
    }))
    .unwrap();
    (observations, bindings)
}

fn input(source: &MonkeyWaitSourceProjection) -> EventCompositionContextInput {
    let digest = "a".repeat(64);
    let mut assumptions = BTreeMap::from([
        ("backend".into(), "epoll".into()),
        ("source_file.mk_core/mk_event.c".into(), digest.clone()),
        (
            "source_file.mk_core/mk_event_libevent.c".into(),
            digest.clone(),
        ),
    ]);
    for (name, hash) in source.source_summary_digests() {
        assumptions.insert(format!("source_summary.{name}"), hash.clone());
    }
    for (name, hash) in source.contract_digests() {
        assumptions.insert(format!("contract.{name}"), hash.clone());
    }
    EventCompositionContextInput {
        artifact: "fixture-run".into(),
        semantic_profile: OBSERVED_WAIT_PROFILE.into(),
        scope: source.selected_path_indices().keys().cloned().collect(),
        bounds: BTreeMap::from([
            ("dispatchers".into(), 1),
            ("events".into(), 1),
            ("records".into(), 11),
            ("runs".into(), 1),
        ]),
        assumptions,
        source_compile_task_sha256: digest.clone(),
        fluent_bit_revision: source.source_revision().into(),
        monkey_source_sha256: digest.clone(),
        monkey_profile_sha256: source.profile_assumptions()["profile_digest"].clone(),
        monkey_build_sha256: digest.clone(),
        libevent_revision: "b".repeat(40),
        libevent_source_sha256: digest.clone(),
        libevent_profile_sha256: digest.clone(),
        libevent_build_sha256: digest.clone(),
        contracts_sha256: sha256_hex(&serde_json::to_vec(source.contract_digests()).unwrap()),
        source_rules_sha256: sha256_hex(
            qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES.as_bytes(),
        ),
        observation_rules_sha256: sha256_hex(OBSERVED_WAIT_RULES.as_bytes()),
        observer_binary_sha256: digest.clone(),
        observer_harness_sha256: digest.clone(),
        linked_libraries_sha256: BTreeMap::from([
            ("monkey".into(), digest.clone()),
            ("libevent".into(), digest),
        ]),
        observation_trust: DeclaredObservationTrust::ProducerSuppliedRuntimeEvidence,
    }
}

#[test]
fn complete_join_replays_lifecycle_and_rejects_cross_component_substitutions() {
    // Source summary creation requires the real Clang importer. CI requires Clang.
    let source = project(CALLBACKS, &wrapper_source());
    let (observations, bindings) = fixture();
    let payload = EventCompositionPayload {
        observations: &observations,
        bindings: &bindings,
    };
    let context = EventCompositionContext::derive(input(&source), payload).unwrap();
    let admitted = check_event_composition_context_for(&context, payload, &context).unwrap();
    let result =
        check_observed_wait_composition(&source, &admitted, &observations, &bindings).unwrap();
    let report = serde_json::to_value(result).unwrap();
    assert_eq!(report["checked_operations"], 11);
    assert_eq!(report["checked_records"], 11);
    assert_eq!(report["identity_mapping"]["event"][0], 0x1_0000_0014_u64);
    assert_eq!(report["identity_mapping"]["event"][2], 1);
    for key in [
        "backend",
        "source_file.mk_core/mk_event.c",
        "source_summary.cb_wait_2_timeout",
        "contract.monkey/composition.qcore",
    ] {
        let mut changed = input(&source);
        changed.assumptions.insert(key.into(), "unrelated".into());
        let context = EventCompositionContext::derive(changed, payload).unwrap();
        let admitted = check_event_composition_context_for(&context, payload, &context).unwrap();
        assert!(
            check_observed_wait_composition(&source, &admitted, &observations, &bindings).is_err(),
            "{key}"
        );
    }
    let mut changed = input(&source);
    changed.bounds.insert("runs".into(), 2);
    let context = EventCompositionContext::derive(changed, payload).unwrap();
    let wrong_bound = check_event_composition_context_for(&context, payload, &context).unwrap();
    assert!(
        check_observed_wait_composition(&source, &wrong_bound, &observations, &bindings).is_err()
    );
    let mut changed_bytes = observations.clone();
    changed_bytes.push(b' ');
    assert!(
        check_observed_wait_composition(&source, &admitted, &changed_bytes, &bindings).is_err()
    );
    // Coherently rehashing a malformed trace must still fail identity refinement.
    let mut records: Vec<Value> = std::str::from_utf8(&observations)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    records[5]["action"]["event"]["generation"] = json!(2);
    let malformed = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes();
    let mut binding: Value = serde_json::from_slice(&bindings).unwrap();
    binding["observations"]["sha256"] = json!(sha256_hex(&malformed));
    let malformed_bindings = serde_json::to_vec(&binding).unwrap();
    let payload = EventCompositionPayload {
        observations: &malformed,
        bindings: &malformed_bindings,
    };
    let context = EventCompositionContext::derive(input(&source), payload).unwrap();
    let admitted = check_event_composition_context_for(&context, payload, &context).unwrap();
    assert!(
        check_observed_wait_composition(&source, &admitted, &malformed, &malformed_bindings)
            .is_err()
    );
}
