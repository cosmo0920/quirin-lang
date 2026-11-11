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

//! The bounded **symbolic** other-backend `cio_chunk_close` composition (card
//! `P04-COMPOSE-UNSUPPORTED`).
//!
//! Relocated **verbatim** from
//! `compiler/src/verifier_plugins/chunkio/unsupported_close.rs` so the retained
//! in-process adapter and the moved process route call one implementation and
//! cannot drift. The only change to the moved body is that the constants it read
//! from its former module scope — the pinned revision, the filesystem probe's
//! function name and this composition's own contract — are now explicit
//! parameters ([`UnsupportedCloseContracts`]).
//!
//! ## What is new here: a symbolic class, not a sampled value
//!
//! The three composition arms below this one each branch on a *value*: the
//! memory discriminant, the filesystem discriminant, or the null entry. This arm
//! branches on everything else — the class of every `int` that is neither
//! probed constant — and no member of that class is ever chosen as a
//! representative. The replay carries the class itself
//! (`Value::OtherBackend(memory, file)`), and every comparison it cannot decide
//! for the *whole* class is a refusal:
//!
//! * a truth test on the class refuses whenever `0` is inside it, because the
//!   two arms of the branch would then both be reachable and no single reading
//!   covers the class;
//! * an equality against an integer answers `false` only for the two excluded
//!   constants, and refuses for every other integer, because the class contains
//!   that integer and its complement both;
//! * a comparison against anything that is not an integer refuses outright.
//!
//! ## What is new here: the conclusion is `Unknown`, by construction
//!
//! An admitted reading here establishes the wrapper's own six releases and
//! **withholds** two things by name: the unrecognised backend's own custody, and
//! whether any caller can reach this class at all. [`UnsupportedCloseEvidence`]
//! carries both as published prose and its `require_nested_discharge` returns an
//! `Unknown:` refusal unconditionally. Nothing in this module can promote it,
//! and the moved route's proposal record cannot express a promotion either.
//!
//! Nothing here observes OS behaviour, and nothing here infers reachability: the
//! entry state is a stipulated nonnull chunk whose stream's backend is outside
//! both constants, and whether a constructor can produce one is exactly what
//! stays open.

use qcore_c_gateway::typed_source::SourceSummary;
use serde::Serialize;

use crate::filesystem_chunk_close_composition::FilesystemChunkCloseEvidence;
use crate::memory_close_composition::{MemoryCloseCertificate, probe_literal, unsupported_path};

/// The pinned contract and probe identity this composition is taken under.
/// Every one of them is the core's own selection; none travels.
pub struct UnsupportedCloseContracts<'a> {
    pub pinned_revision: &'a str,
    /// The composition's own pinned contract.
    pub contract: &'a str,
    /// The filesystem discriminant probe's function name. It is the core's own
    /// constant: a probe body under any other name is refused rather than read.
    pub probe_function: &'a str,
}

/// The owned form retained on a certificate so `recheck` needs no argument, and
/// so a certificate cannot be rechecked under contracts other than the ones it
/// was checked under. It is **not serialized**: the certificate's wire shape is
/// byte-identical to the shape the retained in-process route published before
/// this relocation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnedUnsupportedCloseContracts {
    pinned_revision: String,
    contract: String,
    probe_function: String,
}

impl OwnedUnsupportedCloseContracts {
    #[must_use]
    pub fn of(contracts: &UnsupportedCloseContracts<'_>) -> Self {
        Self {
            pinned_revision: contracts.pinned_revision.into(),
            contract: contracts.contract.into(),
            probe_function: contracts.probe_function.into(),
        }
    }

    #[must_use]
    pub fn borrowed(&self) -> UnsupportedCloseContracts<'_> {
        UnsupportedCloseContracts {
            pinned_revision: &self.pinned_revision,
            contract: &self.contract,
            probe_function: &self.probe_function,
        }
    }
}

/// Classify the complete nonnull, non-memory, non-filesystem close domain.
/// This is conditional partial-release evidence, never a Safe/Unsafe verdict.
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
    /// The core's own pinned selection, retained so `recheck` re-runs under the
    /// same contracts. Skipped on the wire, so this relocation does not move a
    /// single byte of the published certificate.
    #[serde(skip)]
    contracts: OwnedUnsupportedCloseContracts,
}

impl UnsupportedCloseEvidence {
    pub fn recheck(&self) -> Result<Self, String> {
        check(
            &self.memory_scope,
            &self.filesystem_kind_probe,
            &self.contracts.borrowed(),
        )
    }
    pub fn memory_scope(&self) -> &MemoryCloseCertificate {
        &self.memory_scope
    }
    /// The filesystem discriminant probe this reading excluded, retained so a
    /// recheck and an independent rederivation read the same body.
    pub fn filesystem_kind_probe(&self) -> &SourceSummary {
        &self.filesystem_kind_probe
    }
    pub fn source_path(&self) -> usize {
        self.source_path
    }
    pub fn excluded_kinds(&self) -> [i32; 2] {
        self.excluded_kinds
    }
    /// The published verdict word. It is `Unknown` on every admitted reading;
    /// this arm has no other inhabitant.
    pub fn status(&self) -> &str {
        &self.status
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
    pub fn require_nested_discharge(&self) -> Result<(), String> {
        Err(format!("Unknown: {}", self.reason))
    }
}

/// Decide the composition from an already-checked filesystem reading. The
/// filesystem certificate is **re-run** first: nothing about it is taken on
/// trust, including by the retained in-process caller.
pub fn check_unsupported_close_source(
    filesystem: &FilesystemChunkCloseEvidence,
    contracts: &UnsupportedCloseContracts<'_>,
) -> Result<UnsupportedCloseEvidence, String> {
    let filesystem = filesystem.recheck()?;
    check(
        filesystem.memory_scope(),
        filesystem.filesystem_kind_probe(),
        contracts,
    )
}

/// Decide the composition over an already-checked memory certificate and an
/// already-imported filesystem probe, under an explicit contract selection. The
/// separation exists so a caller that already has the bodies — a recheck, or a
/// mutation that falsifies one pinned contract — does not re-run the whole
/// filesystem composition to ask the same question.
pub fn check_unsupported_close_bodies(
    memory: &MemoryCloseCertificate,
    probe: &SourceSummary,
    contracts: &UnsupportedCloseContracts<'_>,
) -> Result<UnsupportedCloseEvidence, String> {
    check(memory, probe, contracts)
}

fn check(
    memory: &MemoryCloseCertificate,
    probe: &SourceSummary,
    contracts: &UnsupportedCloseContracts<'_>,
) -> Result<UnsupportedCloseEvidence, String> {
    let contract = contracts.contract;
    let memory = memory.recheck()?;
    if probe.source_revision() != contracts.pinned_revision
        || probe.function() != contracts.probe_function
    {
        return Err("unsupported close filesystem probe identity mismatch".into());
    }
    let kinds = [memory.memory_kind()?, probe_literal(probe)?];
    if kinds[0] == kinds[1] {
        return Err("unsupported close backend discriminants overlap".into());
    }
    let source_path = unsupported_path(&memory, kinds[1], false)?;
    if unsupported_path(&memory, kinds[1], true)? != source_path {
        return Err("unsupported close unexpectedly depends on the delete argument".into());
    }
    qcore_contract::contract_observation::check_contract_facts(
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
        contracts: OwnedUnsupportedCloseContracts::of(contracts),
    })
}
