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

//! Verifier-plugin protocol v1 data model.
//!
//! Moved verbatim from `qcore-compiler`'s `verifier_plugin` module (W0 of
//! `docs/GENERALIZATION_PLAN.md`): manifests, capability and target
//! declarations, request/response frames, and proposed evidence. Providers
//! are deliberately outside the trusted computing base; the trusted core's
//! firewall, registry, and certificate types stay in the compiler crate.
//! Protocol v1 carries only caller assertions, so the core refuses all v1
//! `Safe` proposals. Manifest capabilities describe routing, not verified
//! support.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::obligation::Obligation;
use crate::witness::{PLUGIN_EVIDENCE_VERSION_V2, SemanticWitness};

pub const PLUGIN_PROTOCOL_VERSION: u16 = 1;
pub const PLUGIN_EVIDENCE_VERSION: u16 = 1;
pub const DEFAULT_MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginId(String);

impl PluginId {
    pub fn new(value: impl Into<String>) -> Result<Self, ManifestError> {
        let value = value.into();
        let valid = value.len() <= 128
            && value.split('.').count() >= 2
            && value.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 48
                    && part
                        .chars()
                        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(ManifestError::InvalidPluginId(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierCapability {
    ResourceContracts,
    EventTraces,
    ChunkLifecycle,
    Concurrency,
    TranslationUnitProjection,
    ObservationHooks,
}

impl VerifierCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ResourceContracts => "resource_contracts",
            Self::EventTraces => "event_traces",
            Self::ChunkLifecycle => "chunk_lifecycle",
            Self::Concurrency => "concurrency",
            Self::TranslationUnitProjection => "translation_unit_projection",
            Self::ObservationHooks => "observation_hooks",
        }
    }
}

impl fmt::Display for VerifierCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for VerifierCapability {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "resource_contracts" => Ok(Self::ResourceContracts),
            "event_traces" => Ok(Self::EventTraces),
            "chunk_lifecycle" => Ok(Self::ChunkLifecycle),
            "concurrency" => Ok(Self::Concurrency),
            "translation_unit_projection" => Ok(Self::TranslationUnitProjection),
            "observation_hooks" => Ok(Self::ObservationHooks),
            _ => Err(format!("unknown verifier capability `{value}`")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginTrustClass {
    FirstParty,
    ThirdParty,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginEntrypoint {
    BuiltIn,
    Executable { path: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginLimits {
    pub timeout_ms: u64,
    pub memory_bytes: u64,
    /// Maximum JSON payload bytes for both requests and responses; the
    /// four-byte frame header is additional. Stderr has a separate 64 KiB cap.
    pub output_bytes: usize,
    /// Additional declared limit *kinds*, keyed by kind name.
    ///
    /// This field is a declaration, not an enforcement mechanism: whether any
    /// kind here (or `memory_bytes` above) is actually applied depends on the
    /// host's process execution profile. A profile that cannot enforce a
    /// declared kind must refuse the package rather than run it unenforced.
    /// Empty by default and omitted from the serialized manifest when empty,
    /// so existing manifests round-trip byte-identically.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub additional: BTreeMap<String, u64>,
}

impl Default for PluginLimits {
    fn default() -> Self {
        Self {
            timeout_ms: 5_000,
            memory_bytes: 256 * 1024 * 1024,
            output_bytes: DEFAULT_MAX_FRAME_BYTES,
            additional: BTreeMap::new(),
        }
    }
}

/// Limit-kind names that name a mandatory [`PluginLimits`] field. Repeating one
/// in `additional` would leave two declarations of the same kind, so it is
/// rejected rather than silently resolved in favor of either.
const RESERVED_LIMIT_KINDS: [&str; 3] = ["timeout_ms", "memory_bytes", "output_bytes"];

fn valid_limit_kind(kind: &str) -> bool {
    (1..=64).contains(&kind.len())
        && kind
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && !RESERVED_LIMIT_KINDS.contains(&kind)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportedTarget {
    pub library: String,
    pub revisions: BTreeSet<String>,
    pub profiles: BTreeSet<String>,
    pub dialects: BTreeSet<String>,
    pub platforms: BTreeSet<String>,
    pub backends: BTreeSet<String>,
}

impl SupportedTarget {
    fn matches(&self, requested: &TargetProfile) -> bool {
        fn includes(values: &BTreeSet<String>, value: &str) -> bool {
            values.contains("*") || values.contains(value)
        }
        self.library == requested.library
            && includes(&self.revisions, &requested.revision)
            && includes(&self.profiles, &requested.profile)
            && includes(&self.dialects, &requested.dialect)
            && includes(&self.platforms, &requested.platform)
            && includes(&self.backends, &requested.backend)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub plugin_id: PluginId,
    pub package_version: String,
    pub protocol_versions: BTreeSet<u16>,
    pub evidence_versions: BTreeSet<u16>,
    pub entrypoint: PluginEntrypoint,
    pub package_digest: String,
    pub targets: Vec<SupportedTarget>,
    pub capabilities: BTreeSet<VerifierCapability>,
    pub required_foreign_summaries: BTreeSet<String>,
    pub required_observation_hooks: BTreeSet<String>,
    pub filesystem_inputs: BTreeSet<String>,
    pub network_access: bool,
    pub limits: PluginLimits,
    pub trust_class: PluginTrustClass,
    pub certificate_checker: String,
    pub lean_theorem_family: String,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        PluginId::new(self.plugin_id.as_str())?;
        if !valid_semver(&self.package_version) {
            return Err(ManifestError::InvalidPackageVersion(
                self.package_version.clone(),
            ));
        }
        if self.protocol_versions.is_empty() || self.evidence_versions.is_empty() {
            return Err(ManifestError::MissingVersions);
        }
        if self.targets.is_empty() || self.capabilities.is_empty() {
            return Err(ManifestError::EmptyScope);
        }
        if self.network_access {
            return Err(ManifestError::NetworkAccessNotAllowed);
        }
        if self.limits.timeout_ms == 0
            || self.limits.memory_bytes == 0
            || self.limits.output_bytes == 0
            || self.limits.output_bytes > 64 * 1024 * 1024
            || self
                .limits
                .additional
                .keys()
                .any(|kind| !valid_limit_kind(kind))
        {
            return Err(ManifestError::InvalidLimits);
        }
        if self.certificate_checker.is_empty() || self.lean_theorem_family.is_empty() {
            return Err(ManifestError::MissingTrustBinding);
        }
        match &self.entrypoint {
            PluginEntrypoint::BuiltIn => {
                if self.package_digest != format!("builtin:{}", self.plugin_id) {
                    return Err(ManifestError::InvalidPackageDigest);
                }
            }
            PluginEntrypoint::Executable { path } => {
                if !safe_relative_path(path)
                    || self.package_digest.len() != 64
                    || !self
                        .package_digest
                        .chars()
                        .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
                {
                    return Err(ManifestError::InvalidPackageDigest);
                }
            }
        }
        if self
            .filesystem_inputs
            .iter()
            .any(|path| !safe_relative_path(path))
        {
            return Err(ManifestError::InvalidTarget);
        }
        for target in &self.targets {
            if target.library.is_empty()
                || target.revisions.is_empty()
                || target.profiles.is_empty()
                || target.dialects.is_empty()
                || target.platforms.is_empty()
                || target.backends.is_empty()
            {
                return Err(ManifestError::InvalidTarget);
            }
        }
        Ok(())
    }

    pub fn supports(&self, capability: VerifierCapability, target: &TargetProfile) -> bool {
        self.capabilities.contains(&capability)
            && self
                .targets
                .iter()
                .any(|candidate| candidate.matches(target))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestError {
    InvalidPluginId(String),
    InvalidPackageVersion(String),
    MissingVersions,
    EmptyScope,
    NetworkAccessNotAllowed,
    InvalidLimits,
    MissingTrustBinding,
    InvalidPackageDigest,
    InvalidTarget,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPluginId(id) => write!(formatter, "invalid plugin ID `{id}`"),
            Self::InvalidPackageVersion(version) => {
                write!(formatter, "invalid package version `{version}`")
            }
            Self::MissingVersions => formatter.write_str("manifest has no protocol versions"),
            Self::EmptyScope => formatter.write_str("manifest has no targets or capabilities"),
            Self::NetworkAccessNotAllowed => {
                formatter.write_str("verifier plugins may not request network access")
            }
            Self::InvalidLimits => formatter.write_str("invalid plugin resource limits"),
            Self::MissingTrustBinding => {
                formatter.write_str("manifest is missing its core checker or Lean theorem family")
            }
            Self::InvalidPackageDigest => formatter.write_str("invalid package digest"),
            Self::InvalidTarget => formatter.write_str("invalid supported target"),
        }
    }
}

impl std::error::Error for ManifestError {}

fn valid_semver(value: &str) -> bool {
    let core = value.split_once('-').map_or(value, |(core, _)| core);
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitIdentity {
    pub source_digest: String,
    pub build_digest: String,
    pub translation_unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetProfile {
    pub library: String,
    pub revision: String,
    pub profile: String,
    pub dialect: String,
    pub platform: String,
    pub backend: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactPredicate {
    ApiRecognized,
    IdentityUnique,
    ResourceBalanced,
    LifecycleTerminal,
    ConcurrencySafe,
    ObservationMatched,
    ProjectionComplete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedFact {
    pub fact_id: String,
    pub subject: String,
    pub predicate: FactPredicate,
    pub holds: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRequest {
    pub protocol_version: u16,
    pub evidence_version: u16,
    pub request_id: String,
    pub plugin_id: PluginId,
    pub capability: VerifierCapability,
    pub target: TargetProfile,
    pub unit: UnitIdentity,
    pub facts: Vec<NormalizedFact>,
    pub assumptions: BTreeMap<String, bool>,
}

impl VerificationRequest {
    pub fn validate(&self) -> Result<(), RequestError> {
        if self.request_id.is_empty()
            || self.unit.translation_unit.is_empty()
            || !valid_digest(&self.unit.source_digest)
            || !valid_digest(&self.unit.build_digest)
            || PluginId::new(self.plugin_id.as_str()).is_err()
        {
            return Err(RequestError::InvalidIdentity);
        }
        let ids = self
            .facts
            .iter()
            .map(|fact| fact.fact_id.as_str())
            .collect::<BTreeSet<_>>();
        if ids.len() != self.facts.len()
            || self
                .facts
                .iter()
                .any(|fact| fact.fact_id.is_empty() || fact.subject.is_empty())
        {
            return Err(RequestError::InvalidFacts);
        }
        Ok(())
    }
}

/// The evidence-v2 request frame (design §1.1).
///
/// The protocol frame version and framing are unchanged; only the evidence
/// schema moves, so v2 is expressed as a *parallel* frame type rather than as
/// optional fields on [`VerificationRequest`]. This keeps every v1 frame
/// byte-identical, makes it structurally impossible for a v1 envelope to
/// carry a witness, and satisfies design open question 8's requirement that
/// the v2 fields are omitted entirely when negotiating evidence v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRequestV2 {
    pub protocol_version: u16,
    pub evidence_version: u16,
    pub request_id: String,
    pub plugin_id: PluginId,
    pub capability: VerifierCapability,
    pub target: TargetProfile,
    pub unit: UnitIdentity,
    /// Retained for envelope compatibility. In v2 the obligation inventory,
    /// not this list, is the obligation carrier.
    pub facts: Vec<NormalizedFact>,
    pub assumptions: BTreeMap<String, bool>,
    /// Digest of the core's `VerificationTask`; every witness must be bound
    /// to it.
    pub task_digest: String,
    /// The core-derived obligation inventory. A provider proposes witnesses,
    /// never this list.
    pub obligations: Vec<Obligation>,
    /// Optional, separately digest-bound source input for the explicit Boolean
    /// field contract profile. Omitted for existing v2 exchanges and every v1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_effect: Option<crate::chunk_summary::BooleanFieldSourceInput>,
    /// Optional, separately digest-bound source projection for the explicit
    /// unsigned counter contract profile. Omitted for every other exchange,
    /// and never present together with `source_effect`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter_effect: Option<crate::counter_summary::CounterSourceInput>,
    /// Optional, separately digest-bound program projection for one admitted
    /// reactor callback family. Omitted for every other exchange, and never
    /// present together with another source projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reactor_program: Option<crate::reactor_program::ReactorProgramInput>,
    /// Optional, separately digest-bound source projection for one admitted
    /// ChunkIO teardown family. Omitted for every other exchange, and never
    /// present together with another projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teardown_source: Option<crate::teardown_summary::TeardownSourceInput>,
    /// Optional, separately digest-bound call-result def/use projection for one
    /// admitted cleanup-attempt family. Omitted for every other exchange, and
    /// never present together with another projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_use_source: Option<crate::call_use_summary::CallUseSourceInput>,
    /// Optional, separately digest-bound symbolic-replay projection for one
    /// admitted ChunkIO cleanup-outcome family. Omitted for every other
    /// exchange, and never present together with another projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanup_replay_source: Option<crate::cleanup_replay_summary::CleanupReplaySourceInput>,
    /// Optional, separately digest-bound composition projection for one
    /// admitted ChunkIO composition family. Omitted for every other exchange,
    /// and never present together with another projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_source: Option<crate::composition_summary::CompositionSourceInput>,
}

impl VerificationRequestV2 {
    /// The v1-shaped envelope this request carries, used for the identity and
    /// envelope checks that v2 inherits from v1.
    #[must_use]
    pub fn base(&self) -> VerificationRequest {
        VerificationRequest {
            protocol_version: self.protocol_version,
            evidence_version: self.evidence_version,
            request_id: self.request_id.clone(),
            plugin_id: self.plugin_id.clone(),
            capability: self.capability,
            target: self.target.clone(),
            unit: self.unit.clone(),
            facts: self.facts.clone(),
            assumptions: self.assumptions.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), RequestError> {
        self.base().validate()?;
        if self.protocol_version != PLUGIN_PROTOCOL_VERSION
            || self.evidence_version != PLUGIN_EVIDENCE_VERSION_V2
            || !valid_digest(&self.task_digest)
        {
            return Err(RequestError::InvalidObligations);
        }
        let ids = self
            .obligations
            .iter()
            .map(|obligation| obligation.obligation_id.as_str())
            .collect::<BTreeSet<_>>();
        if ids.len() != self.obligations.len()
            || self.obligations.iter().any(|obligation| {
                obligation.obligation_id.is_empty()
                    || obligation.capability != self.capability
                    || obligation.evidence_version != self.evidence_version
            })
        {
            return Err(RequestError::InvalidObligations);
        }
        let field_obligations = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.property == crate::obligation::ObligationProperty::BooleanFieldContract
            })
            .count();
        match &self.source_effect {
            Some(input)
                if input.version_matches_result()
                    && self.capability == VerifierCapability::ResourceContracts
                    && field_obligations == 1
                    && self.obligations.len() == 1
                    && matches!(&self.obligations[0].subject,
                    crate::obligation::ObligationSubject::Function { symbol, translation_unit }
                    if symbol == &input.function && translation_unit == &self.unit.translation_unit) =>
                {}
            None if field_obligations == 0 => {}
            _ => return Err(RequestError::InvalidObligations),
        }
        let counter_obligations = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.property == crate::obligation::ObligationProperty::CounterUpdateContract
            })
            .count();
        match &self.counter_effect {
            // One projection carries exactly one counter obligation for the
            // function it names, and never travels with a Boolean projection.
            Some(input)
                if input.version_matches()
                    && self.source_effect.is_none()
                    && self.capability == VerifierCapability::ResourceContracts
                    && counter_obligations == 1
                    && self.obligations.len() == 1
                    && matches!(&self.obligations[0].subject,
                    crate::obligation::ObligationSubject::Function { symbol, translation_unit }
                    if symbol == &input.function && translation_unit == &self.unit.translation_unit) =>
                {}
            None if counter_obligations == 0 => {}
            _ => return Err(RequestError::InvalidObligations),
        }
        // A teardown projection carries exactly one obligation, whose property
        // must be the one its own family decides, for the function it names.
        // It never travels with another projection.
        let teardown_property = self.obligations.iter().find_map(|obligation| {
            matches!(
                obligation.property,
                crate::obligation::ObligationProperty::ListUnlinkContract
                    | crate::obligation::ObligationProperty::FieldDestructorContract
            )
            .then_some(obligation.property)
        });
        match &self.teardown_source {
            Some(input)
                if input.version_matches()
                    && self.source_effect.is_none()
                    && self.counter_effect.is_none()
                    && self.reactor_program.is_none()
                    && self.capability == VerifierCapability::ResourceContracts
                    && self.obligations.len() == 1
                    && teardown_property
                        == Some(match &input.family {
                            crate::teardown_summary::TeardownFamilyInput::ListUnlink { .. } => {
                                crate::obligation::ObligationProperty::ListUnlinkContract
                            }
                            crate::teardown_summary::TeardownFamilyInput::FieldDestructor {
                                ..
                            } => crate::obligation::ObligationProperty::FieldDestructorContract,
                        })
                    && matches!(&self.obligations[0].subject,
                    crate::obligation::ObligationSubject::Function { symbol, translation_unit }
                    if symbol == input.subject_function()
                        && translation_unit == &self.unit.translation_unit) => {}
            None if teardown_property.is_none() => {}
            _ => return Err(RequestError::InvalidObligations),
        }
        // A call-use projection carries exactly one obligation, of its own
        // property, for the function it names, and never travels with another
        // projection.
        let call_use_obligations = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.property == crate::obligation::ObligationProperty::CallResultUseContract
            })
            .count();
        match &self.call_use_source {
            Some(input)
                if input.version_matches()
                    && self.source_effect.is_none()
                    && self.counter_effect.is_none()
                    && self.reactor_program.is_none()
                    && self.teardown_source.is_none()
                    && self.capability == VerifierCapability::ResourceContracts
                    && call_use_obligations == 1
                    && self.obligations.len() == 1
                    && matches!(&self.obligations[0].subject,
                    crate::obligation::ObligationSubject::Function { symbol, translation_unit }
                    if symbol == input.subject_function()
                        && translation_unit == &self.unit.translation_unit) => {}
            None if call_use_obligations == 0 => {}
            _ => return Err(RequestError::InvalidObligations),
        }
        // A cleanup-replay projection carries exactly one obligation, of its
        // own property, for the function it names, and never travels with
        // another projection. Its prerequisite triple must be complete: an
        // empty component is a lookup key that names nothing.
        let cleanup_replay_obligations = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.property == crate::obligation::ObligationProperty::CleanupReplayContract
            })
            .count();
        match &self.cleanup_replay_source {
            Some(input)
                if input.version_matches()
                    && input.prerequisite.is_complete()
                    && self.source_effect.is_none()
                    && self.counter_effect.is_none()
                    && self.reactor_program.is_none()
                    && self.teardown_source.is_none()
                    && self.call_use_source.is_none()
                    && self.capability == VerifierCapability::ResourceContracts
                    && cleanup_replay_obligations == 1
                    && self.obligations.len() == 1
                    && matches!(&self.obligations[0].subject,
                    crate::obligation::ObligationSubject::Function { symbol, translation_unit }
                    if symbol == input.subject_function()
                        && translation_unit == &self.unit.translation_unit) => {}
            None if cleanup_replay_obligations == 0 => {}
            _ => return Err(RequestError::InvalidObligations),
        }
        // A composition projection carries exactly one obligation, of its own
        // property, for the wrapper it names, and never travels with another
        // projection. Its prerequisite identities must each be complete and all
        // of them distinct: an empty component is a lookup key that names
        // nothing, and a repeated one names a prerequisite it has not
        // established. Its family and its prerequisite set must also agree —
        // one family's set is never read as another's, and the deleting arm's
        // delete body and delete prerequisite are present or absent together.
        let composition_obligations = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.property == crate::obligation::ObligationProperty::CompositionContract
            })
            .count();
        match &self.composition_source {
            Some(input)
                if input.version_matches()
                    && input.prerequisites.is_complete()
                    && input.prerequisites.are_distinct()
                    && input.family.matches(&input.prerequisites)
                    && self.source_effect.is_none()
                    && self.counter_effect.is_none()
                    && self.reactor_program.is_none()
                    && self.teardown_source.is_none()
                    && self.call_use_source.is_none()
                    && self.cleanup_replay_source.is_none()
                    && self.capability == VerifierCapability::ResourceContracts
                    && composition_obligations == 1
                    && self.obligations.len() == 1
                    && matches!(&self.obligations[0].subject,
                    crate::obligation::ObligationSubject::Function { symbol, translation_unit }
                    if symbol == input.subject_function()
                        && translation_unit == &self.unit.translation_unit) => {}
            None if composition_obligations == 0 => {}
            _ => return Err(RequestError::InvalidObligations),
        }
        // The quiescence obligation is the reactor projection's marker: it is
        // derived only on this route, so its presence without a projection —
        // and a projection without it — are both refused. The generation
        // obligation is *not* the marker, because the reserved event-trace
        // property predates this route and other exchanges still name it.
        let quiescence = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.property
                    == crate::obligation::ObligationProperty::CallbackQuiescentAtTeardown
            })
            .count();
        let generations = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.property
                    == crate::obligation::ObligationProperty::EventGenerationDischarged
            })
            .count();
        match &self.reactor_program {
            // One projection carries exactly the two model obligations its
            // single indivisible replay decides, over the whole unit, and
            // never travels with a source projection.
            Some(input)
                if input.version_matches()
                    && self.source_effect.is_none()
                    && self.counter_effect.is_none()
                    && self.capability == VerifierCapability::EventTraces
                    && quiescence == 1
                    && generations == 1
                    && self.obligations.len() == 2
                    && self.obligations.iter().all(|obligation| {
                        obligation.subject == crate::obligation::ObligationSubject::Unit
                    }) => {}
            None if quiescence == 0 => {}
            _ => return Err(RequestError::InvalidObligations),
        }
        Ok(())
    }

    /// Identifiers of every obligation in this request's inventory.
    #[must_use]
    pub fn coverage(&self) -> BTreeSet<String> {
        self.obligations
            .iter()
            .map(|obligation| obligation.obligation_id.clone())
            .collect()
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestError {
    InvalidIdentity,
    InvalidFacts,
    InvalidObligations,
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity => formatter.write_str("invalid request identity"),
            Self::InvalidFacts => formatter.write_str("normalized facts are invalid"),
            Self::InvalidObligations => {
                formatter.write_str("the obligation inventory does not match the evidence version")
            }
        }
    }
}

impl std::error::Error for RequestError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposedVerdict {
    Safe,
    Unsafe,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginDiagnostic {
    pub code: String,
    pub message: String,
    pub fact_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceClaim {
    pub fact_id: String,
    pub subject: String,
    pub predicate: FactPredicate,
    pub holds: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reproducibility {
    pub provider_version: String,
    pub package_digest: String,
    pub deterministic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedEvidence {
    pub evidence_version: u16,
    pub unit: UnitIdentity,
    pub capability: VerifierCapability,
    pub claims: Vec<EvidenceClaim>,
    pub coverage: BTreeSet<String>,
    pub assumptions_used: BTreeMap<String, bool>,
    pub reproducibility: Reproducibility,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationResponse {
    pub protocol_version: u16,
    pub request_id: String,
    pub plugin_id: PluginId,
    pub verdict: ProposedVerdict,
    pub diagnostics: Vec<PluginDiagnostic>,
    pub evidence: Option<ProposedEvidence>,
}

/// Evidence-v2 proposal (design §2.1). `coverage` refers to `obligation_id`s
/// from the core inventory, not to submitted fact counts, and `witness`
/// carries the semantic witness the core replays. There is no way to express
/// a v2 proposal without a witness kind, and no way to attach a witness to a
/// v1 frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedEvidenceV2 {
    pub evidence_version: u16,
    pub unit: UnitIdentity,
    pub capability: VerifierCapability,
    pub coverage: BTreeSet<String>,
    pub assumptions_used: BTreeMap<String, bool>,
    pub reproducibility: Reproducibility,
    pub witness: SemanticWitness,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationResponseV2 {
    pub protocol_version: u16,
    pub request_id: String,
    pub plugin_id: PluginId,
    pub verdict: ProposedVerdict,
    pub diagnostics: Vec<PluginDiagnostic>,
    pub evidence: Option<ProposedEvidenceV2>,
}
