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

//! Configured consumption of bounded Monkey timed-wait failure evidence.

use crate::observed_wait_inputs::load_wait_source;
use qcore_schemas::sha256_hex;
use qcore_verification::monkey_wait_projection::{
    MonkeyWaitContracts, check_wait_failure_paths, project_monkey_timed_wait_source,
    wait_failure_policy_digest,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const EXPECTED_HARNESS: &[u8] =
    include_bytes!("../../../compiler/examples/cycle18_timed_wait_failures.c");
const EXPECTED_HEADER: &[u8] =
    include_bytes!("../../../compiler/examples/cycle18_timed_wait_failures.h");
const EXPECTED_FIXTURE: &[u8] =
    include_bytes!("../../../compiler/tests/fixtures/monkey/wrappers.c");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitFailuresConfig {
    pub schema_version: u32,
    pub artifact: String,
    pub monkey_profile: PathBuf,
    pub fluent_checkout: PathBuf,
    pub monkey_inputs: PathBuf,
    pub source_compile_database: PathBuf,
    pub wrapper_contract: PathBuf,
    pub composition_contract: PathBuf,
    pub failure_policy: PathBuf,
    pub observer_harness: PathBuf,
    pub observer_header: PathBuf,
    pub wrapper_fixture: PathBuf,
    pub observations: PathBuf,
    pub run_manifest: PathBuf,
    pub report: PathBuf,
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn manifest_hash<'a>(manifest: &'a Value, name: &str) -> Result<&'a str, String> {
    manifest
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| format!("manifest has no valid {name}"))
}

fn validate_manifest(
    manifest_bytes: &[u8],
    harness: &[u8],
    header: &[u8],
    fixture: &[u8],
    observations: &[u8],
    source_revision: &str,
) -> Result<String, String> {
    let manifest: Value = serde_json::from_slice(manifest_bytes).map_err(|e| e.to_string())?;
    if manifest.get("schema_version").and_then(Value::as_u64) != Some(1)
        || manifest.get("status").and_then(Value::as_str) != Some("bounded_fault_injection")
        || manifest.get("source_revision").and_then(Value::as_str) != Some(source_revision)
        || manifest
            .pointer("/scope/dependency_behavior")
            .and_then(Value::as_str)
            != Some("stubbed fault model; not a Libevent execution")
        || manifest
            .pointer("/scope/success_is_all_path_safe")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return Err("unsupported timed-wait failure manifest or evidence class".into());
    }
    for (name, bytes) in [
        ("harness_sha256", harness),
        ("harness_header_sha256", header),
        ("wrapper_fixture_sha256", fixture),
        ("observations_sha256", observations),
    ] {
        if manifest_hash(&manifest, name)? != sha256_hex(bytes) {
            return Err(format!("timed-wait failure manifest does not bind {name}"));
        }
    }
    Ok(sha256_hex(manifest_bytes))
}

fn destination(path: &Path, depth: usize) -> Result<PathBuf, String> {
    if depth > 16 {
        return Err("too many symbolic links in report path".into());
    }
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        let target = fs::read_link(path).map_err(|e| e.to_string())?;
        let target = if target.is_absolute() {
            target
        } else {
            path.parent().unwrap_or(Path::new(".")).join(target)
        };
        return destination(&target, depth + 1);
    }
    if path.exists() {
        return path.canonicalize().map_err(|e| e.to_string());
    }
    let parent = path
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let name = path.file_name().ok_or("report path has no file name")?;
    Ok(parent.join(name))
}

#[cfg(unix)]
fn same_file(left: &Path, right: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let (Ok(left), Ok(right)) = (fs::metadata(left), fs::metadata(right)) else {
        return false;
    };
    left.dev() == right.dev() && left.ino() == right.ino()
}
#[cfg(not(unix))]
fn same_file(_: &Path, _: &Path) -> bool {
    false
}

fn reject_aliases(report: &Path, files: &[PathBuf], directories: &[PathBuf]) -> Result<(), String> {
    let target = destination(report, 0)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if fs::metadata(report).is_ok_and(|metadata| metadata.nlink() > 1) {
            return Err("report destination has multiple hard links".into());
        }
    }
    for file in files {
        let canonical = file
            .canonicalize()
            .map_err(|e| format!("{}: {e}", file.display()))?;
        if target == canonical || same_file(report, file) {
            return Err(format!(
                "report destination aliases verified input: {}",
                file.display()
            ));
        }
    }
    for directory in directories {
        let canonical = directory
            .canonicalize()
            .map_err(|e| format!("{}: {e}", directory.display()))?;
        if target.starts_with(canonical) {
            return Err(format!(
                "report destination is inside verified input directory: {}",
                directory.display()
            ));
        }
    }
    Ok(())
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or("report path has no file name")?
        .to_string_lossy();
    let temporary = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("{}: {e}", temporary.display()))?;
    let result = (|| {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn snapshots_unchanged(snapshots: &[(PathBuf, Vec<u8>)]) -> bool {
    snapshots
        .iter()
        .all(|(selected, snapshot)| read(selected).as_deref() == Ok(snapshot.as_slice()))
}

pub fn run_wait_failures_config(path: &Path) -> Result<(), String> {
    prepare_wait_failures(path)?.publish()
}

pub(crate) fn prepare_wait_failures(
    path: &Path,
) -> Result<crate::cycle18_inputs::PreparedAudit<crate::cycle18_inputs::Failures>, String> {
    let config_bytes = read(path)?;
    let config: WaitFailuresConfig =
        serde_json::from_slice(&config_bytes).map_err(|e| e.to_string())?;
    if config.schema_version != 1 || config.artifact.is_empty() {
        return Err("unsupported wait-failures configuration".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let resolve = |selected: &Path| crate::resolve(base, selected);
    let selected = [
        resolve(&config.wrapper_contract),
        resolve(&config.composition_contract),
        resolve(&config.failure_policy),
        resolve(&config.observer_harness),
        resolve(&config.observer_header),
        resolve(&config.wrapper_fixture),
        resolve(&config.observations),
        resolve(&config.run_manifest),
    ];
    let snapshots = selected
        .iter()
        .map(|selected| Ok((selected.clone(), read(selected)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let bytes = |index: usize| snapshots[index].1.as_slice();
    if bytes(3) != EXPECTED_HARNESS || bytes(4) != EXPECTED_HEADER || bytes(5) != EXPECTED_FIXTURE {
        return Err(
            "configured harness, header or wrapper fixture differs from audited repository bytes"
                .into(),
        );
    }
    if sha256_hex(bytes(2)) != wait_failure_policy_digest() {
        return Err("configured failure policy differs from executable bundled policy".into());
    }
    let source = load_wait_source(
        &resolve(&config.monkey_profile),
        &resolve(&config.fluent_checkout),
        &resolve(&config.monkey_inputs),
        &resolve(&config.source_compile_database),
    )?;
    let aggregate_identity = crate::cycle18_inputs::SourceIdentity::from_source(&source)?;
    let compile_digest = source.compile_task.digest()?;
    let source_snapshot = (source.source_sha256.clone(), source.build_sha256.clone());
    let contracts = MonkeyWaitContracts::parse(
        std::str::from_utf8(bytes(0)).map_err(|e| e.to_string())?,
        std::str::from_utf8(bytes(1)).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("source contracts refused: {e:?}"))?;
    let projection = project_monkey_timed_wait_source(&source.summaries, &contracts)
        .map_err(|e| format!("source projection refused: {e:?}"))?;
    let manifest_digest = validate_manifest(
        bytes(7),
        bytes(3),
        bytes(4),
        bytes(5),
        bytes(6),
        &source.revision,
    )?;
    let cases = check_wait_failure_paths(&projection, bytes(6))
        .map_err(|e| format!("failure-path evidence refused: {e:?}"))?;

    let mut protected_files = source.protected_files.clone();
    protected_files.push(path.to_path_buf());
    protected_files.extend(selected.iter().cloned());
    let report = resolve(&config.report);
    reject_aliases(&report, &protected_files, &source.protected_directories)?;

    let reloaded = load_wait_source(
        &resolve(&config.monkey_profile),
        &resolve(&config.fluent_checkout),
        &resolve(&config.monkey_inputs),
        &resolve(&config.source_compile_database),
    )?;
    if reloaded.compile_task.digest()? != compile_digest
        || (reloaded.source_sha256, reloaded.build_sha256) != source_snapshot
        || read(path)? != config_bytes
        || !snapshots_unchanged(&snapshots)
    {
        return Err("source or selected inputs changed during wait-failures audit".into());
    }
    let report_value = serde_json::json!({
        "schema_version": 1, "status": "timed_wait_failure_paths_checked",
        "evidence_class": "instrumented_source_model_not_real_libevent_execution",
        "artifact": config.artifact, "source_revision": source.revision,
        "source_compile_task_sha256": compile_digest, "source_sha256": source.source_sha256,
        "source_build_sha256": source.build_sha256, "manifest_sha256": manifest_digest,
        "failure_policy_sha256": wait_failure_policy_digest(), "cases": cases,
        "limitations": ["Stubbed dependency fault outcomes, not a real Libevent execution",
            "Unknown rows remain in the denominator", "No callback or aggregate Cycle 18 promotion"]
    });
    let monkey_revision = aggregate_identity.revision.clone();
    crate::cycle18_inputs::PreparedAudit::new(
        aggregate_identity,
        crate::cycle18_inputs::Failures {
            projection,
            observations: bytes(6).to_vec(),
        },
        report,
        serde_json::to_vec_pretty(&report_value).map_err(|e| e.to_string())?,
        protected_files,
        source.protected_directories,
    )
    .map(|prepared| {
        prepared.with_checkout_revision(resolve(&config.fluent_checkout), monkey_revision)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "qcore-wait-failures-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn manifest(harness: &[u8], header: &[u8], fixture: &[u8], observations: &[u8]) -> Vec<u8> {
        serde_json::to_vec(&json!({"schema_version":1,"status":"bounded_fault_injection",
            "source_revision":"revision","harness_sha256":sha256_hex(harness),
            "harness_header_sha256":sha256_hex(header),
            "wrapper_fixture_sha256":sha256_hex(fixture),"observations_sha256":sha256_hex(observations),
            "scope":{"dependency_behavior":"stubbed fault model; not a Libevent execution",
                "success_is_all_path_safe":false}})).unwrap()
    }

    #[test]
    fn manifest_binds_every_selected_artifact_and_evidence_class() {
        let (harness, header, fixture, observations) = (
            b"h".as_slice(),
            b"d".as_slice(),
            b"f".as_slice(),
            b"o".as_slice(),
        );
        let baseline = manifest(harness, header, fixture, observations);
        assert!(
            validate_manifest(
                &baseline,
                harness,
                header,
                fixture,
                observations,
                "revision"
            )
            .is_ok()
        );
        assert!(
            validate_manifest(
                &baseline,
                b"changed",
                header,
                fixture,
                observations,
                "revision"
            )
            .is_err()
        );
        assert!(
            validate_manifest(
                &baseline,
                harness,
                b"changed",
                fixture,
                observations,
                "revision"
            )
            .is_err()
        );
        assert!(
            validate_manifest(
                &baseline,
                harness,
                header,
                b"changed",
                observations,
                "revision"
            )
            .is_err()
        );
        assert!(
            validate_manifest(&baseline, harness, header, fixture, b"changed", "revision").is_err()
        );
        assert!(
            validate_manifest(&baseline, harness, header, fixture, observations, "changed")
                .is_err()
        );
        let mut changed: Value = serde_json::from_slice(&baseline).unwrap();
        changed["scope"]["dependency_behavior"] = "real libevent".into();
        assert!(
            validate_manifest(
                &serde_json::to_vec(&changed).unwrap(),
                harness,
                header,
                fixture,
                observations,
                "revision"
            )
            .is_err()
        );
    }

    #[test]
    fn snapshots_are_compared_by_bytes_before_publication() {
        let root = directory();
        let input = root.join("input");
        fs::write(&input, b"first").unwrap();
        let snapshots = vec![(input.clone(), b"first".to_vec())];
        assert!(snapshots_unchanged(&snapshots));
        fs::write(&input, b"second").unwrap();
        assert!(!snapshots_unchanged(&snapshots));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn report_write_is_atomic_and_direct_aliases_refuse() {
        let root = directory();
        let input = root.join("input");
        let report = root.join("report.json");
        fs::write(&input, b"protected").unwrap();
        assert!(reject_aliases(&report, std::slice::from_ref(&input), &[]).is_ok());
        assert!(reject_aliases(&input, std::slice::from_ref(&input), &[]).is_err());
        write_atomic(&report, b"report").unwrap();
        assert_eq!(fs::read(&report).unwrap(), b"report");
        assert_eq!(fs::read(&input).unwrap(), b"protected");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_and_hard_link_report_aliases_refuse() {
        use std::os::unix::fs::symlink;
        let root = directory();
        let input = root.join("input");
        let symbolic = root.join("symbolic");
        let hard = root.join("hard");
        fs::write(&input, b"protected").unwrap();
        symlink(&input, &symbolic).unwrap();
        fs::hard_link(&input, &hard).unwrap();
        assert!(reject_aliases(&symbolic, std::slice::from_ref(&input), &[]).is_err());
        assert!(reject_aliases(&hard, std::slice::from_ref(&input), &[]).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
