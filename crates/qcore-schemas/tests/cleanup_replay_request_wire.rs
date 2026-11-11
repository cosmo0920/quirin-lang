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

//! Wire compatibility and admission rules for the symbolic cleanup-replay
//! projection — the sixth optional projection on a v2 request.
//!
//! These are format checks only. Nothing here establishes source identity: the
//! trusted core binds a projection through the task digest, rebuilds the
//! declared profile from its own contract, re-establishes the prerequisite and
//! re-runs its own replay before any obligation is covered.
use qcore_schemas::cleanup_replay_summary::{
    CLEANUP_REPLAY_SEMANTIC_PROFILE, CLEANUP_REPLAY_SOURCE_INPUT_KEY, CleanupEntryRecord,
    CleanupLatticeValue, CleanupPrestateRecord, CleanupReplaySourceInput,
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
        "prerequisite": {
            "semantic_profile": "qcore.c_call_result_use.v1",
            "task_digest": "a".repeat(64),
            "obligation_id": "resource_contracts:0001"
        },
        "family": {
            "family": "native_cleanup_outcome",
            "body": {
                "function": "cio_file_native_close",
                "source_revision": "revision",
                "signature": "int (struct cio_file *)",
                "paths": [{"operations": [
                    {"kind":"read","place": field("fd")},
                    {"kind":"call","function":"close","signature":"int (int)",
                     "arguments":[{"c_type":"int","expression":{"kind":"symbol","operation":0}}],
                     "result_type":"int"},
                    {"kind":"return_value",
                     "value":{"c_type":"int","expression":{"kind":"integer","value":"0"}}}
                ]}],
                "profile_assumptions": {}
            },
            "profile": {
                "function": "cio_file_native_close",
                "root_type": "struct cio_file *",
                "syscall": "close",
                "syscall_signature": "int (int)",
                "presence_field": "fd",
                "fields": [{"name":"fd","c_type":"int","reset":{"reset":"integer","value":-1}}],
                "argument_fields": ["fd"],
                "errno_function": "__errno_location",
                "report_function": "cio_errno_print"
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
    input: Option<CleanupReplaySourceInput>,
    obligations: Vec<Obligation>,
) -> VerificationRequestV2 {
    VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "cleanup-replay".into(),
        plugin_id: PluginId::new("qcore.chunkio").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "revision".into(),
            profile: CLEANUP_REPLAY_SEMANTIC_PROFILE.into(),
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
        call_use_source: None,
        cleanup_replay_source: input,
        composition_source: None,
    }
}

#[test]
fn a_request_without_a_projection_keeps_its_previous_wire_shape() {
    let plain = request(None, Vec::new());
    let value = serde_json::to_value(&plain).unwrap();
    // All six earlier projections and the seventh serialize away, so an
    // existing request's bytes are unchanged by the 9.0.0 and 10.0.0 additions.
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
    assert_eq!(
        CLEANUP_REPLAY_SOURCE_INPUT_KEY,
        "qcore:cleanup-replay-source-input:v1"
    );
    assert_eq!(CLEANUP_REPLAY_SEMANTIC_PROFILE, "qcore.c_cleanup_replay.v1");
}

#[test]
fn a_projection_travels_with_exactly_its_own_obligation() {
    let input: CleanupReplaySourceInput = serde_json::from_value(projection()).unwrap();
    assert!(input.version_matches());
    assert_eq!(input.subject_function(), "cio_file_native_close");
    assert_eq!(input.digest_key(), CLEANUP_REPLAY_SOURCE_INPUT_KEY);

    let good = request(
        Some(input.clone()),
        vec![obligation(
            ObligationProperty::CleanupReplayContract,
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
                ObligationProperty::CleanupReplayContract,
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
                ObligationProperty::CleanupReplayContract,
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
                ObligationProperty::CallResultUseContract,
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
    moved.cleanup_replay_source.as_mut().unwrap().schema_version = 2;
    assert!(moved.validate().is_err());

    // An incomplete prerequisite triple: a lookup key naming nothing is
    // refused at the envelope, before any core derivation.
    for blank in 0..3 {
        let mut incomplete = good.clone();
        let prerequisite = &mut incomplete
            .cleanup_replay_source
            .as_mut()
            .unwrap()
            .prerequisite;
        match blank {
            0 => prerequisite.semantic_profile.clear(),
            1 => prerequisite.task_digest.clear(),
            _ => prerequisite.obligation_id.clear(),
        }
        assert!(incomplete.validate().is_err());
    }

    // Never together with another projection.
    let mut paired = good.clone();
    paired.call_use_source = serde_json::from_value(json!({
        "schema_version": 1,
        "family": {
            "family": "filesystem_attempt",
            "body": {
                "function": "cio_file_native_close",
                "source_revision": "revision",
                "signature": "int (struct cio_file *)",
                "paths": [{"operations": [{"kind":"return"}]}],
                "profile_assumptions": {}
            }
        }
    }))
    .unwrap();
    assert!(paired.validate().is_err());
}

/// The vocabulary is closed by construction: an unknown lattice constructor,
/// an unknown step form, an unknown prestate and an unknown profile field have
/// no representation at all.
#[test]
fn the_closed_vocabulary_refuses_every_unrepresented_form() {
    // A lattice value outside the nine constructors.
    assert!(serde_json::from_value::<CleanupLatticeValue>(json!({"lattice":"unknown"})).is_err());
    assert!(serde_json::from_value::<CleanupLatticeValue>(json!({"lattice":"opaque"})).is_err());
    // …and the nine that do exist all round-trip.
    for value in [
        CleanupLatticeValue::Null,
        CleanupLatticeValue::Root,
        CleanupLatticeValue::Pointer {
            field: "map".into(),
        },
        CleanupLatticeValue::Integer { value: -1 },
        CleanupLatticeValue::Symbol { field: "fd".into() },
        CleanupLatticeValue::ErrnoPointer,
        CleanupLatticeValue::Errno,
        CleanupLatticeValue::Text,
        CleanupLatticeValue::ReportResult,
    ] {
        let text = serde_json::to_string(&value).unwrap();
        assert_eq!(
            serde_json::from_str::<CleanupLatticeValue>(&text).unwrap(),
            value
        );
    }
    // A bare `return` step, which the call-use vocabulary does have.
    let mut bare = projection();
    bare["family"]["body"]["paths"][0]["operations"] = json!([{"kind":"return"}]);
    assert!(serde_json::from_value::<CleanupReplaySourceInput>(bare).is_err());
    // A scan loop and a termination have no spelling either.
    for kind in ["scan_loop", "terminate"] {
        let mut outside = projection();
        outside["family"]["body"]["paths"][0]["operations"] = json!([{"kind": kind}]);
        assert!(serde_json::from_value::<CleanupReplaySourceInput>(outside).is_err());
    }
    // An unknown family, and an unknown profile member.
    let mut family = projection();
    family["family"]["family"] = json!("filesystem_attempt");
    assert!(serde_json::from_value::<CleanupReplaySourceInput>(family).is_err());
    let mut extra = projection();
    extra["family"]["profile"]["policy"] = json!("native_close_outcomes");
    assert!(
        serde_json::from_value::<CleanupReplaySourceInput>(extra).is_err(),
        "the policy is not a profile member"
    );
    // An unknown top-level member.
    let mut smuggled = projection();
    smuggled["verdict"] = json!("safe");
    assert!(serde_json::from_value::<CleanupReplaySourceInput>(smuggled).is_err());
    // Each family's sampled prestate domain is fixed and complete.
    assert_eq!(
        CleanupPrestateRecord::native_cleanup_outcome_domain().len(),
        4
    );
    assert_eq!(CleanupPrestateRecord::guarded_delete_domain().len(), 5);
    assert_eq!(
        CleanupPrestateRecord::native_cleanup_outcome_domain()[0],
        CleanupPrestateRecord::NativeCleanupOutcome {
            entry: CleanupEntryRecord::Null,
            syscall_result: None
        }
    );
}
