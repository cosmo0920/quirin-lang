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

fn base(backend: ChunkBackend) -> Vec<ChunkOperation> {
    vec![
        ChunkOperation::CreateContext { context: 1 },
        ChunkOperation::CreateStream {
            context: 1,
            stream: 10,
        },
        ChunkOperation::OpenChunk {
            context: 1,
            stream: 10,
            chunk: 100,
            backend,
            residency: ChunkResidency::Up,
        },
    ]
}

#[test]
fn context_destroy_runs_the_nested_stream_chunk_backend_chain() {
    for backend in [ChunkBackend::Memory, ChunkBackend::File] {
        let mut operations = base(backend);
        operations.push(ChunkOperation::DestroyContext { context: 1 });
        let certificate = check_chunk_program(&ChunkProgram { operations }).unwrap();
        let chunk = &certificate.final_state.chunks[&100];
        assert!(!chunk.live);
        assert!(chunk.backend_discharged());
        assert_eq!(certificate.final_state.contexts[&1].chunk_total, 0);
        assert!(certificate.trace.windows(4).any(|events| {
            matches!(events[0], ChunkTraceEvent::BackendClosed(100, _))
                && events[1]
                    == ChunkTraceEvent::BackingDelete(100, BackingDeleteOutcome::NotRequested)
                && events[2] == ChunkTraceEvent::ChunkUnlinked(100)
                && events[3] == ChunkTraceEvent::ChunkCounterDecremented(1)
        }));
    }
}

#[test]
fn delete_stream_content_is_not_a_stream_destructor() {
    let mut operations = base(ChunkBackend::File);
    operations.push(ChunkOperation::DeleteStreamContent {
        stream: 10,
        filesystem_delete_succeeds: true,
    });
    let diagnostics = check_chunk_program(&ChunkProgram { operations }).unwrap_err();
    assert!(diagnostics.contains(&ChunkDiagnostic::StreamStillLive(10)));
    assert!(diagnostics.contains(&ChunkDiagnostic::ContextStillLive(1)));
    assert!(!diagnostics.contains(&ChunkDiagnostic::ChunkStillLive(100)));
}

#[test]
fn close_records_only_an_auditable_file_delete_request() {
    for (backend, expected) in [
        (ChunkBackend::Memory, BackingDeleteOutcome::NotRequested),
        (ChunkBackend::File, BackingDeleteOutcome::Requested),
    ] {
        let mut operations = base(backend);
        operations.extend([
            ChunkOperation::CloseChunk {
                chunk: 100,
                delete: true,
            },
            ChunkOperation::DestroyContext { context: 1 },
        ]);
        let certificate = check_chunk_program(&ChunkProgram { operations }).unwrap();
        assert_eq!(
            certificate.final_state.chunks[&100].backing_delete,
            expected
        );
    }
}

#[test]
fn delete_failure_has_an_explicit_partial_state_that_can_still_be_destroyed() {
    let mut operations = base(ChunkBackend::File);
    operations.extend([
        ChunkOperation::DeleteStreamContent {
            stream: 10,
            filesystem_delete_succeeds: false,
        },
        ChunkOperation::DestroyContext { context: 1 },
    ]);
    let certificate = check_chunk_program(&ChunkProgram { operations }).unwrap();
    assert_eq!(
        certificate.final_state.streams[&10].backing_delete,
        BackingDeleteOutcome::Failed
    );
    assert!(
        certificate
            .trace
            .contains(&ChunkTraceEvent::StreamContentDeleted(
                10,
                BackingDeleteOutcome::Failed
            ))
    );
}

#[test]
fn teardown_rejects_lock_busy_task_route_and_transaction_obligations() {
    let obligations = [
        ChunkOperation::Lock { chunk: 100 },
        ChunkOperation::SetBusy {
            chunk: 100,
            busy: true,
        },
        ChunkOperation::AttachTask {
            chunk: 100,
            task: 7,
        },
        ChunkOperation::AddRoute {
            chunk: 100,
            route: 9,
        },
        ChunkOperation::BeginTransaction { chunk: 100 },
    ];
    for obligation in obligations {
        let mut operations = base(ChunkBackend::Memory);
        operations.extend([obligation, ChunkOperation::DestroyContext { context: 1 }]);
        assert!(check_chunk_program(&ChunkProgram { operations }).is_err());
    }
}

#[test]
fn settled_wrapper_obligations_allow_teardown() {
    let mut operations = base(ChunkBackend::File);
    operations.extend([
        ChunkOperation::SetResidency {
            chunk: 100,
            residency: ChunkResidency::Down,
        },
        ChunkOperation::Lock { chunk: 100 },
        ChunkOperation::Unlock { chunk: 100 },
        ChunkOperation::SetBusy {
            chunk: 100,
            busy: true,
        },
        ChunkOperation::SetBusy {
            chunk: 100,
            busy: false,
        },
        ChunkOperation::AttachTask {
            chunk: 100,
            task: 7,
        },
        ChunkOperation::DetachTask {
            chunk: 100,
            task: 7,
        },
        ChunkOperation::AddRoute {
            chunk: 100,
            route: 9,
        },
        ChunkOperation::RemoveRoute {
            chunk: 100,
            route: 9,
        },
        ChunkOperation::BeginTransaction { chunk: 100 },
        ChunkOperation::RollbackTransaction { chunk: 100 },
        ChunkOperation::DestroyContext { context: 1 },
    ]);
    assert!(check_chunk_program(&ChunkProgram { operations }).is_ok());
}

#[test]
fn failed_open_does_not_create_list_or_counter_obligations() {
    for failure in [
        ChunkOpenFailure::ChunkAllocation,
        ChunkOpenFailure::NameAllocation,
        ChunkOpenFailure::BackendOpen,
        ChunkOpenFailure::Registration,
    ] {
        let program = ChunkProgram {
            operations: vec![
                ChunkOperation::CreateContext { context: 1 },
                ChunkOperation::CreateStream {
                    context: 1,
                    stream: 10,
                },
                ChunkOperation::FailOpenChunk {
                    context: 1,
                    stream: 10,
                    chunk: 100,
                    failure,
                },
                ChunkOperation::DestroyContext { context: 1 },
            ],
        };
        let certificate = check_chunk_program(&program).unwrap();
        assert_eq!(certificate.final_state.contexts[&1].chunk_total, 0);
        assert!(!certificate.final_state.chunks.contains_key(&100));
    }
}
