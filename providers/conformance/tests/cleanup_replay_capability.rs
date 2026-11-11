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
//! for the symbolic cleanup-replay capability.
//!
//! This is the end-to-end route: the shipped executable computes the replay
//! itself over the transported paths and profile, and the core admits it only
//! after rebuilding the profile from its own contract, re-establishing the
//! prerequisite and re-running its own unedited replay.
#![cfg(not(target_os = "windows"))]

#[path = "../../../crates/qcore-verification/tests/fixtures/cleanup_replay_source.rs"]
mod fixture;
use fixture::*;
use qcore_schemas::SemanticWitness;
use qcore_schemas::cleanup_replay_summary::{
    CLEANUP_REPLAY_SOURCE_INPUT_KEY, CleanupReplayCaseRecord, CleanupReplayFamilyInput,
    CleanupReplayOperationRecord, CleanupReplayProposalRecord, CleanupReplaySourceInput,
};
use qcore_schemas::{PluginId, ProposedVerdict};
use qcore_verification::cleanup_replay_source::CleanupReplaySourceTask;
use qcore_verifier_host::{ProcessProvider, VerifierRegistry};
use std::process::Command;
use std::sync::Arc;

const CLOSE: &str = "cio_file_native_close";
const UNMAP: &str = "cio_file_native_unmap";
const DELETE: &str = "cio_file_native_delete";
const UNIX_TU: &str = "cio_file_unix.c";

fn plugin() -> PluginId {
    PluginId::new("qcore.chunkio").unwrap()
}

fn build_packages() -> std::path::PathBuf {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output = std::env::temp_dir().join(format!(
        "qcore-cleanup-replay-package-{}",
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
    output
}

/// The projections the provider unit tests read are the ones the live gateway
/// still produces. Without this the provider fixtures could drift into a shape
/// the importer no longer emits and the mutation matrix would test nothing.
///
/// **Measured, and deliberately not papered over:** the `prerequisite` triple
/// is *not* a content-only function of the body. `SourceInputIdentity` binds
/// `std::env::current_dir()` by design (the recorded identity must name the
/// directory the tools actually ran in), so the prerequisite's task digest and
/// obligation id differ between the crate the fixture was generated in and this
/// one. Everything the provider actually replays over — the schema version, the
/// body's complete path set and the declared profile — is compared exactly; the
/// prerequisite is checked for shape and for its semantic profile only, and the
/// working-directory sensitivity is recorded as boundary
/// `P04-B2-PREREQ-IDENTITY-CWD` rather than asserted away.
#[test]
fn the_provider_fixtures_are_the_live_projections() {
    if !clang_available() {
        return;
    }
    for (source, function, records, name) in [
        (
            OUTCOMES_SOURCE,
            CLOSE,
            &OUTCOMES_RECORDS[..],
            include_str!(
                "../../quirin-provider-shared/fixtures/native_close_cleanup_replay_projection.json"
            ),
        ),
        (
            OUTCOMES_SOURCE,
            UNMAP,
            &OUTCOMES_RECORDS[..],
            include_str!(
                "../../quirin-provider-shared/fixtures/native_unmap_cleanup_replay_projection.json"
            ),
        ),
        (
            DELETE_SOURCE,
            DELETE,
            &DELETE_RECORDS[..],
            include_str!(
                "../../quirin-provider-shared/fixtures/native_delete_cleanup_replay_projection.json"
            ),
        ),
    ] {
        let committed: CleanupReplaySourceInput = serde_json::from_str(name).unwrap();
        let (import, _) = route(source, function, records, UNIX_TU);
        let live = import.source_input();
        assert_eq!(
            committed.schema_version, live.schema_version,
            "{function} schema version drifted"
        );
        assert_eq!(
            committed.family, live.family,
            "{function} projection drifted"
        );
        assert_eq!(
            committed.prerequisite.semantic_profile, live.prerequisite.semantic_profile,
            "{function} prerequisite profile drifted"
        );
        assert!(committed.prerequisite.is_complete());
        assert!(live.prerequisite.is_complete());
        // The measurement itself: the triple really is directory-sensitive, so
        // a future change that made it content-only would fail here and be
        // re-evaluated rather than silently inherited.
        assert_ne!(
            committed.prerequisite.task_digest, live.prerequisite.task_digest,
            "the prerequisite identity became content-only; re-evaluate \
             P04-B2-PREREQ-IDENTITY-CWD"
        );
    }
}

#[test]
fn real_package_replays_cleanup_outcomes_without_admitting_false_contracts() {
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

    // Positive: the process replays the transported body on its own and the
    // core admits it only after re-establishing the prerequisite and
    // re-evaluating the pinned contract.
    let (_, unmap) = route(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, UNIX_TU);
    let request = unmap.request(plugin());
    let result = registry.verify_v2(&request, unmap.task(), Some(&unmap));
    assert_eq!(
        result.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        result.diagnostics
    );
    let certificate = result.certificate.unwrap();
    assert_eq!(certificate.task(), Some(unmap.task()));
    assert_eq!(certificate.witness().unwrap().kind(), "cleanup_replay");
    let Some(SemanticWitness::CleanupReplay(witness)) = certificate.witness() else {
        panic!("wrong witness kind")
    };
    let CleanupReplayProposalRecord::NativeCleanupOutcome(replay) = &witness.summary else {
        panic!("wrong family")
    };
    assert_eq!(replay.function, UNMAP);
    assert_eq!(replay.cases.len(), 4);
    let CleanupReplayCaseRecord::NativeCleanupOutcome {
        field_writes,
        fields_reset,
        ..
    } = &replay.cases[3]
    else {
        panic!("wrong case variant")
    };
    assert_eq!(field_writes, &["alloc_size", "map", "map_truncated_warned"]);
    assert!(fields_reset);

    // The request carries the question only: paths, the declared profile and
    // the prerequisite triple. No case vector, field state or verdict travels.
    let value = serde_json::to_value(request.cleanup_replay_source.as_ref().unwrap()).unwrap();
    assert!(value["family"]["body"]["paths"].is_array());
    assert!(value["family"]["profile"]["fields"].is_array());
    let text = serde_json::to_string(&value).unwrap();
    for derived in [
        "cases",
        "field_writes",
        "fields_preserved",
        "fields_reset",
        "cleanup_calls",
        "source_path",
        "verdict",
        "native_unmap_outcomes",
    ] {
        assert!(!text.contains(derived), "`{derived}` travelled");
    }

    // The sibling body and the other family round-trip through the same
    // package under their own contracts.
    for (source, function, records, contract) in [
        (
            OUTCOMES_SOURCE,
            CLOSE,
            &OUTCOMES_RECORDS[..],
            OUTCOMES_CONTRACT,
        ),
        (DELETE_SOURCE, DELETE, &DELETE_RECORDS[..], DELETE_CONTRACT),
    ] {
        let _ = contract;
        let (_, task) = route(source, function, records, UNIX_TU);
        let outcome = registry.verify_v2(&task.request(plugin()), task.task(), Some(&task));
        assert_eq!(
            outcome.verdict,
            ProposedVerdict::Safe,
            "{function}: {:?}",
            outcome.diagnostics
        );
    }

    // Missing evidence: no core checker, no registered provider, a shipped
    // package that does not implement the capability, and a disabled provider
    // each stay Unknown with no certificate.
    let no_checker = registry.verify_v2(&request, unmap.task(), None);
    assert_eq!(no_checker.verdict, ProposedVerdict::Unknown);
    assert!(no_checker.diagnostics.iter().any(|d| d.code == "QP016"));
    assert!(no_checker.certificate.is_none());
    assert_eq!(
        VerifierRegistry::default()
            .verify_v2(&request, unmap.task(), Some(&unmap))
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
    let foreign = others.verify_v2(&foreign_request, unmap.task(), Some(&unmap));
    assert_eq!(foreign.verdict, ProposedVerdict::Unknown);
    assert!(foreign.certificate.is_none());

    // Tampered transport: a projection the core never imported is refused by
    // the host's digest binding; rebinding it to a recomputed task digest gets
    // past that and is then refused by the core's own comparison.
    let mut forged = request.clone();
    {
        let CleanupReplayFamilyInput::NativeCleanupOutcome { body, .. } =
            &mut forged.cleanup_replay_source.as_mut().unwrap().family
        else {
            panic!("wrong family")
        };
        // Drop the field writes from every path: a well-formed body the core
        // never imported.
        for path in &mut body.paths {
            path.operations
                .retain(|op| !matches!(op, CleanupReplayOperationRecord::Write { .. }));
        }
    }
    let rejection = registry.verify_v2(&forged, unmap.task(), Some(&unmap));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(
        rejection.diagnostics.iter().any(|d| d.code == "QP001"),
        "{:?}",
        rejection.diagnostics
    );
    let mut forged_task = unmap.task().clone();
    forged_task.contract_sources.insert(
        CLEANUP_REPLAY_SOURCE_INPUT_KEY.into(),
        forged.cleanup_replay_source.as_ref().unwrap().digest(),
    );
    forged.task_digest = forged_task.task_digest();
    let rejection = registry.verify_v2(&forged, &forged_task, Some(&unmap));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.certificate.is_none());

    // A body the process replays correctly still cannot buy a false contract.
    let (import, _) = route(OUTCOMES_SOURCE, UNMAP, &OUTCOMES_RECORDS, UNIX_TU);
    let false_contract = CleanupReplaySourceTask::derive(
        &import,
        prerequisite(
            OUTCOMES_SOURCE,
            Vec::new(),
            UNMAP,
            &OUTCOMES_RECORDS,
            UNIX_TU,
        ),
        UNIX_TU,
        &OUTCOMES_CONTRACT.replace("ensures normal ", "ensures normal false && "),
        "native_unmap_outcomes",
        target(&import),
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

    registry.set_enabled(&plugin(), false).unwrap();
    assert_eq!(
        registry
            .verify_v2(&request, unmap.task(), Some(&unmap))
            .verdict,
        ProposedVerdict::Unknown
    );
    std::fs::remove_dir_all(output).unwrap();
}
