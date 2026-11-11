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
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/unsupported_close.qv"
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
#[path = "../../tests/unit/unsupported_close.rs"]
mod tests;
