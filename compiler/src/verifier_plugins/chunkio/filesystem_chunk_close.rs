//! Conditional filesystem arm of the enclosing chunk destructor. Allocation
//! release and list/counter updates retain the backend's residual OS obligations.
use super::{
    PINNED_FLUENT_BIT_REVISION,
    file_close::{DeletionOutcome, FileCloseDeletionEvidence, ResidualCustody},
    memory_close::{self, MemoryCloseCertificate, MemoryCloseDependencies},
};
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use serde::Serialize;
use std::collections::BTreeSet;

pub const CONTRACT: &str =
    include_str!("../../../../contract/qcore/chunkio/filesystem_chunk_close.qcore");
const PROBE: &str = "\nint qcore_cio_filesystem_store_value(void) { return CIO_STORE_FS; }\n";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FilesystemChunkCloseCase {
    pub source_path: usize,
    pub chunk_present: bool,
    pub delete: bool,
    /// Index into the dependency's deleting or non-deleting cases, selected by delete.
    pub backend_case: Option<usize>,
    pub backend_released: bool,
    pub path_released: bool,
    pub mapping: ResidualCustody,
    pub descriptor: ResidualCustody,
    pub deletion: Option<DeletionOutcome>,
    pub primary_unlinked: bool,
    pub state_unlinked: bool,
    pub name_released: bool,
    pub chunk_released: bool,
    pub total_counter_decrements: usize,
    pub up_counter_decrements: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FilesystemChunkCloseEvidence {
    memory_scope: MemoryCloseCertificate,
    backend: FileCloseDeletionEvidence,
    filesystem_kind_probe: SourceSummary,
    probe_source: String,
    contract_source: String,
    cases: Vec<FilesystemChunkCloseCase>,
    checked_paths: Vec<usize>,
    uncovered_paths: Vec<usize>,
    combined_uncovered_paths: Vec<usize>,
}
impl FilesystemChunkCloseEvidence {
    pub(super) fn recheck(&self) -> Result<Self, String> {
        check(
            &self.memory_scope,
            &self.filesystem_kind_probe,
            &self.backend,
            CONTRACT,
        )
    }
    pub(super) fn memory_scope(&self) -> &MemoryCloseCertificate {
        &self.memory_scope
    }
    pub fn cases(&self) -> &[FilesystemChunkCloseCase] {
        &self.cases
    }
    pub fn checked_paths(&self) -> &[usize] {
        &self.checked_paths
    }
    pub fn uncovered_paths(&self) -> &[usize] {
        &self.uncovered_paths
    }
    pub fn combined_uncovered_paths(&self) -> &[usize] {
        &self.combined_uncovered_paths
    }
    /// Conditional gate only; callers still bind entry custody and OS outcomes.
    pub fn require_nested_discharge(&self, case: usize) -> Result<(), String> {
        let case = self
            .cases
            .get(case)
            .ok_or("chunk close case out of range")?;
        if case.mapping != ResidualCustody::Absent || case.descriptor != ResidualCustody::Absent {
            return Err("Unknown: freeing the chunk retains unresolved OS custody".into());
        }
        Ok(())
    }
    pub fn require_successful_deletion(&self, case: usize) -> Result<(), String> {
        self.require_nested_discharge(case)?;
        let case = &self.cases[case];
        if !case.chunk_present
            || !case.delete
            || case.deletion != Some(DeletionOutcome::SyscallSucceeded)
        {
            return Err("Unknown: chunk release does not establish backing deletion".into());
        }
        Ok(())
    }
}

pub fn check_filesystem_chunk_close_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    dependencies: &MemoryCloseDependencies,
    backend: &FileCloseDeletionEvidence,
) -> Result<FilesystemChunkCloseEvidence, String> {
    let memory = memory_close::check_memory_close_source(unit, revision, dependencies)?;
    let combined = format!("{}{PROBE}", unit.source);
    let probe_unit = C11TranslationUnit {
        source: &combined,
        include_paths: unit.include_paths.clone(),
        definitions: unit.definitions.clone(),
        dialect: unit.dialect,
        function_scope: unit.function_scope,
    };
    let probes = import_source_functions(
        &probe_unit,
        revision,
        &["qcore_cio_filesystem_store_value"],
        &SourceImportProfile {
            records: &[],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    let [SourceResult::Summary(probe)] = probes.as_slice() else {
        return Err(format!(
            "filesystem kind source probe incomplete: {probes:?}"
        ));
    };
    check(&memory, probe, backend, CONTRACT)
}

fn check(
    memory: &MemoryCloseCertificate,
    probe: &SourceSummary,
    backend: &FileCloseDeletionEvidence,
    contract: &str,
) -> Result<FilesystemChunkCloseEvidence, String> {
    let memory = memory.recheck()?;
    let backend = backend.recheck()?;
    if probe.source_revision() != PINNED_FLUENT_BIT_REVISION
        || probe.function() != "qcore_cio_filesystem_store_value"
        || backend.source().source_revision() != memory.source().source_revision()
        || backend.source().signature() != "void (struct cio_chunk *, int)"
    {
        return Err("filesystem chunk dependency/probe identity mismatch".into());
    }
    let kind = memory_close::probe_literal(probe)?;
    if kind == memory.memory_kind()? {
        return Err("filesystem and memory backend discriminants overlap".into());
    }
    let mut cases = Vec::new();
    let mut covered = BTreeSet::new();
    for delete in [false, true] {
        let count = if delete {
            backend.deleting_cases().len()
        } else {
            backend.non_deleting_cases().len()
        };
        for entry in std::iter::once(None).chain((0..count).map(Some)) {
            let nonnull = entry.is_some();
            let source_path = memory_close::filesystem_path(&memory, kind, nonnull, delete)?;
            let (mapping, descriptor, deletion, backend_released, path_released, up) = match entry {
                None => (
                    ResidualCustody::Absent,
                    ResidualCustody::Absent,
                    None,
                    false,
                    false,
                    0,
                ),
                Some(i) if delete => {
                    let c = &backend.deleting_cases()[i];
                    (
                        c.mapping,
                        c.descriptor,
                        Some(c.deletion),
                        c.backend_released,
                        c.path_released,
                        c.up_counter_decrements,
                    )
                }
                Some(i) => {
                    let c = &backend.non_deleting_cases()[i];
                    (
                        c.mapping,
                        c.descriptor,
                        None,
                        c.backend_released,
                        c.path_released,
                        c.up_counter_decrements,
                    )
                }
            };
            let c = FilesystemChunkCloseCase {
                source_path,
                chunk_present: nonnull,
                delete,
                backend_case: entry,
                backend_released,
                path_released,
                mapping,
                descriptor,
                deletion,
                primary_unlinked: nonnull,
                state_unlinked: nonnull,
                name_released: nonnull,
                chunk_released: nonnull,
                total_counter_decrements: usize::from(nonnull),
                up_counter_decrements: up,
            };
            let unresolved_map = i128::from(mapping == ResidualCustody::Unresolved);
            let unresolved_fd = i128::from(descriptor == ResidualCustody::Unresolved);
            let succeeded = i128::from(deletion == Some(DeletionOutcome::SyscallSucceeded));
            let discharged = i128::from(unresolved_map == 0 && unresolved_fd == 0);
            crate::contract_observation::check_contract_facts(
                contract,
                "filesystem_chunk_close",
                &[
                    ("nullable_owned_filesystem_chunk_and_live_name", 1),
                    ("delete_argument_is_boolean", 1),
                    ("backend_dependency_custody_outcome_and_frame_premises", 1),
                    (
                        "backend_path_name_and_chunk_have_disjoint_owned_provenance",
                        1,
                    ),
                    ("live_exclusive_distinct_reciprocal_list_memberships", 1),
                    (
                        "context_and_stream_outlive_and_are_disjoint_from_owned_storage",
                        1,
                    ),
                    ("total_and_up_counters_cover_entry_obligations", 1),
                    ("no_lock_busy_task_route_or_transaction_obligations", 1),
                    (
                        "reset_list_counter_and_backend_cross_unit_source_abi_bindings",
                        1,
                    ),
                    ("free_resolves_to_libc", 1),
                    ("chunk_was_nonnull", i128::from(nonnull)),
                    ("delete_argument", i128::from(delete)),
                    ("backend_calls", i128::from(nonnull)),
                    ("primary_unlinked", i128::from(c.primary_unlinked)),
                    ("state_unlinked", i128::from(c.state_unlinked)),
                    ("name_released", i128::from(c.name_released)),
                    ("chunk_released", i128::from(c.chunk_released)),
                    (
                        "total_counter_decrements",
                        c.total_counter_decrements as i128,
                    ),
                    ("mapping_unresolved", unresolved_map),
                    ("dependency_mapping_unresolved", unresolved_map),
                    ("descriptor_unresolved", unresolved_fd),
                    ("dependency_descriptor_unresolved", unresolved_fd),
                    ("deletion_succeeded", succeeded),
                    ("dependency_deletion_succeeded", succeeded),
                    ("up_counter_decrements", c.up_counter_decrements as i128),
                    ("dependency_up_counter_decrements", up as i128),
                    ("nested_discharge_permitted", discharged),
                    (
                        "successful_deletion_permitted",
                        i128::from(delete && nonnull && succeeded == 1 && discharged == 1),
                    ),
                ],
            )?;
            covered.insert(source_path);
            cases.push(c);
        }
    }
    Ok(FilesystemChunkCloseEvidence {
        backend,
        filesystem_kind_probe: probe.clone(),
        probe_source: PROBE.into(),
        contract_source: contract.into(),
        cases,
        uncovered_paths: (0..memory.source().paths().len())
            .filter(|i| !covered.contains(i))
            .collect(),
        combined_uncovered_paths: (0..memory.source().paths().len())
            .filter(|i| !covered.contains(i) && !memory.checked_paths().contains(i))
            .collect(),
        checked_paths: covered.into_iter().collect(),
        memory_scope: memory,
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/filesystem_chunk_close.rs"]
pub(crate) mod tests;
