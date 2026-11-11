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
//! for the unsigned counter capability.
#![cfg(unix)]
#[path = "../../../crates/qcore-verification/tests/fixtures/counter_source.rs"]
mod fixture;
use fixture::*;
use qcore_schemas::counter_summary::COUNTER_SOURCE_INPUT_KEY;
use qcore_schemas::{PluginId, ProposedVerdict, SemanticWitness};
use qcore_verifier_host::{ProcessProvider, VerifierRegistry};
use std::process::Command;
use std::sync::Arc;

fn plugin() -> PluginId {
    PluginId::new("qcore.chunkio").unwrap()
}

#[test]
fn real_package_proposes_counter_updates_without_admitting_false_contracts() {
    if !clang_available() {
        return;
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output = std::env::temp_dir().join(format!(
        "qcore-counter-effect-package-{}",
        std::process::id()
    ));
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
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(
            ProcessProvider::from_package(output.join("qcore.chunkio")).unwrap(),
        ))
        .unwrap();

    // Positive: the process reads the transported path on its own and the core
    // admits it only after re-evaluating the pinned contract.
    let import = imported(SOURCE, vec![]);
    let replay = task(&import, CONTRACT);
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
    assert_eq!(certificate.witness().unwrap().kind(), "counter_contract");
    let Some(SemanticWitness::CounterContract(witness)) = certificate.witness() else {
        panic!("wrong witness kind")
    };
    assert_eq!(witness.summary.field, "total_chunks");
    assert_eq!(witness.summary.delta, -1);
    assert_eq!(witness.summary.counter_c_type, "unsigned long");

    // The request carries the path only; no expected reading travels with it.
    let input = serde_json::to_value(request.counter_effect.as_ref().unwrap()).unwrap();
    assert!(input.get("delta").is_none() && input.get("field").is_none());
    assert!(input.get("operations").is_some());
    let mut wrong_version = request.clone();
    wrong_version
        .counter_effect
        .as_mut()
        .unwrap()
        .schema_version = 0;
    assert!(wrong_version.validate().is_err());
    // The two source projections are mutually exclusive in one request.
    let boolean = {
        let mut both = request.clone();
        both.source_effect = serde_json::from_str(
            r#"{"schema_version":2,"function":"cio_chunk_counter_total_sub","source_revision":"x",
                "field":"Locked","program":{"Return":"Field"},"dependencies":{},
                "profile_assumptions":{}}"#,
        )
        .unwrap();
        both
    };
    assert!(boolean.validate().is_err());

    // A real increment source is read as an increment by the same process.
    let increment_source = SOURCE.replace("total_chunks--", "total_chunks++");
    let increment_contract = CONTRACT
        .replace(
            "requires counter_positive;",
            "requires counter_below_maximum;",
        )
        .replace("ensures normal delta == -1;", "ensures normal delta == 1;");
    let increment = imported(&increment_source, vec![]);
    let increment_task = task(&increment, &increment_contract);
    let response = registry.verify_v2(
        &increment_task.request(plugin()),
        increment_task.task(),
        Some(&increment_task),
    );
    assert_eq!(
        response.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        response.diagnostics
    );
    let Some(SemanticWitness::CounterContract(witness)) =
        response.certificate.unwrap().witness().cloned()
    else {
        panic!("wrong witness kind")
    };
    assert_eq!(witness.summary.delta, 1);

    // Missing evidence: no core checker, no registered provider, and a
    // disabled provider each stay Unknown with no certificate.
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
    // A shipped package that does not implement this capability answers
    // without a witness; nothing is admitted and no certificate appears.
    let other = PluginId::new("qcore.libevent").unwrap();
    let mut others = VerifierRegistry::default();
    others
        .register(Arc::new(
            ProcessProvider::from_package(output.join("qcore.libevent")).unwrap(),
        ))
        .unwrap();
    let mut foreign_request = request.clone();
    foreign_request.plugin_id = other;
    let foreign = others.verify_v2(&foreign_request, replay.task(), Some(&replay));
    assert_eq!(foreign.verdict, ProposedVerdict::Unknown);
    assert!(foreign.certificate.is_none());

    // Tampered transport: a projection the core did not send is refused, and
    // rebinding it to a recomputed task digest is refused by the core's own
    // import comparison instead.
    let mut forged = request.clone();
    let projection = forged.counter_effect.as_mut().unwrap();
    // A well-formed path the core never imported: the same helper written as
    // an increment. The process reads it faithfully and proposes `delta == 1`.
    let json = serde_json::to_string(&projection.operations)
        .unwrap()
        .replace("\"operator\":\"-\"", "\"operator\":\"+\"");
    projection.operations = serde_json::from_str(&json).unwrap();
    let rejection = registry.verify_v2(&forged, replay.task(), Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(
        rejection
            .diagnostics
            .iter()
            .any(|d| d.code == "QP009" && d.message.contains("checked source update")),
        "{:?}",
        rejection.diagnostics
    );
    let mut forged_task = replay.task().clone();
    forged_task.contract_sources.insert(
        COUNTER_SOURCE_INPUT_KEY.into(),
        forged.counter_effect.as_ref().unwrap().digest(),
    );
    forged.task_digest = forged_task.task_digest();
    let rejection = registry.verify_v2(&forged, &forged_task, Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));

    // A path the process reads correctly still cannot buy a false contract.
    let false_contract = task(
        &import,
        &CONTRACT.replace("ensures normal delta == -1;", "ensures normal delta == 1;"),
    );
    let rejection = registry.verify_v2(
        &false_contract.request(plugin()),
        false_contract.task(),
        Some(&false_contract),
    );
    assert_eq!(
        rejection.verdict,
        ProposedVerdict::Unknown,
        "a Safe proposal cannot bypass contract evaluation"
    );
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));

    // Changed source bytes or build options retire the checker's context.
    for changed in [
        imported(&format!("// changed actual source\n{SOURCE}"), vec![]),
        imported(SOURCE, vec!["QCORE_OTHER_BUILD=1".into()]),
    ] {
        let other = task(&changed, CONTRACT);
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
