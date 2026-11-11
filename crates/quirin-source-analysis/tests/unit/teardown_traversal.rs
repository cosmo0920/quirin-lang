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

pub(crate) const FIXTURE: &str = r#"
#include <stddef.h>
struct mk_list { struct mk_list *prev, *next; };
struct cio_ctx { struct mk_list streams; };
struct cio_stream { struct mk_list _head, chunks, chunks_up; char *name; };
struct cio_chunk { struct mk_list _head, _state_head; };
void cio_chunk_close(struct cio_chunk *, int);
void cio_stream_destroy(struct cio_stream *);
#define each(curr,n,head) for(curr=(head)->next,n=curr->next;curr!=(head);curr=n,n=curr->next)
#define entry(ptr,type,member) ({const typeof(((type*)0)->member) *__mptr=(ptr);(type*)((char*)__mptr - offsetof(type,member));})
void cio_chunk_close_stream(struct cio_stream *st) {
    struct mk_list *tmp;
    struct mk_list *head;
    struct cio_chunk *ch;
    each(head,tmp,&st->chunks) {
        ch=entry(head,struct cio_chunk,_head);
        cio_chunk_close(ch,0);
    }
}
void cio_stream_destroy_all(struct cio_ctx *ctx) {
    struct mk_list *tmp;
    struct mk_list *head;
    struct cio_stream *st;
    if (!ctx) { return; }
    each(head,tmp,&ctx->streams) {
        st=entry(head,struct cio_stream,_head);
        cio_stream_destroy(st);
    }
}
"#;
fn check(source: &str, kind: TraversalKind) -> Result<TraversalSource, String> {
    check_traversal_source(
        &C11TranslationUnit {
            source,
            dialect: crate::legacy_c::CCompilerDialect::Gnu11,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        kind,
    )
}

#[test]
fn traversals_bind_complete_typed_bodies_and_local_identities() {
    if !crate::test_support::clang_available() {
        return;
    }
    for kind in [TraversalKind::Chunks, TraversalKind::Streams] {
        let evidence = check(FIXTURE, kind).unwrap();
        assert_eq!(evidence.kind(), kind);
        assert_eq!(
            evidence.source().source_revision(),
            PINNED_FLUENT_BIT_REVISION
        );
        check(
            &FIXTURE
                .replace("tmp", "saved_next")
                .replace("head;", "head; /* identity-preserving comment */"),
            kind,
        )
        .unwrap();
    }
}

#[test]
fn traversal_rejects_skips_stale_next_wrong_container_and_hidden_macro_changes() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        ("curr=n,n=curr->next", "curr=curr->next,n=curr->next"),
        ("n=curr->next", "n=curr->next->next"),
        ("curr!=(head)", "curr==(head)"),
        ("&st->chunks)", "&st->chunks_up)"),
        ("cio_chunk_close(ch,0);", "cio_chunk_close(ch,1);"),
        ("cio_chunk_close(ch,0);", "cio_chunk_close(ch,0);break;"),
        (
            "cio_chunk_close(ch,0);",
            "cio_chunk_close(ch,0);cio_chunk_close(ch,0);",
        ),
        ("cio_chunk_close(ch,0);", "cio_chunk_close((void*)0,0);"),
        (
            "entry(head,struct cio_chunk,_head)",
            "entry(tmp,struct cio_chunk,_head)",
        ),
        (
            "entry(head,struct cio_chunk,_head)",
            "entry(head,struct cio_chunk,_state_head)",
        ),
        ("offsetof(type,member)", "offsetof(type,member)+1"),
        (
            "struct mk_list *prev, *next",
            "struct mk_list *prev; struct mk_list *volatile next",
        ),
    ] {
        assert!(FIXTURE.contains(old));
        assert!(
            check(&FIXTURE.replace(old, new), TraversalKind::Chunks).is_err(),
            "accepted {new}"
        );
    }
    for (old, new) in [
        ("cio_stream_destroy(st);", "cio_stream_destroy(st);return;"),
        ("cio_stream_destroy(st);", "cio_stream_destroy((void*)0);"),
        ("if (!ctx) { return; }", "if (ctx) { return; }"),
    ] {
        assert!(
            check(&FIXTURE.replace(old, new), TraversalKind::Streams).is_err(),
            "accepted {new}"
        );
    }
}

#[test]
fn traversal_policy_and_revision_fail_closed() {
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
        check_traversal_source(
            &C11TranslationUnit {
                source: FIXTURE,
                ..Default::default()
            },
            "wrong",
            TraversalKind::Chunks
        )
        .is_err()
    );
}

#[test]
fn offsetof_token_boundaries_cannot_alias_a_different_record() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = format!(
        "typedef struct {{ int padding; struct {{ void *p; }} _head; }} structcio_chunk;\n{FIXTURE}"
    );
    let bad = source.replace("offsetof(type,member)", "offsetof(structcio_chunk,member)");
    assert!(check(&bad, TraversalKind::Chunks).is_err());
}
