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

//! Non-deleting and deleting `cio_file_close` source compositions. Releasing the
//! backend's C allocation does not erase unresolved mapping/descriptor cleanup
//! obligations.
//!
//! The decision procedure itself now lives in
//! `qcore_verification::file_close_composition` (card `P04-COMPOSE-A`),
//! relocated verbatim so this retained in-process adapter and the moved process
//! route call one implementation and cannot drift. What stays here is the
//! ChunkIO half: which contracts, which record profiles, which declared
//! eligibility field and which pinned revision the compositions are taken under.
use super::{
    PINNED_FLUENT_BIT_REVISION, file_attempts::check_file_attempt_sources, file_outcomes,
    unmap_wrapper::UnmapCertificate,
};
use crate::cleanup_outcome::{CleanupCertificate, CleanupProfile, GuardedDeleteCertificate};
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::SourceSummary;
pub use qcore_verification::file_close_composition::{
    DeletingCloseCase, DeletionOutcome, FileCloseCase, FileCloseDeletionEvidence,
    FileCloseEvidence, ResidualCustody,
};
use qcore_verification::file_close_composition::{
    FileCloseContracts, FileCloseDeleteContracts, check_file_close_composition,
    check_file_close_deletion,
};

pub const CONTRACT: &str =
    include_str!("../../../../providers/qcore-provider-chunkio/contracts/chunkio/file_close.qv");
pub const DELETE_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/file_close_delete.qv"
);

/// The native close prerequisite's record profile, the same table the retained
/// `file_outcomes` route uses. The moved process route derives its own from the
/// pinned contract's declared vocabulary and the two are measured equal in
/// `compiler/tests/composition_route_parity.rs`.
pub(super) fn close_profile() -> Result<CleanupProfile, String> {
    file_outcomes::profile("cio_file_native_close")
}

/// The pinned contracts this adapter composes the non-deleting arm under.
pub(super) fn contracts<'a>(
    unmap: &'a qcore_verification::unmap_composition::UnmapCompositionContracts<'a>,
    close_profile: &'a CleanupProfile,
    contract: &'a str,
) -> FileCloseContracts<'a> {
    FileCloseContracts {
        pinned_revision: PINNED_FLUENT_BIT_REVISION,
        unmap,
        close_function: "cio_file_native_close",
        close_profile,
        close_contract: file_outcomes::CONTRACT,
        contract,
    }
}

/// The pinned contracts, record profile and declared eligibility field the
/// deleting arm additionally composes under.
pub(super) fn delete_contracts<'a>(
    delete_profile: &'a CleanupProfile,
    contract: &'a str,
) -> FileCloseDeleteContracts<'a> {
    FileCloseDeleteContracts {
        delete_function: "cio_file_native_delete",
        delete_profile,
        delete_mapping_field: "map",
        delete_contract: super::file_delete::CONTRACT,
        contract,
    }
}

pub fn check_file_close_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    unmap: &UnmapCertificate,
    close: &CleanupCertificate,
) -> Result<FileCloseEvidence, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("file-close revision mismatch".into());
    }
    let evidence = check_file_attempt_sources(unit, revision, false)?;
    if evidence
        .iter()
        .find(|e| e.source().function() == "munmap_file")
        .map(|e| e.source())
        != Some(unmap.source())
    {
        return Err("file-close same-unit unmap dependency source mismatch".into());
    }
    let source = evidence
        .iter()
        .find(|e| e.source().function() == "cio_file_close")
        .ok_or("missing file-close source")?
        .source();
    check(source, unmap, close, CONTRACT)
}

pub(super) fn check(
    source: &SourceSummary,
    unmap: &UnmapCertificate,
    close: &CleanupCertificate,
    contract: &str,
) -> Result<FileCloseEvidence, String> {
    let unmap_profile = super::unmap_wrapper::native_profile()?;
    let unmap_contracts = super::unmap_wrapper::contracts(&unmap_profile);
    let close_profile = close_profile()?;
    check_file_close_composition(
        source,
        unmap,
        close,
        &contracts(&unmap_contracts, &close_profile, contract),
    )
}

pub fn check_file_close_deletion_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    unmap: &UnmapCertificate,
    close: &CleanupCertificate,
    delete: &GuardedDeleteCertificate,
) -> Result<FileCloseDeletionEvidence, String> {
    let no_delete = check_file_close_source(unit, revision, unmap, close)?;
    check_deletion(no_delete, delete, DELETE_CONTRACT)
}

pub(super) fn check_deletion(
    no_delete: FileCloseEvidence,
    delete: &GuardedDeleteCertificate,
    contract: &str,
) -> Result<FileCloseDeletionEvidence, String> {
    let unmap_profile = super::unmap_wrapper::native_profile()?;
    let unmap_contracts = super::unmap_wrapper::contracts(&unmap_profile);
    let close_profile = close_profile()?;
    let delete_profile = super::file_delete::profile();
    check_file_close_deletion(
        no_delete,
        delete,
        &contracts(&unmap_contracts, &close_profile, CONTRACT),
        &delete_contracts(&delete_profile, contract),
    )
}

#[cfg(test)]
#[path = "../../tests/unit/file_close.rs"]
pub(crate) mod tests;
