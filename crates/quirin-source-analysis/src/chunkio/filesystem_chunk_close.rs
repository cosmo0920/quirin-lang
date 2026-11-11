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

//! Conditional filesystem arm of the enclosing chunk destructor. Allocation
//! release and list/counter updates retain the backend's residual OS obligations.
//!
//! The decision procedure itself now lives in
//! `qcore_verification::filesystem_chunk_close_composition` (card
//! `P04-COMPOSE-FS`), relocated verbatim so this retained in-process adapter and
//! the moved process route call one implementation and cannot drift. What stays
//! here is the ChunkIO half: which contracts, which record profiles, which
//! declared eligibility field, which derivation recipe and which pinned revision
//! the composition is taken under.
use super::{
    PINNED_FLUENT_BIT_REVISION, file_close, file_delete, file_outcomes, memory_close,
    teardown_source::COUNTER_CONTRACT, unmap_wrapper,
};
use crate::legacy_c::C11TranslationUnit;
pub use qcore_verification::file_close_composition::{
    DeletionOutcome, FileCloseDeletionEvidence, ResidualCustody,
};
pub use qcore_verification::filesystem_chunk_close_composition::{
    FilesystemChunkCloseCase, FilesystemChunkCloseEvidence,
};
use qcore_verification::filesystem_chunk_close_composition::{
    FilesystemChunkCloseContracts, OwnedBackendContracts,
    check_filesystem_chunk_close_source as check_source,
};
use qcore_verification::memory_close_composition::MemoryCloseDependencies;

pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_chunk_close.qv"
);

/// The derivation recipe this route appends to the unit under analysis.
///
/// The filesystem discriminant `cio_chunk_close` branches on is a preprocessor
/// macro, so the route defines a constant probe that returns it and reads the
/// expansion as an ordinary integer literal instead of guessing one. It is a
/// **second** recipe over the same base unit, distinct from the memory family's
/// in both its text and the function it defines; the moved route requires that
/// distinctness explicitly rather than assuming it.
pub const PROBE: &str = "\nint qcore_cio_filesystem_store_value(void) { return CIO_STORE_FS; }\n";
/// The function that recipe defines.
pub const PROBE_FUNCTION: &str = "qcore_cio_filesystem_store_value";

/// The backend prerequisite's whole pinned selection, as this adapter takes it.
/// The moved process route derives the same selection from the same constants,
/// and `compiler/tests/filesystem_chunk_close_route_parity.rs` measures the two
/// certificates byte-identical.
pub fn backend_contracts() -> Result<OwnedBackendContracts, String> {
    let close_profile = file_outcomes::profile("cio_file_native_close")?;
    let unmap_profile = file_outcomes::profile("cio_file_native_unmap")?;
    let delete_profile = file_delete::profile();
    Ok(OwnedBackendContracts {
        pinned_revision: PINNED_FLUENT_BIT_REVISION.into(),
        contract: file_close::CONTRACT.into(),
        delete_contract: file_close::DELETE_CONTRACT.into(),
        close_function: "cio_file_native_close".into(),
        close_profile,
        close_contract: file_outcomes::CONTRACT.into(),
        delete_function: "cio_file_native_delete".into(),
        delete_profile,
        delete_mapping_field: "map".into(),
        delete_prerequisite_contract: file_delete::CONTRACT.into(),
        unmap_native_function: "cio_file_native_unmap".into(),
        unmap_native_profile: unmap_profile,
        unmap_native_contract: file_outcomes::CONTRACT.into(),
        unmap_counter_contract: COUNTER_CONTRACT.into(),
        unmap_counter_root_c_type: "struct cio_ctx *".into(),
        unmap_counter_root_qcore_type: "ChunkContext".into(),
        unmap_contract: unmap_wrapper::CONTRACT.into(),
    })
}

/// The pinned contracts and recipe this adapter composes under.
pub fn contracts<'a>(backend: &'a OwnedBackendContracts) -> FilesystemChunkCloseContracts<'a> {
    contracts_with(backend, CONTRACT)
}

/// The same selection under an explicit composition contract, so a falsified
/// contract can be driven through the one decision procedure without a second
/// copy of the ChunkIO half.
pub fn contracts_with<'a>(
    backend: &'a OwnedBackendContracts,
    contract: &'a str,
) -> FilesystemChunkCloseContracts<'a> {
    FilesystemChunkCloseContracts {
        pinned_revision: PINNED_FLUENT_BIT_REVISION,
        contract,
        probe_recipe: PROBE,
        probe_function: PROBE_FUNCTION,
        backend,
    }
}

pub fn check_filesystem_chunk_close_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    dependencies: &MemoryCloseDependencies,
    backend: &FileCloseDeletionEvidence,
) -> Result<FilesystemChunkCloseEvidence, String> {
    let memory = memory_close::check_memory_close_source(unit, revision, dependencies)?;
    let selection = backend_contracts()?;
    check_source(unit, revision, &memory, backend, &contracts(&selection))
}

#[cfg(test)]
#[path = "../../tests/unit/filesystem_chunk_close.rs"]
pub(crate) mod tests;
