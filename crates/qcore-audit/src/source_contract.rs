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
use qcore_verification::source_effect::{BooleanFieldSourceTask, semantic_profile_for};
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
    /// Package directories, relative to the configuration.
    #[serde(default)]
    packages: Option<Vec<PathBuf>>,
    /// Card W44-QUIRIN-AUDIT-PROVIDERS: a `providers/package.py build` output
    /// directory instead of `packages`. Each binding's provider id names its
    /// package, `<package_root>/<provider id>`, which is then verified exactly
    /// as a listed package is. Exactly one of the two is required.
    #[serde(default)]
    package_root: Option<PathBuf>,
    bindings: Vec<BindingConfiguration>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingConfiguration {
    library: String,
    provider: String,
    contract: PolicySelection,
    #[serde(default)]
    frame_helpers: Vec<FrameHelperConfiguration>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameHelperConfiguration {
    source: PathBuf,
    function: String,
    record: String,
    contract: PolicySelection,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PolicySelection {
    Path(PathBuf),
    Bundle(BundleSelection),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleSelection {
    manifest: PathBuf,
    sha256: String,
    owner: String,
    path: String,
}

/// Declared helper selection and immutable policy bytes; not source evidence.
#[derive(Debug, serde::Serialize)]
pub struct FrameHelperPolicy {
    source: PathBuf,
    function: String,
    record: String,
    contract: String,
}
impl FrameHelperPolicy {
    pub fn source(&self) -> &Path {
        &self.source
    }
    pub fn function(&self) -> &str {
        &self.function
    }
    pub fn record(&self) -> &str {
        &self.record
    }
    pub fn contract(&self) -> &str {
        &self.contract
    }
}

#[derive(Debug)]
struct Binding {
    provider: PluginId,
    contract: String,
    frame_helpers: Vec<FrameHelperPolicy>,
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

pub(crate) fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, SourceContractError> {
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

/// The package directories a configuration names: its `packages` list, or,
/// under `package_root`, one directory per distinct binding provider id
/// (`providers/package.py build` names each package directory by its id).
fn package_inventory(
    packages: Option<Vec<PathBuf>>,
    package_root: Option<&Path>,
    bindings: &[BindingConfiguration],
) -> Result<Vec<PathBuf>, SourceContractError> {
    let refuse = |message: &str| Err(SourceContractError::Configuration(message.into()));
    match (packages, package_root) {
        (Some(packages), None) => Ok(packages),
        (None, Some(root)) => {
            if root.as_os_str().is_empty() {
                return refuse("empty package root");
            }
            let mut ids = BTreeSet::new();
            for binding in bindings {
                let id = PluginId::new(binding.provider.clone())
                    .map_err(|e| SourceContractError::Configuration(e.to_string()))?;
                let mut parts = Path::new(id.as_str()).components();
                if !matches!(
                    (parts.next(), parts.next()),
                    (Some(std::path::Component::Normal(_)), None)
                ) {
                    return Err(SourceContractError::Configuration(format!(
                        "provider id `{}` cannot name a package directory",
                        id.as_str()
                    )));
                }
                ids.insert(id.as_str().to_owned());
            }
            Ok(ids.into_iter().map(|id| root.join(id)).collect())
        }
        (Some(_), Some(_)) => refuse("give either `packages` or `package_root`, not both"),
        (None, None) => refuse("a package inventory is required: `packages` or `package_root`"),
    }
}

fn c_identifier(value: &str) -> bool {
    let mut chars = value.bytes();
    value.len() <= 128
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn snapshot_policy(
    base: &Path,
    selection: &PolicySelection,
    version: u32,
    retained: &mut u64,
) -> Result<String, SourceContractError> {
    const MAX_POLICY_BYTES: u64 = 32 * 1024 * 1024;
    let remaining = MAX_POLICY_BYTES.checked_sub(*retained).ok_or_else(|| {
        SourceContractError::Configuration("policy snapshot byte budget exhausted".into())
    })?;
    if remaining == 0 {
        return Err(SourceContractError::Configuration(
            "policy snapshots exceed the 32 MiB aggregate byte budget".into(),
        ));
    }
    let bytes = match selection {
        PolicySelection::Path(path) => {
            if path.as_os_str().is_empty() {
                return Err(SourceContractError::Configuration(
                    "empty policy path".into(),
                ));
            }
            read_bounded(&crate::resolve(base, path), 8_388_608.min(remaining))?
        }
        PolicySelection::Bundle(selected) => {
            if version != 2 || selected.manifest.as_os_str().is_empty() {
                return Err(SourceContractError::Configuration(
                    "bundle selection requires version 2 and a manifest path".into(),
                ));
            }
            let bundle = qcore_contract::bundle::ContractBundle::load(
                &crate::resolve(base, &selected.manifest),
                &selected.sha256,
                &selected.owner,
            )
            .map_err(SourceContractError::Configuration)?;
            let source = bundle
                .contract(&selected.path)
                .map_err(SourceContractError::Configuration)?;
            if source.len() as u64 > 8_388_608.min(remaining) {
                return Err(SourceContractError::Configuration(
                    "policy snapshot byte budget exhausted".into(),
                ));
            }
            source.as_bytes().to_vec()
        }
    };
    *retained += bytes.len() as u64;
    let policy = String::from_utf8(bytes)
        .map_err(|error| SourceContractError::Configuration(error.to_string()))?;
    if policy.trim().is_empty() {
        return Err(SourceContractError::Configuration(
            "empty contract source".into(),
        ));
    }
    Ok(policy)
}

impl SourceContractPolicies {
    /// Selection metadata from the immutable configuration, not execution evidence.
    pub fn frame_helper_policy_inventory(&self) -> BTreeMap<&str, &[FrameHelperPolicy]> {
        self.bindings
            .iter()
            .filter(|(_, binding)| !binding.frame_helpers.is_empty())
            .map(|(library, binding)| (library.as_str(), binding.frame_helpers.as_slice()))
            .collect()
    }

    /// Helpers explicitly selected for this library; no declaration means none.
    pub fn frame_helpers_for_library(&self, library: &str) -> &[FrameHelperPolicy] {
        self.bindings
            .get(library)
            .map(|binding| binding.frame_helpers.as_slice())
            .unwrap_or(&[])
    }

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
        let packages = package_inventory(
            config.packages,
            config.package_root.as_deref(),
            &config.bindings,
        )?;
        validate_package_paths(&packages)?;
        if !matches!(config.schema_version, 1 | 2)
            || config.bindings.is_empty()
            || config.bindings.len() > 4096
        {
            return Err(SourceContractError::Configuration(
                "unsupported version, empty binding inventory, or excessive inventory".into(),
            ));
        }
        let mut bindings = BTreeMap::new();
        // Bound total retained snapshots as well as each individual file.
        // Repeated references still retain distinct bytes and consume budget.
        let mut retained_bytes = 0u64;
        for binding in config.bindings {
            if binding.library.trim().is_empty() || binding.library.trim() != binding.library {
                return Err(SourceContractError::Configuration(
                    "binding requires an exact nonempty library and contract path".into(),
                ));
            }
            let provider = PluginId::new(binding.provider)
                .map_err(|e| SourceContractError::Configuration(e.to_string()))?;
            let contract = snapshot_policy(
                &base,
                &binding.contract,
                config.schema_version,
                &mut retained_bytes,
            )?;
            if binding.frame_helpers.len() > 256 {
                return Err(SourceContractError::Configuration(
                    "more than 256 frame helpers in one binding".into(),
                ));
            }
            let mut helper_names = BTreeSet::new();
            let mut frame_helpers = Vec::new();
            for helper in binding.frame_helpers {
                if helper.source.as_os_str().is_empty()
                    || !helper
                        .source
                        .components()
                        .all(|part| matches!(part, std::path::Component::Normal(_)))
                    || !c_identifier(&helper.function)
                    || !c_identifier(&helper.record)
                    || !helper_names.insert(helper.function.clone())
                {
                    return Err(SourceContractError::Configuration("frame helper requires a unique C function, C record, and contained relative source path".into()));
                }
                let contract = snapshot_policy(
                    &base,
                    &helper.contract,
                    config.schema_version,
                    &mut retained_bytes,
                )?;
                frame_helpers.push(FrameHelperPolicy {
                    source: helper.source,
                    function: helper.function,
                    record: helper.record,
                    contract,
                });
            }
            if bindings
                .insert(
                    binding.library,
                    Binding {
                        provider,
                        contract,
                        frame_helpers,
                    },
                )
                .is_some()
            {
                return Err(SourceContractError::Configuration(
                    "duplicate library binding".into(),
                ));
            }
        }
        Ok(Self {
            base,
            packages,
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
    pub fn bound_libraries(&self) -> impl Iterator<Item = &str> {
        self.bindings.keys().map(String::as_str)
    }

    /// Immutable loaded selection and package identities; this is provenance,
    /// not evidence that any configured function has been checked.
    pub fn audit_inventory(&self) -> serde_json::Value {
        serde_json::json!({
            "packages": self.registry.manifests(),
            "bindings": self.bindings.iter().map(|(library, binding)| serde_json::json!({
                "library": library, "provider": binding.provider,
                "contract": binding.contract, "frame_helpers": binding.frame_helpers,
            })).collect::<Vec<_>>(),
        })
    }

    /// Selection metadata from the immutable configuration, not execution evidence.
    pub fn frame_helper_policy_inventory(&self) -> BTreeMap<&str, &[FrameHelperPolicy]> {
        self.bindings
            .iter()
            .filter(|(_, binding)| !binding.frame_helpers.is_empty())
            .map(|(library, binding)| (library.as_str(), binding.frame_helpers.as_slice()))
            .collect()
    }

    /// Helpers explicitly selected for this library; no declaration means none.
    pub fn frame_helpers_for_library(&self, library: &str) -> &[FrameHelperPolicy] {
        self.bindings
            .get(library)
            .map(|binding| binding.frame_helpers.as_slice())
            .unwrap_or(&[])
    }

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
            if !matches!(
                manifest.certificate_checker.as_str(),
                "qcore.boolean_field_contract.v1" | "qcore.boolean_field_contract.v2"
            ) || !manifest.protocol_versions.contains(&1)
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
            profile: semantic_profile_for(summary).into(),
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
        if summary.result_kind == qcore_schemas::chunk_summary::BooleanFieldResult::Unit
            && manifest.certificate_checker != "qcore.boolean_field_contract.v2"
        {
            return Err(SourceContractError::Configuration(
                "unit-result source requires Boolean witness checker v2".into(),
            ));
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

/// One selected helper with sealed source evidence and checked policy predicates.
/// Concrete live/exclusive access and cross-unit C ABI equality remain premises.
#[derive(Debug)]
pub struct CheckedFrameHelper {
    pub summary: qcore_c_gateway::boolean_source::ChunkFrameSummary,
    pub checked_postconditions: usize,
}

/// Import only explicitly selected helper TUs through their actual compile entries.
/// Neither matching a filename nor parsing a policy creates source authority.
pub fn import_configured_frame_helpers(
    helpers: &[FrameHelperPolicy],
    source_root: &Path,
    entries: &[crate::compile_db::CompileEntry],
    revision: &str,
    profile: &qcore_contract::profile_loader::LoadedProfile,
    library: &str,
    source_rules: &str,
) -> Result<Vec<CheckedFrameHelper>, SourceContractError> {
    import_frame_helpers(
        helpers,
        source_root,
        entries,
        revision,
        profile,
        library,
        source_rules,
        crate::compile_db::compiler_options,
    )
}

/// Explicit historical option projection for pinned compatibility adapters.
/// Unrepresented compiler flags remain an outer-audit assumption. General
/// callers should use `import_configured_frame_helpers`, which rejects them.
pub fn import_configured_frame_helpers_compatibility(
    helpers: &[FrameHelperPolicy],
    source_root: &Path,
    entries: &[crate::compile_db::CompileEntry],
    revision: &str,
    profile: &qcore_contract::profile_loader::LoadedProfile,
    library: &str,
    source_rules: &str,
) -> Result<Vec<CheckedFrameHelper>, SourceContractError> {
    import_frame_helpers(
        helpers,
        source_root,
        entries,
        revision,
        profile,
        library,
        source_rules,
        crate::compile_db::compiler_options_compatibility,
    )
}

#[allow(clippy::too_many_arguments)]
fn import_frame_helpers(
    helpers: &[FrameHelperPolicy],
    source_root: &Path,
    entries: &[crate::compile_db::CompileEntry],
    revision: &str,
    profile: &qcore_contract::profile_loader::LoadedProfile,
    library: &str,
    source_rules: &str,
    options_for: fn(
        &crate::compile_db::CompileEntry,
    ) -> Result<crate::compile_db::CompilerOptions, String>,
) -> Result<Vec<CheckedFrameHelper>, SourceContractError> {
    use qcore_c_gateway::legacy_c::{C11TranslationUnit, LegacyFunctionScope};
    if helpers.is_empty() {
        return Ok(Vec::new());
    }
    let root = source_root
        .canonicalize()
        .map_err(|error| SourceContractError::Derivation(error.to_string()))?;
    let mut result = Vec::new();
    let mut retained_source_bytes = 0usize;
    for selected in helpers {
        let path = root
            .join(selected.source())
            .canonicalize()
            .map_err(|error| SourceContractError::Derivation(error.to_string()))?;
        if !path.starts_with(&root) {
            return Err(SourceContractError::Derivation(
                "helper source escapes its selected library root".into(),
            ));
        }
        let matches = entries
            .iter()
            .filter(|entry| {
                crate::resolve(&entry.directory, &entry.file)
                    .canonicalize()
                    .is_ok_and(|candidate| candidate == path)
            })
            .collect::<Vec<_>>();
        let [entry] = matches.as_slice() else {
            return Err(SourceContractError::Derivation(format!(
                "helper {} requires exactly one selected compile entry",
                selected.function()
            )));
        };
        let source = String::from_utf8(read_bounded(&path, 8_388_608)?)
            .map_err(|error| SourceContractError::Derivation(error.to_string()))?;
        if !entry.directory.is_absolute() {
            return Err(SourceContractError::Derivation(
                "helper compile directory must be absolute".into(),
            ));
        }
        let mut entry = (*entry).clone();
        entry.file = path.clone();
        let options = options_for(&entry).map_err(SourceContractError::Derivation)?;
        let unit = C11TranslationUnit {
            source: &source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: options.input,
        };
        let summary = qcore_c_gateway::boolean_source::import_chunk_frame_helper_with_profile(
            &unit,
            revision,
            selected.function(),
            profile,
            library,
            selected.record(),
            source_rules,
        )
        .map_err(SourceContractError::Derivation)?;
        retained_source_bytes = retained_source_bytes
            .checked_add(summary.source.len())
            .ok_or_else(|| {
                SourceContractError::Derivation("helper source byte budget overflow".into())
            })?;
        if retained_source_bytes > 32 * 1024 * 1024 {
            return Err(SourceContractError::Derivation(
                "retained helper source exceeds 32 MiB".into(),
            ));
        }
        let checked_postconditions =
            qcore_contract::chunk_contract::evaluate_boolean_frame_contract(
                &summary,
                selected.contract(),
                summary.record(),
                summary.protected_fields(),
            )
            .map_err(SourceContractError::Evaluation)?;
        result.push(CheckedFrameHelper {
            summary,
            checked_postconditions,
        });
    }
    Ok(result)
}
