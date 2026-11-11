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

//! Generic declared-profile gates over custody and chunk lifecycle models.
//! Profile labels and evidence are explicit premises; these APIs do not authenticate
//! a checkout or establish native synchronization. Missing requirements reject.

use qcore_core::chunk_lifecycle::{
    ChunkCertificate, ChunkDiagnostic, ChunkOperation, ChunkProgram, check_chunk_program,
};
pub use qcore_core::library_custody::*;
use qcore_schemas::profile::OperationKind;
use std::collections::{BTreeMap, BTreeSet};

pub use qcore_schemas::mechanism::LibraryMechanismSummary;

/// W2 replaced the closed `AuditedLibrary` enum and its per-library `permits`
/// match arms with the `libraries` map of a source-set profile: the library is
/// now a name declared in data, and the permitted mechanism set is
/// `LibraryProfile::permitted_mechanisms`. A library with no profile entry has
/// no permitted set, so every summary is unsupported and the import stays
/// Unknown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySummaryProfile {
    pub source_revision: String,
    pub library: String,
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
    pub library: String,
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

/// `permitted` is the library's declared `permitted_mechanisms` set and
/// `expected_revision` its declared pin, both supplied by the loaded source-set
/// profile. A summary outside the permitted set, or a used summary the source
/// audit did not grant, remains fail-closed.
pub fn check_imported_library_custody(
    program: &ImportedLibraryCustodyProgram,
    permitted: &BTreeSet<LibraryMechanismSummary>,
    expected_revision: &str,
) -> Result<LibraryImportCertificate, Vec<LibraryImportDiagnostic>> {
    let mut diagnostics = BTreeSet::new();
    if program.profile.source_revision != expected_revision {
        diagnostics.insert(LibraryImportDiagnostic::SourceRevisionMismatch);
    }
    for summary in &program.profile.summaries {
        if !permitted.contains(summary) {
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
        library: program.profile.library.clone(),
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
    MissingSummary(String),
    /// The operation's kind has no `operation_summaries` entry in the loaded
    /// profile, so no required summary can be derived (design §3, F13). Absent
    /// is never "no requirement": the import stays Unknown.
    UnmodeledOperation(OperationKind),
    Lifecycle(Vec<ChunkDiagnostic>),
}

/// The abstract operation kind a concrete chunk operation belongs to. The
/// kind vocabulary is the published closed enum, so a profile cannot name an
/// operation the checker does not model.
#[must_use]
pub const fn chunk_operation_kind(operation: &ChunkOperation) -> OperationKind {
    match operation {
        ChunkOperation::CreateContext { .. } => OperationKind::CreateContext,
        ChunkOperation::CreateStream { .. } => OperationKind::CreateStream,
        ChunkOperation::OpenChunk { .. } | ChunkOperation::FailOpenChunk { .. } => {
            OperationKind::OpenChunk
        }
        ChunkOperation::SetResidency { residency, .. } => match residency {
            qcore_core::chunk_lifecycle::ChunkResidency::Up => OperationKind::ResidencyUp,
            qcore_core::chunk_lifecycle::ChunkResidency::Down => OperationKind::ResidencyDown,
        },
        ChunkOperation::Lock { .. } => OperationKind::Lock,
        ChunkOperation::Unlock { .. } => OperationKind::Unlock,
        ChunkOperation::SetBusy { .. } => OperationKind::SetBusy,
        ChunkOperation::AttachTask { .. } | ChunkOperation::DetachTask { .. } => {
            OperationKind::TaskCustody
        }
        ChunkOperation::AddRoute { .. } | ChunkOperation::RemoveRoute { .. } => {
            OperationKind::RouteCustody
        }
        ChunkOperation::BeginTransaction { .. } => OperationKind::BeginTransaction,
        ChunkOperation::CommitTransaction { .. } => OperationKind::CommitTransaction,
        ChunkOperation::RollbackTransaction { .. } => OperationKind::RollbackTransaction,
        ChunkOperation::CloseChunk { .. } => OperationKind::CloseChunk,
        ChunkOperation::DeleteStreamContent { .. } => OperationKind::DeleteStreamContent,
        ChunkOperation::DestroyStream { .. } => OperationKind::DestroyStream,
        ChunkOperation::DestroyContext { .. } => OperationKind::DestroyContext,
    }
}

/// The summary name an operation requires, looked up in the operation table a
/// source-set profile declares (W2 replaced the hardcoded name table). `None`
/// means the profile models no summary for that kind: fail-closed.
#[must_use]
pub fn required_chunk_summary<'table>(
    operation: &ChunkOperation,
    summaries: &'table BTreeMap<OperationKind, String>,
) -> Option<&'table str> {
    summaries
        .get(&chunk_operation_kind(operation))
        .map(String::as_str)
}

/// `summaries` is the operation table declared by the loaded source-set
/// profile (the ChunkIO and input-chunk entries, merged), `expected_revision`
/// the declared pin.
pub fn check_imported_chunk_program(
    program: &ImportedChunkProgram,
    summaries: &BTreeMap<OperationKind, String>,
    expected_revision: &str,
) -> Result<ChunkImportCertificate, Vec<ChunkImportDiagnostic>> {
    let mut diagnostics = Vec::new();
    if program.profile.source_revision != expected_revision {
        diagnostics.push(ChunkImportDiagnostic::SourceRevisionMismatch);
    }
    if !program.profile.control_flow_complete {
        diagnostics.push(ChunkImportDiagnostic::IncompleteControlFlow);
    }
    let mut used_summaries = BTreeSet::new();
    for operation in &program.operations {
        match required_chunk_summary(operation, summaries) {
            Some(summary) => {
                used_summaries.insert(summary.to_owned());
            }
            None => diagnostics.push(ChunkImportDiagnostic::UnmodeledOperation(
                chunk_operation_kind(operation),
            )),
        }
    }
    for summary in &used_summaries {
        if !program.profile.summaries.contains(summary) {
            diagnostics.push(ChunkImportDiagnostic::MissingSummary(summary.clone()));
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
