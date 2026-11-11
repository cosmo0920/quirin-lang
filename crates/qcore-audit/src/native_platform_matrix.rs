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

//! Configured audit of the Cycle 18 host/backend evidence matrix.

use crate::observed_wait::reject_report_aliases;
use crate::wait_failures::write_atomic;
use qcore_schemas::sha256_hex;
use qcore_verification::native_platform_matrix::{
    NativePlatformMatrix, PlatformRowStatus, check_native_platform_matrix,
    native_platform_historical_digest, native_platform_policy_digest,
    native_platform_source_revision,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
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

fn git_worktree_status(path: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args([
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignored",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("could not inspect selected Libevent source worktree".into());
    }
    Ok(output.stdout)
}

fn compiler_version(path: &str) -> Result<String, String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("could not execute selected compiler {path}: {error}"))?;
    if !output.status.success() {
        return Err(format!("selected compiler {path} --version failed"));
    }
    let version = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let version = version.trim();
    if version.is_empty() {
        return Err(format!("selected compiler {path} returned no version"));
    }
    Ok(version.to_owned())
}

fn archive_members(archiver: &str, path: &Path) -> Result<Vec<String>, String> {
    let output = Command::new(archiver)
        .arg("t")
        .arg(path)
        .output()
        .map_err(|error| {
            format!(
                "could not inspect selected archive {} with {archiver}: {error}",
                path.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "selected archiver {archiver} could not inspect {}",
            path.display()
        ));
    }
    let members = String::from_utf8(output.stdout)
        .map_err(|error| error.to_string())?
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if members.is_empty() || members.iter().any(|member| member.is_empty()) {
        return Err(format!(
            "selected archive {} has no members",
            path.display()
        ));
    }
    Ok(members)
}

#[derive(Clone)]
struct BoundSet {
    compiler: String,
    compiler_version: String,
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
    if config.schema_version != 1 {
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
    let source_status = git_worktree_status(&source_path)?;
    if !source_status.is_empty() {
        return Err("selected Libevent source worktree is not clean".into());
    }
    let mut bound = BTreeMap::new();
    let mut source_snapshots = BTreeMap::<PathBuf, Vec<u8>>::new();
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
        if !Path::new(compiler).is_absolute() {
            return Err(format!(
                "{} selected compiler path is not absolute",
                set.sanitizers
            ));
        }
        let selected_compiler_version = compiler_version(compiler)?;
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
        let archiver = cache
            .lines()
            .find_map(|line| line.strip_prefix("CMAKE_AR:FILEPATH="))
            .ok_or_else(|| format!("{} CMake cache lacks selected archiver", set.sanitizers))?;
        if !Path::new(archiver).is_absolute() {
            return Err(format!(
                "{} CMake cache archiver path is not absolute",
                set.sanitizers
            ));
        }
        let compile_database: Value = serde_json::from_slice(bytes(&compile_database_path)?)
            .map_err(|error| error.to_string())?;
        let entries = compile_database
            .as_array()
            .ok_or("compile_commands must be an array")?;
        let source_prefix = format!("{source_text}/");
        let compiler_prefix = format!("{compiler} ");
        let expected_core_source = format!("{source_text}/event.c");
        let expected_pthreads_source = format!("{source_text}/evthread_pthread.c");
        let valid_entry = |entry: &Value| {
            let file = entry["file"].as_str()?;
            let command = entry["command"].as_str()?;
            let output = entry["output"].as_str()?;
            let relative_source = Path::new(file).strip_prefix(&source_path).ok()?;
            Some(
                entry["directory"] == build_text
                    && file.starts_with(&source_prefix)
                    && !relative_source.as_os_str().is_empty()
                    && relative_source
                        .components()
                        .all(|component| matches!(component, std::path::Component::Normal(_)))
                    && file.ends_with(".c")
                    && command.split_ascii_whitespace().next() == Some(compiler.as_str())
                    && command.starts_with(&compiler_prefix)
                    && command
                        .split_ascii_whitespace()
                        .any(|argument| argument == expected_flag)
                    && command.contains(&format!(" -o {output} "))
                    && command.ends_with(&format!(" -c {file}"))
                    && output.starts_with("CMakeFiles/"),
            )
        };
        let role_members = |role: &str| -> Option<Vec<String>> {
            entries
                .iter()
                .filter(|entry| {
                    entry["output"]
                        .as_str()
                        .is_some_and(|output| output.starts_with(role))
                })
                .map(|entry| {
                    Path::new(entry["output"].as_str()?)
                        .file_name()?
                        .to_str()
                        .map(str::to_owned)
                })
                .collect()
        };
        let core_members = role_members("CMakeFiles/event_core_static.dir/")
            .ok_or("core compile role contains an invalid output")?;
        let pthread_members = role_members("CMakeFiles/event_pthreads_static.dir/")
            .ok_or("pthread compile role contains an invalid output")?;
        let core_archive_members = archive_members(archiver, &core_path)?;
        let pthread_archive_members = archive_members(archiver, &pthreads_path)?;
        let exact_role = |compiled: &[String], archived: &[String]| {
            !compiled.is_empty()
                && compiled.len() == archived.len()
                && compiled.iter().collect::<BTreeSet<_>>().len() == compiled.len()
                && archived.iter().collect::<BTreeSet<_>>().len() == archived.len()
                && compiled.iter().collect::<BTreeSet<_>>()
                    == archived.iter().collect::<BTreeSet<_>>()
        };
        let core_role = entries.iter().any(|entry| {
            entry["file"] == expected_core_source
                && entry["output"] == "CMakeFiles/event_core_static.dir/event.c.o"
        }) && exact_role(&core_members, &core_archive_members);
        let pthread_role = entries.iter().any(|entry| {
            entry["file"] == expected_pthreads_source
                && entry["output"] == "CMakeFiles/event_pthreads_static.dir/evthread_pthread.c.o"
        }) && exact_role(&pthread_members, &pthread_archive_members);
        if entries.is_empty()
            || entries.iter().any(|entry| valid_entry(entry) != Some(true))
            || !core_role
            || !pthread_role
        {
            return Err(format!(
                "{} compile_commands identity mismatch",
                set.sanitizers
            ));
        }
        for entry in entries {
            let source = PathBuf::from(
                entry["file"]
                    .as_str()
                    .ok_or("compile_commands entry lacks source file")?,
            );
            source_snapshots
                .entry(source.clone())
                .or_insert(read(&source)?);
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
            compiler: compiler.clone(),
            compiler_version: selected_compiler_version,
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
    let history_digest = historical_validation_digest(bytes(&history_path)?);
    if history_digest != native_platform_historical_digest()? {
        return Err("configured historical validation differs from policy identity".into());
    }
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
                || evidence.build.compiler_path != selected.compiler
                || evidence.build.compiler_version != selected.compiler_version
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
        || source_snapshots
            .iter()
            .any(|(selected, snapshot)| read(selected).as_deref() != Ok(snapshot.as_slice()))
        || git_revision(&source_path)? != policy_revision
        || git_worktree_status(&source_path)? != source_status
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

fn historical_validation_digest(bytes: &[u8]) -> String {
    // Git may check this Markdown record out with CRLF on Windows. Its policy
    // identity pins the repository's LF bytes, so permit only that translation.
    let normalized = bytes
        .split(|byte| *byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .collect::<Vec<_>>();
    sha256_hex(&normalized.join(&b'\n'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn historical_digest_accepts_only_checkout_line_ending_translation() {
        assert_eq!(
            historical_validation_digest(b"first\nsecond\n"),
            historical_validation_digest(b"first\r\nsecond\r\n")
        );
        assert_ne!(
            historical_validation_digest(b"first\nsecond\n"),
            historical_validation_digest(b"first\rsecond\n")
        );
    }

    #[test]
    fn config_rejects_unknown_fields() {
        let value = serde_json::json!({"schema_version":1,"matrix":"m","policy":"p","harness":"h","historical_validation":"v","libevent_source":"s","artifact_sets":[],"report":"o","invented":true});
        assert!(serde_json::from_value::<NativePlatformMatrixConfig>(value).is_err());
    }

    #[test]
    fn configured_all_unknown_gap_report_is_valid() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("audit crate is two levels below the repository")
            .to_path_buf();
        let policy = repository
            .join("providers/qcore-provider-libevent/contracts/monkey/native_platform_matrix.qv");
        let harness = repository.join("crates/qcore-audit/tests/fixtures/native_platform_probe.c");
        let history = repository.join("docs/cycles/CYCLE_18_LIBEVENT_VALIDATION.md");
        let source_repository = repository.join("third_party/libevent");
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "qcore-native-platform-all-unknown-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        // Native builds can leave ignored generated files in the shared submodule.
        // Audit a private checkout of the pinned commit, retaining the production
        // cleanliness gate (including ignored files).
        let source = directory.join("libevent");
        let safe_source = source_repository.to_string_lossy().replace('\\', "/");
        // A linked worktree has a .git file and its own submodule metadata
        // beneath the common repository's worktrees directory. Ask Git for
        // this submodule's actual directory rather than assuming a primary
        // checkout's .git/modules layout.
        let source_git_dir = Command::new("git")
            .args(["-c", &format!("safe.directory={safe_source}")])
            .arg("-C")
            .arg(&source_repository)
            .args(["rev-parse", "--absolute-git-dir"])
            .output()
            .unwrap();
        assert!(source_git_dir.status.success(), "{source_git_dir:?}");
        let safe_git_dir = String::from_utf8(source_git_dir.stdout)
            .unwrap()
            .trim_end_matches(['\r', '\n'])
            .replace('\\', "/");
        let git_config = directory.join("gitconfig");
        fs::write(
            &git_config,
            format!("[safe]\n\tdirectory = {safe_source}\n\tdirectory = {safe_git_dir}\n"),
        )
        .unwrap();
        let clone = Command::new("git")
            // `clone --shared` launches upload-pack as a child process. Pass
            // the two scoped ownership exceptions through a private global
            // config so that child sees protected-scope values too; `-c`
            // values alone do not propagate on Windows.
            .env("GIT_CONFIG_GLOBAL", &git_config)
            .args(["-c", &format!("safe.directory={safe_source}")])
            .args(["-c", &format!("safe.directory={safe_git_dir}")])
            .args(["clone", "--shared", "--no-checkout"])
            .arg(&safe_git_dir)
            .arg(&source)
            .output()
            .unwrap();
        assert!(clone.status.success(), "{clone:?}");
        let checkout = Command::new("git")
            .arg("-C")
            .arg(&source)
            .args(["checkout", "--detach"])
            .arg(native_platform_source_revision().unwrap())
            .output()
            .unwrap();
        assert!(checkout.status.success(), "{checkout:?}");
        let history_digest = historical_validation_digest(&read(&history).unwrap());
        let profiles = [
            (
                "linux_epoll_asan_profile",
                "linux",
                "epoll",
                "address,undefined,leak",
                "libevent_pthreads_initialized",
                "missing_current_epoll_sanitizer_observer_artifact_join",
                Some("linux_full_native_84_of_84_passed"),
            ),
            (
                "linux_poll_asan_profile",
                "linux",
                "poll",
                "address,undefined,leak",
                "libevent_pthreads_initialized",
                "missing_current_poll_sanitizer_observer_artifact_join",
                Some("linux_full_native_84_of_84_passed"),
            ),
            (
                "linux_select_asan_profile",
                "linux",
                "select",
                "address,undefined,leak",
                "libevent_pthreads_initialized",
                "missing_current_select_sanitizer_observer_artifact_join",
                Some("linux_full_native_84_of_84_passed"),
            ),
            (
                "macos_kqueue_asan_profile",
                "macos",
                "kqueue",
                "address,undefined",
                "libevent_pthreads_initialized",
                "missing_current_kqueue_sanitizer_observer_artifact_join",
                Some("macos_kqueue_21_of_21_and_full_65_of_65_passed"),
            ),
            (
                "windows_win32_asan_profile",
                "windows",
                "win32",
                "address",
                "libevent_windows_threads_initialized",
                "missing_current_win32_sanitizer_observer_artifact_join",
                Some("windows_full_native_44_of_44_passed"),
            ),
            (
                "windows_iocp_asan_profile",
                "windows",
                "iocp_profile",
                "address",
                "libevent_windows_threads_initialized",
                "missing_current_iocp_sanitizer_observer_artifact_join",
                Some("windows_iocp_21_of_21_passed"),
            ),
            (
                "linux_epoll_tsan_profile",
                "linux",
                "epoll",
                "thread",
                "libevent_pthreads_initialized",
                "thread_sanitizer_not_executed_for_this_probe",
                None,
            ),
        ];
        let rows = profiles.map(|(policy, host, backend, sanitizers, thread_adapter, reason, historical)| {
            serde_json::json!({
                "policy": policy,
                "dimensions": {"host":host,"backend":backend,"sanitizers":sanitizers,"thread_adapter":thread_adapter,"observer":"bounded_generic_libevent_probe"},
                "status":"unknown","reason":reason,"evidence":null,
                "historical_native_result": historical.map(|recorded_result| serde_json::json!({
                    "source_document_sha256":history_digest,
                    "source_revision":native_platform_source_revision().unwrap(),
                    "recorded_result":recorded_result
                }))
            })
        });
        let matrix_path = directory.join("matrix.json");
        fs::write(
            &matrix_path,
            serde_json::to_vec(&serde_json::json!({
                "schema_version":1,
                "artifact":"libevent/generic-cross-thread-activation",
                "rows":rows
            }))
            .unwrap(),
        )
        .unwrap();
        let config_path = directory.join("config.json");
        fs::write(
            &config_path,
            serde_json::to_vec(&serde_json::json!({
                "schema_version":1,"matrix":matrix_path,"policy":policy,"harness":harness,
                "historical_validation":history,"libevent_source":source,"artifact_sets":[],
                "report":directory.join("report.json")
            }))
            .unwrap(),
        )
        .unwrap();
        run_native_platform_matrix_config(&config_path).unwrap();
        let report: Value =
            serde_json::from_slice(&read(&directory.join("report.json")).unwrap()).unwrap();
        assert_eq!(report["observed_rows"], 0);
        assert_eq!(report["unknown_rows"], 7);
        fs::remove_file(directory.join("report.json")).unwrap();
        for relative in [
            "test/regress.gen.c",
            "qcore-untracked-fixture",
            ".gitignore",
        ] {
            let path = source.join(relative);
            let original = fs::read(&path).ok();
            fs::write(&path, b"dirty source fixture\n").unwrap();
            assert_eq!(
                run_native_platform_matrix_config(&config_path).unwrap_err(),
                "selected Libevent source worktree is not clean"
            );
            assert!(!directory.join("report.json").exists());
            if let Some(bytes) = original {
                fs::write(&path, &bytes).unwrap();
                assert_eq!(fs::read(&path).unwrap(), bytes);
            } else {
                fs::remove_file(&path).unwrap();
            }
        }
        run_native_platform_matrix_config(&config_path).unwrap();
        fs::remove_dir_all(directory).unwrap();
    }
}
