use super::*;
use crate::verifier_plugins::chunkio::{
    import_chunk_frame_helper,
    teardown_source::{
        check_counter_sources, check_list_unlink_source, check_memory_backend_source,
    },
};
pub(crate) const FIXTURE:&str="
#define CIO_STORE_MEM 1
#define CIO_STORE_FS 0
typedef unsigned long qcore_test_size_t;
struct mk_list { struct mk_list *prev; struct mk_list *next; };
struct cio_ctx {qcore_test_size_t total_chunks; qcore_test_size_t total_chunks_up;};
struct cio_stream {int type;};
struct cio_chunk {void *backend; char *name; struct cio_ctx *ctx; struct cio_stream *st; struct mk_list _head; struct mk_list _state_head; int lock; int error_n;};
struct cio_memfs {char *name; char *buf_data; char *meta_data;};
void free(void *);
void cio_file_close(struct cio_chunk *,int);
void cio_error_reset(struct cio_chunk *ch){ch->error_n=0;}
void cio_memfs_close(struct cio_chunk *ch){struct cio_memfs *mf=ch->backend;if(!mf)return;free(mf->name);free(mf->buf_data);free(mf->meta_data);free(mf);}
void __mk_list_del(struct mk_list *prev,struct mk_list *next){prev->next=next;next->prev=prev;}
void mk_list_del(struct mk_list *entry){__mk_list_del(entry->prev,entry->next);entry->prev=(void *)0;entry->next=(void *)0;}
qcore_test_size_t cio_chunk_counter_total_add(struct cio_ctx *ctx){ctx->total_chunks++;return ctx->total_chunks;}
qcore_test_size_t cio_chunk_counter_total_sub(struct cio_ctx *ctx){ctx->total_chunks--;return ctx->total_chunks;}
qcore_test_size_t cio_chunk_counter_total_up_add(struct cio_ctx *ctx){ctx->total_chunks_up++;return ctx->total_chunks_up;}
qcore_test_size_t cio_chunk_counter_total_up_sub(struct cio_ctx *ctx){ctx->total_chunks_up--;return ctx->total_chunks_up;}
void cio_chunk_close(struct cio_chunk *ch,int delete){
int type;struct cio_ctx *ctx;
if(!ch){return;}
cio_error_reset(ch);
ctx=ch->ctx;type=ch->st->type;
if(type==CIO_STORE_MEM){cio_memfs_close(ch);}
else if(type==CIO_STORE_FS){cio_file_close(ch,delete);}
mk_list_del(&ch->_head);mk_list_del(&ch->_state_head);
free(ch->name);free(ch);
cio_chunk_counter_total_sub(ctx);
}";
pub(crate) fn dependencies() -> MemoryCloseDependencies {
    let unit = C11TranslationUnit {
        source: FIXTURE,
        ..Default::default()
    };
    MemoryCloseDependencies {
        reset: import_chunk_frame_helper(&unit, PINNED_FLUENT_BIT_REVISION, "cio_error_reset")
            .unwrap(),
        backend: check_memory_backend_source(&unit, PINNED_FLUENT_BIT_REVISION).unwrap(),
        unlink: check_list_unlink_source(&unit, PINNED_FLUENT_BIT_REVISION).unwrap(),
        counter: check_counter_sources(&unit, PINNED_FLUENT_BIT_REVISION)
            .unwrap()
            .remove(1),
    }
}
#[test]
fn memory_close_composes_null_and_memory_paths_into_concrete_chunk_identity() {
    if !crate::test_support::clang_available() {
        return;
    }
    let cert = check_memory_close_source(
        &C11TranslationUnit {
            source: FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &dependencies(),
    )
    .unwrap();
    assert_eq!(cert.checked_paths().len(), 2);
    assert_eq!(cert.uncovered_paths().len(), 2);
    use crate::chunk_lifecycle::{
        ChunkBackend, ChunkOperation as C, ChunkResidency, execute_chunk_prefix,
    };
    let mut ops = vec![
        C::CreateContext { context: 7 },
        C::CreateStream {
            context: 7,
            stream: 8,
        },
        C::OpenChunk {
            context: 7,
            stream: 8,
            chunk: 9,
            backend: ChunkBackend::Memory,
            residency: ChunkResidency::Up,
        },
    ];
    let state = execute_chunk_prefix(&ops).unwrap().final_state;
    assert!(cert.instantiate(&state, None, false).unwrap().is_empty());
    let close = cert.instantiate(&state, Some(9), true).unwrap();
    assert_eq!(
        close,
        vec![C::CloseChunk {
            chunk: 9,
            delete: true
        }]
    );
    ops.extend(close);
    let after = execute_chunk_prefix(&ops).unwrap().final_state;
    let chunk = &after.chunks[&9];
    assert!(
        !chunk.live
            && !chunk.primary_linked
            && !chunk.residency_linked
            && !chunk.backend_object_live
            && !chunk.memory_buffer_live
            && !chunk.metadata_buffer_live
            && !chunk.name_live
    );
    assert_eq!(after.contexts[&7].chunk_total, 0);
    assert!(after.streams[&8].chunks.is_empty());
    assert!(cert.instantiate(&after, Some(9), false).is_err());
    for mutation in 0..6 {
        let mut bad = state.clone();
        match mutation {
            0 => bad.chunks.get_mut(&9).unwrap().busy = true,
            1 => bad.chunks.get_mut(&9).unwrap().backend = ChunkBackend::File,
            2 => bad.chunks.get_mut(&9).unwrap().primary_linked = false,
            3 => bad.contexts.get_mut(&7).unwrap().chunk_total = 0,
            4 => bad.chunks.get_mut(&9).unwrap().transaction_active = true,
            _ => bad.chunks.get_mut(&9).unwrap().context = 99,
        };
        assert!(cert.instantiate(&bad, Some(9), false).is_err());
    }
}
#[test]
fn close_rejects_missing_reordered_or_unbound_dependency_effects() {
    if !crate::test_support::clang_available() {
        return;
    }
    let deps = dependencies();
    for (old, new) in [
        (
            "if(!ch){return;}",
            "if(!ch){return;} if ((!ch)==0) { return; }",
        ),
        ("struct mk_list _head;", "struct mk_list *_head;"),
        ("mk_list_del(&ch->_state_head);", "mk_list_del(&ch->_head);"),
        (
            "cio_chunk_counter_total_sub(ctx);",
            "cio_chunk_counter_total_sub(ch->ctx);",
        ),
        (
            "cio_chunk_counter_total_sub(ctx);",
            "cio_chunk_counter_total_up_sub(ctx);",
        ),
        ("free(ch->name);free(ch);", "free(ch);free(ch->name);"),
        (
            "if(type==CIO_STORE_MEM){cio_memfs_close(ch);}",
            "if(type==CIO_STORE_MEM){}",
        ),
        ("free(ch->name);free(ch);", "free(ch->name);"),
        (
            "ctx=ch->ctx;type=ch->st->type;",
            "ctx=ch->ctx;while(0){}type=ch->st->type;",
        ),
    ] {
        let source = FIXTURE.replace(old, new);
        assert!(
            check_memory_close_source(
                &C11TranslationUnit {
                    source: &source,
                    ..Default::default()
                },
                PINNED_FLUENT_BIT_REVISION,
                &deps
            )
            .is_err(),
            "accepted {new}"
        );
    }
    let changed = FIXTURE.replace("#define CIO_STORE_MEM 1", "#define CIO_STORE_MEM 4");
    assert!(
        check_memory_close_source(
            &C11TranslationUnit {
                source: &changed,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
            &deps
        )
        .is_ok()
    );
}
#[test]
fn close_contract_clauses_and_dependency_role_are_rechecked() {
    if !crate::test_support::clang_available() {
        return;
    }
    let deps = dependencies();
    let cert = check_memory_close_source(
        &C11TranslationUnit {
            source: FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &deps,
    )
    .unwrap();
    for (old, new) in [
        ("requires context_counter_covers_live_chunk;", ""),
        ("requires context_and_stream_outlive_chunk;", ""),
        (
            "requires embedded_links_have_distinct_stream_memberships;",
            "",
        ),
        ("ptr<mut, Chunk>", "ptr<mut, Other>"),
        ("-> unit", "-> i32"),
        ("requires exclusive_access;", "requires unknown;"),
        (
            "ensures normal chunk_was_nonnull or call_count == 0;",
            "ensures error chunk_was_nonnull or call_count == 0;",
        ),
    ] {
        assert!(
            check(
                &cert.source,
                &cert.constant_probe,
                &deps,
                &CONTRACT.replace(old, new)
            )
            .is_err()
        );
    }
    for (i, _) in CONTRACT.match_indices("ensures normal ") {
        let j = i + CONTRACT[i..].find(';').unwrap();
        for predicate in ["false", "unsupported_predicate"] {
            let changed = format!(
                "{}ensures normal {predicate}{}",
                &CONTRACT[..i],
                &CONTRACT[j..]
            );
            assert!(check(&cert.source, &cert.constant_probe, &deps, &changed).is_err());
        }
    }
    let mut wrong = deps.clone();
    wrong.counter = check_counter_sources(
        &C11TranslationUnit {
            source: FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap()
    .remove(3);
    assert!(check(&cert.source, &cert.constant_probe, &wrong, CONTRACT).is_err());
}
