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

#[cfg(unix)]
impl ProcessProvider {
    fn verify_process<T: for<'de> Deserialize<'de>>(
        &self,
        frame: Vec<u8>,
    ) -> Result<T, ProviderFailure> {
        use std::os::unix::process::CommandExt;
        let PluginEntrypoint::Executable { path } = &self.manifest.entrypoint else {
            return Err(ProviderFailure::Protocol("missing executable".into()));
        };
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(self.manifest.limits.timeout_ms))
            .ok_or_else(|| {
                ProviderFailure::Protocol("provider timeout exceeds clock range".into())
            })?;
        let limit = self.manifest.limits.output_bytes;
        let wire_limit = limit.checked_add(4).ok_or(ProviderFailure::OutputLimit)?;
        let executable = Path::new(&self.package_root).join(path);
        let mut child = ProviderChild(
            Command::new(&executable)
                .current_dir(&self.package_root)
                .env_clear()
                .process_group(0)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
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
        let mut output = Vec::new();
        let mut error_output = Vec::new();
        let (mut output_done, mut error_done) = (false, false);
        let mut status = None;
        let mut input_failure = None;
        loop {
            // Observe direct-child exit before deciding which bound failed.
            // An exited provider can leave descendant-held output pipes even
            // when its input reader vanished before our first write.
            if status.is_none() {
                status = child
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
                output_done = read_available(&mut stdout, &mut output, wire_limit)?;
            }
            if !error_done {
                error_done = read_available(&mut stderr, &mut error_output, 64 * 1024)?;
            }
            if let Some(status) = status
                && output_done
                && error_done
                && stdin.is_none()
            {
                if let Some(error) = input_failure {
                    return Err(error);
                }
                if !status.success() {
                    return Err(ProviderFailure::Crashed(status.code())
                        .with_detail(&String::from_utf8_lossy(&error_output)));
                }
                return decode_protocol_frame(&output, limit);
            }
            thread::sleep(Duration::from_millis(1));
        }
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

pub mod package;
pub use package::verify_executable_package;
