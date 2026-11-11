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

//! Attributed CT41 property-only C route; native/proof evidence is separate.
use super::*;
use qcore_c_gateway::atomic_publication_source::AtomicPublicationSourceRefusal as SourceRefusal;
use qcore_core::AtomicPublicationRefusal as ModelRefusal;
use qcore_verification::atomic_publication_source::{
    AtomicPublicationExecutionBounds, AtomicPublicationRefusal, AtomicPublicationSourceProfile,
    check_atomic_publication_source,
};

pub(super) fn run(case: &ManifestCase) {
    let directory = fixture_root().join("CT41");
    let read_source = |file: &str| {
        fs::read_to_string(directory.join(file))
            .unwrap()
            .replace("\r\n", "\n")
    };
    let expected: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(directory.join("expected.json")).unwrap())
            .unwrap();
    assert_eq!(
        expected,
        serde_json::json!({
            "id": "CT41", "verdict": "no_synchronization_proof",
            "property": "payload write happens before successful consumer payload read",
            "bounds": [1, 1],
            "controls": {
                "plain.c": "plain_no_synchronization", "release_acquire.c": "ordered_payload",
                "relaxed_store.c": "store_not_release", "relaxed_load.c": "load_not_acquire",
                // Card `CT41-MAP`: the two arms the landed control set does not
                // reach. Neither duplicates one above; each is measured below.
                "qualified_atomic.c": "qualifier_on_atomic_unsupported"
            },
            // Card `CT41-MAP`. A control that is deliberately NOT valid C11, so
            // it cannot ride the loop below, whose first act is to require the
            // artifact to be admissible C in its own right. It is the
            // measurement behind this fixture's shape rather than an
            // attribution: see the runner's own comment at the block that
            // drives it.
            "language_boundary": { "volatile_atomic_ops.c": "not_valid_c11" }
        })
    );
    assert_eq!(case.route.as_deref(), Some("atomic_publication"));
    assert_eq!(
        case.harness_verdict_mode.as_deref(),
        Some("no_synchronization_proof")
    );
    assert_eq!(
        case.semantic_profile.as_deref(),
        Some("c11-bounded-atomic-publication-v1")
    );
    assert!(case.evidence_target.is_none());
    let artifact = "compiler/tests/fixtures/ct/CT41/case.c";
    assert_eq!(case.source_fixture.as_deref(), Some(artifact));
    let runner = "compiler/tests/ct_corpus.rs::atomic_publication::run";
    assert!(RUNNER_REGISTRY.contains(&runner));
    for stage in STAGE_NAMES {
        let wanted = if matches!(*stage, "parse" | "atomic_publication_check") {
            StageRecord {
                status: "executed".into(),
                executor: Some(runner.into()),
                artifact: Some(artifact.into()),
                observations: Some(1),
                tool_gate: Some(vec!["clang".into()]),
                host_gate: None,
                record: None,
            }
        } else {
            StageRecord {
                status: if *stage == "proof" {
                    "unassessed"
                } else {
                    "not_applicable"
                }
                .into(),
                executor: None,
                artifact: None,
                observations: None,
                tool_gate: None,
                host_gate: None,
                record: None,
            }
        };
        assert_eq!(case.stages[*stage], wanted, "CT41 stage {stage}");
    }
    let controls = expected["controls"].as_object().unwrap();
    let actual_files: BTreeSet<_> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    let boundary = expected["language_boundary"].as_object().unwrap();
    let mut wanted_files: BTreeSet<_> = controls.keys().cloned().collect();
    wanted_files.extend(boundary.keys().cloned());
    wanted_files.extend(["case.c".into(), "expected.json".into()]);
    assert_eq!(actual_files, wanted_files);
    if !legacy_clang_available("CT41") {
        return;
    }
    let source = read_source("case.c");
    let plain = read_source("plain.c");
    assert_eq!(source.replace("volatile ", ""), plain);
    let atomic = read_source("release_acquire.c");
    let replacement = "#include <stdatomic.h>\n".to_owned()
        + &plain
            .replace("static int done", "static _Atomic int done")
            .replace(
                "done = 1;",
                "atomic_store_explicit(&done, 1, memory_order_release);",
            )
            .replace(
                "int seen = done;",
                "int seen = atomic_load_explicit(&done, memory_order_acquire);",
            );
    assert_eq!(
        atomic, replacement,
        "the accepting control changes only the flag protocol"
    );
    assert_eq!(
        read_source("relaxed_store.c"),
        atomic.replace("memory_order_release", "memory_order_relaxed")
    );
    assert_eq!(
        read_source("relaxed_load.c"),
        atomic.replace("memory_order_acquire", "memory_order_relaxed")
    );
    // Card `CT41-MAP`: the two added controls are derived from the accepting
    // control by a single declaration substitution each, so what they change is
    // the flag's TYPE and nothing else.
    assert_eq!(
        read_source("qualified_atomic.c"),
        atomic.replace(
            "static _Atomic int done",
            "static volatile _Atomic int done"
        ),
        "the qualified-atomic control changes only the flag declaration"
    );
    assert_eq!(
        read_source("volatile_atomic_ops.c"),
        atomic.replace("static _Atomic int done", "static volatile int done"),
        "the language-boundary control changes only the flag declaration"
    );

    // Card `CT41-MAP`. The landed attribution replaces the flag PROTOCOL and
    // says so; this block measures WHY the alternative -- replacing the
    // qualifier alone, one token, and reaching the accepting side -- does not
    // exist to be chosen. C11 admits no atomic operation on a volatile
    // non-atomic object, so `release_acquire.c` with `_Atomic` written
    // `volatile` is not a program at all. It is therefore checked as a language
    // fact, never used as CT41's attribution: a parser rejection is exactly the
    // "the projection cannot express this at all" answer the historical control
    // clause excludes.
    for (file, outcome) in boundary {
        assert_eq!(outcome.as_str().unwrap(), "not_valid_c11");
        let source = read_source(file);
        let error = validate_unrestricted_c11(&C11TranslationUnit {
            source: &source,
            ..Default::default()
        })
        .expect_err("a language-boundary control is not admissible C11");
        assert!(
            format!("{error:?}")
                .contains("address argument to atomic operation must be a pointer to _Atomic type"),
            "CT41 {file}: C itself must refuse this, and for the stated reason"
        );
    }

    // Clang 17 accepts the C11 programs but emits the older atomic AST schema,
    // so the source route deliberately returns UnsupportedPublisher. Keep the
    // fixture/derivation and language-boundary checks above active; only the
    // CG19 schema-dependent observations require Clang 18 or newer.
    if !qcore_c_gateway::clang_version().is_ok_and(|version| version.supports_cg19_atomic_ast()) {
        return;
    }

    for (file, outcome) in std::iter::once(("case.c", "volatile_no_synchronization")).chain(
        controls
            .iter()
            .map(|(file, outcome)| (file.as_str(), outcome.as_str().unwrap())),
    ) {
        let source = read_source(file);
        let unit = C11TranslationUnit {
            source: &source,
            ..Default::default()
        };
        validate_unrestricted_c11(&unit).expect("CT41 artifact must be valid C11");
        let result = check_atomic_publication_source(
            &unit,
            "ct41-publication-mapping-v1",
            &AtomicPublicationSourceProfile {
                publisher: "publish",
                consumer: "consume",
                flag: "done",
                payload: "payload",
            },
            AtomicPublicationExecutionBounds {
                publisher_invocations: 1,
                successful_consumer_invocations: 1,
            },
        );
        match outcome {
            "volatile_no_synchronization" => assert_eq!(
                result,
                Err(AtomicPublicationRefusal::Source(
                    SourceRefusal::VolatileFlagHasNoSynchronization
                ))
            ),
            "plain_no_synchronization" => assert_eq!(
                result,
                Err(AtomicPublicationRefusal::Source(
                    SourceRefusal::PlainFlagHasNoSynchronization
                ))
            ),
            "store_not_release" => assert_eq!(
                result,
                Err(AtomicPublicationRefusal::Model(
                    ModelRefusal::StoreIsNotRelease
                ))
            ),
            "load_not_acquire" => assert_eq!(
                result,
                Err(AtomicPublicationRefusal::Model(
                    ModelRefusal::LoadIsNotAcquire
                ))
            ),
            // Card `CT41-MAP`: the flag-type arm neither qualifier refusal
            // reaches. A `volatile _Atomic int` IS a synchronization primitive,
            // so reporting it under the volatile arm would state something false
            // of it; and this route models exactly `_Atomic(int)`, so admitting
            // it would widen the shape. It refuses as the fail-closed third
            // answer, which is what bounds the volatile arm to a flag that is
            // not atomic -- the discrimination the historical `typed_source`
            // qualifier branch cannot make (it refuses `volatile`, `_Atomic`
            // and `restrict` in one branch with one message, unchanged here;
            // card CG19-ATOMIC-SOURCE-ROUTE later gave `_Atomic` its own arm).
            "qualifier_on_atomic_unsupported" => assert_eq!(
                result,
                Err(AtomicPublicationRefusal::Source(
                    SourceRefusal::UnsupportedAtomicFlag
                ))
            ),
            "ordered_payload" => {
                let certificate = result.unwrap();
                assert_eq!(
                    certificate.source.artifact_raw_sha256,
                    qcore_schemas::sha256_hex(source.as_bytes())
                );
                assert_eq!(certificate.source.bounds.publisher_invocations, 1);
                assert_eq!(certificate.source.bounds.successful_consumer_invocations, 1);
                assert_eq!(
                    certificate.semantic_profile,
                    "c11-bounded-atomic-publication-v1"
                );
                assert_eq!(
                    certificate.model.payload_happens_before,
                    qcore_core::weak_memory::RelationEdge::new(1, 4)
                );
                assert!(
                    certificate
                        .model
                        .execution
                        .happens_before
                        .contains(&certificate.model.payload_happens_before)
                );
                assert!(!certificate.assumptions.is_empty());
            }
            _ => panic!("unknown CT41 expectation"),
        }
    }
}

#[test]
fn ct41_attributed_publication_case_and_controls() {
    let manifest = load_manifest();
    run(manifest
        .cases
        .iter()
        .find(|case| case.id == "CT41")
        .unwrap());
}
