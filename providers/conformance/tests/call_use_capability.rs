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
//! for the call-result def/use capability.
#![cfg(not(target_os = "windows"))]

#[path = "../../../crates/qcore-verification/tests/fixtures/call_use_source.rs"]
mod fixture;
use fixture::*;
use qcore_schemas::SemanticWitness;
use qcore_schemas::call_use_summary::{
    CALL_USE_SOURCE_INPUT_KEY, CallUseFamilyInput, CallUseProposalRecord, CallUseSourceInput,
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
        std::env::temp_dir().join(format!("qcore-call-use-package-{}", std::process::id()));
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
    let close: CallUseSourceInput = serde_json::from_str(include_str!(
        "../../quirin-provider-shared/fixtures/file_close_call_use_projection.json"
    ))
    .unwrap();
    assert_eq!(
        close,
        import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS).source_input()
    );
    let native: CallUseSourceInput = serde_json::from_str(include_str!(
        "../../quirin-provider-shared/fixtures/native_close_call_use_projection.json"
    ))
    .unwrap();
    assert_eq!(
        native,
        import(NATIVE_SOURCE, "cio_file_native_close", &ATTEMPT_RECORDS).source_input()
    );
}

#[test]
fn real_package_proposes_call_use_readings_without_admitting_false_contracts() {
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

    // Positive: the process reads the transported body on its own and the core
    // admits it only after re-evaluating the pinned contract.
    let native = import(NATIVE_SOURCE, "cio_file_native_close", &ATTEMPT_RECORDS);
    let replay = task(&native, "cio_file_unix.c", ATTEMPT_CONTRACT);
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
    assert_eq!(certificate.witness().unwrap().kind(), "call_result_use");
    let Some(SemanticWitness::CallResultUse(witness)) = certificate.witness() else {
        panic!("wrong witness kind")
    };
    let CallUseProposalRecord::FilesystemAttempt(reading) = &witness.summary;
    assert_eq!(reading.function, "cio_file_native_close");
    assert_eq!(reading.paths.len(), 2);
    for path in &reading.paths {
        let [call] = path.calls.as_slice() else {
            panic!("one call per path")
        };
        assert_eq!(call.function, "close");
        assert_eq!(call.uses, vec![2, 3]);
        assert_eq!(call.guard_uses, vec![3]);
    }

    // The request carries paths only; no reading travels with it.
    let value = serde_json::to_value(request.call_use_source.as_ref().unwrap()).unwrap();
    assert!(value["family"]["body"]["paths"].is_array());
    for derived in ["uses", "guard_uses", "calls", "cleanup_calls"] {
        assert!(
            !serde_json::to_string(&value).unwrap().contains(derived),
            "{derived} travelled"
        );
    }

    // The wrapper body, whose four results are all ignored, also round-trips.
    let wrapper = import(CLOSE_SOURCE, "cio_file_close", &ATTEMPT_RECORDS);
    let wrapper_replay = task(&wrapper, "cio_file.c", ATTEMPT_CONTRACT);
    let wrapper_result = registry.verify_v2(
        &wrapper_replay.request(plugin()),
        wrapper_replay.task(),
        Some(&wrapper_replay),
    );
    assert_eq!(wrapper_result.verdict, ProposedVerdict::Safe);

    // The guarded deletion attempt, whose short-circuit guards read record
    // fields rather than a call result, round-trips under its own contract.
    let delete = import(DELETE_SOURCE, "cio_file_native_delete", &DELETE_RECORDS);
    let delete_replay = task(&delete, "cio_file.c", DELETE_CONTRACT);
    let delete_result = registry.verify_v2(
        &delete_replay.request(plugin()),
        delete_replay.task(),
        Some(&delete_replay),
    );
    assert_eq!(
        delete_result.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        delete_result.diagnostics
    );
    let Some(SemanticWitness::CallResultUse(delete_witness)) =
        delete_result.certificate.unwrap().witness().cloned()
    else {
        panic!("wrong witness kind")
    };
    let CallUseProposalRecord::FilesystemAttempt(delete_reading) = &delete_witness.summary;
    assert_eq!(delete_reading.function, "cio_file_native_delete");
    assert!(
        delete_reading
            .paths
            .iter()
            .any(|path| path.calls.iter().any(|call| call.function == "unlink")),
        "the unlink attempt must be visible in the reading"
    );

    // Missing evidence: no core checker, no registered provider, a shipped
    // package that does not implement the capability, and a disabled provider
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
        let CallUseFamilyInput::FilesystemAttempt { body } =
            &mut forged.call_use_source.as_mut().unwrap().family;
        // Drop the guard from both paths: a well-formed body the core never
        // imported. The process reads it faithfully and proposes a reading
        // with no guarded use.
        for path in &mut body.paths {
            path.operations.retain(|operation| {
                !matches!(
                    operation,
                    qcore_schemas::call_use_summary::CallUseOperationRecord::Guard { .. }
                )
            });
        }
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
        CALL_USE_SOURCE_INPUT_KEY.into(),
        forged.call_use_source.as_ref().unwrap().digest(),
    );
    forged.task_digest = forged_task.task_digest();
    let rejection = registry.verify_v2(&forged, &forged_task, Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.certificate.is_none());

    // A body the process reads correctly still cannot buy a false contract.
    let false_contract = qcore_verification::call_use_source::FileAttemptSourceTask::derive(
        &native,
        "cio_file_unix.c",
        &ATTEMPT_CONTRACT.replace(
            "ensures normal ignored_results == 0;",
            "ensures normal false;",
        ),
        &policy("cio_file_native_close"),
        target(&native),
    )
    .unwrap();
    let rejection = registry.verify_v2(
        &false_contract.request(plugin()),
        false_contract.task(),
        Some(&false_contract),
    );
    assert_eq!(
        rejection.verdict,
        ProposedVerdict::Unknown,
        "a Safe proposal bypassed contract evaluation"
    );
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));

    // Changed source bytes or build options retire the checker's context.
    for changed in [
        import(
            &format!("// changed actual source\n{NATIVE_SOURCE}"),
            "cio_file_native_close",
            &ATTEMPT_RECORDS,
        ),
        match import_result(
            NATIVE_SOURCE,
            vec!["QCORE_OTHER_BUILD=1".into()],
            "cio_file_native_close",
            &ATTEMPT_RECORDS,
            admitted_callees("cio_file_native_close"),
        ) {
            qcore_c_gateway::call_use_source::FileAttemptSourceResult::Import(import) => *import,
            other => panic!("{other:?}"),
        },
    ] {
        let other = task(&changed, "cio_file_unix.c", ATTEMPT_CONTRACT);
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
