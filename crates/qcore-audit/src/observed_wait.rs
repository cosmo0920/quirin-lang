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

//! Configured audit of one complete native timed-wait observation.
use crate::observed_wait_inputs::load_wait_source;
use qcore_contract::profile_loader::{load_audited_bundle, load_profile_directory, verify_pin};
use qcore_schemas::sha256_hex;
use qcore_verification::event_composition_context::{
    DeclaredObservationTrust, EventCompositionContext, EventCompositionContextInput,
    EventCompositionPayload, check_event_composition_context_for,
};
use qcore_verification::monkey_wait_projection::{
    MonkeyWaitContracts, project_monkey_timed_wait_source,
};
use qcore_verification::observed_wait_composition::{
    check_observed_wait_composition, observed_wait_bounds, wait_observation_rules_digest,
};
use qcore_verification::timed_wait_observation_wire::decode_timed_wait_observation;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedWaitConfig {
    pub schema_version: u32,
    pub artifact: String,
    pub monkey_profile: PathBuf,
    pub fluent_checkout: PathBuf,
    pub monkey_inputs: PathBuf,
    pub source_compile_database: PathBuf,
    pub libevent_profile: PathBuf,
    pub libevent_checkout: PathBuf,
    pub libevent_build: PathBuf,
    pub wrapper_contract: PathBuf,
    pub composition_contract: PathBuf,
    pub observations: PathBuf,
    pub bindings: PathBuf,
    pub observer_binary: PathBuf,
    pub observer_harness: PathBuf,
    pub monkey_object: PathBuf,
    pub libevent_library: PathBuf,
    pub run_manifest: PathBuf,
    pub report: PathBuf,
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn report_destination(path: &Path) -> Result<PathBuf, String> {
    report_destination_with_depth(path, 0)
}

fn report_destination_with_depth(path: &Path, symlink_depth: usize) -> Result<PathBuf, String> {
    if symlink_depth > 16 {
        return Err(format!(
            "too many symbolic links in report path: {}",
            path.display()
        ));
    }
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        let target = fs::read_link(path)
            .map_err(|error| format!("cannot read report symlink {}: {error}", path.display()))?;
        let target = if target.is_absolute() {
            target
        } else {
            path.parent().unwrap_or(Path::new(".")).join(target)
        };
        return report_destination_with_depth(&target, symlink_depth + 1);
    }
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| format!("cannot resolve report path {}: {error}", path.display()));
    }
    let name = path
        .file_name()
        .ok_or_else(|| format!("report path has no file name: {}", path.display()))?;
    let parent = path.parent().unwrap_or(Path::new("."));
    parent
        .canonicalize()
        .map(|parent| parent.join(name))
        .map_err(|error| format!("cannot resolve report parent {}: {error}", parent.display()))
}

#[cfg(unix)]
fn same_existing_file(left: &Path, right: &Path) -> bool {
    let (Ok(left), Ok(right)) = (fs::metadata(left), fs::metadata(right)) else {
        return false;
    };
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_existing_file(_left: &Path, _right: &Path) -> bool {
    false
}

#[cfg(unix)]
fn has_multiple_hard_links(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(path).is_ok_and(|metadata| metadata.nlink() > 1)
}

#[cfg(not(unix))]
fn has_multiple_hard_links(_path: &Path) -> bool {
    false
}

/// Refuse a report destination that can overwrite any evidence used by the
/// audit. Canonical paths catch lexical and symbolic-link aliases; Unix file
/// identity additionally catches hard links to individually selected inputs.
pub(crate) fn reject_report_aliases(
    report: &Path,
    protected_files: &[PathBuf],
    protected_directories: &[PathBuf],
) -> Result<(), String> {
    let destination = report_destination(report)?;
    if has_multiple_hard_links(report) {
        return Err("report destination has multiple hard links".into());
    }
    for input in protected_files {
        let canonical = input.canonicalize().map_err(|error| {
            format!(
                "cannot resolve protected input {}: {error}",
                input.display()
            )
        })?;
        if destination == canonical || same_existing_file(report, input) {
            return Err(format!(
                "report destination aliases verified input: {}",
                input.display()
            ));
        }
    }
    for directory in protected_directories {
        let canonical = directory.canonicalize().map_err(|error| {
            format!(
                "cannot resolve protected directory {}: {error}",
                directory.display()
            )
        })?;
        if destination.starts_with(&canonical) {
            return Err(format!(
                "report destination is inside verified input directory: {}",
                directory.display()
            ));
        }
    }
    Ok(())
}

/// Refuse two report destinations that resolve to the same planned path or
/// existing file. Unlike protected evidence, either report may not exist yet.
pub(crate) fn reject_report_destination_alias(
    report: &Path,
    protected_report: &Path,
) -> Result<(), String> {
    let destination = report_destination(report)?;
    let protected_destination = report_destination(protected_report)?;
    if destination == protected_destination || same_existing_file(report, protected_report) {
        return Err(format!(
            "aggregate report aliases prepared leaf report: {}",
            protected_report.display()
        ));
    }
    Ok(())
}

/// The caller selects the artifacts. The report explicitly retains the trust
/// premise that their producer accurately recorded execution and linkage.
/// Hashes identify the selected artifacts; they do not attest that premise.
pub fn run_observed_wait_config(path: &Path) -> Result<(), String> {
    prepare_observed_wait(path)?.publish()
}

pub(crate) fn prepare_observed_wait(
    path: &Path,
) -> Result<crate::cycle18_inputs::PreparedAudit<crate::cycle18_inputs::ObservedWait>, String> {
    let config: ObservedWaitConfig =
        serde_json::from_slice(&read(path)?).map_err(|error| error.to_string())?;
    if config.schema_version != 1 {
        return Err("unsupported observed-wait configuration".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let resolve = |path: &Path| crate::resolve(base, path);
    let manifest_bytes = read(&resolve(&config.run_manifest))?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    if manifest["schema_version"] != 1 || manifest["status"] != "observed_success" {
        return Err("unsupported native run manifest".into());
    }
    let mut snapshots = Vec::new();
    for (section, field, configured) in [
        ("monkey", "inputs", &config.monkey_inputs),
        (
            "monkey",
            "source_compile_commands",
            &config.source_compile_database,
        ),
        ("observer", "executable", &config.observer_binary),
        ("observer", "source", &config.observer_harness),
        ("observer", "object", &config.monkey_object),
        ("libevent", "archive", &config.libevent_library),
        ("outputs", "observations", &config.observations),
        ("outputs", "bindings", &config.bindings),
    ] {
        let selected = resolve(configured);
        let bytes = read(&selected)?;
        if manifest[section][format!("{field}_sha256")].as_str()
            != Some(sha256_hex(&bytes).as_str())
        {
            return Err(format!(
                "native manifest does not bind selected {section}.{field}"
            ));
        }
        snapshots.push((selected, bytes));
    }
    let source = load_wait_source(
        &resolve(&config.monkey_profile),
        &resolve(&config.fluent_checkout),
        &resolve(&config.monkey_inputs),
        &resolve(&config.source_compile_database),
    )?;
    let aggregate_identity = crate::cycle18_inputs::SourceIdentity::from_source(&source)?;
    let mut protected_files = source.protected_files.clone();
    protected_files.extend([
        path.to_path_buf(),
        resolve(&config.wrapper_contract),
        resolve(&config.composition_contract),
        resolve(&config.observations),
        resolve(&config.bindings),
        resolve(&config.observer_binary),
        resolve(&config.observer_harness),
        resolve(&config.monkey_object),
        resolve(&config.libevent_library),
        resolve(&config.run_manifest),
    ]);
    let mut protected_directories = source.protected_directories.clone();
    protected_directories.extend([
        resolve(&config.libevent_profile),
        resolve(&config.libevent_checkout),
        resolve(&config.libevent_build),
    ]);
    let compile_task_digest = source.compile_task.digest()?;
    let source_snapshot = (source.source_sha256.clone(), source.build_sha256.clone());
    let wrapper_bytes = read(&resolve(&config.wrapper_contract))?;
    let composition_bytes = read(&resolve(&config.composition_contract))?;
    let contracts = MonkeyWaitContracts::parse(
        std::str::from_utf8(&wrapper_bytes).map_err(|e| e.to_string())?,
        std::str::from_utf8(&composition_bytes).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("source contracts refused: {e:?}"))?;
    let projection = project_monkey_timed_wait_source(&source.summaries, &contracts)
        .map_err(|e| format!("source projection refused: {e:?}"))?;
    let libevent =
        load_profile_directory(&resolve(&config.libevent_profile)).map_err(|e| e.to_string())?;
    let library_profile_path = libevent.directory().join("profile.json");
    protected_files.push(library_profile_path.clone());
    snapshots.push((
        library_profile_path,
        libevent.manifest_text().as_bytes().to_vec(),
    ));
    let library = libevent
        .library("libevent")
        .ok_or("missing Libevent library profile")?;
    let checkout = resolve(&config.libevent_checkout);
    let revision = verify_pin(
        "libevent",
        libevent
            .manifest()
            .pin_for("libevent")
            .ok_or("missing Libevent pin")?,
        &checkout,
    )
    .map_err(|e| e.to_string())?;
    if manifest["libevent"]["source_revision"].as_str() != Some(&revision)
        || manifest["monkey"]["source_revision"].as_str() != Some(&source.revision)
    {
        return Err("native manifest source revision mismatch".into());
    }
    let bundle = load_audited_bundle(library, &checkout, &revision)
        .map_err(|e| format!("Libevent source identity refused: {e:?}"))?;
    protected_files.extend(bundle.files.keys().map(|file| checkout.join(file)));
    snapshots.extend(
        bundle
            .files
            .iter()
            .map(|(file, contents)| (checkout.join(file), contents.as_bytes().to_vec())),
    );
    let build = resolve(&config.libevent_build);
    let cache = read(&build.join("CMakeCache.txt"))?;
    protected_files.push(build.join("CMakeCache.txt"));
    snapshots.push((build.join("CMakeCache.txt"), cache.clone()));
    if manifest["libevent"]["cmake_cache_sha256"].as_str() != Some(sha256_hex(&cache).as_str()) {
        return Err("native manifest build configuration mismatch".into());
    }
    let cache_text = std::str::from_utf8(&cache).map_err(|e| e.to_string())?;
    let entries: BTreeMap<_, _> = cache_text
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let (key, _) = key.split_once(':')?;
            Some((key, value))
        })
        .collect();
    if entries.get("CMAKE_HOME_DIRECTORY").copied() != checkout.to_str()
        || entries.get("EVENT__DISABLE_OPENSSL").copied() != Some("ON")
        || entries.get("EVENT__DISABLE_THREAD_SUPPORT").copied() != Some("OFF")
    {
        return Err(
            "Libevent build does not select pinned source, pthreads and disabled OpenSSL".into(),
        );
    }
    let observations = read(&resolve(&config.observations))?;
    let bindings = read(&resolve(&config.bindings))?;
    let decoded = decode_timed_wait_observation(&observations, &bindings)?;
    let observed_bounds = observed_wait_bounds(&decoded.trace);
    let semantic_profile = decoded.semantic_profile;
    let backend = decoded.context.backend;
    if !library.accepted_builds.iter().any(|candidate| {
        candidate.backend == backend
            && candidate.threads == "pthreads"
            && candidate.forbid.iter().all(|feature| feature == "openssl")
    }) {
        return Err("observed backend/build is outside the accepted Libevent profile".into());
    }
    let mut assumptions = source.profile.assumptions();
    // These labels describe this producer/build's actual evidence class.
    // Never fill observed facts from the policy's desired values: doing so
    // would manufacture agreement when a contract asks for stronger trust.
    assumptions.insert("backend".into(), backend);
    assumptions.insert("threads".into(), "pthreads".into());
    assumptions.insert(
        "observation_trust".into(),
        "producer_supplied_runtime_evidence".into(),
    );
    assumptions.insert(
        "linkage_trust".into(),
        "producer_supplied_selected_artifact_linkage".into(),
    );
    assumptions.insert(
        "native_run_manifest_sha256".into(),
        sha256_hex(&manifest_bytes),
    );
    for (name, digest) in projection.source_summary_digests() {
        assumptions.insert(format!("source_summary.{name}"), digest.clone());
    }
    for (name, digest) in projection.contract_digests() {
        assumptions.insert(format!("contract.{name}"), digest.clone());
    }
    for name in ["mk_core/mk_event.c", "mk_core/mk_event_libevent.c"] {
        assumptions.insert(
            format!("source_file.{name}"),
            source.source_files[name].clone(),
        );
    }
    let payload = EventCompositionPayload {
        observations: &observations,
        bindings: &bindings,
    };
    let context = EventCompositionContext::derive(
        EventCompositionContextInput {
            artifact: config.artifact,
            semantic_profile,
            scope: projection.selected_path_indices().keys().cloned().collect(),
            bounds: observed_bounds,
            assumptions,
            source_compile_task_sha256: compile_task_digest.clone(),
            fluent_bit_revision: source.revision,
            monkey_source_sha256: source.source_sha256,
            monkey_profile_sha256: source.profile.manifest_digest().into(),
            monkey_build_sha256: source.build_sha256,
            libevent_revision: revision.clone(),
            libevent_source_sha256: sha256_hex(
                &serde_json::to_vec(&bundle.files).map_err(|e| e.to_string())?,
            ),
            libevent_profile_sha256: libevent.manifest_digest().into(),
            libevent_build_sha256: sha256_hex(&cache),
            contracts_sha256: sha256_hex(
                &serde_json::to_vec(projection.contract_digests()).map_err(|e| e.to_string())?,
            ),
            source_rules_sha256: sha256_hex(
                qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES.as_bytes(),
            ),
            observation_rules_sha256: wait_observation_rules_digest(),
            observer_binary_sha256: sha256_hex(&read(&resolve(&config.observer_binary))?),
            observer_harness_sha256: sha256_hex(&read(&resolve(&config.observer_harness))?),
            linked_libraries_sha256: BTreeMap::from([
                (
                    "monkey".into(),
                    sha256_hex(&read(&resolve(&config.monkey_object))?),
                ),
                (
                    "libevent".into(),
                    sha256_hex(&read(&resolve(&config.libevent_library))?),
                ),
            ]),
            observation_trust: DeclaredObservationTrust::ProducerSuppliedRuntimeEvidence,
        },
        payload,
    )
    .map_err(|e| e.to_string())?;
    // This audit produces a fresh context from configured files; it does not
    // import or authenticate a producer's certificate context.
    let admitted = check_event_composition_context_for(&context, payload, &context)
        .map_err(|e| e.to_string())?;
    let checked =
        check_observed_wait_composition(&projection, &admitted, &observations, &bindings)?;
    let reloaded = load_wait_source(
        &resolve(&config.monkey_profile),
        &resolve(&config.fluent_checkout),
        &resolve(&config.monkey_inputs),
        &resolve(&config.source_compile_database),
    )?;
    if reloaded.compile_task.digest()? != compile_task_digest
        || (reloaded.source_sha256, reloaded.build_sha256) != source_snapshot
        || read(&resolve(&config.wrapper_contract))? != wrapper_bytes
        || read(&resolve(&config.composition_contract))? != composition_bytes
        || read(&build.join("CMakeCache.txt"))? != cache
        || read(&resolve(&config.run_manifest))? != manifest_bytes
    {
        return Err("source, contracts or build inputs changed during audit".into());
    }
    for (selected, bytes) in snapshots {
        if read(&selected)? != bytes {
            return Err(format!(
                "input changed during audit: {}",
                selected.display()
            ));
        }
    }
    reject_report_aliases(
        &resolve(&config.report),
        &protected_files,
        &protected_directories,
    )?;
    let report = serde_json::json!({
        "schema_version": 1, "status": "observed_composition_checked",
        "context": context, "composition": checked,
        "limitations": ["Producer-supplied runtime and linkage evidence, not attestation",
            "One observed success path; unobserved failure paths remain Unknown",
            "No callback coverage or Cycle 18 acceptance promotion"]
    });
    let monkey_revision = aggregate_identity.revision.clone();
    crate::cycle18_inputs::PreparedAudit::new(
        aggregate_identity,
        crate::cycle18_inputs::ObservedWait {
            projection,
            context: admitted,
            observations,
            bindings,
            summaries: source.summaries,
        },
        resolve(&config.report),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        protected_files,
        protected_directories,
    )
    .map(|prepared| {
        prepared
            .with_checkout_revision(resolve(&config.fluent_checkout), monkey_revision)
            .with_checkout_revision(checkout, revision)
    })
}

#[cfg(test)]
mod tests {
    use super::{reject_report_aliases, reject_report_destination_alias};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "qcore-observed-wait-report-alias-{}-{}",
                std::process::id(),
                NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn distinct_report_destination_is_accepted() {
        let scratch = Scratch::new();
        let input = scratch.path().join("input.json");
        let protected = scratch.path().join("checkout");
        let output = scratch.path().join("report.json");
        fs::write(&input, b"input").unwrap();
        fs::create_dir(&protected).unwrap();
        assert!(reject_report_aliases(&output, &[input], &[protected]).is_ok());
    }

    #[test]
    fn exact_input_and_protected_directory_destinations_are_refused() {
        let scratch = Scratch::new();
        let input = scratch.path().join("observations.jsonl");
        let protected = scratch.path().join("checkout");
        fs::write(&input, b"input").unwrap();
        fs::create_dir(&protected).unwrap();
        assert!(reject_report_aliases(&input, std::slice::from_ref(&input), &[]).is_err());
        assert!(
            reject_report_aliases(
                &protected.join("new-report.json"),
                &[],
                std::slice::from_ref(&protected),
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_and_hard_link_aliases_are_refused() {
        use std::os::unix::fs::symlink;

        let scratch = Scratch::new();
        let input = scratch.path().join("bindings.json");
        let symbolic = scratch.path().join("symbolic-report.json");
        let dangling_symbolic = scratch.path().join("dangling-symbolic-report.json");
        let hard = scratch.path().join("hard-report.json");
        let protected = scratch.path().join("checkout");
        fs::write(&input, b"input").unwrap();
        fs::create_dir(&protected).unwrap();
        symlink(&input, &symbolic).unwrap();
        symlink(protected.join("future-report.json"), &dangling_symbolic).unwrap();
        fs::hard_link(&input, &hard).unwrap();
        assert!(reject_report_aliases(&symbolic, std::slice::from_ref(&input), &[]).is_err());
        assert!(reject_report_aliases(&hard, std::slice::from_ref(&input), &[]).is_err());
        assert!(
            reject_report_aliases(&dangling_symbolic, &[], std::slice::from_ref(&protected),)
                .is_err()
        );
    }

    #[test]
    fn planned_leaf_report_alias_is_refused_before_either_report_exists() {
        let scratch = Scratch::new();
        let report = scratch.path().join("report.json");
        assert!(reject_report_destination_alias(&report, &report).is_err());
        assert!(!report.exists());
    }

    #[cfg(unix)]
    #[test]
    fn existing_hard_linked_leaf_report_alias_is_refused() {
        let scratch = Scratch::new();
        let leaf = scratch.path().join("leaf.json");
        let aggregate = scratch.path().join("aggregate.json");
        fs::write(&leaf, b"leaf").unwrap();
        fs::hard_link(&leaf, &aggregate).unwrap();
        assert!(reject_report_destination_alias(&aggregate, &leaf).is_err());
    }
}
