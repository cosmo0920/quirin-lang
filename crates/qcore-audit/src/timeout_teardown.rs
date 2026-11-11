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

//! Configured source/native audit for Monkey `cb_timeout` self teardown.

use crate::observed_wait_inputs::load_monkey_source;
use qcore_schemas::sha256_hex;
use qcore_verification::timeout_teardown::{
    CloseObservation, TimeoutTeardownContracts, TimeoutTeardownEvidence,
    check_timeout_teardown_evidence, project_cb_timeout_source,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeoutTeardownConfig {
    pub schema_version: u32,
    pub artifact: String,
    pub monkey_profile: PathBuf,
    pub fluent_checkout: PathBuf,
    pub monkey_inputs: PathBuf,
    pub source_compile_database: PathBuf,
    pub libevent_checkout: PathBuf,
    pub linked_libevent_event_c: PathBuf,
    pub timeout_contract: PathBuf,
    pub observations: PathBuf,
    pub bindings: PathBuf,
    pub observer_binary: PathBuf,
    pub observer_harness: PathBuf,
    pub libevent_library: PathBuf,
    pub run_manifest: PathBuf,
    pub callback_enter_sequence: u64,
    pub report: PathBuf,
}

fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    }
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn revision(checkout: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(["rev-parse", "--verify", "HEAD^{commit}"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("cannot resolve Libevent revision".into());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().into())
        .map_err(|e| e.to_string())
}

fn lines(bytes: &[u8]) -> Result<Vec<Value>, String> {
    std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(|e| e.to_string()))
        .collect()
}

fn number(value: &Value, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing integer `{key}`"))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing text `{key}`"))
}

fn derive_evidence(
    observations: &[u8],
    bindings: &[u8],
    enter: u64,
    projection: &qcore_verification::timeout_teardown::TimeoutTeardownSourceProjection,
) -> Result<TimeoutTeardownEvidence, String> {
    let observations = lines(observations)?;
    let bindings = lines(bindings)?;
    if observations.iter().any(|row| row["schema_version"] != 1)
        || observations.windows(2).any(|pair| {
            pair[0]["sequence"]
                .as_u64()
                .zip(pair[1]["sequence"].as_u64())
                .is_none_or(|(left, right)| left >= right)
        })
    {
        return Err("observation sequences are not globally unique and ordered".into());
    }
    let binding = bindings
        .iter()
        .filter(|row| row["kind"] == "callback" && row["enter"] == enter)
        .collect::<Vec<_>>();
    let [binding] = binding.as_slice() else {
        return Err("selected callback binding is missing or ambiguous".into());
    };
    let event = number(binding, "event")?;
    let exit = number(binding, "exit")?;
    let native_map_address = number(binding, "argument")?;
    let before = binding["before"]
        .as_array()
        .ok_or("callback before-snapshot is absent")?;
    let map_cell = |field: &str, c_type: &str| -> Result<&Value, String> {
        let matches = before
            .iter()
            .filter(|cell| {
                cell["object"] == native_map_address
                    && cell["field"] == field
                    && cell["c_type"] == c_type
            })
            .collect::<Vec<_>>();
        let [cell] = matches.as_slice() else {
            return Err(format!("map snapshot lacks unique typed {field} cell"));
        };
        Ok(&cell["value"])
    };
    let event_pointer = map_cell("event", "struct event *")?
        .as_u64()
        .ok_or("callback event pointer is absent")?;
    let map_write_descriptor = map_cell("pipe[1]", "int")?
        .as_i64()
        .and_then(|v| i32::try_from(v).ok())
        .ok_or("callback write descriptor is absent")?;
    let calls = binding["calls"]
        .as_array()
        .ok_or("callback calls are absent")?;
    let call_names = calls
        .iter()
        .map(|call| text(call, "function").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let paths = projection
        .path_call_sequences()
        .iter()
        .enumerate()
        .filter_map(|(index, expected)| (*expected == call_names).then_some(index))
        .collect::<Vec<_>>();
    let [path_index] = paths.as_slice() else {
        return Err("native callback calls match no unique source path".into());
    };
    let close_call = calls
        .iter()
        .find(|call| call["function"] == "evutil_closesocket")
        .ok_or("missing callback close call")?;
    let delete_call = calls
        .iter()
        .find(|call| call["function"] == "event_del")
        .ok_or("missing callback delete call")?;
    let free_call = calls
        .iter()
        .find(|call| call["function"] == "event_free")
        .ok_or("missing callback free call")?;
    let map_free_calls = calls
        .iter()
        .filter(|call| call["function"] == "mk_mem_free")
        .collect::<Vec<_>>();
    let [map_free_call] = map_free_calls.as_slice() else {
        return Err("callback map release is missing or duplicated".into());
    };
    if delete_call["result"].as_i64() != Some(0)
        || free_call["result"].as_i64() != Some(0)
        || map_free_call["result"].as_i64() != Some(0)
    {
        return Err("callback delete/free result mismatch".into());
    }
    for call in [delete_call, free_call] {
        if call["arguments"]
            .as_array()
            .and_then(|arguments| arguments.first())
            .and_then(Value::as_u64)
            != Some(event_pointer)
        {
            return Err("event delete/free pointer differs from registered event".into());
        }
    }
    if map_free_call["arguments"]
        .as_array()
        .and_then(|arguments| arguments.first())
        .and_then(Value::as_u64)
        != Some(native_map_address)
    {
        return Err("map-free pointer differs from callback argument".into());
    }
    let registrations = bindings
        .iter()
        .filter(|row| {
            row["kind"] == "registration"
                && row["event"] == event
                && row["argument"] == native_map_address
                && row["event_pointer"] == event_pointer
        })
        .collect::<Vec<_>>();
    let [_registration] = registrations.as_slice() else {
        return Err("callback registration identity mismatch".into());
    };
    let callback_descriptor = close_call["arguments"]
        .as_array()
        .and_then(|v| v.first())
        .and_then(Value::as_i64)
        .and_then(|v| i32::try_from(v).ok())
        .ok_or("invalid callback descriptor")?;
    if close_call["result"].as_i64() != Some(0) {
        return Err("callback close failure lacks observed errno classification".into());
    }
    let entered = observations
        .iter()
        .filter(|row| {
            row["sequence"] == enter
                && row["action"]["kind"] == "event"
                && row["action"]["operation"] == "callback_enter"
                && row["action"]["callback"] == "cb_timeout"
                && row["action"]["event"] == event
                && row["action"]["descriptor"].is_null()
        })
        .collect::<Vec<_>>();
    let [entered] = entered.as_slice() else {
        return Err("callback-enter observation mismatch".into());
    };
    let base =
        u32::try_from(number(entered, "base")?).map_err(|_| "base identity is outside u32")?;
    let dispatcher = u32::try_from(number(entered, "dispatcher")?)
        .map_err(|_| "dispatcher identity is outside u32")?;
    let thread = number(entered, "thread")?;
    let event_id = u32::try_from(event).map_err(|_| "event identity is outside u32")?;
    let matching_event = |call: &Value, operation: &str| -> Result<&Value, String> {
        let sequence = number(call, "sequence")?;
        let rows = observations
            .iter()
            .filter(|row| {
                row["sequence"] == sequence
                    && row["action"]["kind"] == "event"
                    && row["action"]["operation"] == operation
                    && row["action"]["callback"] == "cb_timeout"
                    && row["action"]["event"] == event
            })
            .collect::<Vec<_>>();
        let [row] = rows.as_slice() else {
            return Err(format!("native {operation} observation mismatch"));
        };
        Ok(*row)
    };
    let delete_observation = matching_event(delete_call, "event_del")?;
    let free_observation = matching_event(free_call, "event_free")?;
    let exit_rows = observations
        .iter()
        .filter(|row| {
            row["sequence"] == exit
                && row["action"]["kind"] == "event"
                && row["action"]["operation"] == "callback_exit"
                && row["action"]["callback"] == "cb_timeout"
                && row["action"]["event"] == event
        })
        .collect::<Vec<_>>();
    let [exit_observation] = exit_rows.as_slice() else {
        return Err("callback-exit observation mismatch".into());
    };
    let close_sequence = number(close_call, "sequence")?;
    let close_rows = observations
        .iter()
        .filter(|row| {
            row["sequence"] == close_sequence
                && row["action"]["kind"] == "descriptor_close"
                && row["action"]["descriptor"]["fd"].as_i64()
                    == Some(i64::from(callback_descriptor))
                && row["action"]["result"] == 0
        })
        .collect::<Vec<_>>();
    let [close_observation] = close_rows.as_slice() else {
        return Err("callback descriptor-close observation mismatch".into());
    };
    for row in [
        *close_observation,
        delete_observation,
        free_observation,
        *exit_observation,
    ] {
        if row["base"].as_u64() != Some(u64::from(base))
            || row["dispatcher"].as_u64() != Some(u64::from(dispatcher))
            || row["thread"].as_u64() != Some(thread)
        {
            return Err("callback teardown changed base, dispatcher or thread".into());
        }
    }
    let socketpairs = observations
        .iter()
        .filter(|row| {
            row["base"] == u64::from(base)
                && row["sequence"]
                    .as_u64()
                    .is_some_and(|sequence| sequence < enter)
                && row["action"]["kind"] == "socketpair"
                && row["action"]["result"] == 0
                && row["action"]["descriptors"]
                    .as_array()
                    .is_some_and(|descriptors| {
                        descriptors
                            .iter()
                            .any(|d| d["fd"].as_i64() == Some(i64::from(callback_descriptor)))
                    })
        })
        .collect::<Vec<_>>();
    let socketpair = socketpairs
        .last()
        .ok_or("missing callback socket-pair acquisition")?;
    let pair_descriptors = socketpair["action"]["descriptors"].as_array().unwrap();
    let callback_generation = pair_descriptors
        .iter()
        .find(|d| d["fd"].as_i64() == Some(i64::from(callback_descriptor)))
        .and_then(|d| d["generation"].as_u64())
        .ok_or("missing callback descriptor generation")?;
    if close_observation["action"]["descriptor"]["generation"].as_u64() != Some(callback_generation)
    {
        return Err("callback close generation differs from acquired pair".into());
    }
    let peer = pair_descriptors
        .iter()
        .find(|d| d["fd"].as_i64() != Some(i64::from(callback_descriptor)))
        .ok_or("missing distinct peer descriptor")?;
    let peer_descriptor = peer["fd"]
        .as_i64()
        .and_then(|fd| i32::try_from(fd).ok())
        .ok_or("invalid peer descriptor")?;
    let peer_generation = number(peer, "generation")?;
    let socket_sequence = number(socketpair, "sequence")?;
    let peer_closes = observations
        .iter()
        .filter(|row| {
            row["sequence"]
                .as_u64()
                .is_some_and(|sequence| sequence > socket_sequence && sequence < enter)
                && row["action"]["kind"] == "descriptor_close"
                && row["action"]["descriptor"]["fd"].as_i64() == Some(i64::from(peer_descriptor))
                && row["action"]["descriptor"]["generation"].as_u64() == Some(peer_generation)
                && row["action"]["result"] == 0
        })
        .collect::<Vec<_>>();
    let [peer_close_observation] = peer_closes.as_slice() else {
        return Err("caller peer-close observation mismatch".into());
    };
    if peer_close_observation["base"].as_u64() != Some(u64::from(base))
        || peer_close_observation["dispatcher"].as_u64() != Some(u64::from(dispatcher))
        || peer_close_observation["thread"].as_u64() != Some(thread)
    {
        return Err("caller peer-close changed base, dispatcher or thread".into());
    }
    Ok(TimeoutTeardownEvidence {
        source_summary_digest: projection.source_summary_digest().into(),
        libevent_event_c_digest: projection.libevent_event_c_digest().into(),
        contract_digest: projection.contract_digest().into(),
        source_identity_digest: projection.source_identity_digest().into(),
        path_index: *path_index,
        callback: text(&entered["action"], "callback")?.into(),
        base,
        event_id,
        event_generation: 1,
        dispatcher,
        map_argument: event_id,
        native_map_address,
        event_pointer,
        event_descriptor: None,
        peer_descriptor,
        callback_descriptor,
        map_write_descriptor,
        delete_event_id: u32::try_from(number(&delete_observation["action"], "event")?)
            .map_err(|_| "delete event outside u32")?,
        free_event_id: u32::try_from(number(&free_observation["action"], "event")?)
            .map_err(|_| "free event outside u32")?,
        exit_event_id: u32::try_from(number(&exit_observation["action"], "event")?)
            .map_err(|_| "exit event outside u32")?,
        delete_dispatcher: u32::try_from(number(delete_observation, "dispatcher")?)
            .map_err(|_| "delete dispatcher outside u32")?,
        free_dispatcher: u32::try_from(number(free_observation, "dispatcher")?)
            .map_err(|_| "free dispatcher outside u32")?,
        exit_dispatcher: u32::try_from(number(exit_observation, "dispatcher")?)
            .map_err(|_| "exit dispatcher outside u32")?,
        callback_enter_sequence: enter,
        close_sequence,
        delete_sequence: number(delete_call, "sequence")?,
        free_sequence: number(free_call, "sequence")?,
        map_free_sequence: number(map_free_call, "sequence")?,
        callback_exit_sequence: exit,
        close_function: text(close_call, "function")?.into(),
        delete_operation: text(&delete_observation["action"], "operation")?.into(),
        free_operation: text(&free_observation["action"], "operation")?.into(),
        map_free_function: text(map_free_call, "function")?.into(),
        exit_operation: text(&exit_observation["action"], "operation")?.into(),
        callback_close: CloseObservation::Success,
        peer_close: CloseObservation::Success,
    })
}

fn reject_alias(
    report: &Path,
    protected: &[PathBuf],
    directories: &[PathBuf],
) -> Result<(), String> {
    let parent = report
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let destination = if report.exists() {
        report.canonicalize().map_err(|e| e.to_string())?
    } else {
        parent.join(report.file_name().ok_or("report has no filename")?)
    };
    for path in protected {
        if destination == path.canonicalize().map_err(|e| e.to_string())? {
            return Err("report aliases a verified input".into());
        }
    }
    for directory in directories {
        if destination.starts_with(directory.canonicalize().map_err(|e| e.to_string())?) {
            return Err("report is inside a verified input directory".into());
        }
    }
    Ok(())
}

pub fn run_timeout_teardown_config(path: &Path) -> Result<(), String> {
    prepare_timeout_teardown(path)?.publish()
}

pub(crate) fn prepare_timeout_teardown(
    path: &Path,
) -> Result<crate::cycle18_inputs::PreparedAudit<crate::cycle18_inputs::Timeout>, String> {
    let config: TimeoutTeardownConfig =
        serde_json::from_slice(&read(path)?).map_err(|e| e.to_string())?;
    if config.schema_version != 1 || config.artifact.is_empty() {
        return Err("unsupported timeout-teardown configuration".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let mut selected = [
        &config.monkey_inputs,
        &config.source_compile_database,
        &config.linked_libevent_event_c,
        &config.timeout_contract,
        &config.observations,
        &config.bindings,
        &config.observer_binary,
        &config.observer_harness,
        &config.libevent_library,
        &config.run_manifest,
    ]
    .into_iter()
    .map(|file| resolve(base, file))
    .collect::<Vec<_>>();
    let bound_artifact_count = selected.len() - 1;
    selected.push(resolve(base, &config.libevent_checkout).join("event.c"));
    selected.push(path.to_path_buf());
    let snapshots = selected
        .iter()
        .map(|file| read(file).map(|bytes| (file.clone(), bytes)))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest: Value = serde_json::from_slice(&read(&resolve(base, &config.run_manifest))?)
        .map_err(|e| e.to_string())?;
    if manifest["schema_version"] != 1 || manifest["status"] != "observed_success" {
        return Err("unsupported native run manifest".into());
    }
    let inventory = manifest["artifacts_sha256"]
        .as_object()
        .ok_or("native manifest lacks artifact hashes")?;
    for (file, bytes) in snapshots.iter().take(bound_artifact_count) {
        let key = file
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or("invalid selected filename")?;
        if inventory.get(key).and_then(Value::as_str) != Some(sha256_hex(bytes).as_str()) {
            return Err(format!("native manifest does not bind {key}"));
        }
    }
    for (role, configured) in [
        ("monkey_inputs", &config.monkey_inputs),
        ("source_compile_database", &config.source_compile_database),
        ("linked_libevent_event_c", &config.linked_libevent_event_c),
        ("timeout_contract", &config.timeout_contract),
        ("observations", &config.observations),
        ("bindings", &config.bindings),
        ("observer_binary", &config.observer_binary),
        ("observer_harness", &config.observer_harness),
        ("libevent_library", &config.libevent_library),
    ] {
        let recorded = manifest["roles"][role]
            .as_str()
            .ok_or_else(|| format!("missing artifact role {role}"))?;
        if Path::new(recorded)
            .canonicalize()
            .map_err(|e| e.to_string())?
            != resolve(base, configured)
                .canonicalize()
                .map_err(|e| e.to_string())?
        {
            return Err(format!("native artifact role mismatch: {role}"));
        }
    }
    let source = load_monkey_source(
        &resolve(base, &config.monkey_profile),
        &resolve(base, &config.fluent_checkout),
        &resolve(base, &config.monkey_inputs),
        &resolve(base, &config.source_compile_database),
        &["cb_timeout"],
    )?;
    let aggregate_identity = crate::cycle18_inputs::SourceIdentity::from_source(&source)?;
    let [summary] = source.summaries.as_slice() else {
        return Err("expected one cb_timeout source summary".into());
    };
    let compile_task_digest = source.compile_task.digest()?;
    let source_snapshot = (
        source.revision.clone(),
        source.source_sha256.clone(),
        source.build_sha256.clone(),
    );
    let libevent_revision = revision(&resolve(base, &config.libevent_checkout))?;
    if manifest["monkey_revision"].as_str() != Some(&source.revision)
        || manifest["libevent_revision"].as_str() != Some(&libevent_revision)
    {
        return Err("native manifest source revision mismatch".into());
    }
    let event_c = read(&resolve(base, &config.linked_libevent_event_c))?;
    if event_c != read(&resolve(base, &config.libevent_checkout).join("event.c"))? {
        return Err("linked Libevent event.c differs from pinned checkout".into());
    }
    let contracts =
        TimeoutTeardownContracts::parse().map_err(|e| format!("policy refused: {e:?}"))?;
    let projection = project_cb_timeout_source(summary, &contracts, &libevent_revision, &event_c)
        .map_err(|e| format!("source projection refused: {e:?}"))?;
    if sha256_hex(&read(&resolve(base, &config.timeout_contract))?) != projection.contract_digest()
    {
        return Err("configured timeout policy differs from bundled executable policy".into());
    }
    let observations = read(&resolve(base, &config.observations))?;
    let bindings = read(&resolve(base, &config.bindings))?;
    let evidence = derive_evidence(
        &observations,
        &bindings,
        config.callback_enter_sequence,
        &projection,
    )?;
    let certificate = check_timeout_teardown_evidence(&projection, &evidence)
        .map_err(|e| format!("timeout teardown refused: {e:?}"))?;
    let reloaded = load_monkey_source(
        &resolve(base, &config.monkey_profile),
        &resolve(base, &config.fluent_checkout),
        &resolve(base, &config.monkey_inputs),
        &resolve(base, &config.source_compile_database),
        &["cb_timeout"],
    )?;
    if reloaded.compile_task.digest()? != compile_task_digest
        || (
            reloaded.revision,
            reloaded.source_sha256,
            reloaded.build_sha256,
        ) != source_snapshot
        || revision(&resolve(base, &config.libevent_checkout))? != libevent_revision
    {
        return Err(
            "source, profile, compilation or Libevent identity changed during audit".into(),
        );
    }
    for (file, before) in &snapshots {
        if read(file)? != *before {
            return Err(format!("input changed during audit: {}", file.display()));
        }
    }
    let report = resolve(base, &config.report);
    let mut protected_directories = source.protected_directories;
    protected_directories.push(resolve(base, &config.libevent_checkout));
    let mut protected_files = selected.clone();
    protected_files.extend(source.protected_files);
    reject_alias(&report, &protected_files, &protected_directories)?;
    let report_value = serde_json::json!({
        "schema_version": 1, "status": "timeout_self_teardown_checked", "artifact": config.artifact,
        "source": projection, "native_evidence": evidence, "certificate": certificate,
        "identities": { "manifest_sha256": sha256_hex(&read(&resolve(base, &config.run_manifest))?),
            "source_compile_task_sha256": compile_task_digest,
            "monkey_source_sha256": source.source_sha256, "monkey_build_sha256": source.build_sha256 },
        "limitations": ["One bounded native callback execution, not whole-program safety",
            "Runtime and linkage records are producer-supplied evidence, not attestation",
            "The peer endpoint close belongs to the enclosing caller, not cb_timeout"]
    });
    let monkey_revision = aggregate_identity.revision.clone();
    crate::cycle18_inputs::PreparedAudit::new(
        aggregate_identity,
        crate::cycle18_inputs::Timeout {
            projection,
            evidence,
            summaries: source.summaries,
        },
        report,
        serde_json::to_vec_pretty(&report_value).map_err(|e| e.to_string())?,
        protected_files,
        protected_directories,
    )
    .map(|prepared| {
        prepared
            .with_checkout_revision(resolve(base, &config.fluent_checkout), monkey_revision)
            .with_checkout_revision(resolve(base, &config.libevent_checkout), libevent_revision)
    })
}
