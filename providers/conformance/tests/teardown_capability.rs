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

//! Real typed C import → packaged ChunkIO process → trusted contract replay
//! for the two teardown capabilities.
#![cfg(unix)]
#[path = "../../../crates/qcore-verification/tests/fixtures/teardown_source.rs"]
mod fixture;
use fixture::*;
use qcore_schemas::SemanticWitness;
use qcore_schemas::teardown_summary::{
    TEARDOWN_SOURCE_INPUT_KEY, TeardownFamilyInput, TeardownProposalRecord, TeardownSourceInput,
};
use qcore_schemas::{PluginId, ProposedVerdict};
use qcore_verifier_host::{ProcessProvider, VerifierRegistry};
use std::process::Command;
use std::sync::Arc;

fn plugin() -> PluginId {
    PluginId::new("qcore.chunkio").unwrap()
}

fn build_packages() -> std::path::PathBuf {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output =
        std::env::temp_dir().join(format!("qcore-teardown-package-{}", std::process::id()));
    let built = Command::new("python3")
        .arg(root.join("package.py"))
        .args(["build", "--output"])
        .arg(&output)
        .arg("--target-dir")
        .arg(root.join("target/conformance-artifacts"))
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    output
}

/// The projections the provider unit tests read are the ones the live gateway
/// still produces. Without this the provider fixtures could drift into a shape
/// the importer no longer emits and the mutation matrix would test nothing.
#[test]
fn the_provider_fixtures_are_the_live_projections() {
    if !clang_available() {
        return;
    }
    let unlink: TeardownSourceInput = serde_json::from_str(include_str!(
        "../../quirin-provider-shared/fixtures/list_unlink_projection.json"
    ))
    .unwrap();
    assert_eq!(unlink, unlink_import(UNLINK_SOURCE, vec![]).source_input());
    let destructor: TeardownSourceInput = serde_json::from_str(include_str!(
        "../../quirin-provider-shared/fixtures/field_destructor_projection.json"
    ))
    .unwrap();
    assert_eq!(
        destructor,
        destructor_import(DESTRUCTOR_SOURCE, vec![]).source_input()
    );
}

#[test]
fn real_package_proposes_teardown_readings_without_admitting_false_contracts() {
    if !clang_available() {
        return;
    }
    let output = build_packages();
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(
            ProcessProvider::from_package(output.join("qcore.chunkio")).unwrap(),
        ))
        .unwrap();

    // Positive, unlink: the process reads the two transported bodies on its
    // own and the core admits it only after re-evaluating the pinned contract.
    let import = unlink_import(UNLINK_SOURCE, vec![]);
    let replay = unlink_task(&import, UNLINK_CONTRACT);
    let request = replay.request(plugin());
    let result = registry.verify_v2(&request, replay.task(), Some(&replay));
    assert_eq!(
        result.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        result.diagnostics
    );
    let certificate = result.certificate.unwrap();
    assert_eq!(certificate.task(), Some(replay.task()));
    assert_eq!(certificate.witness().unwrap().kind(), "teardown_contract");
    let Some(SemanticWitness::TeardownContract(witness)) = certificate.witness() else {
        panic!("wrong witness kind")
    };
    let TeardownProposalRecord::ListUnlink(reading) = &witness.summary else {
        panic!("wrong proposal family")
    };
    assert_eq!(reading.function, UNLINK_ENTRY);
    assert_eq!(reading.helper_function, UNLINK_HELPER);
    assert_eq!(reading.cases.len(), 2);
    assert!(reading.cases.iter().all(|case| case.helper_calls == 1
        && case.writes.len() == 4
        && case.writes[2].value.is_none()));

    // The request carries the two bodies only; no reading travels with it.
    let value = serde_json::to_value(request.teardown_source.as_ref().unwrap()).unwrap();
    assert!(value.get("cases").is_none());
    assert!(value["family"]["entry"]["paths"].is_array());
    assert!(value["family"]["helper"]["paths"].is_array());

    // Positive, destructor: nine realizable nullable states, each read from
    // the transported paths.
    let destructor_import_value = destructor_import(DESTRUCTOR_SOURCE, vec![]);
    let destructor_replay = destructor_task(&destructor_import_value, DESTRUCTOR_CONTRACT);
    let destructor_request = destructor_replay.request(plugin());
    let response = registry.verify_v2(
        &destructor_request,
        destructor_replay.task(),
        Some(&destructor_replay),
    );
    assert_eq!(
        response.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        response.diagnostics
    );
    let Some(SemanticWitness::TeardownContract(witness)) =
        response.certificate.unwrap().witness().cloned()
    else {
        panic!("wrong witness kind")
    };
    let TeardownProposalRecord::FieldDestructor(reading) = &witness.summary else {
        panic!("wrong proposal family")
    };
    assert_eq!(reading.cases.len(), 9);
    assert_eq!(
        reading.cases.last().unwrap().released_in_order,
        [
            "backend__name",
            "backend__buf_data",
            "backend__meta_data",
            "backend"
        ]
    );

    // Missing evidence: no core checker, no registered provider, a disabled
    // provider, and a shipped package that does not implement the capability
    // each stay Unknown with no certificate.
    let no_checker = registry.verify_v2(&request, replay.task(), None);
    assert_eq!(no_checker.verdict, ProposedVerdict::Unknown);
    assert!(no_checker.diagnostics.iter().any(|d| d.code == "QP016"));
    assert!(no_checker.certificate.is_none());
    assert_eq!(
        VerifierRegistry::default()
            .verify_v2(&request, replay.task(), Some(&replay))
            .verdict,
        ProposedVerdict::Unknown
    );
    let mut others = VerifierRegistry::default();
    others
        .register(Arc::new(
            ProcessProvider::from_package(output.join("qcore.libevent")).unwrap(),
        ))
        .unwrap();
    let mut foreign_request = request.clone();
    foreign_request.plugin_id = PluginId::new("qcore.libevent").unwrap();
    let foreign = others.verify_v2(&foreign_request, replay.task(), Some(&replay));
    assert_eq!(foreign.verdict, ProposedVerdict::Unknown);
    assert!(foreign.certificate.is_none());

    // Tampered transport: a projection the core never imported is refused by
    // the host's digest binding; rebinding it to a recomputed task digest gets
    // past that and is then refused by the core's own comparison.
    let mut forged = request.clone();
    {
        let TeardownFamilyInput::ListUnlink { helper, .. } =
            &mut forged.teardown_source.as_mut().unwrap().family
        else {
            panic!("unlink projection")
        };
        // `prev->next = prev`: a well-formed body the core never imported. The
        // process reads it faithfully and proposes the wrong link.
        let json = serde_json::to_string(&helper.paths)
            .unwrap()
            .replace("\"index\":1", "\"index\":0");
        helper.paths = serde_json::from_str(&json).unwrap();
    }
    let rejection = registry.verify_v2(&forged, replay.task(), Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(
        rejection.diagnostics.iter().any(|d| d.code == "QP001"),
        "{:?}",
        rejection.diagnostics
    );
    let mut forged_task = replay.task().clone();
    forged_task.contract_sources.insert(
        TEARDOWN_SOURCE_INPUT_KEY.into(),
        forged.teardown_source.as_ref().unwrap().digest(),
    );
    forged.task_digest = forged_task.task_digest();
    let rejection = registry.verify_v2(&forged, &forged_task, Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.certificate.is_none());

    // A body the process reads correctly still cannot buy a false contract.
    let false_unlink = unlink_task(
        &import,
        &UNLINK_CONTRACT.replace(
            "ensures normal write_count == 4;",
            "ensures normal write_count == 3;",
        ),
    );
    let rejection = registry.verify_v2(
        &false_unlink.request(plugin()),
        false_unlink.task(),
        Some(&false_unlink),
    );
    assert_eq!(
        rejection.verdict,
        ProposedVerdict::Unknown,
        "a Safe unlink proposal bypassed contract evaluation"
    );
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));
    let false_destructor = destructor_task(
        &destructor_import_value,
        &DESTRUCTOR_CONTRACT.replace(
            "ensures normal live__backend == 0;",
            "ensures normal live__backend == 1;",
        ),
    );
    let rejection = registry.verify_v2(
        &false_destructor.request(plugin()),
        false_destructor.task(),
        Some(&false_destructor),
    );
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));

    // Changed source bytes or build options retire the checker's context.
    for changed in [
        unlink_import(
            &format!("// changed actual source\n{UNLINK_SOURCE}"),
            vec![],
        ),
        unlink_import(UNLINK_SOURCE, vec!["QCORE_OTHER_BUILD=1".into()]),
    ] {
        let other = unlink_task(&changed, UNLINK_CONTRACT);
        let rejection = registry.verify_v2(&other.request(plugin()), other.task(), Some(&replay));
        assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    }

    registry.set_enabled(&plugin(), false).unwrap();
    assert_eq!(
        registry
            .verify_v2(&request, replay.task(), Some(&replay))
            .verdict,
        ProposedVerdict::Unknown
    );
    std::fs::remove_dir_all(output).unwrap();
}
