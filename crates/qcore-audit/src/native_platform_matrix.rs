//! Configured audit of the Cycle 18 host/backend evidence matrix.

use crate::observed_wait::reject_report_aliases;
use crate::wait_failures::write_atomic;
use qcore_schemas::sha256_hex;
use qcore_verification::native_platform_matrix::{
    NativePlatformMatrix, PlatformRowStatus, check_native_platform_matrix,
    native_platform_policy_digest, native_platform_source_revision,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BUNDLED_HARNESS: &[u8] = include_bytes!("../tests/fixtures/native_platform_probe.c");
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeArtifactSetConfig {
    pub sanitizers: String,
    pub build_commands: PathBuf,
    pub cmake_cache: PathBuf,
    pub compile_commands: PathBuf,
    pub libevent_core: PathBuf,
    pub libevent_pthreads: PathBuf,
    pub executable: PathBuf,
    pub runtime_records: Vec<PathBuf>,
    pub run_manifest: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildCommands {
    configure: Vec<String>,
    build: Vec<String>,
    compile_link: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePlatformMatrixConfig {
    pub schema_version: u32,
    pub matrix: PathBuf,
    pub policy: PathBuf,
    pub harness: PathBuf,
    pub historical_validation: PathBuf,
    pub libevent_source: PathBuf,
    pub artifact_sets: Vec<NativeArtifactSetConfig>,
    pub report: PathBuf,
}

fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}
fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}
fn git_revision(path: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("could not resolve selected Libevent source revision".into());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().into())
        .map_err(|error| error.to_string())
}

#[derive(Clone)]
struct BoundSet {
    commands: String,
    core: String,
    pthreads: String,
    executable: String,
    runtime: BTreeMap<String, Value>,
}

pub fn run_native_platform_matrix_config(path: &Path) -> Result<(), String> {
    let config_bytes = read(path)?;
    let config: NativePlatformMatrixConfig =
        serde_json::from_slice(&config_bytes).map_err(|error| error.to_string())?;
    if config.schema_version != 1 || config.artifact_sets.is_empty() {
        return Err("unsupported native platform matrix configuration".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let matrix_path = resolve(base, &config.matrix);
    let policy_path = resolve(base, &config.policy);
    let harness_path = resolve(base, &config.harness);
    let history_path = resolve(base, &config.historical_validation);
    let source_path = resolve(base, &config.libevent_source);
    let mut protected = vec![
        path.to_path_buf(),
        matrix_path.clone(),
        policy_path.clone(),
        harness_path.clone(),
        history_path.clone(),
    ];
    for set in &config.artifact_sets {
        protected.extend([
            resolve(base, &set.build_commands),
            resolve(base, &set.cmake_cache),
            resolve(base, &set.compile_commands),
            resolve(base, &set.libevent_core),
            resolve(base, &set.libevent_pthreads),
            resolve(base, &set.executable),
            resolve(base, &set.run_manifest),
        ]);
        protected.extend(
            set.runtime_records
                .iter()
                .map(|selected| resolve(base, selected)),
        );
    }
    let snapshots = protected
        .iter()
        .skip(1)
        .map(|selected| Ok((selected.clone(), read(selected)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let bytes = |selected: &Path| {
        snapshots
            .iter()
            .find(|(path, _)| path == selected)
            .map(|(_, bytes)| bytes.as_slice())
            .ok_or_else(|| format!("missing snapshot {}", selected.display()))
    };
    if bytes(&harness_path)? != BUNDLED_HARNESS {
        return Err("configured harness differs from audited repository bytes".into());
    }
    if sha256_hex(bytes(&policy_path)?) != native_platform_policy_digest() {
        return Err("configured policy differs from bundled executable policy".into());
    }
    let policy_revision = native_platform_source_revision()?;
    if git_revision(&source_path)? != policy_revision {
        return Err("selected Libevent source revision mismatch".into());
    }
    let mut bound = BTreeMap::new();
    for set in &config.artifact_sets {
        let commands_path = resolve(base, &set.build_commands);
        let cache_path = resolve(base, &set.cmake_cache);
        let compile_database_path = resolve(base, &set.compile_commands);
        let core_path = resolve(base, &set.libevent_core);
        let pthreads_path = resolve(base, &set.libevent_pthreads);
        let executable_path = resolve(base, &set.executable);
        let manifest_path = resolve(base, &set.run_manifest);
        let commands: BuildCommands =
            serde_json::from_slice(bytes(&commands_path)?).map_err(|error| error.to_string())?;
        let expected_flag = if set.sanitizers == "thread" {
            "-fsanitize=thread"
        } else {
            "-fsanitize=address,undefined"
        };
        let build_directory = cache_path.parent().ok_or("CMake cache has no parent")?;
        let compiler = commands
            .compile_link
            .first()
            .ok_or("empty compile/link command")?;
        let source_text = source_path.to_string_lossy().into_owned();
        let build_text = build_directory.to_string_lossy().into_owned();
        let harness_text = harness_path.to_string_lossy().into_owned();
        let core_text = core_path.to_string_lossy().into_owned();
        let pthreads_text = pthreads_path.to_string_lossy().into_owned();
        let executable_text = executable_path.to_string_lossy().into_owned();
        let expected_configure = vec![
            "cmake".into(),
            "-S".into(),
            source_text.clone(),
            "-B".into(),
            build_text.clone(),
            "-DCMAKE_BUILD_TYPE=Debug".into(),
            "-DEVENT__LIBRARY_TYPE=STATIC".into(),
            "-DEVENT__DISABLE_OPENSSL=ON".into(),
            "-DEVENT__DISABLE_MBEDTLS=ON".into(),
            "-DEVENT__DISABLE_TESTS=ON".into(),
            "-DEVENT__DISABLE_REGRESS=ON".into(),
            "-DEVENT__DISABLE_BENCHMARK=ON".into(),
            "-DEVENT__DISABLE_SAMPLES=ON".into(),
            "-DEVENT__DISABLE_THREAD_SUPPORT=OFF".into(),
            "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".into(),
            format!("-DCMAKE_C_COMPILER={compiler}"),
            format!(
                "-DCMAKE_C_FLAGS={expected_flag}{}",
                if set.sanitizers == "thread" {
                    ""
                } else {
                    " -fno-omit-frame-pointer"
                }
            ),
            format!(
                "-DCMAKE_EXE_LINKER_FLAGS={expected_flag}{}",
                if set.sanitizers == "thread" {
                    ""
                } else {
                    " -fno-omit-frame-pointer"
                }
            ),
        ];
        if commands.configure != expected_configure {
            return Err(format!("{} configure argv mismatch", set.sanitizers));
        }
        let expected_build = vec![
            "cmake".into(),
            "--build".into(),
            build_text.clone(),
            "--parallel".into(),
            "2".into(),
            "--target".into(),
            "event_core_static".into(),
            "event_pthreads_static".into(),
        ];
        if commands.build != expected_build {
            return Err(format!("{} build argv mismatch", set.sanitizers));
        }
        let mut expected_link = vec![
            compiler.clone(),
            "-std=gnu17".into(),
            "-Wall".into(),
            "-Wextra".into(),
            "-Werror".into(),
            "-g".into(),
            expected_flag.into(),
        ];
        if set.sanitizers != "thread" {
            expected_link.push("-fno-omit-frame-pointer".into());
        }
        expected_link.extend([
            "-I".into(),
            build_directory
                .join("include")
                .to_string_lossy()
                .into_owned(),
            "-I".into(),
            source_path.join("include").to_string_lossy().into_owned(),
            harness_text,
            pthreads_text,
            core_text,
            "-pthread".into(),
            "-o".into(),
            executable_text,
        ]);
        if commands.compile_link != expected_link {
            return Err(format!("{} compile/link argv mismatch", set.sanitizers));
        }
        let cache = std::str::from_utf8(bytes(&cache_path)?).map_err(|error| error.to_string())?;
        let expected_c_flags = format!(
            "CMAKE_C_FLAGS:STRING={expected_flag}{}",
            if set.sanitizers == "thread" {
                ""
            } else {
                " -fno-omit-frame-pointer"
            }
        );
        if !cache.lines().any(|line| {
            line == format!("CMAKE_C_COMPILER:FILEPATH={compiler}")
                || line == format!("CMAKE_C_COMPILER:UNINITIALIZED={compiler}")
        }) || !cache.lines().any(|line| line == expected_c_flags)
        {
            return Err(format!("{} CMake cache identity mismatch", set.sanitizers));
        }
        let compile_database: Value = serde_json::from_slice(bytes(&compile_database_path)?)
            .map_err(|error| error.to_string())?;
        let entries = compile_database
            .as_array()
            .ok_or("compile_commands must be an array")?;
        if entries.is_empty()
            || entries.iter().any(|entry| {
                entry["directory"] != build_text
                    || !entry["command"].as_str().is_some_and(|command| {
                        command
                            .split_ascii_whitespace()
                            .any(|argument| argument == expected_flag)
                    })
            })
        {
            return Err(format!(
                "{} compile_commands identity mismatch",
                set.sanitizers
            ));
        }
        let manifest: Value =
            serde_json::from_slice(bytes(&manifest_path)?).map_err(|error| error.to_string())?;
        if manifest["schema_version"] != 1
            || manifest["status"] != "completed_linux_native_probe"
            || manifest["source_revision"] != policy_revision
            || manifest["sanitizers"] != set.sanitizers
        {
            return Err("unsupported native artifact-set manifest".into());
        }
        let hashes = [
            ("harness", sha256_hex(BUNDLED_HARNESS)),
            ("build_commands", sha256_hex(bytes(&commands_path)?)),
            ("cmake_cache", sha256_hex(bytes(&cache_path)?)),
            (
                "compile_commands",
                sha256_hex(bytes(&compile_database_path)?),
            ),
            ("libevent_core", sha256_hex(bytes(&core_path)?)),
            ("libevent_pthreads", sha256_hex(bytes(&pthreads_path)?)),
            ("executable", sha256_hex(bytes(&executable_path)?)),
        ];
        for (role, digest) in &hashes {
            if manifest["artifacts_sha256"][*role].as_str() != Some(digest) {
                return Err(format!("manifest does not bind {role}"));
            }
        }
        let mut runtime = BTreeMap::new();
        for selected in &set.runtime_records {
            let runtime_path = resolve(base, selected);
            let value: Value =
                serde_json::from_slice(bytes(&runtime_path)?).map_err(|error| error.to_string())?;
            let backend = value["backend"]
                .as_str()
                .ok_or("runtime record lacks backend")?
                .to_owned();
            if runtime.insert(backend.clone(), value).is_some() {
                return Err("duplicate runtime backend".into());
            }
            if manifest["runtime_sha256"][&backend].as_str()
                != Some(sha256_hex(bytes(&runtime_path)?).as_str())
            {
                return Err(format!("manifest does not bind runtime {backend}"));
            }
        }
        let entry = BoundSet {
            commands: hashes[1].1.clone(),
            core: hashes[4].1.clone(),
            pthreads: hashes[5].1.clone(),
            executable: hashes[6].1.clone(),
            runtime,
        };
        if bound.insert(set.sanitizers.clone(), entry).is_some() {
            return Err("duplicate sanitizer artifact set".into());
        }
    }
    let matrix: NativePlatformMatrix =
        serde_json::from_slice(bytes(&matrix_path)?).map_err(|error| error.to_string())?;
    let history_digest = sha256_hex(bytes(&history_path)?);
    for row in &matrix.rows {
        if let Some(history) = &row.historical_native_result
            && history.source_document_sha256 != history_digest
        {
            return Err("carried historical result does not bind validation record".into());
        }
        if row.status == PlatformRowStatus::Observed {
            let evidence = row.evidence.as_ref().ok_or("observed row lacks evidence")?;
            let selected = bound
                .get(&row.dimensions.sanitizers)
                .ok_or("observed row lacks selected sanitizer artifact set")?;
            let runtime = selected
                .runtime
                .get(&row.dimensions.backend)
                .ok_or("observed row lacks raw backend record")?;
            if evidence.build.command_sha256 != selected.commands
                || evidence.build.libevent_core_sha256 != selected.core
                || evidence.build.libevent_pthreads_sha256 != selected.pthreads
                || evidence.build.executable_sha256 != selected.executable
            {
                return Err(
                    "observed row build/link identity differs from selected artifacts".into(),
                );
            }
            let encoded =
                serde_json::to_value(&evidence.runtime).map_err(|error| error.to_string())?;
            for (field, actual) in [
                ("selected_backend", runtime["backend"].clone()),
                ("dispatch_thread", runtime["dispatch_thread"].clone()),
                ("activating_thread", runtime["activating_thread"].clone()),
                ("callback_thread", runtime["callback_thread"].clone()),
                ("activation_count", runtime["activation_count"].clone()),
                ("callback_count", runtime["callback_count"].clone()),
                ("callback_flags", runtime["callback_flags"].clone()),
                ("process_exit", runtime["process_exit"].clone()),
            ] {
                if encoded[field] != actual {
                    return Err(format!("runtime differs from raw {field}"));
                }
            }
            if runtime["dispatch_result"] != 0 {
                return Err("raw native dispatch did not exit successfully".into());
            }
        }
    }
    let checked = check_native_platform_matrix(matrix, &sha256_hex(BUNDLED_HARNESS))?;
    let report_path = resolve(base, &config.report);
    reject_report_aliases(&report_path, &protected, std::slice::from_ref(&source_path))?;
    if read(path)? != config_bytes
        || snapshots
            .iter()
            .any(|(selected, snapshot)| read(selected).as_deref() != Ok(snapshot.as_slice()))
        || git_revision(&source_path)? != policy_revision
    {
        return Err("configured platform evidence changed during audit".into());
    }
    let observed = checked
        .rows
        .iter()
        .filter(|row| row.status == PlatformRowStatus::Observed)
        .count();
    let report = serde_json::json!({"schema_version":1,"status":"native_platform_matrix_checked","observed_rows":observed,"unknown_rows":checked.rows.len()-observed,"historical_validation_sha256":history_digest,"matrix":checked,"limitations":["Observed rows are generic Libevent probes, not Monkey callback evidence","Observer transparency is not claimed","Carried historical native results do not satisfy current sanitizer/observer/artifact joins"]});
    write_atomic(
        &report_path,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_rejects_unknown_fields() {
        let value = serde_json::json!({"schema_version":1,"matrix":"m","policy":"p","harness":"h","historical_validation":"v","libevent_source":"s","artifact_sets":[],"report":"o","invented":true});
        assert!(serde_json::from_value::<NativePlatformMatrixConfig>(value).is_err());
    }
}
