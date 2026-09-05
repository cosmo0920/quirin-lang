use super::*;
use crate::chunk_lifecycle::{ChunkBackend, ChunkResidency};
use crate::event_driven::{
    ActivationReason, ArgumentMode, CallbackSummary, EventFlags, EventKey, PayloadAccessKind,
    ThreadMode,
};

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
            source_revision: PINNED_FLUENT_BIT_REVISION.to_owned(),
            library: AuditedLibrary::Librdkafka2150,
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
    let imported_certificate = check_imported_library_custody(&imported).unwrap();
    assert_eq!(imported_certificate.library, AuditedLibrary::Librdkafka2150);

    let mut missing = imported.clone();
    missing
        .profile
        .summaries
        .remove(&LibraryMechanismSummary::TypedQueue);
    assert!(
        check_imported_library_custody(&missing)
            .unwrap_err()
            .contains(&LibraryImportDiagnostic::MissingSummary(
                LibraryMechanismSummary::TypedQueue
            ))
    );

    let mut wrong_library = imported;
    wrong_library.profile.library = AuditedLibrary::Cfl;
    assert!(
        check_imported_library_custody(&wrong_library)
            .unwrap_err()
            .contains(&LibraryImportDiagnostic::UnsupportedSummary(
                LibraryMechanismSummary::TypedQueue
            ))
    );
}

#[test]
fn repeated_retains_by_one_holder_are_counted() {
    let program = LibraryCustodyProgram {
        operations: vec![
            LibraryCustodyOperation::Acquire {
                resource: 1,
                holder: ExternalHolder::Local(1),
            },
            LibraryCustodyOperation::Retain {
                resource: 1,
                from: ExternalHolder::Local(1),
                retained: ExternalHolder::Local(1),
            },
            LibraryCustodyOperation::Release {
                resource: 1,
                holder: ExternalHolder::Local(1),
            },
            LibraryCustodyOperation::Release {
                resource: 1,
                holder: ExternalHolder::Local(1),
            },
        ],
        ..LibraryCustodyProgram::default()
    };
    assert!(check_library_custody(&program).is_ok());

    let mut leaked = program;
    leaked.operations.pop();
    assert_eq!(
        check_library_custody(&leaked).unwrap_err(),
        vec![LibraryCustodyDiagnostic::LiveResource(1)]
    );
}

#[test]
fn arena_destructor_cascades_but_queue_and_publication_fail_closed() {
    let arena = LibraryCustodyProgram {
        operations: vec![
            LibraryCustodyOperation::CreateContainer {
                container: 1,
                kind: ExternalContainerKind::Arena,
            },
            LibraryCustodyOperation::Acquire {
                resource: 2,
                holder: ExternalHolder::Local(0),
            },
            LibraryCustodyOperation::Retain {
                resource: 2,
                from: ExternalHolder::Local(0),
                retained: ExternalHolder::Local(0),
            },
            LibraryCustodyOperation::Store {
                container: 1,
                resource: 2,
                from: ExternalHolder::Local(0),
            },
            LibraryCustodyOperation::Store {
                container: 1,
                resource: 2,
                from: ExternalHolder::Local(0),
            },
            LibraryCustodyOperation::DestroyContainer {
                container: 1,
                destroy_members: true,
            },
        ],
        ..LibraryCustodyProgram::default()
    };
    assert!(check_library_custody(&arena).is_ok());

    let mut missing_evidence = arena;
    missing_evidence.operations.insert(
        2,
        LibraryCustodyOperation::Publish {
            resource: 2,
            from: ExternalHolder::Local(0),
            publication: 1,
            evidence: 99,
        },
    );
    assert!(
        check_library_custody(&missing_evidence)
            .unwrap_err()
            .contains(&LibraryCustodyDiagnostic::MissingSynchronization(99))
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
        .map(required_chunk_summary)
        .map(str::to_owned)
        .collect();
    let program = ImportedChunkProgram {
        profile: ChunkImportProfile {
            source_revision: PINNED_FLUENT_BIT_REVISION.to_owned(),
            control_flow_complete: true,
            summaries,
        },
        operations,
    };
    assert!(check_imported_chunk_program(&program).is_ok());

    let mut incomplete = program.clone();
    incomplete.profile.control_flow_complete = false;
    assert!(
        check_imported_chunk_program(&incomplete)
            .unwrap_err()
            .contains(&ChunkImportDiagnostic::IncompleteControlFlow)
    );

    let mut missing = program;
    missing.profile.summaries.remove("cio_chunk_down");
    assert!(
        check_imported_chunk_program(&missing)
            .unwrap_err()
            .contains(&ChunkImportDiagnostic::MissingSummary("cio_chunk_down"))
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
    let event_certificate = check_event_program(&events).unwrap();
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
            .map(|callback| (callback, MonkeyCallbackSummary::audited(callback)))
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
    assert!(check_monkey_composition(&program).is_ok());

    let mut wrong_source = program.clone();
    wrong_source.profile.source_revision = "unpinned".into();
    assert!(
        check_monkey_composition(&wrong_source)
            .unwrap_err()
            .contains(&MonkeyCompositionDiagnostic::SourceRevisionMismatch)
    );

    let mut missing = program.clone();
    missing.profile.callbacks.remove(&MonkeyCallback::Timeout);
    assert!(check_monkey_composition(&missing).unwrap_err().contains(
        &MonkeyCompositionDiagnostic::MissingCallback(MonkeyCallback::Timeout)
    ));

    let mut invalid = program.clone();
    invalid.profile.callbacks.insert(
        MonkeyCallback::Timeout,
        MonkeyCallbackSummary::audited(MonkeyCallback::Event),
    );
    assert!(check_monkey_composition(&invalid).unwrap_err().contains(
        &MonkeyCompositionDiagnostic::InvalidCallbackSummary(MonkeyCallback::Timeout)
    ));

    let mut wrong_event_build = program.clone();
    let Some(EventOperation::CreateBase { profile, .. }) =
        wrong_event_build.events.operations.first_mut()
    else {
        panic!("fixture begins by creating the event base")
    };
    profile.backend = EventBackend::Poll;
    assert!(
        check_monkey_composition(&wrong_event_build)
            .unwrap_err()
            .contains(&MonkeyCompositionDiagnostic::EventBuildMismatch)
    );

    let mut wrong_backend = program;
    wrong_backend.observation.backend = EventBackend::Poll;
    assert!(
        check_monkey_composition(&wrong_backend)
            .unwrap_err()
            .contains(&MonkeyCompositionDiagnostic::BackendMismatch)
    );
}
