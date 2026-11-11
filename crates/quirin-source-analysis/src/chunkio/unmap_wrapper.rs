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

//! Source composition of munmap_file with checked native unmap and counter
//! helpers. Sync/log frames remain explicit unproved external-call premises.
//!
//! The decision procedure itself now lives in
//! `qcore_verification::unmap_composition` (card P04-COMPOSE), relocated
//! verbatim so this retained in-process adapter and the moved process route
//! call one implementation and cannot drift. What stays here is the ChunkIO
//! half: which contracts, which record profile and which pinned revision the
//! composition is taken under.
use super::{
    PINNED_FLUENT_BIT_REVISION, file_attempts::check_file_attempt_sources, file_outcomes,
    teardown_source::COUNTER_CONTRACT,
};
#[cfg(test)]
use crate::cleanup_outcome::check_cleanup_outcomes;
use crate::cleanup_outcome::{CleanupCertificate, CleanupProfile};
use crate::counter_contract::CounterCertificate;
#[cfg(test)]
use crate::counter_contract::check_counter_contract;
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::SourceSummary;
pub use qcore_verification::unmap_composition::{UnmapCase, UnmapCertificate};
use qcore_verification::unmap_composition::{UnmapCompositionContracts, check_unmap_composition};

pub const CONTRACT: &str =
    include_str!("../../../../providers/qcore-provider-chunkio/contracts/chunkio/unmap_wrapper.qv");

/// The prerequisite record profile this composition is taken under. It is the
/// same table the retained `file_outcomes` route uses; the moved process route
/// derives its own from the pinned contract's declared vocabulary and the two
/// are measured equal in `compiler/tests/composition_route_parity.rs`.
pub(super) fn native_profile() -> Result<CleanupProfile, String> {
    file_outcomes::profile("cio_file_native_unmap")
}

/// The pinned contracts and declared root types this adapter composes under.
pub(super) fn contracts<'a>(profile: &'a CleanupProfile) -> UnmapCompositionContracts<'a> {
    contracts_with(profile, CONTRACT)
}

fn contracts_with<'a>(
    profile: &'a CleanupProfile,
    contract: &'a str,
) -> UnmapCompositionContracts<'a> {
    UnmapCompositionContracts {
        pinned_revision: PINNED_FLUENT_BIT_REVISION,
        native_function: "cio_file_native_unmap",
        native_profile: profile,
        native_contract: file_outcomes::CONTRACT,
        counter_contract: COUNTER_CONTRACT,
        counter_root_c_type: "struct cio_ctx *",
        counter_root_qcore_type: "ChunkContext",
        contract,
    }
}

pub fn check_unmap_wrapper_source(
    unit: &C11TranslationUnit<'_>,
    revision: &str,
    native: &CleanupCertificate,
    counter: &CounterCertificate,
) -> Result<UnmapCertificate, String> {
    if revision != PINNED_FLUENT_BIT_REVISION {
        return Err("unmap wrapper revision mismatch".into());
    }
    let evidence = check_file_attempt_sources(unit, revision, false)?;
    let source = evidence
        .iter()
        .find(|e| e.source().function() == "munmap_file")
        .ok_or("missing munmap_file source")?
        .source();
    check(source, native, counter, CONTRACT)
}

fn check(
    source: &SourceSummary,
    native: &CleanupCertificate,
    counter: &CounterCertificate,
    contract: &str,
) -> Result<UnmapCertificate, String> {
    let profile = native_profile()?;
    check_unmap_composition(source, native, counter, &contracts_with(&profile, contract))
}

#[cfg(test)]
#[path = "../../tests/unit/unmap_wrapper.rs"]
mod tests;
