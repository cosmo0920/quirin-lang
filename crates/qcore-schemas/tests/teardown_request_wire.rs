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

//! Wire compatibility and admission rules for the teardown source projection.
//!
//! These are format checks only. Nothing here establishes source identity: the
//! trusted core binds a projection through the task digest and re-evaluates
//! the pinned contracts before any obligation is covered.
use qcore_schemas::obligation::{
    Obligation, ObligationProperty, ObligationScope, ObligationSubject, PathBounds, TaskBounds,
};
use qcore_schemas::teardown_summary::{TEARDOWN_SOURCE_INPUT_KEY, TeardownSourceInput};
use qcore_schemas::{
    PluginId, TargetProfile, UnitIdentity, VerificationRequestV2, VerifierCapability,
};
use serde_json::json;

fn node(name: &str) -> serde_json::Value {
    json!({"c_type":"struct mk_list *","expression":{"kind":"field",
        "base":{"c_type":"struct mk_list *","expression":{"kind":"parameter","index":0}},
        "name":name}})
}

fn unlink_projection() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "family": {
            "family": "list_unlink",
            "entry": {
                "function": "mk_list_del",
                "source_revision": "revision",
                "signature": "void (struct mk_list *)",
                "paths": [{"operations": [
                    {"kind":"read","place": node("prev")},
                    {"kind":"read","place": node("next")},
                    {"kind":"call","function":"__mk_list_del",
                     "signature":"void (struct mk_list *, struct mk_list *)",
                     "arguments":[
                        {"c_type":"struct mk_list *","expression":{"kind":"symbol","operation":0}},
                        {"c_type":"struct mk_list *","expression":{"kind":"symbol","operation":1}}],
                     "result_type":"void"},
                    {"kind":"return"}
                ]}],
                "profile_assumptions": {}
            },
            "helper": {
                "function": "__mk_list_del",
                "source_revision": "revision",
                "signature": "void (struct mk_list *, struct mk_list *)",
                "paths": [{"operations": [{"kind":"return"}]}],
                "profile_assumptions": {}
            },
            "node_c_type": "struct mk_list *"
        }
    })
}

fn destructor_projection() -> serde_json::Value {
    json!({
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
    })
}

fn obligation(property: ObligationProperty, symbol: &str) -> Obligation {
    Obligation {
        obligation_id: format!("{property}|{symbol}"),
        capability: VerifierCapability::ResourceContracts,
        property,
        subject: ObligationSubject::Function {
            symbol: symbol.into(),
            translation_unit: "mk_list.c".into(),
        },
        scope: ObligationScope::Functions {
            symbols: [symbol.to_owned()].into(),
        },
        bounds: TaskBounds::single_path(),
        assumptions: Default::default(),
        evidence_version: 2,
    }
}

fn request(
    teardown: Option<TeardownSourceInput>,
    obligations: Vec<Obligation>,
) -> VerificationRequestV2 {
    VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "teardown".into(),
        plugin_id: PluginId::new("qcore.chunkio").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "revision".into(),
            profile: "qcore.c_list_unlink_contract.v1".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "mk_list.c".into(),
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
        teardown_source: teardown,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: None,
    }
}

#[test]
fn a_request_without_a_projection_keeps_its_previous_wire_shape() {
    let plain = request(None, Vec::new());
    let value = serde_json::to_value(&plain).unwrap();
    // The three earlier projections and this one all serialize away, so an
    // existing request's bytes are unchanged by the 7.0.0 addition.
    for key in [
        "source_effect",
        "counter_effect",
        "reactor_program",
        "teardown_source",
        "call_use_source",
    ] {
        assert!(value.get(key).is_none(), "{key} appeared in the wire form");
    }
    plain.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<VerificationRequestV2>(value).unwrap(),
        plain
    );
    assert_eq!(TEARDOWN_SOURCE_INPUT_KEY, "qcore:teardown-source-input:v1");
}

#[test]
fn a_projection_travels_with_exactly_its_own_family_obligation() {
    let input: TeardownSourceInput = serde_json::from_value(unlink_projection()).unwrap();
    assert!(input.version_matches());
    assert_eq!(input.subject_function(), "mk_list_del");
    assert_eq!(serde_json::to_value(&input).unwrap(), unlink_projection());
    let symbol = "mk_list_del";
    request(
        Some(input.clone()),
        vec![obligation(ObligationProperty::ListUnlinkContract, symbol)],
    )
    .validate()
    .unwrap();

    // An obligation with no projection, a projection with none, the other
    // family's property, a foreign subject, a second obligation, an
    // unsupported input version, and another projection alongside it all
    // reject.
    assert!(
        request(
            None,
            vec![obligation(ObligationProperty::ListUnlinkContract, symbol)]
        )
        .validate()
        .is_err()
    );
    assert!(request(Some(input.clone()), Vec::new()).validate().is_err());
    assert!(
        request(
            Some(input.clone()),
            vec![obligation(
                ObligationProperty::FieldDestructorContract,
                symbol
            )]
        )
        .validate()
        .is_err()
    );
    assert!(
        request(
            Some(input.clone()),
            vec![obligation(ObligationProperty::CustodyDischarged, symbol)]
        )
        .validate()
        .is_err()
    );
    assert!(
        request(
            Some(input.clone()),
            vec![obligation(
                ObligationProperty::ListUnlinkContract,
                "__mk_list_del"
            )]
        )
        .validate()
        .is_err()
    );
    assert!(
        request(
            Some(input.clone()),
            vec![
                obligation(ObligationProperty::ListUnlinkContract, symbol),
                obligation(ObligationProperty::CustodyDischarged, symbol),
            ]
        )
        .validate()
        .is_err()
    );
    let mut wrong_version = input.clone();
    wrong_version.schema_version = 2;
    assert!(!wrong_version.version_matches());
    assert!(
        request(
            Some(wrong_version),
            vec![obligation(ObligationProperty::ListUnlinkContract, symbol)]
        )
        .validate()
        .is_err()
    );
    let mut both = request(
        Some(input),
        vec![obligation(ObligationProperty::ListUnlinkContract, symbol)],
    );
    both.counter_effect = Some(
        serde_json::from_value(json!({"schema_version":1,"function":"mk_list_del",
            "source_revision":"revision","signature":"unsigned long (struct cio_ctx *)",
            "root_c_type":"struct cio_ctx *","operations":[],"profile_assumptions":{}}))
        .unwrap(),
    );
    assert!(both.validate().is_err());
}

#[test]
fn the_destructor_family_carries_its_own_property_and_bounds() {
    let input: TeardownSourceInput = serde_json::from_value(destructor_projection()).unwrap();
    assert_eq!(input.subject_function(), "cio_memfs_close");
    let symbol = "cio_memfs_close";
    let mut accepted = request(
        Some(input.clone()),
        vec![Obligation {
            bounds: TaskBounds {
                path_exploration: PathBounds::BoundedPaths { paths: 2 },
                ..TaskBounds::single_path()
            },
            ..obligation(ObligationProperty::FieldDestructorContract, symbol)
        }],
    );
    accepted.unit.translation_unit = "mk_list.c".into();
    accepted.target.profile = "qcore.c_field_destructor_contract.v1".into();
    accepted.validate().unwrap();
    // The unlink property cannot carry a destructor projection.
    let mut crossed = accepted.clone();
    crossed.obligations = vec![obligation(ObligationProperty::ListUnlinkContract, symbol)];
    assert!(crossed.validate().is_err());

    // Unknown fields and unknown operation/family tags stay rejected.
    let mut unknown = destructor_projection();
    unknown["extra"] = json!(1);
    assert!(serde_json::from_value::<TeardownSourceInput>(unknown).is_err());
    let mut foreign_family = destructor_projection();
    foreign_family["family"]["family"] = json!("chunk_close");
    assert!(serde_json::from_value::<TeardownSourceInput>(foreign_family).is_err());
    let mut foreign_operation = destructor_projection();
    foreign_operation["family"]["destructor"]["paths"][0]["operations"][0] =
        json!({"kind":"scan_loop"});
    assert!(serde_json::from_value::<TeardownSourceInput>(foreign_operation).is_err());
    // Two different projections have two different digests.
    let unlink: TeardownSourceInput = serde_json::from_value(unlink_projection()).unwrap();
    assert_ne!(unlink.digest(), input.digest());
    assert_eq!(unlink.digest_key(), input.digest_key());
}
