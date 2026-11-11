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

//! Wire compatibility and admission rules for the reactor program projection.
//!
//! These are format checks only. Nothing here establishes model authority: the
//! trusted core binds a projection through the task digest and runs its own
//! reactor model checker before any obligation is covered.
use qcore_schemas::obligation::{
    Obligation, ObligationProperty, ObligationScope, ObligationSubject, TaskBounds,
};
use qcore_schemas::reactor_program::{
    REACTOR_PROGRAM_INPUT_KEY, REACTOR_PROGRAM_INPUT_VERSION, ReactorProgramInput,
};
use qcore_schemas::{
    PluginId, TargetProfile, UnitIdentity, VerificationRequestV2, VerifierCapability,
};
use serde_json::json;

fn projection() -> serde_json::Value {
    let event = json!({"id": 1, "generation": 1});
    json!({
        "schema_version": REACTOR_PROGRAM_INPUT_VERSION,
        "library": "libevent",
        "source_revision": "revision",
        "build_profile": {"backend": "epoll", "threads": "pthreads"},
        "live_arguments": [9],
        "operations": [
            {"operation": "create_base", "base": 1},
            {"operation": "assign", "event": event, "base": 1, "descriptor": 3, "flags": 2,
             "priority": 0, "summary": {"argument": 9, "argument_mode": "owned",
                "trigger_reasons": ["read"],
                "payload_authority": [{"payload": 7, "kind": "write"}],
                "may_defer": false, "may_spawn_tasks": false}},
            {"operation": "add", "event": event},
            {"operation": "callback_enter", "event": event, "reason": "read"},
            {"operation": "access", "context": {"kind": "event", "event": event},
             "payload": 7, "kind": "write", "synchronized": false},
            {"operation": "callback_exit", "event": event},
            {"operation": "free_event", "event": event},
            {"operation": "free_base", "base": 1}
        ],
        "profile_assumptions": {}
    })
}

fn obligation(property: ObligationProperty) -> Obligation {
    Obligation {
        obligation_id: format!("{property}|unit"),
        capability: VerifierCapability::EventTraces,
        property,
        subject: ObligationSubject::Unit,
        scope: ObligationScope::WholeUnit,
        bounds: TaskBounds::single_path(),
        assumptions: Default::default(),
        evidence_version: 2,
    }
}

fn pair() -> Vec<Obligation> {
    vec![
        obligation(ObligationProperty::EventGenerationDischarged),
        obligation(ObligationProperty::CallbackQuiescentAtTeardown),
    ]
}

fn request(
    program: Option<ReactorProgramInput>,
    obligations: Vec<Obligation>,
) -> VerificationRequestV2 {
    VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "reactor".into(),
        plugin_id: PluginId::new("qcore.libevent").unwrap(),
        capability: VerifierCapability::EventTraces,
        target: TargetProfile {
            library: "libevent".into(),
            revision: "revision".into(),
            profile: "qcore.reactor_callback_program.v1".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "reactor.c".into(),
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "c".repeat(64),
        obligations,
        source_effect: None,
        counter_effect: None,
        reactor_program: program,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: None,
    }
}

fn input() -> ReactorProgramInput {
    serde_json::from_value(projection()).unwrap()
}

#[test]
fn the_projection_round_trips_and_pins_its_identity() {
    let value = projection();
    let parsed: ReactorProgramInput = serde_json::from_value(value.clone()).unwrap();
    assert!(parsed.version_matches());
    assert_eq!(parsed.digest_key(), REACTOR_PROGRAM_INPUT_KEY);
    assert_eq!(REACTOR_PROGRAM_INPUT_KEY, "qcore:reactor-program-input:v1");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), value);
    // The projection carries the program and nothing derived from it.
    assert!(value.get("callbacks").is_none() && value.get("proposal").is_none());

    // The digest covers the whole record: any change moves it.
    let mut moved = parsed.clone();
    moved.build_profile.insert("backend".into(), "poll".into());
    assert_ne!(parsed.digest(), moved.digest());
    let mut relabelled = parsed.clone();
    relabelled.source_revision = "other".into();
    assert_ne!(parsed.digest(), relabelled.digest());
}

#[test]
fn unknown_fields_and_unsupported_operations_are_rejected_not_reinterpreted() {
    let mut extra = projection();
    extra["unexpected"] = json!(true);
    assert!(serde_json::from_value::<ReactorProgramInput>(extra).is_err());

    // There is no record for bufferevent, socket, ordering or loop operations,
    // and no catch-all variant to absorb one.
    for unsupported in [
        json!({"operation": "create_bufferevent", "bev": {"id": 1, "generation": 1}}),
        json!({"operation": "close_socket", "descriptor": 3, "succeeds": true}),
        json!({"operation": "assume_ordering", "evidence": "priority"}),
        json!({"operation": "loop_request", "base": 1, "request": "break", "cross_thread": false}),
    ] {
        let mut program = projection();
        program["operations"]
            .as_array_mut()
            .unwrap()
            .push(unsupported);
        assert!(serde_json::from_value::<ReactorProgramInput>(program).is_err());
    }
    // An access attributed to a bufferevent context has no context record.
    let mut bufferevent = projection();
    bufferevent["operations"][4]["context"] =
        json!({"kind": "bufferevent", "bev": {"id": 1, "generation": 1}});
    assert!(serde_json::from_value::<ReactorProgramInput>(bufferevent).is_err());
}

#[test]
fn a_request_without_a_projection_keeps_its_previous_wire_shape() {
    let plain = request(None, Vec::new());
    let value = serde_json::to_value(&plain).unwrap();
    assert!(value.get("reactor_program").is_none());
    assert!(value.get("counter_effect").is_none());
    assert!(value.get("source_effect").is_none());
    plain.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<VerificationRequestV2>(value).unwrap(),
        plain
    );
    // The generation property alone still travels without a projection: it
    // predates this route and other exchanges name it.
    request(
        None,
        vec![obligation(ObligationProperty::EventGenerationDischarged)],
    )
    .validate()
    .unwrap();
}

#[test]
fn a_projection_is_admitted_only_with_its_own_obligation_pair() {
    request(Some(input()), pair()).validate().unwrap();

    // The quiescence obligation is the marker: it may not travel alone.
    assert!(
        request(
            None,
            vec![obligation(ObligationProperty::CallbackQuiescentAtTeardown)]
        )
        .validate()
        .is_err()
    );
    // A projection needs exactly the pair, over the whole unit.
    assert!(request(Some(input()), Vec::new()).validate().is_err());
    assert!(
        request(
            Some(input()),
            vec![obligation(ObligationProperty::EventGenerationDischarged)]
        )
        .validate()
        .is_err()
    );
    let mut extra = pair();
    extra.push(Obligation {
        obligation_id: "third".into(),
        ..obligation(ObligationProperty::EventGenerationDischarged)
    });
    assert!(request(Some(input()), extra).validate().is_err());
    let mut function_scoped = pair();
    function_scoped[0].subject = ObligationSubject::Function {
        symbol: "f".into(),
        translation_unit: "reactor.c".into(),
    };
    assert!(request(Some(input()), function_scoped).validate().is_err());

    // An unsupported input version, a foreign capability and a co-travelling
    // source projection each fail validation.
    let mut wrong_version = input();
    wrong_version.schema_version = 0;
    assert!(request(Some(wrong_version), pair()).validate().is_err());

    let mut foreign_capability = request(Some(input()), pair());
    foreign_capability.capability = VerifierCapability::ResourceContracts;
    for obligation in &mut foreign_capability.obligations {
        obligation.capability = VerifierCapability::ResourceContracts;
    }
    assert!(foreign_capability.validate().is_err());

    let mut both = request(Some(input()), pair());
    both.source_effect = serde_json::from_str(
        r#"{"schema_version":2,"function":"f","source_revision":"x","field":"Locked",
            "program":{"Return":"Field"},"dependencies":{},"profile_assumptions":{}}"#,
    )
    .unwrap();
    assert!(both.validate().is_err());
}
