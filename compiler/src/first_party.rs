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

//! H6 first-party package discovery through the W2 profile manifest path.
//!
//! All metadata is loaded and validated as schema records. There is no
//! library/id/capability table here. The existing built-in v1 adapter still
//! returns Unknown; an executable entry uses the existing process adapter.
//! Discovery is atomic across the selected root, and ordering is by plugin id.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use qcore_contract::profile_loader::load_profile_directory;

use crate::verifier_plugin::{
    BuiltInProvider, PackageLoadError, PluginEntrypoint, PluginManifest, VerifierProvider,
    VerifierRegistry, verify_executable_package,
};

fn failure(path: &Path, error: impl ToString) -> PackageLoadError {
    PackageLoadError {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

fn packages(root: &Path) -> Result<Vec<(PluginManifest, PathBuf)>, PackageLoadError> {
    let entries = std::fs::read_dir(root).map_err(|error| failure(root, error))?;
    let mut directories = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| failure(root, error))?;
        if entry
            .file_type()
            .map_err(|error| failure(&entry.path(), error))?
            .is_dir()
        {
            directories.push(entry.path());
        }
    }
    directories.sort();
    let mut packages = Vec::new();
    let mut ids = BTreeSet::new();
    for directory in directories {
        let profile =
            load_profile_directory(&directory).map_err(|error| failure(&directory, error))?;
        // The loader validated all package paths and manifest records. Resolve
        // executable roots from each manifest path without re-reading any file.
        for manifest in profile.providers() {
            if !ids.insert(manifest.plugin_id.clone()) {
                return Err(failure(
                    &directory,
                    format!("duplicate provider `{}`", manifest.plugin_id),
                ));
            }
            let package_root =
                profile
                    .provider_directory(&manifest.plugin_id)
                    .ok_or_else(|| {
                        failure(&directory, "provider has no validated package directory")
                    })?;
            packages.push((manifest.clone(), package_root.to_path_buf()));
        }
    }
    packages.sort_by(|a, b| a.0.plugin_id.as_str().cmp(b.0.plugin_id.as_str()));
    Ok(packages)
}

pub fn load_first_party_manifests(root: &Path) -> Result<Vec<PluginManifest>, PackageLoadError> {
    Ok(packages(root)?
        .into_iter()
        .map(|(manifest, _)| manifest)
        .collect())
}

pub fn load_first_party_registry(root: &Path) -> Result<VerifierRegistry, PackageLoadError> {
    let mut registry = VerifierRegistry::default();
    for (manifest, package_root) in packages(root)? {
        let provider: Arc<dyn VerifierProvider> = match &manifest.entrypoint {
            PluginEntrypoint::BuiltIn => Arc::new(
                BuiltInProvider::new(manifest).map_err(|error| failure(&package_root, error))?,
            ),
            // Verified, not merely loaded (card `P01-F3`): the declared digest
            // and the executable's own embedded manifest template must bind
            // before the package can serve a request, and the verified manifest
            // must be exactly the one the profile declared. Before this, an
            // executable provider was registered from profile metadata alone.
            PluginEntrypoint::Executable { .. } => {
                let provider = verify_executable_package(&package_root)
                    .map_err(|error| failure(&package_root, error.message))?;
                if provider.manifest() != &manifest {
                    return Err(failure(
                        &package_root,
                        format!(
                            "installed manifest for `{}` differs from the profile's declaration",
                            manifest.plugin_id
                        ),
                    ));
                }
                Arc::new(provider)
            }
        };
        registry
            .register(provider)
            .map_err(|error| failure(&package_root, error))?;
    }
    Ok(registry)
}

/// Compatibility helper: failed default discovery provides no manifests.
/// Call the Result-returning loader to retain the diagnostic.
#[must_use]
pub fn first_party_manifests() -> Vec<PluginManifest> {
    load_first_party_manifests(&crate::source_profiles::profiles_root()).unwrap_or_default()
}

/// Compatibility helper used by source compilation. An invalid package root
/// means an empty registry, so every verify scope reports its provider absent.
/// A source file that requests no provider remains independent of packages.
#[must_use]
pub fn first_party_registry() -> VerifierRegistry {
    load_first_party_registry(&crate::source_profiles::profiles_root()).unwrap_or_default()
}
