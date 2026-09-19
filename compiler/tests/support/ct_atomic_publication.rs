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
                "relaxed_store.c": "store_not_release", "relaxed_load.c": "load_not_acquire"
            }
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
    let mut wanted_files: BTreeSet<_> = controls.keys().cloned().collect();
    wanted_files.extend(["case.c".into(), "expected.json".into()]);
    assert_eq!(actual_files, wanted_files);
    if !legacy_clang_available("CT41") {
        return;
    }
    let source = fs::read_to_string(directory.join("case.c")).unwrap();
    let plain = fs::read_to_string(directory.join("plain.c")).unwrap();
    assert_eq!(source.replace("volatile ", ""), plain);
    let atomic = fs::read_to_string(directory.join("release_acquire.c")).unwrap();
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
        fs::read_to_string(directory.join("relaxed_store.c")).unwrap(),
        atomic.replace("memory_order_release", "memory_order_relaxed")
    );
    assert_eq!(
        fs::read_to_string(directory.join("relaxed_load.c")).unwrap(),
        atomic.replace("memory_order_acquire", "memory_order_relaxed")
    );

    for (file, outcome) in std::iter::once(("case.c", "volatile_no_synchronization")).chain(
        controls
            .iter()
            .map(|(file, outcome)| (file.as_str(), outcome.as_str().unwrap())),
    ) {
        let source = fs::read_to_string(directory.join(file)).unwrap();
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
