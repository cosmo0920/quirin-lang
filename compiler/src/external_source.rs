//! Source-derived profiles for pinned external-library custody summaries.

use std::collections::BTreeSet;

use crate::external_integration::{
    AuditedLibrary, LibraryMechanismSummary, LibrarySummaryProfile, PINNED_FLUENT_BIT_REVISION,
};

const LIBRDKAFKA_FILES: &[&str] = &[
    "lib/librdkafka-2.15.0/src/rd.h",
    "lib/librdkafka-2.15.0/src/rdkafka_queue.h",
    "lib/librdkafka-2.15.0/src/rdkafka_queue.c",
];
const CFL_FILES: &[&str] = &["lib/cfl/include/cfl/cfl_list.h", "lib/cfl/src/cfl_arena.c"];
const CHUNK_IO_FILES: &[&str] = &[
    "lib/chunkio/src/chunkio.c",
    "lib/chunkio/src/cio_stream.c",
    "lib/chunkio/src/cio_chunk.c",
];

pub use qcore_schemas::identity::ExternalSourceBundle;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySourceCertificate {
    pub profile: LibrarySummaryProfile,
    pub checked_files: BTreeSet<String>,
    pub checked_contracts: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LibrarySourceDiagnostic {
    SourceRevisionMismatch,
    MissingFile(String),
    ContractMismatch { file: String, contract: String },
}

#[must_use]
pub const fn required_library_source_paths(library: AuditedLibrary) -> &'static [&'static str] {
    match library {
        AuditedLibrary::Librdkafka2150 => LIBRDKAFKA_FILES,
        AuditedLibrary::Cfl => CFL_FILES,
        AuditedLibrary::ChunkIo => CHUNK_IO_FILES,
    }
}

pub fn derive_library_summary_profile(
    library: AuditedLibrary,
    source: &ExternalSourceBundle,
) -> Result<LibrarySourceCertificate, Vec<LibrarySourceDiagnostic>> {
    let mut diagnostics = BTreeSet::new();
    if source.source_revision != PINNED_FLUENT_BIT_REVISION {
        diagnostics.insert(LibrarySourceDiagnostic::SourceRevisionMismatch);
    }
    for path in required_library_source_paths(library) {
        if !source.files.contains_key(*path) {
            diagnostics.insert(LibrarySourceDiagnostic::MissingFile((*path).to_owned()));
        }
    }

    let contracts = match library {
        AuditedLibrary::Librdkafka2150 => librdkafka_contracts(),
        AuditedLibrary::Cfl => cfl_contracts(),
        AuditedLibrary::ChunkIo => chunk_io_contracts(),
    };
    let mut summaries = BTreeSet::new();
    let mut checked_contracts = BTreeSet::new();
    for contract in contracts {
        let Some(contents) = source.files.get(contract.file) else {
            continue;
        };
        if contract
            .fragments
            .iter()
            .all(|fragment| contents.contains(fragment))
        {
            summaries.extend(contract.summaries);
            checked_contracts.insert(contract.name.to_owned());
        } else {
            diagnostics.insert(LibrarySourceDiagnostic::ContractMismatch {
                file: contract.file.to_owned(),
                contract: contract.name.to_owned(),
            });
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics.into_iter().collect());
    }

    Ok(LibrarySourceCertificate {
        profile: LibrarySummaryProfile {
            source_revision: source.source_revision.clone(),
            library,
            summaries,
        },
        checked_files: required_library_source_paths(library)
            .iter()
            .map(|path| (*path).to_owned())
            .collect(),
        checked_contracts,
    })
}

struct SourceContract {
    name: &'static str,
    file: &'static str,
    fragments: &'static [&'static str],
    summaries: &'static [LibraryMechanismSummary],
}

fn librdkafka_contracts() -> Vec<SourceContract> {
    vec![
        SourceContract {
            name: "librdkafka-refcount",
            file: LIBRDKAFKA_FILES[0],
            fragments: &[
                "static RD_INLINE RD_UNUSED int rd_refcnt_add0",
                "static RD_INLINE RD_UNUSED int rd_refcnt_sub0",
                "r = rd_atomic32_sub(R, 1);",
                "if (r < 0)",
                "#define rd_refcnt_destroywrapper",
                "if (rd_refcnt_sub(REFCNT) > 0)",
                "DESTROY_CALL;",
            ],
            summaries: &[LibraryMechanismSummary::ReferenceCount],
        },
        SourceContract {
            name: "librdkafka-typed-queue",
            file: LIBRDKAFKA_FILES[1],
            fragments: &[
                "rd_kafka_q_enq0",
                "TAILQ_INSERT_TAIL(&rkq->rkq_q, rko, rko_link)",
                "rd_kafka_q_deq0",
                "TAILQ_REMOVE(&rkq->rkq_q, rko, rko_link)",
                "mtx_lock(&rkq->rkq_lock);",
                "mtx_unlock(&rkq->rkq_lock);",
                "@locality any thread.",
            ],
            summaries: &[
                LibraryMechanismSummary::IntrusiveList,
                LibraryMechanismSummary::TypedQueue,
                LibraryMechanismSummary::ThreadHandoff,
            ],
        },
        SourceContract {
            name: "librdkafka-queue-destructor",
            file: LIBRDKAFKA_FILES[2],
            fragments: &[
                "void rd_kafka_q_destroy_final",
                "rd_kafka_q_purge0(rkq, 0 /*no-lock*/);",
                "mtx_destroy(&rkq->rkq_lock);",
                "cnd_destroy(&rkq->rkq_cond);",
            ],
            summaries: &[LibraryMechanismSummary::CascadingDestructor],
        },
    ]
}

fn cfl_contracts() -> Vec<SourceContract> {
    vec![
        SourceContract {
            name: "cfl-intrusive-list",
            file: CFL_FILES[0],
            fragments: &[
                "static inline void cfl_list_del",
                "__cfl_list_del(entry->prev, entry->next);",
                "entry->prev = NULL;",
                "entry->next = NULL;",
                "static inline void cfl_list_add",
                "__cfl_list_add(_new, head->prev, head);",
            ],
            summaries: &[LibraryMechanismSummary::IntrusiveList],
        },
        SourceContract {
            name: "cfl-arena-lifetime",
            file: CFL_FILES[1],
            fragments: &[
                "struct cfl_arena *cfl_arena_create_with_options",
                "arena = malloc_fn(allocator_context, sizeof(struct cfl_arena));",
                "void cfl_arena_destroy(struct cfl_arena *arena)",
                "arena_external_destroy(arena);",
                "arena_chunks_destroy(arena);",
                "arena->free_fn(arena->allocator_context, arena);",
            ],
            summaries: &[
                LibraryMechanismSummary::Arena,
                LibraryMechanismSummary::CascadingDestructor,
            ],
        },
    ]
}

fn chunk_io_contracts() -> Vec<SourceContract> {
    vec![
        SourceContract {
            name: "chunkio-context-destructor",
            file: CHUNK_IO_FILES[0],
            fragments: &[
                "void cio_destroy(struct cio_ctx *ctx)",
                "cio_stream_destroy_all(ctx);",
                "free(ctx);",
            ],
            summaries: &[LibraryMechanismSummary::CascadingDestructor],
        },
        SourceContract {
            name: "chunkio-stream-destructor",
            file: CHUNK_IO_FILES[1],
            fragments: &[
                "void cio_stream_destroy(struct cio_stream *st)",
                "cio_chunk_close_stream(st);",
                "mk_list_del(&st->_head);",
                "free(st->name);",
                "free(st);",
            ],
            summaries: &[
                LibraryMechanismSummary::IntrusiveList,
                LibraryMechanismSummary::CascadingDestructor,
            ],
        },
        SourceContract {
            name: "chunkio-chunk-destructor",
            file: CHUNK_IO_FILES[2],
            fragments: &[
                "void cio_chunk_close(struct cio_chunk *ch, int delete)",
                "cio_memfs_close(ch);",
                "cio_file_close(ch, delete);",
                "mk_list_del(&ch->_head);",
                "mk_list_del(&ch->_state_head);",
                "free(ch->name);",
                "cio_chunk_counter_total_sub(ctx);",
            ],
            summaries: &[
                LibraryMechanismSummary::IntrusiveList,
                LibraryMechanismSummary::CascadingDestructor,
            ],
        },
    ]
}

#[cfg(test)]
#[path = "../tests/unit/external_source.rs"]
mod tests;
