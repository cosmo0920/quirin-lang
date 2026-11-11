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

//! Real typed C imports → packaged ChunkIO process → trusted composition, for
//! the composition capability (card P04-COMPOSE).
//!
//! This is the end-to-end route: the shipped executable recomputes the
//! prerequisite reading and the wrapper replay itself over the transported
//! paths and profile, and the core admits it only after re-establishing all
//! three prerequisites, rebuilding the record profile from its own contract and
//! re-running its own unedited composition.
#![cfg(not(target_os = "windows"))]

#[path = "../../../crates/qcore-verification/tests/fixtures/composition_source.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::SemanticWitness;
use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, CompositionFamilyInput, CompositionProposalRecord,
    CompositionSourceInput,
};
use qcore_schemas::{PluginId, ProposedVerdict};
use qcore_verification::composition_source::CompositionSourceTask;
use qcore_verifier_host::{ProcessProvider, VerifierRegistry};
use std::process::Command;
use std::sync::Arc;

const COMMITTED: &str =
    include_str!("../../quirin-provider-shared/fixtures/unmap_wrapper_composition_projection.json");

fn plugin() -> PluginId {
    PluginId::new("qcore.chunkio").unwrap()
}

fn build_packages() -> std::path::PathBuf {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output =
        std::env::temp_dir().join(format!("qcore-composition-package-{}", std::process::id()));
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
/// The three prerequisite triples are **not** content-only functions of the
/// bodies: `SourceInputIdentity` binds `std::env::current_dir()` by design, so
/// each triple differs between the crate the fixture was generated in and this
/// one. Everything the provider actually composes over — the schema version,
/// both bodies' complete path sets and the declared prerequisite profile — is
/// compared exactly; the triples are checked for shape and semantic profile
/// only, and the working-directory sensitivity is recorded as the inherited
/// boundary `P04-B2-PREREQ-IDENTITY-CWD` rather than asserted away.
#[test]
fn the_provider_fixture_is_the_live_projection() {
    if !clang_available() {
        return;
    }
    let committed: CompositionSourceInput = serde_json::from_str(COMMITTED).unwrap();
    let prereqs = prerequisites(SOURCE, UNIX_TU);
    let live = import(SOURCE, identities(&prereqs)).source_input();
    assert_eq!(committed.schema_version, live.schema_version);
    assert_eq!(committed.family, live.family, "the projection drifted");
    let committed_set = committed
        .prerequisites
        .unmap_wrapper()
        .expect("the leaf fixture carries the leaf set");
    let live_set = live
        .prerequisites
        .unmap_wrapper()
        .expect("the live leaf projection carries the leaf set");
    for (a, b) in [
        (&committed_set.wrapper_call_use, &live_set.wrapper_call_use),
        (&committed_set.native_cleanup, &live_set.native_cleanup),
        (&committed_set.counter, &live_set.counter),
    ] {
        assert_eq!(a.semantic_profile, b.semantic_profile);
        assert!(a.is_complete() && b.is_complete());
        // The measurement itself: a future change that made a triple
        // content-only fails here and is re-evaluated rather than inherited.
        assert_ne!(
            a.task_digest, b.task_digest,
            "a prerequisite identity became content-only; re-evaluate \
             P04-B2-PREREQ-IDENTITY-CWD"
        );
    }
    assert!(committed.prerequisites.are_distinct());
    assert!(committed.family.matches(&committed.prerequisites));
}

#[test]
fn real_package_composes_without_admitting_false_contracts() {
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

    // Positive: the process composes on its own and the core admits it only
    // after re-establishing all three prerequisites and re-running its own
    // composition over its own contracts.
    let task = route(SOURCE, UNIX_TU);
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
    let CompositionProposalRecord::UnmapWrapper(record) = &witness.summary else {
        panic!("the leaf route must publish a leaf reading")
    };
    assert_eq!(record.function, WRAPPER);
    assert_eq!(record.cases.len(), 8);
    assert_eq!(
        record
            .cases
            .iter()
            .filter(|case| case.native_source_path.is_some())
            .count(),
        6
    );

    // The request carries the question only: both bodies' paths, the declared
    // prerequisite profile and three identity triples. No composed reading and
    // no prerequisite conclusion travels.
    let value = serde_json::to_value(request.composition_source.as_ref().unwrap()).unwrap();
    assert!(value["family"]["wrapper"]["paths"].is_array());
    assert!(value["family"]["native"]["paths"].is_array());
    assert!(value["family"]["native_profile"]["fields"].is_array());
    let text = serde_json::to_string(&value).unwrap();
    for derived in [
        "cases",
        "native_source_path",
        "counter_decrements",
        "wrapper_writes",
        "field_writes",
        "fields_reset",
        "verdict",
        "certificate",
        NATIVE_POLICY,
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
        let CompositionFamilyInput::UnmapWrapper { wrapper, .. } =
            &mut forged.composition_source.as_mut().unwrap().family
        else {
            panic!("the leaf route must carry the leaf family")
        };
        // A projection the process still answers — the composed reading is
        // recomputed the same way — but one the core never imported.
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

    // A composition the process computes correctly still cannot buy a false
    // contract — the core's own evaluation decides.
    let mut false_composition = pinned();
    let falsified = COMPOSITION_CONTRACT.replace("ensures normal ", "ensures normal false && ");
    false_composition.composition = &falsified;
    let false_task = build_route(SOURCE, UNIX_TU, &false_composition).unwrap();
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
    // composition's own contract untouched: the prerequisite is re-established
    // by the core, never read out of the proposal.
    let mut false_prerequisite = pinned();
    let falsified_outcomes = {
        let anchor = OUTCOMES_CONTRACT.find(NATIVE_POLICY).unwrap();
        let marker = "ensures normal ";
        let i = anchor + OUTCOMES_CONTRACT[anchor..].find(marker).unwrap();
        format!(
            "{}{marker}false && {}",
            &OUTCOMES_CONTRACT[..i],
            &OUTCOMES_CONTRACT[i + marker.len()..]
        )
    };
    false_prerequisite.outcomes = &falsified_outcomes;
    let prerequisite_task = build_route(SOURCE, UNIX_TU, &false_prerequisite).unwrap();
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
    let _: fn(&CompositionSourceTask) -> &qcore_schemas::VerificationTask = |t| t.task();
}
