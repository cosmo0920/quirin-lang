//! Provider-side matrix for the composition producer, driven over the **real**
//! projection the gateway emits for these bodies.
//!
//! Every refusal row is a *decline*: the process returns no proposal, the host
//! sees no witness, and the obligation stays Unknown. Nothing here evaluates a
//! contract or asserts anything about the OS.
use super::*;
use qcore_schemas::cleanup_replay_summary::{CleanupReplayPathRecord, CleanupResetRecord};

const PROJECTION: &str = include_str!("fixtures/unmap_wrapper_composition_projection.json");

fn projection() -> CompositionSourceInput {
    serde_json::from_str(PROJECTION).expect("the committed projection parses")
}

fn parts() -> (
    CleanupReplayFunctionRecord,
    CleanupReplayFunctionRecord,
    CleanupProfileRecord,
) {
    match projection().family {
        CompositionFamilyInput::UnmapWrapper {
            wrapper,
            native,
            native_profile,
        } => (wrapper, native, native_profile),
    }
}

fn compose(
    wrapper: &CleanupReplayFunctionRecord,
    native: &CleanupReplayFunctionRecord,
    profile: &CleanupProfileRecord,
) -> Option<CompositionRecord> {
    compose_unmap_wrapper(wrapper, native, profile)
}

/// The producer recomputes the whole reading itself: eight entry states, each
/// with its selected wrapper path and the prerequisite's own selected path.
#[test]
fn the_producer_recomputes_the_whole_composed_reading() {
    let (wrapper, native, profile) = parts();
    let record = compose(&wrapper, &native, &profile).expect("the real projection composes");
    assert_eq!(record.function, "munmap_file");
    assert_eq!(record.cases.len(), 8);
    assert_eq!(
        record
            .cases
            .iter()
            .filter(|case| case.native_result.is_some())
            .count(),
        6
    );
    // Only the successful native outcome decrements and writes.
    for case in &record.cases {
        let success = case.native_result == Some(0);
        assert_eq!(case.returned, if success { 0 } else { -1 });
        assert_eq!(case.counter_decrements, u32::from(success));
        assert_eq!(case.wrapper_writes.len(), if success { 2 } else { 0 });
        assert_eq!(
            case.native_source_path.is_some(),
            case.native_result.is_some()
        );
    }
    // The prerequisite reading is genuinely recomputed: the two outcomes select
    // different prerequisite paths.
    let failure = record
        .cases
        .iter()
        .find(|case| case.native_result == Some(-1))
        .unwrap()
        .native_source_path;
    let success = record
        .cases
        .iter()
        .find(|case| case.native_result == Some(0))
        .unwrap()
        .native_source_path;
    assert_ne!(failure, success);
}

/// The prerequisite half is recomputed from the prerequisite's own paths, so a
/// change to *those* changes the composed reading rather than being ignored.
#[test]
fn a_changed_prerequisite_body_changes_the_composed_reading() {
    let (wrapper, native, profile) = parts();
    let honest = compose(&wrapper, &native, &profile).unwrap();
    let mut reordered = native.clone();
    reordered.paths.reverse();
    match compose(&wrapper, &reordered, &profile) {
        // Either the prerequisite reading no longer composes at all, or the
        // prerequisite's selected paths move. What must not happen is silence.
        None => {}
        Some(changed) => assert_ne!(changed, honest),
    }
}

/// Degenerate and hostile projections decline without panicking.
#[test]
fn degenerate_projections_decline_bounded() {
    let (wrapper, native, profile) = parts();
    let mut rows = 0;
    {
        let mut declined = |w: CleanupReplayFunctionRecord,
                            n: CleanupReplayFunctionRecord,
                            p: CleanupProfileRecord| {
            assert!(compose(&w, &n, &p).is_none());
            rows += 1;
        };

        // Empty and oversized bodies.
        let mut empty = wrapper.clone();
        empty.paths.clear();
        declined(empty, native.clone(), profile.clone());
        let mut huge = wrapper.clone();
        huge.paths = std::iter::repeat_n(wrapper.paths[0].clone(), MAX_PATHS + 1).collect();
        declined(huge, native.clone(), profile.clone());
        let mut long = wrapper.clone();
        long.paths[0].operations =
            std::iter::repeat_n(wrapper.paths[0].operations[0].clone(), MAX_OPERATIONS + 1)
                .collect();
        declined(long, native.clone(), profile.clone());
        let mut no_ops = wrapper.clone();
        no_ops.paths[0] = CleanupReplayPathRecord { operations: vec![] };
        declined(no_ops, native.clone(), profile.clone());

        // Identity degeneracies.
        let mut anonymous = wrapper.clone();
        anonymous.function = String::new();
        declined(anonymous, native.clone(), profile.clone());
        let mut wrong_signature = wrapper.clone();
        wrong_signature.signature = "int (struct cio_file *)".into();
        declined(wrong_signature, native.clone(), profile.clone());
        let mut moved_revision = wrapper.clone();
        moved_revision.source_revision = "0".repeat(40);
        declined(moved_revision, native.clone(), profile.clone());
        // The wrapper and the prerequisite cannot be the same body.
        declined(wrapper.clone(), wrapper.clone(), profile.clone());

        // A prerequisite profile the prerequisite replay itself refuses.
        let mut bad_profile = profile.clone();
        bad_profile.fields.clear();
        declined(wrapper.clone(), native.clone(), bad_profile);
        let mut bad_reset = profile.clone();
        bad_reset.fields[0].reset = CleanupResetRecord::Integer { value: 7 };
        declined(wrapper.clone(), native.clone(), bad_reset);
    }
    assert_eq!(rows, 10, "row count moved");
}

/// A wrapper body whose call order, guard shape or writes are outside the
/// family's rules declines rather than being normalised into a reading.
#[test]
fn wrapper_bodies_outside_the_family_rules_decline() {
    let (wrapper, native, profile) = parts();
    let mut rows = 0;
    // Drop the terminal return of a path.
    let mut truncated = wrapper.clone();
    truncated.paths[0].operations.pop();
    assert!(compose(&truncated, &native, &profile).is_none());
    rows += 1;

    // Duplicate every path: no entry state can then select exactly one.
    let mut duplicated = wrapper.clone();
    duplicated.paths.extend(wrapper.paths.clone());
    assert!(compose(&duplicated, &native, &profile).is_none());
    rows += 1;

    // Keep only one path: the eight entry states cannot all be answered.
    let mut single = wrapper.clone();
    single.paths.truncate(1);
    assert!(compose(&single, &native, &profile).is_none());
    rows += 1;

    // A native body with no complete path leaves the composition unreadable.
    let mut hollow = native.clone();
    hollow.paths.truncate(1);
    assert!(compose(&wrapper, &hollow, &profile).is_none());
    rows += 1;
    assert_eq!(rows, 4, "row count moved");
}

/// The request gate: this producer answers only its own capability, profile,
/// dialect and projection, and declines everything else.
#[test]
fn the_request_gate_declines_every_other_exchange() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    let base =
        |profile: &str, capability: VerifierCapability, dialect: &str| VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: "composition".into(),
            plugin_id: manifest.plugin_id.clone(),
            capability,
            target: TargetProfile {
                library: "chunkio".into(),
                revision: "66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into(),
                profile: profile.into(),
                dialect: dialect.into(),
                platform: std::env::consts::OS.into(),
                backend: "native".into(),
            },
            unit: UnitIdentity {
                translation_unit: "cio_file_unix.c".into(),
                source_digest: "0".repeat(64),
                build_digest: "0".repeat(64),
            },
            facts: Vec::new(),
            assumptions: Default::default(),
            task_digest: "0".repeat(64),
            obligations: Vec::new(),
            source_effect: None,
            counter_effect: None,
            reactor_program: None,
            teardown_source: None,
            call_use_source: None,
            cleanup_replay_source: None,
            composition_source: Some(projection()),
        };

    // The honest shape is answered.
    let honest = base(
        UNMAP_WRAPPER_SEMANTIC_PROFILE,
        VerifierCapability::ResourceContracts,
        "c11",
    );
    let response = propose(&honest, &manifest).expect("the honest request is answered");
    assert!(matches!(
        response.evidence.unwrap().witness,
        SemanticWitness::Composition(_)
    ));

    let mut rows = 0;
    {
        let mut declines = |request: VerificationRequestV2| {
            assert!(propose(&request, &manifest).is_none());
            rows += 1;
        };
        declines(base(
            "qcore.c_cleanup_replay.v1",
            VerifierCapability::ResourceContracts,
            "c11",
        ));
        declines(base(
            UNMAP_WRAPPER_SEMANTIC_PROFILE,
            VerifierCapability::ChunkLifecycle,
            "c11",
        ));
        declines(base(
            UNMAP_WRAPPER_SEMANTIC_PROFILE,
            VerifierCapability::ResourceContracts,
            "qcore",
        ));
        let mut absent = honest.clone();
        absent.composition_source = None;
        declines(absent);
        let mut version = honest.clone();
        version.composition_source.as_mut().unwrap().schema_version = 2;
        declines(version);
        let mut incomplete = honest.clone();
        incomplete
            .composition_source
            .as_mut()
            .unwrap()
            .prerequisites
            .counter
            .task_digest
            .clear();
        declines(incomplete);
        let mut repeated = honest.clone();
        {
            let prerequisites = &mut repeated.composition_source.as_mut().unwrap().prerequisites;
            prerequisites.counter = prerequisites.native_cleanup.clone();
        }
        declines(repeated);
    }
    assert_eq!(rows, 7, "row count moved");
}
