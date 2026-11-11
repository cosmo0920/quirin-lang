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

//! Wire compatibility and admission rules for the call-result def/use
//! projection.
//!
//! These are format checks only. Nothing here establishes source identity: the
//! trusted core binds a projection through the task digest and re-evaluates
//! the pinned contract before any obligation is covered.
use qcore_schemas::call_use_summary::{
    CALL_USE_SOURCE_INPUT_KEY, CallUseSourceInput, PrerequisiteIdentity,
};
use qcore_schemas::obligation::{
    Obligation, ObligationProperty, ObligationScope, ObligationSubject, PathBounds, TaskBounds,
};
use qcore_schemas::{
    PluginId, TargetProfile, UnitIdentity, VerificationRequestV2, VerifierCapability,
};
use serde_json::json;

fn field(name: &str) -> serde_json::Value {
    json!({"c_type":"int","expression":{"kind":"field",
        "base":{"c_type":"struct cio_file *","expression":{"kind":"parameter","index":0}},
        "name":name}})
}

fn projection() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "family": {
            "family": "filesystem_attempt",
            "body": {
                "function": "cio_file_native_close",
                "source_revision": "revision",
                "signature": "int (struct cio_file *)",
                "paths": [{"operations": [
                    {"kind":"read","place": field("fd")},
                    {"kind":"call","function":"close","signature":"int (int)",
                     "arguments":[{"c_type":"int","expression":{"kind":"symbol","operation":0}}],
                     "result_type":"int"},
                    {"kind":"local_write","local":1,
                     "value":{"c_type":"int","expression":{"kind":"symbol","operation":1}}},
                    {"kind":"guard","taken":true,
                     "condition":{"c_type":"int","expression":{"kind":"symbol","operation":1}}},
                    {"kind":"local_end","local":1},
                    {"kind":"return_value",
                     "value":{"c_type":"int","expression":{"kind":"integer","value":"0"}}}
                ]}],
                "profile_assumptions": {}
            }
        }
    })
}

fn obligation(property: ObligationProperty, symbol: &str) -> Obligation {
    Obligation {
        obligation_id: format!("{property}|{symbol}"),
        capability: VerifierCapability::ResourceContracts,
        property,
        subject: ObligationSubject::Function {
            symbol: symbol.into(),
            translation_unit: "cio_file_unix.c".into(),
        },
        scope: ObligationScope::Functions {
            symbols: [symbol.to_owned()].into(),
        },
        bounds: TaskBounds {
            path_exploration: PathBounds::BoundedPaths { paths: 1 },
            schedules_explored: None,
            observed_runs_only: false,
            max_operations: None,
        },
        assumptions: Default::default(),
        evidence_version: 2,
    }
}

fn request(
    call_use: Option<CallUseSourceInput>,
    obligations: Vec<Obligation>,
) -> VerificationRequestV2 {
    VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "call-use".into(),
        plugin_id: PluginId::new("qcore.chunkio").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "revision".into(),
            profile: "qcore.c_call_result_use.v1".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "cio_file_unix.c".into(),
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "c".repeat(64),
        obligations,
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: call_use,
        cleanup_replay_source: None,
        composition_source: None,
    }
}

#[test]
fn a_request_without_a_projection_keeps_its_previous_wire_shape() {
    let plain = request(None, Vec::new());
    let value = serde_json::to_value(&plain).unwrap();
    // All four earlier projections, this fifth one, the sixth
    // (cleanup-replay) and the seventh (composition) serialize away, so an
    // existing request's bytes are unchanged by the 8.0.0, 9.0.0 and 10.0.0
    // additions alike.
    for key in [
        "source_effect",
        "counter_effect",
        "reactor_program",
        "teardown_source",
        "call_use_source",
        "cleanup_replay_source",
        "composition_source",
    ] {
        assert!(value.get(key).is_none(), "{key} appeared in the wire form");
    }
    plain.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<VerificationRequestV2>(value).unwrap(),
        plain
    );
    assert_eq!(CALL_USE_SOURCE_INPUT_KEY, "qcore:call-use-source-input:v1");
}

#[test]
fn a_projection_travels_with_exactly_its_own_obligation() {
    let input: CallUseSourceInput = serde_json::from_value(projection()).unwrap();
    assert!(input.version_matches());
    assert_eq!(input.subject_function(), "cio_file_native_close");
    assert_eq!(input.digest_key(), CALL_USE_SOURCE_INPUT_KEY);

    let good = request(
        Some(input.clone()),
        vec![obligation(
            ObligationProperty::CallResultUseContract,
            "cio_file_native_close",
        )],
    );
    good.validate().unwrap();

    // The obligation without the projection, and the projection without the
    // obligation, are both refused.
    assert!(
        request(
            None,
            vec![obligation(
                ObligationProperty::CallResultUseContract,
                "cio_file_native_close"
            )]
        )
        .validate()
        .is_err()
    );
    assert!(request(Some(input.clone()), Vec::new()).validate().is_err());

    // Another function's obligation, another property, and two obligations.
    assert!(
        request(
            Some(input.clone()),
            vec![obligation(
                ObligationProperty::CallResultUseContract,
                "cio_file_native_unmap"
            )]
        )
        .validate()
        .is_err()
    );
    assert!(
        request(
            Some(input.clone()),
            vec![obligation(
                ObligationProperty::ListUnlinkContract,
                "cio_file_native_close"
            )]
        )
        .validate()
        .is_err()
    );
    let mut two = good.clone();
    two.obligations.push(obligation(
        ObligationProperty::CustodyDischarged,
        "cio_file_native_close",
    ));
    assert!(two.validate().is_err());

    // An unsupported projection version, in both directions.
    let mut moved = good.clone();
    moved.call_use_source.as_mut().unwrap().schema_version = 2;
    assert!(moved.validate().is_err());

    // Never together with another projection.
    let mut paired = good.clone();
    paired.teardown_source = serde_json::from_value(json!({
        "schema_version": 1,
        "family": {
            "family": "field_destructor",
            "destructor": {
                "function": "cio_memfs_close",
                "source_revision": "revision",
                "signature": "void (struct cio_chunk *)",
                "paths": [{"operations": [{"kind":"return"}]}],
                "profile_assumptions": {}
            },
            "root_c_type": "struct cio_chunk *",
            "declared_owned_fields": ["backend"]
        }
    }))
    .unwrap();
    assert!(paired.validate().is_err());

    // A wrong capability.
    let mut capability = good.clone();
    capability.capability = VerifierCapability::ChunkLifecycle;
    capability.obligations[0].capability = VerifierCapability::ChunkLifecycle;
    assert!(capability.validate().is_err());
}

#[test]
fn the_vocabulary_rejects_unknown_fields_families_and_operations() {
    let mut unknown = projection();
    unknown["family"]["body"]["extra"] = json!(1);
    assert!(serde_json::from_value::<CallUseSourceInput>(unknown).is_err());

    let mut foreign_family = projection();
    foreign_family["family"]["family"] = json!("list_unlink");
    assert!(serde_json::from_value::<CallUseSourceInput>(foreign_family).is_err());

    // `scan_loop` and `terminate` are the two operation forms the projector
    // refuses by name; they have no wire representation either.
    for kind in ["scan_loop", "terminate", "local_storage"] {
        let mut foreign = projection();
        foreign["family"]["body"]["paths"][0]["operations"][0] = json!({"kind": kind});
        assert!(
            serde_json::from_value::<CallUseSourceInput>(foreign).is_err(),
            "`{kind}` decoded into the vocabulary"
        );
    }
    // The expression forms outside the vocabulary have no wire form either.
    for kind in [
        "local_storage",
        "loop_index",
        "size_of",
        "aggregate",
        "index",
        "function",
    ] {
        let mut foreign = projection();
        foreign["family"]["body"]["paths"][0]["operations"][0]["place"]["expression"] =
            json!({"kind": kind});
        assert!(
            serde_json::from_value::<CallUseSourceInput>(foreign).is_err(),
            "`{kind}` decoded into the expression vocabulary"
        );
    }

    // A projection is data: its digest is a pure function of its bytes.
    let input: CallUseSourceInput = serde_json::from_value(projection()).unwrap();
    let again: CallUseSourceInput = serde_json::from_value(projection()).unwrap();
    assert_eq!(input.digest(), again.digest());
    let mut moved = input.clone();
    moved.schema_version = 2;
    assert_ne!(input.digest(), moved.digest());
}

#[test]
fn a_prerequisite_identity_is_complete_or_it_is_nothing() {
    let identity = PrerequisiteIdentity {
        semantic_profile: "qcore.c_call_result_use.v1".into(),
        task_digest: "d".repeat(64),
        obligation_id: "call_result_use_contract|cio_file_native_close".into(),
    };
    assert!(identity.is_complete());
    assert_eq!(
        PrerequisiteIdentity::DIGEST_KEY,
        "qcore:prerequisite-identity:v1"
    );
    for mutate in [
        |i: &mut PrerequisiteIdentity| i.semantic_profile = String::new(),
        |i: &mut PrerequisiteIdentity| i.task_digest = String::new(),
        |i: &mut PrerequisiteIdentity| i.obligation_id = String::new(),
    ] {
        let mut moved = identity.clone();
        mutate(&mut moved);
        assert!(!moved.is_complete());
        assert_ne!(moved.digest(), identity.digest());
    }
    // Unknown fields never decode into an identity.
    assert!(
        serde_json::from_value::<PrerequisiteIdentity>(json!({
            "semantic_profile": "p", "task_digest": "d", "obligation_id": "o", "verdict": "safe"
        }))
        .is_err()
    );
}
