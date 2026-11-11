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

//! Explicit local provider selection. No implicit home-directory configuration.
use qcore_compiler::verifier_plugin::{
    ExecutionPolicy, PluginId, PluginManifest, VerifierProvider, VerifierRegistry,
    verify_executable_package,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    version: u16,
    packages: Vec<Selection>,
    /// The execution profile every provider this configuration builds runs
    /// under (card `P03-F`). Absent means the recorded default: the
    /// historical, explicitly labeled `unenforced-legacy` profile. The key is
    /// omitted when it holds that default, so a configuration written before
    /// this field existed round-trips byte-identically.
    #[serde(default, skip_serializing_if = "is_default_execution")]
    execution: ExecutionPolicy,
}

fn is_default_execution(policy: &ExecutionPolicy) -> bool {
    *policy == ExecutionPolicy::default()
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    path: PathBuf,
    enabled: bool,
    /// Pin the complete validated manifest, including executable identity.
    manifest: PluginManifest,
}
fn read(path: &Path) -> Result<Configuration, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let config: Configuration = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if config.version != 1 {
        return Err("unsupported provider configuration version".into());
    }
    // A written execution selection is checked before any package is opened,
    // so a configuration that names an impossible one builds no provider at
    // all rather than falling back to a weaker profile.
    config.execution.validate().map_err(|e| e.to_string())?;
    let mut ids = BTreeSet::new();
    for package in &config.packages {
        package.manifest.validate().map_err(|e| e.to_string())?;
        if !package.path.is_absolute() || !ids.insert(package.manifest.plugin_id.clone()) {
            return Err("provider paths must be absolute and plugin IDs unique".into());
        }
    }
    Ok(config)
}
fn write(path: &Path, config: &Configuration) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
    // The sibling temporary file makes replacement atomic on supported hosts.
    // create_new prevents overwriting another writer's pending file.
    let temporary = path.with_extension(format!("qcore-{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("{}: {e}", temporary.display()))?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("{}: {e}", path.display()))
}
pub fn registry(path: &Path) -> Result<VerifierRegistry, String> {
    let config = read(path)?;
    let mut registry = VerifierRegistry::default();
    for package in config.packages.into_iter().filter(|p| p.enabled) {
        let provider = verify_executable_package(&package.path).map_err(|e| e.to_string())?;
        if provider.manifest() != &package.manifest {
            return Err(format!(
                "plugin `{}` changed since enable; enable the reviewed package again",
                package.manifest.plugin_id
            ));
        }
        // The written selection is applied after identity verification and
        // before registration, so every provider in this registry runs under
        // exactly the profile the configuration names.
        let provider = config
            .execution
            .apply(provider)
            .map_err(|e| format!("{}: {e}", package.manifest.plugin_id))?;
        registry
            .register(Arc::new(provider))
            .map_err(|e| e.to_string())?;
    }
    Ok(registry)
}

/// The execution profile this configuration selects, for `plugins list` and
/// for a caller that wants to state what a run enforced.
pub fn execution_policy(path: &Path) -> Result<ExecutionPolicy, String> {
    Ok(read(path)?.execution)
}
pub fn enable(config_path: &Path, package_path: &Path) -> Result<(), String> {
    let provider = verify_executable_package(package_path).map_err(|e| e.to_string())?;
    let mut config = match fs::metadata(config_path) {
        Ok(_) => read(config_path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Configuration {
            version: 1,
            packages: vec![],
            // A new configuration records the historical default explicitly;
            // it is not serialized, so the written bytes are unchanged.
            execution: ExecutionPolicy::default(),
        },
        Err(e) => return Err(e.to_string()),
    };
    let selected = Selection {
        path: package_path.canonicalize().map_err(|e| e.to_string())?,
        enabled: true,
        manifest: provider.manifest().clone(),
    };
    config
        .packages
        .retain(|p| p.manifest.plugin_id != selected.manifest.plugin_id);
    config.packages.push(selected);
    config
        .packages
        .sort_by(|a, b| a.manifest.plugin_id.cmp(&b.manifest.plugin_id));
    write(config_path, &config)
}
pub fn disable(config_path: &Path, id: &str) -> Result<(), String> {
    let id = PluginId::new(id).map_err(|e| e.to_string())?;
    let mut config = read(config_path)?;
    let selected = config
        .packages
        .iter_mut()
        .find(|p| p.manifest.plugin_id == id)
        .ok_or_else(|| format!("plugin `{id}` is not configured"))?;
    selected.enabled = false;
    write(config_path, &config)
}
