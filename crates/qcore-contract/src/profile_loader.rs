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

//! Total-or-reject loader for source-set profile manifests.
//!
//! Implements the approved design `docs/design/W2_PROFILE_MANIFEST_SCHEMA.md`
//! (§§1–3) over the L0 types in [`qcore_schemas::profile`]. Layer L1, beside
//! the W1 resource-contract loader, and for the same reason: a profile is a
//! trusted assumption whose acceptance rules must be one auditable place.
//!
//! Every failure mode in the design's §3 table maps to a rejection here and
//! to `Unknown` at the call site — never a default profile, never a partial
//! import, never a skipped check:
//!
//! - F1 missing directory/`profile.json` → [`ProfileDiagnostic::ProfileNotFound`]
//! - F2/F4 unreadable, non-JSON, unknown field → [`ProfileDiagnostic::ProfileUnreadable`]
//! - F3 unknown `schema_version` → [`ProfileDiagnostic::UnsupportedSchemaVersion`]
//! - F5 broken internal reference → [`ProfileDiagnostic::ProfileInconsistent`]
//! - F6 needle without rationale → [`ProfileDiagnostic::NeedleWithoutRationale`]
//! - F7/F8 pin mismatch or dirty checkout → [`PinDiagnostic`]
//! - F9/F10 missing file or digest mismatch → [`FileAuditDiagnostic`]
//! - F18 provider manifest invalid → [`ProfileDiagnostic::ProviderManifestInvalid`]
//!
//! A profile that is 95% valid is 0% valid: load-time rejections are whole
//! profile, exactly like the W1 loader. Loading a profile grants nothing; it
//! only supplies the bounds an existing check already enforced in Rust.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use qcore_schemas::identity::ExternalSourceBundle;
use qcore_schemas::plugin::{PluginId, PluginManifest};
use qcore_schemas::profile::{
    LibraryProfile, ProfileManifest, SUPPORTED_PROFILE_MANIFEST_VERSIONS, SourcePin, WitnessCheck,
};

/// Structural load diagnostics (design §3, F1–F6 and F18). Numeric codes are
/// deliberately not assigned here: design Q4 defers numbering to W7, since
/// `Q700/Q701` collide today and `QP***` is provider-scoped.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProfileDiagnostic {
    /// F1: no profile directory or no `profile.json` for a requested set.
    ProfileNotFound { source_set: String },
    /// F2/F4: unreadable, not valid JSON, or structurally invalid content
    /// (including any unknown field, which `deny_unknown_fields` rejects).
    ProfileUnreadable { path: String, error: String },
    /// F3: the declared version is outside the supported set.
    UnsupportedSchemaVersion { found: u32, supported: Vec<u32> },
    /// F5: a broken internal reference or a duplicated name.
    ProfileInconsistent { detail: String },
    /// F6: a substring witness without usable needles or rationale.
    NeedleWithoutRationale { witness: String },
    /// F18: a referenced provider manifest is missing, invalid, or disagrees
    /// with the library's [`qcore_schemas::profile::ProviderBinding`].
    ProviderManifestInvalid { path: String, error: String },
}

impl fmt::Display for ProfileDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileNotFound { source_set } => {
                write!(
                    formatter,
                    "no profile manifest for source set `{source_set}`"
                )
            }
            Self::ProfileUnreadable { path, error } => {
                write!(formatter, "{path}: unreadable profile manifest: {error}")
            }
            Self::UnsupportedSchemaVersion { found, supported } => write!(
                formatter,
                "unsupported profile schema_version {found} (supported: {supported:?})"
            ),
            Self::ProfileInconsistent { detail } => {
                write!(formatter, "inconsistent profile manifest: {detail}")
            }
            Self::NeedleWithoutRationale { witness } => write!(
                formatter,
                "witness `{witness}` uses substring needles without needles or a rationale"
            ),
            Self::ProviderManifestInvalid { path, error } => {
                write!(formatter, "{path}: invalid provider manifest: {error}")
            }
        }
    }
}

impl std::error::Error for ProfileDiagnostic {}

/// F7/F8: pinned-checkout verification outcomes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PinDiagnostic {
    /// `git` could not be run, or reported failure. Fail-closed: never
    /// interpreted as "the checkout is fine".
    GitUnavailable { pin: String },
    /// F7: the checkout's `HEAD` is not the pinned commit.
    SourceRevisionMismatch {
        pin: String,
        expected: String,
        actual: String,
    },
    /// F8: tracked modifications exist under the pin's clean paths.
    DirtyCheckout { pin: String, paths: Vec<String> },
}

impl fmt::Display for PinDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitUnavailable { pin } => {
                write!(formatter, "could not inspect the `{pin}` checkout with git")
            }
            Self::SourceRevisionMismatch {
                pin,
                expected,
                actual,
            } => write!(
                formatter,
                "pin `{pin}`: expected {expected}, found {actual}"
            ),
            Self::DirtyCheckout { pin, paths } => write!(
                formatter,
                "pin `{pin}`: tracked modifications under {paths:?}"
            ),
        }
    }
}

impl std::error::Error for PinDiagnostic {}

/// F9/F10: audited-file inventory outcomes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileAuditDiagnostic {
    /// F9: an audited file is absent from the checkout.
    MissingFile { path: String },
    /// F10: an audited file's bytes do not hash to the declared digest.
    DigestMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for FileAuditDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFile { path } => write!(formatter, "{path}: audited file is missing"),
            Self::DigestMismatch {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "{path}: expected sha256 {expected}, found {actual}"
            ),
        }
    }
}

impl std::error::Error for FileAuditDiagnostic {}

/// A validated profile together with the identity of the exact bytes it was
/// loaded from. The digest and version are what a certificate must record as
/// assumed inputs (design Q7 / plan invariant 6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedProfile {
    manifest: ProfileManifest,
    directory: PathBuf,
    manifest_text: String,
    manifest_digest: String,
    providers: Vec<PluginManifest>,
    provider_directories: BTreeMap<PluginId, PathBuf>,
}

impl LoadedProfile {
    #[must_use]
    pub const fn manifest(&self) -> &ProfileManifest {
        &self.manifest
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The exact manifest text that was accepted, retained for replay.
    #[must_use]
    pub fn manifest_text(&self) -> &str {
        &self.manifest_text
    }

    /// Lowercase hex SHA-256 of the accepted `profile.json` bytes.
    #[must_use]
    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }

    /// Provider manifests bound to this source set, in sorted plugin-id order
    /// (deterministic registry ordering, design §1.2).
    #[must_use]
    pub fn providers(&self) -> &[PluginManifest] {
        &self.providers
    }

    /// Directory relative to which a validated provider executable is resolved.
    #[must_use]
    pub fn provider_directory(&self, id: &PluginId) -> Option<&Path> {
        self.provider_directories.get(id).map(PathBuf::as_path)
    }

    /// The library entry, or `None`. An unmatched library is Unknown at the
    /// call site; this never returns a default.
    #[must_use]
    pub fn library(&self, name: &str) -> Option<&LibraryProfile> {
        self.manifest.library(name)
    }

    /// The pin a library is bound to. Load-time validation guarantees the
    /// reference resolves for every declared library.
    #[must_use]
    pub fn pin_for(&self, library: &str) -> Option<&SourcePin> {
        self.manifest.pin_for(library)
    }

    /// Path of the resource-contract manifest this profile references,
    /// resolved against the profile directory, or `None` when the profile
    /// binds no manifest (resource projection then stays Unknown).
    #[must_use]
    pub fn resource_contract_manifest_path(&self) -> Option<PathBuf> {
        self.manifest
            .resource_contract_manifest
            .as_ref()
            .map(|relative| self.directory.join(relative))
    }

    /// The union of the named libraries' `operation_summaries`, rejecting any
    /// key two libraries map to different summary names. Total-or-reject: a
    /// conflict is a manifest defect, never a silent pick.
    pub fn merged_operation_summaries(
        &self,
        libraries: &[&str],
    ) -> Result<BTreeMap<qcore_schemas::profile::OperationKind, String>, ProfileDiagnostic> {
        let mut merged: BTreeMap<qcore_schemas::profile::OperationKind, String> = BTreeMap::new();
        for name in libraries {
            let Some(library) = self.library(name) else {
                return Err(ProfileDiagnostic::ProfileInconsistent {
                    detail: format!(
                        "no library `{name}` in source set `{}`",
                        self.manifest.source_set
                    ),
                });
            };
            for (kind, summary) in &library.operation_summaries {
                if let Some(existing) = merged.get(kind)
                    && existing != summary
                {
                    return Err(ProfileDiagnostic::ProfileInconsistent {
                        detail: format!(
                            "operation {kind:?} maps to both `{existing}` and `{summary}`"
                        ),
                    });
                }
                merged.insert(*kind, summary.clone());
            }
        }
        Ok(merged)
    }

    /// The assumption record a report or certificate must carry alongside any
    /// profile-derived result (design Q7): which source set, at which schema
    /// version and digest, with which pinned commits.
    #[must_use]
    pub fn assumptions(&self) -> BTreeMap<String, String> {
        let mut record = BTreeMap::new();
        record.insert("source_set".to_owned(), self.manifest.source_set.clone());
        record.insert(
            "schema_version".to_owned(),
            self.manifest.schema_version.to_string(),
        );
        record.insert("profile_digest".to_owned(), self.manifest_digest.clone());
        for (name, pin) in &self.manifest.pins {
            record.insert(format!("pin.{name}"), pin.git_commit.clone());
        }
        record.insert(
            "trust".to_owned(),
            "declared source-set assumptions, not verified upstream behavior".to_owned(),
        );
        record
    }
}

/// Load `<profiles_root>/<source_set>/profile.json` and every provider
/// manifest it references. Total-or-reject: on any diagnostic nothing is
/// returned and no partial profile escapes.
pub fn load_source_set_profile(
    profiles_root: &Path,
    source_set: &str,
) -> Result<LoadedProfile, ProfileDiagnostic> {
    if source_set.is_empty() || !is_safe_name(source_set) {
        return Err(ProfileDiagnostic::ProfileNotFound {
            source_set: source_set.to_owned(),
        });
    }
    load_profile_directory(&profiles_root.join(source_set))
}

/// Load a profile from its directory. The directory name is the source-set
/// identity and must equal the manifest's `source_set` (F5).
pub fn load_profile_directory(directory: &Path) -> Result<LoadedProfile, ProfileDiagnostic> {
    let expected_set = directory
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let manifest_path = directory.join("profile.json");
    if !manifest_path.is_file() {
        return Err(ProfileDiagnostic::ProfileNotFound {
            source_set: expected_set,
        });
    }
    let text = fs::read_to_string(&manifest_path).map_err(|error| {
        ProfileDiagnostic::ProfileUnreadable {
            path: manifest_path.display().to_string(),
            error: error.to_string(),
        }
    })?;
    let unique = crate::profile_json::parse(&text).map_err(|error| {
        ProfileDiagnostic::ProfileUnreadable {
            path: manifest_path.display().to_string(),
            error,
        }
    })?;
    // F3 before F4: the version is read first so an unknown version is
    // reported as such rather than as a field error against v1's shape.
    let version = declared_schema_version(&text, &manifest_path)?;
    if !SUPPORTED_PROFILE_MANIFEST_VERSIONS.contains(&version) {
        return Err(ProfileDiagnostic::UnsupportedSchemaVersion {
            found: version,
            supported: SUPPORTED_PROFILE_MANIFEST_VERSIONS.to_vec(),
        });
    }
    let manifest: ProfileManifest =
        serde_json::from_value(unique).map_err(|error| ProfileDiagnostic::ProfileUnreadable {
            path: manifest_path.display().to_string(),
            error: error.to_string(),
        })?;
    if manifest.source_set != expected_set {
        return Err(ProfileDiagnostic::ProfileInconsistent {
            detail: format!(
                "source_set `{}` does not match directory `{expected_set}`",
                manifest.source_set
            ),
        });
    }
    validate_manifest(&manifest)?;
    let mut provider_directories = BTreeMap::new();
    let providers = load_provider_manifests(directory, &manifest, &mut provider_directories)?;
    Ok(LoadedProfile {
        manifest_digest: sha256_hex(text.as_bytes()),
        manifest,
        directory: directory.to_path_buf(),
        manifest_text: text,
        providers,
        provider_directories,
    })
}

fn declared_schema_version(text: &str, path: &Path) -> Result<u32, ProfileDiagnostic> {
    #[derive(serde::Deserialize)]
    struct VersionProbe {
        schema_version: u32,
    }
    serde_json::from_str::<VersionProbe>(text)
        .map(|probe| probe.schema_version)
        .map_err(|error| ProfileDiagnostic::ProfileUnreadable {
            path: path.display().to_string(),
            error: error.to_string(),
        })
}

/// Validate the audited-file inventory and required construct witnesses of a
/// library, including profiles constructed directly through the public schema.
/// Both manifest loading and source-summary derivation use these exact rules.
/// This checks source-witness structure only: pin lookup, provider bindings and
/// other whole-manifest requirements still belong to the manifest loader. It
/// does not verify source bytes or turn declared mechanisms into semantic proof.
pub fn validate_library_source_profile(
    name: &str,
    library: &LibraryProfile,
) -> Result<(), ProfileDiagnostic> {
    let inconsistent = |detail: String| ProfileDiagnostic::ProfileInconsistent { detail };
    let mut audited = BTreeSet::new();
    for file in &library.audited_files {
        if !is_relative_without_traversal(&file.path) {
            return Err(inconsistent(format!(
                "library `{name}` audits non-relative path `{}`",
                file.path
            )));
        }
        if !is_lower_hex(&file.sha256, 64) {
            return Err(inconsistent(format!(
                "library `{name}` file `{}` has a malformed sha256",
                file.path
            )));
        }
        if !audited.insert(file.path.clone()) {
            return Err(inconsistent(format!(
                "library `{name}` audits `{}` twice",
                file.path
            )));
        }
    }
    let mut witnesses = BTreeSet::new();
    for witness in &library.source_witnesses {
        if !witnesses.insert(witness.name.clone()) {
            return Err(inconsistent(format!(
                "library `{name}` declares witness `{}` twice",
                witness.name
            )));
        }
        if !audited.contains(&witness.file) {
            return Err(inconsistent(format!(
                "library `{name}` witness `{}` names unaudited file `{}`",
                witness.name, witness.file
            )));
        }
        match &witness.check {
            WitnessCheck::Digest { construct } => {
                if construct.trim().is_empty() {
                    return Err(inconsistent(format!(
                        "library `{name}` witness `{}` names no construct",
                        witness.name
                    )));
                }
            }
            WitnessCheck::Substrings { needles, rationale } => {
                if needles.is_empty()
                    || needles.iter().any(|needle| needle.is_empty())
                    || rationale.trim().is_empty()
                {
                    return Err(ProfileDiagnostic::NeedleWithoutRationale {
                        witness: format!("{name}/{}", witness.name),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_manifest(manifest: &ProfileManifest) -> Result<(), ProfileDiagnostic> {
    let inconsistent = |detail: String| ProfileDiagnostic::ProfileInconsistent { detail };
    if manifest.libraries.is_empty() {
        return Err(inconsistent("no libraries declared".to_owned()));
    }
    if let Some(relative) = &manifest.resource_contract_manifest
        && !is_relative_with_parents(relative)
    {
        return Err(inconsistent(format!(
            "resource_contract_manifest `{relative}` must be a relative path"
        )));
    }
    for pin in manifest.pins.values() {
        if !is_lower_hex(&pin.git_commit, 40) {
            return Err(inconsistent(format!(
                "pin commit `{}` is not a full lowercase hex hash",
                pin.git_commit
            )));
        }
        for path in &pin.clean_paths {
            if !is_relative_without_traversal(path) {
                return Err(inconsistent(format!(
                    "clean path `{path}` must be relative without traversal"
                )));
            }
        }
    }
    let mut declared_providers = BTreeSet::new();
    for relative in &manifest.provider_manifests {
        if !is_relative_without_traversal(relative) {
            return Err(inconsistent(format!(
                "provider manifest path `{relative}` must be relative without traversal"
            )));
        }
        if !declared_providers.insert(relative.clone()) {
            return Err(inconsistent(format!(
                "duplicate provider manifest path `{relative}`"
            )));
        }
    }
    for (name, library) in &manifest.libraries {
        crate::typed_profile::validate_typed_profile(library)
            .map_err(|detail| inconsistent(format!("library `{name}`: {detail}")))?;
        if !manifest.pins.contains_key(&library.pin) {
            return Err(inconsistent(format!(
                "library `{name}` references unknown pin `{}`",
                library.pin
            )));
        }
        validate_library_source_profile(name, library)?;
        for root in &library.source_roots {
            if !is_admissible_source_root(&root.root, manifest.schema_version) {
                return Err(inconsistent(format!(
                    "library `{name}` source root `{}` must be relative",
                    root.root
                )));
            }
            if let qcore_schemas::profile::UnitSelection::Exact { file } = &root.selection
                && !is_relative_without_traversal(file)
            {
                return Err(inconsistent(format!(
                    "library `{name}` selects non-relative unit `{file}`"
                )));
            }
        }
        for (kind, summary) in &library.operation_summaries {
            if summary.trim().is_empty() {
                return Err(inconsistent(format!(
                    "library `{name}` declares an empty summary for {kind:?}"
                )));
            }
        }
        for build in &library.accepted_builds {
            if build.backend.trim().is_empty() || build.threads.trim().is_empty() {
                return Err(inconsistent(format!(
                    "library `{name}` declares an unnamed build selector"
                )));
            }
        }
        let mut field_models = BTreeSet::new();
        for model in &library.boolean_field_models {
            if !field_models.insert((&model.record, &model.field)) {
                return Err(inconsistent(format!(
                    "library `{name}` declares a duplicate Boolean field model"
                )));
            }
            if model.record.trim().is_empty() || model.field.trim().is_empty() {
                return Err(inconsistent(format!(
                    "library `{name}` declares an unnamed Boolean field model"
                )));
            }
        }
        if let Some(provider) = &library.provider
            && (provider.plugin_id.trim().is_empty()
                || provider.lean_theorem_family.trim().is_empty()
                || provider.diagnostic_namespace.trim().is_empty())
        {
            return Err(inconsistent(format!(
                "library `{name}` declares an incomplete provider binding"
            )));
        }
    }
    Ok(())
}

fn load_provider_manifests(
    directory: &Path,
    manifest: &ProfileManifest,
    provider_directories: &mut BTreeMap<PluginId, PathBuf>,
) -> Result<Vec<PluginManifest>, ProfileDiagnostic> {
    let mut providers = Vec::new();
    for relative in &manifest.provider_manifests {
        let path = directory.join(relative);
        let text = fs::read_to_string(&path).map_err(|error| {
            ProfileDiagnostic::ProviderManifestInvalid {
                path: path.display().to_string(),
                error: error.to_string(),
            }
        })?;
        let plugin: PluginManifest = serde_json::from_str(&text).map_err(|error| {
            ProfileDiagnostic::ProviderManifestInvalid {
                path: path.display().to_string(),
                error: error.to_string(),
            }
        })?;
        plugin
            .validate()
            .map_err(|error| ProfileDiagnostic::ProviderManifestInvalid {
                path: path.display().to_string(),
                error: error.to_string(),
            })?;
        provider_directories.insert(
            plugin.plugin_id.clone(),
            path.parent().unwrap_or(directory).to_path_buf(),
        );
        providers.push(plugin);
    }
    providers.sort_by(|left, right| left.plugin_id.as_str().cmp(right.plugin_id.as_str()));
    for window in providers.windows(2) {
        if window[0].plugin_id == window[1].plugin_id {
            return Err(ProfileDiagnostic::ProfileInconsistent {
                detail: format!("duplicate provider plugin id `{}`", window[0].plugin_id),
            });
        }
    }
    // F5/F18: every library's binding must name a declared provider and agree
    // with its manifest. Disagreement rejects the profile rather than picking
    // one of the two claims.
    for (name, library) in &manifest.libraries {
        let Some(binding) = &library.provider else {
            continue;
        };
        let Some(plugin) = providers
            .iter()
            .find(|plugin| plugin.plugin_id.as_str() == binding.plugin_id)
        else {
            return Err(ProfileDiagnostic::ProfileInconsistent {
                detail: format!(
                    "library `{name}` binds undeclared provider `{}`",
                    binding.plugin_id
                ),
            });
        };
        if plugin.lean_theorem_family != binding.lean_theorem_family {
            return Err(ProfileDiagnostic::ProviderManifestInvalid {
                path: binding.plugin_id.clone(),
                error: format!(
                    "library `{name}` declares theorem family `{}`, manifest declares `{}`",
                    binding.lean_theorem_family, plugin.lean_theorem_family
                ),
            });
        }
    }
    Ok(providers)
}

/// F7/F8: verify a pinned checkout. Returns the verified commit on success.
fn git_in(checkout: &Path) -> Command {
    let canonical = checkout
        .canonicalize()
        .unwrap_or_else(|_| checkout.to_path_buf());
    let text = canonical.to_string_lossy();
    let safe = text
        .strip_prefix(r"\\?\")
        .unwrap_or(&text)
        .replace('\\', "/");
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg(format!("safe.directory={safe}"))
        .arg("-C")
        .arg(checkout);
    command
}

pub fn verify_pin(name: &str, pin: &SourcePin, checkout: &Path) -> Result<String, PinDiagnostic> {
    let unavailable = || PinDiagnostic::GitUnavailable {
        pin: name.to_owned(),
    };
    let head = git_in(checkout)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|_| unavailable())?;
    if !head.status.success() {
        return Err(unavailable());
    }
    let actual = String::from_utf8_lossy(&head.stdout).trim().to_owned();
    if actual != pin.git_commit {
        return Err(PinDiagnostic::SourceRevisionMismatch {
            pin: name.to_owned(),
            expected: pin.git_commit.clone(),
            actual,
        });
    }
    if pin.clean_paths.is_empty() {
        let status = git_in(checkout)
            .args(["status", "--porcelain", "--untracked-files=no"])
            .output()
            .map_err(|_| unavailable())?;
        if !status.status.success() {
            return Err(unavailable());
        }
        if !status.stdout.is_empty() {
            return Err(PinDiagnostic::DirtyCheckout {
                pin: name.to_owned(),
                paths: Vec::new(),
            });
        }
    } else {
        let status = git_in(checkout)
            .args(["diff", "--quiet", "HEAD", "--"])
            .args(&pin.clean_paths)
            .status()
            .map_err(|_| unavailable())?;
        if !status.success() {
            return Err(PinDiagnostic::DirtyCheckout {
                pin: name.to_owned(),
                paths: pin.clean_paths.clone(),
            });
        }
    }
    Ok(actual)
}

/// F9/F10: read a library's audited files from a checkout, checking every
/// declared digest. The bundle is the input the construct witnesses are
/// evaluated against; a single mismatch rejects the whole inventory.
pub fn load_audited_bundle(
    library: &LibraryProfile,
    checkout: &Path,
    revision: &str,
) -> Result<ExternalSourceBundle, Vec<FileAuditDiagnostic>> {
    let mut files = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for audited in &library.audited_files {
        let path = checkout.join(&audited.path);
        let Ok(bytes) = fs::read(&path) else {
            diagnostics.push(FileAuditDiagnostic::MissingFile {
                path: audited.path.clone(),
            });
            continue;
        };
        let actual = sha256_hex(&bytes);
        let verified_bytes = if actual == audited.sha256 {
            Some(bytes)
        } else {
            pinned_lf_bytes_for_crlf_checkout(checkout, revision, &audited.path, &bytes)
                .filter(|canonical| sha256_hex(canonical) == audited.sha256)
        };
        let Some(bytes) = verified_bytes else {
            diagnostics.push(FileAuditDiagnostic::DigestMismatch {
                path: audited.path.clone(),
                expected: audited.sha256.clone(),
                actual,
            });
            continue;
        };
        files.insert(
            audited.path.clone(),
            String::from_utf8_lossy(&bytes).into_owned(),
        );
    }
    if diagnostics.is_empty() {
        Ok(ExternalSourceBundle {
            source_revision: revision.to_owned(),
            files,
        })
    } else {
        Err(diagnostics)
    }
}

/// Git may materialize a pinned LF blob as CRLF in a clean Windows worktree.
/// Accept that one reversible checkout transform only when the canonical blob
/// at the declared revision has exactly the expected contents. Any other byte
/// difference remains a digest mismatch.
fn pinned_lf_bytes_for_crlf_checkout(
    checkout: &Path,
    revision: &str,
    path: &str,
    worktree: &[u8],
) -> Option<Vec<u8>> {
    if !worktree.windows(2).any(|pair| pair == b"\r\n") {
        return None;
    }
    let blob = git_in(checkout)
        .args(["show", &format!("{revision}:{path}")])
        .output()
        .ok()?;
    if !blob.status.success() {
        return None;
    }
    let mut normalized = Vec::with_capacity(worktree.len());
    let mut index = 0;
    while index < worktree.len() {
        if worktree.get(index..index + 2) == Some(b"\r\n") {
            normalized.push(b'\n');
            index += 2;
        } else {
            normalized.push(worktree[index]);
            index += 1;
        }
    }
    (normalized == blob.stdout).then_some(blob.stdout)
}

fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

/// A relative path that may step out of the profile directory (the shared
/// contract manifest lives above `contract/profiles/`), but is never absolute
/// and never a filesystem root.
fn is_relative_with_parents(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::ParentDir))
}

/// Whether a declared source root is admissible at this manifest version.
///
/// Manifest version 2 admits exactly one additional spelling, the literal
/// [`CHECKOUT_ROOT_SOURCE_ROOT`], naming the pinned checkout root itself. The
/// test is string equality, not path normalisation: `"./src"`, `"a/."`, `".."`,
/// `"/"` and every absolute spelling keep the version-1 rejection, so the new
/// root reaches no directory the pinned checkout does not already contain, and
/// it widens no other declared path (audited files, clean paths, exact unit
/// selections and provider manifests are validated separately and unchanged).
#[must_use]
pub fn is_admissible_source_root(root: &str, schema_version: u32) -> bool {
    is_relative_without_traversal(root)
        || (schema_version >= 2 && root == qcore_schemas::profile::CHECKOUT_ROOT_SOURCE_ROOT)
}

/// The directory a declared source root names inside a checkout.
///
/// One resolver so every consumer agrees on what the checkout-root spelling
/// means: it is the checkout itself, not `checkout/.`, whose textual form would
/// compare unequal to a canonicalised entry path.
#[must_use]
pub fn source_root_path(checkout: &Path, root: &str) -> PathBuf {
    if root == qcore_schemas::profile::CHECKOUT_ROOT_SOURCE_ROOT {
        checkout.to_path_buf()
    } else {
        checkout.join(root)
    }
}

fn is_relative_without_traversal(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
}

const SHA256_ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// Lowercase hex SHA-256 (FIPS 180-4). Implemented here rather than taken as
/// a dependency: the workspace deliberately carries only `serde`/`serde_json`
/// outside the standard library, and the digest rule is a load-bearing part
/// of the profile contract that must be auditable in-tree.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let mut message = bytes.to_vec();
    let bit_length = (bytes.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    for block in message.as_chunks::<64>().0 {
        let mut schedule = [0u32; 64];
        for (index, word) in block.as_chunks::<4>().0.iter().enumerate() {
            schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(SHA256_ROUND_CONSTANTS[index])
                .wrapping_add(schedule[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }

    state
        .iter()
        .flat_map(|word| word.to_be_bytes())
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
