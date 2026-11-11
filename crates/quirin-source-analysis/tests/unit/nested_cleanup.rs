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
use crate::legacy_c::C11TranslationUnit;
use crate::verifier_plugins::chunkio::{
    PINNED_FLUENT_BIT_REVISION, filesystem_chunk_close, memory_close, nested_memory,
};

fn source() -> NestedCleanupSource {
    let chain = nested_memory::tests::source();
    let file = filesystem_chunk_close::check_filesystem_chunk_close_source(
        &C11TranslationUnit {
            source: memory_close::tests::FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &memory_close::tests::dependencies(),
        &filesystem_chunk_close::tests::backend(),
    )
    .unwrap();
    check_nested_cleanup_source(&chain, &file).unwrap()
}

#[test]
fn nested_unsupported_custody_survives_stream_and_context_release() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let base = source();
    let unsupported =
        crate::verifier_plugins::chunkio::unsupported_close::check_unsupported_close_source(
            &base.filesystem,
        )
        .unwrap();
    let source = base.with_unsupported(&unsupported).unwrap();
    let (file_case, file) = source
        .filesystem
        .cases()
        .iter()
        .enumerate()
        .find(|(_, c)| c.chunk_present && !c.delete && c.mapping == ResidualCustody::Unresolved)
        .unwrap();
    for kind in [i32::MIN, -1, 2, i32::MAX] {
        let streams = [
            ConditionalStream {
                stream: 10,
                chunks: vec![ChildCase::Memory { chunk: 11 }],
            },
            ConditionalStream {
                stream: 20,
                chunks: vec![
                    ChildCase::Unsupported { chunk: 21, kind },
                    ChildCase::Unsupported { chunk: 22, kind },
                ],
            },
            ConditionalStream {
                stream: 30,
                chunks: vec![ChildCase::Filesystem {
                    chunk: 31,
                    case: file_case,
                }],
            },
        ];
        let result = source
            .project(1, &streams, 4, file.up_counter_decrements)
            .unwrap();
        assert_eq!(result.released_chunks(), &[11, 21, 22, 31]);
        assert_eq!(result.released_streams, [10, 20, 30]);
        assert!(result.context_released);
        assert_eq!(result.total_counter_after, 0);
        assert_eq!(result.up_counter_after(), 0);
        assert_eq!(
            result.opaque_residuals(),
            &[
                OpaqueChildResidual {
                    stream: 20,
                    chunk: 21,
                    backend_kind: kind
                },
                OpaqueChildResidual {
                    stream: 20,
                    chunk: 22,
                    backend_kind: kind
                },
            ]
        );
        assert_eq!(result.residuals().len(), 1);
        assert!(result.require_conditional_discharge().is_err());
    }
    let result = source
        .project(
            1,
            &[ConditionalStream {
                stream: 2,
                chunks: vec![ChildCase::Unsupported { chunk: 3, kind: 2 }],
            }],
            1,
            0,
        )
        .unwrap();
    assert!(result.residuals().is_empty());
    assert_eq!(result.opaque_residuals().len(), 1);
    assert!(result.require_conditional_discharge().is_err());
}

#[test]
fn nested_unsupported_requires_matching_source_unknown_kind_and_unique_identity() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let base = source();
    let unsupported =
        crate::verifier_plugins::chunkio::unsupported_close::check_unsupported_close_source(
            &base.filesystem,
        )
        .unwrap();
    let source = base.with_unsupported(&unsupported).unwrap();
    let stream = ConditionalStream {
        stream: 2,
        chunks: vec![ChildCase::Unsupported { chunk: 3, kind: 2 }],
    };
    assert!(
        base.project(1, std::slice::from_ref(&stream), 1, 0)
            .is_err()
    );
    assert!(
        source
            .project(1, std::slice::from_ref(&stream), 0, 0)
            .is_err()
    );
    assert!(source.project(1, &[stream.clone(), stream], 2, 0).is_err());
    for kind in unsupported.excluded_kinds() {
        assert!(
            source
                .project(
                    1,
                    &[ConditionalStream {
                        stream: 2,
                        chunks: vec![ChildCase::Unsupported { chunk: 3, kind }]
                    }],
                    1,
                    0
                )
                .is_err()
        );
    }
    assert!(
        source
            .project(
                1,
                &[
                    ConditionalStream {
                        stream: 2,
                        chunks: vec![ChildCase::Memory { chunk: 3 }]
                    },
                    ConditionalStream {
                        stream: 4,
                        chunks: vec![ChildCase::Unsupported { chunk: 3, kind: 2 }]
                    },
                ],
                2,
                0
            )
            .is_err()
    );
    let changed =
        memory_close::tests::FIXTURE.replace("#define CIO_STORE_MEM 1", "#define CIO_STORE_MEM 7");
    let file = filesystem_chunk_close::check_filesystem_chunk_close_source(
        &C11TranslationUnit {
            source: &changed,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &memory_close::tests::dependencies(),
        &filesystem_chunk_close::tests::backend(),
    )
    .unwrap();
    let other =
        crate::verifier_plugins::chunkio::unsupported_close::check_unsupported_close_source(&file)
            .unwrap();
    assert!(base.with_unsupported(&other).is_err());
}

#[test]
fn nested_unsupported_executes_policy_and_preserves_existing_cases() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let base = source();
    let unsupported =
        crate::verifier_plugins::chunkio::unsupported_close::check_unsupported_close_source(
            &base.filesystem,
        )
        .unwrap();
    let source = base.with_unsupported(&unsupported).unwrap();
    for (i, c) in base
        .filesystem
        .cases()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.chunk_present && !c.delete)
    {
        let streams = [ConditionalStream {
            stream: 2,
            chunks: vec![ChildCase::Filesystem { chunk: 3, case: i }],
        }];
        assert_eq!(
            base.project(1, &streams, 1, c.up_counter_decrements),
            source.project(1, &streams, 1, c.up_counter_decrements)
        );
    }
    let streams = [ConditionalStream {
        stream: 2,
        chunks: vec![ChildCase::Unsupported { chunk: 3, kind: 2 }],
    }];
    for clause in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(clause) {
            let end = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unknown_fact"] {
                let changed = format!("{}{clause}{predicate}{}", &CONTRACT[..i], &CONTRACT[end..]);
                assert!(
                    source
                        .project_with_contract(1, &streams, 1, 0, &changed)
                        .is_err()
                );
            }
        }
    }
}
#[test]
fn nested_cleanup_preserves_every_non_deleting_file_case_through_parent_free() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let source = source();
    let mut checked = 0;
    let mut unresolved = 0;
    for (i, child) in source
        .filesystem
        .cases()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.chunk_present && !c.delete)
    {
        let streams = vec![ConditionalStream {
            stream: 10,
            chunks: vec![
                ChildCase::Memory { chunk: 100 },
                ChildCase::Filesystem {
                    chunk: 101,
                    case: i,
                },
            ],
        }];
        let e = source
            .project(1, &streams, 2, child.up_counter_decrements)
            .unwrap();
        assert_eq!(e.released_chunks(), &[100, 101]);
        assert_eq!(e.released_streams, vec![10]);
        assert!(e.context_released);
        assert_eq!(e.total_counter_after, 0);
        assert_eq!(e.up_counter_after(), 0);
        let fails =
            child.mapping != ResidualCustody::Absent || child.descriptor != ResidualCustody::Absent;
        assert_eq!(e.require_conditional_discharge().is_err(), fails);
        if fails {
            assert_eq!(
                e.residuals(),
                &[ChildResidual {
                    stream: 10,
                    chunk: 101,
                    mapping: child.mapping,
                    descriptor: child.descriptor
                }]
            );
            unresolved += 1;
        } else {
            assert!(e.residuals().is_empty());
        }
        checked += 1;
    }
    assert_eq!((checked, unresolved), (43, 26));
}
#[test]
fn nested_cleanup_rejects_wrong_cases_identities_and_counter_binding() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let source = source();
    for (i, c) in source
        .filesystem
        .cases()
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.chunk_present || c.delete)
    {
        assert!(
            source
                .project(
                    1,
                    &[ConditionalStream {
                        stream: 2,
                        chunks: vec![ChildCase::Filesystem { chunk: 3, case: i }]
                    }],
                    1,
                    c.up_counter_decrements
                )
                .is_err()
        );
    }
    let child = source
        .filesystem
        .cases()
        .iter()
        .enumerate()
        .find(|(_, c)| c.chunk_present && !c.delete && c.up_counter_decrements == 1)
        .unwrap()
        .0;
    let stream = ConditionalStream {
        stream: 2,
        chunks: vec![ChildCase::Filesystem {
            chunk: 3,
            case: child,
        }],
    };
    assert!(
        source
            .project(1, std::slice::from_ref(&stream), 1, 0)
            .is_err()
    );
    assert!(
        source
            .project(1, std::slice::from_ref(&stream), 2, 1)
            .is_err()
    );
    assert!(source.project(1, &[stream.clone(), stream], 2, 2).is_err());
    assert!(
        source
            .project(
                1,
                &[ConditionalStream {
                    stream: 2,
                    chunks: vec![
                        ChildCase::Memory { chunk: 3 },
                        ChildCase::Memory { chunk: 3 }
                    ]
                }],
                2,
                0
            )
            .is_err()
    );
    assert!(
        source
            .project(
                1,
                &[ConditionalStream {
                    stream: 2,
                    chunks: vec![ChildCase::Filesystem {
                        chunk: 3,
                        case: usize::MAX
                    }]
                }],
                1,
                0
            )
            .is_err()
    );
    assert!(
        source
            .project(1, &[], 0, 1)
            .unwrap()
            .require_conditional_discharge()
            .is_err()
    );
    assert!(
        source
            .project(1, &[], 0, 0)
            .unwrap()
            .require_conditional_discharge()
            .is_ok()
    );
}
#[test]
fn nested_cleanup_retains_multiple_stream_residual_identities_and_policy() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let source = source();
    let (i, c) = source
        .filesystem
        .cases()
        .iter()
        .enumerate()
        .find(|(_, c)| c.chunk_present && !c.delete && c.mapping == ResidualCustody::Unresolved)
        .unwrap();
    let streams = [
        ConditionalStream {
            stream: 10,
            chunks: vec![ChildCase::Filesystem { chunk: 11, case: i }],
        },
        ConditionalStream {
            stream: 20,
            chunks: vec![ChildCase::Filesystem { chunk: 21, case: i }],
        },
    ];
    let case = source
        .project(1, &streams, 2, c.up_counter_decrements * 2)
        .unwrap();
    assert_eq!(
        case.residuals()
            .iter()
            .map(|r| (r.stream, r.chunk))
            .collect::<Vec<_>>(),
        vec![(10, 11), (20, 21)]
    );
    assert!(case.require_conditional_discharge().is_err());
    for clause in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(clause) {
            let end = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unknown_fact"] {
                assert!(
                    source
                        .project_with_contract(
                            1,
                            &streams,
                            2,
                            c.up_counter_decrements * 2,
                            &format!("{}{clause}{predicate}{}", &CONTRACT[..i], &CONTRACT[end..])
                        )
                        .is_err()
                );
            }
        }
    }
}
