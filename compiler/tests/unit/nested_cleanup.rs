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
fn nested_cleanup_preserves_every_non_deleting_file_case_through_parent_free() {
    if !crate::test_support::clang_available() {
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
    if !crate::test_support::clang_available() {
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
    if !crate::test_support::clang_available() {
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
