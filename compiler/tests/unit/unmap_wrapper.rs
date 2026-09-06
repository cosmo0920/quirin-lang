use super::*;
use crate::typed_source::{SourceImportProfile, SourceResult, import_source_functions};
const SOURCE: &str = r#"
typedef unsigned long size_t;
struct cio_ctx {size_t total_chunks_up; size_t total_chunks;};
struct cio_stream {char *name;};
struct cio_chunk {void *backend; struct cio_ctx *ctx; struct cio_stream *st; char *name;};
struct cio_file {char *map; size_t alloc_size; size_t data_size; int synced; int map_truncated_warned; char *path;};
int munmap(void *,size_t); int *__errno_location(void); int cio_errno_print(int,const char *,int);
int cio_file_sync(struct cio_chunk *);int cio_file_native_close(struct cio_file *);void free(void *);
void cio_log_print(void *,int,const char *,int,const char *,...);
int cio_file_native_unmap(struct cio_file *cf){if(!cf)return -1;if(!cf->map)return 0;int r=munmap(cf->map,cf->alloc_size);if(r!=0){cio_errno_print(*__errno_location(),"test.c",1);return -1;}cf->map=(void*)0;cf->alloc_size=0;cf->map_truncated_warned=0;return 0;}
size_t cio_chunk_counter_total_up_sub(struct cio_ctx *ctx){ctx->total_chunks_up--;return ctx->total_chunks_up;}
size_t cio_chunk_counter_total_sub(struct cio_ctx *ctx){ctx->total_chunks--;return ctx->total_chunks;}
static int munmap_file(struct cio_ctx *ctx, struct cio_chunk *ch){
struct cio_file *cf=ch->backend;
if(!cf)return -1;
if(!cf->map)return -1;
int ret;
if(cf->synced==0){ret=cio_file_sync(ch);if(ret==-1)cio_log_print(ch->ctx,1,"test.c",1,"[cio file] error syncing file at %s:%s",ch->st->name,ch->name);}
ret=cio_file_native_unmap(cf);
if(ret!=0)return -1;
cf->data_size=0;cf->alloc_size=0;cio_chunk_counter_total_up_sub(ctx);return 0;
}
void cio_file_close(struct cio_chunk *ch,int delete){struct cio_file *cf=ch->backend;if(!cf)return;munmap_file(ch->ctx,ch);cio_file_native_close(cf);free(cf->path);free(cf);}
"#;
fn import(c: &str, function: &str) -> SourceSummary {
    let sources = import_source_functions(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &[function],
        &SourceImportProfile {
            records: &["cio_ctx", "cio_stream", "cio_chunk", "cio_file"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )
    .unwrap();
    let SourceResult::Summary(s) = &sources[0] else {
        panic!("{sources:?}")
    };
    s.clone()
}
fn dependencies() -> (CleanupCertificate, CounterCertificate) {
    let native = check_cleanup_outcomes(
        &import(SOURCE, "cio_file_native_unmap"),
        &file_outcomes::profile("cio_file_native_unmap").unwrap(),
        file_outcomes::CONTRACT,
    )
    .unwrap();
    let counter = check_counter_contract(
        &import(SOURCE, "cio_chunk_counter_total_up_sub"),
        COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )
    .unwrap();
    (native, counter)
}
#[test]
fn unmap_composes_native_outcomes_with_exact_up_counter_bounds() {
    if !crate::test_support::clang_available() {
        return;
    }
    let (native, counter) = dependencies();
    let cert = check_unmap_wrapper_source(
        &C11TranslationUnit {
            source: SOURCE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &native,
        &counter,
    )
    .unwrap();
    assert_eq!(cert.cases().len(), 8);
    assert_eq!(
        cert.cases()
            .iter()
            .filter(|c| c.counter_decrements == 1)
            .count(),
        3
    );
    for (i, c) in cert.cases().iter().enumerate() {
        if c.native_result == Some(0) {
            assert_eq!(cert.counter_after(i, 3, 10).unwrap(), 2);
            assert!(cert.counter_after(i, 0, 10).is_err());
            assert_eq!(c.wrapper_writes, ["data_size", "alloc_size"]);
        } else {
            assert_eq!(cert.counter_after(i, 3, 10).unwrap(), 3);
            assert_eq!(cert.counter_after(i, 0, 10).unwrap(), 0);
            assert!(c.wrapper_writes.is_empty());
        }
        assert!(cert.counter_after(i, 11, 10).is_err());
    }
    assert!(cert.counter_after(8, 3, 10).is_err());
    // Failed sync still attempts native unmap; only native success decrements.
    assert_eq!(
        cert.cases()
            .iter()
            .filter(|c| c.sync_result == Some(-1) && c.native_result.is_some())
            .count(),
        2
    );
}
#[test]
fn unmap_rejects_wrong_context_effect_order_and_outcome_mutations() {
    if !crate::test_support::clang_available() {
        return;
    }
    let (native, counter) = dependencies();
    for (old, new) in [
        ("if(ret!=0)return -1;", "if(ret==0)return -1;"),
        (
            "if(ret!=0)return -1;",
            "if(ret!=0){cio_chunk_counter_total_up_sub(ctx);return -1;}",
        ),
        (
            "if(ret!=0)return -1;",
            "if(ret!=0){cf->data_size=0;return -1;}",
        ),
        ("cio_file_sync(ch)", "cio_file_sync((void*)0)"),
        (
            "ret=cio_file_native_unmap(cf)",
            "ret=cio_file_native_unmap((void*)0)",
        ),
        (
            "cio_chunk_counter_total_up_sub(ctx);return 0;",
            "cio_chunk_counter_total_sub(ctx);return 0;",
        ),
        (
            "cio_chunk_counter_total_up_sub(ctx);return 0;",
            "cio_chunk_counter_total_up_sub((void*)0);return 0;",
        ),
        ("cf->data_size=0;cf->alloc_size=0;", "cf->data_size=0;"),
        (
            "cf->data_size=0;cf->alloc_size=0;",
            "cf->data_size=0;cf->data_size=0;",
        ),
        (
            "cf->data_size=0;cf->alloc_size=0;",
            "cio_chunk_counter_total_up_sub(ctx);cf->data_size=0;cf->alloc_size=0;",
        ),
        ("if(!cf->map)return -1;", "if(!cf->map)return 0;"),
        ("if(cf->synced==0)", "if(cf->synced==1)"),
        ("ch->st->name,ch->name", "ch->name,ch->st->name"),
        (
            "cio_chunk_counter_total_up_sub(ctx);return 0;",
            "cio_chunk_counter_total_up_sub(ctx);free(cf);return 0;",
        ),
    ] {
        assert!(
            check(
                &import(&SOURCE.replace(old, new), "munmap_file"),
                &native,
                &counter,
                CONTRACT
            )
            .is_err(),
            "accepted {old} -> {new}"
        );
    }
    let wrong_counter = check_counter_contract(
        &import(SOURCE, "cio_chunk_counter_total_sub"),
        COUNTER_CONTRACT,
        "struct cio_ctx *",
        "ChunkContext",
    )
    .unwrap();
    assert!(
        check(
            &import(SOURCE, "munmap_file"),
            &native,
            &wrong_counter,
            CONTRACT
        )
        .is_err()
    );
}
#[test]
fn unmap_native_contract_clauses_are_executed() {
    if !crate::test_support::clang_available() {
        return;
    }
    let (native, counter) = dependencies();
    let source = import(SOURCE, "munmap_file");
    for (i, _) in CONTRACT.match_indices("ensures normal ") {
        let j = i + CONTRACT[i..].find(';').unwrap();
        for pred in ["false", "unbound_unmap_fact"] {
            let mutated = format!("{}ensures normal {pred}{}", &CONTRACT[..i], &CONTRACT[j..]);
            assert!(check(&source, &native, &counter, &mutated).is_err());
        }
    }
}
