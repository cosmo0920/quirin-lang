//! Configured composition replays the original evidence through sealed inputs.
use crate::cycle18_inputs::SourceIdentity;
use qcore_c_gateway::typed_source::SourceSummary;
use qcore_schemas::sha256_hex;
use qcore_verification::cycle18_composition::{
    CYCLE18_COMPOSITION_POLICY, Cycle18CompositionInput, Cycle18PreparedSourceIdentity,
    check_cycle18_composition,
};
use qcore_verification::event_payload::{EventPayloadEvidence, declared_event_payload_callback};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    observed_wait: PathBuf,
    wait_failures: PathBuf,
    event_payload: PathBuf,
    timeout_teardown: PathBuf,
    policy: PathBuf,
    report: PathBuf,
}

fn identity(source: &SourceIdentity) -> Cycle18PreparedSourceIdentity {
    Cycle18PreparedSourceIdentity {
        source_revision: source.revision.clone(),
        source_compile_task_sha256: source.compile_task.clone(),
        source_sha256: source.source.clone(),
        profile_sha256: source.profile.clone(),
        build_sha256: source.build.clone(),
    }
}

fn selected<'a>(sources: &'a [SourceSummary], name: &str) -> Result<&'a SourceSummary, String> {
    let matches = sources.iter().filter(|source| source.function() == name).collect::<Vec<_>>();
    match matches.as_slice() {
        [source] => Ok(source),
        _ => Err(format!("expected exactly one independently imported {name}")),
    }
}

pub fn run_cycle18_composition_config(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let config: Config = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if config.schema_version != 1 {
        return Err("unsupported Cycle 18 composition configuration".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let resolve = |path: &Path| crate::resolve(base, path);
    let policy_path = resolve(&config.policy);
    let policy = fs::read(&policy_path).map_err(|error| error.to_string())?;
    if policy != CYCLE18_COMPOSITION_POLICY.as_bytes() {
        return Err("configured aggregate policy differs from executable bundled policy".into());
    }
    let wait = crate::observed_wait::prepare_observed_wait(&resolve(&config.observed_wait))?;
    let failures = crate::wait_failures::prepare_wait_failures(&resolve(&config.wait_failures))?;
    let payload = crate::event_payload::prepare_event_payload(&resolve(&config.event_payload))?;
    let timeout = crate::timeout_teardown::prepare_timeout_teardown(&resolve(&config.timeout_teardown))?;
    let failure_identity = identity(&failures.source_identity);
    let timeout_identity = identity(&timeout.source_identity);
    let callback = declared_event_payload_callback(&payload.evidence.authority);
    let aggregate = check_cycle18_composition(Cycle18CompositionInput {
        callback_summaries: [
            selected(&payload.evidence.summaries, "cb_event")?,
            selected(&timeout.evidence.summaries, "cb_timeout")?,
            selected(&wait.evidence.summaries, "cb_wait_2_timeout")?,
        ],
        wait_source: &wait.evidence.projection,
        wait_context: &wait.evidence.context,
        wait_observations: &wait.evidence.observations,
        wait_bindings: &wait.evidence.bindings,
        wait_failure_observations: &failures.evidence.observations,
        payload_source: &payload.evidence.projection,
        payload_context: &payload.evidence.context,
        payload_evidence: EventPayloadEvidence {
            observations: &payload.evidence.observations,
            bindings: &payload.evidence.bindings,
            allocation: &payload.evidence.allocation,
        },
        payload_authority: &payload.evidence.authority,
        payload_callback: &callback,
        timeout_source: &timeout.evidence.projection,
        timeout_evidence: &timeout.evidence.evidence,
        wait_failure_identity: &failure_identity,
        timeout_identity: &timeout_identity,
        // No configured production caller inventory has been established.
        caller_summaries: &[],
    }).map_err(|error| error.to_string())?;
    let mut protected = vec![path.to_path_buf(), policy_path.clone()];
    let mut directories = Vec::new();
    for (files, dirs) in [
        (&wait.protected_files, &wait.protected_directories),
        (&failures.protected_files, &failures.protected_directories),
        (&payload.protected_files, &payload.protected_directories),
        (&timeout.protected_files, &timeout.protected_directories),
    ] {
        protected.extend(files.iter().cloned());
        directories.extend(dirs.iter().cloned());
    }
    let destination = resolve(&config.report);
    crate::observed_wait::reject_report_aliases(&destination, &protected, &directories)?;
    wait.recheck()?;
    failures.recheck()?;
    payload.recheck()?;
    timeout.recheck()?;
    if fs::read(path).map_err(|error| error.to_string())? != bytes
        || fs::read(&policy_path).map_err(|error| error.to_string())? != policy
    {
        return Err("aggregate configuration or policy changed during checking".into());
    }
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "cycle18_bounded_composition_checked",
        "aggregate_config_sha256": sha256_hex(&bytes),
        "aggregate": aggregate,
        "limitations": [
            "Aggregate remains Unknown; bounded evidence is not full callback safety",
            "No complete production caller inventory or universal null/size bound is claimed",
            "Each configured leaf audit was rerun; serialized leaf reports were not consumed",
            "Runtime and linkage evidence remains producer-supplied, not attestation"
        ]
    });
    crate::wait_failures::write_atomic(&destination, &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?)
}
