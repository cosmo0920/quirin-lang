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

//! Bounded source composition for null and memory-backed cio_chunk_close.
//! Filesystem/other backend paths remain explicit uncovered paths. Dependency
//! contracts are rechecked against their private source summaries before use.
//!
//! The decision procedure itself now lives in
//! `qcore_verification::memory_close_composition` (card
//! `P04-COMPOSE-MEMCLOSE`), relocated verbatim so this retained in-process
//! adapter and the moved process route call one implementation and cannot
//! drift. What stays here is the ChunkIO half: which contracts, which declared
//! root types, which derivation recipe and which pinned revision the
//! composition is taken under.
use super::{
    PINNED_FLUENT_BIT_REVISION,
    teardown_source::{COUNTER_CONTRACT, LIST_UNLINK_CONTRACT, MEMORY_BACKEND_CONTRACT},
};
use crate::legacy_c::C11TranslationUnit;
pub use qcore_verification::memory_close_composition::{
    MemoryCloseCertificate, MemoryCloseDependencies, MemoryCloseEntry, MemoryCloseReading,
    filesystem_path, probe_literal, unsupported_path,
};
use qcore_verification::memory_close_composition::{
    MemoryCloseContracts, check_memory_close_source as check_source,
};

pub const CONTRACT: &str =
    include_str!("../../../../providers/qcore-provider-chunkio/contracts/chunkio/memory_close.qv");

/// The frame helper prerequisite's pinned contract.
pub const FRAME_CONTRACT: &str =
    include_str!("../../../../providers/qcore-provider-chunkio/contracts/chunkio/error_state.qv");

/// The derivation recipe this route appends to the unit under analysis.
///
/// The backend discriminant `cio_chunk_close` branches on is a preprocessor
/// macro, so the route defines a constant probe that returns it and reads the
/// expansion as an ordinary integer literal instead of guessing one. The text
/// is a core-side constant: it never travels as an authority, and a transported
/// claim about it is compared against this and refused when it differs.
pub const PROBE: &str = "\nint qcore_cio_memory_store_value(void) { return CIO_STORE_MEM; }\n";
/// The function that recipe defines.
pub const PROBE_FUNCTION: &str = "qcore_cio_memory_store_value";

/// The pinned contracts, declared root types and recipe this adapter composes
/// under. The moved process route derives the same selection from the same
/// constants, and `compiler/tests/memory_close_route_parity.rs` measures the
/// two certificates byte-identical.
pub fn contracts() -> MemoryCloseContracts<'static> {
    contracts_with(CONTRACT)
}

/// The same selection under an explicit composition contract, so a falsified
/// contract can be driven through the one decision procedure without a second
/// copy of the ChunkIO half.
pub fn contracts_with(contract: &str) -> MemoryCloseContracts<'_> {
    MemoryCloseContracts {
        pinned_revision: PINNED_FLUENT_BIT_REVISION,
        contract,
        probe_recipe: PROBE,
        probe_function: PROBE_FUNCTION,
        frame_contract: FRAME_CONTRACT,
        backend_contract: MEMORY_BACKEND_CONTRACT,
        backend_root_c_type: "struct cio_chunk *",
        backend_root_qcore_type: "Chunk",
        unlink_contract: LIST_UNLINK_CONTRACT,
        unlink_node_c_type: "struct mk_list *",
        unlink_node_qcore_type: "ListNode",
        counter_contract: COUNTER_CONTRACT,
        counter_root_c_type: "struct cio_ctx *",
        counter_root_qcore_type: "ChunkContext",
        counter_field: "total_chunks",
        counter_delta: -1,
    }
}

pub fn check_memory_close_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    dependencies: &MemoryCloseDependencies,
) -> Result<MemoryCloseCertificate, String> {
    check_source(unit, revision, dependencies, &contracts())
}

#[cfg(test)]
#[path = "../../tests/unit/memory_close.rs"]
pub(crate) mod tests;
