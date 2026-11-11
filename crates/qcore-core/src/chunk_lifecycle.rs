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

//! Cycle 19 bounded ChunkIO and Fluent Bit input-chunk lifecycle model.
//!
//! The checker models the concrete nested teardown chain rather than treating
//! every function containing "delete" or "close" as an interchangeable
//! destructor.  A certificate is issued only when contexts, streams, chunks,
//! backend resources, list links, counters, transactions, routes, and tasks
//! are all discharged.

use std::collections::{BTreeMap, BTreeSet};

pub type ChunkContextId = u32;
pub type ChunkStreamId = u32;
pub type ChunkId = u32;
pub type ChunkTaskId = u32;
pub type ChunkRouteId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkBackend {
    Memory,
    File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkResidency {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkOpenFailure {
    ChunkAllocation,
    NameAllocation,
    BackendOpen,
    Registration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackingDeleteOutcome {
    NotRequested,
    Requested,
    Deleted,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkOperation {
    CreateContext {
        context: ChunkContextId,
    },
    CreateStream {
        context: ChunkContextId,
        stream: ChunkStreamId,
    },
    OpenChunk {
        context: ChunkContextId,
        stream: ChunkStreamId,
        chunk: ChunkId,
        backend: ChunkBackend,
        residency: ChunkResidency,
    },
    FailOpenChunk {
        context: ChunkContextId,
        stream: ChunkStreamId,
        chunk: ChunkId,
        failure: ChunkOpenFailure,
    },
    SetResidency {
        chunk: ChunkId,
        residency: ChunkResidency,
    },
    Lock {
        chunk: ChunkId,
    },
    Unlock {
        chunk: ChunkId,
    },
    SetBusy {
        chunk: ChunkId,
        busy: bool,
    },
    AttachTask {
        chunk: ChunkId,
        task: ChunkTaskId,
    },
    DetachTask {
        chunk: ChunkId,
        task: ChunkTaskId,
    },
    AddRoute {
        chunk: ChunkId,
        route: ChunkRouteId,
    },
    RemoveRoute {
        chunk: ChunkId,
        route: ChunkRouteId,
    },
    BeginTransaction {
        chunk: ChunkId,
    },
    CommitTransaction {
        chunk: ChunkId,
    },
    RollbackTransaction {
        chunk: ChunkId,
    },
    CloseChunk {
        chunk: ChunkId,
        delete: bool,
    },
    /// Mirrors `cio_stream_delete`: chunks and backing content are removed,
    /// but the stream object and its context membership remain live.
    DeleteStreamContent {
        stream: ChunkStreamId,
        filesystem_delete_succeeds: bool,
    },
    DestroyStream {
        stream: ChunkStreamId,
    },
    DestroyContext {
        context: ChunkContextId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkProgram {
    pub operations: Vec<ChunkOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChunkTraceEvent {
    ContextCreated(ChunkContextId),
    StreamCreated(ChunkStreamId),
    ChunkOpened(ChunkId),
    ChunkOpenFailed(ChunkId, ChunkOpenFailure),
    ResidencyChanged(ChunkId, ChunkResidency),
    Locked(ChunkId),
    Unlocked(ChunkId),
    BusyChanged(ChunkId, bool),
    TaskAttached(ChunkId, ChunkTaskId),
    TaskDetached(ChunkId, ChunkTaskId),
    RouteAdded(ChunkId, ChunkRouteId),
    RouteRemoved(ChunkId, ChunkRouteId),
    TransactionBegan(ChunkId),
    TransactionCommitted(ChunkId),
    TransactionRolledBack(ChunkId),
    BackendClosed(ChunkId, ChunkBackend),
    BackingDelete(ChunkId, BackingDeleteOutcome),
    ChunkUnlinked(ChunkId),
    ChunkCounterDecremented(ChunkContextId),
    ChunkClosed(ChunkId),
    StreamContentDeleted(ChunkStreamId, BackingDeleteOutcome),
    StreamDestroyed(ChunkStreamId),
    ContextDestroyed(ChunkContextId),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChunkDiagnostic {
    DuplicateContext(ChunkContextId),
    DuplicateStream(ChunkStreamId),
    DuplicateChunk(ChunkId),
    MissingContext(ChunkContextId),
    MissingStream(ChunkStreamId),
    MissingChunk(ChunkId),
    WrongContext {
        stream: ChunkStreamId,
        context: ChunkContextId,
    },
    InvalidTransition {
        chunk: ChunkId,
        operation: &'static str,
    },
    OutstandingLock(ChunkId),
    OutstandingBusy(ChunkId),
    OutstandingTask(ChunkId, ChunkTaskId),
    OutstandingRoutes(ChunkId),
    OutstandingTransaction(ChunkId),
    ChunkCounterMismatch(ChunkContextId),
    StreamStillLive(ChunkStreamId),
    ContextStillLive(ChunkContextId),
    ChunkStillLive(ChunkId),
    BackendResourceStillLive(ChunkId),
    ListLinkStillLive(ChunkId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkRecord {
    pub context: ChunkContextId,
    pub stream: ChunkStreamId,
    pub backend: ChunkBackend,
    pub residency: ChunkResidency,
    pub live: bool,
    pub locked: bool,
    pub busy: bool,
    pub task: Option<ChunkTaskId>,
    pub routes: BTreeSet<ChunkRouteId>,
    pub transaction_active: bool,
    pub primary_linked: bool,
    pub residency_linked: bool,
    pub name_live: bool,
    pub backend_object_live: bool,
    pub memory_buffer_live: bool,
    pub metadata_buffer_live: bool,
    pub file_mapping_live: bool,
    pub file_descriptor_live: bool,
    pub file_path_live: bool,
    pub backing_delete: BackingDeleteOutcome,
}

impl ChunkRecord {
    fn new(
        context: ChunkContextId,
        stream: ChunkStreamId,
        backend: ChunkBackend,
        residency: ChunkResidency,
    ) -> Self {
        Self {
            context,
            stream,
            backend,
            residency,
            live: true,
            locked: false,
            busy: false,
            task: None,
            routes: BTreeSet::new(),
            transaction_active: false,
            primary_linked: true,
            residency_linked: true,
            name_live: true,
            backend_object_live: true,
            memory_buffer_live: backend == ChunkBackend::Memory,
            metadata_buffer_live: backend == ChunkBackend::Memory,
            file_mapping_live: backend == ChunkBackend::File && residency == ChunkResidency::Up,
            file_descriptor_live: backend == ChunkBackend::File,
            file_path_live: backend == ChunkBackend::File,
            backing_delete: BackingDeleteOutcome::NotRequested,
        }
    }

    fn backend_discharged(&self) -> bool {
        !self.backend_object_live
            && !self.memory_buffer_live
            && !self.metadata_buffer_live
            && !self.file_mapping_live
            && !self.file_descriptor_live
            && !self.file_path_live
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkStreamRecord {
    pub context: ChunkContextId,
    pub live: bool,
    pub chunks: BTreeSet<ChunkId>,
    pub backing_delete: BackingDeleteOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkContextRecord {
    pub live: bool,
    pub streams: BTreeSet<ChunkStreamId>,
    pub chunk_total: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChunkState {
    pub contexts: BTreeMap<ChunkContextId, ChunkContextRecord>,
    pub streams: BTreeMap<ChunkStreamId, ChunkStreamRecord>,
    pub chunks: BTreeMap<ChunkId, ChunkRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkCertificate {
    pub final_state: ChunkState,
    pub trace: Vec<ChunkTraceEvent>,
}

pub fn check_chunk_program(
    program: &ChunkProgram,
) -> Result<ChunkCertificate, Vec<ChunkDiagnostic>> {
    let execution = execute_chunk_prefix(&program.operations)?;
    let diagnostics = terminal_diagnostics(&execution.final_state);
    if diagnostics.is_empty() {
        Ok(ChunkCertificate {
            final_state: execution.final_state,
            trace: execution.trace,
        })
    } else {
        Err(diagnostics)
    }
}

/// A checked execution prefix may still own resources. It is deliberately
/// distinct from a teardown certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkExecution {
    pub final_state: ChunkState,
    pub trace: Vec<ChunkTraceEvent>,
}

pub fn execute_chunk_prefix(
    operations: &[ChunkOperation],
) -> Result<ChunkExecution, Vec<ChunkDiagnostic>> {
    let mut state = ChunkState::default();
    let mut trace = Vec::new();
    for operation in operations {
        let mut next = state.clone();
        let mut next_trace = trace.clone();
        if let Err(diagnostic) = apply(operation, &mut next, &mut next_trace) {
            return Err(vec![diagnostic]);
        }
        state = next;
        trace = next_trace;
    }
    Ok(ChunkExecution {
        final_state: state,
        trace,
    })
}

fn apply(
    operation: &ChunkOperation,
    state: &mut ChunkState,
    trace: &mut Vec<ChunkTraceEvent>,
) -> Result<(), ChunkDiagnostic> {
    match *operation {
        ChunkOperation::CreateContext { context } => {
            if state.contexts.contains_key(&context) {
                return Err(ChunkDiagnostic::DuplicateContext(context));
            }
            state.contexts.insert(
                context,
                ChunkContextRecord {
                    live: true,
                    streams: BTreeSet::new(),
                    chunk_total: 0,
                },
            );
            trace.push(ChunkTraceEvent::ContextCreated(context));
        }
        ChunkOperation::CreateStream { context, stream } => {
            if state.streams.contains_key(&stream) {
                return Err(ChunkDiagnostic::DuplicateStream(stream));
            }
            let parent = live_context_mut(state, context)?;
            parent.streams.insert(stream);
            state.streams.insert(
                stream,
                ChunkStreamRecord {
                    context,
                    live: true,
                    chunks: BTreeSet::new(),
                    backing_delete: BackingDeleteOutcome::NotRequested,
                },
            );
            trace.push(ChunkTraceEvent::StreamCreated(stream));
        }
        ChunkOperation::OpenChunk {
            context,
            stream,
            chunk,
            backend,
            residency,
        } => {
            require_stream_context(state, context, stream)?;
            if state.chunks.contains_key(&chunk) {
                return Err(ChunkDiagnostic::DuplicateChunk(chunk));
            }
            state
                .streams
                .get_mut(&stream)
                .expect("checked stream")
                .chunks
                .insert(chunk);
            state
                .contexts
                .get_mut(&context)
                .expect("checked context")
                .chunk_total += 1;
            state
                .chunks
                .insert(chunk, ChunkRecord::new(context, stream, backend, residency));
            trace.push(ChunkTraceEvent::ChunkOpened(chunk));
        }
        ChunkOperation::FailOpenChunk {
            context,
            stream,
            chunk,
            failure,
        } => {
            require_stream_context(state, context, stream)?;
            if state.chunks.contains_key(&chunk) {
                return Err(ChunkDiagnostic::DuplicateChunk(chunk));
            }
            // Every modeled construction failure occurs before list insertion
            // and counter increment, or explicitly rolls those effects back.
            trace.push(ChunkTraceEvent::ChunkOpenFailed(chunk, failure));
        }
        ChunkOperation::SetResidency { chunk, residency } => {
            let record = live_chunk_mut(state, chunk)?;
            if record.locked || record.busy || record.transaction_active {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "set residency",
                });
            }
            record.residency = residency;
            record.file_mapping_live =
                record.backend == ChunkBackend::File && residency == ChunkResidency::Up;
            trace.push(ChunkTraceEvent::ResidencyChanged(chunk, residency));
        }
        ChunkOperation::Lock { chunk } => {
            let record = live_chunk_mut(state, chunk)?;
            if record.locked {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "lock",
                });
            }
            record.locked = true;
            trace.push(ChunkTraceEvent::Locked(chunk));
        }
        ChunkOperation::Unlock { chunk } => {
            let record = live_chunk_mut(state, chunk)?;
            if !record.locked {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "unlock",
                });
            }
            record.locked = false;
            trace.push(ChunkTraceEvent::Unlocked(chunk));
        }
        ChunkOperation::SetBusy { chunk, busy } => {
            let record = live_chunk_mut(state, chunk)?;
            if record.busy == busy {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "set busy",
                });
            }
            record.busy = busy;
            trace.push(ChunkTraceEvent::BusyChanged(chunk, busy));
        }
        ChunkOperation::AttachTask { chunk, task } => {
            let record = live_chunk_mut(state, chunk)?;
            if record.task.is_some() {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "attach task",
                });
            }
            record.task = Some(task);
            trace.push(ChunkTraceEvent::TaskAttached(chunk, task));
        }
        ChunkOperation::DetachTask { chunk, task } => {
            let record = live_chunk_mut(state, chunk)?;
            if record.task != Some(task) {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "detach task",
                });
            }
            record.task = None;
            trace.push(ChunkTraceEvent::TaskDetached(chunk, task));
        }
        ChunkOperation::AddRoute { chunk, route } => {
            let record = live_chunk_mut(state, chunk)?;
            if !record.routes.insert(route) {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "add route",
                });
            }
            trace.push(ChunkTraceEvent::RouteAdded(chunk, route));
        }
        ChunkOperation::RemoveRoute { chunk, route } => {
            let record = live_chunk_mut(state, chunk)?;
            if !record.routes.remove(&route) {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "remove route",
                });
            }
            trace.push(ChunkTraceEvent::RouteRemoved(chunk, route));
        }
        ChunkOperation::BeginTransaction { chunk } => {
            let record = live_chunk_mut(state, chunk)?;
            if record.transaction_active {
                return Err(ChunkDiagnostic::InvalidTransition {
                    chunk,
                    operation: "begin transaction",
                });
            }
            record.transaction_active = true;
            trace.push(ChunkTraceEvent::TransactionBegan(chunk));
        }
        ChunkOperation::CommitTransaction { chunk } => {
            settle_transaction(state, chunk, true, trace)?;
        }
        ChunkOperation::RollbackTransaction { chunk } => {
            settle_transaction(state, chunk, false, trace)?;
        }
        ChunkOperation::CloseChunk { chunk, delete } => {
            close_chunk(state, chunk, delete, trace)?;
        }
        ChunkOperation::DeleteStreamContent {
            stream,
            filesystem_delete_succeeds,
        } => {
            delete_stream_content(state, stream, filesystem_delete_succeeds, trace)?;
        }
        ChunkOperation::DestroyStream { stream } => {
            destroy_stream(state, stream, trace)?;
        }
        ChunkOperation::DestroyContext { context } => {
            destroy_context(state, context, trace)?;
        }
    }
    Ok(())
}

fn live_context_mut(
    state: &mut ChunkState,
    context: ChunkContextId,
) -> Result<&mut ChunkContextRecord, ChunkDiagnostic> {
    state
        .contexts
        .get_mut(&context)
        .filter(|record| record.live)
        .ok_or(ChunkDiagnostic::MissingContext(context))
}

fn live_stream_mut(
    state: &mut ChunkState,
    stream: ChunkStreamId,
) -> Result<&mut ChunkStreamRecord, ChunkDiagnostic> {
    state
        .streams
        .get_mut(&stream)
        .filter(|record| record.live)
        .ok_or(ChunkDiagnostic::MissingStream(stream))
}

fn live_chunk_mut(
    state: &mut ChunkState,
    chunk: ChunkId,
) -> Result<&mut ChunkRecord, ChunkDiagnostic> {
    state
        .chunks
        .get_mut(&chunk)
        .filter(|record| record.live)
        .ok_or(ChunkDiagnostic::MissingChunk(chunk))
}

fn require_stream_context(
    state: &ChunkState,
    context: ChunkContextId,
    stream: ChunkStreamId,
) -> Result<(), ChunkDiagnostic> {
    if !state
        .contexts
        .get(&context)
        .is_some_and(|record| record.live)
    {
        return Err(ChunkDiagnostic::MissingContext(context));
    }
    let Some(record) = state.streams.get(&stream).filter(|record| record.live) else {
        return Err(ChunkDiagnostic::MissingStream(stream));
    };
    if record.context != context {
        return Err(ChunkDiagnostic::WrongContext { stream, context });
    }
    Ok(())
}

fn settle_transaction(
    state: &mut ChunkState,
    chunk: ChunkId,
    commit: bool,
    trace: &mut Vec<ChunkTraceEvent>,
) -> Result<(), ChunkDiagnostic> {
    let record = live_chunk_mut(state, chunk)?;
    if !record.transaction_active {
        return Err(ChunkDiagnostic::InvalidTransition {
            chunk,
            operation: if commit { "commit" } else { "rollback" },
        });
    }
    record.transaction_active = false;
    trace.push(if commit {
        ChunkTraceEvent::TransactionCommitted(chunk)
    } else {
        ChunkTraceEvent::TransactionRolledBack(chunk)
    });
    Ok(())
}

fn close_chunk(
    state: &mut ChunkState,
    chunk: ChunkId,
    delete: bool,
    trace: &mut Vec<ChunkTraceEvent>,
) -> Result<(), ChunkDiagnostic> {
    let record = state
        .chunks
        .get(&chunk)
        .filter(|record| record.live)
        .cloned()
        .ok_or(ChunkDiagnostic::MissingChunk(chunk))?;
    if record.locked {
        return Err(ChunkDiagnostic::OutstandingLock(chunk));
    }
    if record.busy {
        return Err(ChunkDiagnostic::OutstandingBusy(chunk));
    }
    if let Some(task) = record.task {
        return Err(ChunkDiagnostic::OutstandingTask(chunk, task));
    }
    if !record.routes.is_empty() {
        return Err(ChunkDiagnostic::OutstandingRoutes(chunk));
    }
    if record.transaction_active {
        return Err(ChunkDiagnostic::OutstandingTransaction(chunk));
    }

    let current = state.chunks.get_mut(&chunk).expect("checked chunk");
    current.memory_buffer_live = false;
    current.metadata_buffer_live = false;
    current.file_mapping_live = false;
    current.file_descriptor_live = false;
    current.file_path_live = false;
    current.backend_object_live = false;
    trace.push(ChunkTraceEvent::BackendClosed(chunk, current.backend));

    // `cio_file_close` is `void`: it logs a native-delete failure but cannot
    // establish that the backing file was deleted for its caller. Record only
    // the request here. Memory-backed chunks have no filesystem deletion.
    current.backing_delete = if delete && current.backend == ChunkBackend::File {
        BackingDeleteOutcome::Requested
    } else {
        BackingDeleteOutcome::NotRequested
    };
    trace.push(ChunkTraceEvent::BackingDelete(
        chunk,
        current.backing_delete,
    ));

    current.primary_linked = false;
    current.residency_linked = false;
    current.name_live = false;
    trace.push(ChunkTraceEvent::ChunkUnlinked(chunk));

    state
        .streams
        .get_mut(&record.stream)
        .expect("live chunk has stream")
        .chunks
        .remove(&chunk);
    let context = state
        .contexts
        .get_mut(&record.context)
        .expect("live chunk has context");
    context.chunk_total = context
        .chunk_total
        .checked_sub(1)
        .ok_or(ChunkDiagnostic::ChunkCounterMismatch(record.context))?;
    trace.push(ChunkTraceEvent::ChunkCounterDecremented(record.context));
    current.live = false;
    trace.push(ChunkTraceEvent::ChunkClosed(chunk));
    Ok(())
}

fn delete_stream_content(
    state: &mut ChunkState,
    stream: ChunkStreamId,
    filesystem_delete_succeeds: bool,
    trace: &mut Vec<ChunkTraceEvent>,
) -> Result<(), ChunkDiagnostic> {
    let chunks = live_stream_mut(state, stream)?.chunks.clone();
    for chunk in chunks {
        close_chunk(state, chunk, true, trace)?;
    }
    let outcome = if filesystem_delete_succeeds {
        BackingDeleteOutcome::Deleted
    } else {
        BackingDeleteOutcome::Failed
    };
    state
        .streams
        .get_mut(&stream)
        .expect("checked stream")
        .backing_delete = outcome;
    trace.push(ChunkTraceEvent::StreamContentDeleted(stream, outcome));
    Ok(())
}

fn destroy_stream(
    state: &mut ChunkState,
    stream: ChunkStreamId,
    trace: &mut Vec<ChunkTraceEvent>,
) -> Result<(), ChunkDiagnostic> {
    let record = live_stream_mut(state, stream)?.clone();
    for chunk in record.chunks {
        close_chunk(state, chunk, false, trace)?;
    }
    state
        .contexts
        .get_mut(&record.context)
        .expect("live stream has context")
        .streams
        .remove(&stream);
    state.streams.get_mut(&stream).expect("checked stream").live = false;
    trace.push(ChunkTraceEvent::StreamDestroyed(stream));
    Ok(())
}

fn destroy_context(
    state: &mut ChunkState,
    context: ChunkContextId,
    trace: &mut Vec<ChunkTraceEvent>,
) -> Result<(), ChunkDiagnostic> {
    let streams = live_context_mut(state, context)?.streams.clone();
    for stream in streams {
        destroy_stream(state, stream, trace)?;
    }
    let record = state.contexts.get_mut(&context).expect("checked context");
    if record.chunk_total != 0 {
        return Err(ChunkDiagnostic::ChunkCounterMismatch(context));
    }
    record.live = false;
    trace.push(ChunkTraceEvent::ContextDestroyed(context));
    Ok(())
}

fn terminal_diagnostics(state: &ChunkState) -> Vec<ChunkDiagnostic> {
    let mut diagnostics = BTreeSet::new();
    for (&chunk, record) in &state.chunks {
        if record.live {
            diagnostics.insert(ChunkDiagnostic::ChunkStillLive(chunk));
        }
        if !record.backend_discharged() || record.name_live {
            diagnostics.insert(ChunkDiagnostic::BackendResourceStillLive(chunk));
        }
        if record.primary_linked || record.residency_linked {
            diagnostics.insert(ChunkDiagnostic::ListLinkStillLive(chunk));
        }
    }
    for (&stream, record) in &state.streams {
        if record.live {
            diagnostics.insert(ChunkDiagnostic::StreamStillLive(stream));
        }
    }
    for (&context, record) in &state.contexts {
        if record.live {
            diagnostics.insert(ChunkDiagnostic::ContextStillLive(context));
        }
        if record.chunk_total != 0 {
            diagnostics.insert(ChunkDiagnostic::ChunkCounterMismatch(context));
        }
    }
    diagnostics.into_iter().collect()
}

#[cfg(test)]
#[path = "../tests/unit/chunk_lifecycle.rs"]
mod tests;
