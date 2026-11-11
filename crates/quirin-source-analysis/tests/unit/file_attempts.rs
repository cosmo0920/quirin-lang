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
const SOURCE: &str = "struct cio_file{char *path;};struct cio_chunk{void *backend;struct cio_ctx *ctx;};struct cio_ctx{};int munmap_file(struct cio_ctx*,struct cio_chunk*);int cio_file_native_close(struct cio_file*);void free(void*);void cio_file_close(struct cio_chunk *ch,int delete){struct cio_file *cf=ch->backend;if(!cf)return;munmap_file(ch->ctx,ch);cio_file_native_close(cf);free(cf->path);free(cf);}";
fn source(c: &str) -> SourceSummary {
    let summaries = import_source_functions(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &["cio_file_close"],
        &SourceImportProfile {
            records: &["cio_file", "cio_chunk", "cio_ctx"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    let SourceResult::Summary(source) = &summaries[0] else {
        panic!("{summaries:?}")
    };
    source.clone()
}
#[test]
fn file_close_preserves_ignored_results_as_unresolved_cleanup() {
    if !crate::test_support::clang_available() {
        return;
    }
    let summary = source(SOURCE);
    let evidence = check(&summary, CONTRACT).unwrap();
    assert_eq!(evidence.calls.len(), 2);
    let calls = &evidence.calls[1].calls;
    assert_eq!(calls[0].function, "munmap_file");
    assert!(calls[0].uses.is_empty());
    assert_eq!(calls[1].function, "cio_file_native_close");
    assert!(calls[1].uses.is_empty());
    assert!(evidence.status.contains("Unknown"));
    for (old, new) in [
        (
            "cio_file_native_close(cf);",
            "if(cio_file_native_close(cf))return;",
        ),
        ("free(cf->path);free(cf);", "free(cf);"),
        ("munmap_file(ch->ctx,ch);", ""),
    ] {
        assert!(check(&source(&SOURCE.replace(old, new)), CONTRACT).is_err());
    }
    for (i, _) in
        CONTRACT[..CONTRACT.find("extern fn file_unmap").unwrap()].match_indices("ensures normal ")
    {
        let j = i + CONTRACT[i..].find(';').unwrap();
        for predicate in ["false", "unknown_fact"] {
            let changed = format!(
                "{}ensures normal {predicate}{}",
                &CONTRACT[..i],
                &CONTRACT[j..]
            );
            assert!(check(&summary, &changed).is_err());
        }
    }
}

#[test]
fn native_and_wrapper_guards_and_each_policy_predicate_are_checked() {
    if !crate::test_support::clang_available() {
        return;
    }
    let c = "struct cio_file {int fd;void *map;unsigned long len;}; struct cio_ctx {int x;}; struct cio_chunk {void *backend;}; int close(int); int munmap(void *,unsigned long); int cio_file_native_close(struct cio_file *cf){int r=close(cf->fd);if(r==-1)return -1;return 0;} int cio_file_native_unmap(struct cio_file *cf){int r=munmap(cf->map,cf->len);if(r!=0)return -1;return 0;} int munmap_file(struct cio_ctx *ctx,struct cio_chunk *ch){int r=cio_file_native_unmap(ch->backend);if(r!=0)return -1;return 0;}";
    let summaries = import_source_functions(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &[
            "cio_file_native_close",
            "cio_file_native_unmap",
            "munmap_file",
        ],
        &SourceImportProfile {
            records: &["cio_file", "cio_ctx", "cio_chunk"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    for (result, policy) in summaries.iter().zip([
        "native_close_result_use",
        "native_unmap_result_use",
        "file_unmap_result_use",
    ]) {
        let SourceResult::Summary(summary) = result else {
            panic!("{result:?}")
        };
        check(summary, CONTRACT).unwrap();
        let start = CONTRACT.find(&format!("extern fn {policy}(")).unwrap();
        let end = start + CONTRACT[start..].find("};").unwrap();
        for (offset, _) in CONTRACT[start..end].match_indices("ensures normal ") {
            let i = start + offset;
            let j = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unsupported_predicate"] {
                let changed = format!(
                    "{}ensures normal {predicate}{}",
                    &CONTRACT[..i],
                    &CONTRACT[j..]
                );
                assert!(check(summary, &changed).is_err());
            }
        }
    }
}

#[test]
fn native_delete_preserves_short_circuit_reads_and_guarded_unlink_result() {
    if !crate::test_support::clang_available() {
        return;
    }
    let c = "struct cio_file {int fd;void *map;char *path;};int unlink(const char*);int cio_file_native_delete(struct cio_file *cf){if(cf->fd != -1 || cf->map != (void*)0)return -1;int r=unlink(cf->path);if(r == -1)return -1;return 0;}";
    let evidence = check_file_delete_source(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap();
    assert_eq!(evidence.source().paths().len(), 4);
    let paths = evidence.source().paths();
    let reads = |index: usize| {
        paths[index]
            .operations
            .iter()
            .filter(|op| matches!(op, crate::typed_source::SourceOperation::Read { .. }))
            .count()
    };
    assert_eq!(reads(0), 1); // open fd skips reading map and calling unlink
    assert_eq!(reads(1), 2); // mapped file skips reading path and calling unlink
    assert!(evidence.calls()[0].calls.is_empty());
    assert!(evidence.calls()[1].calls.is_empty());
    for p in &evidence.calls()[2..] {
        assert_eq!(p.calls.len(), 1);
        assert_eq!(p.calls[0].function, "unlink");
        assert!(!p.calls[0].guard_uses.is_empty());
    }
    let ignored = c.replace(
        "int r=unlink(cf->path);if(r == -1)return -1;",
        "unlink(cf->path);",
    );
    assert!(
        check_file_delete_source(
            &C11TranslationUnit {
                source: &ignored,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION
        )
        .is_err()
    );
    for (i, _) in DELETE_CONTRACT.match_indices("ensures normal ") {
        let j = i + DELETE_CONTRACT[i..].find(';').unwrap();
        for predicate in ["false", "unknown_delete_outcome"] {
            let mutated = format!(
                "{}ensures normal {predicate}{}",
                &DELETE_CONTRACT[..i],
                &DELETE_CONTRACT[j..]
            );
            assert!(check(evidence.source(), &mutated).is_err());
        }
    }
}
