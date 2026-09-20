//! Configured, bounded cb_event payload checking over pinned native inputs.
use crate::observed_wait::reject_report_aliases;
use crate::observed_wait_inputs::load_monkey_source;
use qcore_contract::profile_loader::{load_audited_bundle, load_profile_directory, verify_pin};
use qcore_core::event_reactor::{ActivationReason, ArgumentMode, CallbackSummary, PayloadAccessKind};
use qcore_schemas::sha256_hex;
use qcore_verification::event_composition_context::{DeclaredObservationTrust, EventCompositionContext, EventCompositionContextInput, EventCompositionPayload, check_event_composition_context_for};
use qcore_verification::event_payload::{EVENT_PAYLOAD_CONTRACT, EventPayloadBoundary, EventPayloadEvidence, EventPayloadProjectionOutcome, EventPayloadWriteAuthority, check_event_payload_boundary, event_payload_context_bindings, project_event_payload};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventPayloadConfig {
    pub schema_version: u32,
    pub artifact: String,
    pub monkey_profile: PathBuf,
    pub fluent_checkout: PathBuf,
    pub monkey_inputs: PathBuf,
    pub source_compile_database: PathBuf,
    pub libevent_profile: PathBuf,
    pub libevent_checkout: PathBuf,
    pub libevent_build: PathBuf,
    pub observations: PathBuf,
    pub bindings: PathBuf,
    pub allocation: PathBuf,
    pub runtime: PathBuf,
    pub observer_binary: PathBuf,
    pub observer_harness: PathBuf,
    pub monkey_object: PathBuf,
    pub libevent_library: PathBuf,
    pub run_manifest: PathBuf,
    /// Caller-declared exclusive storage ownership; never inferred from a hash.
    pub authority: EventPayloadWriteAuthority,
    pub report: PathBuf,
}
fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn run_event_payload_config(path: &Path) -> Result<(), String> {
    let config_bytes = read(path)?;
    let config: EventPayloadConfig = serde_json::from_slice(&config_bytes).map_err(|e| e.to_string())?;
    if config.schema_version != 1 { return Err("unsupported payload configuration".into()); }
    let resolve = |p: &Path| crate::resolve(path.parent().unwrap_or(Path::new(".")), p);
    let manifest_bytes = read(&resolve(&config.run_manifest))?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    if manifest["schema_version"] != 1 || manifest["status"] != "observed_success"
        || manifest["profile"] != "cycle18.cb_event.single_dispatcher.v1" {
        return Err("unsupported payload native manifest".into());
    }
    let mut snapshots = BTreeMap::new();
    for section in ["files", "dependencies"] {
        let files = manifest[section].as_object().filter(|v| !v.is_empty()).ok_or("missing native file inventory")?;
        for (name, expected) in files {
            let selected = PathBuf::from(name);
            if !selected.is_absolute() { return Err("native inventory path must be absolute".into()); }
            let bytes = read(&selected)?;
            if expected.as_str() != Some(sha256_hex(&bytes).as_str()) { return Err(format!("native inventory digest mismatch: {name}")); }
            snapshots.insert(selected.canonicalize().map_err(|e| e.to_string())?, bytes);
        }
    }
    for selected in [&config.monkey_inputs, &config.source_compile_database, &config.observations,
        &config.bindings, &config.allocation, &config.runtime, &config.observer_binary,
        &config.observer_harness, &config.monkey_object, &config.libevent_library] {
        let canonical = resolve(selected).canonicalize().map_err(|e| e.to_string())?;
        if !snapshots.contains_key(&canonical) { return Err(format!("selected input absent from native inventory: {}", selected.display())); }
    }
    let selected = ["cb_event", "_mk_event_loop_create"];
    let source = load_monkey_source(&resolve(&config.monkey_profile), &resolve(&config.fluent_checkout), &resolve(&config.monkey_inputs), &resolve(&config.source_compile_database), &selected)?;
    let summary = |name| source.summaries.iter().find(|s| s.function() == name).ok_or_else(|| format!("missing {name} source summary"));
    let projection = match project_event_payload(summary("cb_event")?, summary("_mk_event_loop_create")?) {
        EventPayloadProjectionOutcome::Projected(p) => p,
        EventPayloadProjectionOutcome::Unknown { reason } => return Err(reason),
    };
    let libevent = load_profile_directory(&resolve(&config.libevent_profile)).map_err(|e| e.to_string())?;
    let library = libevent.library("libevent").ok_or("missing libevent profile")?;
    let checkout = resolve(&config.libevent_checkout);
    let revision = verify_pin("libevent", libevent.manifest().pin_for("libevent").ok_or("missing libevent pin")?, &checkout).map_err(|e| e.to_string())?;
    if manifest["source_revision"] != source.revision || manifest["libevent_revision"] != revision { return Err("native revision mismatch".into()); }
    let bundle = load_audited_bundle(library, &checkout, &revision).map_err(|e| format!("Libevent source refused: {e:?}"))?;
    let cache_path = resolve(&config.libevent_build).join("CMakeCache.txt");
    let cache = read(&cache_path)?;
    if snapshots.get(&cache_path.canonicalize().map_err(|e| e.to_string())?) != Some(&cache) { return Err("build cache absent from manifest".into()); }
    let cache_text = std::str::from_utf8(&cache).map_err(|e| e.to_string())?;
    let entries: BTreeMap<_, _> = cache_text.lines().filter_map(|line| {let (k,v)=line.split_once('=')?; let(k,_)=k.split_once(':')?; Some((k,v))}).collect();
    if entries.get("CMAKE_HOME_DIRECTORY").copied() != checkout.to_str()
        || entries.get("EVENT__DISABLE_OPENSSL").copied() != Some("ON")
        || entries.get("EVENT__DISABLE_THREAD_SUPPORT").copied() != Some("OFF") { return Err("unsupported Libevent build".into()); }
    let runtime: serde_json::Value = serde_json::from_slice(&read(&resolve(&config.runtime))?).map_err(|e| e.to_string())?;
    if runtime != manifest["runtime"] || runtime["dispatcher_count"] != 1 { return Err("runtime context mismatch".into()); }
    let backend = runtime["backend"].as_str().ok_or("missing native backend")?;
    if !library.accepted_builds.iter().any(|c| c.backend == backend && c.threads == "pthreads" && c.forbid.iter().all(|f| f == "openssl")) { return Err("native backend outside profile".into()); }
    let observations = read(&resolve(&config.observations))?;
    let bindings = read(&resolve(&config.bindings))?;
    let allocation = read(&resolve(&config.allocation))?;
    let model = CallbackSummary {
        argument: config.authority.callback_argument, argument_mode: ArgumentMode::Borrowed,
        trigger_reasons: BTreeSet::from([ActivationReason::Read, ActivationReason::Write]),
        payload_authority: BTreeMap::from([(config.authority.context_payload, PayloadAccessKind::Write), (config.authority.fired_payload, PayloadAccessKind::Write)]),
        may_defer: false, may_spawn_tasks: false,
    };
    let mut assumptions = source.profile.assumptions();
    assumptions.extend(event_payload_context_bindings(&projection, &config.authority, &model));
    assumptions.insert("event_payload_allocation_evidence_sha256".into(), sha256_hex(&allocation));
    assumptions.insert("observation_authority".into(), "producer-supplied runtime and linkage evidence; not attestation".into());
    assumptions.insert("payload_write_authority".into(), "caller-declared exclusive authority over selected context and fired storage".into());
    assumptions.insert("native_run_manifest_sha256".into(), sha256_hex(&manifest_bytes));
    let compile_digest = source.compile_task.digest()?;
    let payload = EventCompositionPayload { observations: &observations, bindings: &bindings };
    let context = EventCompositionContext::derive(EventCompositionContextInput {
        artifact: config.artifact, semantic_profile: format!("monkey-forced-libevent+linux-{backend}"),
        scope: BTreeSet::from(["cb_event".into()]), bounds: BTreeMap::from([("callback_executions".into(),4), ("source_paths".into(),4)]), assumptions,
        source_compile_task_sha256: compile_digest.clone(), fluent_bit_revision: source.revision.clone(), monkey_source_sha256: source.source_sha256.clone(), monkey_profile_sha256: source.profile.manifest_digest().into(), monkey_build_sha256: source.build_sha256.clone(),
        libevent_revision: revision, libevent_source_sha256: sha256_hex(&serde_json::to_vec(&bundle.files).map_err(|e| e.to_string())?), libevent_profile_sha256: libevent.manifest_digest().into(), libevent_build_sha256: sha256_hex(&cache),
        contracts_sha256: sha256_hex(EVENT_PAYLOAD_CONTRACT.as_bytes()), source_rules_sha256: sha256_hex(qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES.as_bytes()), observation_rules_sha256: sha256_hex(EVENT_PAYLOAD_CONTRACT.as_bytes()),
        observer_binary_sha256: sha256_hex(&read(&resolve(&config.observer_binary))?), observer_harness_sha256: sha256_hex(&read(&resolve(&config.observer_harness))?), linked_libraries_sha256: BTreeMap::from([("monkey".into(),sha256_hex(&read(&resolve(&config.monkey_object))?)),("libevent".into(),sha256_hex(&read(&resolve(&config.libevent_library))?))]), observation_trust: DeclaredObservationTrust::ProducerSuppliedRuntimeEvidence,
    }, payload).map_err(|e| e.to_string())?;
    let admitted = check_event_composition_context_for(&context, payload, &context).map_err(|e| e.to_string())?;
    let checked = match check_event_payload_boundary(&projection, &admitted, EventPayloadEvidence { observations: &observations, bindings: &bindings, allocation: &allocation }, &config.authority, &model) {
        EventPayloadBoundary::Checked(report) => report,
        EventPayloadBoundary::Unknown { reason } => return Err(reason),
    };
    if checked.accounted_non_cb_event_bindings != 0 { return Err("dedicated run includes unrelated callbacks".into()); }
    let reloaded = load_monkey_source(&resolve(&config.monkey_profile), &resolve(&config.fluent_checkout), &resolve(&config.monkey_inputs), &resolve(&config.source_compile_database), &selected)?;
    if reloaded.compile_task.digest()? != compile_digest || reloaded.source_sha256 != source.source_sha256 || reloaded.build_sha256 != source.build_sha256 || read(path)? != config_bytes || read(&resolve(&config.run_manifest))? != manifest_bytes { return Err("inputs changed during payload audit".into()); }
    for (p, bytes) in &snapshots { if read(p)? != *bytes { return Err(format!("input changed during audit: {}", p.display())); } }
    let mut files = source.protected_files;
    files.extend(snapshots.keys().cloned());
    files.extend([path.to_path_buf(), resolve(&config.run_manifest)]);
    let mut directories = source.protected_directories;
    directories.extend([resolve(&config.libevent_profile), checkout, resolve(&config.libevent_build)]);
    reject_report_aliases(&resolve(&config.report), &files, &directories)?;
    let report = serde_json::json!({"schema_version":1,"status":"bounded_payload_checked","context":context,"payload":checked,"limitations":["Exclusive write authority is caller-declared","Runtime and linkage evidence is producer-supplied","Four bounded executions; no universal safety or aggregate acceptance promotion"]});
    fs::write(resolve(&config.report), serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
