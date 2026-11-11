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
use crate::cleanup_outcome::check_cleanup_outcomes;
use crate::counter_contract::check_counter_contract;
use crate::typed_source::{SourceImportProfile, SourceResult, import_source_functions};
use crate::verifier_plugins::chunkio::{
    teardown_source::COUNTER_CONTRACT, unmap_wrapper::check_unmap_wrapper_source,
};
fn fixture() -> String {
    let source =
        crate::test_support::normalize_newlines(include_str!("../fixtures/chunkio/unmap_source.c"))
            .replace("struct cio_file {", "struct cio_file {int fd;");
    let start = source.find("void cio_file_close(").unwrap();
    format!(
        "{}{}",
        &source[..start],
        r#"
int close(int);int cio_file_native_delete(struct cio_file *);
int cio_file_native_close(struct cio_file *cf){if(!cf)return -1;if(cf->fd!=-1){int r=close(cf->fd);if(r==-1){cio_errno_print(*__errno_location(),"test.c",1);return -1;}cf->fd=-1;}return 0;}
void cio_file_close(struct cio_chunk *ch,int delete){struct cio_file *cf=ch->backend;if(!cf)return;
munmap_file(ch->ctx,ch);cio_file_native_close(cf);
if(delete==1){int r=cio_file_native_delete(cf);if(r!=0)cio_log_print(ch->ctx,1,"test.c",1,"delete failed %s:%s",ch->st->name,ch->name);}
free(cf->path);free(cf);
}
"#
    )
}
fn import(c: &str, f: &str) -> SourceSummary {
    let summaries = import_source_functions(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &[f],
        &SourceImportProfile {
            records: &["cio_ctx", "cio_chunk", "cio_file", "cio_stream"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    let SourceResult::Summary(s) = &summaries[0] else {
        panic!("{summaries:?}")
    };
    s.clone()
}
pub(crate) fn dependencies(c: &str) -> (UnmapCertificate, CleanupCertificate) {
    let native = |function: &str| {
        check_cleanup_outcomes(
            &import(c, function),
            &file_outcomes::profile(function).unwrap(),
            file_outcomes::CONTRACT,
        )
        .unwrap()
    };
    let counter = check_counter_contract(
        &import(c, "cio_chunk_counter_total_up_sub"),
        COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )
    .unwrap();
    let unmap = check_unmap_wrapper_source(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &native("cio_file_native_unmap"),
        &counter,
    )
    .unwrap();
    (unmap, native("cio_file_native_close"))
}
#[test]
fn backend_release_does_not_discharge_unknown_os_custody() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let c = fixture();
    let (unmap, close) = dependencies(&c);
    let evidence = check_file_close_source(
        &C11TranslationUnit {
            source: &c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &unmap,
        &close,
    )
    .unwrap();
    assert_eq!(evidence.cases().len(), 43);
    assert_eq!(evidence.uncovered_paths(), [1, 2]);
    assert_eq!(
        evidence
            .cases()
            .iter()
            .filter(|c| c.backend_released)
            .count(),
        42
    );
    assert_eq!(
        evidence.cases().iter().filter(|c| c.path_released).count(),
        21
    );
    let mut accepted = 0;
    for (i, case) in evidence.cases().iter().enumerate() {
        let discharged =
            case.mapping == ResidualCustody::Absent && case.descriptor == ResidualCustody::Absent;
        assert_eq!(evidence.require_nested_discharge(i).is_ok(), discharged);
        accepted += usize::from(discharged);
        if !discharged {
            assert!(case.backend_released);
        }
    }
    assert_eq!(accepted, 17);
    assert!(evidence.require_nested_discharge(43).is_err());
    assert!(
        evidence
            .cases()
            .iter()
            .any(|c| c.mapping == ResidualCustody::Unresolved
                && c.descriptor == ResidualCustody::Unresolved
                && c.backend_released)
    );
}
#[test]
fn file_close_rejects_wrong_resource_identity_order_and_result_handling() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let c = fixture();
    let (unmap, close) = dependencies(&c);
    for (old, new) in [
        (
            "munmap_file(ch->ctx,ch);cio_file_native_close(cf);",
            "cio_file_native_close(cf);munmap_file(ch->ctx,ch);",
        ),
        (
            "munmap_file(ch->ctx,ch);cio_file_native_close(cf);",
            "munmap_file((void*)0,ch);cio_file_native_close(cf);",
        ),
        (
            "munmap_file(ch->ctx,ch);cio_file_native_close(cf);",
            "munmap_file(ch->ctx,ch);cio_file_native_close((void*)0);",
        ),
        (
            "munmap_file(ch->ctx,ch);cio_file_native_close(cf);",
            "if(munmap_file(ch->ctx,ch))return;cio_file_native_close(cf);",
        ),
        ("free(cf->path);free(cf);", "free(cf);free(cf->path);"),
        ("free(cf->path);free(cf);", "free(ch->name);free(cf);"),
        ("free(cf->path);free(cf);", "free((void*)0);free(cf);"),
        ("free(cf->path);free(cf);", "free(cf->path);"),
        ("free(cf->path);free(cf);", "free(cf->path);free(ch);"),
        (
            "free(cf->path);free(cf);",
            "free(cf->path);free(cf);ch->backend=(void*)0;",
        ),
        (
            "free(cf->path);free(cf);",
            "if(!cf->path)return;free(cf->path);free(cf);",
        ),
    ] {
        assert!(
            check(
                &import(&c.replace(old, new), "cio_file_close"),
                &unmap,
                &close,
                CONTRACT
            )
            .is_err(),
            "accepted {old} -> {new}"
        );
    }
    let changed = c.replace("if(!cf->map)return -1;", "if(!cf->map)return 0;");
    assert!(
        check_file_close_source(
            &C11TranslationUnit {
                source: &changed,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            &unmap,
            &close
        )
        .is_err()
    );
}
#[test]
fn file_close_native_policy_cannot_hide_failure_custody() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let c = fixture();
    let (unmap, close) = dependencies(&c);
    let source = import(&c, "cio_file_close");
    for marker in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(marker) {
            let j = i + CONTRACT[i..].find(';').unwrap();
            for pred in ["false", "unbound_file_custody"] {
                let mutated = format!("{}{marker}{pred}{}", &CONTRACT[..i], &CONTRACT[j..]);
                assert!(check(&source, &unmap, &close, &mutated).is_err());
            }
        }
    }
}

pub(crate) fn deletion_fixture() -> String {
    format!(
        "{}{}",
        fixture().replace(
            "delete failed %s:%s",
            "[cio file] error deleting file at close %s:%s"
        ),
        r#"
int unlink(const char *);
int cio_file_native_delete(struct cio_file *cf){if(cf->fd!=-1 || cf->map!=(void*)0)return -1;int r=unlink(cf->path);if(r==-1){cio_errno_print(*__errno_location(),"test.c",1);return -1;}return 0;}
"#
    )
}

/// The moved process route reads its five bodies out of a committed `.c` file
/// rather than reconstructing them, so the two routes cannot silently drift onto
/// different source. This asserts the committed syntax, after excluding the
/// checkout's line-ending policy, **is** what this retained in-process suite
/// builds; `QCORE_WRITE_FILE_CLOSE_FIXTURE` names a path to regenerate it.
#[test]
fn the_committed_file_close_fixture_is_the_source_this_route_reads() {
    let built = deletion_fixture();
    if let Some(path) = std::env::var_os("QCORE_WRITE_FILE_CLOSE_FIXTURE") {
        std::fs::write(path, &built).unwrap();
        return;
    }
    assert_eq!(
        built,
        crate::test_support::normalize_newlines(include_str!(
            "../fixtures/chunkio/file_close_source.c"
        )),
        "the committed file_close fixture is no longer the source the \
         in-process route reads"
    );
}
#[test]
fn deleting_close_covers_all_paths_and_keeps_failure_classifications() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let c = deletion_fixture();
    let (unmap, close) = dependencies(&c);
    let delete = crate::verifier_plugins::chunkio::file_delete::check_native_delete_source(
        &C11TranslationUnit {
            source: &c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap();
    let evidence = check_file_close_deletion_source(
        &C11TranslationUnit {
            source: &c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &unmap,
        &close,
        &delete,
    )
    .unwrap();
    assert_eq!(evidence.deleting_cases().len(), 30);
    assert!(evidence.uncovered_paths().is_empty());
    assert_eq!(evidence.no_delete().cases().len(), 43);
    for (kind, count) in [
        (DeletionOutcome::NoBackend, 1),
        (DeletionOutcome::BlockedByFields, 13),
        (DeletionOutcome::SyscallFailed, 8),
        (DeletionOutcome::SyscallSucceeded, 8),
    ] {
        assert_eq!(
            evidence
                .deleting_cases()
                .iter()
                .filter(|c| c.deletion == kind)
                .count(),
            count
        );
    }
    for (i, case) in evidence.deleting_cases().iter().enumerate() {
        assert_eq!(
            evidence.require_successful_deletion(i).is_ok(),
            case.deletion == DeletionOutcome::SyscallSucceeded
        );
        assert_eq!(case.backend_released, case.unmap_case.is_some());
        assert_eq!(case.path_released, case.backend_released);
        if case.deletion == DeletionOutcome::BlockedByFields {
            assert!(
                case.mapping == ResidualCustody::Unresolved
                    || case.descriptor == ResidualCustody::Unresolved
            );
        }
    }
    assert!(evidence.require_successful_deletion(30).is_err());
}
#[test]
fn deleting_close_rejects_wrong_delete_guards_args_logs_and_early_free() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let c = deletion_fixture();
    let (unmap, close) = dependencies(&c);
    let delete = crate::verifier_plugins::chunkio::file_delete::check_native_delete_source(
        &C11TranslationUnit {
            source: &c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap();
    for (old, new) in [
        (
            "int r=cio_file_native_delete(cf)",
            "int r=cio_file_native_delete((void*)0)",
        ),
        (
            "int r=cio_file_native_delete(cf)",
            "free(cf);int r=cio_file_native_delete(cf)",
        ),
        ("if(r!=0)cio_log_print", "if(r==0)cio_log_print"),
        ("if(r!=0)cio_log_print", "if(r!=0)return;else cio_log_print"),
        ("if(delete==1)", "if(delete==0)"),
        (
            "[cio file] error deleting file at close %s:%s\",ch->st->name,ch->name",
            "[cio file] error deleting file at close %s:%s\",ch->name,ch->st->name",
        ),
    ] {
        let changed = c.replace(old, new);
        assert!(
            check_file_close_deletion_source(
                &C11TranslationUnit {
                    source: &changed,
                    ..Default::default()
                },
                PINNED_FLUENT_BIT_REVISION,
                &unmap,
                &close,
                &delete
            )
            .is_err(),
            "accepted {old} -> {new}"
        );
    }
}
#[test]
fn deleting_close_executes_all_native_contract_clauses() {
    if !crate::test_support::posix_clang_test() {
        return;
    }
    let c = deletion_fixture();
    let (unmap, close) = dependencies(&c);
    let delete = crate::verifier_plugins::chunkio::file_delete::check_native_delete_source(
        &C11TranslationUnit {
            source: &c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap();
    let base = check_file_close_source(
        &C11TranslationUnit {
            source: &c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &unmap,
        &close,
    )
    .unwrap();
    for marker in ["requires ", "ensures normal "] {
        for (i, _) in DELETE_CONTRACT.match_indices(marker) {
            let j = i + DELETE_CONTRACT[i..].find(';').unwrap();
            for pred in ["false", "unknown_delete_fact"] {
                let changed = format!(
                    "{}{marker}{pred}{}",
                    &DELETE_CONTRACT[..i],
                    &DELETE_CONTRACT[j..]
                );
                assert!(check_deletion(base.clone(), &delete, &changed).is_err());
            }
        }
    }
}
