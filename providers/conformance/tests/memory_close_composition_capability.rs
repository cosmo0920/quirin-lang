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

//! Real typed C imports from **derived** text → packaged ChunkIO process →
//! trusted composition, for the memory-close capability (card
//! `P04-COMPOSE-MEMCLOSE`).
//!
//! This is the end-to-end route. The shipped executable reads the backend
//! discriminant out of the transported probe body and replays the wrapper over
//! both entry states itself; the core admits it only after re-establishing all
//! four prerequisites, checking the transported derivation against the one its
//! own importer measured under its own recipe, and re-running its own unedited
//! composition.
#![cfg(not(target_os = "windows"))]

#[path = "../../../crates/qcore-verification/tests/fixtures/memory_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::SemanticWitness;
use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, CompositionFamilyInput, CompositionProposalRecord,
    CompositionSourceInput,
};
use qcore_schemas::{PluginId, ProposedVerdict};
use qcore_verifier_host::{ProcessProvider, VerifierRegistry};
use std::process::Command;
use std::sync::Arc;

const COMMITTED: &str =
    include_str!("../../quirin-provider-shared/fixtures/memory_close_composition_projection.json");

fn plugin() -> PluginId {
    PluginId::new("qcore.chunkio").unwrap()
}

fn build_packages() -> std::path::PathBuf {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output =
        std::env::temp_dir().join(format!("qcore-memory-close-package-{}", std::process::id()));
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

/// The projection the provider unit tests read is the one the live gateway
/// still produces. Without this the provider fixture could drift into a shape
/// the importer no longer emits and the matrix would test nothing.
///
/// As on every other transported route, the four prerequisite triples are not
/// content-only functions of the bodies — `SourceInputIdentity` binds
/// `current_dir()` by design — so they are checked for shape and semantic
/// profile, and the working-directory sensitivity stays the inherited boundary
/// `P04-B2-PREREQ-IDENTITY-CWD`. **The derivation record is compared exactly**,
/// because it is a function of the text alone.
#[test]
fn the_provider_fixture_is_the_live_projection() {
    if !clang_available() {
        return;
    }
    let committed: CompositionSourceInput = serde_json::from_str(COMMITTED).unwrap();
    let tasks = prerequisites(SOURCE);
    let selection = selection(identities(&tasks), callees(&tasks), PROBE, PROBE_FUNCTION);
    let live = import(SOURCE, Vec::new(), &selection).source_input();
    assert_eq!(committed.schema_version, live.schema_version);
    assert_eq!(committed.family, live.family, "the projection drifted");
    let committed_set = committed
        .prerequisites
        .memory_close()
        .expect("the memory-close fixture carries the memory-close set");
    let live_set = live
        .prerequisites
        .memory_close()
        .expect("the live projection carries the memory-close set");
    for (a, b) in [
        (&committed_set.reset, &live_set.reset),
        (&committed_set.backend, &live_set.backend),
        (&committed_set.unlink, &live_set.unlink),
        (&committed_set.counter, &live_set.counter),
    ] {
        assert_eq!(a.semantic_profile, b.semantic_profile);
        assert!(a.is_complete() && b.is_complete());
    }
    assert!(committed.prerequisites.are_distinct());
    assert!(committed.family.matches(&committed.prerequisites));
    // The derivation is a function of the text, so it must agree exactly.
    assert_eq!(
        committed.family.derivation(),
        live.family.derivation(),
        "the recorded derivation drifted from the live one"
    );
}

#[test]
fn real_package_composes_the_derived_question_without_admitting_false_contracts() {
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

    // Positive: the process reads the discriminant out of the probe body and
    // replays the wrapper itself; the core admits only after re-establishing
    // all four prerequisites and re-running its own composition.
    let task = task();
    let request = task.request(plugin());
    let result = registry.verify_v2(&request, task.task(), Some(&task));
    assert_eq!(
        result.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        result.diagnostics
    );
    let certificate = result.certificate.unwrap();
    assert_eq!(certificate.task(), Some(task.task()));
    assert_eq!(certificate.witness().unwrap().kind(), "composition");
    let Some(SemanticWitness::Composition(witness)) = certificate.witness() else {
        panic!("wrong witness kind")
    };
    let CompositionProposalRecord::MemoryClose(record) = &witness.summary else {
        panic!("the memory-close route must publish a memory-close reading")
    };
    assert_eq!(record.function, WRAPPER);
    assert_eq!(record.cases.len(), 2);
    assert_eq!(record.memory_kind, 1);
    assert_eq!(record.checked_paths.len(), 2);
    assert_eq!(record.uncovered_paths.len(), 2);

    // The request carries the question only: both bodies' paths, the derivation
    // as a claim and four identity triples. No composed reading, no probed
    // value and no prerequisite conclusion travels.
    let value = serde_json::to_value(request.composition_source.as_ref().unwrap()).unwrap();
    assert!(value["family"]["wrapper"]["paths"].is_array());
    assert!(value["family"]["probe"]["paths"].is_array());
    assert!(value["family"]["derivation"]["recipe"].is_string());
    let text = serde_json::to_string(&value).unwrap();
    for derived in [
        "cases",
        "memory_kind",
        "checked_paths",
        "uncovered_paths",
        "backend_discharged",
        "counter_decremented",
        "verdict",
        "certificate",
    ] {
        assert!(!text.contains(derived), "`{derived}` travelled");
    }

    // Missing evidence: no core checker, no registered provider, a shipped
    // package that does not implement the capability, and a disabled provider
    // each stay Unknown with no certificate.
    let no_checker = registry.verify_v2(&request, task.task(), None);
    assert_eq!(no_checker.verdict, ProposedVerdict::Unknown);
    assert!(no_checker.diagnostics.iter().any(|d| d.code == "QP016"));
    assert!(no_checker.certificate.is_none());
    assert_eq!(
        VerifierRegistry::default()
            .verify_v2(&request, task.task(), Some(&task))
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
    let foreign = others.verify_v2(&foreign_request, task.task(), Some(&task));
    assert_eq!(foreign.verdict, ProposedVerdict::Unknown);
    assert!(foreign.certificate.is_none());

    // Tampered transport: a projection the core never imported is refused by
    // the host's digest binding; rebinding it to a recomputed task digest gets
    // past that and is then refused by the core's own comparison.
    let mut forged = request.clone();
    {
        let CompositionFamilyInput::MemoryClose { wrapper, .. } =
            &mut forged.composition_source.as_mut().unwrap().family
        else {
            panic!("the memory-close route must carry the memory-close family")
        };
        wrapper
            .profile_assumptions
            .insert("forged".into(), "assumption".into());
    }
    let rejection = registry.verify_v2(&forged, task.task(), Some(&task));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(
        rejection.diagnostics.iter().any(|d| d.code == "QP001"),
        "{:?}",
        rejection.diagnostics
    );
    let mut forged_task = task.task().clone();
    forged_task.contract_sources.insert(
        COMPOSITION_SOURCE_INPUT_KEY.into(),
        forged.composition_source.as_ref().unwrap().digest(),
    );
    forged.task_digest = forged_task.task_digest();
    let rejection = registry.verify_v2(&forged, &forged_task, Some(&task));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.certificate.is_none());

    // **The derived-text row, end to end.** A projection derived under a
    // substituted recipe is refused by the core's own derivation check, and the
    // process's own honest recomputation buys nothing.
    let substituted: &'static str =
        "\nint qcore_cio_memory_store_value(void) { return CIO_STORE_FS; }\n";
    assert!(
        task_with_recipe(SOURCE, substituted, &contracts()).is_err(),
        "a projection derived under a substituted recipe reached a task"
    );

    // A composition the process computes correctly still cannot buy a false
    // contract — the core's own evaluation decides.
    let falsified: &'static str = Box::leak(
        COMPOSITION_CONTRACT
            .replace("ensures normal ", "ensures normal false && ")
            .into_boxed_str(),
    );
    let false_task = task_from(
        SOURCE,
        prerequisites(SOURCE),
        &contracts_with(falsified, PROBE),
    )
    .unwrap();
    let rejection = registry.verify_v2(
        &false_task.request(plugin()),
        false_task.task(),
        Some(&false_task),
    );
    assert_eq!(
        rejection.verdict,
        ProposedVerdict::Unknown,
        "a Safe proposal bypassed the composition's own contract evaluation"
    );
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));

    // And a false *prerequisite* contract is refused too, with the
    // composition's own contract untouched: every prerequisite is
    // re-established by the core, never read out of the proposal.
    let falsified_counter = falsify_all_ensures(COUNTER_CONTRACT);
    let prerequisite_task = task_from(
        SOURCE,
        prerequisites_with(
            SOURCE,
            FRAME_CONTRACT,
            BACKEND_CONTRACT,
            UNLINK_CONTRACT,
            &falsified_counter,
        ),
        &contracts(),
    )
    .unwrap();
    let rejection = registry.verify_v2(
        &prerequisite_task.request(plugin()),
        prerequisite_task.task(),
        Some(&prerequisite_task),
    );
    assert_eq!(
        rejection.verdict,
        ProposedVerdict::Unknown,
        "a composition was admitted over a falsified prerequisite contract"
    );

    registry.set_enabled(&plugin(), false).unwrap();
    assert_eq!(
        registry
            .verify_v2(&request, task.task(), Some(&task))
            .verdict,
        ProposedVerdict::Unknown
    );
    std::fs::remove_dir_all(output).unwrap();
}
