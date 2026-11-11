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
use crate::verifier_plugins::chunkio::{file_close, file_delete};
use memory_close::tests::{FIXTURE, dependencies};

pub(crate) fn backend() -> FileCloseDeletionEvidence {
    let source = file_close::tests::deletion_fixture();
    let (unmap, close) = file_close::tests::dependencies(&source);
    let unit = C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    let delete =
        file_delete::check_native_delete_source(&unit, PINNED_FLUENT_BIT_REVISION).unwrap();
    file_close::check_file_close_deletion_source(
        &unit,
        PINNED_FLUENT_BIT_REVISION,
        &unmap,
        &close,
        &delete,
    )
    .unwrap()
}
fn checked(
    source: &str,
    backend: &FileCloseDeletionEvidence,
    deps: &MemoryCloseDependencies,
) -> Result<FilesystemChunkCloseEvidence, String> {
    check_filesystem_chunk_close_source(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        deps,
        backend,
    )
}

#[test]
fn filesystem_chunk_release_preserves_nested_and_delete_outcomes() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let backend = backend();
    let e = checked(FIXTURE, &backend, &dependencies()).unwrap();
    assert_eq!(e.cases().len(), 75);
    assert_eq!(e.checked_paths().len(), 2);
    assert_eq!(e.uncovered_paths().len(), 2);
    assert_eq!(e.combined_uncovered_paths().len(), 1);
    assert_eq!(e.cases().iter().filter(|c| c.chunk_released).count(), 73);
    assert_eq!(
        (0..75)
            .filter(|i| e.require_nested_discharge(*i).is_ok())
            .count(),
        36
    );
    assert_eq!(
        (0..75)
            .filter(|i| e.require_successful_deletion(*i).is_ok())
            .count(),
        8
    );
    for c in e.cases() {
        assert_eq!(c.total_counter_decrements, usize::from(c.chunk_present));
        assert_eq!(c.primary_unlinked, c.chunk_present);
        assert_eq!(c.state_unlinked, c.chunk_present);
        assert_eq!(c.name_released, c.chunk_present);
        if let Some(i) = c.backend_case {
            if c.delete {
                let inner = &backend.deleting_cases()[i];
                assert_eq!(c.mapping, inner.mapping);
                assert_eq!(c.descriptor, inner.descriptor);
                assert_eq!(c.deletion, Some(inner.deletion));
                assert_eq!(c.up_counter_decrements, inner.up_counter_decrements);
                assert_eq!(c.backend_released, inner.backend_released);
            } else {
                let inner = &backend.non_deleting_cases()[i];
                assert_eq!(c.mapping, inner.mapping);
                assert_eq!(c.descriptor, inner.descriptor);
                assert_eq!(c.deletion, None);
                assert_eq!(c.up_counter_decrements, inner.up_counter_decrements);
                assert_eq!(c.backend_released, inner.backend_released);
            }
        }
    }
    assert!(e.require_nested_discharge(75).is_err());
    assert!(e.require_successful_deletion(75).is_err());
}

#[test]
fn filesystem_dispatch_rejects_wrong_arguments_and_release_order() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let backend = backend();
    let deps = dependencies();
    for (old, new) in [
        ("cio_file_close(ch,delete);", "cio_file_close(ch,0);"),
        ("cio_file_close(ch,delete);", "cio_file_close(ch,1);"),
        (
            "cio_file_close(ch,delete);",
            "cio_file_close((void*)0,delete);",
        ),
        (
            "cio_file_close(ch,delete);",
            "cio_file_close(ch,delete);cio_file_close(ch,delete);",
        ),
        ("cio_file_close(ch,delete);", "return;"),
        (
            "cio_file_close(ch,delete);",
            "free(ch);cio_file_close(ch,delete);",
        ),
        (
            "cio_file_close(ch,delete);",
            "cio_file_close(ch,delete);free(ch->name);",
        ),
        ("type==CIO_STORE_FS", "type==CIO_STORE_MEM"),
        ("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 1"),
        ("mk_list_del(&ch->_state_head);", "mk_list_del(&ch->_head);"),
        (
            "cio_chunk_counter_total_sub(ctx);",
            "cio_chunk_counter_total_sub(ch->ctx);",
        ),
    ] {
        assert!(
            checked(&FIXTURE.replace(old, new), &backend, &deps).is_err(),
            "accepted {new}"
        );
    }
    assert!(
        checked(
            &FIXTURE.replace("#define CIO_STORE_FS 0", "#define CIO_STORE_FS 9"),
            &backend,
            &deps
        )
        .is_ok()
    );
}

#[test]
fn filesystem_chunk_policy_and_dependencies_fail_closed() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let backend = backend();
    let deps = dependencies();
    let e = checked(FIXTURE, &backend, &deps).unwrap();
    // The decision half is the relocated procedure; the ChunkIO half — the
    // pinned contracts, profiles and recipe — is this adapter's own selection.
    let selection = backend_contracts().unwrap();
    let decide = |contract: &str| {
        qcore_verification::filesystem_chunk_close_composition::check_filesystem_chunk_close_bodies(
            e.memory_scope(),
            e.filesystem_kind_probe(),
            e.backend(),
            &contracts_with(&selection, contract),
        )
    };
    assert_eq!(e, decide(CONTRACT).unwrap());
    for clause in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(clause) {
            let end = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unknown_fact"] {
                let changed = format!("{}{clause}{predicate}{}", &CONTRACT[..i], &CONTRACT[end..]);
                assert!(decide(&changed).is_err());
            }
        }
    }
    let mut wrong = deps;
    wrong.counter = crate::verifier_plugins::chunkio::teardown_source::check_counter_sources(
        &C11TranslationUnit {
            source: FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap()
    .remove(3);
    assert!(checked(FIXTURE, &backend, &wrong).is_err());
    assert!(
        check_filesystem_chunk_close_source(
            &C11TranslationUnit {
                source: FIXTURE,
                ..Default::default()
            },
            "wrong",
            &dependencies(),
            &backend
        )
        .is_err()
    );
}
