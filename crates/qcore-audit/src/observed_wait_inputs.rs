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

//! Independently loaded source/build inputs for the observed timed-wait audit.
//!
//! A producer manifest is compared with the selected checkout and files. It is
//! never used as its own expected source identity.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, prepare_translation_unit,
};
use qcore_c_gateway::typed_source::{
    SourceResult, SourceSummary, import_source_functions_from_profile,
};
use qcore_contract::profile_loader::{LoadedProfile, load_profile_directory, verify_pin};
use qcore_schemas::profile::ImportRole;
use qcore_schemas::sha256_hex;
use serde_json::Value;

use crate::compile_db::{
    CompileTaskIdentity, EffectiveInput, compile_entries, compile_task_identity, compiler_options,
    select_unique_configuration,
};

pub(crate) struct WaitSourceInputs {
    pub profile: LoadedProfile,
    pub revision: String,
    pub summaries: Vec<SourceSummary>,
    pub compile_task: CompileTaskIdentity,
    pub source_sha256: String,
    pub build_sha256: String,
    pub source_files: BTreeMap<String, String>,
    /// Directories whose contents contributed to the prepared source identity.
    /// An audit report must not be written through any path inside them.
    pub protected_directories: Vec<PathBuf>,
    /// Individual configured inputs outside the protected directories.
    pub protected_files: Vec<PathBuf>,
}

pub(crate) fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value = serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((bytes, value))
}

pub(crate) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing nonempty `{key}`"))
}

fn relative_path(name: &str) -> Result<&Path, String> {
    let path = Path::new(name);
    if name.is_empty()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(format!("invalid relative source path: {name}"));
    }
    Ok(path)
}

fn verify_hashes(
    manifest: &Value,
    key: &str,
    directory: &Path,
    canonical: Option<&Path>,
) -> Result<BTreeMap<String, String>, String> {
    let entries = manifest
        .get(key)
        .and_then(Value::as_object)
        .filter(|entries| !entries.is_empty())
        .ok_or_else(|| format!("missing nonempty `{key}` inventory"))?;
    let mut hashes = BTreeMap::new();
    for (name, expected) in entries {
        let path = relative_path(name)?;
        let bytes = fs::read(directory.join(path)).map_err(|e| format!("{name}: {e}"))?;
        let actual = sha256_hex(&bytes);
        if expected.as_str() != Some(actual.as_str()) {
            return Err(format!("{key} digest mismatch: {name}"));
        }
        if let Some(canonical) = canonical {
            let original = fs::read(canonical.join(path)).map_err(|e| format!("{name}: {e}"))?;
            if original != bytes {
                return Err(format!("source copy differs from pinned checkout: {name}"));
            }
        }
        hashes.insert(name.clone(), actual);
    }
    Ok(hashes)
}

/// Load a pinned, forced-Libevent Monkey compilation and import its real source.
pub(crate) fn load_wait_source(
    profile_directory: &Path,
    fluent_checkout: &Path,
    monkey_inputs: &Path,
    observer_compile_database: &Path,
) -> Result<WaitSourceInputs, String> {
    load_monkey_source(
        profile_directory,
        fluent_checkout,
        monkey_inputs,
        observer_compile_database,
        &[
            "cb_wait_2_timeout",
            "_mk_event_loop_create",
            "_mk_event_wait_with_flags",
            "_mk_event_wait_2",
            "_mk_event_loop_destroy",
        ],
    )
}

/// Reuse the same pinned compilation and immutable input identity for an
/// explicitly selected set of callback/wrapper summaries. The consuming
/// verification policy must independently require its complete inventory.
pub(crate) fn load_monkey_source(
    profile_directory: &Path,
    fluent_checkout: &Path,
    monkey_inputs: &Path,
    observer_compile_database: &Path,
    selected: &[&str],
) -> Result<WaitSourceInputs, String> {
    let profile = load_profile_directory(profile_directory).map_err(|e| e.to_string())?;
    let pin = profile
        .manifest()
        .pin_for("monkey")
        .ok_or("missing Monkey source pin")?;
    let revision = verify_pin("monkey", pin, fluent_checkout).map_err(|e| e.to_string())?;
    let (manifest_bytes, manifest) = read_json(monkey_inputs)?;
    if manifest.get("schema_version").and_then(Value::as_u64) != Some(1)
        || text(&manifest, "source_revision")? != revision
        || Path::new(text(&manifest, "upstream")?) != fluent_checkout
    {
        return Err("Monkey input manifest does not identify the selected pinned checkout".into());
    }
    let source_root = PathBuf::from(text(&manifest, "source")?);
    let build_root = PathBuf::from(text(&manifest, "build")?);
    if !source_root.is_absolute() || !build_root.is_absolute() {
        return Err("Monkey source/build paths must be absolute".into());
    }
    let source_hashes = verify_hashes(
        &manifest,
        "source_sha256",
        &source_root,
        Some(&fluent_checkout.join("lib/monkey")),
    )?;
    for name in ["mk_core/mk_event.c", "mk_core/mk_event_libevent.c"] {
        if !source_hashes.contains_key(name) {
            return Err(format!("missing critical Monkey source identity: {name}"));
        }
    }
    let generated = verify_hashes(&manifest, "generated_source_sha256", &source_root, None)?;
    // Older retained setup manifests do not enumerate generated headers.
    // Their effective contents are still bound by the prepared compile task.
    let headers = if manifest.get("generated_headers_sha256").is_some() {
        verify_hashes(&manifest, "generated_headers_sha256", &build_root, None)?
    } else {
        BTreeMap::new()
    };
    let mut protected_files = source_hashes
        .keys()
        .chain(generated.keys())
        .map(|name| source_root.join(name))
        .chain(headers.keys().map(|name| build_root.join(name)))
        .collect::<Vec<_>>();
    let build_cache = fs::read(build_root.join("CMakeCache.txt")).map_err(|e| e.to_string())?;
    let database_path = Path::new(text(&manifest, "compile_database")?);
    let database_bytes = fs::read(database_path).map_err(|e| e.to_string())?;
    if text(&manifest, "compile_database_sha256")? != sha256_hex(&database_bytes) {
        return Err("Monkey compilation database digest mismatch".into());
    }
    let observer_database_bytes = fs::read(observer_compile_database).map_err(|e| e.to_string())?;
    let database = std::str::from_utf8(&observer_database_bytes).map_err(|e| e.to_string())?;
    let entries = compile_entries(database)?;
    let source_path = source_root.join("mk_core/mk_event.c");
    let entry = select_unique_configuration(&entries, &source_path)?;
    let raw = fs::read_to_string(&entry.file).map_err(|e| e.to_string())?;
    let options = compiler_options(entry)?;
    let unit = C11TranslationUnit {
        source: &raw,
        include_paths: options.include_paths,
        definitions: options.definitions,
        preprocessor_options: options.preprocessor_options,
        dialect: options.dialect,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: options.input,
    };
    let prepared = prepare_translation_unit(&unit);
    let effective = prepared.preprocessed_source().map_err(|e| e.to_string())?;
    let dependencies = BTreeMap::from([
        (
            "source_profile".into(),
            profile.manifest_digest().to_owned(),
        ),
        ("monkey_inputs".into(), sha256_hex(&manifest_bytes)),
        ("compile_database".into(), sha256_hex(&database_bytes)),
        (
            "observer_compile_database".into(),
            sha256_hex(&observer_database_bytes),
        ),
        (
            "source_rules".into(),
            sha256_hex(qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES.as_bytes()),
        ),
    ]);
    let compile_task = compile_task_identity(
        entry,
        &unit,
        EffectiveInput::Accepted(effective),
        &ClangIdentity::capture()?,
        dependencies,
    )?;
    let mut summaries = Vec::new();
    for role in [ImportRole::Callback, ImportRole::Wrapper] {
        for result in
            import_source_functions_from_profile(&unit, &revision, &profile, "monkey", role)?
        {
            match result {
                SourceResult::Summary(summary) if selected.contains(&summary.function()) => {
                    summaries.push(summary)
                }
                SourceResult::Unknown { function, reason }
                    if selected.contains(&function.as_str()) =>
                {
                    return Err(format!("{function}: source Unknown: {reason}"));
                }
                _ => {}
            }
        }
    }
    let source_sha256 = sha256_hex(&serde_json::to_vec(&source_hashes).map_err(|e| e.to_string())?);
    let build_sha256 = sha256_hex(
        &serde_json::to_vec(&(
            sha256_hex(&manifest_bytes),
            generated,
            headers,
            sha256_hex(&database_bytes),
            sha256_hex(&build_cache),
        ))
        .map_err(|e| e.to_string())?,
    );
    Ok(WaitSourceInputs {
        profile,
        revision,
        summaries,
        compile_task,
        source_sha256,
        build_sha256,
        source_files: source_hashes,
        protected_directories: vec![
            profile_directory.to_path_buf(),
            fluent_checkout.to_path_buf(),
            source_root.clone(),
            build_root.clone(),
        ],
        protected_files: {
            protected_files.extend([
                profile_directory.join("profile.json"),
                monkey_inputs.to_path_buf(),
                observer_compile_database.to_path_buf(),
                database_path.to_path_buf(),
                entry.file.clone(),
                build_root.join("CMakeCache.txt"),
            ]);
            protected_files
        },
    })
}
