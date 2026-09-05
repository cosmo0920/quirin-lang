//! Cycle 20 verifier-plugin protocol, certificate firewall, and registry.
//!
//! Providers are deliberately outside the trusted computing base. They may
//! propose a verdict and evidence, but [`VerifierRegistry`] only returns a
//! [`CoreCertificate`] after capability-specific semantic checking. Protocol v1
//! carries only caller assertions, so it cannot currently issue certificates.
//! Built-in providers use exactly the same request/response path as process
//! providers. Manifest capabilities describe routing, not verified support.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{Read, Write};
use std::path::{Component, Path};
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::frontend::SourceProgram;

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
    pub output_bytes: usize,
}

impl Default for PluginLimits {
    fn default() -> Self {
        Self {
            timeout_ms: 5_000,
            memory_bytes: 256 * 1024 * 1024,
            output_bytes: DEFAULT_MAX_FRAME_BYTES,
        }
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestError {
    InvalidIdentity,
    InvalidFacts,
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity => formatter.write_str("invalid request identity"),
            Self::InvalidFacts => formatter.write_str("normalized facts are invalid"),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreCertificate {
    plugin_id: PluginId,
    capability: VerifierCapability,
    unit: UnitIdentity,
    evidence: ProposedEvidence,
}

impl CoreCertificate {
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    pub const fn capability(&self) -> VerifierCapability {
        self.capability
    }

    pub fn unit(&self) -> &UnitIdentity {
        &self.unit
    }

    pub fn evidence(&self) -> &ProposedEvidence {
        &self.evidence
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationOutcome {
    pub verdict: ProposedVerdict,
    pub diagnostics: Vec<PluginDiagnostic>,
    pub certificate: Option<CoreCertificate>,
}

impl VerificationOutcome {
    fn unknown(code: &str, message: impl Into<String>) -> Self {
        Self {
            verdict: ProposedVerdict::Unknown,
            diagnostics: vec![PluginDiagnostic {
                code: code.to_owned(),
                message: message.into(),
                fact_id: None,
            }],
            certificate: None,
        }
    }
}

pub trait VerifierProvider: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderFailure {
    Spawn(String),
    Io(String),
    Timeout,
    Crashed(Option<i32>),
    OutputLimit,
    Protocol(String),
}

impl fmt::Display for ProviderFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(message) | Self::Io(message) | Self::Protocol(message) => {
                formatter.write_str(message)
            }
            Self::Timeout => formatter.write_str("provider timed out"),
            Self::Crashed(code) => write!(formatter, "provider exited unsuccessfully ({code:?})"),
            Self::OutputLimit => formatter.write_str("provider exceeded its output limit"),
        }
    }
}

impl std::error::Error for ProviderFailure {}

#[derive(Clone)]
pub struct BuiltInProvider {
    manifest: PluginManifest,
}

impl BuiltInProvider {
    pub fn new(manifest: PluginManifest) -> Result<Self, ManifestError> {
        manifest.validate()?;
        if !matches!(manifest.entrypoint, PluginEntrypoint::BuiltIn) {
            return Err(ManifestError::InvalidTarget);
        }
        Ok(Self { manifest })
    }
}

impl VerifierProvider for BuiltInProvider {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        // Boolean assertions cannot establish either safety or a reachable
        // violation. Keep legacy analysis routes until semantic adapters exist.
        Ok(VerificationResponse {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: ProposedVerdict::Unknown,
            diagnostics: vec![PluginDiagnostic {
                code: format!("{}.semantic_evidence_unavailable", self.manifest.plugin_id),
                message: "protocol v1 facts lack core-derived obligations and semantic witnesses"
                    .into(),
                fact_id: None,
            }],
            evidence: None,
        })
    }
}

pub struct ProcessProvider {
    manifest: PluginManifest,
    package_root: String,
}

impl ProcessProvider {
    pub fn new(
        manifest: PluginManifest,
        package_root: impl AsRef<Path>,
    ) -> Result<Self, ManifestError> {
        manifest.validate()?;
        if !matches!(manifest.entrypoint, PluginEntrypoint::Executable { .. }) {
            return Err(ManifestError::InvalidTarget);
        }
        Ok(Self {
            manifest,
            package_root: package_root.as_ref().to_string_lossy().into_owned(),
        })
    }

    pub fn from_package(package_root: impl AsRef<Path>) -> Result<Self, PackageLoadError> {
        let package_root = package_root.as_ref();
        let manifest_path = package_root.join("verifier-plugin.json");
        let bytes = std::fs::read(&manifest_path).map_err(|error| PackageLoadError {
            path: manifest_path.to_string_lossy().into_owned(),
            message: error.to_string(),
        })?;
        let manifest =
            serde_json::from_slice::<PluginManifest>(&bytes).map_err(|error| PackageLoadError {
                path: manifest_path.to_string_lossy().into_owned(),
                message: error.to_string(),
            })?;
        Self::new(manifest, package_root).map_err(|error| PackageLoadError {
            path: manifest_path.to_string_lossy().into_owned(),
            message: error.to_string(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageLoadError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for PackageLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for PackageLoadError {}

impl VerifierProvider for ProcessProvider {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure> {
        let PluginEntrypoint::Executable { path } = &self.manifest.entrypoint else {
            return Err(ProviderFailure::Protocol("missing executable".into()));
        };
        let executable = Path::new(&self.package_root).join(path);
        let mut child = Command::new(&executable)
            .current_dir(&self.package_root)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| ProviderFailure::Spawn(error.to_string()))?;
        let frame = encode_protocol_frame(request, self.manifest.limits.output_bytes)?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| ProviderFailure::Io("missing provider stdin".into()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProviderFailure::Io("missing provider stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ProviderFailure::Io("missing provider stderr".into()))?;
        let limit = self.manifest.limits.output_bytes;
        let input_thread = thread::spawn(move || {
            stdin
                .write_all(&frame)
                .map_err(|error| ProviderFailure::Io(error.to_string()))
        });
        let (sender, receiver) = mpsc::channel();
        let output_thread = thread::spawn(move || {
            let _ = sender.send(read_limited(stdout, limit));
        });
        let error_thread = thread::spawn(move || read_limited(stderr, 64 * 1024));
        let deadline = Instant::now() + Duration::from_millis(self.manifest.limits.timeout_ms);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(2)),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = input_thread.join();
                    let _ = output_thread.join();
                    let _ = error_thread.join();
                    return Err(ProviderFailure::Timeout);
                }
                Err(error) => return Err(ProviderFailure::Io(error.to_string())),
            }
        };
        input_thread
            .join()
            .map_err(|_| ProviderFailure::Io("provider input worker panicked".into()))??;
        let output = receiver
            .recv()
            .map_err(|error| ProviderFailure::Io(error.to_string()))??;
        let _ = output_thread.join();
        let error = error_thread.join().unwrap_or_else(|_| Ok(Vec::new()))?;
        if !status.success() {
            let detail = String::from_utf8_lossy(&error);
            return Err(ProviderFailure::Crashed(status.code()).with_detail(&detail));
        }
        decode_protocol_frame(&output, limit)
    }
}

trait FailureDetail {
    fn with_detail(self, detail: &str) -> Self;
}

impl FailureDetail for ProviderFailure {
    fn with_detail(self, detail: &str) -> Self {
        if detail.trim().is_empty() {
            self
        } else {
            ProviderFailure::Protocol(format!("{self}: {}", detail.trim()))
        }
    }
}

fn read_limited(mut reader: impl Read, limit: usize) -> Result<Vec<u8>, ProviderFailure> {
    let mut output = Vec::new();
    reader
        .by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut output)
        .map_err(|error| ProviderFailure::Io(error.to_string()))?;
    if output.len() > limit {
        Err(ProviderFailure::OutputLimit)
    } else {
        Ok(output)
    }
}

pub fn encode_protocol_frame<T: Serialize>(
    value: &T,
    max_bytes: usize,
) -> Result<Vec<u8>, ProviderFailure> {
    let payload =
        serde_json::to_vec(value).map_err(|error| ProviderFailure::Protocol(error.to_string()))?;
    if payload.len() > max_bytes || payload.len() > u32::MAX as usize {
        return Err(ProviderFailure::OutputLimit);
    }
    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_protocol_frame<T: for<'de> Deserialize<'de>>(
    frame: &[u8],
    max_bytes: usize,
) -> Result<T, ProviderFailure> {
    if frame.len() < 4 {
        return Err(ProviderFailure::Protocol("truncated frame header".into()));
    }
    let length = u32::from_be_bytes(frame[0..4].try_into().expect("four-byte header")) as usize;
    if length > max_bytes {
        return Err(ProviderFailure::OutputLimit);
    }
    if frame.len() != length + 4 {
        return Err(ProviderFailure::Protocol("invalid frame length".into()));
    }
    serde_json::from_slice(&frame[4..])
        .map_err(|error| ProviderFailure::Protocol(error.to_string()))
}

struct RegistryEntry {
    enabled: bool,
    provider: Arc<dyn VerifierProvider>,
}

#[derive(Default)]
pub struct VerifierRegistry {
    entries: BTreeMap<PluginId, RegistryEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedVerifierUse {
    pub plugin_id: PluginId,
    pub package_version: String,
    pub profile: String,
    pub capabilities: BTreeSet<VerifierCapability>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedVerifierScope {
    pub name: String,
    pub providers: Vec<ResolvedVerifierUse>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeResolutionError {
    pub scope: String,
    pub plugin_id: String,
    pub message: String,
}

impl fmt::Display for ScopeResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "verifier scope `{}` plugin `{}`: {}",
            self.scope, self.plugin_id, self.message
        )
    }
}

impl std::error::Error for ScopeResolutionError {}

impl VerifierRegistry {
    pub fn register(&mut self, provider: Arc<dyn VerifierProvider>) -> Result<(), RegistryError> {
        provider.manifest().validate()?;
        let id = provider.manifest().plugin_id.clone();
        if self.entries.contains_key(&id) {
            return Err(RegistryError::DuplicatePlugin(id));
        }
        self.entries.insert(
            id,
            RegistryEntry {
                enabled: true,
                provider,
            },
        );
        Ok(())
    }

    /// Discovers immediate child packages in lexical path order. Each package
    /// must contain `verifier-plugin.json`; malformed packages are reported
    /// together, while valid packages remain registered.
    pub fn discover_packages(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<usize, Vec<PackageLoadError>> {
        let root = root.as_ref();
        let mut paths = match std::fs::read_dir(root) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_dir() && path.join("verifier-plugin.json").is_file())
                .collect::<Vec<_>>(),
            Err(error) => {
                return Err(vec![PackageLoadError {
                    path: root.to_string_lossy().into_owned(),
                    message: error.to_string(),
                }]);
            }
        };
        paths.sort();
        let mut installed = 0;
        let mut errors = Vec::new();
        for path in paths {
            match ProcessProvider::from_package(&path) {
                Ok(provider) => match self.register(Arc::new(provider)) {
                    Ok(()) => installed += 1,
                    Err(error) => errors.push(PackageLoadError {
                        path: path.to_string_lossy().into_owned(),
                        message: error.to_string(),
                    }),
                },
                Err(error) => errors.push(error),
            }
        }
        if errors.is_empty() {
            Ok(installed)
        } else {
            Err(errors)
        }
    }

    pub fn manifests(&self) -> Vec<&PluginManifest> {
        self.entries
            .values()
            .map(|entry| entry.provider.manifest())
            .collect()
    }

    pub fn manifest(&self, id: &PluginId) -> Option<&PluginManifest> {
        self.entries.get(id).map(|entry| entry.provider.manifest())
    }

    pub fn set_enabled(&mut self, id: &PluginId, enabled: bool) -> Result<(), RegistryError> {
        let entry = self
            .entries
            .get_mut(id)
            .ok_or_else(|| RegistryError::MissingPlugin(id.clone()))?;
        entry.enabled = enabled;
        Ok(())
    }

    /// Resolves source-level `verify scope` declarations without invoking any
    /// provider. Unknown IDs, version mismatches, and undeclared capabilities
    /// fail closed before normalized source facts leave the compiler.
    pub fn resolve_source_scopes(
        &self,
        program: &SourceProgram,
    ) -> Result<Vec<ResolvedVerifierScope>, Vec<ScopeResolutionError>> {
        let mut resolved = Vec::new();
        let mut errors = Vec::new();
        let mut scope_names = BTreeSet::new();
        for (scope_name, scope) in program.verifier_scopes() {
            if !scope_names.insert(scope_name) {
                errors.push(scope_error(
                    scope_name,
                    "<scope>",
                    "duplicate verifier scope name",
                ));
                continue;
            }
            let mut providers = Vec::new();
            for selected in &scope.providers {
                let id = match PluginId::new(&selected.plugin_id) {
                    Ok(id) => id,
                    Err(error) => {
                        errors.push(scope_error(scope_name, &selected.plugin_id, error));
                        continue;
                    }
                };
                let Some(entry) = self.entries.get(&id) else {
                    errors.push(scope_error(
                        scope_name,
                        &selected.plugin_id,
                        "plugin is not installed",
                    ));
                    continue;
                };
                if !entry.enabled {
                    errors.push(scope_error(
                        scope_name,
                        &selected.plugin_id,
                        "plugin is disabled",
                    ));
                    continue;
                }
                let manifest = entry.provider.manifest();
                if selected.version.as_deref().is_some_and(|requirement| {
                    !version_matches(requirement, &manifest.package_version)
                }) {
                    errors.push(scope_error(
                        scope_name,
                        &selected.plugin_id,
                        format!(
                            "version requirement `{}` does not match `{}`",
                            selected.version.as_deref().unwrap_or_default(),
                            manifest.package_version
                        ),
                    ));
                    continue;
                }
                let mut capabilities = BTreeSet::new();
                let mut invalid = false;
                for name in &selected.capabilities {
                    let capability = match name.parse::<VerifierCapability>() {
                        Ok(capability) => capability,
                        Err(error) => {
                            errors.push(scope_error(scope_name, &selected.plugin_id, error));
                            invalid = true;
                            continue;
                        }
                    };
                    if !manifest.capabilities.contains(&capability) {
                        errors.push(scope_error(
                            scope_name,
                            &selected.plugin_id,
                            format!("capability `{capability}` is not declared by the plugin"),
                        ));
                        invalid = true;
                    }
                    capabilities.insert(capability);
                }
                if !manifest.targets.iter().any(|target| {
                    target.profiles.contains("*") || target.profiles.contains(&selected.profile)
                }) {
                    errors.push(scope_error(
                        scope_name,
                        &selected.plugin_id,
                        format!(
                            "profile `{}` is not declared by the plugin",
                            selected.profile
                        ),
                    ));
                    invalid = true;
                }
                if !invalid {
                    providers.push(ResolvedVerifierUse {
                        plugin_id: id,
                        package_version: manifest.package_version.clone(),
                        profile: selected.profile.clone(),
                        capabilities,
                    });
                }
            }
            resolved.push(ResolvedVerifierScope {
                name: scope_name.to_owned(),
                providers,
            });
        }
        if errors.is_empty() {
            Ok(resolved)
        } else {
            Err(errors)
        }
    }

    pub fn verify(&self, request: &VerificationRequest) -> VerificationOutcome {
        if let Err(error) = request.validate() {
            return VerificationOutcome::unknown("QP001", error.to_string());
        }
        let Some(entry) = self.entries.get(&request.plugin_id) else {
            return VerificationOutcome::unknown(
                "QP002",
                format!("plugin `{}` is not installed", request.plugin_id),
            );
        };
        if !entry.enabled {
            return VerificationOutcome::unknown(
                "QP003",
                format!("plugin `{}` is disabled", request.plugin_id),
            );
        }
        let manifest = entry.provider.manifest();
        if !manifest
            .protocol_versions
            .contains(&request.protocol_version)
            || !manifest
                .evidence_versions
                .contains(&request.evidence_version)
        {
            return VerificationOutcome::unknown("QP004", "incompatible protocol version");
        }
        if !manifest.supports(request.capability, &request.target) {
            return VerificationOutcome::unknown(
                "QP005",
                "plugin does not declare the requested capability and target profile",
            );
        }
        let response = match entry.provider.verify(request) {
            Ok(response) => response,
            Err(error) => return VerificationOutcome::unknown("QP006", error.to_string()),
        };
        self.check_response(manifest, request, response)
    }

    fn check_response(
        &self,
        manifest: &PluginManifest,
        request: &VerificationRequest,
        response: VerificationResponse,
    ) -> VerificationOutcome {
        if response.protocol_version != request.protocol_version
            || response.request_id != request.request_id
            || response.plugin_id != request.plugin_id
            || response.diagnostics.iter().any(|diagnostic| {
                !diagnostic
                    .code
                    .starts_with(&format!("{}.", manifest.plugin_id))
            })
        {
            return VerificationOutcome::unknown("QP007", "invalid plugin response envelope");
        }
        match response.verdict {
            ProposedVerdict::Unknown => VerificationOutcome {
                verdict: ProposedVerdict::Unknown,
                diagnostics: response.diagnostics,
                certificate: None,
            },
            ProposedVerdict::Unsafe => VerificationOutcome {
                verdict: ProposedVerdict::Unsafe,
                diagnostics: response.diagnostics,
                certificate: None,
            },
            ProposedVerdict::Safe => {
                let Some(evidence) = response.evidence else {
                    return VerificationOutcome::unknown("QP008", "safe result has no evidence");
                };
                if let Err(reason) = self.check_evidence(manifest, request, &evidence) {
                    return VerificationOutcome::unknown("QP009", reason);
                }
                VerificationOutcome {
                    verdict: ProposedVerdict::Safe,
                    diagnostics: response.diagnostics,
                    certificate: Some(CoreCertificate {
                        plugin_id: request.plugin_id.clone(),
                        capability: request.capability,
                        unit: request.unit.clone(),
                        evidence,
                    }),
                }
            }
        }
    }

    fn check_evidence(
        &self,
        manifest: &PluginManifest,
        request: &VerificationRequest,
        evidence: &ProposedEvidence,
    ) -> Result<(), &'static str> {
        // Enforced in the core for every provider, including process providers.
        // No independently checked no-obligations argument exists in v1.
        if request.facts.is_empty() || evidence.claims.is_empty() || evidence.coverage.is_empty() {
            return Err("empty evidence has no core-established no-obligations proof");
        }
        let requested = request
            .facts
            .iter()
            .map(|fact| (fact.fact_id.as_str(), fact))
            .collect::<BTreeMap<_, _>>();
        let coverage = requested.keys().map(|id| (*id).to_owned()).collect();
        if evidence.evidence_version != request.evidence_version
            || evidence.unit != request.unit
            || evidence.capability != request.capability
            || evidence.coverage != coverage
            || evidence.assumptions_used != request.assumptions
            || evidence.reproducibility.provider_version != manifest.package_version
            || evidence.reproducibility.package_digest != manifest.package_digest
            || !evidence.reproducibility.deterministic
            || evidence.claims.len() != request.facts.len()
        {
            return Err("the core certificate checker rejected the evidence envelope");
        }
        let mut seen = BTreeSet::new();
        let claims_match = evidence.claims.iter().all(|claim| {
            let Some(fact) = requested.get(claim.fact_id.as_str()) else {
                return false;
            };
            seen.insert(claim.fact_id.clone())
                && claim.subject == fact.subject
                && claim.predicate == fact.predicate
                && claim.holds
                && claim.holds == fact.holds
        });
        if !claims_match {
            return Err("the core certificate checker rejected evidence claims");
        }
        // Matching untrusted assertions is necessary for envelope replay but
        // never sufficient for any capability. Fail closed even for predicates
        // named ResourceBalanced/ConcurrencySafe and plausible source digests.
        // Re-enable only with a core-derived inventory, bound semantic inputs,
        // and a versioned capability-specific witness checker (reevaluation G2).
        Err("protocol v1 lacks core-derived obligations and capability-specific semantic witnesses")
    }
}

fn scope_error(scope: &str, plugin_id: &str, message: impl fmt::Display) -> ScopeResolutionError {
    ScopeResolutionError {
        scope: scope.to_owned(),
        plugin_id: plugin_id.to_owned(),
        message: message.to_string(),
    }
}

fn version_matches(requirement: &str, actual: &str) -> bool {
    if requirement == actual {
        return true;
    }
    let required = requirement
        .trim_end_matches(".*")
        .split('.')
        .collect::<Vec<_>>();
    let actual = actual.split_once('-').map_or(actual, |(core, _)| core);
    let actual = actual.split('.').collect::<Vec<_>>();
    !required.is_empty()
        && required.len() < 3
        && required
            .iter()
            .zip(actual.iter())
            .all(|(required, actual)| required == actual)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    Manifest(ManifestError),
    DuplicatePlugin(PluginId),
    MissingPlugin(PluginId),
}

impl From<ManifestError> for RegistryError {
    fn from(error: ManifestError) -> Self {
        Self::Manifest(error)
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(error) => error.fmt(formatter),
            Self::DuplicatePlugin(id) => write!(formatter, "duplicate plugin `{id}`"),
            Self::MissingPlugin(id) => write!(formatter, "missing plugin `{id}`"),
        }
    }
}

impl std::error::Error for RegistryError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositeCertificate {
    certificates: Vec<CoreCertificate>,
}

impl CompositeCertificate {
    pub fn certificates(&self) -> &[CoreCertificate] {
        &self.certificates
    }
}

pub fn compose_verification(outcomes: Vec<VerificationOutcome>) -> VerificationOutcome {
    if outcomes.is_empty() {
        return VerificationOutcome::unknown("QP010", "composition has no plugin results");
    }
    if let Some(unsafe_outcome) = outcomes
        .iter()
        .find(|outcome| outcome.verdict == ProposedVerdict::Unsafe)
    {
        return unsafe_outcome.clone();
    }
    if outcomes
        .iter()
        .any(|outcome| outcome.verdict == ProposedVerdict::Unknown)
    {
        return VerificationOutcome::unknown("QP011", "a required plugin result is unknown");
    }
    if outcomes.iter().any(|outcome| outcome.certificate.is_none()) {
        return VerificationOutcome::unknown(
            "QP014",
            "a safe plugin result is missing its core certificate",
        );
    }
    let certificates = outcomes
        .into_iter()
        .filter_map(|outcome| outcome.certificate)
        .collect::<Vec<_>>();
    let unit = certificates
        .first()
        .expect("nonempty checked composition")
        .unit();
    if certificates
        .iter()
        .any(|certificate| certificate.unit() != unit)
    {
        return VerificationOutcome::unknown(
            "QP015",
            "plugins certify different translation-unit identities",
        );
    }
    let mut assumptions = BTreeMap::<String, bool>::new();
    let mut claims = BTreeMap::<(String, FactPredicate), (String, bool)>::new();
    for certificate in &certificates {
        for (name, value) in &certificate.evidence.assumptions_used {
            if assumptions
                .insert(name.clone(), *value)
                .is_some_and(|old| old != *value)
            {
                return VerificationOutcome::unknown(
                    "QP012",
                    "plugins use conflicting assumptions",
                );
            }
        }
        for claim in &certificate.evidence.claims {
            let key = (claim.subject.clone(), claim.predicate);
            if claims
                .insert(key, (claim.fact_id.clone(), claim.holds))
                .is_some_and(|old| old.1 != claim.holds)
            {
                return VerificationOutcome::unknown("QP013", "plugins make conflicting claims");
            }
        }
    }
    // Composition itself is represented as a core certificate collection; it
    // cannot fabricate a new provider certificate. The leading certificate is
    // returned for API compatibility and all component certificates are
    // available through `compose_certificates`.
    VerificationOutcome {
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        certificate: certificates.first().cloned(),
    }
}

pub fn compose_certificates(
    outcomes: &[VerificationOutcome],
) -> Result<CompositeCertificate, Box<VerificationOutcome>> {
    let result = compose_verification(outcomes.to_vec());
    if result.verdict != ProposedVerdict::Safe {
        return Err(Box::new(result));
    }
    Ok(CompositeCertificate {
        certificates: outcomes
            .iter()
            .filter_map(|outcome| outcome.certificate.clone())
            .collect(),
    })
}

pub fn first_party_manifests() -> Vec<PluginManifest> {
    vec![
        first_party_manifest(
            "qcore.libevent",
            "libevent",
            &[
                VerifierCapability::ResourceContracts,
                VerifierCapability::EventTraces,
                VerifierCapability::Concurrency,
                VerifierCapability::ObservationHooks,
            ],
            "QCore.EventDriven",
        ),
        first_party_manifest(
            "qcore.chunkio",
            "chunkio",
            &[
                VerifierCapability::ResourceContracts,
                VerifierCapability::ChunkLifecycle,
                VerifierCapability::TranslationUnitProjection,
            ],
            "QCore.ChunkLifecycle",
        ),
        first_party_manifest(
            "qcore.cmetrics",
            "cmetrics",
            &[VerifierCapability::ResourceContracts],
            "QCore.Resource",
        ),
        first_party_manifest(
            "qcore.ctraces",
            "ctraces",
            &[VerifierCapability::ResourceContracts],
            "QCore.Resource",
        ),
        first_party_manifest(
            "qcore.librdkafka",
            "librdkafka",
            &[
                VerifierCapability::ResourceContracts,
                VerifierCapability::Concurrency,
            ],
            "QCore.Concurrency",
        ),
    ]
}

fn first_party_manifest(
    id: &str,
    library: &str,
    capabilities: &[VerifierCapability],
    theorem_family: &str,
) -> PluginManifest {
    let plugin_id = PluginId::new(id).expect("first-party plugin IDs are valid");
    PluginManifest {
        package_version: env!("CARGO_PKG_VERSION").into(),
        protocol_versions: BTreeSet::from([PLUGIN_PROTOCOL_VERSION]),
        evidence_versions: BTreeSet::from([PLUGIN_EVIDENCE_VERSION]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: format!("builtin:{plugin_id}"),
        targets: vec![SupportedTarget {
            library: library.into(),
            revisions: BTreeSet::from(["*".into()]),
            profiles: BTreeSet::from(["*".into()]),
            dialects: BTreeSet::from(["c11".into(), "qcore".into()]),
            platforms: BTreeSet::from(["*".into()]),
            backends: BTreeSet::from(["*".into()]),
        }],
        capabilities: capabilities.iter().copied().collect(),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "qcore.unsupported_assertions.v1".into(),
        lean_theorem_family: theorem_family.into(),
        plugin_id,
    }
}

pub fn first_party_registry() -> VerifierRegistry {
    let mut registry = VerifierRegistry::default();
    for manifest in first_party_manifests() {
        registry
            .register(Arc::new(
                BuiltInProvider::new(manifest).expect("first-party manifest is valid"),
            ))
            .expect("first-party plugin IDs are unique");
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicting_assumptions_make_composition_unknown() {
        // Test composition with internal fixtures. These are not certificates
        // issuable through the v1 firewall, which rejects Boolean-only evidence.
        let outcome = |assumption| {
            let unit = UnitIdentity {
                source_digest: "a".repeat(64),
                build_digest: "b".repeat(64),
                translation_unit: "fixture.c".into(),
            };
            VerificationOutcome {
                verdict: ProposedVerdict::Safe,
                diagnostics: vec![],
                certificate: Some(CoreCertificate {
                    plugin_id: PluginId::new("example.fixture").unwrap(),
                    capability: VerifierCapability::ResourceContracts,
                    unit: unit.clone(),
                    evidence: ProposedEvidence {
                        evidence_version: PLUGIN_EVIDENCE_VERSION,
                        unit,
                        capability: VerifierCapability::ResourceContracts,
                        claims: vec![],
                        coverage: BTreeSet::new(),
                        assumptions_used: BTreeMap::from([(
                            "callbacks_bounded".into(),
                            assumption,
                        )]),
                        reproducibility: Reproducibility {
                            provider_version: "0.1.0".into(),
                            package_digest: "test-fixture".into(),
                            deterministic: true,
                        },
                    },
                }),
            }
        };
        let composed = compose_verification(vec![outcome(true), outcome(false)]);
        assert_eq!(composed.verdict, ProposedVerdict::Unknown);
        assert_eq!(composed.diagnostics[0].code, "QP012");
    }
}
