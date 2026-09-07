use super::*;
use crate::legacy_c::C11TranslationUnit;
use crate::verifier_plugins::chunkio::{
    filesystem_chunk_close::{check_filesystem_chunk_close_source, tests::backend},
    memory_close::tests::{FIXTURE, dependencies},
};

#[test]
fn unsupported_close_classifies_the_remaining_path_without_discharge() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let deps = dependencies();
    let backend = backend();
    for (mem, fs) in [(1, 0), (7, 9), (i32::MAX, 42)] {
        let source = FIXTURE
            .replace(
                "#define CIO_STORE_MEM 1",
                &format!("#define CIO_STORE_MEM {mem}"),
            )
            .replace(
                "#define CIO_STORE_FS 0",
                &format!("#define CIO_STORE_FS {fs}"),
            );
        let file = check_filesystem_chunk_close_source(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            &deps,
            &backend,
        );
        let file = file.unwrap();
        let evidence = check_unsupported_close_source(&file).unwrap();
        assert_eq!(file.combined_uncovered_paths(), &[evidence.source_path()]);
        assert_eq!(evidence.excluded_kinds(), [mem, fs]);
        assert!(
            evidence
                .require_nested_discharge()
                .unwrap_err()
                .starts_with("Unknown:")
        );
        assert_eq!(evidence.status, "Unknown");
        assert_eq!(
            evidence,
            check(
                &evidence.memory_scope,
                &evidence.filesystem_kind_probe,
                CONTRACT
            )
            .unwrap()
        );
    }
}

#[test]
fn unsupported_close_rejects_domain_splitting_and_changed_partial_effects() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let deps = dependencies();
    let backend = backend();
    for added in [
        "else if(type==2){return;}",
        "else if(type==2147483647){return;}",
        "else if(type==type){return;}",
        "else {return;}",
        "else {cio_memfs_close(ch);}",
        "else {cio_file_close(ch,delete);}",
        "else {free(ch);}",
        "else {mk_list_del(&ch->_head);}",
        "else {cio_chunk_counter_total_sub(ctx);}",
        "else {if(delete){return;}}",
        "else {ch->backend=(void*)0;}",
    ] {
        let source = FIXTURE.replace(
            "else if(type==CIO_STORE_FS){cio_file_close(ch,delete);}",
            &format!("else if(type==CIO_STORE_FS){{cio_file_close(ch,delete);}}{added}"),
        );
        let file = check_filesystem_chunk_close_source(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            &deps,
            &backend,
        )
        .unwrap();
        assert!(
            check_unsupported_close_source(&file).is_err(),
            "accepted {added}"
        );
    }
}

#[test]
fn unsupported_close_policy_rejects_false_and_unknown_predicates() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let file = check_filesystem_chunk_close_source(
        &C11TranslationUnit {
            source: FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &dependencies(),
        &backend(),
    )
    .unwrap();
    for clause in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(clause) {
            let end = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unknown_fact"] {
                let changed = format!("{}{clause}{predicate}{}", &CONTRACT[..i], &CONTRACT[end..]);
                assert!(
                    check(file.memory_scope(), file.filesystem_kind_probe(), &changed).is_err()
                );
            }
        }
    }
}

#[test]
fn unsupported_backend_truth_tests_cannot_assume_zero_is_excluded() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let deps = dependencies();
    let backend = backend();
    for condition in ["type", "!type", "type==0"] {
        let source = FIXTURE.replace("#define CIO_STORE_MEM 1", "#define CIO_STORE_MEM 7")
            .replace("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 9")
            .replace("else if(type==CIO_STORE_FS){cio_file_close(ch,delete);}",
                &format!("else if(type==CIO_STORE_FS){{cio_file_close(ch,delete);}}else {{if({condition}){{return;}}}}"));
        let file = check_filesystem_chunk_close_source(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            &deps,
            &backend,
        )
        .unwrap();
        assert!(
            check_unsupported_close_source(&file).is_err(),
            "accepted {condition}"
        );
    }
}
