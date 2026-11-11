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

//! Provider-side matrix for the composition producer, driven over the **real**
//! projection the gateway emits for these bodies.
//!
//! Every refusal row is a *decline*: the process returns no proposal, the host
//! sees no witness, and the obligation stays Unknown. Nothing here evaluates a
//! contract or asserts anything about the OS.
use super::*;
use qcore_schemas::cleanup_replay_summary::{CleanupReplayPathRecord, CleanupResetRecord};
use qcore_schemas::composition_summary::{CompositionPrerequisiteSet, FileClosePrerequisites};

const PROJECTION: &str = include_str!("../../fixtures/unmap_wrapper_composition_projection.json");

fn projection() -> CompositionSourceInput {
    serde_json::from_str(PROJECTION).expect("the committed projection parses")
}

/// A syntactically complete lookup key. It names nothing this process can reach:
/// identities are the receiver's business, never a producer's.
fn identity(digest: char) -> qcore_schemas::call_use_summary::PrerequisiteIdentity {
    qcore_schemas::call_use_summary::PrerequisiteIdentity {
        semantic_profile: "qcore.example.v1".into(),
        task_digest: digest.to_string().repeat(64),
        obligation_id: format!("rc:{digest}"),
    }
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
        CompositionFamilyInput::FileClose { .. }
        | CompositionFamilyInput::MemoryClose { .. }
        | CompositionFamilyInput::FilesystemChunkClose { .. }
        | CompositionFamilyInput::UnsupportedClose { .. } => {
            panic!("the leaf fixture must carry the leaf family")
        }
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
        "../../../qcore-provider-chunkio/provider-template.json"
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
        version.composition_source.as_mut().unwrap().schema_version = 1;
        declines(version);
        let mut incomplete = honest.clone();
        {
            let CompositionPrerequisiteSet::UnmapWrapper(set) = &mut incomplete
                .composition_source
                .as_mut()
                .unwrap()
                .prerequisites
            else {
                panic!("the leaf fixture must carry the leaf set");
            };
            set.counter.task_digest.clear();
        }
        declines(incomplete);
        let mut repeated = honest.clone();
        {
            let CompositionPrerequisiteSet::UnmapWrapper(set) =
                &mut repeated.composition_source.as_mut().unwrap().prerequisites
            else {
                panic!("the leaf fixture must carry the leaf set");
            };
            set.counter = set.native_cleanup.clone();
        }
        declines(repeated);
        // A projection whose family and prerequisite set disagree is declined
        // before any replay: one family's set is never read as another's.
        let mut crossed = honest.clone();
        crossed.composition_source.as_mut().unwrap().prerequisites =
            CompositionPrerequisiteSet::FileClose(FileClosePrerequisites {
                wrapper_call_use: identity('a'),
                unmap_composition: identity('b'),
                native_cleanup: identity('c'),
                native_delete: None,
            });
        declines(crossed);
    }
    assert_eq!(rows, 8, "row count moved");
}

// ---------------------------------------------------------------------------
// The two-level `cio_file_close` producer (card P04-COMPOSE-A).
// ---------------------------------------------------------------------------

const FILE_CLOSE_PROJECTION: &str =
    include_str!("../../fixtures/file_close_composition_projection.json");
const FILE_CLOSE_DELETE_PROJECTION: &str =
    include_str!("../../fixtures/file_close_delete_composition_projection.json");

fn file_close_projection(deleting: bool) -> CompositionSourceInput {
    let text = if deleting {
        FILE_CLOSE_DELETE_PROJECTION
    } else {
        FILE_CLOSE_PROJECTION
    };
    serde_json::from_str(text).expect("the committed file_close projection parses")
}

type FileCloseParts = (
    CleanupReplayFunctionRecord,
    CompositionFamilyInput,
    CleanupReplayFunctionRecord,
    CleanupProfileRecord,
    Option<FileCloseDeleteInput>,
);

fn file_close_parts(deleting: bool) -> FileCloseParts {
    match file_close_projection(deleting).family {
        CompositionFamilyInput::FileClose {
            wrapper,
            unmap,
            close,
            close_profile,
            delete,
        } => (wrapper, *unmap, close, close_profile, delete.map(|d| *d)),
        CompositionFamilyInput::UnmapWrapper { .. }
        | CompositionFamilyInput::MemoryClose { .. }
        | CompositionFamilyInput::FilesystemChunkClose { .. }
        | CompositionFamilyInput::UnsupportedClose { .. } => {
            panic!("the file_close fixture must carry the file_close family")
        }
    }
}

/// The producer recomputes the whole **two-level** reading itself: the nested
/// composed reading, both cleanup readings and the wrapper replay, over the
/// entry domain the trusted composition enumerates.
#[test]
fn the_producer_recomputes_the_whole_two_level_reading() {
    for deleting in [false, true] {
        let (wrapper, unmap, close, profile, delete) = file_close_parts(deleting);
        let record = compose_file_close(&wrapper, &unmap, &close, &profile, delete.as_ref())
            .expect("the real projection composes");
        assert_eq!(record.function, "cio_file_close");
        assert_eq!(record.cases.len(), 43);
        assert_eq!(record.uncovered_paths, vec![1, 2]);
        assert_eq!(
            record
                .cases
                .iter()
                .filter(|case| case.backend_released)
                .count(),
            42
        );
        // The composed figure: every reached case names the nested composition's
        // own case, and its counter reading travels with it.
        assert!(
            record
                .cases
                .iter()
                .all(|case| case.unmap_case.is_some() == case.backend_present)
        );
        let decrements: std::collections::BTreeSet<_> = record
            .cases
            .iter()
            .map(|case| case.up_counter_decrements)
            .collect();
        assert_eq!(decrements, std::collections::BTreeSet::from([0, 1]));
        assert_eq!(record.deleting_cases.is_some(), deleting);
        if deleting {
            let cases = record.deleting_cases.as_ref().unwrap();
            assert_eq!(cases.len(), 30);
            assert_eq!(
                cases
                    .iter()
                    .filter(|case| case.deletion == DeletionOutcomeRecord::SyscallSucceeded)
                    .count(),
                8
            );
            assert_eq!(record.deleting_uncovered_paths.as_deref(), Some(&[][..]));
        }
    }
}

/// The nested half is genuinely recomputed, not received: a nested question the
/// leaf replay refuses makes the **whole** two-level reading decline. A producer
/// that had been handed the nested conclusion would not have to look at the
/// nested bodies at all, and could not notice.
#[test]
fn the_nested_reading_is_recomputed_not_received() {
    let (wrapper, unmap, close, profile, delete) = file_close_parts(true);
    assert!(compose_file_close(&wrapper, &unmap, &close, &profile, delete.as_ref()).is_some());
    let CompositionFamilyInput::UnmapWrapper {
        wrapper: nested_wrapper,
        native: nested_native,
        native_profile,
    } = &unmap
    else {
        panic!("the nested question must be a leaf composition")
    };
    // Break only the nested question's own bodies, one at a time. Nothing at
    // this level changes; the whole reading declines because the nested reading
    // does.
    for (name, mutated) in [
        ("nested wrapper truncated", {
            let mut body = nested_wrapper.clone();
            body.paths[0].operations.pop();
            CompositionFamilyInput::UnmapWrapper {
                wrapper: body,
                native: nested_native.clone(),
                native_profile: native_profile.clone(),
            }
        }),
        ("nested prerequisite truncated", {
            let mut body = nested_native.clone();
            body.paths[0].operations.pop();
            CompositionFamilyInput::UnmapWrapper {
                wrapper: nested_wrapper.clone(),
                native: body,
                native_profile: native_profile.clone(),
            }
        }),
    ] {
        assert!(
            compose_file_close(&wrapper, &mutated, &close, &profile, delete.as_ref()).is_none(),
            "`{name}` did not reach the composed reading"
        );
    }
}

/// A measured boundary, pinned rather than hidden (`P04-COMPOSE-A-CASE-INDEX`).
///
/// This level publishes each prerequisite's **case index** over a fixed sampled
/// prestate domain, not the prerequisite's selected source path. Permuting a
/// prerequisite body's paths therefore moves that prerequisite's own reading —
/// which its own task publishes and its own witness binds — while leaving *this*
/// reading identical. The permuted bytes are still refused end to end, by the
/// prerequisite task's own source binding, not by this figure; this row records
/// exactly which layer notices.
#[test]
fn permuting_a_prerequisite_bodys_paths_is_invisible_to_this_level() {
    let (wrapper, unmap, close, profile, delete) = file_close_parts(true);
    let honest = compose_file_close(&wrapper, &unmap, &close, &profile, delete.as_ref()).unwrap();
    let CompositionFamilyInput::UnmapWrapper {
        wrapper: nested_wrapper,
        native: nested_native,
        native_profile,
    } = &unmap
    else {
        panic!("the nested question must be a leaf composition")
    };
    // The nested prerequisite's paths reversed: the nested reading's own
    // `native_source_path` moves, but this level does not publish it.
    let mut reordered_native = nested_native.clone();
    reordered_native.paths.reverse();
    let nested_reading = compose_unmap_wrapper(nested_wrapper, nested_native, native_profile)
        .expect("the honest nested reading");
    let reordered_nested_reading =
        compose_unmap_wrapper(nested_wrapper, &reordered_native, native_profile)
            .expect("the reordered nested reading still composes");
    assert_ne!(
        nested_reading, reordered_nested_reading,
        "the nested reading is insensitive to its own prerequisite's paths"
    );
    let mutated = CompositionFamilyInput::UnmapWrapper {
        wrapper: nested_wrapper.clone(),
        native: reordered_native,
        native_profile: native_profile.clone(),
    };
    assert_eq!(
        compose_file_close(&wrapper, &mutated, &close, &profile, delete.as_ref()).as_ref(),
        Some(&honest),
        "this level unexpectedly publishes the nested prerequisite's selected path"
    );
    // The same for the close prerequisite's own body.
    let mut reordered_close = close.clone();
    reordered_close.paths.reverse();
    assert_eq!(
        compose_file_close(
            &wrapper,
            &unmap,
            &reordered_close,
            &profile,
            delete.as_ref()
        )
        .as_ref(),
        Some(&honest),
        "this level unexpectedly publishes the close prerequisite's selected path"
    );
}

/// Degenerate and hostile file_close projections decline without panicking, at
/// the nested level.
#[test]
fn degenerate_nested_questions_decline_bounded() {
    let (wrapper, unmap, close, profile, delete) = file_close_parts(true);
    let CompositionFamilyInput::UnmapWrapper {
        wrapper: nested_wrapper,
        native: nested_native,
        native_profile,
    } = &unmap
    else {
        panic!("the nested question must be a leaf composition")
    };
    let nested = |w: CleanupReplayFunctionRecord, n: CleanupReplayFunctionRecord| {
        CompositionFamilyInput::UnmapWrapper {
            wrapper: w,
            native: n,
            native_profile: native_profile.clone(),
        }
    };
    let mut rows = 0;
    let mut declined = |name: &str, family: CompositionFamilyInput| {
        assert!(
            compose_file_close(&wrapper, &family, &close, &profile, delete.as_ref()).is_none(),
            "`{name}` was composed"
        );
        rows += 1;
    };
    let mut empty = nested_wrapper.clone();
    empty.paths.clear();
    declined(
        "nested wrapper emptied",
        nested(empty, nested_native.clone()),
    );
    let mut huge = nested_wrapper.clone();
    huge.paths = std::iter::repeat_n(nested_wrapper.paths[0].clone(), MAX_PATHS + 1).collect();
    declined(
        "nested wrapper oversized",
        nested(huge, nested_native.clone()),
    );
    let mut long = nested_native.clone();
    long.paths[0].operations = std::iter::repeat_n(
        nested_native.paths[0].operations[0].clone(),
        MAX_OPERATIONS + 1,
    )
    .collect();
    declined(
        "nested prerequisite oversized",
        nested(nested_wrapper.clone(), long),
    );
    let mut anonymous = nested_wrapper.clone();
    anonymous.function = String::new();
    declined(
        "nested wrapper anonymous",
        nested(anonymous, nested_native.clone()),
    );
    let mut moved = nested_wrapper.clone();
    moved.source_revision = "0".repeat(40);
    declined(
        "nested wrapper revision moved",
        nested(moved, nested_native.clone()),
    );
    declined(
        "nested wrapper and prerequisite identical",
        nested(nested_wrapper.clone(), nested_wrapper.clone()),
    );
    // A nested question that is itself a file_close question: this vocabulary
    // nests exactly one level, and the producer says so.
    declined(
        "nested question is another file_close question",
        CompositionFamilyInput::FileClose {
            wrapper: wrapper.clone(),
            unmap: Box::new(unmap.clone()),
            close: close.clone(),
            close_profile: profile.clone(),
            delete: None,
        },
    );
    assert_eq!(rows, 7, "row count moved");
}

/// Degenerate and hostile file_close projections decline without panicking, at
/// the outer level.
#[test]
fn degenerate_file_close_questions_decline_bounded() {
    let (wrapper, unmap, close, profile, delete) = file_close_parts(true);
    let mut rows = 0;
    let mut declined = |name: &str,
                        w: CleanupReplayFunctionRecord,
                        c: CleanupReplayFunctionRecord,
                        p: CleanupProfileRecord,
                        d: Option<FileCloseDeleteInput>| {
        assert!(
            compose_file_close(&w, &unmap, &c, &p, d.as_ref()).is_none(),
            "`{name}` was composed"
        );
        rows += 1;
    };
    let mut empty = wrapper.clone();
    empty.paths.clear();
    declined(
        "wrapper emptied",
        empty,
        close.clone(),
        profile.clone(),
        delete.clone(),
    );
    let mut wrong = wrapper.clone();
    wrong.signature = "int (struct cio_chunk *)".into();
    declined(
        "wrapper signature",
        wrong,
        close.clone(),
        profile.clone(),
        delete.clone(),
    );
    let mut truncated = wrapper.clone();
    truncated.paths[0].operations.pop();
    declined(
        "wrapper path truncated",
        truncated,
        close.clone(),
        profile.clone(),
        delete.clone(),
    );
    let mut duplicated = wrapper.clone();
    duplicated.paths.extend(wrapper.paths.clone());
    declined(
        "wrapper paths duplicated",
        duplicated,
        close.clone(),
        profile.clone(),
        delete.clone(),
    );
    let mut single = wrapper.clone();
    single.paths.truncate(1);
    declined(
        "wrapper reduced to one path",
        single,
        close.clone(),
        profile.clone(),
        delete.clone(),
    );
    declined(
        "wrapper and close prerequisite identical",
        wrapper.clone(),
        wrapper.clone(),
        profile.clone(),
        delete.clone(),
    );
    let mut bad = profile.clone();
    bad.fields.clear();
    declined(
        "close profile emptied",
        wrapper.clone(),
        close.clone(),
        bad,
        delete.clone(),
    );
    let mut bad_delete = delete.clone().unwrap();
    bad_delete.mapping_field = "fd".into();
    declined(
        "delete eligibility field moved",
        wrapper.clone(),
        close.clone(),
        profile.clone(),
        Some(bad_delete),
    );
    let mut empty_delete = delete.clone().unwrap();
    empty_delete.body.paths.clear();
    declined(
        "delete body emptied",
        wrapper.clone(),
        close.clone(),
        profile.clone(),
        Some(empty_delete),
    );
    assert_eq!(rows, 9, "row count moved");
}

/// The request gate for the two `file_close` profiles: the transported family
/// and the declared profile must agree, so one family's question is never
/// answered under another's profile.
#[test]
fn the_file_close_request_gate_binds_family_to_profile() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    let request = |profile: &str, input: CompositionSourceInput| VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "composition".into(),
        plugin_id: manifest.plugin_id.clone(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into(),
            profile: profile.into(),
            dialect: "c11".into(),
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
        composition_source: Some(input),
    };
    // Each entry point is answered under its own profile.
    for (profile, deleting) in [
        (FILE_CLOSE_SEMANTIC_PROFILE, false),
        (FILE_CLOSE_DELETE_SEMANTIC_PROFILE, true),
    ] {
        let response = propose(
            &request(profile, file_close_projection(deleting)),
            &manifest,
        )
        .expect("the honest request is answered");
        let SemanticWitness::Composition(witness) = response.evidence.unwrap().witness else {
            panic!("wrong witness kind")
        };
        assert!(matches!(
            witness.summary,
            CompositionProposalRecord::FileClose(_)
        ));
    }
    // Every cross pairing declines.
    let mut rows = 0;
    for (profile, deleting) in [
        (FILE_CLOSE_SEMANTIC_PROFILE, true),
        (FILE_CLOSE_DELETE_SEMANTIC_PROFILE, false),
        (UNMAP_WRAPPER_SEMANTIC_PROFILE, false),
        (UNMAP_WRAPPER_SEMANTIC_PROFILE, true),
    ] {
        assert!(
            propose(
                &request(profile, file_close_projection(deleting)),
                &manifest
            )
            .is_none(),
            "a {profile} request answered a deleting={deleting} question"
        );
        rows += 1;
    }
    // And the leaf question under a file_close profile.
    assert!(
        propose(
            &request(FILE_CLOSE_SEMANTIC_PROFILE, projection()),
            &manifest
        )
        .is_none()
    );
    rows += 1;
    assert_eq!(rows, 5, "row count moved");
}

// ---------------------------------------------------------------------------
// The memory-close family (card `P04-COMPOSE-MEMCLOSE`).
// ---------------------------------------------------------------------------

/// The **real** projection the gateway emits for these bodies. A conformance
/// test asserts the committed fixture is still the live one, so this matrix
/// cannot drift onto a stale question.
const MEMORY_CLOSE_PROJECTION: &str =
    include_str!("../../fixtures/memory_close_composition_projection.json");

fn memory_close_projection() -> CompositionSourceInput {
    serde_json::from_str(MEMORY_CLOSE_PROJECTION).unwrap()
}

fn memory_close_parts() -> (
    CleanupReplayFunctionRecord,
    CleanupReplayFunctionRecord,
    DerivedUnitRecord,
    MemoryCloseCalleeRecord,
) {
    match memory_close_projection().family {
        CompositionFamilyInput::MemoryClose {
            wrapper,
            probe,
            derivation,
            callees,
        } => (wrapper, probe, derivation, callees),
        other => panic!("the memory-close fixture must carry the memory-close family: {other:?}"),
    }
}

fn memory_close_reading() -> Option<MemoryCloseCompositionRecord> {
    let (wrapper, probe, derivation, callees) = memory_close_parts();
    compose_memory_close(&wrapper, &probe, &derivation, &callees)
}

#[test]
fn the_producer_recomputes_the_whole_memory_close_reading() {
    let reading = memory_close_reading().expect("the honest question composes");
    assert_eq!(reading.function, "cio_chunk_close");
    assert_eq!(
        reading.memory_kind, 1,
        "read out of the probe body, not received"
    );
    assert_eq!(reading.cases.len(), 2, "the null and nonnull entry states");
    assert_eq!(reading.checked_paths.len(), 2);
    assert_eq!(reading.uncovered_paths.len(), 2);

    let null = &reading.cases[0];
    assert!(!null.nonnull);
    assert_eq!(null.calls, 0, "the null entry resolves no frame at all");
    assert!(!null.backend_discharged && !null.chunk_released && !null.counter_decremented);

    let memory = &reading.cases[1];
    assert!(memory.nonnull);
    assert_eq!(
        memory.calls, 7,
        "reset, backend, two unlinks, two frees, counter"
    );
    assert!(
        memory.backend_discharged
            && memory.primary_unlinked
            && memory.state_unlinked
            && memory.name_released
            && memory.chunk_released
            && memory.counter_decremented
    );
    // The two entries select different paths, so the reading is not constant.
    assert_ne!(null.source_path, memory.source_path);
    assert!(
        !reading
            .uncovered_paths
            .iter()
            .any(|path| reading.checked_paths.contains(path)),
        "an uncovered path is never also a checked one"
    );
}

/// The producer reads the backend discriminant out of the probe **body**. A
/// changed probe changes the reading rather than being ignored, which is what
/// makes the probe a question and not an answer.
#[test]
fn a_changed_probe_body_changes_the_reading_rather_than_being_ignored() {
    let (wrapper, mut probe, derivation, callees) = memory_close_parts();
    let honest = compose_memory_close(&wrapper, &probe, &derivation, &callees).unwrap();
    probe.paths[0].operations = vec![CleanupReplayOperationRecord::ReturnValue {
        value: CleanupReplayValueRecord {
            c_type: "int".into(),
            expression: CleanupReplayExpressionRecord::Integer {
                value: "7".to_owned(),
            },
        },
    }];
    // The wrapper's guard compares the stream kind against the probed value, so
    // a different discriminant selects no complete memory path: a decline.
    let changed = compose_memory_close(&wrapper, &probe, &derivation, &callees);
    assert!(
        changed.is_none() || changed.as_ref() != Some(&honest),
        "the probe body really decides the reading"
    );
}

/// Degenerate and hostile questions decline without panicking. Every row is a
/// *decline*: the process returns no proposal, the host sees no witness and the
/// obligation stays Unknown.
#[test]
fn degenerate_memory_close_questions_decline() {
    let (wrapper, probe, derivation, callees) = memory_close_parts();
    let empty_body = |function: &str, signature: &str| CleanupReplayFunctionRecord {
        function: function.into(),
        source_revision: wrapper.source_revision.clone(),
        signature: signature.into(),
        paths: Vec::new(),
        profile_assumptions: BTreeMap::new(),
    };
    type MemoryCloseRow = (
        &'static str,
        Box<dyn Fn() -> Option<MemoryCloseCompositionRecord>>,
    );
    let rows: Vec<MemoryCloseRow> = vec![
        ("wrapper emptied", {
            let (probe, derivation, callees) = (probe.clone(), derivation.clone(), callees.clone());
            let body = empty_body("cio_chunk_close", &wrapper.signature);
            Box::new(move || compose_memory_close(&body, &probe, &derivation, &callees))
        }),
        ("probe emptied", {
            let (wrapper, derivation, callees) =
                (wrapper.clone(), derivation.clone(), callees.clone());
            let body = empty_body("qcore_cio_memory_store_value", "int (void)");
            Box::new(move || compose_memory_close(&wrapper, &body, &derivation, &callees))
        }),
        ("wrapper wrongly signed", {
            let (probe, derivation, callees) = (probe.clone(), derivation.clone(), callees.clone());
            let mut body = wrapper.clone();
            body.signature = "int (struct cio_chunk *, int)".into();
            Box::new(move || compose_memory_close(&body, &probe, &derivation, &callees))
        }),
        ("probe wrongly signed", {
            let (wrapper, derivation, callees) =
                (wrapper.clone(), derivation.clone(), callees.clone());
            let mut body = probe.clone();
            body.signature = "long (void)".into();
            Box::new(move || compose_memory_close(&wrapper, &body, &derivation, &callees))
        }),
        ("wrapper and probe are one body", {
            let (derivation, callees) = (derivation.clone(), callees.clone());
            let body = wrapper.clone();
            Box::new(move || compose_memory_close(&body, &body.clone(), &derivation, &callees))
        }),
        ("the two bodies disagree on the revision", {
            let (wrapper, derivation, callees) =
                (wrapper.clone(), derivation.clone(), callees.clone());
            let mut body = probe.clone();
            body.source_revision = "0".repeat(40);
            Box::new(move || compose_memory_close(&wrapper, &body, &derivation, &callees))
        }),
        ("the derivation derives nothing", {
            let (wrapper, probe, callees) = (wrapper.clone(), probe.clone(), callees.clone());
            let mut record = derivation.clone();
            record.derived_source_digest = record.base_source_digest.clone();
            Box::new(move || compose_memory_close(&wrapper, &probe, &record, &callees))
        }),
        ("the derivation is incomplete", {
            let (wrapper, probe, callees) = (wrapper.clone(), probe.clone(), callees.clone());
            let mut record = derivation.clone();
            record.recipe = String::new();
            Box::new(move || compose_memory_close(&wrapper, &probe, &record, &callees))
        }),
        ("the recipe names another function than the probe", {
            let (wrapper, probe, callees) = (wrapper.clone(), probe.clone(), callees.clone());
            let mut record = derivation.clone();
            record.recipe_function = "other".into();
            Box::new(move || compose_memory_close(&wrapper, &probe, &record, &callees))
        }),
        ("two callees collapsed onto one frame", {
            let (wrapper, probe, derivation) = (wrapper.clone(), probe.clone(), derivation.clone());
            let mut record = callees.clone();
            record.unlink = record.backend.clone();
            Box::new(move || compose_memory_close(&wrapper, &probe, &derivation, &record))
        }),
        ("a callee identity emptied", {
            let (wrapper, probe, derivation) = (wrapper.clone(), probe.clone(), derivation.clone());
            let mut record = callees.clone();
            record.counter.function = String::new();
            Box::new(move || compose_memory_close(&wrapper, &probe, &derivation, &record))
        }),
        ("a callee renamed, so the wrapper resolves nothing", {
            let (wrapper, probe, derivation) = (wrapper.clone(), probe.clone(), derivation.clone());
            let mut record = callees.clone();
            record.backend.function = "other_close".into();
            Box::new(move || compose_memory_close(&wrapper, &probe, &derivation, &record))
        }),
        ("an oversized body", {
            let (probe, derivation, callees) = (probe.clone(), derivation.clone(), callees.clone());
            let mut body = wrapper.clone();
            let path = body.paths[0].clone();
            body.paths = std::iter::repeat_n(path, MAX_PATHS + 1).collect();
            Box::new(move || compose_memory_close(&body, &probe, &derivation, &callees))
        }),
        ("a truncated wrapper path", {
            let (probe, derivation, callees) = (probe.clone(), derivation.clone(), callees.clone());
            let mut body = wrapper.clone();
            for path in &mut body.paths {
                path.operations.truncate(1);
            }
            Box::new(move || compose_memory_close(&body, &probe, &derivation, &callees))
        }),
        ("duplicated wrapper paths, so an entry selects two", {
            let (probe, derivation, callees) = (probe.clone(), derivation.clone(), callees.clone());
            let mut body = wrapper.clone();
            body.paths.extend(body.paths.clone());
            Box::new(move || compose_memory_close(&body, &probe, &derivation, &callees))
        }),
    ];
    assert_eq!(rows.len(), 15, "fifteen degenerate memory-close rows");
    for (label, row) in rows {
        assert!(row().is_none(), "the `{label}` row was answered");
    }
}

/// The request gate: the memory-close question is answered only under its own
/// semantic profile, and never as another family's.
#[test]
fn the_memory_close_request_gate_is_exact() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    let base = |profile: &str,
                capability: VerifierCapability,
                dialect: &str,
                source: Option<CompositionSourceInput>| VerificationRequestV2 {
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
            translation_unit: "cio_chunk.c".into(),
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
        composition_source: source,
    };

    let honest = base(
        MEMORY_CLOSE_SEMANTIC_PROFILE,
        VerifierCapability::ResourceContracts,
        "c11",
        Some(memory_close_projection()),
    );
    let response = propose(&honest, &manifest).expect("the honest request is answered");
    assert!(matches!(
        response.evidence.unwrap().witness,
        SemanticWitness::Composition(_)
    ));

    let mut rows: Vec<(&str, VerificationRequestV2)> = vec![
        (
            "another profile",
            base(
                UNMAP_WRAPPER_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(memory_close_projection()),
            ),
        ),
        (
            "another family's question under this profile",
            base(
                MEMORY_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(projection()),
            ),
        ),
        (
            "this family's question under another family's profile",
            base(
                FILE_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(memory_close_projection()),
            ),
        ),
        (
            "no projection at all",
            base(
                MEMORY_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                None,
            ),
        ),
        (
            "another capability",
            base(
                MEMORY_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ObservationHooks,
                "c11",
                Some(memory_close_projection()),
            ),
        ),
        (
            "an unadmitted dialect",
            base(
                MEMORY_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c99",
                Some(memory_close_projection()),
            ),
        ),
    ];
    // Three rows that mutate the transported question itself.
    let mutated = |label: &'static str, change: fn(&mut CompositionSourceInput)| {
        let mut projection = memory_close_projection();
        change(&mut projection);
        (
            label,
            base(
                MEMORY_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(projection),
            ),
        )
    };
    rows.push(mutated("an unsupported version", |p| p.schema_version = 1));
    rows.push(mutated("an incomplete identity", |p| {
        if let CompositionPrerequisiteSet::MemoryClose(set) = &mut p.prerequisites {
            set.counter.task_digest = String::new();
        }
    }));
    rows.push(mutated("a repeated identity", |p| {
        if let CompositionPrerequisiteSet::MemoryClose(set) = &mut p.prerequisites {
            set.unlink = set.counter.clone();
        }
    }));
    assert_eq!(rows.len(), 9, "nine memory-close request-gate rows");
    for (label, request) in rows {
        assert!(
            propose(&request, &manifest).is_none(),
            "the `{label}` request was answered"
        );
    }
}

// ---------------------------------------------------------------------------
// The filesystem family (card `P04-COMPOSE-FS`).
// ---------------------------------------------------------------------------

/// The **real** projection the gateway emits for these bodies. A conformance
/// test asserts the committed fixture is still the live one, so this matrix
/// cannot drift onto a stale question.
const FS_CLOSE_PROJECTION: &str =
    include_str!("../../fixtures/filesystem_chunk_close_composition_projection.json");

fn fs_close_projection() -> CompositionSourceInput {
    serde_json::from_str(FS_CLOSE_PROJECTION).unwrap()
}

fn fs_close_parts() -> (
    CompositionFamilyInput,
    CompositionFamilyInput,
    CleanupReplayFunctionRecord,
    DerivedUnitRecord,
) {
    match fs_close_projection().family {
        CompositionFamilyInput::FilesystemChunkClose {
            memory,
            backend,
            probe,
            derivation,
        } => (*memory, *backend, probe, derivation),
        other => panic!("the filesystem fixture must carry the filesystem family: {other:?}"),
    }
}

fn fs_close_reading() -> Option<FilesystemChunkCloseCompositionRecord> {
    let (memory, backend, probe, derivation) = fs_close_parts();
    compose_filesystem_chunk_close(&memory, &backend, &probe, &derivation)
}

#[test]
fn the_producer_recomputes_the_whole_three_level_filesystem_reading() {
    let reading = fs_close_reading().expect("the honest question composes");
    assert_eq!(reading.function, "cio_chunk_close");
    assert_eq!(
        reading.filesystem_kind, 0,
        "read out of this family's own probe body, not received"
    );
    assert_eq!(
        reading.memory_kind, 1,
        "read out of the *nested* question's probe body, not received"
    );
    assert_ne!(reading.filesystem_kind, reading.memory_kind);
    assert_eq!(reading.cases.len(), 75);
    assert_eq!(reading.checked_paths.len(), 2);
    assert_eq!(reading.uncovered_paths.len(), 2);
    assert_eq!(reading.combined_uncovered_paths.len(), 1);

    // The two null entries resolve no frame and carry no backend case at all.
    let nulls: Vec<_> = reading
        .cases
        .iter()
        .filter(|case| !case.chunk_present)
        .collect();
    assert_eq!(nulls.len(), 2, "one null entry per delete flag");
    for null in &nulls {
        assert!(null.backend_case.is_none());
        assert!(!null.chunk_released && !null.backend_released && !null.path_released);
        assert_eq!(null.total_counter_decrements, 0);
        assert_eq!(null.up_counter_decrements, 0);
        assert_eq!(null.mapping, ResidualCustodyRecord::Absent);
        assert_eq!(null.descriptor, ResidualCustodyRecord::Absent);
        assert!(null.deletion.is_none());
    }
    // Every present entry names a backend case and releases the chunk.
    let present: Vec<_> = reading
        .cases
        .iter()
        .filter(|case| case.chunk_present)
        .collect();
    assert_eq!(present.len(), 73);
    for case in &present {
        assert!(case.backend_case.is_some());
        assert!(case.chunk_released && case.primary_unlinked && case.state_unlinked);
        assert_eq!(case.total_counter_decrements, 1);
        assert_eq!(case.deletion.is_some(), case.delete);
    }
    // The two entry states select different wrapper paths, so the reading is
    // not constant, and no uncovered path is also a checked one.
    assert_ne!(nulls[0].source_path, present[0].source_path);
    assert!(
        !reading
            .uncovered_paths
            .iter()
            .any(|path| reading.checked_paths.contains(path))
    );
    // The composed figures really come from the backend composition: at least
    // one entry carries unresolved OS custody and at least one does not.
    assert!(
        present
            .iter()
            .any(|case| case.mapping == ResidualCustodyRecord::Unresolved)
    );
    assert!(
        present
            .iter()
            .any(|case| case.mapping == ResidualCustodyRecord::Absent)
    );
}

/// The producer reads *both* discriminants out of *two different* probe bodies.
/// A changed probe changes the reading rather than being ignored, which is what
/// makes each probe a question and not an answer.
#[test]
fn a_changed_filesystem_probe_changes_the_reading_rather_than_being_ignored() {
    let (memory, backend, probe, derivation) = fs_close_parts();
    let honest = compose_filesystem_chunk_close(&memory, &backend, &probe, &derivation).unwrap();
    let mut changed_probe = probe.clone();
    changed_probe.paths[0].operations = vec![CleanupReplayOperationRecord::ReturnValue {
        value: CleanupReplayValueRecord {
            c_type: "int".into(),
            expression: CleanupReplayExpressionRecord::Integer {
                value: "1".to_owned(),
            },
        },
    }];
    // A filesystem discriminant equal to the memory one is refused outright:
    // the two arms would collapse.
    assert!(
        compose_filesystem_chunk_close(&memory, &backend, &changed_probe, &derivation).is_none(),
        "a colliding discriminant is a decline"
    );
    // A third value selects no complete filesystem path: also a decline.
    changed_probe.paths[0].operations = vec![CleanupReplayOperationRecord::ReturnValue {
        value: CleanupReplayValueRecord {
            c_type: "int".into(),
            expression: CleanupReplayExpressionRecord::Integer {
                value: "7".to_owned(),
            },
        },
    }];
    let third = compose_filesystem_chunk_close(&memory, &backend, &changed_probe, &derivation);
    assert!(
        third.is_none() || third.as_ref() != Some(&honest),
        "the probe body really decides the reading"
    );
}

/// Degenerate and hostile filesystem questions decline without panicking. Every
/// row is a *decline*: the process returns no proposal, the host sees no
/// witness, and the obligation stays Unknown.
#[test]
fn degenerate_filesystem_questions_decline() {
    let (memory, backend, probe, derivation) = fs_close_parts();
    type FsRow = (
        &'static str,
        Box<dyn Fn() -> Option<FilesystemChunkCloseCompositionRecord>>,
    );
    let compose = |memory: CompositionFamilyInput,
                   backend: CompositionFamilyInput,
                   probe: CleanupReplayFunctionRecord,
                   derivation: DerivedUnitRecord|
     -> Box<dyn Fn() -> Option<FilesystemChunkCloseCompositionRecord>> {
        Box::new(move || compose_filesystem_chunk_close(&memory, &backend, &probe, &derivation))
    };
    let mut rows: Vec<FsRow> = Vec::new();

    rows.push(("the probe emptied", {
        let mut body = probe.clone();
        body.paths.clear();
        compose(memory.clone(), backend.clone(), body, derivation.clone())
    }));
    rows.push(("the probe wrongly signed", {
        let mut body = probe.clone();
        body.signature = "void (void)".into();
        compose(memory.clone(), backend.clone(), body, derivation.clone())
    }));
    rows.push(("an oversized probe", {
        let mut body = probe.clone();
        body.paths = (0..(MAX_PATHS + 1))
            .map(|_| body.paths[0].clone())
            .collect();
        compose(memory.clone(), backend.clone(), body, derivation.clone())
    }));
    rows.push(("a derivation that derives nothing", {
        let mut record = derivation.clone();
        record.derived_source_digest = record.base_source_digest.clone();
        compose(memory.clone(), backend.clone(), probe.clone(), record)
    }));
    rows.push(("an incomplete derivation", {
        let mut record = derivation.clone();
        record.recipe = String::new();
        compose(memory.clone(), backend.clone(), probe.clone(), record)
    }));
    rows.push(("a derivation naming another function", {
        let mut record = derivation.clone();
        record.recipe_function = "qcore_cio_memory_store_value".into();
        compose(memory.clone(), backend.clone(), probe.clone(), record)
    }));
    rows.push(("the two derivations sharing a recipe", {
        let mut record = derivation.clone();
        record.recipe = memory
            .derivation()
            .expect("the nested question derives")
            .recipe
            .clone();
        compose(memory.clone(), backend.clone(), probe.clone(), record)
    }));
    rows.push(("the two derivations over different base texts", {
        let mut record = derivation.clone();
        record.base_source_digest = "f".repeat(64);
        compose(memory.clone(), backend.clone(), probe.clone(), record)
    }));
    rows.push(("the two derivations deriving one text", {
        let mut record = derivation.clone();
        record.derived_source_digest = memory
            .derivation()
            .expect("the nested question derives")
            .derived_source_digest
            .clone();
        compose(memory.clone(), backend.clone(), probe.clone(), record)
    }));
    rows.push(("the memory slot holding another family", {
        compose(
            backend.clone(),
            backend.clone(),
            probe.clone(),
            derivation.clone(),
        )
    }));
    rows.push(("the backend slot holding another family", {
        compose(
            memory.clone(),
            memory.clone(),
            probe.clone(),
            derivation.clone(),
        )
    }));
    rows.push(("the backend slot holding the non-deleting arm", {
        let mut stripped = backend.clone();
        if let CompositionFamilyInput::FileClose { delete, .. } = &mut stripped {
            *delete = None;
        }
        compose(memory.clone(), stripped, probe.clone(), derivation.clone())
    }));
    rows.push(("a memory wrapper with no complete path", {
        let mut broken = memory.clone();
        if let CompositionFamilyInput::MemoryClose { wrapper, .. } = &mut broken {
            wrapper.paths.clear();
        }
        compose(broken, backend.clone(), probe.clone(), derivation.clone())
    }));
    rows.push((
        "duplicated memory wrapper paths, so an entry selects two",
        {
            let mut duplicated = memory.clone();
            if let CompositionFamilyInput::MemoryClose { wrapper, .. } = &mut duplicated {
                wrapper.paths.extend(wrapper.paths.clone());
            }
            compose(
                duplicated,
                backend.clone(),
                probe.clone(),
                derivation.clone(),
            )
        },
    ));
    rows.push(("a memory question whose callees collapse", {
        let mut collapsed = memory.clone();
        if let CompositionFamilyInput::MemoryClose { callees, .. } = &mut collapsed {
            callees.unlink = callees.counter.clone();
        }
        compose(
            collapsed,
            backend.clone(),
            probe.clone(),
            derivation.clone(),
        )
    }));
    rows.push(("a backend wrapper with no complete path", {
        let mut broken = backend.clone();
        if let CompositionFamilyInput::FileClose { wrapper, .. } = &mut broken {
            wrapper.paths.clear();
        }
        compose(memory.clone(), broken, probe.clone(), derivation.clone())
    }));
    rows.push(("a backend delete body emptied", {
        let mut broken = backend.clone();
        if let CompositionFamilyInput::FileClose {
            delete: Some(delete),
            ..
        } = &mut broken
        {
            delete.body.paths.clear();
        }
        compose(memory.clone(), broken, probe.clone(), derivation.clone())
    }));
    rows.push(("a backend revision disagreement", {
        let mut broken = backend.clone();
        if let CompositionFamilyInput::FileClose { wrapper, .. } = &mut broken {
            wrapper.source_revision = "0".repeat(40);
        }
        compose(memory.clone(), broken, probe.clone(), derivation.clone())
    }));

    assert_eq!(rows.len(), 18, "eighteen degenerate filesystem rows");
    for (label, row) in rows {
        assert!(row().is_none(), "the `{label}` row was answered");
    }
}

/// The request gate: the filesystem question is answered only under its own
/// semantic profile, and never as another family's.
#[test]
fn the_filesystem_request_gate_is_exact() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    let base = |profile: &str,
                capability: VerifierCapability,
                dialect: &str,
                source: Option<CompositionSourceInput>| VerificationRequestV2 {
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
            translation_unit: "cio_chunk.c".into(),
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
        composition_source: source,
    };
    let mut rows = vec![
        (
            "another profile",
            base(
                MEMORY_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(fs_close_projection()),
            ),
        ),
        (
            "another family's question under this profile",
            base(
                FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(memory_close_projection()),
            ),
        ),
        (
            "no projection at all",
            base(
                FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                None,
            ),
        ),
        (
            "another capability",
            base(
                FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ObservationHooks,
                "c11",
                Some(fs_close_projection()),
            ),
        ),
        (
            "an unadmitted dialect",
            base(
                FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c99",
                Some(fs_close_projection()),
            ),
        ),
    ];
    let mutated = |label: &'static str, change: fn(&mut CompositionSourceInput)| {
        let mut projection = fs_close_projection();
        change(&mut projection);
        (
            label,
            base(
                FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(projection),
            ),
        )
    };
    rows.push(mutated("an unsupported version", |p| p.schema_version = 1));
    rows.push(mutated("an incomplete identity", |p| {
        if let CompositionPrerequisiteSet::FilesystemChunkClose(set) = &mut p.prerequisites {
            set.memory_close.task_digest = String::new();
        }
    }));
    rows.push(mutated("a repeated identity", |p| {
        if let CompositionPrerequisiteSet::FilesystemChunkClose(set) = &mut p.prerequisites {
            set.file_close_delete = set.memory_close.clone();
        }
    }));
    rows.push(mutated("another family's prerequisite set", |p| {
        p.prerequisites = CompositionPrerequisiteSet::FileClose(FileClosePrerequisites {
            wrapper_call_use: identity('a'),
            unmap_composition: identity('b'),
            native_cleanup: identity('c'),
            native_delete: Some(identity('d')),
        });
    }));
    assert_eq!(rows.len(), 9, "nine filesystem request-gate rows");
    for (label, request) in rows {
        assert!(
            propose(&request, &manifest).is_none(),
            "the `{label}` request was answered"
        );
    }
}

/// The honest filesystem question is answered, and the proposal is exactly the
/// reading the producer recomputed — never anything it was handed.
#[test]
fn the_honest_filesystem_question_is_answered_with_the_recomputed_reading() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    let projection = fs_close_projection();
    let request = VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "composition".into(),
        plugin_id: manifest.plugin_id.clone(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into(),
            profile: FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "cio_chunk.c".into(),
            source_digest: "0".repeat(64),
            build_digest: "0".repeat(64),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "9".repeat(64),
        obligations: Vec::new(),
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: Some(projection),
    };
    let response = propose(&request, &manifest).expect("the honest question is answered");
    let evidence = response.evidence.expect("a proposal carries evidence");
    let SemanticWitness::Composition(witness) = evidence.witness else {
        panic!("a composition witness");
    };
    assert_eq!(witness.task_digest, request.task_digest);
    let CompositionProposalRecord::FilesystemChunkClose(record) = witness.summary else {
        panic!("a filesystem-family proposal");
    };
    assert_eq!(Some(record), fs_close_reading());
}

// ---------------------------------------------------------------------------
// The symbolic other-backend family (card `P04-COMPOSE-UNSUPPORTED`).
// ---------------------------------------------------------------------------

/// The **real** projection the gateway emits for these bodies. A conformance
/// test asserts the committed fixture is still the live one, so this matrix
/// cannot drift onto a stale question.
const UNSUPPORTED_PROJECTION: &str =
    include_str!("../../fixtures/unsupported_close_composition_projection.json");

fn unsupported_projection() -> CompositionSourceInput {
    serde_json::from_str(UNSUPPORTED_PROJECTION).unwrap()
}

fn unsupported_nested() -> CompositionFamilyInput {
    match unsupported_projection().family {
        CompositionFamilyInput::UnsupportedClose { filesystem } => *filesystem,
        other => panic!("the symbolic fixture must carry the symbolic family: {other:?}"),
    }
}

fn unsupported_reading() -> Option<UnsupportedCloseCompositionRecord> {
    compose_unsupported_close(&unsupported_nested())
}

/// The producer recomputes the whole four-level reading itself, and only then
/// runs a fourth replay over a domain no transported value names.
#[test]
fn the_producer_recomputes_the_whole_symbolic_reading() {
    let reading = unsupported_reading().expect("the honest question composes");
    assert_eq!(reading.function, "cio_chunk_close");
    assert_eq!(reading.signature, "void (struct cio_chunk *, int)");
    assert_eq!(
        reading.memory_kind, 1,
        "read out of the nested memory question's probe, not received"
    );
    assert_eq!(
        reading.filesystem_kind, 0,
        "read out of the nested filesystem question's probe, not received"
    );
    assert_ne!(reading.memory_kind, reading.filesystem_kind);
    // The symbolic class selects exactly one wrapper path, and it is the path
    // the filesystem arm leaves combined-uncovered — which this process also
    // recomputed, so the two really agree.
    let CompositionFamilyInput::FilesystemChunkClose {
        memory,
        backend,
        probe,
        derivation,
    } = unsupported_nested()
    else {
        panic!("the nested question is the filesystem family");
    };
    let filesystem =
        compose_filesystem_chunk_close(&memory, &backend, &probe, &derivation).unwrap();
    assert_eq!(
        filesystem.combined_uncovered_paths,
        vec![reading.source_path]
    );
    assert!(!filesystem.checked_paths.contains(&reading.source_path));
}

/// The symbolic domain, on the producer side: a class this process cannot decide
/// is a decline, never a sampled representative.
#[test]
fn the_producer_declines_every_split_of_the_symbolic_domain() {
    let nested = unsupported_nested();
    let CompositionFamilyInput::FilesystemChunkClose {
        memory,
        backend,
        probe,
        derivation,
    } = nested.clone()
    else {
        panic!("the nested question is the filesystem family");
    };
    let CompositionFamilyInput::MemoryClose {
        wrapper, callees, ..
    } = &*memory
    else {
        panic!("the memory question");
    };
    let honest = unsupported_path(wrapper, callees, 1, 0, false).expect("the honest class reads");
    assert_eq!(
        unsupported_path(wrapper, callees, 1, 0, true),
        Some(honest),
        "the class's reading does not depend on the delete flag"
    );
    // Measured correction to this row's first draft, kept rather than weakened.
    // The excluded class is not a free parameter: it is fixed by the *guards in
    // the body*. This wrapper's guards compare the discriminant against `1` and
    // `0`, so a claimed class excluding 7 and 9 leaves `type == 1` undecided —
    // `1` is inside that class and outside it both — and the replay declines
    // instead of sampling. A producer therefore cannot widen or move the class
    // away from what the two probes and the two guards jointly fix.
    assert_eq!(
        unsupported_path(wrapper, callees, 7, 9, false),
        None,
        "a class the body's own guards do not decide is a decline"
    );
    // The same in the other direction: excluding only one of the two guarded
    // constants leaves the other undecided.
    assert_eq!(unsupported_path(wrapper, callees, 1, 9, false), None);
    assert_eq!(unsupported_path(wrapper, callees, 7, 0, false), None);
    // A collapsed class is a decline: the two constants must differ.
    assert!(compose_unsupported_close(&nested).is_some());
    let mut collapsed = probe.clone();
    collapsed.paths[0].operations = vec![CleanupReplayOperationRecord::ReturnValue {
        value: CleanupReplayValueRecord {
            c_type: "int".into(),
            expression: CleanupReplayExpressionRecord::Integer {
                value: "1".to_owned(),
            },
        },
    }];
    let collapsed_question = CompositionFamilyInput::UnsupportedClose {
        filesystem: Box::new(CompositionFamilyInput::FilesystemChunkClose {
            memory: memory.clone(),
            backend: backend.clone(),
            probe: collapsed,
            derivation: derivation.clone(),
        }),
    };
    assert!(
        compose_unsupported_close(&collapsed_question).is_none(),
        "a collapsed class is a decline"
    );
}

/// Degenerate and hostile symbolic questions decline without panicking. Every
/// row is a *decline*: the process returns no proposal, the host sees no
/// witness, and the obligation stays Unknown.
///
/// Twelve of the fifteen rows break the *nested* filesystem question rather than
/// this one, deliberately: the symbolic arm recomputes that whole reading before
/// asking its own question, so it inherits every refusal that matrix already
/// carries, and this test measures the inheritance rather than assuming it.
#[test]
fn degenerate_symbolic_questions_decline() {
    let nested = unsupported_nested();
    let CompositionFamilyInput::FilesystemChunkClose {
        memory,
        backend,
        probe,
        derivation,
    } = nested.clone()
    else {
        panic!("the nested question is the filesystem family");
    };
    let rebuild = |memory: Box<CompositionFamilyInput>,
                   backend: Box<CompositionFamilyInput>,
                   probe: CleanupReplayFunctionRecord,
                   derivation: DerivedUnitRecord| {
        CompositionFamilyInput::UnsupportedClose {
            filesystem: Box::new(CompositionFamilyInput::FilesystemChunkClose {
                memory,
                backend,
                probe,
                derivation,
            }),
        }
    };

    type Row = (
        &'static str,
        Box<dyn Fn() -> Option<UnsupportedCloseCompositionRecord>>,
    );
    let mut rows: Vec<Row> = Vec::new();

    // The nested question is not a filesystem question at all.
    for (label, substitute) in [
        ("the memory question in the nested slot", (*memory).clone()),
        (
            "the backend question in the nested slot",
            (*backend).clone(),
        ),
    ] {
        let question = CompositionFamilyInput::UnsupportedClose {
            filesystem: Box::new(substitute),
        };
        rows.push((
            label,
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    // The symbolic question nested inside itself.
    {
        let doubled = CompositionFamilyInput::UnsupportedClose {
            filesystem: Box::new(CompositionFamilyInput::UnsupportedClose {
                filesystem: Box::new(nested.clone()),
            }),
        };
        rows.push((
            "the symbolic question nested inside itself",
            Box::new(move || compose_unsupported_close(&doubled)),
        ));
    }
    // The nested filesystem question broken in each of the ways its own matrix
    // already refuses — the symbolic arm inherits every one of them, because it
    // recomputes that whole reading before asking its own question.
    {
        let mut broken = probe.clone();
        broken.paths.clear();
        let question = rebuild(memory.clone(), backend.clone(), broken, derivation.clone());
        rows.push((
            "an emptied filesystem probe",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken = probe.clone();
        broken.signature = "int (int)".into();
        let question = rebuild(memory.clone(), backend.clone(), broken, derivation.clone());
        rows.push((
            "a wrongly signed filesystem probe",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken = derivation.clone();
        broken.derived_source_digest = broken.base_source_digest.clone();
        let question = rebuild(memory.clone(), backend.clone(), probe.clone(), broken);
        rows.push((
            "a derivation that derives nothing",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken = derivation.clone();
        broken.recipe_function = "qcore_cio_other".into();
        let question = rebuild(memory.clone(), backend.clone(), probe.clone(), broken);
        rows.push((
            "a derivation naming another function",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken_memory = (*memory).clone();
        if let CompositionFamilyInput::MemoryClose { derivation: d, .. } = &mut broken_memory {
            d.recipe = derivation.recipe.clone();
        }
        let question = rebuild(
            Box::new(broken_memory),
            backend.clone(),
            probe.clone(),
            derivation.clone(),
        );
        rows.push((
            "the two derivations sharing a recipe",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken_memory = (*memory).clone();
        if let CompositionFamilyInput::MemoryClose { derivation: d, .. } = &mut broken_memory {
            d.base_source_digest = "0".repeat(64);
        }
        let question = rebuild(
            Box::new(broken_memory),
            backend.clone(),
            probe.clone(),
            derivation.clone(),
        );
        rows.push((
            "two derivations over different base texts",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken_memory = (*memory).clone();
        if let CompositionFamilyInput::MemoryClose { wrapper, .. } = &mut broken_memory {
            wrapper.paths.clear();
        }
        let question = rebuild(
            Box::new(broken_memory),
            backend.clone(),
            probe.clone(),
            derivation.clone(),
        );
        rows.push((
            "a memory wrapper with no complete path",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken_memory = (*memory).clone();
        if let CompositionFamilyInput::MemoryClose { wrapper, .. } = &mut broken_memory {
            let first = wrapper.paths[0].clone();
            wrapper.paths.push(first);
        }
        let question = rebuild(
            Box::new(broken_memory),
            backend.clone(),
            probe.clone(),
            derivation.clone(),
        );
        rows.push((
            "duplicated memory wrapper paths, so the class selects two",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken_memory = (*memory).clone();
        if let CompositionFamilyInput::MemoryClose { callees, .. } = &mut broken_memory {
            callees.unlink = callees.backend.clone();
        }
        let question = rebuild(
            Box::new(broken_memory),
            backend.clone(),
            probe.clone(),
            derivation.clone(),
        );
        rows.push((
            "a memory question whose callees collapse",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken_backend = (*backend).clone();
        if let CompositionFamilyInput::FileClose { delete, .. } = &mut broken_backend {
            *delete = None;
        }
        let question = rebuild(
            memory.clone(),
            Box::new(broken_backend),
            probe.clone(),
            derivation.clone(),
        );
        rows.push((
            "the non-deleting backend arm",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken_backend = (*backend).clone();
        if let CompositionFamilyInput::FileClose { wrapper, .. } = &mut broken_backend {
            wrapper.paths.clear();
        }
        let question = rebuild(
            memory.clone(),
            Box::new(broken_backend),
            probe.clone(),
            derivation.clone(),
        );
        rows.push((
            "a backend wrapper with no complete path",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }
    {
        let mut broken_backend = (*backend).clone();
        if let CompositionFamilyInput::FileClose { wrapper, .. } = &mut broken_backend {
            wrapper.source_revision = "0".repeat(40);
        }
        let question = rebuild(
            memory.clone(),
            Box::new(broken_backend),
            probe.clone(),
            derivation.clone(),
        );
        rows.push((
            "a backend revision disagreement",
            Box::new(move || compose_unsupported_close(&question)),
        ));
    }

    assert_eq!(rows.len(), 15, "fifteen degenerate symbolic rows");
    for (label, row) in rows {
        assert!(row().is_none(), "the `{label}` row was answered");
    }
}

/// The request gate: the symbolic question is answered only under its own
/// semantic profile, and never as another family's.
#[test]
fn the_symbolic_request_gate_is_exact() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    let base = |profile: &str,
                capability: VerifierCapability,
                dialect: &str,
                source: Option<CompositionSourceInput>| VerificationRequestV2 {
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
            translation_unit: "cio_chunk.c".into(),
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
        composition_source: source,
    };
    let mut rows = vec![
        (
            "another profile",
            base(
                FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(unsupported_projection()),
            ),
        ),
        (
            "the filesystem question under this profile",
            base(
                UNSUPPORTED_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(fs_close_projection()),
            ),
        ),
        (
            "the memory question under this profile",
            base(
                UNSUPPORTED_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(memory_close_projection()),
            ),
        ),
        (
            "no projection at all",
            base(
                UNSUPPORTED_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                None,
            ),
        ),
        (
            "another capability",
            base(
                UNSUPPORTED_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ObservationHooks,
                "c11",
                Some(unsupported_projection()),
            ),
        ),
        (
            "an unadmitted dialect",
            base(
                UNSUPPORTED_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c99",
                Some(unsupported_projection()),
            ),
        ),
    ];
    let mutated = |label: &'static str, change: fn(&mut CompositionSourceInput)| {
        let mut projection = unsupported_projection();
        change(&mut projection);
        (
            label,
            base(
                UNSUPPORTED_CLOSE_SEMANTIC_PROFILE,
                VerifierCapability::ResourceContracts,
                "c11",
                Some(projection),
            ),
        )
    };
    rows.push(mutated("an unsupported version", |p| p.schema_version = 1));
    rows.push(mutated("a future version", |p| p.schema_version = 3));
    rows.push(mutated("an incomplete identity", |p| {
        if let CompositionPrerequisiteSet::UnsupportedClose(set) = &mut p.prerequisites {
            set.filesystem_chunk_close.task_digest = String::new();
        }
    }));
    rows.push(mutated("another family's prerequisite set", |p| {
        p.prerequisites = CompositionPrerequisiteSet::FileClose(FileClosePrerequisites {
            wrapper_call_use: identity('a'),
            unmap_composition: identity('b'),
            native_cleanup: identity('c'),
            native_delete: Some(identity('d')),
        });
    }));
    rows.push(mutated("the filesystem family's prerequisite set", |p| {
        p.prerequisites = CompositionPrerequisiteSet::FilesystemChunkClose(
            qcore_schemas::composition_summary::FilesystemChunkClosePrerequisites {
                memory_close: identity('a'),
                file_close_delete: identity('b'),
            },
        );
    }));
    assert_eq!(rows.len(), 11, "eleven symbolic request-gate rows");
    for (label, request) in rows {
        assert!(
            propose(&request, &manifest).is_none(),
            "the `{label}` request was answered"
        );
    }
}

/// The honest symbolic question is answered, and the proposal is exactly the
/// reading the producer recomputed — never anything it was handed. The proposal
/// carries no conclusion at all: the arm's `Unknown` is the receiver's, and this
/// record has no field that could express otherwise.
#[test]
fn the_honest_symbolic_question_is_answered_with_the_recomputed_reading() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../../qcore-provider-chunkio/provider-template.json"
    ))
    .unwrap();
    let projection = unsupported_projection();
    let request = VerificationRequestV2 {
        protocol_version: 1,
        evidence_version: 2,
        request_id: "composition".into(),
        plugin_id: manifest.plugin_id.clone(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "chunkio".into(),
            revision: "66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into(),
            profile: UNSUPPORTED_CLOSE_SEMANTIC_PROFILE.into(),
            dialect: "c11".into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            translation_unit: "cio_chunk.c".into(),
            source_digest: "0".repeat(64),
            build_digest: "0".repeat(64),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "9".repeat(64),
        obligations: Vec::new(),
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: Some(projection),
    };
    let response = propose(&request, &manifest).expect("the honest question is answered");
    let evidence = response.evidence.expect("a proposal carries evidence");
    let SemanticWitness::Composition(witness) = evidence.witness else {
        panic!("a composition witness");
    };
    assert_eq!(witness.task_digest, request.task_digest);
    let CompositionProposalRecord::UnsupportedClose(record) = witness.summary else {
        panic!("a symbolic-family proposal");
    };
    assert_eq!(Some(record.clone()), unsupported_reading());
    let serialized = serde_json::to_string(&record).unwrap();
    for absent in ["status", "verdict", "discharge", "unresolved", "reason"] {
        assert!(
            !serialized.contains(absent),
            "the proposal carries `{absent}`"
        );
    }
}
