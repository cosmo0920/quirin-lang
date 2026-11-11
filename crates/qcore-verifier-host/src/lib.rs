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

pub mod execution_policy;
pub use execution_policy::{EXECUTION_POLICY_VERSION, ExecutionPolicy, ExecutionPolicyError};

// Card W45-USER-CONTRACT-PLUGINS: a checker plugin's answer, classified into
// the four-value verdict by the certificate firewall.
pub mod plugin_firewall;
pub use plugin_firewall::{CounterexampleChecker, PluginClaim};

pub mod process_quota;
pub use process_quota::{
    DeclaredKindRole, MAX_EXECUTION_PROCESSES, MechanismAvailability, MechanismProbe, ProcessQuota,
    QuotaObservation, QuotaPlanRefusal,
};

#[cfg(unix)]
mod request_consumption;

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
    /// One provider's core-checked certificate. On a composed outcome this is
    /// present only when every component is the same certificate; see
    /// [`CompositeCertificate::representative`].
    pub certificate: Option<CoreCertificate>,
    /// The merge, published by [`compose_verification`] and
    /// [`compose_verification_with_inventory`] alone. A single-provider
    /// outcome from the firewall carries `None`: one certificate is not yet a
    /// composition, and nothing but composition may fill this field.
    pub composite: Option<CompositeCertificate>,
}

impl VerificationOutcome {
    /// A scoped `Unknown` with one host diagnostic. Never carries evidence:
    /// a refusal publishes no certificate and no composite.
    #[must_use]
    pub fn unknown(code: &str, message: impl Into<String>) -> Self {
        Self {
            verdict: ProposedVerdict::Unknown,
            diagnostics: vec![PluginDiagnostic {
                code: code.to_owned(),
                message: message.into(),
                fact_id: None,
            }],
            certificate: None,
            composite: None,
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
    /// The provider did not consume its whole request: part of the frame was
    /// never delivered, the pipe had no reader left, or bytes were still
    /// unread when it exited. Decided from consumption, never from whether
    /// the host's write or the provider's exit came first (card
    /// `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`). The text says which.
    RequestUnconsumed(String),
    /// A process other than the provider still held the request pipe's read
    /// end at the deadline, after the provider had exited successfully with
    /// both outputs closed: it could still read, so the request's consumption
    /// was never final (card `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`).
    RequestHeldByDescendant(String),
}

impl fmt::Display for ProviderFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(message) | Self::Io(message) | Self::Protocol(message) => {
                formatter.write_str(message)
            }
            Self::RequestUnconsumed(detail) => write!(formatter, "request-unconsumed: {detail}"),
            Self::RequestHeldByDescendant(detail) => {
                write!(formatter, "request-held-by-descendant: {detail}")
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
    /// The per-execution process quota this execution installs, and the
    /// delegated cgroup subtree it was derived from. `None` for every
    /// execution whose manifest declares no `max_execution_processes` and for
    /// every profile that installs no quota — which is what those records must
    /// say rather than leaving the question open. Absent from the serialized
    /// form when `None`, so records written before this field existed are
    /// byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_quota: Option<ProcessQuota>,
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
        let quota = self.process_quota.as_ref().map_or_else(
            || "; process_quota=none".to_owned(),
            |quota| format!("; process_quota={}", quota.summary()),
        );
        let confinement = format!("{confinement}{quota}");
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
    /// The verified executable changed between verification and this spawn, so
    /// nothing was spawned (card `P01-F3`). Distinct from `RefusedByProfile`:
    /// the profile was satisfiable and the *package identity* was not.
    RefusedByIdentity,
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
    /// The provider did not consume its whole request frame (card
    /// `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`). A successful exit is
    /// refused with it, and so is a broken request pipe, so the outcome does
    /// not depend on whether the host's write or the provider's exit came
    /// first. An unsuccessful exit still classifies as `Terminated`.
    RequestUnconsumed,
    /// The provider exited successfully with both outputs closed, but a
    /// process other than it still held the request pipe at the deadline (card
    /// `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`). The host decides only as
    /// the pipe's last holder, so this is the refusal of a decision the
    /// deadline cut off, distinct from an unconsumed request and from a live
    /// provider's timeout.
    RequestHeldByDescendant,
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
    /// What the per-execution process quota actually observed, read from the
    /// kernel after the child exited and before its cgroup was removed.
    /// `None` when no quota was installed, and — deliberately — also when the
    /// profile refused before spawning, because nothing ran to observe.
    /// Absent from the serialized form when `None`, so records written before
    /// this field existed are byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota_observed: Option<QuotaObservation>,
    /// The measured time-of-check to time-of-use residual, in microseconds:
    /// from the moment the verified executable's spawn-time re-check completed
    /// to the moment `spawn` returned, which is after the kernel has read the
    /// image. It is an upper bound on the window in which a writer could still
    /// substitute the file, and it is *not* zero — this mechanism narrows the
    /// window, it does not close it (card `P01-F3`).
    ///
    /// `None` when nothing was spawned, and when the provider carries no
    /// verified executable to re-check. Absent from the serialized form when
    /// `None`, so records written before this field existed are byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_window_us: Option<u64>,
}

impl ProcessExecutionRecord {
    /// A single-line summary suitable for a report row.
    pub fn summary(&self) -> String {
        let enforced = self.enforced.as_ref().map_or_else(
            || format!("profile={}; refused", self.profile),
            |e| e.summary(),
        );
        let observed = self
            .quota_observed
            .as_ref()
            .map_or_else(String::new, |quota| format!("; {}", quota.summary()));
        format!(
            "{enforced}{observed}; outcome={}; wall_time_ms={}; stdout_bytes={}; \
             stderr_bytes={}; exit_code={:?}; signal={:?}",
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

/// The `setrlimit` kind names, sorted. Both Linux profiles install exactly
/// this table, so the confined profile's composed plan is the limit profile's
/// plan (`docs/reports/remaining-p03/README.md` §2.3).
pub fn enforceable_rlimit_kinds(profile: ProcessExecutionProfile) -> Vec<&'static str> {
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

/// Every limit kind a profile can enforce, sorted, by any mechanism. Empty for
/// the legacy profile, which enforces no declared kind at all.
///
/// The confining profile enforces the `setrlimit` table *and*
/// [`MAX_EXECUTION_PROCESSES`], which it installs as a cgroup quota rather than
/// an `setrlimit` (card `P03-F`); the limit profile enforces the `setrlimit`
/// table only, and refuses the quota kind.
pub fn enforceable_limit_kinds(profile: ProcessExecutionProfile) -> Vec<&'static str> {
    let mut kinds = enforceable_rlimit_kinds(profile);
    if cfg!(target_os = "linux") && process_quota::profile_installs_quota(profile) {
        kinds.extend(process_quota::PROCESS_QUOTA_KINDS);
    }
    kinds.sort_unstable();
    kinds
}

/// Whether a [`ProcessProvider`]'s executable was verified, and the handle that
/// lets each spawn re-check it.
///
/// A [`ProcessProvider`] can only be built publicly through
/// [`verify_executable_package`], so a published provider always carries
/// [`ExecutableIdentity::Verified`]. The other variant exists solely inside
/// [`UnverifiedProvider`], which is deliberately not a [`VerifierProvider`] and
/// therefore cannot be registered.
enum ExecutableIdentity {
    Verified(Box<VerifiedExecutable>),
    /// Reachable only through [`UnverifiedProvider`]: nothing verified this
    /// package, so nothing can be re-checked before a spawn.
    Unverified,
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
    /// Per-mechanism probe selection. Defaults to asking the kernel, and can
    /// only force a mechanism *unavailable*, never available, so it can widen
    /// refusal but never weaken an execution.
    availability: MechanismAvailability,
    /// The most recent execution's evidence record. Interior mutability keeps
    /// [`VerifierProvider`]'s `&self` dispatch, which providers share behind an
    /// `Arc`, unchanged.
    last_execution: std::sync::Mutex<Option<ProcessExecutionRecord>>,
    /// The verified executable this provider re-checks before every spawn.
    identity: ExecutableIdentity,
}

impl ProcessProvider {
    fn build(
        manifest: PluginManifest,
        package_root: impl AsRef<Path>,
        identity: ExecutableIdentity,
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
            availability: MechanismAvailability::default(),
            last_execution: std::sync::Mutex::new(None),
            identity,
        })
    }

    /// The only construction that yields a registrable provider: the caller has
    /// verified the package's digest and embedded-template binding, and hands
    /// over the descriptor those checks read.
    pub(crate) fn verified(
        manifest: PluginManifest,
        package_root: impl AsRef<Path>,
        executable: VerifiedExecutable,
    ) -> Result<Self, ManifestError> {
        Self::build(
            manifest,
            package_root,
            ExecutableIdentity::Verified(Box::new(executable)),
        )
    }

    /// Constructs a provider over an *unverified* package. Private on purpose:
    /// [`UnverifiedProvider`] is the only way to reach it, and that type cannot
    /// enter a [`VerifierRegistry`].
    pub(crate) fn unverified(
        manifest: PluginManifest,
        package_root: impl AsRef<Path>,
    ) -> Result<Self, ManifestError> {
        Self::build(manifest, package_root, ExecutableIdentity::Unverified)
    }

    /// The verified executable behind this provider, if it has one.
    ///
    /// `None` only inside [`UnverifiedProvider`].
    pub fn verified_executable(&self) -> Option<&VerifiedExecutable> {
        match &self.identity {
            ExecutableIdentity::Verified(executable) => Some(executable),
            ExecutableIdentity::Unverified => None,
        }
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

    /// Overrides which kernel mechanisms this provider treats as available.
    ///
    /// The override is one-directional by construction:
    /// [`MechanismProbe`] has no variant that forces a mechanism *available*,
    /// so this call can only cause additional pre-spawn refusals. It can never
    /// produce an execution that ran with weaker enforcement than the kernel
    /// offered, which is why it is safe to expose rather than hide behind a
    /// build flag. Its purpose is to execute the refusal grounds on a host
    /// whose kernel provides every mechanism (card `P03-F`; a genuinely absent
    /// user namespace is exercised separately, without this).
    #[must_use]
    pub const fn with_mechanism_availability(
        mut self,
        availability: MechanismAvailability,
    ) -> Self {
        self.availability = availability;
        self
    }

    /// The mechanism probe selection in force.
    pub const fn mechanism_availability(&self) -> MechanismAvailability {
        self.availability
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
                    process_quota: None,
                })
            }
            ProcessExecutionProfile::LinuxRlimitV1 => self.linux_rlimit_v1_limits(self.profile),
            ProcessExecutionProfile::LinuxConfinedV1 => {
                // The confinement is checked first so an unavailable mechanism
                // names the mechanism, not the composed limit profile.
                let confinement = self.linux_confinement()?;
                let quota = self.linux_process_quota()?;
                let mut enforced = self.linux_rlimit_v1_limits(self.profile)?;
                enforced.confinement = Some(confinement);
                enforced.process_quota = quota;
                Ok(enforced)
            }
        }
    }

    /// The per-execution process quota this manifest declares, or the
    /// pre-spawn refusal that keeps the package from running.
    ///
    /// `Ok(None)` is the manifest declaring no quota kind at all, which is a
    /// complete answer: the execution installs no quota and its record says so.
    /// It is never the answer to a declared quota the host could not install.
    fn linux_process_quota(&self) -> Result<Option<ProcessQuota>, ProviderFailure> {
        let Some(declared) = self
            .manifest
            .limits
            .additional
            .get(process_quota::MAX_EXECUTION_PROCESSES)
        else {
            return Ok(None);
        };
        if *declared == 0 {
            return Err(ProviderFailure::Protocol(
                QuotaPlanRefusal::ZeroQuota {
                    kind: process_quota::MAX_EXECUTION_PROCESSES.to_owned(),
                }
                .to_string(),
            ));
        }
        #[cfg(target_os = "linux")]
        {
            process_quota::linux::describe(*declared, self.availability.process_quota)
                .map(Some)
                .map_err(|error| {
                    ProviderFailure::Protocol(format!(
                        "execution profile `{}` cannot install the declared `{}` quota: {error}",
                        ProcessExecutionProfile::LinuxConfinedV1,
                        process_quota::MAX_EXECUTION_PROCESSES
                    ))
                })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(ProviderFailure::Protocol(format!(
                "execution profile `{}` is unavailable on this target; its per-execution process \
                 quota is a Linux cgroup v2 mechanism and it makes no isolation claim elsewhere",
                ProcessExecutionProfile::LinuxConfinedV1
            )))
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
            confinement::linux::describe(
                policy,
                &self.package_root,
                self.manifest.network_access,
                self.availability,
            )
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
        let rlimit_kinds = enforceable_rlimit_kinds(profile);
        for kind in limits.additional.keys() {
            // Every declared kind must have a role under this profile. A kind
            // this profile installs by a mechanism other than `setrlimit` — the
            // per-execution process quota — has one and is skipped here; a kind
            // with no role at all is refused, never ignored.
            match process_quota::declared_kind_role(profile, kind, &rlimit_kinds) {
                Some(DeclaredKindRole::Rlimit) => {}
                Some(DeclaredKindRole::ProcessQuota) => continue,
                None if process_quota::PROCESS_QUOTA_KINDS.contains(&kind.as_str()) => {
                    return Err(ProviderFailure::Protocol(
                        QuotaPlanRefusal::ProfileEnforcesNoQuota {
                            profile,
                            kind: kind.clone(),
                        }
                        .to_string(),
                    ));
                }
                None => {
                    return Err(ProviderFailure::Protocol(format!(
                        "execution profile `{profile}` cannot enforce declared limit kind \
                         `{kind}`; it enforces [{}]",
                        enforceable_limit_kinds(profile).join(", ")
                    )));
                }
            }
            debug_assert!(resource_of(kind).is_some());
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
            process_quota: None,
        })
    }

    #[cfg(unix)]
    fn record_execution(&self, record: ProcessExecutionRecord) {
        if let Ok(mut slot) = self.last_execution.lock() {
            *slot = Some(record);
        }
    }

    /// Loads and **verifies** an installed package.
    ///
    /// Before card `P01-F3` this was a plain loader that checked neither the
    /// declared digest nor the embedded-template binding, so a registry built
    /// from it ran whatever the directory contained. It now delegates to
    /// [`verify_executable_package`], which is the only construction of a
    /// registrable provider. A caller that genuinely wants an unverified load —
    /// the conformance harness drives deliberately mutated packages — asks for
    /// it by name through [`UnverifiedProvider::load`], and gets a type that
    /// cannot be registered.
    pub fn from_package(package_root: impl AsRef<Path>) -> Result<Self, PackageLoadError> {
        verify_executable_package(package_root)
    }
}

/// A package loaded **without** verifying its digest or its embedded-template
/// binding.
///
/// The type is the statement: `UnverifiedProvider` does not implement
/// [`VerifierProvider`], so it cannot be handed to
/// [`VerifierRegistry::register`], and it exposes no way to extract the
/// [`ProcessProvider`] it wraps by value. Reading its manifest, inspecting the
/// limits a profile would enforce, and running it directly stay available — that
/// is what the adversarial harness and the P02/P03 execution fixtures need —
/// but its results can never reach a verdict route by accident.
///
/// [`UnverifiedProvider::into_verified`] is the one bridge, and it verifies.
///
/// A registry cannot take one:
///
/// ```compile_fail
/// use std::sync::Arc;
/// use qcore_verifier_host::{UnverifiedProvider, VerifierRegistry};
/// let provider = UnverifiedProvider::load("package").unwrap();
/// let mut registry = VerifierRegistry::default();
/// registry.register(Arc::new(provider)).unwrap();
/// ```
///
/// Nor can the wrapped provider be moved out of it and registered instead:
///
/// ```compile_fail
/// use qcore_verifier_host::{ProcessProvider, UnverifiedProvider};
/// let provider = UnverifiedProvider::load("package").unwrap();
/// let taken: ProcessProvider = *provider;
/// ```
///
/// What it *can* do is read its own manifest, which is a load, not a verdict:
///
/// ```no_run
/// use qcore_verifier_host::{UnverifiedProvider, VerifierProvider};
/// let provider = UnverifiedProvider::load("package").unwrap();
/// println!("{}", provider.manifest().plugin_id);
/// ```
pub struct UnverifiedProvider {
    inner: ProcessProvider,
    package_root: std::path::PathBuf,
}

impl UnverifiedProvider {
    /// Wraps a caller-supplied manifest over a package directory, checking only
    /// that the manifest is well formed and declares an executable.
    pub fn new(
        manifest: PluginManifest,
        package_root: impl AsRef<Path>,
    ) -> Result<Self, ManifestError> {
        let package_root = package_root.as_ref().to_path_buf();
        Ok(Self {
            inner: ProcessProvider::unverified(manifest, &package_root)?,
            package_root,
        })
    }

    /// Reads `verifier-plugin.json` and nothing else: no digest comparison, no
    /// template binding, no execution.
    pub fn load(package_root: impl AsRef<Path>) -> Result<Self, PackageLoadError> {
        let package_root = package_root.as_ref();
        let manifest_path = package_root.join("verifier-plugin.json");
        let failure = |error: String| PackageLoadError {
            path: manifest_path.to_string_lossy().into_owned(),
            message: error,
        };
        let bytes = std::fs::read(&manifest_path).map_err(|e| failure(e.to_string()))?;
        let manifest =
            serde_json::from_slice::<PluginManifest>(&bytes).map_err(|e| failure(e.to_string()))?;
        Self::new(manifest, package_root).map_err(|e| failure(e.to_string()))
    }

    /// Selects the execution profile, exactly as [`ProcessProvider`] does.
    #[must_use]
    pub fn with_execution_profile(mut self, profile: ProcessExecutionProfile) -> Self {
        self.inner = self.inner.with_execution_profile(profile);
        self
    }

    /// Supplies the explicit filesystem reach a confining profile grants.
    #[must_use]
    pub fn with_confinement_policy(mut self, policy: ConfinementPolicy) -> Self {
        self.inner = self.inner.with_confinement_policy(policy);
        self
    }

    /// Overrides which kernel mechanisms this provider treats as available.
    #[must_use]
    pub fn with_mechanism_availability(mut self, availability: MechanismAvailability) -> Self {
        self.inner = self.inner.with_mechanism_availability(availability);
        self
    }

    /// Verifies the package and returns a registrable provider, preserving the
    /// execution profile, confinement policy and mechanism selection chosen
    /// here.
    ///
    /// Fails closed: a package whose digest, entrypoint or embedded template
    /// does not bind stays unverified, and no provider is produced.
    pub fn into_verified(self) -> Result<ProcessProvider, PackageLoadError> {
        let mut provider = verify_executable_package(&self.package_root)?;
        provider = provider.with_execution_profile(self.inner.profile);
        if let Some(policy) = self.inner.confinement_policy.clone() {
            provider = provider.with_confinement_policy(policy);
        }
        Ok(provider.with_mechanism_availability(self.inner.availability))
    }
}

impl std::ops::Deref for UnverifiedProvider {
    type Target = ProcessProvider;

    /// Read-only access to the wrapped provider. Deliberately no `DerefMut` and
    /// no owning accessor: a `&ProcessProvider` cannot become the
    /// `Arc<dyn VerifierProvider>` a registry takes.
    fn deref(&self) -> &ProcessProvider {
        &self.inner
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

/// The transport loop's backoff quantum. It bounds how long one wait may last,
/// never how much may be transferred before the next wait.
#[cfg(unix)]
const TRANSPORT_BACKOFF: Duration = Duration::from_millis(1);

/// Wait for one of the live transport pipes to become ready, for at most the
/// backoff quantum and never past the declared deadline.
///
/// Sleeping a fixed quantum whenever a bounded chunk did not move imposes a
/// throughput ceiling of one pipe capacity per quantum — on a host whose pipe
/// capacity is 8 KiB that is 8 KiB/ms, so an 8 MiB request needs at least 1,024
/// quanta and exhausts a 1,000 ms budget while the provider is actively
/// draining it. Whether that ceiling is reached is decided by a race the host
/// loses precisely when it is *fastest*: the peer has not yet been scheduled to
/// drain the pipe by the time an unloaded loop comes back around, so the full
/// quantum is charged for an event that takes microseconds. Waiting for
/// readiness removes the ceiling while preserving both properties the bound
/// relies on: an idle provider never spins, and no wait outlives the deadline
/// or the quantum.
#[cfg(unix)]
fn backoff_until_ready(
    writable: Option<std::os::fd::RawFd>,
    readable_output: Option<std::os::fd::RawFd>,
    readable_error: Option<std::os::fd::RawFd>,
    deadline: Instant,
) {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        // The caller's very next act is the deadline check. Waiting here could
        // only delay the refusal it is about to return.
        return;
    }
    let wait = remaining.min(TRANSPORT_BACKOFF);
    let mut fds = Vec::with_capacity(3);
    if let Some(fd) = writable {
        fds.push(libc::pollfd {
            fd,
            events: libc::POLLOUT,
            revents: 0,
        });
    }
    for fd in [readable_output, readable_error].into_iter().flatten() {
        fds.push(libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        });
    }
    if fds.is_empty() {
        // Every pipe is finished, so only the direct child's exit can still
        // change the loop's state, and that is observed by `try_wait` rather
        // than by readiness. Back off exactly as before.
        thread::sleep(wait);
        return;
    }
    // The descriptors are owned by the live pipes for the whole call, and the
    // slice is valid for its length. A failed `poll` is not a transport
    // failure: it degrades to the timed backoff this call replaced, so an
    // unpollable descriptor cannot turn into a spin.
    let ready = unsafe {
        libc::poll(
            fds.as_mut_ptr(),
            fds.len() as libc::nfds_t,
            wait.as_millis() as libc::c_int,
        )
    };
    if ready < 0 {
        thread::sleep(wait);
    }
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

/// The per-execution cgroup a declared quota installs, carried from the
/// pre-spawn creation to the `pre_exec` join and the post-run observation.
/// `None` is an execution that declares no quota.
#[cfg(target_os = "linux")]
type QuotaHandle = Option<process_quota::linux::QuotaSession>;
#[cfg(all(unix, not(target_os = "linux")))]
type QuotaHandle = Option<()>;

/// The parent's duplicate of a declared quota's `cgroup.procs` descriptor,
/// carried to the child's `pre_exec` join. `None` exactly when no quota was
/// declared: a declared quota without one is refused before spawning.
#[cfg(target_os = "linux")]
type QuotaProcs = Option<std::os::fd::OwnedFd>;
#[cfg(all(unix, not(target_os = "linux")))]
type QuotaProcs = Option<()>;

/// The kernel objects a profile installs in the child, carried together from
/// the pre-spawn build to the `pre_exec` closures. Bundled so the spawn path
/// takes one mechanism argument rather than one per mechanism.
#[cfg(unix)]
struct PreparedMechanisms {
    confinement: ConfinementHandle,
    quota_procs: QuotaProcs,
}

/// What the spawn path observed, carried back out so the execution record can
/// classify the outcome and publish the measured residual window.
#[cfg(unix)]
#[derive(Default)]
struct SpawnObservation {
    /// The direct child's exit status, once it has been reaped.
    status: Option<std::process::ExitStatus>,
    /// The package-identity re-check refused: nothing was spawned.
    identity_refused: bool,
    /// Microseconds from the completed re-check to `spawn` returning.
    identity_window_us: Option<u64>,
    /// `spawn` returned a child. Every `pre_exec` step succeeded, the quota
    /// join first, so a declared quota's cgroup holds that child.
    spawned: bool,
}

/// A write descriptor on the per-execution cgroup's `cgroup.procs`, for the
/// child's `pre_exec` closure. Opened in the parent, with the parent's
/// credentials.
///
/// `Ok(None)` only when no quota was declared. A declared quota whose
/// descriptor cannot be duplicated is an error, which the caller turns into a
/// refusal before spawning: mapping it to "no join" ran the child outside its
/// cgroup while the record named the quota (card
/// `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`).
#[cfg(target_os = "linux")]
fn quota_descriptor(quota: &QuotaHandle, probe: MechanismProbe) -> Result<QuotaProcs, String> {
    quota
        .as_ref()
        .map(|session| session.procs_descriptor(probe))
        .transpose()
}

#[cfg(all(unix, not(target_os = "linux")))]
fn quota_descriptor(quota: &QuotaHandle, probe: MechanismProbe) -> Result<QuotaProcs, String> {
    let _ = (quota, probe);
    Ok(None)
}

/// Reads the quota's own counters and removes the per-execution cgroup.
#[cfg(target_os = "linux")]
fn finish_quota(quota: QuotaHandle) -> Option<QuotaObservation> {
    quota.map(process_quota::linux::QuotaSession::finish)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn finish_quota(quota: QuotaHandle) -> Option<QuotaObservation> {
    let _ = quota;
    None
}

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
                    quota_observed: None,
                    identity_window_us: None,
                });
                return Err(error);
            }
        };
        // Building the Landlock ruleset opens every admitted path, and creating
        // the per-execution cgroup writes into a delegated subtree, so both can
        // fail; both happen here, before anything is spawned, so a failure is a
        // refusal rather than an unconfined or unbounded execution. So does the
        // duplicate of the cgroup's join descriptor: without it the child
        // could not join, and a child outside its cgroup is an unbounded
        // execution under a record that names the bound.
        let prepared = self
            .prepare_confinement(&enforced)
            .and_then(|confinement| Ok((confinement, self.prepare_quota(&enforced)?)))
            .and_then(|(confinement, quota)| {
                let procs = self.prepare_quota_join(&enforced, &quota)?;
                Ok((confinement, quota, procs))
            });
        let (confinement, quota, quota_procs) = match prepared {
            Ok(prepared) => prepared,
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
                    quota_observed: None,
                    identity_window_us: None,
                });
                return Err(error);
            }
        };
        let mut output = Vec::new();
        let mut error_output = Vec::new();
        let mut observed = SpawnObservation::default();
        let result = self.run_process(
            frame,
            &enforced,
            PreparedMechanisms {
                confinement,
                quota_procs,
            },
            &mut output,
            &mut error_output,
            &mut observed,
        );
        let status = observed.status;
        // Observed before the cgroup is removed: `rmdir` only succeeds on an
        // empty cgroup, and the counters go with it. Only a spawned child is
        // known to have joined (the join is its first `pre_exec` step, and a
        // failed step fails the spawn), so an execution refused before the
        // spawn, or whose spawn failed, records no observation of a cgroup no
        // child was shown to join; dropping the session still removes it.
        let quota_observed = if observed.spawned {
            finish_quota(quota)
        } else {
            drop(quota);
            None
        };
        let outcome = match &result {
            // The package identity re-check refused: nothing was spawned, and
            // the refusal is its own outcome rather than a profile refusal.
            Err(_) if observed.identity_refused => ProcessOutcome::RefusedByIdentity,
            Ok(_) => ProcessOutcome::Completed,
            Err(ProviderFailure::Spawn(_)) => ProcessOutcome::SpawnFailed,
            Err(ProviderFailure::Timeout) => ProcessOutcome::WallClockTimeout,
            Err(ProviderFailure::OutputLimit) => ProcessOutcome::OutputLimit,
            Err(ProviderFailure::Crashed(_)) => ProcessOutcome::Terminated,
            Err(ProviderFailure::Io(_)) => ProcessOutcome::Io,
            Err(ProviderFailure::RequestUnconsumed(_)) => ProcessOutcome::RequestUnconsumed,
            Err(ProviderFailure::RequestHeldByDescendant(_)) => {
                ProcessOutcome::RequestHeldByDescendant
            }
            // `Crashed` gains stderr detail through `with_detail`, which
            // rewrites it as `Protocol`; a terminating status still classifies
            // the execution as terminated.
            Err(ProviderFailure::Protocol(_)) => status
                .filter(|status| !status.success())
                .map_or(ProcessOutcome::Protocol, |_| ProcessOutcome::Terminated),
        };
        self.record_execution(ProcessExecutionRecord {
            profile: self.profile,
            // A refused identity spawned nothing, so it reports no enforced
            // limits, exactly as a refused profile does.
            enforced: if observed.identity_refused {
                None
            } else {
                Some(enforced)
            },
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
            quota_observed,
            identity_window_us: observed.identity_window_us,
        });
        result
    }

    /// Creates the per-execution cgroup a declared quota needs.
    ///
    /// `None` means no quota was declared, which its record states explicitly
    /// through `EnforcedLimits::process_quota`.
    #[cfg(target_os = "linux")]
    fn prepare_quota(&self, enforced: &EnforcedLimits) -> Result<QuotaHandle, ProviderFailure> {
        let Some(quota) = enforced.process_quota.as_ref() else {
            return Ok(None);
        };
        process_quota::linux::QuotaSession::create(quota)
            .map(Some)
            .map_err(|error| {
                ProviderFailure::Protocol(format!(
                    "execution profile `{}` cannot install the declared `{}` quota: {error}",
                    enforced.profile, quota.kind
                ))
            })
    }

    /// Duplicates the declared quota's join descriptor for the child, or
    /// refuses the execution before anything is spawned.
    ///
    /// `None` means no quota was declared, exactly as for [`Self::prepare_quota`].
    fn prepare_quota_join(
        &self,
        enforced: &EnforcedLimits,
        quota: &QuotaHandle,
    ) -> Result<QuotaProcs, ProviderFailure> {
        // A session exists only for a declared quota, so the kind is always
        // the declared one; the constant only spells the same kind.
        let kind = enforced
            .process_quota
            .as_ref()
            .map_or(process_quota::MAX_EXECUTION_PROCESSES, |quota| &quota.kind);
        quota_descriptor(quota, self.availability.quota_join_descriptor).map_err(|error| {
            ProviderFailure::Protocol(format!(
                "execution profile `{}` cannot join the child to its declared `{kind}` quota, so \
                 the execution is refused before spawning: {error}",
                enforced.profile
            ))
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn prepare_quota(&self, enforced: &EnforcedLimits) -> Result<QuotaHandle, ProviderFailure> {
        // Off Linux no profile reaches here with a quota: `enforced_limits`
        // refuses a declared quota kind before this point.
        debug_assert!(enforced.process_quota.is_none());
        let _ = enforced;
        Ok(None)
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
        mechanisms: PreparedMechanisms,
        output: &mut Vec<u8>,
        error_output: &mut Vec<u8>,
        observed: &mut SpawnObservation,
    ) -> Result<T, ProviderFailure> {
        use std::os::unix::process::CommandExt;
        let PreparedMechanisms {
            confinement,
            quota_procs,
        } = mechanisms;
        let PluginEntrypoint::Executable { path } = &self.manifest.entrypoint else {
            return Err(ProviderFailure::Protocol("missing executable".into()));
        };
        #[cfg(not(target_os = "linux"))]
        let _ = (confinement, quota_procs);
        let limit = enforced.stdout_bytes;
        let wire_limit = limit.checked_add(4).ok_or(ProviderFailure::OutputLimit)?;
        // A verified provider executes the *canonical* path verification
        // resolved and re-checks below, not the spelled entrypoint. The two
        // differ whenever the entrypoint is a symbolic link, and a check on one
        // path with an `execve` on another would be no check at all.
        let executable = match self.verified_executable() {
            Some(executable) => executable.path().to_path_buf(),
            None => Path::new(&self.package_root).join(path),
        };
        // The host creates the request pipe itself so that it can observe what
        // the provider left unread (`request_consumption`).
        let request_consumption::RequestPipe {
            child_stdin,
            writer: request_writer,
            held_probe,
        } = request_consumption::request_pipe()
            .map_err(|error| ProviderFailure::Io(error.to_string()))?;
        // Off Linux the probe is the read end kept from here, and the decision
        // reads its count as before (the other holders are unobservable there);
        // on Linux it is opened once the whole frame is written, and each
        // holder check that finds the pipe held reopens it.
        let mut probe = held_probe;
        let mut command = Command::new(&executable);
        command
            .current_dir(&self.package_root)
            .env_clear()
            .process_group(0)
            .stdin(Stdio::from(child_stdin))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // The quota is joined *first*, before the confinement and before every
        // `setrlimit`: the child must already be a member of the per-execution
        // cgroup when its first descendant could exist, and joining after the
        // user-namespace unshare would depend on how the kernel re-checks a
        // migration's credentials.
        #[cfg(target_os = "linux")]
        if let Some(procs) = quota_procs {
            use std::os::fd::AsRawFd;
            // SAFETY: this runs in the forked child before `exec`. It allocates
            // nothing, acquires no lock and makes one `write`; `procs` is owned
            // by the closure and stays open until the spawn completes.
            unsafe {
                command.pre_exec(move || process_quota::linux::join(procs.as_raw_fd()));
            }
        }
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
        // Last act before the spawn: confirm the file about to be executed is
        // still the file that was verified. A provider without a verified
        // executable — reachable only through `UnverifiedProvider`, which no
        // registry accepts — has nothing to re-check and says so.
        let checked_at = match self.verified_executable() {
            Some(executable) => {
                if let Err(refusal) = executable.revalidate(&self.manifest) {
                    observed.identity_refused = true;
                    return Err(ProviderFailure::Protocol(refusal.to_string()));
                }
                Some(Instant::now())
            }
            None => None,
        };
        // The declared wall deadline starts here, *after* the host's own
        // checking. A package's `limits.timeout_ms` bounds how long the package
        // may run; charging the host's re-read of the executable against it
        // would shorten every provider's budget by the cost of a check the
        // provider did not ask for, and on an unoptimized build that cost is
        // comparable to the budget itself.
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(enforced.wall_clock_ms))
            .ok_or_else(|| {
                ProviderFailure::Protocol("provider timeout exceeds clock range".into())
            })?;
        let mut child = ProviderChild(
            command
                .spawn()
                .map_err(|error| ProviderFailure::Spawn(error.to_string()))?,
        );
        observed.spawned = true;
        // The command still owns the parent's copy of the child's stdin (and
        // the spawn-time closures). Close it now: while the host writes, it
        // must hold no read end, so that a broken pipe keeps meaning that the
        // provider has no reader left.
        drop(command);
        // `spawn` returns after the child has reported a successful `execve`, so
        // this is a genuine upper bound on the residual window rather than an
        // estimate of it.
        observed.identity_window_us = checked_at
            .map(|checked_at| checked_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64);
        let mut stdin = Some(request_writer);
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
        // A request write met a broken pipe: no reader was left.
        let mut reader_lost = false;
        // The direct child exited successfully with both outputs at end of
        // file, but another process still held the request pipe at the last
        // look, so the decision waits (`request_consumption::holders`).
        let mut held = false;
        loop {
            // Observe direct-child exit before deciding which bound failed.
            // An exited provider can leave descendant-held output pipes even
            // when its input reader vanished before our first write.
            if observed.status.is_none() {
                observed.status = child
                    .0
                    .try_wait()
                    .map_err(|error| ProviderFailure::Io(error.to_string()))?;
            }
            if Instant::now() >= deadline {
                if observed.status.is_none() && reader_lost {
                    return Err(request_consumption::reader_lost_at_deadline(
                        frame.len(),
                        sent,
                    ));
                }
                // The provider is done; only the request pipe's other holder
                // kept the decision open.
                if held {
                    return Err(request_consumption::held_at_deadline(frame.len(), sent));
                }
                return Err(ProviderFailure::Timeout);
            }
            let progress_before = (sent, output.len(), error_output.len());
            if let Some(input) = stdin.as_mut() {
                match input.write(&frame[sent..]) {
                    // Writing nothing into a pipe means no reader took it.
                    Ok(0) => {
                        reader_lost = true;
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
                        // the lost reader while draining under the same bound;
                        // it refuses a successful exit as an unconsumed request.
                        reader_lost = true;
                        stdin = None;
                    }
                    Err(error) => return Err(ProviderFailure::Io(error.to_string())),
                }
            }
            if sent == frame.len()
                && let Some(input) = stdin.take()
            {
                // The probe is taken before the write end closes: on Linux it
                // is opened through that write end.
                #[cfg(target_os = "linux")]
                {
                    probe = Some(request_consumption::probe_after_full_write(&input));
                }
                drop(input);
            }
            if !output_done {
                output_done = read_available(&mut stdout, output, wire_limit)?;
            }
            if !error_done {
                error_done = read_available(&mut stderr, error_output, enforced.stderr_bytes)?;
            }
            // The decision no longer waits for the request write to finish or
            // break: once the direct child has exited and both outputs are at
            // end of file, an undelivered remainder is itself unconsumed.
            if let Some(exit) = observed.status
                && output_done
                && error_done
            {
                // A failed exit explains an early input close. Report it
                // consistently whether the write or the exit was observed
                // first; successful exits still cannot admit partial requests.
                if !exit.success() {
                    return Err(ProviderFailure::Crashed(exit.code())
                        .with_detail(&String::from_utf8_lossy(error_output)));
                }
                // Decided from what the provider consumed, never from whether
                // the host's write or the provider's exit came first, and only
                // once no other process can read any more: a broken pipe has
                // no reader left, and otherwise the host must be the pipe's
                // last holder. While another process holds it the decision
                // waits, bounded by the deadline above.
                let holders = if reader_lost {
                    // No reader is left and none can appear. The count is
                    // gone with the pipe; `classify` refuses a lost reader
                    // before it reads the holders.
                    request_consumption::Holders::Released { unread: 0 }
                } else {
                    request_consumption::holders(stdin.is_some(), &mut probe)
                };
                if holders == request_consumption::Holders::Held {
                    held = true;
                } else {
                    request_consumption::classify(frame.len(), sent, reader_lost, holders)?;
                    return decode_protocol_frame(output, limit);
                }
            }
            // Back off only when all pipes would block, and then wait for one of
            // them to become ready rather than for a fixed quantum. Sleeping
            // after each bounded chunk imposes a throughput ceiling of one pipe
            // capacity per quantum, which can exhaust the deadline while a
            // provider is actively responding. Each iteration still checks the
            // deadline and services both output pipes once, so continuous
            // output cannot starve either.
            if progress_before == (sent, output.len(), error_output.len()) {
                use std::os::fd::AsRawFd;
                backoff_until_ready(
                    stdin.as_ref().map(|input| input.as_raw_fd()),
                    (!output_done).then(|| stdout.as_raw_fd()),
                    (!error_done).then(|| stderr.as_raw_fd()),
                    deadline,
                );
            }
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
    ///
    /// Every discovered package is **verified** before it is registered: its
    /// declared digest and its embedded-template binding must hold (card
    /// `P01-F3`; before it, discovery registered whatever the directory
    /// contained). A package that does not verify is reported as an error and
    /// left unregistered, never registered with a weaker check.
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
            match verify_executable_package(&path) {
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
        match self.dispatch_v2(request, task) {
            Ok((manifest, response)) => {
                self.check_response_v2(manifest, request, task, checker, response)
            }
            Err(refusal) => *refusal,
        }
    }

    /// The gates and the provider call of [`Self::verify_v2`], up to the
    /// firewall: the provider's raw response, or the refusal that stopped the
    /// request first. Card W45-USER-CONTRACT-PLUGINS reads the raw proposal
    /// through this, so the plugin firewall (`plugin_firewall`) can tell an
    /// uncertified claim from a failure; nothing here accepts anything.
    fn dispatch_v2(
        &self,
        request: &VerificationRequestV2,
        task: &VerificationTask,
    ) -> Result<(&PluginManifest, VerificationResponseV2), Box<VerificationOutcome>> {
        fn unknown<T>(
            code: &str,
            message: impl Into<String>,
        ) -> Result<T, Box<VerificationOutcome>> {
            Err(Box::new(VerificationOutcome::unknown(code, message)))
        }
        if let Err(error) = request.validate() {
            return unknown("QP001", error.to_string());
        }
        if let Err(error) = task.validate() {
            return unknown("QP001", error.to_string());
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
            return unknown(
                "QP001",
                "the request's task digest is not the digest of the core-derived task",
            );
        }
        if let Some(input) = &request.source_effect
            && task.contract_sources.get(input.digest_key()) != Some(&input.digest())
        {
            return unknown("QP001", "source projection is not bound to the core task");
        }
        if let Some(input) = &request.reactor_program
            && task.contract_sources.get(input.digest_key()) != Some(&input.digest())
        {
            return unknown(
                "QP001",
                "reactor program projection is not bound to the core task",
            );
        }
        if let Some(input) = &request.teardown_source
            && task.contract_sources.get(input.digest_key()) != Some(&input.digest())
        {
            return unknown(
                "QP001",
                "teardown source projection is not bound to the core task",
            );
        }
        if let Some(input) = &request.call_use_source
            && task.contract_sources.get(input.digest_key()) != Some(&input.digest())
        {
            return unknown(
                "QP001",
                "call-use source projection is not bound to the core task",
            );
        }
        if let Some(input) = &request.cleanup_replay_source
            && task.contract_sources.get(input.digest_key()) != Some(&input.digest())
        {
            return unknown(
                "QP001",
                "cleanup replay projection is not bound to the core task",
            );
        }
        if let Some(input) = &request.composition_source
            && task.contract_sources.get(input.digest_key()) != Some(&input.digest())
        {
            return unknown(
                "QP001",
                "composition projection is not bound to the core task",
            );
        }
        // A capability whose derivation could not classify every construct can
        // never reach `Safe`, whatever a provider returns (design §1.4).
        if task.is_incomplete_for(request.capability) {
            return unknown(
                "QP016",
                format!(
                    "the core derivation is incomplete for `{}`; no witness can discharge \
                     an unknown obligation inventory",
                    request.capability
                ),
            );
        }
        let Some(entry) = self.entries.get(&request.plugin_id) else {
            return unknown(
                "QP002",
                format!("plugin `{}` is not installed", request.plugin_id),
            );
        };
        if !entry.enabled {
            return unknown(
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
            return unknown("QP004", "incompatible protocol version");
        }
        if !manifest.supports(request.capability, &request.target) {
            return unknown(
                "QP005",
                "plugin does not declare the requested capability and target profile",
            );
        }
        match entry.provider.verify_v2(request) {
            Ok(response) => Ok((manifest, response)),
            Err(error) => unknown("QP006", error.to_string()),
        }
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
                composite: None,
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
                    composite: None,
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
                composite: None,
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
                    composite: None,
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

/// The composite of several core-checked certificates over one unit (E03-B1).
///
/// A [`CoreCertificate`] is attributed to one plugin, one capability and one
/// core-derived task, so no single component certificate can stand for the
/// merge. This record is that merge, and it is the only honest publication of
/// a composed `Safe`:
///
/// - **premises: union.** The composite holds only where *every* component's
///   premises hold, and more assumptions is a weaker claim. Publishing one
///   component's premise set would drop an assumption the composite genuinely
///   rests on, which the verdict contract forbids. A key two components bind
///   to different values is not a union but a contradiction: `QP012`.
/// - **coverage: union.** Every identifier here was discharged by *some*
///   component's core-checked replay, and
///   [`compose_verification_with_inventory`] compares exactly this union to
///   the core's own required set. Intersection would be wrong in the unsound
///   direction as a denominator (it under-reports what was required) and wrong
///   in the honest direction as a numerator (it would discard real discharges).
/// - **obligation bounds: union, conflicts refused.** Each entry is the bound
///   the discharging component actually checked that obligation under; one
///   identifier under two bounds is `QP013`, never a silent widening.
/// - **task bounds: intersection (the tightest envelope).** A composite claim
///   is only as strong as its weakest component, so the published path space
///   is the narrower one and `observed_runs_only` is the disjunction: a
///   composite containing one recorded-run component is a recorded-run claim.
/// - **provenance: sets.** Plugin identifiers, capabilities and task digests
///   are published as the whole sets, never one component's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositeCertificate {
    certificates: Vec<CoreCertificate>,
    unit: UnitIdentity,
    premises: BTreeMap<String, bool>,
    coverage: BTreeSet<String>,
    obligation_bounds: BTreeMap<String, TaskBounds>,
    bounds: Option<TaskBounds>,
    plugin_ids: BTreeSet<PluginId>,
    capabilities: BTreeSet<VerifierCapability>,
    task_digests: BTreeSet<String>,
}

/// The narrower of two path-space bounds. `SinglePath` is the least coverage
/// any derivation claims, so it absorbs; two explicit path counts take the
/// minimum. Nothing in this order is provider-supplied.
const fn tighter_paths(left: PathBounds, right: PathBounds) -> PathBounds {
    match (left, right) {
        (PathBounds::SinglePath, _) | (_, PathBounds::SinglePath) => PathBounds::SinglePath,
        (PathBounds::BoundedPaths { paths: left }, PathBounds::BoundedPaths { paths: right }) => {
            PathBounds::BoundedPaths {
                paths: if left <= right { left } else { right },
            }
        }
    }
}

/// `None` means "not applicable" or "structural", never "unbounded", so a
/// component that states an explicit count constrains the composite and a
/// component that states none does not relax it.
const fn tighter_count(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(if left <= right { left } else { right }),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

const fn tighter_bounds(left: &TaskBounds, right: &TaskBounds) -> TaskBounds {
    TaskBounds {
        path_exploration: tighter_paths(left.path_exploration, right.path_exploration),
        schedules_explored: tighter_count(left.schedules_explored, right.schedules_explored),
        observed_runs_only: left.observed_runs_only || right.observed_runs_only,
        max_operations: tighter_count(left.max_operations, right.max_operations),
    }
}

impl CompositeCertificate {
    /// Builds the composite, or the refusal that keeps it unpublished. Every
    /// refusal is an `Unknown`; none of them can produce a verdict.
    fn compose(certificates: Vec<CoreCertificate>) -> Result<Self, Box<VerificationOutcome>> {
        let unit = certificates
            .first()
            .expect("nonempty checked composition")
            .unit()
            .clone();
        if certificates
            .iter()
            .any(|certificate| certificate.unit() != &unit)
        {
            return Err(Box::new(VerificationOutcome::unknown(
                "QP015",
                "plugins certify different translation-unit identities",
            )));
        }
        let mut premises = BTreeMap::<String, bool>::new();
        let mut claims = BTreeMap::<(String, FactPredicate), (String, bool)>::new();
        let mut coverage = BTreeSet::new();
        let mut obligation_bounds = BTreeMap::<String, TaskBounds>::new();
        let mut bounds: Option<TaskBounds> = None;
        let mut every_component_carries_a_task = true;
        let mut plugin_ids = BTreeSet::new();
        let mut capabilities = BTreeSet::new();
        let mut task_digests = BTreeSet::new();
        for certificate in &certificates {
            for (name, value) in &certificate.evidence.assumptions_used {
                if premises
                    .insert(name.clone(), *value)
                    .is_some_and(|old| old != *value)
                {
                    return Err(Box::new(VerificationOutcome::unknown(
                        "QP012",
                        "plugins use conflicting assumptions",
                    )));
                }
            }
            for claim in &certificate.evidence.claims {
                let key = (claim.subject.clone(), claim.predicate);
                if claims
                    .insert(key, (claim.fact_id.clone(), claim.holds))
                    .is_some_and(|old| old.1 != claim.holds)
                {
                    return Err(Box::new(VerificationOutcome::unknown(
                        "QP013",
                        "plugins make conflicting claims",
                    )));
                }
            }
            for id in certificate.coverage() {
                coverage.insert(id.clone());
            }
            for (id, bound) in certificate.obligation_bounds() {
                if obligation_bounds
                    .insert(id.clone(), bound.clone())
                    .is_some_and(|previous| previous != *bound)
                {
                    return Err(Box::new(VerificationOutcome::unknown(
                        "QP013",
                        format!("obligation `{id}` is discharged under incomparable bounds"),
                    )));
                }
            }
            match certificate.task() {
                Some(task) => {
                    bounds = Some(bounds.map_or_else(
                        || task.bounds.clone(),
                        |previous| tighter_bounds(&previous, &task.bounds),
                    ));
                }
                None => every_component_carries_a_task = false,
            }
            plugin_ids.insert(certificate.plugin_id().clone());
            capabilities.insert(certificate.capability());
            if let Some(digest) = certificate.task_digest() {
                task_digests.insert(digest.to_owned());
            }
        }
        Ok(Self {
            certificates,
            unit,
            premises,
            coverage,
            obligation_bounds,
            // A component that carries no task states no bounds, so the
            // composite cannot state one either.
            bounds: bounds.filter(|_| every_component_carries_a_task),
            plugin_ids,
            capabilities,
            task_digests,
        })
    }

    pub fn certificates(&self) -> &[CoreCertificate] {
        &self.certificates
    }

    /// The single translation-unit identity every component certifies.
    pub const fn unit(&self) -> &UnitIdentity {
        &self.unit
    }

    /// The union of every component's premises: the assumption set the
    /// composite rests on, in full.
    pub const fn premises(&self) -> &BTreeMap<String, bool> {
        &self.premises
    }

    /// The union of every component's discharged obligation identifiers.
    pub const fn coverage(&self) -> &BTreeSet<String> {
        &self.coverage
    }

    /// Per-obligation bounds; conflicting entries never reach this map.
    pub const fn obligation_bounds(&self) -> &BTreeMap<String, TaskBounds> {
        &self.obligation_bounds
    }

    /// The tightest task bounds envelope every component was checked within,
    /// or `None` when some component carries no task to state one.
    pub const fn bounds(&self) -> Option<&TaskBounds> {
        self.bounds.as_ref()
    }

    pub const fn plugin_ids(&self) -> &BTreeSet<PluginId> {
        &self.plugin_ids
    }

    pub const fn capabilities(&self) -> &BTreeSet<VerifierCapability> {
        &self.capabilities
    }

    pub const fn task_digests(&self) -> &BTreeSet<String> {
        &self.task_digests
    }

    /// A component certificate that *is* the composite, which exists only when
    /// every component is the same certificate (one provider, or the same
    /// evidence submitted more than once). With two distinct components there
    /// is no such certificate and this is `None`: the published result then
    /// carries the composite alone, so no caller can read one component's
    /// coverage, bounds or provenance as the merge's.
    pub fn representative(&self) -> Option<&CoreCertificate> {
        let first = self.certificates.first()?;
        self.certificates
            .iter()
            .all(|certificate| certificate == first)
            .then_some(first)
    }
}

/// Merges several provider outcomes into one composed outcome.
///
/// Composition is n-ary over components and **associative under its own
/// output** (E03-B-O2): a composed outcome may be fed back in, and any
/// grouping of the same component sequence publishes the same composite. The
/// refusal rules are decided once, over the fully flattened component
/// sequence, so no grouping can hide a conflict from them.
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
    // E03-B-O2. Composition is n-ary over *components*, and a composed outcome
    // is a legitimate component: it is taken apart into the components it was
    // built from, so composing a composite is composing its parts. Grouping is
    // therefore invisible to the published merge — `compose([compose([a, b]),
    // c])` and `compose([a, b, c])` build the same `CompositeCertificate`,
    // because both reduce to the same component sequence before a single
    // `CompositeCertificate::compose` decides every conflict over all of them.
    //
    // This admits nothing new. A `CompositeCertificate` has private fields, a
    // private constructor and no deserialization, so the only way to hold one
    // is to have obtained it from a composition of certificates the firewall
    // already minted; flattening re-decides every conflict rule over exactly
    // those certificates. A `Safe` outcome carrying neither a certificate nor a
    // composite is still `QP014`.
    let mut certificates = Vec::new();
    for outcome in outcomes {
        match (outcome.composite, outcome.certificate) {
            (Some(composite), _) => certificates.extend_from_slice(composite.certificates()),
            (None, Some(certificate)) => certificates.push(certificate),
            (None, None) => {
                return VerificationOutcome::unknown(
                    "QP014",
                    "a safe plugin result is missing its core certificate",
                );
            }
        }
    }
    // E03-B1. Composition cannot fabricate a provider certificate, and it no
    // longer publishes one component's as the merge. It publishes the
    // composite: the union of the components' premises and coverage, the
    // per-obligation bounds they were actually checked under, the tightest
    // task-bounds envelope, and the full provenance sets. `certificate` keeps
    // a component only when every component is that same certificate, so no
    // caller reading it can mistake one provider's coverage, bounds or
    // attribution for the merge's.
    //
    // The premise union supersedes the earlier refusal of unequal-but-
    // compatible premise sets (E03 repair 2, E03-O1). That refusal existed
    // only because the published representative would have understated its
    // assumptions; publishing the union states them in full instead, and a
    // premise two components bind to different values is still `QP012`.
    match CompositeCertificate::compose(certificates) {
        Err(refusal) => *refusal,
        Ok(composite) => VerificationOutcome {
            verdict: ProposedVerdict::Safe,
            diagnostics: Vec::new(),
            certificate: composite.representative().cloned(),
            composite: Some(composite),
        },
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
    let composed = compose_verification(outcomes);
    if composed.verdict != ProposedVerdict::Safe {
        return composed;
    }
    // The denominator comparison runs against the composite's own coverage
    // union, which is the same set `compose_certificates` publishes. Bounds
    // conflicts were already refused while the composite was built.
    let covered = composed
        .composite
        .as_ref()
        .expect("a safe composition publishes its composite")
        .coverage()
        .clone();
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
    match result.composite {
        Some(composite) if result.verdict == ProposedVerdict::Safe => Ok(composite),
        _ => Err(Box::new(result)),
    }
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
                composite: None,
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

pub mod executable_identity;
pub use executable_identity::{IdentityRefusal, VerifiedExecutable};

pub mod package;
pub use package::verify_executable_package;
