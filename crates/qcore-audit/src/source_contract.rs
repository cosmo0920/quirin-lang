//! Configured process proposals for sealed source-derived Boolean contracts.
//!
//! Configuration selects packages and policy bytes; it cannot construct source
//! authority. Admission always uses the existing trusted source-task checker.
//! Package verification is a filesystem snapshot, not immutable execution or
//! a signature check. Callers must protect installed files from concurrent edits.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use qcore_c_gateway::boolean_source::ChunkSourceSummary;
use qcore_contract::chunk_contract::ChunkContractEvaluation;
use qcore_schemas::{
    PluginDiagnostic, PluginId, ProposedVerdict, TargetProfile, VerifierCapability,
};
use qcore_verification::source_effect::{BOOLEAN_FIELD_SEMANTIC_PROFILE, BooleanFieldSourceTask};
use qcore_verifier_host::{VerifierProvider, VerifierRegistry, verify_executable_package};
use serde::Deserialize;

pub type CheckedPluginCertificate = qcore_verifier_host::CoreCertificate;

/// Only returned after package dispatch and independent trusted replay succeed.
#[derive(Debug)]
pub struct SourceContractVerification {
    pub certificate: CheckedPluginCertificate,
    pub evaluation: ChunkContractEvaluation,
}

#[derive(Debug)]
pub enum SourceContractError {
    Configuration(String),
    Package(String),
    MissingBinding {
        library: String,
    },
    IncompatibleTarget {
        provider: String,
    },
    Derivation(String),
    Admission {
        provider: String,
        diagnostics: Vec<PluginDiagnostic>,
    },
    Evaluation(String),
}

impl fmt::Display for SourceContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(reason) => write!(f, "source-contract configuration: {reason}"),
            Self::Package(reason) => write!(f, "source-contract package: {reason}"),
            Self::MissingBinding { library } => {
                write!(f, "no source-contract binding for `{library}`")
            }
            Self::IncompatibleTarget { provider } => write!(
                f,
                "source-contract provider `{provider}` does not support the sealed import target"
            ),
            Self::Derivation(reason) => write!(f, "source-contract task derivation: {reason}"),
            Self::Admission {
                provider,
                diagnostics,
            } => write!(
                f,
                "source-contract evidence from `{provider}` was not admitted: {diagnostics:?}"
            ),
            Self::Evaluation(reason) => write!(f, "source-contract evaluation: {reason}"),
        }
    }
}

impl std::error::Error for SourceContractError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    schema_version: u32,
    packages: Vec<PathBuf>,
    bindings: Vec<BindingConfiguration>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingConfiguration {
    library: String,
    provider: String,
    contract: PathBuf,
}

#[derive(Debug)]
struct Binding {
    provider: PluginId,
    contract: String,
}

pub struct SourceContractRegistry {
    registry: VerifierRegistry,
    bindings: BTreeMap<String, Binding>,
}

impl fmt::Debug for SourceContractRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceContractRegistry")
            .field("libraries", &self.bindings.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, SourceContractError> {
    let failed = |error: std::io::Error| {
        SourceContractError::Configuration(format!("{}: {error}", path.display()))
    };
    let file = File::open(path).map_err(failed)?;
    if !file.metadata().map_err(failed)?.is_file() {
        return Err(SourceContractError::Configuration(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    if bytes.len() as u64 > maximum {
        return Err(SourceContractError::Configuration(format!(
            "{} exceeds {maximum} bytes",
            path.display()
        )));
    }
    Ok(bytes)
}

/// Immutable policy bytes loaded independently of process availability. This
/// is configured policy data, never a source or provider certificate.
#[derive(Debug)]
pub struct SourceContractPolicies {
    base: PathBuf,
    packages: Vec<PathBuf>,
    bindings: BTreeMap<String, Binding>,
}

fn validate_package_paths(packages: &[PathBuf]) -> Result<(), SourceContractError> {
    if packages.is_empty()
        || packages.len() > 256
        || packages.iter().any(|path| path.as_os_str().is_empty())
        || packages.iter().collect::<BTreeSet<_>>().len() != packages.len()
    {
        return Err(SourceContractError::Configuration(
            "package inventory requires 1..256 distinct nonempty paths".into(),
        ));
    }
    Ok(())
}

impl SourceContractPolicies {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SourceContractError> {
        let path = path
            .as_ref()
            .canonicalize()
            .map_err(|e| SourceContractError::Configuration(e.to_string()))?;
        let base = path
            .parent()
            .ok_or_else(|| {
                SourceContractError::Configuration("configuration has no parent".into())
            })?
            .to_path_buf();
        let config: Configuration = serde_json::from_slice(&read_bounded(&path, 1_048_576)?)
            .map_err(|e| SourceContractError::Configuration(e.to_string()))?;
        validate_package_paths(&config.packages)?;
        if config.schema_version != 1 || config.bindings.is_empty() || config.bindings.len() > 4096
        {
            return Err(SourceContractError::Configuration(
                "unsupported version, empty binding inventory, or excessive inventory".into(),
            ));
        }
        let mut bindings = BTreeMap::new();
        // Bound total retained snapshots as well as each individual file.
        // Repeated references still retain distinct bytes and consume budget.
        const MAX_POLICY_BYTES: u64 = 32 * 1024 * 1024;
        let mut retained_bytes = 0u64;
        for binding in config.bindings {
            if binding.library.trim().is_empty()
                || binding.library.trim() != binding.library
                || binding.contract.as_os_str().is_empty()
            {
                return Err(SourceContractError::Configuration(
                    "binding requires an exact nonempty library and contract path".into(),
                ));
            }
            let provider = PluginId::new(binding.provider)
                .map_err(|e| SourceContractError::Configuration(e.to_string()))?;
            let remaining = MAX_POLICY_BYTES
                .checked_sub(retained_bytes)
                .ok_or_else(|| {
                    SourceContractError::Configuration(
                        "policy snapshot byte budget exhausted".into(),
                    )
                })?;
            if remaining == 0 {
                return Err(SourceContractError::Configuration(
                    "policy snapshots exceed the 32 MiB aggregate byte budget".into(),
                ));
            }
            let bytes = read_bounded(
                &crate::resolve(&base, &binding.contract),
                8_388_608.min(remaining),
            )?;
            retained_bytes = retained_bytes
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| {
                    SourceContractError::Configuration(
                        "policy snapshot byte budget overflow".into(),
                    )
                })?;
            let contract = String::from_utf8(bytes)
                .map_err(|e| SourceContractError::Configuration(e.to_string()))?;
            if contract.trim().is_empty() {
                return Err(SourceContractError::Configuration(
                    "empty contract source".into(),
                ));
            }
            if bindings
                .insert(binding.library, Binding { provider, contract })
                .is_some()
            {
                return Err(SourceContractError::Configuration(
                    "duplicate library binding".into(),
                ));
            }
        }
        Ok(Self {
            base,
            packages: config.packages,
            bindings,
        })
    }

    pub fn contract_for_library(&self, library: &str) -> Result<&str, SourceContractError> {
        self.bindings
            .get(library)
            .map(|binding| binding.contract.as_str())
            .ok_or_else(|| SourceContractError::MissingBinding {
                library: library.into(),
            })
    }
}

impl SourceContractRegistry {
    /// The same immutable policy bytes used by process replay, for adapters
    /// retaining a direct compatibility check or policy report.
    pub fn contract_for_library(&self, library: &str) -> Result<&str, SourceContractError> {
        self.bindings
            .get(library)
            .map(|binding| binding.contract.as_str())
            .ok_or_else(|| SourceContractError::MissingBinding {
                library: library.into(),
            })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, SourceContractError> {
        Self::load_inner(path.as_ref(), None)
    }

    /// Replace only package paths, retaining all configured library/policy
    /// bindings. Relative replacements also resolve against the config parent.
    pub fn load_with_packages(
        path: impl AsRef<Path>,
        packages: &[PathBuf],
    ) -> Result<Self, SourceContractError> {
        Self::load_inner(path.as_ref(), Some(packages))
    }

    fn load_inner(
        path: &Path,
        replacement: Option<&[PathBuf]>,
    ) -> Result<Self, SourceContractError> {
        let policies = SourceContractPolicies::load(path)?;
        let packages = replacement.unwrap_or(&policies.packages);
        validate_package_paths(packages)?;
        let base = &policies.base;
        let mut registry = VerifierRegistry::default();
        let mut seen_paths = BTreeSet::new();
        for package in packages {
            if package.as_os_str().is_empty() {
                return Err(SourceContractError::Configuration(
                    "empty package path".into(),
                ));
            }
            let package = crate::resolve(base, package)
                .canonicalize()
                .map_err(|e| SourceContractError::Package(e.to_string()))?;
            if !seen_paths.insert(package.clone()) {
                return Err(SourceContractError::Configuration(
                    "duplicate package path".into(),
                ));
            }
            let provider = verify_executable_package(package)
                .map_err(|e| SourceContractError::Package(e.to_string()))?;
            let manifest = provider.manifest();
            if manifest.certificate_checker != "qcore.boolean_field_contract.v1"
                || !manifest.protocol_versions.contains(&1)
                || !manifest.evidence_versions.contains(&2)
                || !manifest
                    .capabilities
                    .contains(&VerifierCapability::ResourceContracts)
            {
                return Err(SourceContractError::Package(format!(
                    "provider `{}` is incompatible with the Boolean source checker",
                    manifest.plugin_id.as_str()
                )));
            }
            registry
                .register(Arc::new(provider))
                .map_err(|e| SourceContractError::Package(e.to_string()))?;
        }
        for (library, binding) in &policies.bindings {
            let manifest = registry.manifest(&binding.provider).ok_or_else(|| {
                SourceContractError::Configuration(format!(
                    "binding provider `{}` is absent",
                    binding.provider.as_str()
                ))
            })?;
            if !manifest
                .targets
                .iter()
                .any(|target| &target.library == library)
            {
                return Err(SourceContractError::Configuration(format!(
                    "provider `{}` declares no target for `{library}`",
                    binding.provider.as_str()
                )));
            }
        }
        let bindings = policies.bindings;
        Ok(Self { registry, bindings })
    }

    pub fn verify(
        &self,
        summary: &ChunkSourceSummary,
        translation_unit: &str,
    ) -> Result<SourceContractVerification, SourceContractError> {
        let identity = summary.input_identity();
        let library = identity.model_library().ok_or_else(|| {
            SourceContractError::Derivation("sealed import lacks a model library".into())
        })?;
        let binding =
            self.bindings
                .get(library)
                .ok_or_else(|| SourceContractError::MissingBinding {
                    library: library.into(),
                })?;
        let target = TargetProfile {
            library: library.into(),
            revision: summary.source_revision().into(),
            profile: BOOLEAN_FIELD_SEMANTIC_PROFILE.into(),
            dialect: identity.dialect().into(),
            platform: std::env::consts::OS.into(),
            backend: "native".into(),
        };
        let manifest = self.registry.manifest(&binding.provider).ok_or_else(|| {
            SourceContractError::Configuration("loaded binding provider disappeared".into())
        })?;
        if !manifest.supports(VerifierCapability::ResourceContracts, &target) {
            return Err(SourceContractError::IncompatibleTarget {
                provider: binding.provider.as_str().into(),
            });
        }
        let replay =
            BooleanFieldSourceTask::derive(summary, translation_unit, &binding.contract, target)
                .map_err(SourceContractError::Derivation)?;
        let request = replay.request(binding.provider.clone());
        let outcome = self
            .registry
            .verify_v2(&request, replay.task(), Some(&replay));
        if outcome.verdict != ProposedVerdict::Safe || outcome.certificate.is_none() {
            return Err(SourceContractError::Admission {
                provider: binding.provider.as_str().into(),
                diagnostics: outcome.diagnostics,
            });
        }
        let certificate = outcome.certificate.expect("checked above");
        let evaluation = replay.evaluate().map_err(SourceContractError::Evaluation)?;
        Ok(SourceContractVerification {
            certificate,
            evaluation,
        })
    }
}
