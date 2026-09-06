//! Classify the complete nonnull, non-memory, non-filesystem close domain.
//! This is conditional partial-release evidence, never a Safe/Unsafe verdict.
use super::{
    PINNED_FLUENT_BIT_REVISION,
    filesystem_chunk_close::FilesystemChunkCloseEvidence,
    memory_close::{self, MemoryCloseCertificate},
};
use crate::typed_source::SourceSummary;
use serde::Serialize;

pub const CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/unsupported_close.qcore");

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnsupportedCloseEvidence {
    memory_scope: MemoryCloseCertificate,
    filesystem_kind_probe: SourceSummary,
    contract_source: String,
    excluded_kinds: [i32; 2],
    source_path: usize,
    property: String,
    semantic_profile: String,
    scope: String,
    bounds: String,
    evidence_version: String,
    /// Explicit result boundary: no constructor reachability or backend layout
    /// is established for this class, even though the outer releases are known.
    status: String,
    reason: String,
}

impl UnsupportedCloseEvidence {
    pub fn source_path(&self) -> usize {
        self.source_path
    }
    pub fn excluded_kinds(&self) -> [i32; 2] {
        self.excluded_kinds
    }
    pub fn require_nested_discharge(&self) -> Result<(), String> {
        Err(format!("Unknown: {}", self.reason))
    }
}

pub fn check_unsupported_close_source(
    filesystem: &FilesystemChunkCloseEvidence,
) -> Result<UnsupportedCloseEvidence, String> {
    let filesystem = filesystem.recheck()?;
    check(
        filesystem.memory_scope(),
        filesystem.filesystem_kind_probe(),
        CONTRACT,
    )
}

fn check(
    memory: &MemoryCloseCertificate,
    probe: &SourceSummary,
    contract: &str,
) -> Result<UnsupportedCloseEvidence, String> {
    let memory = memory.recheck()?;
    if probe.source_revision() != PINNED_FLUENT_BIT_REVISION
        || probe.function() != "qcore_cio_filesystem_store_value"
    {
        return Err("unsupported close filesystem probe identity mismatch".into());
    }
    let kinds = [memory.memory_kind()?, memory_close::probe_literal(probe)?];
    if kinds[0] == kinds[1] {
        return Err("unsupported close backend discriminants overlap".into());
    }
    let source_path = memory_close::unsupported_path(&memory, kinds[1], false)?;
    if memory_close::unsupported_path(&memory, kinds[1], true)? != source_path {
        return Err("unsupported close unexpectedly depends on the delete argument".into());
    }
    crate::contract_observation::check_contract_facts(
        contract,
        "unsupported_chunk_close",
        &[
            (
                "nonnull_live_chunk_and_stream_with_unrecognized_int_backend",
                1,
            ),
            (
                "nullable_name_has_owned_provenance_disjoint_from_parent_and_backend",
                1,
            ),
            ("live_exclusive_distinct_reciprocal_list_memberships", 1),
            (
                "saved_context_outlives_chunk_and_total_counter_is_positive",
                1,
            ),
            ("reset_list_counter_source_abi_and_libc_free_bindings", 1),
            ("delete_argument_is_boolean_and_no_wrapper_obligations", 1),
            ("backend_calls", 0),
            ("primary_unlinked", 1),
            ("state_unlinked", 1),
            ("name_release_attempts", 1),
            ("chunk_released", 1),
            ("total_counter_decrements", 1),
            ("up_counter_decrements", 0),
            ("call_count", 6),
            ("backend_custody_unresolved", 1),
            ("nested_discharge_permitted", 0),
            ("deletion_established", 0),
            ("constructor_reachability_established", 0),
        ],
    )?;
    Ok(UnsupportedCloseEvidence {
        memory_scope: memory,
        filesystem_kind_probe: probe.clone(),
        contract_source: contract.into(),
        excluded_kinds: kinds,
        source_path,
        property: "nested backend custody after outer chunk release".into(),
        semantic_profile: "pinned typed C11 close path with symbolic int backend class".into(),
        scope: "nonnull cio_chunk_close with backend unequal to both source-probed constants".into(),
        bounds: "all int backend values in the excluded-constant class; delete in {0,1}; no caller reachability inference".into(),
        evidence_version: "cycle19-unsupported-close-v1".into(),
        status: "Unknown".into(),
        reason: "unrecognized stream backend skips backend cleanup; backend custody and constructor reachability remain unresolved".into(),
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/unsupported_close.rs"]
mod tests;
