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
#[cfg(unix)]
use std::io::{Read, Write};
use std::path::Path;
#[cfg(unix)]
use std::process::{Command, Stdio};
use std::sync::Arc;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use qcore_frontend::frontend::SourceProgram;

pub use qcore_schemas::obligation::{
    ChunkEntityRef, DerivationIncomplete, Obligation, ObligationProperty, ObligationScope,
    ObligationSubject, PathBounds, TaskBounds, VERIFICATION_TASK_VERSION, VerificationTask,
};
pub use qcore_schemas::plugin::{
    DEFAULT_MAX_FRAME_BYTES, EvidenceClaim, FactPredicate, ManifestError, NormalizedFact,
    PLUGIN_EVIDENCE_VERSION, PLUGIN_PROTOCOL_VERSION, PluginDiagnostic, PluginEntrypoint, PluginId,
    PluginLimits, PluginManifest, PluginTrustClass, ProposedEvidence, ProposedEvidenceV2,
    ProposedVerdict, Reproducibility, RequestError, SupportedTarget, TargetProfile, UnitIdentity,
    VerificationRequest, VerificationRequestV2, VerificationResponse, VerificationResponseV2,
    VerifierCapability,
};
pub use qcore_schemas::witness::{
    BoundOperation, ChunkBackendRecord, ChunkLifecycleWitness, ChunkOperationRecord,
    ChunkResidencyRecord, ContractObservationRecord, ContractObservationWitness, NativeRunBinding,
    ObservedRun, PLUGIN_EVIDENCE_VERSION_V2, SemanticWitness, SourceSite,
};

pub mod confinement;
pub use confinement::{AdmittedPath, Confinement, ConfinementPolicy, PathAccess, PathState};

/// The evidence-v2 firewall's capability-specific replay hook.
///
/// The *trait* belongs to the verifier host, which orchestrates but never
/// decides; every *implementation* belongs to a trusted-core crate
/// (`docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` open question 5). No
/// provider-reachable code can influence checking, and a capability with no
/// registered checker is `Unknown` (QP016), never `Safe`.
pub trait WitnessChecker {
    /// The capability this checker replays witnesses for.
    fn capability(&self) -> VerifierCapability;

    /// Replays `witness` against core-derived inputs held by the checker and
    /// returns the identifiers of the obligations the replay discharged.
    ///
    /// Implementations must satisfy the anti-vacuousness rule (design §2.2):
    /// no acceptance path may terminate in an equality comparison against
    /// provider-supplied data, and the empty case of every quantifier must be
    /// decided against a core-computed denominator.
    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreCertificate {
    plugin_id: PluginId,
    capability: VerifierCapability,
    unit: UnitIdentity,
    evidence: ProposedEvidence,
    /// Evidence v2 only: the semantic witness the core replayed, retained so
    /// the certificate records what was actually checked (including the
    /// `NativeRunBinding`s behind an observed-agreement claim).
    witness: Option<SemanticWitness>,
    /// Evidence v2 only: the bound each discharged obligation was checked
    /// under. Composition refuses to merge incomparable bounds.
    obligation_bounds: BTreeMap<String, TaskBounds>,
    /// Evidence v2 only: the digest of the core-derived task this certificate
    /// discharges obligations of.
    task_digest: Option<String>,
    task: Option<VerificationTask>,
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

    /// The replayed semantic witness, present exactly for evidence v2.
    pub fn witness(&self) -> Option<&SemanticWitness> {
        self.witness.as_ref()
    }

    /// The obligation identifiers this certificate discharges.
    pub fn coverage(&self) -> &BTreeSet<String> {
        &self.evidence.coverage
    }

    pub fn obligation_bounds(&self) -> &BTreeMap<String, TaskBounds> {
        &self.obligation_bounds
    }

    pub fn task(&self) -> Option<&VerificationTask> {
        self.task.as_ref()
    }

    pub fn task_digest(&self) -> Option<&str> {
        self.task_digest.as_deref()
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

/// Provider diagnostics use the selected manifest's complete plugin ID plus
/// exactly one local component. Like plugin IDs, that component is bounded to
/// 128 ASCII bytes, so a complete diagnostic code has at most 257 bytes.
/// A dotted suffix could impersonate a separately registered nested provider;
/// control characters could obscure attribution in rendered diagnostics.
fn valid_provider_diagnostic_code(plugin_id: &PluginId, code: &str) -> bool {
    let Some(local) = code
        .strip_prefix(plugin_id.as_str())
        .and_then(|suffix| suffix.strip_prefix('.'))
    else {
        return false;
    };
    (1..=128).contains(&local.len())
        && local
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

/// A firewall rejection with the reason code it is reported under. Distinct
/// codes keep "invalid evidence" separable from "no core checker"
/// (`docs/REEVALUATION_SPEC.md` §1's reason-code rule).
struct EvidenceRejection {
    code: &'static str,
    reason: String,
}

pub trait VerifierProvider: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationResponse, ProviderFailure>;

    /// Evidence-v2 dispatch. The default fails closed, so a provider written
    /// against protocol v1 alone can never be mistaken for one that speaks
    /// evidence v2; the manifest's `evidence_versions` gate (QP004) normally
    /// rejects such a provider before this point.
    fn verify_v2(
        &self,
        request: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        let _ = request;
        Err(ProviderFailure::Protocol(
            "provider does not implement evidence schema v2".into(),
        ))
    }
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

/// Captured stderr is bounded separately from the response payload; it is
/// diagnostic text, never decoded evidence.
pub const PROVIDER_STDERR_BYTES: usize = 64 * 1024;

/// The execution profile a [`ProcessProvider`] runs its package under.
///
/// Profiles are *target-qualified*: [`ProcessExecutionProfile::LinuxRlimitV1`]
/// enforces limits with `setrlimit`, which this crate applies only on Linux.
/// Selection is additive — the default is the historical behavior — because
/// silently changing what an existing package runs under would change what
/// existing evidence means.
///
/// No profile is a general OS sandbox.
/// [`ProcessExecutionProfile::LinuxConfinedV1`] confines two named resource
/// classes — the filesystem, with Landlock, and the network, with a network
/// namespace — and its [`Confinement::unconfined`] inventory states what it
/// leaves reachable. The other two profiles confine neither, and no profile
/// installs a seccomp filter or a cgroup.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessExecutionProfile {
    /// The historical path. The host applies a wall-clock deadline, bounded
    /// stdout/stderr reads and a process-group kill on every exit; it installs
    /// **no** `setrlimit`. `limits.memory_bytes` and every entry in
    /// `limits.additional` are therefore declarations the OS does not apply.
    /// [`EnforcedLimits::unenforced_kinds`] names them explicitly.
    #[default]
    UnenforcedLegacy,
    /// Linux only. Every limit kind the manifest declares is installed with
    /// `setrlimit` in the child before `exec`, on top of the wall-clock
    /// deadline, output caps and process-group kill. A declared kind this
    /// profile cannot enforce is refused before spawning, never downgraded.
    LinuxRlimitV1,
    /// Linux only. Everything [`Self::LinuxRlimitV1`] installs, *plus*
    /// kernel-enforced filesystem and network confinement: a Landlock ruleset
    /// admitting only the paths an explicit [`ConfinementPolicy`] names, and a
    /// fresh network namespace. It composes with the limit profile rather than
    /// replacing it, and requires a policy — a caller that supplies none is
    /// refused before spawning, because this profile admits nothing by default.
    LinuxConfinedV1,
}

impl ProcessExecutionProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::UnenforcedLegacy => "unenforced-legacy",
            Self::LinuxRlimitV1 => "linux-rlimit-v1",
            Self::LinuxConfinedV1 => "linux-confined-v1",
        }
    }

    /// Whether this profile installs `setrlimit` bounds. Both Linux profiles
    /// do; the confined profile composes with the limit profile instead of
    /// replacing it.
    pub const fn installs_rlimits(self) -> bool {
        matches!(self, Self::LinuxRlimitV1 | Self::LinuxConfinedV1)
    }

    /// Whether this profile installs filesystem and network confinement.
    pub const fn confines(self) -> bool {
        matches!(self, Self::LinuxConfinedV1)
    }
}

impl fmt::Display for ProcessExecutionProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Where an enforced value came from. A derived or defaulted value is not a
/// declaration, so reports must not present it as one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LimitSource {
    /// The manifest declares this kind and this value.
    Declared,
    /// The profile derived the value from the manifest's wall-clock timeout
    /// because the manifest declares no value for this kind.
    DerivedFromWallClock,
    /// A fixed value this profile always installs.
    ProfileDefault,
}

/// One `setrlimit` call the profile installs in the child before `exec`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcedRlimit {
    /// The manifest limit-kind name.
    pub kind: String,
    /// The POSIX/Linux resource the kind maps to.
    pub resource: String,
    pub soft: u64,
    pub hard: u64,
    pub source: LimitSource,
}

/// What a profile actually enforces for one execution, with values.
///
/// This is the execution evidence a report quotes: it separates limits the OS
/// applied (`rlimits`) from limits the host applies in its own read loop
/// (`wall_clock_ms`, `stdout_bytes`, `stderr_bytes`, `process_group_kill`) from
/// declarations nothing applied (`unenforced_kinds`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcedLimits {
    pub profile: ProcessExecutionProfile,
    /// Host-side deadline; expiry kills the child's process group.
    pub wall_clock_ms: u64,
    /// Maximum response payload bytes read from stdout.
    pub stdout_bytes: usize,
    /// Maximum diagnostic bytes read from stderr.
    pub stderr_bytes: usize,
    /// Whether the host kills the child's process group on every exit path.
    pub process_group_kill: bool,
    /// `setrlimit` calls installed before `exec`, sorted by kind.
    pub rlimits: Vec<EnforcedRlimit>,
    /// Limit kinds the manifest declares that this execution did **not**
    /// enforce. Non-empty only under [`ProcessExecutionProfile::UnenforcedLegacy`];
    /// [`ProcessExecutionProfile::LinuxRlimitV1`] refuses instead of running.
    pub unenforced_kinds: Vec<String>,
    /// The filesystem and network confinement this execution achieved, and the
    /// kernel facts it was derived from. `None` for every profile that confines
    /// neither, which is what those profiles' records must say rather than
    /// leaving the question open. Absent from the serialized form when `None`,
    /// so records written before this field existed are byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confinement: Option<Confinement>,
}

impl EnforcedLimits {
    /// A single-line summary suitable for a diagnostic or a report row.
    pub fn summary(&self) -> String {
        let applied = self
            .rlimits
            .iter()
            .map(|limit| format!("{}={}({})", limit.kind, limit.soft, limit.resource))
            .collect::<Vec<_>>()
            .join(", ");
        let confinement = self.confinement.as_ref().map_or_else(
            || "; confinement=none".to_owned(),
            |confinement| format!("; confinement={}", confinement.summary()),
        );
        format!(
            "profile={}; wall_clock_ms={}; stdout_bytes={}; stderr_bytes={}; \
             process_group_kill={}; rlimits=[{applied}]; unenforced=[{}]{confinement}",
            self.profile,
            self.wall_clock_ms,
            self.stdout_bytes,
            self.stderr_bytes,
            self.process_group_kill,
            self.unenforced_kinds.join(", "),
        )
    }
}

/// How one process execution ended. Every non-`Completed` outcome is an
/// environment fact about the provider run; none of them is a statement about
/// the program under analysis, so the registry reports them as `Unknown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessOutcome {
    /// The child exited successfully and its response frame decoded.
    Completed,
    /// The profile refused the package before spawning anything.
    RefusedByProfile,
    /// The child could not be spawned.
    SpawnFailed,
    /// The wall-clock deadline expired; the process group was killed.
    WallClockTimeout,
    /// The child wrote past an output cap; the process group was killed.
    OutputLimit,
    /// The child exited unsuccessfully or died from a signal. A `setrlimit`
    /// kill (`RLIMIT_CPU`, or an allocation failure under `RLIMIT_AS`) lands
    /// here, with `signal` set when the kernel terminated it.
    Terminated,
    /// A pipe or wait error on the host side.
    Io,
    /// The response was not a well-formed protocol frame.
    Protocol,
}

/// The evidence record of one process execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessExecutionRecord {
    pub profile: ProcessExecutionProfile,
    /// `None` exactly when the profile refused before spawning.
    pub enforced: Option<EnforcedLimits>,
    pub outcome: ProcessOutcome,
    pub wall_time_ms: u64,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    /// The host-side failure text, empty for [`ProcessOutcome::Completed`].
    pub detail: String,
}

impl ProcessExecutionRecord {
    /// A single-line summary suitable for a report row.
    pub fn summary(&self) -> String {
        let enforced = self.enforced.as_ref().map_or_else(
            || format!("profile={}; refused", self.profile),
            |e| e.summary(),
        );
        format!(
            "{enforced}; outcome={}; wall_time_ms={}; stdout_bytes={}; stderr_bytes={}; \
             exit_code={:?}; signal={:?}",
            serde_json::to_value(self.outcome)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_default(),
            self.wall_time_ms,
            self.stdout_bytes,
            self.stderr_bytes,
            self.exit_code,
            self.signal,
        )
    }
}

/// The declarable limit kinds [`ProcessExecutionProfile::LinuxRlimitV1`]
/// enforces, each with the resource it installs. `memory_bytes` is the
/// mandatory manifest field; the rest are `limits.additional` keys.
const LINUX_RLIMIT_V1_KINDS: [(&str, &str); 6] = [
    ("memory_bytes", "RLIMIT_AS"),
    ("cpu_seconds", "RLIMIT_CPU"),
    ("core_bytes", "RLIMIT_CORE"),
    ("max_file_bytes", "RLIMIT_FSIZE"),
    ("max_open_files", "RLIMIT_NOFILE"),
    ("max_processes", "RLIMIT_NPROC"),
];

/// The limit kinds a profile can enforce, sorted. Empty for the legacy
/// profile, which enforces no `setrlimit` kind at all.
pub fn enforceable_limit_kinds(profile: ProcessExecutionProfile) -> Vec<&'static str> {
    if !profile.installs_rlimits() || !cfg!(target_os = "linux") {
        return Vec::new();
    }
    let mut kinds = LINUX_RLIMIT_V1_KINDS
        .iter()
        .map(|(kind, _)| *kind)
        .collect::<Vec<_>>();
    kinds.sort_unstable();
    kinds
}

pub struct ProcessProvider {
    manifest: PluginManifest,
    // Process execution currently requires Unix process-group containment.
    #[cfg_attr(not(unix), expect(dead_code))]
    package_root: String,
    profile: ProcessExecutionProfile,
    /// The explicit filesystem reach a confining profile grants. `None` is a
    /// refusal under [`ProcessExecutionProfile::LinuxConfinedV1`], not an
    /// unconfined execution.
    confinement_policy: Option<ConfinementPolicy>,
    /// The most recent execution's evidence record. Interior mutability keeps
    /// [`VerifierProvider`]'s `&self` dispatch, which providers share behind an
    /// `Arc`, unchanged.
    last_execution: std::sync::Mutex<Option<ProcessExecutionRecord>>,
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
            profile: ProcessExecutionProfile::default(),
            confinement_policy: None,
            last_execution: std::sync::Mutex::new(None),
        })
    }

    /// Selects the execution profile. Additive: the default is unchanged, and
    /// no existing caller's behavior moves without this call.
    #[must_use]
    pub fn with_execution_profile(mut self, profile: ProcessExecutionProfile) -> Self {
        self.profile = profile;
        self
    }

    pub const fn execution_profile(&self) -> ProcessExecutionProfile {
        self.profile
    }

    /// Supplies the explicit filesystem reach a confining profile grants.
    ///
    /// Additive and fail-closed in both directions: a confining profile with no
    /// policy is refused, and a policy handed to a profile that enforces none is
    /// also refused, so a caller can never believe a package was confined when
    /// it was not.
    #[must_use]
    pub fn with_confinement_policy(mut self, policy: ConfinementPolicy) -> Self {
        self.confinement_policy = Some(policy);
        self
    }

    /// The policy this provider would enforce, if any.
    pub const fn confinement_policy(&self) -> Option<&ConfinementPolicy> {
        self.confinement_policy.as_ref()
    }

    /// The evidence record of the most recent execution, if any.
    pub fn last_execution(&self) -> Option<ProcessExecutionRecord> {
        self.last_execution
            .lock()
            .map_or(None, |record| record.clone())
    }

    /// What this provider's profile would enforce for this manifest, or the
    /// refusal that keeps it from running.
    ///
    /// Fails closed: an unavailable target profile or a declared limit kind the
    /// profile cannot enforce is an error here and a pre-spawn refusal in
    /// [`VerifierProvider::verify`], never a silent downgrade to a weaker
    /// profile.
    pub fn enforced_limits(&self) -> Result<EnforcedLimits, ProviderFailure> {
        let limits = &self.manifest.limits;
        if self.confinement_policy.is_some() && !self.profile.confines() {
            return Err(ProviderFailure::Protocol(format!(
                "execution profile `{}` enforces no confinement policy, so the supplied one \
                 would be a declaration nothing installs; select `{}` instead",
                self.profile,
                ProcessExecutionProfile::LinuxConfinedV1
            )));
        }
        match self.profile {
            ProcessExecutionProfile::UnenforcedLegacy => {
                let mut unenforced_kinds = vec!["memory_bytes".to_owned()];
                unenforced_kinds.extend(limits.additional.keys().cloned());
                unenforced_kinds.sort();
                Ok(EnforcedLimits {
                    profile: self.profile,
                    wall_clock_ms: limits.timeout_ms,
                    stdout_bytes: limits.output_bytes,
                    stderr_bytes: PROVIDER_STDERR_BYTES,
                    process_group_kill: cfg!(unix),
                    rlimits: Vec::new(),
                    unenforced_kinds,
                    confinement: None,
                })
            }
            ProcessExecutionProfile::LinuxRlimitV1 => self.linux_rlimit_v1_limits(self.profile),
            ProcessExecutionProfile::LinuxConfinedV1 => {
                // The confinement is checked first so an unavailable mechanism
                // names the mechanism, not the composed limit profile.
                let confinement = self.linux_confinement()?;
                let mut enforced = self.linux_rlimit_v1_limits(self.profile)?;
                enforced.confinement = Some(confinement);
                Ok(enforced)
            }
        }
    }

    /// The confinement the selected policy achieves on this kernel, or the
    /// pre-spawn refusal that keeps the package from running.
    ///
    /// Fail-closed on three separate grounds — a non-Linux target, a missing
    /// policy and a kernel that cannot install one of the two mechanisms — and
    /// none of them has a weaker fallback.
    fn linux_confinement(&self) -> Result<Confinement, ProviderFailure> {
        let profile = ProcessExecutionProfile::LinuxConfinedV1;
        let Some(policy) = self.confinement_policy.as_ref() else {
            return Err(ProviderFailure::Protocol(format!(
                "execution profile `{profile}` requires an explicit confinement policy; it \
                 admits no filesystem path beyond the package root and has no default"
            )));
        };
        #[cfg(target_os = "linux")]
        {
            confinement::linux::describe(policy, &self.package_root, self.manifest.network_access)
                .map_err(|error| {
                    ProviderFailure::Protocol(format!(
                        "execution profile `{profile}` cannot confine this execution: {error}"
                    ))
                })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = policy;
            Err(ProviderFailure::Protocol(format!(
                "execution profile `{profile}` is unavailable on this target; its Landlock and \
                 network-namespace mechanisms are Linux-only and it makes no isolation claim \
                 elsewhere"
            )))
        }
    }

    fn linux_rlimit_v1_limits(
        &self,
        profile: ProcessExecutionProfile,
    ) -> Result<EnforcedLimits, ProviderFailure> {
        if !cfg!(target_os = "linux") {
            return Err(ProviderFailure::Protocol(format!(
                "execution profile `{profile}` is unavailable on this target; it installs Linux \
                 `setrlimit` resources and makes no isolation claim elsewhere"
            )));
        }
        let limits = &self.manifest.limits;
        let resource_of = |kind: &str| {
            LINUX_RLIMIT_V1_KINDS
                .iter()
                .find(|(name, _)| *name == kind)
                .map(|(_, resource)| (*resource).to_owned())
        };
        for kind in limits.additional.keys() {
            if resource_of(kind).is_none() {
                return Err(ProviderFailure::Protocol(format!(
                    "execution profile `{profile}` cannot enforce declared limit kind `{kind}`; \
                     it enforces [{}]",
                    enforceable_limit_kinds(profile).join(", ")
                )));
            }
        }
        let mut rlimits = vec![EnforcedRlimit {
            kind: "memory_bytes".to_owned(),
            resource: "RLIMIT_AS".to_owned(),
            soft: limits.memory_bytes,
            hard: limits.memory_bytes,
            source: LimitSource::Declared,
        }];
        // A wall-clock deadline alone cannot bound CPU consumed by a child that
        // outlives one scheduling quantum of the reader loop, so an undeclared
        // CPU limit is derived rather than omitted. The derivation is recorded
        // as derived, not as a declaration.
        let (cpu_seconds, cpu_source) = match limits.additional.get("cpu_seconds") {
            Some(declared) => (*declared, LimitSource::Declared),
            None => (
                limits.timeout_ms.div_ceil(1_000).saturating_add(1),
                LimitSource::DerivedFromWallClock,
            ),
        };
        rlimits.push(EnforcedRlimit {
            kind: "cpu_seconds".to_owned(),
            resource: "RLIMIT_CPU".to_owned(),
            soft: cpu_seconds,
            hard: cpu_seconds,
            source: cpu_source,
        });
        let (core_bytes, core_source) = match limits.additional.get("core_bytes") {
            Some(declared) => (*declared, LimitSource::Declared),
            None => (0, LimitSource::ProfileDefault),
        };
        rlimits.push(EnforcedRlimit {
            kind: "core_bytes".to_owned(),
            resource: "RLIMIT_CORE".to_owned(),
            soft: core_bytes,
            hard: core_bytes,
            source: core_source,
        });
        for kind in ["max_file_bytes", "max_open_files", "max_processes"] {
            if let Some(value) = limits.additional.get(kind) {
                rlimits.push(EnforcedRlimit {
                    kind: kind.to_owned(),
                    resource: resource_of(kind).expect("kind is in the enforced table"),
                    soft: *value,
                    hard: *value,
                    source: LimitSource::Declared,
                });
            }
        }
        rlimits.sort_by(|left, right| left.kind.cmp(&right.kind));
        Ok(EnforcedLimits {
            profile,
            wall_clock_ms: limits.timeout_ms,
            stdout_bytes: limits.output_bytes,
            stderr_bytes: PROVIDER_STDERR_BYTES,
            process_group_kill: true,
            rlimits,
            unenforced_kinds: Vec::new(),
            confinement: None,
        })
    }

    #[cfg(unix)]
    fn record_execution(&self, record: ProcessExecutionRecord) {
        if let Ok(mut slot) = self.last_execution.lock() {
            *slot = Some(record);
        }
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
        // Invalid requests must never launch a process.
        let frame = encode_protocol_frame(request, self.manifest.limits.output_bytes)?;
        #[cfg(unix)]
        {
            self.verify_process(frame)
        }
        #[cfg(not(unix))]
        {
            let _ = frame;
            Err(ProviderFailure::Protocol(
                "bounded process providers require Unix process groups".into(),
            ))
        }
    }

    fn verify_v2(
        &self,
        request: &VerificationRequestV2,
    ) -> Result<VerificationResponseV2, ProviderFailure> {
        // Identical framing, limits, deadline, and process-group behavior;
        // only the payload schema differs (design §2.1: protocol v1 framing
        // stays).
        let frame = encode_protocol_frame(request, self.manifest.limits.output_bytes)?;
        #[cfg(unix)]
        {
            self.verify_process(frame)
        }
        #[cfg(not(unix))]
        {
            let _ = frame;
            Err(ProviderFailure::Protocol(
                "bounded process providers require Unix process groups".into(),
            ))
        }
    }
}

#[cfg(unix)]
struct ProviderChild(std::process::Child);

#[cfg(unix)]
impl Drop for ProviderChild {
    fn drop(&mut self) {
        // The child starts a fresh process group. Kill descendants on every
        // exit path, including successful direct-child exits with open pipes.
        // This contains ordinary inherited descendants, not processes that
        // deliberately escape the group; providers are not an OS sandbox.
        unsafe {
            libc::kill(-(self.0.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = self.0.kill();
        // Reap without blocking the caller past its deadline. Usually the
        // killed child is ready immediately; otherwise a reaper owns it.
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let child = &mut self.0;
            // waitpid owns no Rust pipe handles and only waits for this PID.
            let pid = child.id() as libc::pid_t;
            thread::spawn(move || {
                loop {
                    let result = unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
                    if result >= 0
                        || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
                    {
                        break;
                    }
                }
            });
        }
    }
}

#[cfg(unix)]
fn nonblocking(pipe: &impl std::os::fd::AsRawFd) -> Result<(), ProviderFailure> {
    let fd = pipe.as_raw_fd();
    // The descriptor remains owned by the live pipe; only its status flags change.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(ProviderFailure::Io(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn read_available(
    reader: &mut impl Read,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<bool, ProviderFailure> {
    // One bounded chunk per iteration keeps a continuously writing provider
    // from starving deadline checks or the other pipe.
    let mut chunk = [0; 8192];
    match reader.read(&mut chunk) {
        Ok(0) => Ok(true),
        Ok(count) => {
            if count > limit.saturating_sub(output.len()) {
                return Err(ProviderFailure::OutputLimit);
            }
            output.extend_from_slice(&chunk[..count]);
            Ok(false)
        }
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(ProviderFailure::Io(error.to_string())),
    }
}

/// The Linux resource a limit kind installs. `None` is a refusal, never a
/// skipped limit.
///
/// The `RLIMIT_*` constants are `c_uint` on glibc and `c_int` on musl, so the
/// casts are redundant on exactly one of the two C libraries.
#[cfg(target_os = "linux")]
#[allow(clippy::unnecessary_cast)]
fn linux_rlimit_resource(kind: &str) -> Option<u32> {
    Some(match kind {
        "memory_bytes" => libc::RLIMIT_AS as u32,
        "cpu_seconds" => libc::RLIMIT_CPU as u32,
        "core_bytes" => libc::RLIMIT_CORE as u32,
        "max_file_bytes" => libc::RLIMIT_FSIZE as u32,
        "max_open_files" => libc::RLIMIT_NOFILE as u32,
        "max_processes" => libc::RLIMIT_NPROC as u32,
        _ => return None,
    })
}

/// The kernel object a confining profile installs in the child, carried from
/// the pre-spawn build to the `pre_exec` closure. `None` is a profile that
/// confines nothing.
#[cfg(target_os = "linux")]
type ConfinementHandle = Option<std::os::fd::OwnedFd>;
#[cfg(all(unix, not(target_os = "linux")))]
type ConfinementHandle = Option<()>;

#[cfg(unix)]
impl ProcessProvider {
    /// Runs the package under the selected execution profile and records what
    /// that execution actually enforced.
    fn verify_process<T: for<'de> Deserialize<'de>>(
        &self,
        frame: Vec<u8>,
    ) -> Result<T, ProviderFailure> {
        let started = Instant::now();
        let enforced = match self.enforced_limits() {
            Ok(enforced) => enforced,
            Err(error) => {
                // Refused before anything was spawned: no process ran, so the
                // record carries no enforced limits at all.
                self.record_execution(ProcessExecutionRecord {
                    profile: self.profile,
                    enforced: None,
                    outcome: ProcessOutcome::RefusedByProfile,
                    wall_time_ms: 0,
                    stdout_bytes: 0,
                    stderr_bytes: 0,
                    exit_code: None,
                    signal: None,
                    detail: error.to_string(),
                });
                return Err(error);
            }
        };
        // Building the Landlock ruleset opens every admitted path, so it can
        // fail; it happens here, before anything is spawned, so a failure is a
        // refusal rather than an unconfined execution.
        let confinement = match self.prepare_confinement(&enforced) {
            Ok(confinement) => confinement,
            Err(error) => {
                self.record_execution(ProcessExecutionRecord {
                    profile: self.profile,
                    enforced: None,
                    outcome: ProcessOutcome::RefusedByProfile,
                    wall_time_ms: 0,
                    stdout_bytes: 0,
                    stderr_bytes: 0,
                    exit_code: None,
                    signal: None,
                    detail: error.to_string(),
                });
                return Err(error);
            }
        };
        let mut output = Vec::new();
        let mut error_output = Vec::new();
        let mut status = None;
        let result = self.run_process(
            frame,
            &enforced,
            confinement,
            &mut output,
            &mut error_output,
            &mut status,
        );
        let outcome = match &result {
            Ok(_) => ProcessOutcome::Completed,
            Err(ProviderFailure::Spawn(_)) => ProcessOutcome::SpawnFailed,
            Err(ProviderFailure::Timeout) => ProcessOutcome::WallClockTimeout,
            Err(ProviderFailure::OutputLimit) => ProcessOutcome::OutputLimit,
            Err(ProviderFailure::Crashed(_)) => ProcessOutcome::Terminated,
            Err(ProviderFailure::Io(_)) => ProcessOutcome::Io,
            // `Crashed` gains stderr detail through `with_detail`, which
            // rewrites it as `Protocol`; a terminating status still classifies
            // the execution as terminated.
            Err(ProviderFailure::Protocol(_)) => status
                .filter(|status| !status.success())
                .map_or(ProcessOutcome::Protocol, |_| ProcessOutcome::Terminated),
        };
        self.record_execution(ProcessExecutionRecord {
            profile: self.profile,
            enforced: Some(enforced),
            outcome,
            wall_time_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            stdout_bytes: output.len(),
            stderr_bytes: error_output.len(),
            exit_code: status.and_then(|status| status.code()),
            signal: status.and_then(|status| {
                use std::os::unix::process::ExitStatusExt;
                status.signal()
            }),
            detail: result
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default(),
        });
        result
    }

    /// Creates the kernel objects a confining profile installs in the child.
    ///
    /// `None` means this profile confines nothing, which its record states
    /// explicitly through `EnforcedLimits::confinement`.
    #[cfg(target_os = "linux")]
    fn prepare_confinement(
        &self,
        enforced: &EnforcedLimits,
    ) -> Result<ConfinementHandle, ProviderFailure> {
        let Some(confinement) = enforced.confinement.as_ref() else {
            return Ok(None);
        };
        confinement::linux::build_ruleset(confinement)
            .map(Some)
            .map_err(|error| {
                ProviderFailure::Protocol(format!(
                    "execution profile `{}` cannot confine this execution: {error}",
                    enforced.profile
                ))
            })
    }

    #[cfg(not(target_os = "linux"))]
    fn prepare_confinement(
        &self,
        enforced: &EnforcedLimits,
    ) -> Result<ConfinementHandle, ProviderFailure> {
        // Off Linux no profile reaches here with a confinement: `enforced_limits`
        // refuses the confining profile before this point.
        debug_assert!(enforced.confinement.is_none());
        let _ = enforced;
        Ok(None)
    }

    fn run_process<T: for<'de> Deserialize<'de>>(
        &self,
        frame: Vec<u8>,
        enforced: &EnforcedLimits,
        confinement: ConfinementHandle,
        output: &mut Vec<u8>,
        error_output: &mut Vec<u8>,
        status: &mut Option<std::process::ExitStatus>,
    ) -> Result<T, ProviderFailure> {
        use std::os::unix::process::CommandExt;
        let PluginEntrypoint::Executable { path } = &self.manifest.entrypoint else {
            return Err(ProviderFailure::Protocol("missing executable".into()));
        };
        #[cfg(not(target_os = "linux"))]
        let _ = confinement;
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(enforced.wall_clock_ms))
            .ok_or_else(|| {
                ProviderFailure::Protocol("provider timeout exceeds clock range".into())
            })?;
        let limit = enforced.stdout_bytes;
        let wire_limit = limit.checked_add(4).ok_or(ProviderFailure::OutputLimit)?;
        let executable = Path::new(&self.package_root).join(path);
        let mut command = Command::new(&executable);
        command
            .current_dir(&self.package_root)
            .env_clear()
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // The ruleset descriptor is close-on-exec, so the child restricts itself
        // with it and the `exec`ed package never sees it. The closure keeps the
        // `OwnedFd` alive until the spawn returns.
        #[cfg(target_os = "linux")]
        if let Some(ruleset) = confinement {
            use std::os::fd::AsRawFd;
            // SAFETY: this runs in the forked child before `exec`. It allocates
            // nothing, acquires no lock and makes only syscalls; `ruleset` is
            // owned by the closure and stays open until the spawn completes.
            unsafe {
                command.pre_exec(move || confinement::linux::apply(ruleset.as_raw_fd()));
            }
        }
        #[cfg(target_os = "linux")]
        if !enforced.rlimits.is_empty() {
            let mut plan = Vec::with_capacity(enforced.rlimits.len());
            for enforced_limit in &enforced.rlimits {
                let resource = linux_rlimit_resource(&enforced_limit.kind).ok_or_else(|| {
                    ProviderFailure::Protocol(format!(
                        "execution profile `{}` cannot enforce declared limit kind `{}`",
                        enforced.profile, enforced_limit.kind
                    ))
                })?;
                plan.push((resource, enforced_limit.soft, enforced_limit.hard));
            }
            // SAFETY: this runs in the forked child before `exec`. It allocates
            // nothing, acquires no lock, and calls only `setrlimit`, a plain
            // syscall. A failed call fails the spawn rather than running the
            // package with a limit missing.
            unsafe {
                command.pre_exec(move || {
                    for (resource, soft, hard) in &plan {
                        let value = libc::rlimit {
                            rlim_cur: *soft as libc::rlim_t,
                            rlim_max: *hard as libc::rlim_t,
                        };
                        if libc::setrlimit(*resource as _, &value) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                    }
                    Ok(())
                });
            }
        }
        let mut child = ProviderChild(
            command
                .spawn()
                .map_err(|error| ProviderFailure::Spawn(error.to_string()))?,
        );
        let mut stdin = child.0.stdin.take();
        let mut stdout = child
            .0
            .stdout
            .take()
            .ok_or_else(|| ProviderFailure::Io("missing provider stdout".into()))?;
        let mut stderr = child
            .0
            .stderr
            .take()
            .ok_or_else(|| ProviderFailure::Io("missing provider stderr".into()))?;
        nonblocking(
            stdin
                .as_ref()
                .ok_or_else(|| ProviderFailure::Io("missing provider stdin".into()))?,
        )?;
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        let mut sent = 0;
        let (mut output_done, mut error_done) = (false, false);
        let mut input_failure = None;
        loop {
            // Observe direct-child exit before deciding which bound failed.
            // An exited provider can leave descendant-held output pipes even
            // when its input reader vanished before our first write.
            if status.is_none() {
                *status = child
                    .0
                    .try_wait()
                    .map_err(|error| ProviderFailure::Io(error.to_string()))?;
            }
            if Instant::now() >= deadline {
                if status.is_none()
                    && let Some(error) = input_failure
                {
                    return Err(error);
                }
                return Err(ProviderFailure::Timeout);
            }
            if let Some(input) = stdin.as_mut() {
                match input.write(&frame[sent..]) {
                    Ok(0) => {
                        input_failure = Some(ProviderFailure::Io("provider stdin closed".into()));
                        stdin = None;
                    }
                    Ok(count) => sent += count,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                        // Do not let write-vs-exit scheduling decide whether
                        // inherited output pipes count as a timeout. Remember
                        // incomplete input while draining under the same bound;
                        // it must still prevent successful response decoding.
                        input_failure = Some(ProviderFailure::Io(error.to_string()));
                        stdin = None;
                    }
                    Err(error) => return Err(ProviderFailure::Io(error.to_string())),
                }
                if sent == frame.len() {
                    stdin = None;
                }
            }
            if !output_done {
                output_done = read_available(&mut stdout, output, wire_limit)?;
            }
            if !error_done {
                error_done = read_available(&mut stderr, error_output, enforced.stderr_bytes)?;
            }
            if let Some(exit) = *status
                && output_done
                && error_done
                && stdin.is_none()
            {
                if let Some(error) = input_failure {
                    return Err(error);
                }
                if !exit.success() {
                    return Err(ProviderFailure::Crashed(exit.code())
                        .with_detail(&String::from_utf8_lossy(error_output)));
                }
                return decode_protocol_frame(output, limit);
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}

#[cfg(unix)]
trait FailureDetail {
    fn with_detail(self, detail: &str) -> Self;
}

#[cfg(unix)]
impl FailureDetail for ProviderFailure {
    fn with_detail(self, detail: &str) -> Self {
        if detail.trim().is_empty() {
            self
        } else {
            ProviderFailure::Protocol(format!("{self}: {}", detail.trim()))
        }
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
    let capacity = payload
        .len()
        .checked_add(4)
        .ok_or(ProviderFailure::OutputLimit)?;
    let mut frame = Vec::with_capacity(capacity);
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
    let wire_length = length.checked_add(4).ok_or(ProviderFailure::OutputLimit)?;
    if frame.len() != wire_length {
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

    /// Evidence-v2 dispatch (design §3.1). The manifest gates QP001–QP005 are
    /// the v1 gates verbatim; the provider then runs through the unchanged
    /// spawn/deadline/output machinery; only the firewall differs, and only a
    /// core witness replay can produce a certificate.
    ///
    /// `checker` is the trusted-core replay hook for `request.capability`.
    /// `None` — a capability with no core checker — is `Unknown` (QP016).
    pub fn verify_v2(
        &self,
        request: &VerificationRequestV2,
        task: &VerificationTask,
        checker: Option<&dyn WitnessChecker>,
    ) -> VerificationOutcome {
        if let Err(error) = request.validate() {
            return VerificationOutcome::unknown("QP001", error.to_string());
        }
        if let Err(error) = task.validate() {
            return VerificationOutcome::unknown("QP001", error.to_string());
        }
        if task.task_digest() != request.task_digest
            || request.unit != task.unit
            || request.target != task.target
            || request.assumptions != task.assumptions
            || request.evidence_version != task.evidence_version
            || request.obligations
                != task
                    .obligations_for(request.capability)
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>()
        {
            return VerificationOutcome::unknown(
                "QP001",
                "the request's task digest is not the digest of the core-derived task",
            );
        }
        if let Some(input) = &request.source_effect
            && task.contract_sources.get(input.digest_key()) != Some(&input.digest())
        {
            return VerificationOutcome::unknown(
                "QP001",
                "source projection is not bound to the core task",
            );
        }
        if let Some(input) = &request.reactor_program
            && task.contract_sources.get(input.digest_key()) != Some(&input.digest())
        {
            return VerificationOutcome::unknown(
                "QP001",
                "reactor program projection is not bound to the core task",
            );
        }
        // A capability whose derivation could not classify every construct can
        // never reach `Safe`, whatever a provider returns (design §1.4).
        if task.is_incomplete_for(request.capability) {
            return VerificationOutcome::unknown(
                "QP016",
                format!(
                    "the core derivation is incomplete for `{}`; no witness can discharge \
                     an unknown obligation inventory",
                    request.capability
                ),
            );
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
        let response = match entry.provider.verify_v2(request) {
            Ok(response) => response,
            Err(error) => return VerificationOutcome::unknown("QP006", error.to_string()),
        };
        self.check_response_v2(manifest, request, task, checker, response)
    }

    fn check_response_v2(
        &self,
        manifest: &PluginManifest,
        request: &VerificationRequestV2,
        task: &VerificationTask,
        checker: Option<&dyn WitnessChecker>,
        response: VerificationResponseV2,
    ) -> VerificationOutcome {
        if response.protocol_version != request.protocol_version
            || response.request_id != request.request_id
            || response.plugin_id != request.plugin_id
            || response.diagnostics.iter().any(|diagnostic| {
                !valid_provider_diagnostic_code(&manifest.plugin_id, &diagnostic.code)
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
            ProposedVerdict::Unsafe => {
                // Evidence v2 ships no counterexample checker, so an `Unsafe`
                // proposal stays an allegation (design open question 4).
                let mut outcome = VerificationOutcome::unknown(
                    "QP010",
                    "unverified provider allegation: evidence v2 has no counterexample checker",
                );
                outcome.diagnostics.extend(response.diagnostics);
                outcome
            }
            ProposedVerdict::Safe => {
                let Some(evidence) = response.evidence else {
                    return VerificationOutcome::unknown("QP008", "safe result has no evidence");
                };
                let discharged =
                    match self.check_evidence_v2(manifest, request, task, checker, &evidence) {
                        Ok(discharged) => discharged,
                        Err(EvidenceRejection { code, reason }) => {
                            return VerificationOutcome::unknown(code, reason);
                        }
                    };
                let obligation_bounds = task
                    .obligations_for(request.capability)
                    .into_iter()
                    .map(|obligation| (obligation.obligation_id.clone(), obligation.bounds.clone()))
                    .collect();
                VerificationOutcome {
                    verdict: ProposedVerdict::Safe,
                    diagnostics: response.diagnostics,
                    certificate: Some(CoreCertificate {
                        plugin_id: request.plugin_id.clone(),
                        capability: request.capability,
                        unit: request.unit.clone(),
                        evidence: ProposedEvidence {
                            evidence_version: evidence.evidence_version,
                            unit: evidence.unit.clone(),
                            capability: evidence.capability,
                            // v2 carries no Boolean claims; the checked
                            // relation is the witness replay, and coverage
                            // refers to core obligation identifiers.
                            claims: Vec::new(),
                            coverage: discharged,
                            assumptions_used: evidence.assumptions_used.clone(),
                            reproducibility: evidence.reproducibility.clone(),
                        },
                        witness: Some(evidence.witness),
                        obligation_bounds,
                        task_digest: Some(request.task_digest.clone()),
                        task: Some(task.clone()),
                    }),
                }
            }
        }
    }

    /// The v2 firewall. The v1 envelope checks run first and remain necessary
    /// but never sufficient; only the trusted-core witness replay can accept.
    fn check_evidence_v2(
        &self,
        manifest: &PluginManifest,
        request: &VerificationRequestV2,
        task: &VerificationTask,
        checker: Option<&dyn WitnessChecker>,
        evidence: &ProposedEvidenceV2,
    ) -> Result<BTreeSet<String>, EvidenceRejection> {
        let reject = |code: &'static str, reason: String| EvidenceRejection { code, reason };
        if evidence.evidence_version != request.evidence_version
            || evidence.evidence_version < PLUGIN_EVIDENCE_VERSION_V2
        {
            return Err(reject(
                "QP009",
                "the evidence version does not match the negotiated request".into(),
            ));
        }
        if evidence.unit != request.unit
            || evidence.capability != request.capability
            || evidence.assumptions_used != request.assumptions
            || evidence.reproducibility.provider_version != manifest.package_version
            || evidence.reproducibility.package_digest != manifest.package_digest
            || !evidence.reproducibility.deterministic
        {
            return Err(reject(
                "QP009",
                "the core certificate checker rejected the evidence envelope".into(),
            ));
        }
        if evidence.witness.task_digest() != request.task_digest {
            return Err(reject(
                "QP009",
                "the witness is bound to a different verification task".into(),
            ));
        }
        let Some(checker) = checker.filter(|checker| checker.capability() == request.capability)
        else {
            return Err(reject(
                "QP016",
                format!(
                    "no core witness checker for capability `{}` at evidence version {}",
                    request.capability, evidence.evidence_version
                ),
            ));
        };
        // The denominator is the core's inventory, never the witness's own
        // account of what it covered (design §2.2).
        let required = task.coverage_for(request.capability);
        if required.is_empty() {
            return Err(reject(
                "QP016",
                "the core derived no obligations; an empty inventory has no independently \
                 checked no-obligations argument"
                    .into(),
            ));
        }
        let discharged = checker
            .check(task, &evidence.witness)
            .map_err(|reason| reject("QP009", reason))?;
        if discharged != required {
            return Err(reject(
                "QP009",
                "the witness replay did not discharge every core-derived obligation".into(),
            ));
        }
        if evidence.coverage != required {
            return Err(reject(
                "QP009",
                "the proposal's coverage is not the core's obligation inventory".into(),
            ));
        }
        Ok(discharged)
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
                !valid_provider_diagnostic_code(&manifest.plugin_id, &diagnostic.code)
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
            ProposedVerdict::Unsafe => {
                let mut outcome = VerificationOutcome::unknown(
                    "QP010",
                    "unverified provider allegation: protocol v1 has no counterexample checker",
                );
                outcome.diagnostics.extend(response.diagnostics);
                outcome
            }
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
                        witness: None,
                        obligation_bounds: BTreeMap::new(),
                        task_digest: None,
                        task: None,
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
    if let Some(alleged) = outcomes
        .iter()
        .find(|outcome| outcome.verdict == ProposedVerdict::Unsafe)
    {
        // E03 acceptance 3. No evidence version ships a counterexample checker,
        // so an `Unsafe` reaching composition is an unchecked allegation about
        // one supported property. `check_response`/`check_response_v2` already
        // downgrade it (QP010); composition is the last place a caller-supplied
        // allegation could become the published composite verdict, and a
        // provisional source-unit diagnostic must never be republished here as
        // a verified counterexample. Its diagnostics are retained verbatim.
        let mut outcome = VerificationOutcome::unknown(
            "QP010",
            "unverified provider allegation: composition has no counterexample checker",
        );
        outcome
            .diagnostics
            .extend(alleged.diagnostics.iter().cloned());
        return outcome;
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
    // E03 acceptance 2. Composition itself is represented as a core certificate
    // collection; it cannot fabricate a new provider certificate, so the
    // leading certificate is returned for API compatibility and all component
    // certificates are available through `compose_certificates`. That
    // representative is only honest about the composite's premises when every
    // component declares the same premise set: a component premise the
    // representative omits would be silently dropped from the published
    // result, which the verdict contract forbids. Non-conflicting but unequal
    // premise sets are therefore refused rather than merged into a certificate
    // no provider issued. Widening this to a first-class composite premise
    // union is recorded as open work, not done here.
    if certificates
        .iter()
        .any(|certificate| certificate.evidence.assumptions_used != assumptions)
    {
        return VerificationOutcome::unknown(
            "QP012",
            "plugins declare different premise sets; the published composite would \
             understate its assumptions",
        );
    }
    VerificationOutcome {
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        certificate: certificates.first().cloned(),
    }
}

/// Composition with explicit required-set validation (design §3.3).
///
/// `compose_verification`'s conflict rules are kept verbatim and extended with
/// the two things protocol v1 could not express: the union of certificate
/// coverages must equal the scope's full core-derived obligation-ID set, and
/// an obligation discharged under two incomparable bounds does not merge. A
/// missing required provider or capability is therefore detected from the
/// inventory, not inferred from an absent certificate.
pub fn compose_verification_with_inventory(
    outcomes: Vec<VerificationOutcome>,
    required: &BTreeSet<String>,
) -> VerificationOutcome {
    if required.is_empty() {
        // §3.5: a complete derivation with zero obligations is not promoted in
        // this increment. Callers must not reach composition with an empty
        // inventory; refuse rather than compose vacuously.
        return VerificationOutcome::unknown(
            "QP016",
            "no obligations were derived; empty-scope promotion requires a \
             derivation-coverage witness",
        );
    }
    let composed = compose_verification(outcomes.clone());
    if composed.verdict != ProposedVerdict::Safe {
        return composed;
    }
    let mut covered = BTreeSet::new();
    let mut bounds = BTreeMap::<String, TaskBounds>::new();
    for certificate in outcomes
        .iter()
        .filter_map(|outcome| outcome.certificate.as_ref())
    {
        for id in certificate.coverage() {
            covered.insert(id.clone());
        }
        for (id, bound) in certificate.obligation_bounds() {
            if bounds
                .insert(id.clone(), bound.clone())
                .is_some_and(|previous| previous != *bound)
            {
                return VerificationOutcome::unknown(
                    "QP013",
                    format!("obligation `{id}` is discharged under incomparable bounds"),
                );
            }
        }
    }
    let missing = required.difference(&covered).count();
    if missing != 0 || covered != *required {
        return VerificationOutcome::unknown(
            "QP011",
            format!("{missing} core-derived obligation(s) are not covered by any certificate"),
        );
    }
    composed
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
                    witness: None,
                    obligation_bounds: BTreeMap::new(),
                    task_digest: None,
                    task: None,
                }),
            }
        };
        let composed = compose_verification(vec![outcome(true), outcome(false)]);
        assert_eq!(composed.verdict, ProposedVerdict::Unknown);
        assert_eq!(composed.diagnostics[0].code, "QP012");
    }
}

/// The published digest function package identities are expressed in; re-exported
/// so package tooling need not depend on the schema crate directly.
pub use qcore_schemas::sha256_hex;

pub mod embedded_template;
pub use embedded_template::{
    TEMPLATE_BEGIN, TEMPLATE_END, TemplateBindingError, envelope, extract_template,
    verify_embedded_template_binding,
};

pub mod package;
pub use package::verify_executable_package;
