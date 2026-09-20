//! Configured audit of one complete native timed-wait observation.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use qcore_contract::profile_loader::{load_audited_bundle, load_profile_directory, verify_pin};
use qcore_schemas::sha256_hex;
use qcore_verification::event_composition_context::{DeclaredObservationTrust,
    EventCompositionContext, EventCompositionContextInput, EventCompositionPayload,
    check_event_composition_context_for};
use qcore_verification::monkey_wait_projection::{MonkeyWaitContracts, project_monkey_timed_wait_source};
use qcore_verification::observed_wait_composition::{OBSERVED_WAIT_PROFILE, OBSERVED_WAIT_RULES,
    check_observed_wait_composition};
use qcore_verification::timed_wait_observation_wire::decode_timed_wait_observation;
use serde::Deserialize;
use crate::observed_wait_inputs::load_wait_source;

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

/// The caller selects the artifacts. The report explicitly retains the trust
/// premise that their producer accurately recorded execution and linkage.
/// Hashes identify the selected artifacts; they do not attest that premise.
pub fn run_observed_wait_config(path: &Path) -> Result<(), String> {
    let config: ObservedWaitConfig = serde_json::from_slice(&read(path)?)
        .map_err(|error| error.to_string())?;
    if config.schema_version != 1 { return Err("unsupported observed-wait configuration".into()); }
    let base = path.parent().unwrap_or(Path::new("."));
    let resolve = |path: &Path| crate::resolve(base, path);
    let manifest_bytes = read(&resolve(&config.run_manifest))?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    if manifest["schema_version"] != 1 || manifest["status"] != "observed_success" {
        return Err("unsupported native run manifest".into());
    }
    let mut snapshots = Vec::new();
    for (section, field, configured) in [
        ("monkey", "inputs", &config.monkey_inputs),
        ("monkey", "source_compile_commands", &config.source_compile_database),
        ("observer", "executable", &config.observer_binary),
        ("observer", "source", &config.observer_harness),
        ("observer", "object", &config.monkey_object),
        ("libevent", "archive", &config.libevent_library),
        ("outputs", "observations", &config.observations),
        ("outputs", "bindings", &config.bindings),
    ] {
        let selected = resolve(configured);
        let bytes = read(&selected)?;
        if manifest[section][format!("{field}_sha256")].as_str() != Some(sha256_hex(&bytes).as_str()) {
            return Err(format!("native manifest does not bind selected {section}.{field}"));
        }
        snapshots.push((selected, bytes));
    }
    let source = load_wait_source(&resolve(&config.monkey_profile),
        &resolve(&config.fluent_checkout), &resolve(&config.monkey_inputs),
        &resolve(&config.source_compile_database))?;
    let wrapper_bytes = read(&resolve(&config.wrapper_contract))?;
    let composition_bytes = read(&resolve(&config.composition_contract))?;
    let contracts = MonkeyWaitContracts::parse(
        std::str::from_utf8(&wrapper_bytes).map_err(|e| e.to_string())?,
        std::str::from_utf8(&composition_bytes).map_err(|e| e.to_string())?,
    ).map_err(|e| format!("source contracts refused: {e:?}"))?;
    let projection = project_monkey_timed_wait_source(&source.summaries, &contracts)
        .map_err(|e| format!("source projection refused: {e:?}"))?;
    let libevent = load_profile_directory(&resolve(&config.libevent_profile))
        .map_err(|e| e.to_string())?;
    let library = libevent.library("libevent").ok_or("missing Libevent library profile")?;
    let checkout = resolve(&config.libevent_checkout);
    let revision = verify_pin("libevent", libevent.manifest().pin_for("libevent")
        .ok_or("missing Libevent pin")?, &checkout).map_err(|e| e.to_string())?;
    if manifest["libevent"]["source_revision"].as_str() != Some(&revision)
        || manifest["monkey"]["source_revision"].as_str() != Some(&source.revision)
    { return Err("native manifest source revision mismatch".into()); }
    let bundle = load_audited_bundle(library, &checkout, &revision)
        .map_err(|e| format!("Libevent source identity refused: {e:?}"))?;
    let build = resolve(&config.libevent_build);
    let cache = read(&build.join("CMakeCache.txt"))?;
    if manifest["libevent"]["cmake_cache_sha256"].as_str() != Some(sha256_hex(&cache).as_str()) {
        return Err("native manifest build configuration mismatch".into());
    }
    let cache_text = std::str::from_utf8(&cache).map_err(|e| e.to_string())?;
    let entries: BTreeMap<_, _> = cache_text.lines().filter_map(|line| {
        let (key, value) = line.split_once('=')?;
        let (key, _) = key.split_once(':')?;
        Some((key, value))
    }).collect();
    if entries.get("CMAKE_HOME_DIRECTORY").copied() != checkout.to_str()
        || entries.get("EVENT__DISABLE_OPENSSL").copied() != Some("ON")
        || entries.get("EVENT__DISABLE_THREAD_SUPPORT").copied() != Some("OFF")
    { return Err("Libevent build does not select pinned source, pthreads and disabled OpenSSL".into()); }
    let observations = read(&resolve(&config.observations))?;
    let bindings = read(&resolve(&config.bindings))?;
    let decoded = decode_timed_wait_observation(&observations, &bindings)?;
    let backend = decoded.context.backend;
    if !library.accepted_builds.iter().any(|candidate|
        candidate.backend == backend && candidate.threads == "pthreads"
            && candidate.forbid.iter().all(|feature| feature == "openssl"))
    { return Err("observed backend/build is outside the accepted Libevent profile".into()); }
    let mut assumptions = source.profile.assumptions();
    assumptions.insert("backend".into(), backend);
    assumptions.insert("threads".into(), "pthreads".into());
    assumptions.insert("observation_trust".into(), "producer_supplied_runtime_evidence".into());
    assumptions.insert("linkage_trust".into(), "producer_supplied_selected_artifact_linkage".into());
    assumptions.insert("native_run_manifest_sha256".into(), sha256_hex(&manifest_bytes));
    for (name, digest) in projection.source_summary_digests() {
        assumptions.insert(format!("source_summary.{name}"), digest.clone());
    }
    for (name, digest) in projection.contract_digests() {
        assumptions.insert(format!("contract.{name}"), digest.clone());
    }
    for name in ["mk_core/mk_event.c", "mk_core/mk_event_libevent.c"] {
        assumptions.insert(format!("source_file.{name}"), source.source_files[name].clone());
    }
    let payload = EventCompositionPayload { observations: &observations, bindings: &bindings };
    let context = EventCompositionContext::derive(EventCompositionContextInput {
        artifact: config.artifact, semantic_profile: OBSERVED_WAIT_PROFILE.into(),
        scope: projection.selected_path_indices().keys().cloned().collect(),
        bounds: BTreeMap::from([("dispatchers".into(), 1), ("events".into(), 1),
            ("records".into(), 11), ("runs".into(), 1)]), assumptions,
        source_compile_task_sha256: source.compile_task.digest()?,
        fluent_bit_revision: source.revision,
        monkey_source_sha256: source.source_sha256,
        monkey_profile_sha256: source.profile.manifest_digest().into(),
        monkey_build_sha256: source.build_sha256,
        libevent_revision: revision,
        libevent_source_sha256: sha256_hex(&serde_json::to_vec(&bundle.files).map_err(|e| e.to_string())?),
        libevent_profile_sha256: libevent.manifest_digest().into(),
        libevent_build_sha256: sha256_hex(&cache),
        contracts_sha256: sha256_hex(&serde_json::to_vec(projection.contract_digests()).map_err(|e| e.to_string())?),
        source_rules_sha256: sha256_hex(qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES.as_bytes()),
        observation_rules_sha256: sha256_hex(OBSERVED_WAIT_RULES.as_bytes()),
        observer_binary_sha256: sha256_hex(&read(&resolve(&config.observer_binary))?),
        observer_harness_sha256: sha256_hex(&read(&resolve(&config.observer_harness))?),
        linked_libraries_sha256: BTreeMap::from([
            ("monkey".into(), sha256_hex(&read(&resolve(&config.monkey_object))?)),
            ("libevent".into(), sha256_hex(&read(&resolve(&config.libevent_library))?)),
        ]), observation_trust: DeclaredObservationTrust::ProducerSuppliedRuntimeEvidence,
    }, payload).map_err(|e| e.to_string())?;
    // This audit produces a fresh context from configured files; it does not
    // import or authenticate a producer's certificate context.
    let admitted = check_event_composition_context_for(&context, payload, &context)
        .map_err(|e| e.to_string())?;
    let checked = check_observed_wait_composition(&projection, &admitted, &observations, &bindings)?;
    for (selected, bytes) in snapshots {
        if read(&selected)? != bytes { return Err(format!("input changed during audit: {}", selected.display())); }
    }
    let report = serde_json::json!({
        "schema_version": 1, "status": "observed_composition_checked",
        "context": context, "composition": checked,
        "limitations": ["Producer-supplied runtime and linkage evidence, not attestation",
            "One observed success path; unobserved failure paths remain Unknown",
            "No callback coverage or Cycle 18 acceptance promotion"]
    });
    fs::write(resolve(&config.report), serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
