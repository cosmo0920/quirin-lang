//! Classify the complete nonnull, non-memory, non-filesystem close domain.
//! This is conditional partial-release evidence, never a Safe/Unsafe verdict.
//!
//! The decision procedure itself now lives in
//! `qcore_verification::unsupported_close_composition` (card
//! `P04-COMPOSE-UNSUPPORTED`), relocated verbatim so this retained in-process
//! adapter and the moved process route call one implementation and cannot drift.
//! What stays here is the ChunkIO half: which contract, which pinned revision
//! and which filesystem probe function the classification is taken under.
use super::{PINNED_FLUENT_BIT_REVISION, filesystem_chunk_close};
pub use qcore_verification::unsupported_close_composition::UnsupportedCloseEvidence;
use qcore_verification::unsupported_close_composition::{
    UnsupportedCloseContracts, check_unsupported_close_source as check_source,
};

pub const CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/unsupported_close.qcore"
);

/// The pinned selection this adapter classifies under.
pub fn contracts(contract: &str) -> UnsupportedCloseContracts<'_> {
    UnsupportedCloseContracts {
        pinned_revision: PINNED_FLUENT_BIT_REVISION,
        contract,
        probe_function: filesystem_chunk_close::PROBE_FUNCTION,
    }
}

pub fn check_unsupported_close_source(
    filesystem: &filesystem_chunk_close::FilesystemChunkCloseEvidence,
) -> Result<UnsupportedCloseEvidence, String> {
    check_source(filesystem, &contracts(CONTRACT))
}

#[cfg(test)]
#[path = "../../../tests/unit/unsupported_close.rs"]
mod tests;
