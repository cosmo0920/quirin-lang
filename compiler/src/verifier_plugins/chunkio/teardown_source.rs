//! Source boundary for the nested ChunkIO destructor chain. Only the bounded
//! memory backend currently has executable field custody; other stages remain open.
use super::PINNED_FLUENT_BIT_REVISION;
use crate::field_destructor::{FieldDestructorCertificate, check_field_destructor};
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::{SourceImportProfile, SourceResult, import_source_functions};

pub const MEMORY_BACKEND_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/memory_backend.qcore");

pub fn check_memory_backend_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
) -> Result<FieldDestructorCertificate, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("ChunkIO teardown revision mismatch".into());
    }
    let results = import_source_functions(
        unit,
        revision,
        &["cio_memfs_close"],
        &SourceImportProfile {
            records: &["cio_chunk", "cio_memfs"],
            sizeof_types: &[],
            signature_constraints: &[("cio_memfs_close", "void", &["struct cio_chunk *"])],
            restricted_calls: &["free"],
        },
    )?;
    match &results[0] {
        SourceResult::Summary(summary) => check_field_destructor(
            summary,
            MEMORY_BACKEND_CONTRACT,
            "struct cio_chunk *",
            "Chunk",
        ),
        SourceResult::Unknown { reason, .. } => Err(reason.clone()),
    }
}

pub const COUNTER_CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/counters.qcore");

pub fn check_counter_sources(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
) -> Result<Vec<crate::counter_contract::CounterCertificate>, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("ChunkIO counter revision mismatch".into());
    }
    let results = import_source_functions(
        unit,
        revision,
        &[
            "cio_chunk_counter_total_add",
            "cio_chunk_counter_total_sub",
            "cio_chunk_counter_total_up_add",
            "cio_chunk_counter_total_up_sub",
        ],
        &SourceImportProfile {
            records: &["cio_ctx"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    results
        .iter()
        .map(|result| match result {
            SourceResult::Summary(summary) => crate::counter_contract::check_counter_contract(
                summary,
                COUNTER_CONTRACT,
                "struct cio_ctx *",
                "ChunkContext",
            ),
            SourceResult::Unknown { function, reason } => Err(format!("{function}: {reason}")),
        })
        .collect()
}
