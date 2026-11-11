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

//! Configured, bounded cb_event payload checking over pinned native inputs.
use crate::observed_wait::reject_report_aliases;
use crate::observed_wait_inputs::load_monkey_source;
use qcore_contract::profile_loader::{load_audited_bundle, load_profile_directory, verify_pin};

use qcore_schemas::sha256_hex;
use qcore_verification::event_composition_context::{
    DeclaredObservationTrust, EventCompositionContext, EventCompositionContextInput,
    EventCompositionPayload, check_event_composition_context_for,
};
use qcore_verification::event_payload::{
    EVENT_PAYLOAD_CONTRACT, EventPayloadBoundary, EventPayloadEvidence,
    EventPayloadProjectionOutcome, EventPayloadWriteAuthority, check_event_payload_boundary,
    event_payload_context_bindings, project_event_payload,
};
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
    prepare_event_payload(path)?.publish()
}

pub(crate) fn prepare_event_payload(
    path: &Path,
) -> Result<crate::cycle18_inputs::PreparedAudit<crate::cycle18_inputs::Payload>, String> {
    let config_bytes = read(path)?;
    let config: EventPayloadConfig =
        serde_json::from_slice(&config_bytes).map_err(|e| e.to_string())?;
    if config.schema_version != 1 {
        return Err("unsupported payload configuration".into());
    }
    let resolve = |p: &Path| crate::resolve(path.parent().unwrap_or(Path::new(".")), p);
    let manifest_bytes = read(&resolve(&config.run_manifest))?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    if manifest["schema_version"] != 1
        || manifest["status"] != "observed_success"
        || manifest["profile"] != "cycle18.cb_event.single_dispatcher.v1"
    {
        return Err("unsupported payload native manifest".into());
    }
    let mut snapshots = BTreeMap::new();
    for section in ["files", "dependencies"] {
        let files = manifest[section]
            .as_object()
            .filter(|v| !v.is_empty())
            .ok_or("missing native file inventory")?;
        for (name, expected) in files {
            let selected = PathBuf::from(name);
            if !selected.is_absolute() {
                return Err("native inventory path must be absolute".into());
            }
            let bytes = read(&selected)?;
            if expected.as_str() != Some(sha256_hex(&bytes).as_str()) {
                return Err(format!("native inventory digest mismatch: {name}"));
            }
            snapshots.insert(selected.canonicalize().map_err(|e| e.to_string())?, bytes);
        }
    }
    for selected in [
        &config.monkey_inputs,
        &config.source_compile_database,
        &config.observations,
        &config.bindings,
        &config.allocation,
        &config.runtime,
        &config.observer_binary,
        &config.observer_harness,
        &config.monkey_object,
        &config.libevent_library,
    ] {
        let canonical = resolve(selected)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !snapshots.contains_key(&canonical) {
            return Err(format!(
                "selected input absent from native inventory: {}",
                selected.display()
            ));
        }
    }
    for (role, configured) in [
        ("monkey_inputs", &config.monkey_inputs),
        ("source_compile_database", &config.source_compile_database),
        ("observations", &config.observations),
        ("bindings", &config.bindings),
        ("allocation", &config.allocation),
        ("runtime", &config.runtime),
        ("observer_binary", &config.observer_binary),
        ("observer_harness", &config.observer_harness),
        ("monkey_object", &config.monkey_object),
        ("libevent_library", &config.libevent_library),
    ] {
        let recorded = manifest["roles"][role]
            .as_str()
            .ok_or_else(|| format!("missing native artifact role {role}"))?;
        if Path::new(recorded)
            .canonicalize()
            .map_err(|e| e.to_string())?
            != resolve(configured)
                .canonicalize()
                .map_err(|e| e.to_string())?
        {
            return Err(format!("native artifact role mismatch: {role}"));
        }
    }
    let link = manifest["commands"]["link"]
        .as_array()
        .ok_or("missing native link command")?;
    for role in [
        "monkey_object",
        "monkey_support_object",
        "libevent_library",
        "observer_binary",
    ] {
        let recorded = manifest["roles"][role]
            .as_str()
            .ok_or("missing linked artifact role")?;
        let canonical = Path::new(recorded)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !snapshots.contains_key(&canonical)
            || !link
                .iter()
                .filter_map(|v| v.as_str())
                .any(|arg| Path::new(arg).canonicalize().ok().as_ref() == Some(&canonical))
        {
            return Err(format!("recorded link command does not select {role}"));
        }
    }
    if manifest["scope"]["callback"] != "cb_event"
        || manifest["scope"]["executions"] != 4
        || manifest["scope"]["source_paths"] != 4
        || manifest["scope"]["dispatcher_count"] != 1
        || manifest["observer"]["activation"]
            != "controlled event_active flags; not readiness or timer evidence"
    {
        return Err("native scope or activation classification mismatch".into());
    }
    let selected = ["cb_event", "_mk_event_loop_create"];
    let source = load_monkey_source(
        &resolve(&config.monkey_profile),
        &resolve(&config.fluent_checkout),
        &resolve(&config.monkey_inputs),
        &resolve(&config.source_compile_database),
        &selected,
    )?;
    let aggregate_identity = crate::cycle18_inputs::SourceIdentity::from_source(&source)?;
    let summary = |name| {
        source
            .summaries
            .iter()
            .find(|s| s.function() == name)
            .ok_or_else(|| format!("missing {name} source summary"))
    };
    let projection =
        match project_event_payload(summary("cb_event")?, summary("_mk_event_loop_create")?) {
            EventPayloadProjectionOutcome::Projected(p) => p,
            EventPayloadProjectionOutcome::Unknown { reason } => return Err(reason),
        };
    let libevent =
        load_profile_directory(&resolve(&config.libevent_profile)).map_err(|e| e.to_string())?;
    snapshots.insert(
        libevent.directory().join("profile.json"),
        libevent.manifest_text().as_bytes().to_vec(),
    );
    let library = libevent
        .library("libevent")
        .ok_or("missing libevent profile")?;
    let checkout = resolve(&config.libevent_checkout);
    let revision = verify_pin(
        "libevent",
        libevent
            .manifest()
            .pin_for("libevent")
            .ok_or("missing libevent pin")?,
        &checkout,
    )
    .map_err(|e| e.to_string())?;
    if manifest["source_revision"] != source.revision || manifest["libevent_revision"] != revision {
        return Err("native revision mismatch".into());
    }
    let bundle = load_audited_bundle(library, &checkout, &revision)
        .map_err(|e| format!("Libevent source refused: {e:?}"))?;
    for (name, contents) in &bundle.files {
        let file = checkout.join(name);
        let bytes = read(&file)?;
        if bytes != contents.as_bytes() {
            return Err("Libevent bundle changed during loading".into());
        }
        snapshots.insert(file.canonicalize().map_err(|e| e.to_string())?, bytes);
    }
    let cache_path = resolve(&config.libevent_build).join("CMakeCache.txt");
    let cache = read(&cache_path)?;
    if snapshots.get(&cache_path.canonicalize().map_err(|e| e.to_string())?) != Some(&cache) {
        return Err("build cache absent from manifest".into());
    }
    let cache_text = std::str::from_utf8(&cache).map_err(|e| e.to_string())?;
    let entries: BTreeMap<_, _> = cache_text
        .lines()
        .filter_map(|line| {
            let (k, v) = line.split_once('=')?;
            let (k, _) = k.split_once(':')?;
            Some((k, v))
        })
        .collect();
    if entries.get("CMAKE_HOME_DIRECTORY").copied() != checkout.to_str()
        || entries.get("EVENT__DISABLE_OPENSSL").copied() != Some("ON")
        || entries.get("EVENT__DISABLE_THREAD_SUPPORT").copied() != Some("OFF")
    {
        return Err("unsupported Libevent build".into());
    }
    let runtime: serde_json::Value =
        serde_json::from_slice(&read(&resolve(&config.runtime))?).map_err(|e| e.to_string())?;
    if runtime != manifest["runtime"] || runtime["dispatcher_count"] != 1 {
        return Err("runtime context mismatch".into());
    }
    let backend = runtime["backend"]
        .as_str()
        .ok_or("missing native backend")?;
    if !library.accepted_builds.iter().any(|c| {
        c.backend == backend && c.threads == "pthreads" && c.forbid.iter().all(|f| f == "openssl")
    }) {
        return Err("native backend outside profile".into());
    }
    let observations = read(&resolve(&config.observations))?;
    let bindings = read(&resolve(&config.bindings))?;
    let allocation = read(&resolve(&config.allocation))?;
    let model =
        qcore_verification::event_payload::declared_event_payload_callback(&config.authority);
    let mut assumptions = source.profile.assumptions();
    assumptions.extend(event_payload_context_bindings(
        &projection,
        &config.authority,
        &model,
    ));
    assumptions.insert(
        "event_payload_allocation_evidence_sha256".into(),
        sha256_hex(&allocation),
    );
    assumptions.insert("activation_scope".into(), "Controlled event_active flags exercise payload branches; no readiness/timer or event-lifecycle composition claim".into());
    assumptions.insert(
        "observation_authority".into(),
        "producer-supplied runtime and linkage evidence; not attestation".into(),
    );
    assumptions.insert(
        "payload_write_authority".into(),
        "caller-declared exclusive authority over selected context and fired storage".into(),
    );
    assumptions.insert(
        "native_run_manifest_sha256".into(),
        sha256_hex(&manifest_bytes),
    );
    let compile_digest = source.compile_task.digest()?;
    let payload = EventCompositionPayload {
        observations: &observations,
        bindings: &bindings,
    };
    let context = EventCompositionContext::derive(
        EventCompositionContextInput {
            artifact: config.artifact,
            semantic_profile: format!("monkey-forced-libevent+linux-{backend}"),
            scope: BTreeSet::from(["cb_event".into()]),
            bounds: BTreeMap::from([
                ("callback_executions".into(), 4),
                ("source_paths".into(), 4),
            ]),
            assumptions,
            source_compile_task_sha256: compile_digest.clone(),
            fluent_bit_revision: source.revision.clone(),
            monkey_source_sha256: source.source_sha256.clone(),
            monkey_profile_sha256: source.profile.manifest_digest().into(),
            monkey_build_sha256: source.build_sha256.clone(),
            libevent_revision: revision.clone(),
            libevent_source_sha256: sha256_hex(
                &serde_json::to_vec(&bundle.files).map_err(|e| e.to_string())?,
            ),
            libevent_profile_sha256: libevent.manifest_digest().into(),
            libevent_build_sha256: sha256_hex(&cache),
            contracts_sha256: sha256_hex(EVENT_PAYLOAD_CONTRACT.as_bytes()),
            source_rules_sha256: sha256_hex(
                qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES.as_bytes(),
            ),
            observation_rules_sha256: sha256_hex(EVENT_PAYLOAD_CONTRACT.as_bytes()),
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
    let admitted = check_event_composition_context_for(&context, payload, &context)
        .map_err(|e| e.to_string())?;
    let checked = match check_event_payload_boundary(
        &projection,
        &admitted,
        EventPayloadEvidence {
            observations: &observations,
            bindings: &bindings,
            allocation: &allocation,
        },
        &config.authority,
        &model,
    ) {
        EventPayloadBoundary::Checked(report) => report,
        EventPayloadBoundary::Unknown { reason } => return Err(reason),
    };
    if checked.accounted_non_cb_event_bindings != 0
        || checked.checked_executions != 4
        || checked.selected_paths != 4
    {
        return Err("dedicated run includes unrelated callbacks".into());
    }
    let reloaded = load_monkey_source(
        &resolve(&config.monkey_profile),
        &resolve(&config.fluent_checkout),
        &resolve(&config.monkey_inputs),
        &resolve(&config.source_compile_database),
        &selected,
    )?;
    if reloaded.compile_task.digest()? != compile_digest
        || reloaded.source_sha256 != source.source_sha256
        || reloaded.build_sha256 != source.build_sha256
        || read(path)? != config_bytes
        || read(&resolve(&config.run_manifest))? != manifest_bytes
    {
        return Err("inputs changed during payload audit".into());
    }
    for (p, bytes) in &snapshots {
        if read(p)? != *bytes {
            return Err(format!("input changed during audit: {}", p.display()));
        }
    }
    let mut files = source.protected_files;
    files.extend(snapshots.keys().cloned());
    files.extend([path.to_path_buf(), resolve(&config.run_manifest)]);
    let mut directories = source.protected_directories;
    directories.extend([
        resolve(&config.libevent_profile),
        checkout.clone(),
        resolve(&config.libevent_build),
    ]);
    reject_report_aliases(&resolve(&config.report), &files, &directories)?;
    let report = serde_json::json!({"schema_version":1,"status":"bounded_payload_checked","context":context,"payload":checked,"limitations":["Exclusive write authority is caller-declared","Runtime and linkage evidence is producer-supplied","Four bounded executions; no universal safety or aggregate acceptance promotion"]});
    let monkey_revision = aggregate_identity.revision.clone();
    crate::cycle18_inputs::PreparedAudit::new(
        aggregate_identity,
        crate::cycle18_inputs::Payload {
            projection,
            context: admitted,
            observations,
            bindings,
            allocation,
            authority: config.authority,
            summaries: source.summaries,
        },
        resolve(&config.report),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        files,
        directories,
    )
    .map(|prepared| {
        prepared
            .with_checkout_revision(resolve(&config.fluent_checkout), monkey_revision)
            .with_checkout_revision(checkout, revision)
    })
}
