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

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{
    SourceResult, SourceSummary, import_source_functions_from_profile,
};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_schemas::{profile::ImportRole, sha256_hex};
use qcore_verification::event_composition_context::*;
use qcore_verification::monkey_wait_projection::*;
use qcore_verification::observed_wait_composition::*;
use qcore_verification::timed_wait_observation_wire::decode_timed_wait_observation;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

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
fn windows_wait_source_summary_refuses_the_linux_identity_join() {
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

fn input_with_policy(
    source: &MonkeyWaitSourceProjection,
    policy: &str,
    observations: &[u8],
    bindings: &[u8],
) -> EventCompositionContextInput {
    // These fixture digests are deliberately synthetic. The public verifier is
    // a conditional replay boundary; qcore-audit separately hashes and
    // snapshots configured files before calling it.
    let digest = "a".repeat(64);
    let mut assumptions = BTreeMap::from([
        ("backend".into(), "epoll".into()),
        (
            "threads".into(),
            wait_identity_policy_text("threads").unwrap(),
        ),
        (
            "observation_trust".into(),
            wait_identity_policy_text("observation_trust").unwrap(),
        ),
        (
            "linkage_trust".into(),
            wait_identity_policy_text("linkage_trust").unwrap(),
        ),
        ("native_run_manifest_sha256".into(), "c".repeat(64)),
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
        semantic_profile: observed_wait_profile().unwrap(),
        scope: source.selected_path_indices().keys().cloned().collect(),
        bounds: observed_wait_bounds(
            &decode_timed_wait_observation(observations, bindings)
                .unwrap()
                .trace,
        ),
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
        observation_rules_sha256: wait_observation_rules_digest_with_policy(policy),
        observer_binary_sha256: digest.clone(),
        observer_harness_sha256: digest.clone(),
        linked_libraries_sha256: BTreeMap::from([
            ("monkey".into(), digest.clone()),
            ("libevent".into(), digest),
        ]),
        observation_trust: DeclaredObservationTrust::ProducerSuppliedRuntimeEvidence,
    }
}

fn input(
    source: &MonkeyWaitSourceProjection,
    observations: &[u8],
    bindings: &[u8],
) -> EventCompositionContextInput {
    input_with_policy(source, DEFAULT_WAIT_IDENTITY_POLICY, observations, bindings)
}

#[cfg(not(windows))]
#[test]
fn conditional_replay_rejects_context_and_assumption_substitutions() {
    // Source summary creation requires the real Clang importer. CI requires Clang.
    let source = project(CALLBACKS, &wrapper_source());
    let (observations, bindings) = fixture();
    let payload = EventCompositionPayload {
        observations: &observations,
        bindings: &bindings,
    };
    let context =
        EventCompositionContext::derive(input(&source, &observations, &bindings), payload).unwrap();
    let admitted = check_event_composition_context_for(&context, payload, &context).unwrap();
    let result =
        check_observed_wait_composition(&source, &admitted, &observations, &bindings).unwrap();
    let report = serde_json::to_value(result).unwrap();
    assert_eq!(report["checked_operations"], 11);
    assert_eq!(report["checked_records"], 11);
    assert_eq!(report["claim"], CONDITIONAL_REPLAY_CLAIM);
    assert!(report["caller_prepared_context_sha256"].is_string());
    assert!(report.get("context_sha256").is_none());
    assert_eq!(report["identity_mapping"]["event"][0], 0x1_0000_0014_u64);
    assert_eq!(report["identity_mapping"]["event"][2], 1);
    for key in [
        "backend",
        "threads",
        "observation_trust",
        "linkage_trust",
        "native_run_manifest_sha256",
        "source_file.mk_core/mk_event.c",
        "source_summary.cb_wait_2_timeout",
        "contract.monkey/composition.qv",
    ] {
        let mut changed = input(&source, &observations, &bindings);
        changed.assumptions.insert(key.into(), "unrelated".into());
        let context = EventCompositionContext::derive(changed, payload).unwrap();
        let admitted = check_event_composition_context_for(&context, payload, &context).unwrap();
        assert!(
            check_observed_wait_composition(&source, &admitted, &observations, &bindings).is_err(),
            "{key}"
        );
    }
    for key in [
        "threads",
        "observation_trust",
        "linkage_trust",
        "native_run_manifest_sha256",
    ] {
        let mut changed = input(&source, &observations, &bindings);
        changed.assumptions.remove(key);
        let context = EventCompositionContext::derive(changed, payload).unwrap();
        let admitted = check_event_composition_context_for(&context, payload, &context).unwrap();
        assert!(
            check_observed_wait_composition(&source, &admitted, &observations, &bindings).is_err(),
            "missing {key}"
        );
    }
    let mut changed = input(&source, &observations, &bindings);
    changed
        .assumptions
        .insert("native_run_manifest_sha256".into(), "not-a-sha256".into());
    let context = EventCompositionContext::derive(changed, payload).unwrap();
    let admitted = check_event_composition_context_for(&context, payload, &context).unwrap();
    assert!(check_observed_wait_composition(&source, &admitted, &observations, &bindings).is_err());
    let mut changed = input(&source, &observations, &bindings);
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
    let context =
        EventCompositionContext::derive(input(&source, &observations, &bindings), payload).unwrap();
    let admitted = check_event_composition_context_for(&context, payload, &context).unwrap();
    assert!(
        check_observed_wait_composition(&source, &admitted, &malformed, &malformed_bindings)
            .is_err()
    );
}

#[cfg(not(windows))]
#[test]
fn declarative_identity_policy_is_load_bearing_and_exactly_bound() {
    let source = project(CALLBACKS, &wrapper_source());
    let (observations, bindings) = fixture();
    let payload = EventCompositionPayload {
        observations: &observations,
        bindings: &bindings,
    };

    let default_context =
        EventCompositionContext::derive(input(&source, &observations, &bindings), payload).unwrap();
    let default_admitted =
        check_event_composition_context_for(&default_context, payload, &default_context).unwrap();

    // Even a semantically inert byte change is a different caller-selected
    // policy and cannot run under a context prepared for the bundled policy.
    let byte_changed = format!("{DEFAULT_WAIT_IDENTITY_POLICY}\n// caller-selected variant\n");
    assert!(
        check_observed_wait_composition_with_policy(
            &source,
            &default_admitted,
            &observations,
            &bindings,
            &byte_changed,
        )
        .is_err()
    );

    // Deleting the policy's own digest-equality clause cannot bypass the
    // immutable selected-policy admission check under a stale context.
    let missing_self_binding = DEFAULT_WAIT_IDENTITY_POLICY.replace(
        "    ensures normal observation_rules_sha256_actual == observation_rules_sha256_expected;\n",
        "",
    );
    assert_ne!(missing_self_binding, DEFAULT_WAIT_IDENTITY_POLICY);
    assert!(
        check_observed_wait_composition_with_policy(
            &source,
            &default_admitted,
            &observations,
            &bindings,
            &missing_self_binding,
        )
        .is_err()
    );

    // Rebinding policy identity does not manufacture a producer fact. The
    // fixture still reports producer-supplied runtime evidence, so a policy
    // mutation demanding attested evidence refuses.
    let stronger_trust = DEFAULT_WAIT_IDENTITY_POLICY.replace(
        "observation_trust == \"producer_supplied_runtime_evidence\"",
        "observation_trust == \"attested_runtime_evidence\"",
    );
    let stronger_context = EventCompositionContext::derive(
        input_with_policy(&source, &stronger_trust, &observations, &bindings),
        payload,
    )
    .unwrap();
    let stronger_admitted =
        check_event_composition_context_for(&stronger_context, payload, &stronger_context).unwrap();
    assert!(
        check_observed_wait_composition_with_policy(
            &source,
            &stronger_admitted,
            &observations,
            &bindings,
            &stronger_trust,
        )
        .is_err()
    );

    // Rebinding the context to a clause mutation proves the policy is
    // executed: the record bound previously accepted by Rust is now refused.
    let changed_clause = DEFAULT_WAIT_IDENTITY_POLICY.replace(
        "ensures normal records_bound_actual == 11;",
        "ensures normal records_bound_actual == 12;",
    );
    let changed_context = EventCompositionContext::derive(
        input_with_policy(&source, &changed_clause, &observations, &bindings),
        payload,
    )
    .unwrap();
    let changed_admitted =
        check_event_composition_context_for(&changed_context, payload, &changed_context).unwrap();
    assert!(
        check_observed_wait_composition_with_policy(
            &source,
            &changed_admitted,
            &observations,
            &bindings,
            &changed_clause,
        )
        .is_err()
    );

    // A policy can never hide an observation it requires. The typed engine
    // rejects the absent primitive instead of treating it as false or empty.
    let missing_fact = DEFAULT_WAIT_IDENTITY_POLICY.replace(
        "ensures normal records_bound_actual == 11;",
        "ensures normal missing_primitive == 11;",
    );
    let missing_context = EventCompositionContext::derive(
        input_with_policy(&source, &missing_fact, &observations, &bindings),
        payload,
    )
    .unwrap();
    let missing_admitted =
        check_event_composition_context_for(&missing_context, payload, &missing_context).unwrap();
    assert!(
        check_observed_wait_composition_with_policy(
            &source,
            &missing_admitted,
            &observations,
            &bindings,
            &missing_fact,
        )
        .is_err()
    );
}

#[test]
fn observed_bounds_are_derived_from_trace_data() {
    let (observations, bindings) = fixture();
    let decoded = decode_timed_wait_observation(&observations, &bindings).unwrap();
    assert_eq!(
        observed_wait_bounds(&decoded.trace),
        BTreeMap::from([
            ("dispatchers".into(), 1),
            ("events".into(), 1),
            ("records".into(), 11),
            ("runs".into(), 1),
        ])
    );

    let mut mixed = decoded.trace.clone();
    mixed.records[5].run = 2;
    mixed.records[5].dispatcher = 4;
    if let qcore_core::event_observation::EventObservationAction::CallbackEnter { event, .. } =
        &mut mixed.records[5].action
    {
        event.id += 1;
    } else {
        panic!("fixture callback-enter record moved");
    }
    let mixed_bounds = observed_wait_bounds(&mixed);
    assert_eq!(mixed_bounds["runs"], 2);
    assert_eq!(mixed_bounds["dispatchers"], 2);
    assert_eq!(mixed_bounds["events"], 2);

    mixed.records.pop();
    assert_eq!(observed_wait_bounds(&mixed)["records"], 10);
}
