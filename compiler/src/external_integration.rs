//! Fail-closed composition for audited external-library custody and lifecycle models.

use std::collections::{BTreeMap, BTreeSet};

use crate::chunk_lifecycle::{
    ChunkCertificate, ChunkDiagnostic, ChunkOperation, ChunkProgram, check_chunk_program,
};
use crate::event_driven::{
    EventBackend, EventCertificate, EventDiagnostic, EventOperation, EventProgram,
    LibeventBuildProfile, ObservedKind, check_event_program, check_observation_refinement,
};

pub type ExternalResourceId = u32;
pub type ExternalContainerId = u32;
pub type ExternalContextId = u32;
pub type ExternalEvidenceId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalHolder {
    Local(ExternalContextId),
    Container(ExternalContainerId),
    Thread(ExternalContextId),
    Publication(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalContainerKind {
    IntrusiveList,
    TypedQueue,
    Arena,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynchronizationEvidenceKind {
    AtomicPublication,
    ThreadHandoff,
    Reclamation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryCustodyOperation {
    Acquire {
        resource: ExternalResourceId,
        holder: ExternalHolder,
    },
    Retain {
        resource: ExternalResourceId,
        from: ExternalHolder,
        retained: ExternalHolder,
    },
    Release {
        resource: ExternalResourceId,
        holder: ExternalHolder,
    },
    CreateContainer {
        container: ExternalContainerId,
        kind: ExternalContainerKind,
    },
    Store {
        container: ExternalContainerId,
        resource: ExternalResourceId,
        from: ExternalHolder,
    },
    Remove {
        container: ExternalContainerId,
        resource: ExternalResourceId,
        to: ExternalHolder,
    },
    DestroyContainer {
        container: ExternalContainerId,
        destroy_members: bool,
    },
    Publish {
        resource: ExternalResourceId,
        from: ExternalHolder,
        publication: u32,
        evidence: ExternalEvidenceId,
    },
    Handoff {
        resource: ExternalResourceId,
        from: ExternalContextId,
        to: ExternalContextId,
        evidence: ExternalEvidenceId,
    },
    Reclaim {
        resource: ExternalResourceId,
        publication: u32,
        to: ExternalContextId,
        evidence: ExternalEvidenceId,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryCustodyProgram {
    pub evidence: BTreeMap<ExternalEvidenceId, SynchronizationEvidenceKind>,
    pub operations: Vec<LibraryCustodyOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LibraryCustodyDiagnostic {
    DuplicateResource(ExternalResourceId),
    DuplicateContainer(ExternalContainerId),
    MissingResource(ExternalResourceId),
    MissingContainer(ExternalContainerId),
    InvalidHolder(ExternalResourceId),
    RefcountOverflow(ExternalResourceId),
    ArenaRemoval(ExternalContainerId),
    ContainerNotEmpty(ExternalContainerId),
    MissingSynchronization(ExternalEvidenceId),
    LiveResource(ExternalResourceId),
    LiveContainer(ExternalContainerId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryCustodyCertificate {
    pub discharged_resources: BTreeSet<ExternalResourceId>,
    pub destroyed_containers: BTreeSet<ExternalContainerId>,
    pub checked_evidence: BTreeSet<ExternalEvidenceId>,
}

pub fn check_library_custody(
    program: &LibraryCustodyProgram,
) -> Result<LibraryCustodyCertificate, Vec<LibraryCustodyDiagnostic>> {
    let mut resources = BTreeMap::<ExternalResourceId, BTreeMap<ExternalHolder, u32>>::new();
    let mut containers = BTreeMap::<ExternalContainerId, ExternalContainerKind>::new();
    let mut discharged = BTreeSet::new();
    let mut destroyed = BTreeSet::new();
    let mut checked_evidence = BTreeSet::new();
    let mut diagnostics = BTreeSet::new();

    for operation in &program.operations {
        match *operation {
            LibraryCustodyOperation::Acquire { resource, holder } => {
                if resources.contains_key(&resource) || discharged.contains(&resource) {
                    diagnostics.insert(LibraryCustodyDiagnostic::DuplicateResource(resource));
                } else if let ExternalHolder::Container(container) = holder
                    && !containers.contains_key(&container)
                {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                } else {
                    resources.insert(resource, BTreeMap::from([(holder, 1)]));
                }
            }
            LibraryCustodyOperation::Retain {
                resource,
                from,
                retained,
            } => match resources.get_mut(&resource) {
                Some(_)
                    if matches!(retained, ExternalHolder::Container(container)
                    if !containers.contains_key(&container)) =>
                {
                    let ExternalHolder::Container(container) = retained else {
                        unreachable!("guard establishes a container holder")
                    };
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                }
                Some(holders) if holders.contains_key(&from) => {
                    let count = holders.entry(retained).or_default();
                    match count.checked_add(1) {
                        Some(incremented) => *count = incremented,
                        None => {
                            diagnostics
                                .insert(LibraryCustodyDiagnostic::RefcountOverflow(resource));
                        }
                    }
                }
                Some(_) => {
                    diagnostics.insert(LibraryCustodyDiagnostic::InvalidHolder(resource));
                }
                None => {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingResource(resource));
                }
            },
            LibraryCustodyOperation::Release { resource, holder } => {
                remove_holder(
                    resource,
                    holder,
                    &mut resources,
                    &mut discharged,
                    &mut diagnostics,
                );
            }
            LibraryCustodyOperation::CreateContainer { container, kind } => {
                if containers.insert(container, kind).is_some() || destroyed.contains(&container) {
                    diagnostics.insert(LibraryCustodyDiagnostic::DuplicateContainer(container));
                }
            }
            LibraryCustodyOperation::Store {
                container,
                resource,
                from,
            } => {
                if !containers.contains_key(&container) {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                } else {
                    move_holder(
                        resource,
                        from,
                        ExternalHolder::Container(container),
                        &mut resources,
                        &mut diagnostics,
                    );
                }
            }
            LibraryCustodyOperation::Remove {
                container,
                resource,
                to,
            } => match containers.get(&container) {
                Some(ExternalContainerKind::Arena) => {
                    diagnostics.insert(LibraryCustodyDiagnostic::ArenaRemoval(container));
                }
                Some(_) => {
                    if let ExternalHolder::Container(target) = to
                        && !containers.contains_key(&target)
                    {
                        diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(target));
                    } else {
                        move_holder(
                            resource,
                            ExternalHolder::Container(container),
                            to,
                            &mut resources,
                            &mut diagnostics,
                        );
                    }
                }
                None => {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                }
            },
            LibraryCustodyOperation::DestroyContainer {
                container,
                destroy_members,
            } => {
                if !containers.contains_key(&container) {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                    continue;
                }
                let members = resources
                    .iter()
                    .filter_map(|(resource, holders)| {
                        holders
                            .contains_key(&ExternalHolder::Container(container))
                            .then_some(*resource)
                    })
                    .collect::<Vec<_>>();
                if !destroy_members && !members.is_empty() {
                    diagnostics.insert(LibraryCustodyDiagnostic::ContainerNotEmpty(container));
                    continue;
                }
                for resource in members {
                    remove_all_holder(
                        resource,
                        ExternalHolder::Container(container),
                        &mut resources,
                        &mut discharged,
                        &mut diagnostics,
                    );
                }
                containers.remove(&container);
                destroyed.insert(container);
            }
            LibraryCustodyOperation::Publish {
                resource,
                from,
                publication,
                evidence,
            } => {
                if require_evidence(
                    evidence,
                    SynchronizationEvidenceKind::AtomicPublication,
                    &program.evidence,
                    &mut checked_evidence,
                    &mut diagnostics,
                ) {
                    move_holder(
                        resource,
                        from,
                        ExternalHolder::Publication(publication),
                        &mut resources,
                        &mut diagnostics,
                    );
                }
            }
            LibraryCustodyOperation::Handoff {
                resource,
                from,
                to,
                evidence,
            } => {
                if require_evidence(
                    evidence,
                    SynchronizationEvidenceKind::ThreadHandoff,
                    &program.evidence,
                    &mut checked_evidence,
                    &mut diagnostics,
                ) {
                    move_holder(
                        resource,
                        ExternalHolder::Thread(from),
                        ExternalHolder::Thread(to),
                        &mut resources,
                        &mut diagnostics,
                    );
                }
            }
            LibraryCustodyOperation::Reclaim {
                resource,
                publication,
                to,
                evidence,
            } => {
                if require_evidence(
                    evidence,
                    SynchronizationEvidenceKind::Reclamation,
                    &program.evidence,
                    &mut checked_evidence,
                    &mut diagnostics,
                ) {
                    move_holder(
                        resource,
                        ExternalHolder::Publication(publication),
                        ExternalHolder::Local(to),
                        &mut resources,
                        &mut diagnostics,
                    );
                }
            }
        }
    }
    diagnostics.extend(
        resources
            .keys()
            .copied()
            .map(LibraryCustodyDiagnostic::LiveResource),
    );
    diagnostics.extend(
        containers
            .keys()
            .copied()
            .map(LibraryCustodyDiagnostic::LiveContainer),
    );
    if diagnostics.is_empty() {
        Ok(LibraryCustodyCertificate {
            discharged_resources: discharged,
            destroyed_containers: destroyed,
            checked_evidence,
        })
    } else {
        Err(diagnostics.into_iter().collect())
    }
}

fn move_holder(
    resource: ExternalResourceId,
    from: ExternalHolder,
    to: ExternalHolder,
    resources: &mut BTreeMap<ExternalResourceId, BTreeMap<ExternalHolder, u32>>,
    diagnostics: &mut BTreeSet<LibraryCustodyDiagnostic>,
) {
    match resources.get_mut(&resource) {
        Some(holders) if holders.contains_key(&from) => {
            if from == to {
                return;
            }
            let Some(destination) = holders.get(&to).copied().unwrap_or_default().checked_add(1)
            else {
                diagnostics.insert(LibraryCustodyDiagnostic::RefcountOverflow(resource));
                return;
            };
            decrement_holder(holders, from);
            holders.insert(to, destination);
        }
        Some(_) => {
            diagnostics.insert(LibraryCustodyDiagnostic::InvalidHolder(resource));
        }
        None => {
            diagnostics.insert(LibraryCustodyDiagnostic::MissingResource(resource));
        }
    }
}

fn remove_holder(
    resource: ExternalResourceId,
    holder: ExternalHolder,
    resources: &mut BTreeMap<ExternalResourceId, BTreeMap<ExternalHolder, u32>>,
    discharged: &mut BTreeSet<ExternalResourceId>,
    diagnostics: &mut BTreeSet<LibraryCustodyDiagnostic>,
) {
    match resources.get_mut(&resource) {
        Some(holders) => {
            if !holders.contains_key(&holder) {
                diagnostics.insert(LibraryCustodyDiagnostic::InvalidHolder(resource));
            } else {
                decrement_holder(holders, holder);
                if holders.is_empty() {
                    resources.remove(&resource);
                    discharged.insert(resource);
                }
            }
        }
        None => {
            diagnostics.insert(LibraryCustodyDiagnostic::MissingResource(resource));
        }
    }
}

fn remove_all_holder(
    resource: ExternalResourceId,
    holder: ExternalHolder,
    resources: &mut BTreeMap<ExternalResourceId, BTreeMap<ExternalHolder, u32>>,
    discharged: &mut BTreeSet<ExternalResourceId>,
    diagnostics: &mut BTreeSet<LibraryCustodyDiagnostic>,
) {
    match resources.get_mut(&resource) {
        Some(holders) => {
            if holders.remove(&holder).is_none() {
                diagnostics.insert(LibraryCustodyDiagnostic::InvalidHolder(resource));
            } else if holders.is_empty() {
                resources.remove(&resource);
                discharged.insert(resource);
            }
        }
        None => {
            diagnostics.insert(LibraryCustodyDiagnostic::MissingResource(resource));
        }
    }
}

fn decrement_holder(holders: &mut BTreeMap<ExternalHolder, u32>, holder: ExternalHolder) {
    let count = holders
        .get_mut(&holder)
        .expect("caller checked that the holder exists");
    *count -= 1;
    if *count == 0 {
        holders.remove(&holder);
    }
}

fn require_evidence(
    evidence: ExternalEvidenceId,
    expected: SynchronizationEvidenceKind,
    available: &BTreeMap<ExternalEvidenceId, SynchronizationEvidenceKind>,
    checked: &mut BTreeSet<ExternalEvidenceId>,
    diagnostics: &mut BTreeSet<LibraryCustodyDiagnostic>,
) -> bool {
    if available.get(&evidence) == Some(&expected) {
        checked.insert(evidence);
        true
    } else {
        diagnostics.insert(LibraryCustodyDiagnostic::MissingSynchronization(evidence));
        false
    }
}

pub const PINNED_FLUENT_BIT_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditedLibrary {
    Librdkafka2150,
    Cfl,
    ChunkIo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LibraryMechanismSummary {
    ReferenceCount,
    IntrusiveList,
    TypedQueue,
    Arena,
    CascadingDestructor,
    AtomicPublication,
    ThreadHandoff,
    Reclamation,
}

impl AuditedLibrary {
    pub fn permits(self, summary: LibraryMechanismSummary) -> bool {
        match self {
            Self::Librdkafka2150 => matches!(
                summary,
                LibraryMechanismSummary::ReferenceCount
                    | LibraryMechanismSummary::IntrusiveList
                    | LibraryMechanismSummary::TypedQueue
                    | LibraryMechanismSummary::CascadingDestructor
                    | LibraryMechanismSummary::AtomicPublication
                    | LibraryMechanismSummary::ThreadHandoff
                    | LibraryMechanismSummary::Reclamation
            ),
            Self::Cfl => matches!(
                summary,
                LibraryMechanismSummary::IntrusiveList
                    | LibraryMechanismSummary::Arena
                    | LibraryMechanismSummary::CascadingDestructor
                    | LibraryMechanismSummary::AtomicPublication
                    | LibraryMechanismSummary::ThreadHandoff
            ),
            Self::ChunkIo => matches!(
                summary,
                LibraryMechanismSummary::IntrusiveList
                    | LibraryMechanismSummary::CascadingDestructor
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySummaryProfile {
    pub source_revision: String,
    pub library: AuditedLibrary,
    pub summaries: BTreeSet<LibraryMechanismSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedLibraryCustodyProgram {
    pub profile: LibrarySummaryProfile,
    pub custody: LibraryCustodyProgram,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryImportCertificate {
    pub source_revision: String,
    pub library: AuditedLibrary,
    pub used_summaries: BTreeSet<LibraryMechanismSummary>,
    pub custody: LibraryCustodyCertificate,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LibraryImportDiagnostic {
    SourceRevisionMismatch,
    UnsupportedSummary(LibraryMechanismSummary),
    MissingSummary(LibraryMechanismSummary),
    Custody(Vec<LibraryCustodyDiagnostic>),
}

pub fn check_imported_library_custody(
    program: &ImportedLibraryCustodyProgram,
) -> Result<LibraryImportCertificate, Vec<LibraryImportDiagnostic>> {
    let mut diagnostics = BTreeSet::new();
    if program.profile.source_revision != PINNED_FLUENT_BIT_REVISION {
        diagnostics.insert(LibraryImportDiagnostic::SourceRevisionMismatch);
    }
    for summary in &program.profile.summaries {
        if !program.profile.library.permits(*summary) {
            diagnostics.insert(LibraryImportDiagnostic::UnsupportedSummary(*summary));
        }
    }

    let used_summaries = required_library_summaries(&program.custody);
    for summary in &used_summaries {
        if !program.profile.summaries.contains(summary) {
            diagnostics.insert(LibraryImportDiagnostic::MissingSummary(*summary));
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics.into_iter().collect());
    }

    let custody = check_library_custody(&program.custody)
        .map_err(|errors| vec![LibraryImportDiagnostic::Custody(errors)])?;
    Ok(LibraryImportCertificate {
        source_revision: program.profile.source_revision.clone(),
        library: program.profile.library,
        used_summaries,
        custody,
    })
}

fn required_library_summaries(
    program: &LibraryCustodyProgram,
) -> BTreeSet<LibraryMechanismSummary> {
    let mut summaries = BTreeSet::new();
    let mut containers = BTreeMap::new();
    for operation in &program.operations {
        match *operation {
            LibraryCustodyOperation::Retain { .. } => {
                summaries.insert(LibraryMechanismSummary::ReferenceCount);
            }
            LibraryCustodyOperation::CreateContainer { container, kind } => {
                containers.insert(container, kind);
                summaries.insert(container_summary(kind));
            }
            LibraryCustodyOperation::Store { container, .. }
            | LibraryCustodyOperation::Remove { container, .. } => {
                if let Some(kind) = containers.get(&container) {
                    summaries.insert(container_summary(*kind));
                }
            }
            LibraryCustodyOperation::DestroyContainer {
                container,
                destroy_members,
            } => {
                if let Some(kind) = containers.remove(&container) {
                    summaries.insert(container_summary(kind));
                }
                if destroy_members {
                    summaries.insert(LibraryMechanismSummary::CascadingDestructor);
                }
            }
            LibraryCustodyOperation::Publish { .. } => {
                summaries.insert(LibraryMechanismSummary::AtomicPublication);
            }
            LibraryCustodyOperation::Handoff { .. } => {
                summaries.insert(LibraryMechanismSummary::ThreadHandoff);
            }
            LibraryCustodyOperation::Reclaim { .. } => {
                summaries.insert(LibraryMechanismSummary::Reclamation);
            }
            LibraryCustodyOperation::Acquire { .. } | LibraryCustodyOperation::Release { .. } => {}
        }
    }
    summaries
}

const fn container_summary(kind: ExternalContainerKind) -> LibraryMechanismSummary {
    match kind {
        ExternalContainerKind::IntrusiveList => LibraryMechanismSummary::IntrusiveList,
        ExternalContainerKind::TypedQueue => LibraryMechanismSummary::TypedQueue,
        ExternalContainerKind::Arena => LibraryMechanismSummary::Arena,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkImportProfile {
    pub source_revision: String,
    pub control_flow_complete: bool,
    pub summaries: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedChunkProgram {
    pub profile: ChunkImportProfile,
    pub operations: Vec<ChunkOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkImportCertificate {
    pub source_revision: String,
    pub used_summaries: BTreeSet<String>,
    pub lifecycle: ChunkCertificate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkImportDiagnostic {
    SourceRevisionMismatch,
    IncompleteControlFlow,
    MissingSummary(&'static str),
    Lifecycle(Vec<ChunkDiagnostic>),
}

pub fn required_chunk_summary(operation: &ChunkOperation) -> &'static str {
    match operation {
        ChunkOperation::CreateContext { .. } => "cio_create",
        ChunkOperation::CreateStream { .. } => "cio_stream_create",
        ChunkOperation::OpenChunk { .. } | ChunkOperation::FailOpenChunk { .. } => "cio_chunk_open",
        ChunkOperation::SetResidency { residency, .. } => match residency {
            crate::chunk_lifecycle::ChunkResidency::Up => "cio_chunk_up",
            crate::chunk_lifecycle::ChunkResidency::Down => "cio_chunk_down",
        },
        ChunkOperation::Lock { .. } => "cio_chunk_lock",
        ChunkOperation::Unlock { .. } => "cio_chunk_unlock",
        ChunkOperation::SetBusy { .. } => "flb_input_chunk_busy_state",
        ChunkOperation::AttachTask { .. } | ChunkOperation::DetachTask { .. } => {
            "flb_input_chunk_task_custody"
        }
        ChunkOperation::AddRoute { .. } | ChunkOperation::RemoveRoute { .. } => {
            "flb_input_chunk_route_custody"
        }
        ChunkOperation::BeginTransaction { .. } => "cio_chunk_tx_begin",
        ChunkOperation::CommitTransaction { .. } => "cio_chunk_tx_commit",
        ChunkOperation::RollbackTransaction { .. } => "cio_chunk_tx_rollback",
        ChunkOperation::CloseChunk { .. } => "cio_chunk_close",
        ChunkOperation::DeleteStreamContent { .. } => "cio_stream_delete",
        ChunkOperation::DestroyStream { .. } => "cio_stream_destroy",
        ChunkOperation::DestroyContext { .. } => "cio_destroy",
    }
}

pub fn check_imported_chunk_program(
    program: &ImportedChunkProgram,
) -> Result<ChunkImportCertificate, Vec<ChunkImportDiagnostic>> {
    let mut diagnostics = Vec::new();
    if program.profile.source_revision != PINNED_FLUENT_BIT_REVISION {
        diagnostics.push(ChunkImportDiagnostic::SourceRevisionMismatch);
    }
    if !program.profile.control_flow_complete {
        diagnostics.push(ChunkImportDiagnostic::IncompleteControlFlow);
    }
    let used_summaries = program
        .operations
        .iter()
        .map(required_chunk_summary)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for summary in &used_summaries {
        if !program.profile.summaries.contains(summary) {
            let required = program
                .operations
                .iter()
                .map(required_chunk_summary)
                .find(|candidate| *candidate == summary)
                .expect("summary came from an operation");
            diagnostics.push(ChunkImportDiagnostic::MissingSummary(required));
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let lifecycle = check_chunk_program(&ChunkProgram {
        operations: program.operations.clone(),
    })
    .map_err(|errors| vec![ChunkImportDiagnostic::Lifecycle(errors)])?;
    Ok(ChunkImportCertificate {
        source_revision: program.profile.source_revision.clone(),
        used_summaries,
        lifecycle,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MonkeyCallback {
    Event,
    Timeout,
    WaitTwoTimeout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonkeyCallbackSummary {
    pub argument_borrowed: bool,
    pub writes_fired_array: bool,
    pub releases_event_map: bool,
    pub closes_descriptor: bool,
    pub no_effect: bool,
}

impl MonkeyCallbackSummary {
    pub const fn audited(callback: MonkeyCallback) -> Self {
        match callback {
            MonkeyCallback::Event => Self {
                argument_borrowed: true,
                writes_fired_array: true,
                releases_event_map: false,
                closes_descriptor: false,
                no_effect: false,
            },
            MonkeyCallback::Timeout => Self {
                argument_borrowed: true,
                writes_fired_array: false,
                releases_event_map: true,
                closes_descriptor: true,
                no_effect: false,
            },
            MonkeyCallback::WaitTwoTimeout => Self {
                argument_borrowed: true,
                writes_fired_array: false,
                releases_event_map: false,
                closes_descriptor: false,
                no_effect: true,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatcherObservation {
    pub dispatcher: ExternalContextId,
    pub backend: EventBackend,
    pub trace: Vec<ObservedKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyCompositionProfile {
    pub source_revision: String,
    pub build: LibeventBuildProfile,
    pub callbacks: BTreeMap<MonkeyCallback, MonkeyCallbackSummary>,
    pub event_map_custody: bool,
    pub nullable_base_creation: bool,
    pub nullable_event_creation: bool,
    pub failed_registration: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyCompositionProgram {
    pub profile: MonkeyCompositionProfile,
    pub events: EventProgram,
    pub observation: DispatcherObservation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonkeyCompositionCertificate {
    pub source_revision: String,
    pub event: EventCertificate,
    pub callbacks: BTreeMap<MonkeyCallback, MonkeyCallbackSummary>,
    pub dispatcher: ExternalContextId,
    pub backend: EventBackend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonkeyCompositionDiagnostic {
    SourceRevisionMismatch,
    UnknownBuildProfile,
    MissingCallback(MonkeyCallback),
    InvalidCallbackSummary(MonkeyCallback),
    MissingEventMapCustody,
    MissingNullableBasePath,
    MissingNullableEventPath,
    MissingRegistrationFailurePath,
    EventBuildMismatch,
    BackendMismatch,
    DispatcherMismatch,
    Event(Vec<EventDiagnostic>),
    Observation(EventDiagnostic),
}

pub fn check_monkey_composition(
    program: &MonkeyCompositionProgram,
) -> Result<MonkeyCompositionCertificate, Vec<MonkeyCompositionDiagnostic>> {
    let mut diagnostics = Vec::new();
    if program.profile.source_revision != PINNED_FLUENT_BIT_REVISION {
        diagnostics.push(MonkeyCompositionDiagnostic::SourceRevisionMismatch);
    }
    if !program.profile.build.accepted() {
        diagnostics.push(MonkeyCompositionDiagnostic::UnknownBuildProfile);
    }
    for callback in [
        MonkeyCallback::Event,
        MonkeyCallback::Timeout,
        MonkeyCallback::WaitTwoTimeout,
    ] {
        match program.profile.callbacks.get(&callback) {
            None => diagnostics.push(MonkeyCompositionDiagnostic::MissingCallback(callback)),
            Some(summary) if *summary != MonkeyCallbackSummary::audited(callback) => {
                diagnostics.push(MonkeyCompositionDiagnostic::InvalidCallbackSummary(
                    callback,
                ));
            }
            Some(_) => {}
        }
    }
    if !program.profile.event_map_custody {
        diagnostics.push(MonkeyCompositionDiagnostic::MissingEventMapCustody);
    }
    if !program.profile.nullable_base_creation {
        diagnostics.push(MonkeyCompositionDiagnostic::MissingNullableBasePath);
    }
    if !program.profile.nullable_event_creation {
        diagnostics.push(MonkeyCompositionDiagnostic::MissingNullableEventPath);
    }
    if !program.profile.failed_registration {
        diagnostics.push(MonkeyCompositionDiagnostic::MissingRegistrationFailurePath);
    }
    let created_bases = program
        .events
        .operations
        .iter()
        .filter_map(|operation| match operation {
            EventOperation::CreateBase { base, profile } => Some((*base, *profile)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let dispatched_bases = program
        .events
        .operations
        .iter()
        .filter_map(|operation| match operation {
            EventOperation::BeginDispatch { base, .. } => Some(*base),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if dispatched_bases.is_empty()
        || dispatched_bases
            .iter()
            .any(|base| created_bases.get(base) != Some(&program.profile.build))
    {
        diagnostics.push(MonkeyCompositionDiagnostic::EventBuildMismatch);
    }
    if program.profile.build.backend != program.observation.backend {
        diagnostics.push(MonkeyCompositionDiagnostic::BackendMismatch);
    }
    let dispatchers = program
        .events
        .operations
        .iter()
        .filter_map(|operation| match operation {
            EventOperation::BeginDispatch { dispatcher, .. } => Some(*dispatcher),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if dispatchers != BTreeSet::from([program.observation.dispatcher]) {
        diagnostics.push(MonkeyCompositionDiagnostic::DispatcherMismatch);
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let event = check_event_program(&program.events)
        .map_err(|errors| vec![MonkeyCompositionDiagnostic::Event(errors)])?;
    check_observation_refinement(&event, &program.observation.trace)
        .map_err(|error| vec![MonkeyCompositionDiagnostic::Observation(error)])?;
    Ok(MonkeyCompositionCertificate {
        source_revision: program.profile.source_revision.clone(),
        event,
        callbacks: program.profile.callbacks.clone(),
        dispatcher: program.observation.dispatcher,
        backend: program.observation.backend,
    })
}

#[cfg(test)]
#[path = "../tests/unit/external_integration.rs"]
mod tests;
