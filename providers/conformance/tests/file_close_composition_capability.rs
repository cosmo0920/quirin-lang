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

//! Real typed C imports → packaged ChunkIO process → trusted two-level
//! composition, for both `cio_file_close` entry points (card `P04-COMPOSE-A`).
//!
//! This is the end-to-end route: the shipped executable recomputes the **nested
//! composed reading**, both cleanup readings and the wrapper replay itself over
//! the transported paths and profiles, and the core admits it only after
//! re-establishing every prerequisite at every level — the nested composition
//! re-establishing its own three in turn — rebuilding both record profiles from
//! its own contracts and re-running its own unedited compositions.
#![cfg(not(target_os = "windows"))]

#[path = "../../../crates/qcore-verification/tests/fixtures/file_close_composition.rs"]
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
    include_str!("../../quirin-provider-shared/fixtures/file_close_composition_projection.json");
const COMMITTED_DELETE: &str = include_str!(
    "../../quirin-provider-shared/fixtures/file_close_delete_composition_projection.json"
);

fn plugin() -> PluginId {
    PluginId::new("qcore.chunkio").unwrap()
}

fn build_packages() -> std::path::PathBuf {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output =
        std::env::temp_dir().join(format!("qcore-file-close-package-{}", std::process::id()));
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
/// the importer no longer emits and the matrix would test nothing.
///
/// The prerequisite identities are **not** content-only functions of the bodies:
/// `SourceInputIdentity` binds `std::env::current_dir()` by design, so each
/// differs between the crate the fixtures were generated in and this one.
/// Everything the provider actually composes over — the schema version, all five
/// bodies' complete path sets, the nested question and every declared profile —
/// is compared exactly; the identities are checked for shape and semantic
/// profile only, and the working-directory sensitivity is recorded as the
/// inherited boundary `P04-B2-PREREQ-IDENTITY-CWD` rather than asserted away.
#[test]
fn the_provider_fixtures_are_the_live_projections() {
    if !clang_available() {
        return;
    }
    let rc = pinned();
    for (text, deleting) in [(COMMITTED, false), (COMMITTED_DELETE, true)] {
        let committed: CompositionSourceInput = serde_json::from_str(text).unwrap();
        let prereqs = prerequisites(SOURCE, UNIX_TU, &rc, deleting);
        let live = import(SOURCE, &rc, deleting, identities(&prereqs)).source_input();
        assert_eq!(committed.schema_version, live.schema_version);
        assert_eq!(committed.family, live.family, "the projection drifted");
        assert_eq!(committed.family.nesting_depth(), 2);
        let committed_set = committed
            .prerequisites
            .file_close()
            .expect("the file_close fixture carries the file_close set");
        let live_set = live
            .prerequisites
            .file_close()
            .expect("the live projection carries the file_close set");
        let mut pairs = vec![
            (&committed_set.wrapper_call_use, &live_set.wrapper_call_use),
            (
                &committed_set.unmap_composition,
                &live_set.unmap_composition,
            ),
            (&committed_set.native_cleanup, &live_set.native_cleanup),
        ];
        if deleting {
            pairs.push((
                committed_set.native_delete.as_ref().unwrap(),
                live_set.native_delete.as_ref().unwrap(),
            ));
        } else {
            assert!(committed_set.native_delete.is_none());
        }
        for (a, b) in pairs {
            assert_eq!(a.semantic_profile, b.semantic_profile);
            assert!(a.is_complete() && b.is_complete());
            // The measurement itself: a future change that made an identity
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
}

#[test]
fn real_package_composes_two_levels_without_admitting_false_contracts() {
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

    for deleting in [false, true] {
        // Positive: the process composes both levels on its own and the core
        // admits it only after re-establishing every prerequisite at every level
        // and re-running its own compositions over its own contracts.
        let task = route(SOURCE, UNIX_TU, deleting);
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
        let CompositionProposalRecord::FileClose(record) = &witness.summary else {
            panic!("the file_close route must publish a file_close reading")
        };
        assert_eq!(record.function, WRAPPER);
        assert_eq!(record.cases.len(), 43);
        assert_eq!(record.deleting_cases.is_some(), deleting);
        assert!(
            record
                .cases
                .iter()
                .all(|case| case.unmap_case.is_some() == case.backend_present)
        );

        // The request carries the question only: five bodies' paths, the nested
        // question, three declared profiles and the identity keys. No composed
        // reading and no prerequisite conclusion travels, at either level.
        let value = serde_json::to_value(request.composition_source.as_ref().unwrap()).unwrap();
        assert!(value["family"]["wrapper"]["paths"].is_array());
        assert!(value["family"]["unmap"]["wrapper"]["paths"].is_array());
        assert!(value["family"]["unmap"]["native"]["paths"].is_array());
        assert!(value["family"]["close"]["paths"].is_array());
        assert!(value["family"]["close_profile"]["fields"].is_array());
        assert_eq!(value["family"]["delete"].is_object(), deleting);
        let text = serde_json::to_string(&value).unwrap();
        for derived in [
            "cases",
            "unmap_case",
            "close_case",
            "delete_case",
            "native_source_path",
            "counter_decrements",
            "wrapper_writes",
            "field_writes",
            "fields_reset",
            "verdict",
            "certificate",
            CLOSE_POLICY,
            UNMAP_POLICY,
            DELETE_POLICY,
        ] {
            assert!(!text.contains(derived), "`{derived}` travelled");
        }

        // Missing evidence: no core checker, no registered provider, and a
        // shipped package that does not implement this capability each stay
        // Unknown with no certificate.
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
        let absent = others.verify_v2(&request, task.task(), Some(&task));
        assert_eq!(absent.verdict, ProposedVerdict::Unknown);
        assert!(absent.certificate.is_none());

        // Tampered transport: a projection the core never imported is refused by
        // the host's digest binding; rebinding it to a recomputed task digest
        // gets past that and is then refused by the core's own comparison.
        let mut forged = request.clone();
        {
            let CompositionFamilyInput::FileClose { wrapper, .. } =
                &mut forged.composition_source.as_mut().unwrap().family
            else {
                panic!("the file_close route must carry the file_close family")
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
    }

    // A reading the process computes correctly still cannot buy a false
    // contract, at any level — the core's own evaluation decides.
    for (name, falsified) in [
        ("the composition's own contract", {
            let mut rc = pinned();
            rc.close = Box::leak(
                CLOSE_CONTRACT
                    .replace("ensures normal ", "ensures normal false && ")
                    .into_boxed_str(),
            );
            rc
        }),
        ("the nested composition's contract", {
            let mut rc = pinned();
            rc.unmap = Box::leak(
                UNMAP_CONTRACT
                    .replace("ensures normal ", "ensures normal false && ")
                    .into_boxed_str(),
            );
            rc
        }),
        ("a leaf prerequisite's contract", {
            let mut rc = pinned();
            rc.counter = Box::leak(
                COUNTER_CONTRACT
                    .replace("ensures normal ", "ensures normal false && ")
                    .into_boxed_str(),
            );
            rc
        }),
    ] {
        let task = build_route(SOURCE, UNIX_TU, &falsified, false).unwrap();
        let rejection = registry.verify_v2(&task.request(plugin()), task.task(), Some(&task));
        assert_eq!(
            rejection.verdict,
            ProposedVerdict::Unknown,
            "a Safe proposal bypassed {name}"
        );
        assert!(rejection.certificate.is_none());
    }

    // The first row above additionally names the contract-evaluation diagnostic.
    {
        let mut rc = pinned();
        rc.close = Box::leak(
            CLOSE_CONTRACT
                .replace("ensures normal ", "ensures normal false && ")
                .into_boxed_str(),
        );
        let task = build_route(SOURCE, UNIX_TU, &rc, false).unwrap();
        let rejection = registry.verify_v2(&task.request(plugin()), task.task(), Some(&task));
        assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));
    }

    let disabled = route(SOURCE, UNIX_TU, false);
    registry.set_enabled(&plugin(), false).unwrap();
    assert_eq!(
        registry
            .verify_v2(
                &disabled.request(plugin()),
                disabled.task(),
                Some(&disabled)
            )
            .verdict,
        ProposedVerdict::Unknown
    );

    std::fs::remove_dir_all(output).unwrap();
}
