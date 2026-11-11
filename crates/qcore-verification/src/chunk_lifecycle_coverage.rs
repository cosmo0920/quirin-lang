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

//! Total, fail-closed lifecycle accounting for a source-selected chunk
//! function inventory.
//!
//! This adapter answers one question per selected function: *is its lifecycle
//! post-state established?* It is deliberately not a field-contract check. A
//! caller either supplies a complete composition — the nested
//! context → stream → chunk → backend teardown, both embedded list unlinks and
//! the context counter, all read out of an accepted
//! [`qcore_core::chunk_lifecycle`] program, plus source-derived error-exit and
//! wrapper-obligation evidence — or the function keeps a precise `Unknown`
//! reason naming exactly which legs are outstanding.
//!
//! Five properties are structural rather than conventional.
//!
//! * The seven model legs cannot be asserted. They are derived here from a
//!   [`ChunkProgram`] this module re-runs itself — [`check_chunk_program`] for
//!   a terminal composition, [`execute_chunk_prefix`] for a prefix one. A
//!   refused program, or one that tears nothing down, gets a rejection rather
//!   than a discharge, and there is no constructor that takes a leg set.
//! * A required premise is not a discharge. A prefix composition records the
//!   identities it leaves live — a close whose context and stream outlive the
//!   chunk by contract keeps both container legs outstanding — and can never
//!   reach `Safe`.
//! * A delete-only operation records the post-state its own program actually
//!   reaches. It retains its live container identities and can never be `Safe`.
//! * Accounting is exact. Omitted, duplicated and unselected rows reject the
//!   whole operation instead of shrinking the denominator.
//! * A composition belongs to the function whose own pinned unit it was
//!   derived from (card `W43-CHUNK-SOURCE-PROFILE`). Its [`SourceDerivation`]
//!   must name that function, the pinned revision and the pinned unit digest,
//!   or the operation is refused; and its model legs count for the function
//!   only when it composes every source path. A composition over some paths
//!   reaches its legs there, discharges nothing for the function, and leaves
//!   each uncovered path as an explicit `Unknown` with its own reason.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::chunk_lifecycle::{
    ChunkCertificate, ChunkProgram, ChunkState, ChunkTraceEvent, check_chunk_program,
    execute_chunk_prefix,
};

/// One obligation a lifecycle result depends on. `Safe` requires all nine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleLeg {
    /// Every `cio_ctx` identity the program names is destroyed.
    ContextTeardown,
    /// Every `cio_stream` identity the program names is destroyed.
    StreamTeardown,
    /// Every `cio_chunk` identity the program names is closed.
    ChunkTeardown,
    /// Backend object, buffers, mapping, descriptor, path and name released.
    BackendTeardown,
    /// The chunk's primary `mk_list` membership is unlinked.
    PrimaryListUnlink,
    /// The chunk's residency (`_up`) `mk_list` membership is unlinked.
    ResidencyListUnlink,
    /// The context's chunk counters return to zero.
    ContextCounters,
    /// Every error exit of the source function is modeled and rolls back.
    ErrorExits,
    /// The Fluent Bit wrapper's busy/lock/task/route custody is discharged.
    WrapperObligations,
}

/// The nine legs an accepted lifecycle result must discharge.
pub const REQUIRED_LEGS: [LifecycleLeg; 9] = [
    LifecycleLeg::ContextTeardown,
    LifecycleLeg::StreamTeardown,
    LifecycleLeg::ChunkTeardown,
    LifecycleLeg::BackendTeardown,
    LifecycleLeg::PrimaryListUnlink,
    LifecycleLeg::ResidencyListUnlink,
    LifecycleLeg::ContextCounters,
    LifecycleLeg::ErrorExits,
    LifecycleLeg::WrapperObligations,
];

/// The legs this module derives from a re-checked model program. The remaining
/// two are source-derived and are supplied separately.
pub const MODEL_LEGS: [LifecycleLeg; 7] = [
    LifecycleLeg::ContextTeardown,
    LifecycleLeg::StreamTeardown,
    LifecycleLeg::ChunkTeardown,
    LifecycleLeg::BackendTeardown,
    LifecycleLeg::PrimaryListUnlink,
    LifecycleLeg::ResidencyListUnlink,
    LifecycleLeg::ContextCounters,
];

impl LifecycleLeg {
    pub fn name(self) -> &'static str {
        match self {
            Self::ContextTeardown => "context_teardown",
            Self::StreamTeardown => "stream_teardown",
            Self::ChunkTeardown => "chunk_teardown",
            Self::BackendTeardown => "backend_teardown",
            Self::PrimaryListUnlink => "primary_list_unlink",
            Self::ResidencyListUnlink => "residency_list_unlink",
            Self::ContextCounters => "context_counters",
            Self::ErrorExits => "error_exits",
            Self::WrapperObligations => "wrapper_obligations",
        }
    }
}

/// What the pinned source projection established about a function's shape.
/// A role is evidence, never a guess from the function's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleRole {
    /// The post-state depends on destruction; the nested teardown composition
    /// is the thing that must discharge.
    DestructionDependent,
    /// A `cio_stream_delete`-shaped operation: content is destroyed while the
    /// container identity stays live. Its actual partial post-state is kept.
    DeleteOnly,
    /// The modeled post-state contains no destruction at all.
    NonDestructive,
    /// The pinned projection refuses this function, so it has no role.
    Unclassified,
}

/// One selected function's source/build identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct FunctionIdentity {
    pub library: String,
    pub file: String,
    pub function: String,
}

impl FunctionIdentity {
    pub fn new(
        library: impl Into<String>,
        file: impl Into<String>,
        function: impl Into<String>,
    ) -> Self {
        Self {
            library: library.into(),
            file: file.into(),
            function: function.into(),
        }
    }

    fn key(&self) -> (String, String, String) {
        (
            self.library.clone(),
            self.file.clone(),
            self.function.clone(),
        )
    }
}

/// The pinned source/build binding every published row carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceBinding {
    /// The artifact the inventory was read from.
    pub artifact: String,
    /// That artifact's uncompressed content digest.
    pub artifact_sha256: String,
    /// The pinned upstream source revision the artifact was produced at.
    pub source_revision: String,
    /// The compilation profile the selection came from.
    pub source_profile: String,
    /// The SHA-256 of every selected translation unit at the pinned revision,
    /// keyed by the file name a [`FunctionIdentity`] carries. A composition is
    /// attachable to a row only when it was derived from exactly these bytes.
    pub pinned_units: BTreeMap<String, String>,
}

fn require_nonempty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("source binding has an empty {label}"));
    }
    Ok(())
}

impl SourceBinding {
    fn validate(&self) -> Result<(), String> {
        require_nonempty("artifact", &self.artifact)?;
        require_nonempty("artifact digest", &self.artifact_sha256)?;
        require_nonempty("source revision", &self.source_revision)?;
        require_nonempty("source profile", &self.source_profile)?;
        for (file, digest) in &self.pinned_units {
            require_nonempty("pinned unit name", file)?;
            require_nonempty("pinned unit digest", digest)?;
        }
        Ok(())
    }
}

/// The translation unit a composition or partial post-state was derived from.
///
/// Card `W43-CHUNK-SOURCE-PROFILE`. The accounting compares every field with
/// the row it is attached to and with the binding's pinned unit digest, so a
/// composition taken over a distilled fixture, over another function, or at
/// another revision is an operation error rather than evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompositionSource {
    pub library: String,
    pub file: String,
    pub function: String,
    pub source_revision: String,
    pub unit_sha256: String,
}

/// A source path of the function that the composition does not reach, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UncoveredPath {
    pub path: String,
    pub reason: String,
}

/// Where a composition came from and which of the function's source paths it
/// composes.
///
/// A composition over only some paths *reaches* its legs on those paths. For
/// the function it discharges no model leg: each uncovered path stays an
/// explicit `Unknown` with its own reason, and the row names them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceDerivation {
    pub source: CompositionSource,
    /// The other pinned units whose rechecked certificates the composition
    /// consumed (path to SHA-256). Recorded for audit; the row's own unit is
    /// the one [`SourceBinding::pinned_units`] binds.
    pub dependency_units: BTreeMap<String, String>,
    pub checked_paths: Vec<String>,
    pub uncovered_paths: Vec<UncoveredPath>,
}

impl SourceDerivation {
    /// Whether every source path of the function is composed.
    pub fn paths_complete(&self) -> bool {
        self.uncovered_paths.is_empty()
    }

    fn validate(&self, row: &FunctionIdentity, binding: &SourceBinding) -> Result<(), String> {
        let function = &row.function;
        let source = &self.source;
        if source.library != row.library || source.file != row.file || source.function != *function
        {
            return Err(format!(
                "`{function}` carries a composition derived from `{}/{}/{}`, not from its own source",
                source.library, source.file, source.function
            ));
        }
        if source.source_revision != binding.source_revision {
            return Err(format!(
                "`{function}` carries a composition derived at revision `{}`, not the pinned `{}`",
                source.source_revision, binding.source_revision
            ));
        }
        let pinned = binding.pinned_units.get(&row.file).ok_or_else(|| {
            format!(
                "`{function}`'s unit `{}` has no pinned digest to bind a composition to",
                row.file
            )
        })?;
        if &source.unit_sha256 != pinned {
            return Err(format!(
                "`{function}` carries a composition derived from unit bytes {} rather than its \
                 pinned unit {pinned}",
                source.unit_sha256
            ));
        }
        if self
            .dependency_units
            .iter()
            .any(|(path, digest)| path.trim().is_empty() || digest.trim().is_empty())
        {
            return Err(format!(
                "`{function}` names a dependency unit without a digest"
            ));
        }
        if self.checked_paths.is_empty() {
            return Err(format!(
                "`{function}` carries a composition that checks no source path"
            ));
        }
        let mut seen = BTreeSet::new();
        for path in &self.checked_paths {
            if path.trim().is_empty() || !seen.insert(path.as_str()) {
                return Err(format!(
                    "`{function}` names an empty or duplicate source path"
                ));
            }
        }
        for uncovered in &self.uncovered_paths {
            if uncovered.path.trim().is_empty() || !seen.insert(uncovered.path.as_str()) {
                return Err(format!(
                    "`{function}` names an empty or duplicate source path"
                ));
            }
            if uncovered.reason.trim().is_empty() {
                return Err(format!(
                    "`{function}` leaves source path `{}` uncovered without a reason",
                    uncovered.path
                ));
            }
        }
        Ok(())
    }
}

/// How far a composed model program reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionKind {
    /// An accepted [`check_chunk_program`] result: nothing is left live.
    Terminal,
    /// An executed prefix that discharges some legs and keeps the rest of the
    /// chain live. Its retained identities are recorded, not elided.
    Prefix,
}

/// Model legs derived from a program this module re-ran itself.
///
/// There is no constructor that takes the legs as an argument. The only ways to
/// obtain one are [`ModelComposition::terminal`], which runs the kernel checker
/// and reads the discharge out of the accepted terminal state, and
/// [`ModelComposition::prefix`], which executes the composed operations and
/// records exactly the legs that execution reaches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelComposition {
    program_digest: String,
    kind: CompositionKind,
    operations: usize,
    /// The legs the re-run program reaches. They count as discharged for a
    /// function only when its [`SourceDerivation`] composes every source path.
    #[serde(rename = "reached_legs")]
    discharged: BTreeSet<LifecycleLeg>,
    retained_live: Vec<String>,
}

impl ModelComposition {
    /// Re-check `program` and derive the model legs it discharges.
    ///
    /// A refused program, or one whose trace records no teardown at all, is
    /// rejected: an empty program must not vacuously discharge seven legs.
    pub fn terminal(
        program_digest: impl Into<String>,
        program: &ChunkProgram,
    ) -> Result<Self, String> {
        let program_digest = digest_of(program_digest)?;
        let certificate: ChunkCertificate = check_chunk_program(program)
            .map_err(|diagnostics| format!("model program is not accepted: {diagnostics:?}"))?;
        let discharged = derive_model_legs(&certificate.final_state, &certificate.trace);
        let outstanding: Vec<_> = MODEL_LEGS
            .iter()
            .copied()
            .filter(|leg| !discharged.contains(leg))
            .map(LifecycleLeg::name)
            .collect();
        if !outstanding.is_empty() {
            return Err(format!(
                "accepted model program records no teardown for: {}",
                outstanding.join(", ")
            ));
        }
        Ok(Self {
            program_digest,
            kind: CompositionKind::Terminal,
            operations: program.operations.len(),
            discharged,
            retained_live: Vec::new(),
        })
    }

    /// Execute a composed prefix and record the legs it actually reaches.
    ///
    /// This is the shape of a function that destroys what it owns while its
    /// container stays live by premise — `cio_chunk_close` under a live
    /// context and stream. The container legs stay outstanding, which is the
    /// point: an assumed premise is not a discharge.
    pub fn prefix(
        program_digest: impl Into<String>,
        program: &ChunkProgram,
    ) -> Result<Self, String> {
        let program_digest = digest_of(program_digest)?;
        let execution = execute_chunk_prefix(&program.operations)
            .map_err(|diagnostics| format!("model prefix is not executable: {diagnostics:?}"))?;
        let discharged = derive_model_legs(&execution.final_state, &execution.trace);
        if discharged.is_empty() {
            return Err("model prefix discharges no lifecycle leg".into());
        }
        let retained_live = live_identities(&execution.final_state);
        Ok(Self {
            program_digest,
            kind: CompositionKind::Prefix,
            operations: program.operations.len(),
            discharged,
            retained_live,
        })
    }

    pub fn program_digest(&self) -> &str {
        &self.program_digest
    }

    pub fn kind(&self) -> CompositionKind {
        self.kind
    }

    pub fn operations(&self) -> usize {
        self.operations
    }

    pub fn discharged(&self) -> &BTreeSet<LifecycleLeg> {
        &self.discharged
    }

    pub fn retained_live(&self) -> &[String] {
        &self.retained_live
    }
}

fn digest_of(program_digest: impl Into<String>) -> Result<String, String> {
    let program_digest = program_digest.into();
    if program_digest.trim().is_empty() {
        return Err("model composition has an empty program digest".into());
    }
    Ok(program_digest)
}

fn live_identities(state: &ChunkState) -> Vec<String> {
    let mut live = Vec::new();
    for (context, record) in &state.contexts {
        if record.live {
            live.push(format!("context {context}"));
        }
    }
    for (stream, record) in &state.streams {
        if record.live {
            live.push(format!("stream {stream}"));
        }
    }
    for (chunk, record) in &state.chunks {
        if record.live {
            live.push(format!("chunk {chunk}"));
        }
    }
    live
}

fn derive_model_legs(state: &ChunkState, trace: &[ChunkTraceEvent]) -> BTreeSet<LifecycleLeg> {
    let mut legs = BTreeSet::new();
    let saw = |matcher: fn(&ChunkTraceEvent) -> bool| trace.iter().any(matcher);
    let contexts_gone = !state.contexts.is_empty() && state.contexts.values().all(|c| !c.live);
    if contexts_gone && saw(|event| matches!(event, ChunkTraceEvent::ContextDestroyed(_))) {
        legs.insert(LifecycleLeg::ContextTeardown);
    }
    let streams_gone = !state.streams.is_empty() && state.streams.values().all(|s| !s.live);
    if streams_gone && saw(|event| matches!(event, ChunkTraceEvent::StreamDestroyed(_))) {
        legs.insert(LifecycleLeg::StreamTeardown);
    }
    let chunks_gone = !state.chunks.is_empty() && state.chunks.values().all(|c| !c.live);
    if chunks_gone && saw(|event| matches!(event, ChunkTraceEvent::ChunkClosed(_))) {
        legs.insert(LifecycleLeg::ChunkTeardown);
    }
    let backends_gone = !state.chunks.is_empty()
        && state.chunks.values().all(|chunk| {
            !chunk.backend_object_live
                && !chunk.memory_buffer_live
                && !chunk.metadata_buffer_live
                && !chunk.file_mapping_live
                && !chunk.file_descriptor_live
                && !chunk.file_path_live
                && !chunk.name_live
        });
    if backends_gone && saw(|event| matches!(event, ChunkTraceEvent::BackendClosed(_, _))) {
        legs.insert(LifecycleLeg::BackendTeardown);
    }
    let unlinked = saw(|event| matches!(event, ChunkTraceEvent::ChunkUnlinked(_)));
    if unlinked
        && !state.chunks.is_empty()
        && state.chunks.values().all(|chunk| !chunk.primary_linked)
    {
        legs.insert(LifecycleLeg::PrimaryListUnlink);
    }
    if unlinked
        && !state.chunks.is_empty()
        && state.chunks.values().all(|chunk| !chunk.residency_linked)
    {
        legs.insert(LifecycleLeg::ResidencyListUnlink);
    }
    let counters_balanced =
        !state.contexts.is_empty() && state.contexts.values().all(|c| c.chunk_total == 0);
    if counters_balanced
        && saw(|event| matches!(event, ChunkTraceEvent::ChunkCounterDecremented(_)))
    {
        legs.insert(LifecycleLeg::ContextCounters);
    }
    legs
}

/// The post-state a delete-only operation actually reaches.
///
/// Like [`ModelComposition`], it is derived rather than asserted: the
/// retained and discharged identities are read out of the executed program.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DeleteOnlyPostState {
    program_digest: String,
    retained_live: Vec<String>,
    discharged: Vec<String>,
    backing_delete_outcomes: Vec<String>,
}

impl DeleteOnlyPostState {
    /// Execute `program` and record what it leaves live.
    ///
    /// The program must actually delete content and must actually retain a
    /// live identity; otherwise it is not a delete-only operation and is
    /// rejected rather than silently classified as one.
    pub fn from_program(
        program_digest: impl Into<String>,
        program: &ChunkProgram,
    ) -> Result<Self, String> {
        let program_digest = digest_of(program_digest)
            .map_err(|_| "delete-only post-state has an empty program digest".to_owned())?;
        let execution = execute_chunk_prefix(&program.operations).map_err(|diagnostics| {
            format!("delete-only program is not executable: {diagnostics:?}")
        })?;
        let deletions: Vec<String> = execution
            .trace
            .iter()
            .filter_map(|event| match event {
                ChunkTraceEvent::StreamContentDeleted(stream, outcome) => {
                    Some(format!("stream {stream}: {outcome:?}"))
                }
                _ => None,
            })
            .collect();
        if deletions.is_empty() {
            return Err("delete-only post-state requires a recorded content deletion".into());
        }
        let state = execution.final_state;
        let mut retained_live = Vec::new();
        let mut discharged = Vec::new();
        for (context, record) in &state.contexts {
            if record.live {
                retained_live.push(format!("context {context}"));
            } else {
                discharged.push(format!("context {context}"));
            }
        }
        for (stream, record) in &state.streams {
            if record.live {
                retained_live.push(format!("stream {stream}"));
            } else {
                discharged.push(format!("stream {stream}"));
            }
        }
        for (chunk, record) in &state.chunks {
            if record.live {
                retained_live.push(format!("chunk {chunk}"));
            } else {
                discharged.push(format!("chunk {chunk}"));
            }
        }
        if retained_live.is_empty() {
            return Err(
                "delete-only post-state retains no live identity; it is not a partial operation"
                    .into(),
            );
        }
        Ok(Self {
            program_digest,
            retained_live,
            discharged,
            backing_delete_outcomes: deletions,
        })
    }

    pub fn retained_live(&self) -> &[String] {
        &self.retained_live
    }

    pub fn discharged(&self) -> &[String] {
        &self.discharged
    }
}

/// One selected function's lifecycle evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleEvidence {
    pub identity: FunctionIdentity,
    pub role: LifecycleRole,
    /// Why the pinned projection could not classify this function. Required
    /// for [`LifecycleRole::Unclassified`] and rejected otherwise.
    pub projection_reason: Option<String>,
    /// The re-checked model composition, when one was supplied.
    pub model: Option<ModelComposition>,
    /// The derived partial post-state of a delete-only operation.
    pub delete_only: Option<DeleteOnlyPostState>,
    /// Source-derived evidence that every error exit is modeled.
    pub error_exits_modeled: bool,
    /// Source-derived evidence that the wrapper's custody is discharged.
    pub wrapper_obligations_discharged: bool,
    /// Where `model` or `delete_only` was derived from, and which source paths
    /// it composes. Required whenever either is supplied, rejected otherwise.
    pub derivation: Option<SourceDerivation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ChunkLifecycleStatus {
    Safe {
        discharged_legs: Vec<LifecycleLeg>,
        model_program_digest: String,
    },
    Unknown {
        reason: String,
        outstanding_legs: Vec<LifecycleLeg>,
    },
}

impl ChunkLifecycleStatus {
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::Safe { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChunkLifecycleCoverage {
    #[serde(flatten)]
    pub identity: FunctionIdentity,
    pub role: LifecycleRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelComposition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_only_post_state: Option<DeleteOnlyPostState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation: Option<SourceDerivation>,
    pub lifecycle: ChunkLifecycleStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutstandingLegTotal {
    pub leg: LifecycleLeg,
    pub functions: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChunkLifecycleCoverageReport {
    pub property: String,
    pub semantic_profile: String,
    pub scope: String,
    pub bounds: Vec<String>,
    pub assumptions: BTreeMap<String, String>,
    pub evidence_version: String,
    #[serde(flatten)]
    pub source_binding: SourceBinding,
    pub selected_functions: usize,
    pub accounted_functions: usize,
    pub safe_functions: usize,
    pub unknown_functions: usize,
    pub role_totals: BTreeMap<String, usize>,
    pub outstanding_leg_totals: Vec<OutstandingLegTotal>,
    pub functions: Vec<ChunkLifecycleCoverage>,
}

/// The published semantic profile of every row this module decides.
pub const SEMANTIC_PROFILE: &str = "qcore.chunk_lifecycle.v1";
/// The evidence version of this accounting route.
///
/// Version 2 (card `W43-CHUNK-SOURCE-PROFILE`) binds every composition to the
/// row's own pinned unit and counts model legs only over complete source paths.
pub const EVIDENCE_VERSION: &str = "chunk-lifecycle-coverage-2";

/// Account for every selected chunk function without letting any leg be
/// asserted rather than checked.
pub fn account_chunk_lifecycle_coverage(
    source_binding: SourceBinding,
    selected: &[FunctionIdentity],
    assumptions: BTreeMap<String, String>,
    evidence: Vec<LifecycleEvidence>,
) -> Result<ChunkLifecycleCoverageReport, String> {
    source_binding.validate()?;
    if selected.is_empty() {
        return Err("lifecycle selection names no function".into());
    }
    // The verdict contract requires assumptions to be carried. An empty map
    // would publish "this route assumes nothing", which is false of every
    // route that reads an imported C projection.
    if assumptions.is_empty() || assumptions.values().any(|value| value.trim().is_empty()) {
        return Err("lifecycle accounting must carry its assumptions".into());
    }
    let mut wanted = BTreeSet::new();
    for identity in selected {
        for (label, value) in [
            ("library", &identity.library),
            ("file", &identity.file),
            ("function", &identity.function),
        ] {
            if value.trim().is_empty() {
                return Err(format!("selected function has an empty {label}"));
            }
        }
        if !wanted.insert(identity.key()) {
            return Err(format!(
                "selected inventory names `{}` twice",
                identity.function
            ));
        }
    }

    let mut rows = BTreeMap::new();
    for row in evidence {
        let key = row.identity.key();
        if !wanted.contains(&key) {
            return Err(format!(
                "lifecycle evidence names unselected function `{}`",
                row.identity.function
            ));
        }
        if rows.insert(key, row).is_some() {
            return Err("lifecycle evidence names a selected function twice".into());
        }
    }
    if rows.len() != wanted.len() {
        return Err("lifecycle evidence omits a selected function".into());
    }

    let mut functions = Vec::with_capacity(selected.len());
    let mut role_totals: BTreeMap<String, usize> = BTreeMap::new();
    let mut outstanding_totals: BTreeMap<LifecycleLeg, usize> = BTreeMap::new();
    for identity in selected {
        let row = rows.remove(&identity.key()).expect("checked membership");
        let lifecycle = decide(&row, &source_binding)?;
        if let ChunkLifecycleStatus::Unknown {
            outstanding_legs, ..
        } = &lifecycle
        {
            for leg in outstanding_legs {
                *outstanding_totals.entry(*leg).or_default() += 1;
            }
        }
        *role_totals
            .entry(role_name(row.role).to_owned())
            .or_default() += 1;
        functions.push(ChunkLifecycleCoverage {
            identity: identity.clone(),
            role: row.role,
            projection_reason: row.projection_reason,
            model: row.model,
            delete_only_post_state: row.delete_only,
            derivation: row.derivation,
            lifecycle,
        });
    }

    let safe_functions = functions.iter().filter(|r| r.lifecycle.is_safe()).count();
    Ok(ChunkLifecycleCoverageReport {
        property: "bounded chunk lifecycle post-state".into(),
        semantic_profile: SEMANTIC_PROFILE.into(),
        scope: "the selected input-chunk wrapper and cio_chunk functions of the pinned \
                forced-Libevent Fluent Bit build; no other translation unit, build \
                configuration or caller is covered"
            .into(),
        bounds: vec![
            "nine composition legs; seven are derived from a re-checked model program and \
             two from source evidence"
                .into(),
            "a delete-only operation keeps its executed partial post-state and is never Safe"
                .into(),
            "model identities are bounded by the kernel's own identity profile".into(),
            "field contracts are not lifecycle results and are not read as such".into(),
            "a composition counts for a function only when it was derived from that function's \
             own pinned unit and composes every one of its source paths; otherwise its legs are \
             reached on the checked paths only and each uncovered path stays Unknown"
                .into(),
        ],
        assumptions,
        evidence_version: EVIDENCE_VERSION.into(),
        source_binding,
        selected_functions: selected.len(),
        accounted_functions: functions.len(),
        safe_functions,
        unknown_functions: functions.len() - safe_functions,
        role_totals,
        outstanding_leg_totals: outstanding_totals
            .into_iter()
            .map(|(leg, functions)| OutstandingLegTotal { leg, functions })
            .collect(),
        functions,
    })
}

fn role_name(role: LifecycleRole) -> &'static str {
    match role {
        LifecycleRole::DestructionDependent => "destruction_dependent",
        LifecycleRole::DeleteOnly => "delete_only",
        LifecycleRole::NonDestructive => "non_destructive",
        LifecycleRole::Unclassified => "unclassified",
    }
}

fn decide(
    row: &LifecycleEvidence,
    binding: &SourceBinding,
) -> Result<ChunkLifecycleStatus, String> {
    let function = &row.identity.function;
    // Structural rejections first: a malformed row is an operation error, not
    // an Unknown that could be mistaken for a measured residual.
    match row.role {
        LifecycleRole::Unclassified => {
            if row
                .projection_reason
                .as_deref()
                .is_none_or(|reason| reason.trim().is_empty())
            {
                return Err(format!(
                    "unclassified function `{function}` lacks a projection reason"
                ));
            }
            if row.model.is_some() || row.delete_only.is_some() || row.derivation.is_some() {
                return Err(format!(
                    "unclassified function `{function}` carries lifecycle evidence it has no role for"
                ));
            }
            if row.error_exits_modeled || row.wrapper_obligations_discharged {
                return Err(format!(
                    "unclassified function `{function}` claims source legs without a role"
                ));
            }
        }
        LifecycleRole::DeleteOnly => {
            if row.delete_only.is_none() {
                return Err(format!(
                    "delete-only function `{function}` supplies no partial post-state"
                ));
            }
            if row.model.is_some() {
                return Err(format!(
                    "delete-only function `{function}` cannot also carry a terminal teardown certificate"
                ));
            }
            if row.projection_reason.is_some() {
                return Err(format!(
                    "classified function `{function}` carries an unclassified projection reason"
                ));
            }
        }
        LifecycleRole::DestructionDependent | LifecycleRole::NonDestructive => {
            if row.delete_only.is_some() {
                return Err(format!(
                    "function `{function}` carries a delete-only post-state without the role"
                ));
            }
            if row.projection_reason.is_some() {
                return Err(format!(
                    "classified function `{function}` carries an unclassified projection reason"
                ));
            }
            if row.role == LifecycleRole::NonDestructive && row.model.is_some() {
                return Err(format!(
                    "non-destructive function `{function}` carries a teardown certificate"
                ));
            }
        }
    }
    // W43: whatever a row composes must come from its own pinned unit.
    match (
        &row.derivation,
        row.model.is_some() || row.delete_only.is_some(),
    ) {
        (Some(derivation), true) => derivation.validate(&row.identity, binding)?,
        (None, true) => {
            return Err(format!(
                "`{function}` carries a composition without the source it was derived from"
            ));
        }
        (Some(_), false) => {
            return Err(format!(
                "`{function}` names a source derivation but carries no composition"
            ));
        }
        (None, false) => {}
    }
    let paths_complete = row
        .derivation
        .as_ref()
        .is_none_or(SourceDerivation::paths_complete);

    // Model legs count only when every source path is composed. A composition
    // over some paths reaches its legs there and discharges nothing for the
    // function; the row keeps them in `model` and lists them as outstanding.
    let mut discharged: BTreeSet<LifecycleLeg> = row
        .model
        .as_ref()
        .filter(|_| paths_complete)
        .map(|model| model.discharged().clone())
        .unwrap_or_default();
    if row.error_exits_modeled {
        discharged.insert(LifecycleLeg::ErrorExits);
    }
    if row.wrapper_obligations_discharged {
        discharged.insert(LifecycleLeg::WrapperObligations);
    }
    let outstanding_legs: Vec<LifecycleLeg> = REQUIRED_LEGS
        .iter()
        .copied()
        .filter(|leg| !discharged.contains(leg))
        .collect();

    let uncovered = row
        .derivation
        .as_ref()
        .filter(|derivation| !derivation.paths_complete())
        .map(|derivation| {
            format!(
                "; {} source path(s) are not composed ({}), so the legs the composition reaches \
                 on its {} checked path(s) discharge nothing for the function",
                derivation.uncovered_paths.len(),
                derivation
                    .uncovered_paths
                    .iter()
                    .map(|path| format!("{}: {}", path.path, path.reason))
                    .collect::<Vec<_>>()
                    .join("; "),
                derivation.checked_paths.len()
            )
        })
        .unwrap_or_default();

    let reason = match row.role {
        LifecycleRole::Unclassified => format!(
            "no source-derived lifecycle role; {}",
            row.projection_reason
                .as_deref()
                .expect("checked projection reason")
        ),
        LifecycleRole::NonDestructive => "the modeled post-state contains no destruction, so no \
             teardown composition establishes this function's lifecycle result"
            .to_owned(),
        LifecycleRole::DeleteOnly => {
            let post = row.delete_only.as_ref().expect("checked post-state");
            format!(
                "delete-only operation: content deletion is recorded and {} identity/identities \
                 remain live ({}); container teardown is not established{uncovered}",
                post.retained_live().len(),
                post.retained_live().join(", ")
            )
        }
        LifecycleRole::DestructionDependent => {
            if outstanding_legs.is_empty() {
                // Unreachable while the seven model legs only arrive with a
                // terminal binding, but the branch stays explicit rather than
                // letting a future evidence source fall through to Safe.
                let Some(model) = row.model.as_ref() else {
                    return Err(format!(
                        "function `{function}` claims every leg without a model program binding"
                    ));
                };
                if model.kind() != CompositionKind::Terminal {
                    return Err(format!(
                        "function `{function}` claims every leg from a non-terminal composition"
                    ));
                }
                if !paths_complete {
                    return Err(format!(
                        "function `{function}` claims every leg from a path-incomplete composition"
                    ));
                }
                String::new()
            } else {
                let retained = row
                    .model
                    .as_ref()
                    .filter(|model| !model.retained_live().is_empty())
                    .map(|model| {
                        format!(
                            "; the composition leaves {} live by premise",
                            model.retained_live().join(", ")
                        )
                    })
                    .unwrap_or_default();
                format!(
                    "{} of {} composition legs are outstanding: {}{retained}{uncovered}",
                    outstanding_legs.len(),
                    REQUIRED_LEGS.len(),
                    outstanding_legs
                        .iter()
                        .map(|leg| leg.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    };

    if row.role == LifecycleRole::DestructionDependent && outstanding_legs.is_empty() {
        let model = row.model.as_ref().expect("checked model binding");
        return Ok(ChunkLifecycleStatus::Safe {
            discharged_legs: REQUIRED_LEGS.to_vec(),
            model_program_digest: model.program_digest().to_owned(),
        });
    }
    Ok(ChunkLifecycleStatus::Unknown {
        reason,
        outstanding_legs,
    })
}
