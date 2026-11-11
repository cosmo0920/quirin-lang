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

//! Wire compatibility and admission rules for the counter source projection.
//!
//! These are format checks only. Nothing here establishes source identity: the
//! trusted core binds a projection through the task digest and re-evaluates
//! the pinned contract before any obligation is covered.
use qcore_schemas::counter_summary::{COUNTER_SOURCE_INPUT_KEY, CounterSourceInput};
use qcore_schemas::obligation::{
    Obligation, ObligationProperty, ObligationScope, ObligationSubject, TaskBounds,
};
use qcore_schemas::{
    PluginId, TargetProfile, UnitIdentity, VerificationRequestV2, VerifierCapability,
};
use serde_json::json;

fn projection() -> serde_json::Value {
    let place = json!({"c_type":"unsigned long","expression":{"kind":"field",
        "base":{"c_type":"struct cio_ctx *","expression":{"kind":"parameter","index":0}},
        "name":"total_chunks"}});
    json!({
        "schema_version": 1,
        "function": "cio_chunk_counter_total_sub",
        "source_revision": "revision",
        "signature": "unsigned long (struct cio_ctx *)",
        "root_c_type": "struct cio_ctx *",
        "operations": [
            {"kind":"read","place": place},
            {"kind":"write","place": place, "value": {"c_type":"unsigned long","expression":{"kind":"binary",
                "operator":"-",
                "left":{"c_type":"unsigned long","expression":{"kind":"symbol","operation":0}},
                "right":{"c_type":"unsigned long","expression":{"kind":"integer","value":"1"}}}}},
            {"kind":"read","place": place},
            {"kind":"return_value","value":{"c_type":"unsigned long","expression":{"kind":"symbol","operation":2}}}
        ],
        "profile_assumptions": {}
    })
}

fn obligation(property: ObligationProperty, symbol: &str) -> Obligation {
    Obligation {
        obligation_id: format!("{property}|{symbol}"),
        capability: VerifierCapability::ResourceContracts,
        property,
        subject: ObligationSubject::Function {
            symbol: symbol.into(),
            translation_unit: "cio_chunk.c".into(),
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
    counter: Option<CounterSourceInput>,
    obligations: Vec<Obligation>,
) -> VerificationRequestV2 {
    VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "counter".into(),
        plugin_id: PluginId::new("qcore.chunkio").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "revision".into(),
            profile: "qcore.c_counter_contract.v1".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "cio_chunk.c".into(),
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "c".repeat(64),
        obligations,
        source_effect: None,
        counter_effect: counter,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: None,
    }
}

#[test]
fn a_request_without_a_projection_keeps_its_previous_wire_shape() {
    let plain = request(None, Vec::new());
    let value = serde_json::to_value(&plain).unwrap();
    assert!(value.get("counter_effect").is_none());
    assert!(value.get("source_effect").is_none());
    // The reactor projection added in 5.0.0 serializes away exactly as the
    // other two do, so an existing request's bytes are unchanged.
    assert!(value.get("reactor_program").is_none());
    plain.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<VerificationRequestV2>(value).unwrap(),
        plain
    );
    assert_eq!(COUNTER_SOURCE_INPUT_KEY, "qcore:counter-source-input:v1");
}

#[test]
fn a_projection_travels_with_exactly_its_own_single_obligation() {
    let input: CounterSourceInput = serde_json::from_value(projection()).unwrap();
    assert!(input.version_matches());
    assert_eq!(serde_json::to_value(&input).unwrap(), projection());
    let symbol = "cio_chunk_counter_total_sub";
    let accepted = request(
        Some(input.clone()),
        vec![obligation(
            ObligationProperty::CounterUpdateContract,
            symbol,
        )],
    );
    accepted.validate().unwrap();

    // A counter obligation with no projection, a projection with none, the
    // wrong property, a foreign subject, a second obligation, an unsupported
    // input version, and a Boolean projection alongside it all reject.
    assert!(
        request(
            None,
            vec![obligation(
                ObligationProperty::CounterUpdateContract,
                symbol
            )]
        )
        .validate()
        .is_err()
    );
    assert!(request(Some(input.clone()), Vec::new()).validate().is_err());
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
                ObligationProperty::CounterUpdateContract,
                "cio_chunk_counter_total_add"
            )]
        )
        .validate()
        .is_err()
    );
    assert!(
        request(
            Some(input.clone()),
            vec![
                obligation(ObligationProperty::CounterUpdateContract, symbol),
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
            vec![obligation(
                ObligationProperty::CounterUpdateContract,
                symbol
            )]
        )
        .validate()
        .is_err()
    );
    let mut both = accepted.clone();
    both.source_effect = Some(
        serde_json::from_value(
            json!({"schema_version":2,"function":"cio_chunk_counter_total_sub",
            "source_revision":"revision","field":"Locked","program":{"Return":"Field"},
            "dependencies":{},"profile_assumptions":{}}),
        )
        .unwrap(),
    );
    assert!(both.validate().is_err());

    // Unknown fields and unknown operation kinds stay rejected.
    let mut unknown = projection();
    unknown["extra"] = json!(1);
    assert!(serde_json::from_value::<CounterSourceInput>(unknown).is_err());
    let mut call = projection();
    call["operations"][0] = json!({"kind":"call","function":"free"});
    assert!(serde_json::from_value::<CounterSourceInput>(call).is_err());
}
