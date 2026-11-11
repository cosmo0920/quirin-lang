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

//! The bounded filesystem-backed `cio_chunk_close` composition (card
//! `P04-COMPOSE-FS`).
//!
//! Relocated **verbatim** from
//! `compiler/src/verifier_plugins/chunkio/filesystem_chunk_close.rs` so the
//! retained in-process adapter and the moved process route call one
//! implementation and cannot drift. The only change to the moved body is that
//! the constants it read from its former module scope — the pinned revision,
//! the probe recipe and function, this composition's own contract and the
//! backend prerequisite's whole pinned contract selection — are now explicit
//! parameters ([`FilesystemChunkCloseContracts`]).
//!
//! ## What is new here: two derived units in one question
//!
//! The memory-close composition already imports its wrapper and its own probe
//! from text derived by appending a constant recipe to the unit. This route
//! composes *that* certificate with a **second** derived import of its own: the
//! filesystem discriminant is a different preprocessor macro, so it is read out
//! of a second probe under a second recipe over the same base text.
//!
//! Both derivations are the core's own. Neither derived text travels, in either
//! direction, and the dependent task additionally requires the two recipes to
//! differ and the two derived texts to differ — a filesystem probe derived by
//! the memory recipe would read the memory discriminant, and the two arms would
//! collapse into one.
//!
//! ## What is new here: the two prerequisites live in different units
//!
//! The wrapper is in the chunk unit; the backend composition is in the file
//! unit. This route therefore has **no single-unit base tie**: what ties the
//! backend prerequisite is its own `PrerequisiteIdentity`, its declared source
//! revision and its declared prototype, each re-derived by the core. Recorded
//! as boundary `P04-COMPOSE-FS-CROSS-UNIT` and published as an assumption.
//!
//! Nothing here observes OS behaviour. A recorded `cio_file_close` frame is a
//! recorded call; whether a mapping, a descriptor or a file was actually torn
//! down stays inside the backend prerequisite's own declared premises.

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, derived_source_text, derived_translation_unit,
};
use qcore_c_gateway::typed_source::{
    SourceImportProfile, SourceResult, SourceSummary, import_source_functions,
};
use serde::Serialize;
use std::collections::BTreeSet;

use crate::cleanup_outcome::CleanupProfile;
use crate::file_close_composition::{
    DeletionOutcome, FileCloseContracts, FileCloseDeleteContracts, FileCloseDeletionEvidence,
    ResidualCustody,
};
use crate::memory_close_composition::{
    MemoryCloseCertificate, WRAPPER_SIGNATURE, filesystem_path, probe_literal,
};
use crate::unmap_composition::UnmapCompositionContracts;

/// The probe's declared C signature: a nullary function returning the macro.
/// It is the same shape the memory family's probe has, and the two are kept
/// apart by their recipes and their function names, never by their prototypes.
pub const PROBE_SIGNATURE: &str = "int (void)";

/// The backend prerequisite's whole pinned selection, owned so a certificate
/// can re-run it with no argument. Every field is the core's own; none travels.
///
/// It is spelled out rather than reusing the borrowed `FileCloseContracts`
/// because that type holds a reference to a nested `UnmapCompositionContracts`,
/// and a certificate must own everything it rechecks under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedBackendContracts {
    pub pinned_revision: String,
    /// The non-deleting `cio_file_close` composition's own contract.
    pub contract: String,
    /// The deleting arm's own contract.
    pub delete_contract: String,
    /// The native close prerequisite.
    pub close_function: String,
    pub close_profile: CleanupProfile,
    pub close_contract: String,
    /// The guarded-delete prerequisite.
    pub delete_function: String,
    pub delete_profile: CleanupProfile,
    pub delete_mapping_field: String,
    pub delete_prerequisite_contract: String,
    /// The nested unmap composition the `file_close` composition itself
    /// re-establishes.
    pub unmap_native_function: String,
    pub unmap_native_profile: CleanupProfile,
    pub unmap_native_contract: String,
    pub unmap_counter_contract: String,
    pub unmap_counter_root_c_type: String,
    pub unmap_counter_root_qcore_type: String,
    pub unmap_contract: String,
}

impl OwnedBackendContracts {
    #[must_use]
    pub fn unmap(&self) -> UnmapCompositionContracts<'_> {
        UnmapCompositionContracts {
            pinned_revision: &self.pinned_revision,
            native_function: &self.unmap_native_function,
            native_profile: &self.unmap_native_profile,
            native_contract: &self.unmap_native_contract,
            counter_contract: &self.unmap_counter_contract,
            counter_root_c_type: &self.unmap_counter_root_c_type,
            counter_root_qcore_type: &self.unmap_counter_root_qcore_type,
            contract: &self.unmap_contract,
        }
    }

    #[must_use]
    pub fn close<'a>(&'a self, unmap: &'a UnmapCompositionContracts<'a>) -> FileCloseContracts<'a> {
        FileCloseContracts {
            pinned_revision: &self.pinned_revision,
            unmap,
            close_function: &self.close_function,
            close_profile: &self.close_profile,
            close_contract: &self.close_contract,
            contract: &self.contract,
        }
    }

    #[must_use]
    pub fn delete(&self) -> FileCloseDeleteContracts<'_> {
        FileCloseDeleteContracts {
            delete_function: &self.delete_function,
            delete_profile: &self.delete_profile,
            delete_mapping_field: &self.delete_mapping_field,
            delete_contract: &self.delete_prerequisite_contract,
            contract: &self.delete_contract,
        }
    }
}

/// The pinned contracts and derivation recipe this composition is taken under.
/// Every one of them is the core's own selection; none travels.
pub struct FilesystemChunkCloseContracts<'a> {
    pub pinned_revision: &'a str,
    /// The composition's own pinned contract.
    pub contract: &'a str,
    /// The derivation recipe appended to the base unit, and the function it
    /// defines. Both are the core's own constants: a transported claim about
    /// either is compared against these and refused when it differs.
    pub probe_recipe: &'a str,
    pub probe_function: &'a str,
    /// The backend prerequisite's whole pinned selection.
    pub backend: &'a OwnedBackendContracts,
}

/// The owned form retained on a certificate so `recheck` needs no argument, and
/// so a certificate cannot be rechecked under contracts other than the ones it
/// was checked under. It is **not serialized**: the certificate's wire shape is
/// byte-identical to the shape the retained in-process route published before
/// this relocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedFilesystemChunkCloseContracts {
    pinned_revision: String,
    contract: String,
    probe_recipe: String,
    probe_function: String,
    backend: OwnedBackendContracts,
}

impl OwnedFilesystemChunkCloseContracts {
    #[must_use]
    pub fn of(contracts: &FilesystemChunkCloseContracts<'_>) -> Self {
        Self {
            pinned_revision: contracts.pinned_revision.into(),
            contract: contracts.contract.into(),
            probe_recipe: contracts.probe_recipe.into(),
            probe_function: contracts.probe_function.into(),
            backend: contracts.backend.clone(),
        }
    }

    #[must_use]
    pub fn borrowed(&self) -> FilesystemChunkCloseContracts<'_> {
        FilesystemChunkCloseContracts {
            pinned_revision: &self.pinned_revision,
            contract: &self.contract,
            probe_recipe: &self.probe_recipe,
            probe_function: &self.probe_function,
            backend: &self.backend,
        }
    }
}

/// A default that names nothing, so a certificate deserialized from nowhere
/// cannot be rechecked. Every field is empty and every recheck under it fails
/// closed at the first identity comparison.
impl Default for OwnedFilesystemChunkCloseContracts {
    fn default() -> Self {
        Self {
            pinned_revision: String::new(),
            contract: String::new(),
            probe_recipe: String::new(),
            probe_function: String::new(),
            backend: OwnedBackendContracts {
                pinned_revision: String::new(),
                contract: String::new(),
                delete_contract: String::new(),
                close_function: String::new(),
                close_profile: empty_profile(),
                close_contract: String::new(),
                delete_function: String::new(),
                delete_profile: empty_profile(),
                delete_mapping_field: String::new(),
                delete_prerequisite_contract: String::new(),
                unmap_native_function: String::new(),
                unmap_native_profile: empty_profile(),
                unmap_native_contract: String::new(),
                unmap_counter_contract: String::new(),
                unmap_counter_root_c_type: String::new(),
                unmap_counter_root_qcore_type: String::new(),
                unmap_contract: String::new(),
            },
        }
    }
}

fn empty_profile() -> CleanupProfile {
    CleanupProfile {
        function: String::new(),
        root_type: String::new(),
        syscall: String::new(),
        syscall_signature: String::new(),
        presence_field: String::new(),
        fields: Vec::new(),
        argument_fields: Vec::new(),
        policy: String::new(),
        errno_function: String::new(),
        report_function: String::new(),
    }
}

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
    /// The core's own pinned selection, retained so `recheck` re-runs under the
    /// same contracts. Skipped on the wire, so this relocation does not move a
    /// single byte of the published certificate.
    #[serde(skip)]
    contracts: OwnedFilesystemChunkCloseContracts,
}

impl FilesystemChunkCloseEvidence {
    pub fn recheck(&self) -> Result<Self, String> {
        check(
            &self.memory_scope,
            &self.filesystem_kind_probe,
            &self.backend,
            &self.contracts.borrowed(),
        )
    }
    pub fn memory_scope(&self) -> &MemoryCloseCertificate {
        &self.memory_scope
    }
    pub fn filesystem_kind_probe(&self) -> &SourceSummary {
        &self.filesystem_kind_probe
    }
    /// The backend composition's re-established reading.
    pub fn backend(&self) -> &FileCloseDeletionEvidence {
        &self.backend
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
    /// The filesystem discriminant this reading probed.
    pub fn filesystem_kind(&self) -> Result<i32, String> {
        probe_literal(&self.filesystem_kind_probe)
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

/// Import this family's one body — the filesystem discriminant probe — from
/// text derived here, and decide the composition. The derived text is built
/// from the caller's base unit and the core's own recipe; no caller supplies
/// derived text.
pub fn check_filesystem_chunk_close_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    memory: &MemoryCloseCertificate,
    backend: &FileCloseDeletionEvidence,
    contracts: &FilesystemChunkCloseContracts<'_>,
) -> Result<FilesystemChunkCloseEvidence, String> {
    let probe = import_filesystem_probe(unit, revision, contracts)?;
    check(memory, &probe, backend, contracts)
}

/// Import exactly the one body this composition reads, from the derived unit.
pub fn import_filesystem_probe(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    contracts: &FilesystemChunkCloseContracts<'_>,
) -> Result<SourceSummary, String> {
    if revision != contracts.pinned_revision {
        return Err("filesystem chunk close revision mismatch".into());
    }
    let combined = derived_source_text(unit.source, contracts.probe_recipe);
    let probe_unit = derived_translation_unit(unit, &combined);
    let probes = import_source_functions(
        &probe_unit,
        revision,
        &[contracts.probe_function],
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
    Ok(probe.clone())
}

/// Decide the composition over an already-imported probe, an already-checked
/// memory certificate and an already-checked backend reading, under an explicit
/// contract selection. The importing half is separate so a caller that has the
/// bodies does not re-import to ask the same question.
pub fn check_filesystem_chunk_close_bodies(
    memory: &MemoryCloseCertificate,
    probe: &SourceSummary,
    backend: &FileCloseDeletionEvidence,
    contracts: &FilesystemChunkCloseContracts<'_>,
) -> Result<FilesystemChunkCloseEvidence, String> {
    check(memory, probe, backend, contracts)
}

fn check(
    memory: &MemoryCloseCertificate,
    probe: &SourceSummary,
    backend: &FileCloseDeletionEvidence,
    contracts: &FilesystemChunkCloseContracts<'_>,
) -> Result<FilesystemChunkCloseEvidence, String> {
    let contract = contracts.contract;
    // Neither supplied reading is trusted: both are re-run under the core's own
    // pinned contracts before a single case is decided.
    let memory = memory.recheck()?;
    let unmap = contracts.backend.unmap();
    let backend = backend.recheck(
        &contracts.backend.close(&unmap),
        &contracts.backend.delete(),
    )?;
    if probe.source_revision() != contracts.pinned_revision
        || probe.function() != contracts.probe_function
        || probe.signature() != PROBE_SIGNATURE
        || backend.source().source_revision() != memory.source().source_revision()
        || backend.source().signature() != WRAPPER_SIGNATURE
    {
        return Err("filesystem chunk dependency/probe identity mismatch".into());
    }
    let kind = probe_literal(probe)?;
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
            let source_path = filesystem_path(&memory, kind, nonnull, delete)?;
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
            qcore_contract::contract_observation::check_contract_facts(
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
        probe_source: contracts.probe_recipe.into(),
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
        contracts: OwnedFilesystemChunkCloseContracts::of(contracts),
    })
}
