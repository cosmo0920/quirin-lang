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

use super::*;
use qcore_core::chunk_lifecycle::ChunkOperation;
use qcore_schemas::profile::OperationKind;

use std::sync::LazyLock;

use qcore_contract::profile_loader::LoadedProfile;

use crate::chunk_lifecycle::{ChunkBackend, ChunkResidency};
use crate::event_driven::{
    ActivationReason, ArgumentMode, CallbackSummary, EventFlags, EventKey, PayloadAccessKind,
    ThreadMode,
};
use crate::source_profiles::{FLUENT_BIT_SOURCE_SET, LIBEVENT_SOURCE_SET, load_profile};

/// The declared source-set profiles (W2). The permitted mechanism sets, the
/// operation-summary table and the accepted-build matrix under test are the
/// shipped data, not permissive fixtures.
static FLUENT_BIT: LazyLock<LoadedProfile> =
    LazyLock::new(|| load_profile(FLUENT_BIT_SOURCE_SET).expect("fluent-bit profile loads"));
static LIBEVENT: LazyLock<LoadedProfile> =
    LazyLock::new(|| load_profile(LIBEVENT_SOURCE_SET).expect("libevent profile loads"));

fn library(name: &str) -> &'static qcore_schemas::profile::LibraryProfile {
    FLUENT_BIT
        .library(name)
        .expect("the fluent-bit profile declares this library")
}

fn permitted(name: &str) -> &'static BTreeSet<LibraryMechanismSummary> {
    &library(name).permitted_mechanisms
}

static CHUNK_SUMMARIES: LazyLock<BTreeMap<OperationKind, String>> = LazyLock::new(|| {
    FLUENT_BIT
        .merged_operation_summaries(&["chunkio", "fluent_bit"])
        .expect("the chunk operation table merges without conflict")
});

fn chunk_summaries() -> &'static BTreeMap<OperationKind, String> {
    &CHUNK_SUMMARIES
}

fn revision() -> &'static str {
    &FLUENT_BIT
        .pin_for("chunkio")
        .expect("chunkio binds a pin")
        .git_commit
}

fn builds() -> &'static [BuildProfileSelector] {
    &LIBEVENT
        .library(LIBEVENT_SOURCE_SET)
        .expect("the libevent profile declares its library")
        .accepted_builds
}

#[test]
fn queue_refcounts_and_synchronized_publication_discharge() {
    let program = LibraryCustodyProgram {
        evidence: BTreeMap::from([
            (1, SynchronizationEvidenceKind::AtomicPublication),
            (2, SynchronizationEvidenceKind::Reclamation),
            (3, SynchronizationEvidenceKind::ThreadHandoff),
        ]),
        operations: vec![
            LibraryCustodyOperation::CreateContainer {
                container: 10,
                kind: ExternalContainerKind::TypedQueue,
            },
            LibraryCustodyOperation::Acquire {
                resource: 20,
                holder: ExternalHolder::Local(1),
            },
            LibraryCustodyOperation::Retain {
                resource: 20,
                from: ExternalHolder::Local(1),
                retained: ExternalHolder::Thread(1),
            },
            LibraryCustodyOperation::Store {
                container: 10,
                resource: 20,
                from: ExternalHolder::Local(1),
            },
            LibraryCustodyOperation::Remove {
                container: 10,
                resource: 20,
                to: ExternalHolder::Local(1),
            },
            LibraryCustodyOperation::Publish {
                resource: 20,
                from: ExternalHolder::Local(1),
                publication: 7,
                evidence: 1,
            },
            LibraryCustodyOperation::Reclaim {
                resource: 20,
                publication: 7,
                to: 1,
                evidence: 2,
            },
            LibraryCustodyOperation::Release {
                resource: 20,
                holder: ExternalHolder::Local(1),
            },
            LibraryCustodyOperation::Handoff {
                resource: 20,
                from: 1,
                to: 2,
                evidence: 3,
            },
            LibraryCustodyOperation::Release {
                resource: 20,
                holder: ExternalHolder::Thread(2),
            },
            LibraryCustodyOperation::DestroyContainer {
                container: 10,
                destroy_members: false,
            },
        ],
    };
    let certificate = check_library_custody(&program).unwrap();
    assert_eq!(certificate.discharged_resources, BTreeSet::from([20]));
    assert_eq!(certificate.checked_evidence, BTreeSet::from([1, 2, 3]));

    let imported = ImportedLibraryCustodyProgram {
        profile: LibrarySummaryProfile {
            source_revision: revision().to_owned(),
            library: "librdkafka".to_owned(),
            summaries: BTreeSet::from([
                LibraryMechanismSummary::ReferenceCount,
                LibraryMechanismSummary::TypedQueue,
                LibraryMechanismSummary::AtomicPublication,
                LibraryMechanismSummary::ThreadHandoff,
                LibraryMechanismSummary::Reclamation,
            ]),
        },
        custody: program,
    };
    let imported_certificate =
        check_imported_library_custody(&imported, permitted("librdkafka"), revision()).unwrap();
    assert_eq!(imported_certificate.library, "librdkafka");

    let mut missing = imported.clone();
    missing
        .profile
        .summaries
        .remove(&LibraryMechanismSummary::TypedQueue);
    assert!(
        check_imported_library_custody(&missing, permitted("librdkafka"), revision())
            .unwrap_err()
            .contains(&LibraryImportDiagnostic::MissingSummary(
                LibraryMechanismSummary::TypedQueue
            ))
    );

    let mut wrong_library = imported;
    "cfl".clone_into(&mut wrong_library.profile.library);
    assert!(
        check_imported_library_custody(&wrong_library, permitted("cfl"), revision())
            .unwrap_err()
            .contains(&LibraryImportDiagnostic::UnsupportedSummary(
                LibraryMechanismSummary::TypedQueue
            ))
    );
}

fn chunk_operations() -> Vec<ChunkOperation> {
    vec![
        ChunkOperation::CreateContext { context: 1 },
        ChunkOperation::CreateStream {
            context: 1,
            stream: 2,
        },
        ChunkOperation::OpenChunk {
            context: 1,
            stream: 2,
            chunk: 3,
            backend: ChunkBackend::File,
            residency: ChunkResidency::Up,
        },
        ChunkOperation::SetResidency {
            chunk: 3,
            residency: ChunkResidency::Down,
        },
        ChunkOperation::DestroyContext { context: 1 },
    ]
}

#[test]
fn chunk_import_requires_pinned_source_complete_cfg_and_every_summary() {
    let operations = chunk_operations();
    let summaries = operations
        .iter()
        .map(|operation| {
            required_chunk_summary(operation, chunk_summaries())
                .expect("the profile models every operation in this program")
                .to_owned()
        })
        .collect();
    let program = ImportedChunkProgram {
        profile: ChunkImportProfile {
            source_revision: revision().to_owned(),
            control_flow_complete: true,
            summaries,
        },
        operations,
    };
    assert!(check_imported_chunk_program(&program, chunk_summaries(), revision()).is_ok());

    let mut incomplete = program.clone();
    incomplete.profile.control_flow_complete = false;
    assert!(
        check_imported_chunk_program(&incomplete, chunk_summaries(), revision())
            .unwrap_err()
            .contains(&ChunkImportDiagnostic::IncompleteControlFlow)
    );

    let mut missing = program;
    missing.profile.summaries.remove("cio_chunk_down");
    assert!(
        check_imported_chunk_program(&missing, chunk_summaries(), revision())
            .unwrap_err()
            .contains(&ChunkImportDiagnostic::MissingSummary(
                "cio_chunk_down".to_owned()
            ))
    );
}

fn monkey_events() -> EventProgram {
    let event = EventKey::new(1, 1);
    EventProgram {
        live_arguments: BTreeSet::from([9]),
        operations: vec![
            EventOperation::CreateBase {
                base: 1,
                profile: LibeventBuildProfile::LINUX_EPOLL,
            },
            EventOperation::Assign {
                event,
                base: 1,
                descriptor: Some(3),
                flags: EventFlags::new(EventFlags::READ),
                priority: 0,
                summary: CallbackSummary {
                    argument: 9,
                    argument_mode: ArgumentMode::Owned,
                    trigger_reasons: BTreeSet::from([ActivationReason::Read]),
                    payload_authority: BTreeMap::from([(7, PayloadAccessKind::Write)]),
                    may_defer: false,
                    may_spawn_tasks: false,
                },
            },
            EventOperation::Add { event },
            EventOperation::BeginDispatch {
                base: 1,
                dispatcher: 10,
            },
            EventOperation::Activate {
                event,
                reason: ActivationReason::Read,
                cross_thread: false,
            },
            EventOperation::CallbackEnter {
                event,
                reason: ActivationReason::Read,
            },
            EventOperation::CallbackExit { event },
            EventOperation::EndDispatch { base: 1 },
            EventOperation::FreeEvent { event },
            EventOperation::FreeBase { base: 1 },
        ],
    }
}

#[test]
fn monkey_composition_requires_callbacks_failure_paths_backend_and_observation() {
    let events = monkey_events();
    let event_certificate = check_event_program(&events, builds()).unwrap();
    let trace = event_certificate
        .trace
        .iter()
        .filter_map(|event| (event.kind != ObservedKind::Lifecycle).then_some(event.kind))
        .collect();
    let program = MonkeyCompositionProgram {
        profile: MonkeyCompositionProfile {
            source_revision: PINNED_FLUENT_BIT_REVISION.to_owned(),
            build: LibeventBuildProfile {
                backend: EventBackend::Epoll,
                threads: ThreadMode::Pthreads,
                openssl: false,
                debug_mode: true,
            },
            callbacks: [
                MonkeyCallback::Event,
                MonkeyCallback::Timeout,
                MonkeyCallback::WaitTwoTimeout,
            ]
            .into_iter()
            .map(|callback| (callback, MonkeyCallbackSummary::audited(callback).unwrap()))
            .collect(),
            event_map_custody: true,
            nullable_base_creation: true,
            nullable_event_creation: true,
            failed_registration: true,
        },
        events,
        observation: DispatcherObservation {
            dispatcher: 10,
            backend: EventBackend::Epoll,
            trace,
        },
    };
    assert!(check_monkey_composition(&program, builds()).is_ok());

    let mut wrong_source = program.clone();
    wrong_source.profile.source_revision = "unpinned".into();
    assert!(
        check_monkey_composition(&wrong_source, builds())
            .unwrap_err()
            .contains(&MonkeyCompositionDiagnostic::SourceRevisionMismatch)
    );

    let mut missing = program.clone();
    missing.profile.callbacks.remove(&MonkeyCallback::Timeout);
    assert!(
        check_monkey_composition(&missing, builds())
            .unwrap_err()
            .contains(&MonkeyCompositionDiagnostic::MissingCallback(
                MonkeyCallback::Timeout
            ))
    );

    let mut invalid = program.clone();
    invalid.profile.callbacks.insert(
        MonkeyCallback::Timeout,
        MonkeyCallbackSummary::audited(MonkeyCallback::Event).unwrap(),
    );
    assert!(
        check_monkey_composition(&invalid, builds())
            .unwrap_err()
            .contains(&MonkeyCompositionDiagnostic::InvalidCallbackSummary(
                MonkeyCallback::Timeout
            ))
    );

    let mut wrong_event_build = program.clone();
    let Some(EventOperation::CreateBase { profile, .. }) =
        wrong_event_build.events.operations.first_mut()
    else {
        panic!("fixture begins by creating the event base")
    };
    profile.backend = EventBackend::Poll;
    assert!(
        check_monkey_composition(&wrong_event_build, builds())
            .unwrap_err()
            .contains(&MonkeyCompositionDiagnostic::EventBuildMismatch)
    );

    let mut wrong_backend = program;
    wrong_backend.observation.backend = EventBackend::Poll;
    assert!(
        check_monkey_composition(&wrong_backend, builds())
            .unwrap_err()
            .contains(&MonkeyCompositionDiagnostic::BackendMismatch)
    );
}
