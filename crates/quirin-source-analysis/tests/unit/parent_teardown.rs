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

pub(crate) const RECORDS: &str = r#"
/* Syntax-only fixture; no target libc headers or linked allocator required. */
void free(void *);
struct mk_list { struct mk_list *prev, *next; };
struct cio_options { char *user, *group, *chmod, *root_path; };
struct cio_ctx { struct cio_options options; void *processed_user, *processed_group; struct mk_list streams; };
struct cio_stream { struct mk_list _head; char *name; };
void cio_chunk_close_stream(struct cio_stream *);
void mk_list_del(struct mk_list *);
void cio_stream_destroy_all(struct cio_ctx *);
"#;

pub(crate) fn fixture() -> String {
    format!(
        "{RECORDS}\n{}\n{}",
        STREAM.replace("qcore_source_reference", "cio_stream_destroy"),
        CONTEXT.replace("qcore_source_reference", "cio_destroy")
    )
}
fn check(source: &str, kind: ParentKind) -> Result<ParentSource, String> {
    check_parent_source(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        kind,
    )
}
#[test]
fn parent_source_binds_null_guard_nested_call_and_final_release() {
    if !crate::test_support::clang_available() {
        return;
    }
    for kind in [ParentKind::Stream, ParentKind::Context] {
        let e = check(&fixture(), kind).unwrap();
        assert_eq!(e.kind(), kind);
        assert_eq!(e.source().source_revision(), PINNED_FLUENT_BIT_REVISION);
    }
}
#[test]
fn parent_source_rejects_early_free_wrong_fields_and_missing_children() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (kind, old, new) in [
        (
            ParentKind::Stream,
            "cio_chunk_close_stream(st);",
            "free(st);cio_chunk_close_stream(st);",
        ),
        (ParentKind::Stream, "cio_chunk_close_stream(st);", ""),
        (
            ParentKind::Stream,
            "cio_chunk_close_stream(st);",
            "cio_chunk_close_stream((void*)0);",
        ),
        (ParentKind::Stream, "mk_list_del(&st->_head);", ""),
        (
            ParentKind::Stream,
            "free(st->name);",
            "free(st->name);free(st->name);",
        ),
        (ParentKind::Stream, "if (!st)", "if (st)"),
        (
            ParentKind::Context,
            "cio_stream_destroy_all(ctx);",
            "cio_stream_destroy_all((void*)0);",
        ),
        (
            ParentKind::Context,
            "free(ctx->options.group);",
            "free(ctx->options.user);",
        ),
        (ParentKind::Context, "free(ctx->processed_user);", ""),
        (
            ParentKind::Context,
            "free(ctx);",
            "free(ctx);free(ctx->options.user);",
        ),
        (ParentKind::Context, "if (!ctx)", "if (ctx)"),
        (
            ParentKind::Context,
            "free(ctx->options.root_path);",
            "return;",
        ),
    ] {
        assert!(
            check(&fixture().replace(old, new), kind).is_err(),
            "accepted {new}"
        );
    }
}
#[test]
fn parent_policy_rejects_false_or_unknown_facts() {
    for clause in ["requires ", "ensures normal "] {
        for (i, _) in CONTRACT.match_indices(clause) {
            let end = i + CONTRACT[i..].find(';').unwrap();
            for predicate in ["false", "unknown_fact"] {
                let changed = format!("{}{clause}{predicate}{}", &CONTRACT[..i], &CONTRACT[end..]);
                assert!(check_policy(&changed).is_err());
            }
        }
    }
    assert!(
        check_parent_source(
            &C11TranslationUnit {
                source: &fixture(),
                ..Default::default()
            },
            "wrong",
            ParentKind::Context
        )
        .is_err()
    );
}

#[test]
fn parent_reference_cannot_share_a_corrupted_owned_field_representation() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (kind, old, new) in [
        (ParentKind::Stream, "char *name;", "long name;"),
        (
            ParentKind::Context,
            "void *processed_user, *processed_group;",
            "long processed_user, processed_group;",
        ),
        (
            ParentKind::Context,
            "char *user, *group, *chmod, *root_path;",
            "long user, group, chmod, root_path;",
        ),
    ] {
        assert!(check(&fixture().replace(old, new), kind).is_err());
    }
}
