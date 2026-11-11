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
use crate::chunk_lifecycle::{
    ChunkBackend, ChunkProgram, ChunkResidency, ChunkTraceEvent, check_chunk_program,
};
use crate::legacy_c::{C11TranslationUnit, CCompilerDialect};
use crate::verifier_plugins::chunkio::{memory_close, parent_teardown, traversal};
use ChunkOperation as C;

pub(crate) fn source() -> NestedMemorySource {
    let unit = C11TranslationUnit {
        source: traversal::tests::FIXTURE,
        dialect: CCompilerDialect::Gnu11,
        ..Default::default()
    };
    let chunks =
        traversal::check_traversal_source(&unit, PINNED_FLUENT_BIT_REVISION, TraversalKind::Chunks)
            .unwrap();
    let streams = traversal::check_traversal_source(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        TraversalKind::Streams,
    )
    .unwrap();
    let parent_fixture = parent_teardown::tests::fixture();
    let unit = C11TranslationUnit {
        source: &parent_fixture,
        ..Default::default()
    };
    let stream =
        parent_teardown::check_parent_source(&unit, PINNED_FLUENT_BIT_REVISION, ParentKind::Stream)
            .unwrap();
    let context = parent_teardown::check_parent_source(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        ParentKind::Context,
    )
    .unwrap();
    let memory = memory_close::check_memory_close_source(
        &C11TranslationUnit {
            source: memory_close::tests::FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &memory_close::tests::dependencies(),
    )
    .unwrap();
    check_nested_memory_source(&chunks, &streams, &stream, &context, &memory).unwrap()
}
fn prefix() -> Vec<C> {
    vec![
        C::CreateContext { context: 1 },
        C::CreateStream {
            context: 1,
            stream: 10,
        },
        C::CreateStream {
            context: 1,
            stream: 20,
        },
        C::OpenChunk {
            context: 1,
            stream: 10,
            chunk: 100,
            backend: ChunkBackend::Memory,
            residency: ChunkResidency::Up,
        },
        C::OpenChunk {
            context: 1,
            stream: 10,
            chunk: 101,
            backend: ChunkBackend::Memory,
            residency: ChunkResidency::Up,
        },
        C::OpenChunk {
            context: 1,
            stream: 20,
            chunk: 200,
            backend: ChunkBackend::Memory,
            residency: ChunkResidency::Up,
        },
    ]
}
fn rings() -> Vec<StreamRing> {
    vec![
        StreamRing {
            stream: 20,
            chunks: vec![200],
        },
        StreamRing {
            stream: 10,
            chunks: vec![101, 100],
        },
    ]
}
#[test]
fn nested_memory_binds_ring_order_and_complete_terminal_teardown() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = source();
    let mut prefix = prefix();
    let operations = source.instantiate(&prefix, Some(1), &rings()).unwrap();
    assert_eq!(
        operations,
        vec![
            C::CloseChunk {
                chunk: 200,
                delete: false
            },
            C::DestroyStream { stream: 20 },
            C::CloseChunk {
                chunk: 101,
                delete: false
            },
            C::CloseChunk {
                chunk: 100,
                delete: false
            },
            C::DestroyStream { stream: 10 },
            C::DestroyContext { context: 1 }
        ]
    );
    prefix.extend(operations);
    let checked = check_chunk_program(&ChunkProgram {
        operations: prefix.clone(),
    })
    .unwrap();
    let closed: Vec<_> = checked
        .trace
        .iter()
        .filter_map(|event| {
            if let ChunkTraceEvent::ChunkClosed(c) = event {
                Some(*c)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(closed, vec![200, 101, 100]);
    assert!(source.instantiate(&prefix, Some(1), &rings()).is_err());
    assert!(source.instantiate(&[], None, &[]).unwrap().is_empty());
    assert!(source.instantiate(&[], None, &rings()).is_err());
    assert_eq!(
        source
            .instantiate(&[C::CreateContext { context: 1 }], Some(1), &[])
            .unwrap(),
        vec![C::DestroyContext { context: 1 }]
    );
    let empty_stream = vec![
        C::CreateContext { context: 1 },
        C::CreateStream {
            context: 1,
            stream: 10,
        },
    ];
    assert_eq!(
        source
            .instantiate(
                &empty_stream,
                Some(1),
                &[StreamRing {
                    stream: 10,
                    chunks: vec![]
                }]
            )
            .unwrap(),
        vec![
            C::DestroyStream { stream: 10 },
            C::DestroyContext { context: 1 }
        ]
    );
}
#[test]
fn nested_memory_rejects_missing_duplicate_wrong_and_filesystem_children() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = source();
    for bad in [
        vec![],
        vec![rings()[0].clone()],
        vec![rings()[0].clone(), rings()[0].clone()],
        vec![
            StreamRing {
                stream: 20,
                chunks: vec![200],
            },
            StreamRing {
                stream: 10,
                chunks: vec![100],
            },
        ],
        vec![
            StreamRing {
                stream: 20,
                chunks: vec![200],
            },
            StreamRing {
                stream: 10,
                chunks: vec![100, 100],
            },
        ],
        vec![
            StreamRing {
                stream: 20,
                chunks: vec![101],
            },
            StreamRing {
                stream: 10,
                chunks: vec![200, 100],
            },
        ],
        vec![
            StreamRing {
                stream: 99,
                chunks: vec![],
            },
            rings()[1].clone(),
        ],
    ] {
        assert!(source.instantiate(&prefix(), Some(1), &bad).is_err());
    }
    for extra in [
        C::Lock { chunk: 100 },
        C::SetBusy {
            chunk: 100,
            busy: true,
        },
        C::BeginTransaction { chunk: 100 },
        C::AttachTask {
            chunk: 100,
            task: 7,
        },
        C::AddRoute {
            chunk: 100,
            route: 8,
        },
    ] {
        let mut bad = prefix();
        bad.push(extra);
        assert!(source.instantiate(&bad, Some(1), &rings()).is_err());
    }
    let mut file = prefix();
    if let C::OpenChunk { backend, .. } = &mut file[3] {
        *backend = ChunkBackend::File;
    }
    assert!(source.instantiate(&file, Some(1), &rings()).is_err());
    assert!(source.instantiate(&prefix(), Some(99), &rings()).is_err());
}
#[test]
fn nested_memory_keeps_unrelated_context_live_and_checks_dependency_roles() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = source();
    let mut prefix = prefix();
    prefix.push(C::CreateContext { context: 2 });
    let operations = source.instantiate(&prefix, Some(1), &rings()).unwrap();
    prefix.extend(operations);
    assert!(execute_chunk_prefix(&prefix).unwrap().final_state.contexts[&2].live);
    assert!(check_chunk_program(&ChunkProgram { operations: prefix }).is_err());
    assert!(
        check_nested_memory_source(
            &source.streams,
            &source.chunks,
            &source.stream_parent,
            &source.context_parent,
            &source.memory
        )
        .is_err()
    );
    assert!(
        check_nested_memory_source(
            &source.chunks,
            &source.streams,
            &source.context_parent,
            &source.stream_parent,
            &source.memory
        )
        .is_err()
    );
}
#[test]
fn nested_memory_policy_fails_closed() {
    for clause in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(clause) {
            let end = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unknown_fact"] {
                assert!(
                    check_policy(&format!(
                        "{}{clause}{predicate}{}",
                        &CONTRACT[..i],
                        &CONTRACT[end..]
                    ))
                    .is_err()
                );
            }
        }
    }
}
