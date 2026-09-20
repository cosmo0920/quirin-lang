//! Configured audit for the Cycle 18 callback task-handoff boundary.

use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::typed_source::{SourceResult, import_source_functions_from_profile};
use qcore_contract::profile_loader::load_source_set_profile;
use qcore_schemas::profile::ImportRole;
use qcore_schemas::sha256_hex;
use qcore_verification::callback_task_handoff::{
    CALLBACK_TASK_HANDOFF_EVIDENCE_VERSION, CALLBACK_TASK_HANDOFF_POLICY,
    CallbackTaskObservationBundle, check_callback_task_observations,
    check_taskful_callback_observations, project_callback_task_source,
    project_taskful_callback_source,
};
use serde::{Deserialize, Serialize};

use crate::observed_wait::reject_report_aliases;
use crate::observed_wait_inputs::load_monkey_source;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallbackTaskHandoffConfig {
    pub schema_version: u16,
    pub artifact: String,
    pub monkey_profile: PathBuf,
    pub fluent_checkout: PathBuf,
    pub monkey_inputs: PathBuf,
    pub source_compile_database: PathBuf,
    pub policy: PathBuf,
    pub observations: PathBuf,
    pub report: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskfulFixtureConfig {
    pub schema_version: u16,
    pub artifact: String,
    pub profile_directory: PathBuf,
    pub source: PathBuf,
    pub policy: PathBuf,
    pub observations: PathBuf,
    pub observer_binary: PathBuf,
    pub compiler: PathBuf,
    pub run_manifest: PathBuf,
    pub report: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TaskfulRunManifest {
    schema_version: u16,
    platform: String,
    threading: String,
    source_sha256: String,
    profile_sha256: String,
    policy_sha256: String,
    compiler_program: String,
    compiler_version: String,
    compiler_target: String,
    compile_argv: Vec<String>,
    binary_sha256: String,
    run_argv: Vec<String>,
    run_exit: i32,
    runtime_stdout_sha256: String,
    runtime_stderr_sha256: String,
    observations_sha256: String,
    callback_thread: u64,
    worker_thread: u64,
}

#[derive(Debug, Serialize)]
struct CallbackTaskHandoffReport {
    artifact: String,
    property: String,
    semantic_profile: String,
    scope: String,
    bounds: Vec<String>,
    assumptions: Vec<String>,
    evidence_version: String,
    source_revision: String,
    source_digest: String,
    policy_digest: String,
    callbacks_observed: Vec<String>,
    observation_count: usize,
    result: String,
}

fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    }
}

/// Run the configured source-plus-observation audit.
pub fn run_callback_task_handoff_config(path: &Path) -> Result<(), String> {
    let config_path = path.canonicalize().map_err(|error| error.to_string())?;
    let base = config_path.parent().ok_or("configuration has no parent")?;
    let config_bytes = fs::read(&config_path).map_err(|error| error.to_string())?;
    let config: CallbackTaskHandoffConfig =
        serde_json::from_slice(&config_bytes).map_err(|error| error.to_string())?;
    if config.schema_version != 1 || config.artifact.is_empty() {
        return Err("unsupported callback-task-handoff configuration".into());
    }
    let profile = resolve(base, &config.monkey_profile);
    let checkout = resolve(base, &config.fluent_checkout);
    let monkey_inputs = resolve(base, &config.monkey_inputs);
    let compile_database = resolve(base, &config.source_compile_database);
    let policy = resolve(base, &config.policy);
    let observations = resolve(base, &config.observations);
    let report = resolve(base, &config.report);
    let policy_bytes = fs::read(&policy).map_err(|error| error.to_string())?;
    if policy_bytes != CALLBACK_TASK_HANDOFF_POLICY.as_bytes() {
        return Err("configured callback task-handoff policy differs from the verifier".into());
    }
    let observation_bytes = fs::read(&observations).map_err(|error| error.to_string())?;
    let observation_bundle: CallbackTaskObservationBundle =
        serde_json::from_slice(&observation_bytes).map_err(|error| error.to_string())?;
    let inputs = load_monkey_source(
        &profile,
        &checkout,
        &monkey_inputs,
        &compile_database,
        &["cb_event", "cb_timeout", "cb_wait_2_timeout"],
    )?;
    let compile_task_digest = inputs.compile_task.digest()?;
    let source_snapshot = (
        inputs.revision.clone(),
        inputs.source_sha256.clone(),
        inputs.build_sha256.clone(),
    );
    let projection = project_callback_task_source(&inputs.summaries)
        .map_err(|error| format!("source projection refused: {error:?}"))?;
    let certificate = check_callback_task_observations(&projection, &observation_bundle)
        .map_err(|error| format!("observation check refused: {error:?}"))?;

    let mut protected = vec![
        config_path.clone(),
        policy.clone(),
        observations.clone(),
        monkey_inputs.clone(),
        compile_database.clone(),
    ];
    protected.extend(inputs.protected_files.clone());
    reject_report_aliases(&report, &protected, &inputs.protected_directories)?;
    let output = CallbackTaskHandoffReport {
        artifact: config.artifact,
        property: "callback-task-handoff-and-deferred-work-lifetime".into(),
        semantic_profile: "cycle18.callback-task-handoff.v1".into(),
        scope: "three selected Monkey callbacks; eight typed source paths; supplied bounded callback windows".into(),
        bounds: vec![
            "opaque callees require an exact declared no-task/no-defer effect contract".into(),
            "the supplied observation proves absence only in the recorded callback windows".into(),
            "generic positive task/deferred mechanics are model checks, not Monkey source evidence".into(),
        ],
        assumptions: vec![
            "faithful typed C extraction".into(),
            "declared callee effect contracts are trusted".into(),
            "native observation producer and callback-window completeness are trusted".into(),
            "observer capability declaration completely records task and deferred lifecycle actions".into(),
            "Lean-model-to-Rust correspondence is maintained by review".into(),
        ],
        evidence_version: CALLBACK_TASK_HANDOFF_EVIDENCE_VERSION.into(),
        source_revision: projection.source_revision().into(),
        source_digest: projection.source_digest().into(),
        policy_digest: projection.policy_digest().into(),
        callbacks_observed: certificate.callbacks_observed,
        observation_count: certificate.observation_count,
        result: "bounded obligations checked; no aggregate Cycle 18 promotion".into(),
    };
    let encoded = serde_json::to_vec_pretty(&output).map_err(|error| error.to_string())?;
    // Re-read mutable configured inputs before publication.
    if fs::read(&config_path).map_err(|error| error.to_string())? != config_bytes
        || fs::read(&policy).map_err(|error| error.to_string())? != policy_bytes
        || fs::read(&observations).map_err(|error| error.to_string())? != observation_bytes
    {
        return Err("configured inputs changed during callback task-handoff audit".into());
    }
    let reloaded = load_monkey_source(
        &profile,
        &checkout,
        &monkey_inputs,
        &compile_database,
        &["cb_event", "cb_timeout", "cb_wait_2_timeout"],
    )?;
    if reloaded.compile_task.digest()? != compile_task_digest
        || (
            reloaded.revision,
            reloaded.source_sha256,
            reloaded.build_sha256,
        ) != source_snapshot
    {
        return Err("source, profile, or compilation changed during audit".into());
    }
    if sha256_hex(CALLBACK_TASK_HANDOFF_POLICY.as_bytes()) != projection.policy_digest() {
        return Err("policy identity changed during callback task-handoff audit".into());
    }
    crate::wait_failures::write_atomic(&report, &encoded)
}

/// Audit the separate taskful pthread fixture against exact source-derived operations.
pub fn run_taskful_fixture_config(path: &Path) -> Result<(), String> {
    let config_path = path.canonicalize().map_err(|error| error.to_string())?;
    let base = config_path.parent().ok_or("configuration has no parent")?;
    let config_bytes = fs::read(&config_path).map_err(|error| error.to_string())?;
    let config: TaskfulFixtureConfig =
        serde_json::from_slice(&config_bytes).map_err(|error| error.to_string())?;
    if config.schema_version != 1 || config.artifact.is_empty() {
        return Err("unsupported taskful callback fixture configuration".into());
    }
    let profile_directory = resolve(base, &config.profile_directory);
    let source_path = resolve(base, &config.source);
    let policy_path = resolve(base, &config.policy);
    let observation_path = resolve(base, &config.observations);
    let binary_path = resolve(base, &config.observer_binary);
    let compiler_path = resolve(base, &config.compiler);
    let manifest_path = resolve(base, &config.run_manifest);
    let report_path = resolve(base, &config.report);
    let source_bytes = fs::read(&source_path).map_err(|error| error.to_string())?;
    let source_text = std::str::from_utf8(&source_bytes).map_err(|error| error.to_string())?;
    let policy_bytes = fs::read(&policy_path).map_err(|error| error.to_string())?;
    if policy_bytes != CALLBACK_TASK_HANDOFF_POLICY.as_bytes() {
        return Err("configured callback task-handoff policy differs from the verifier".into());
    }
    let observation_bytes = fs::read(&observation_path).map_err(|error| error.to_string())?;
    let observations: CallbackTaskObservationBundle =
        serde_json::from_slice(&observation_bytes).map_err(|error| error.to_string())?;
    let binary_bytes = fs::read(&binary_path).map_err(|error| error.to_string())?;
    if binary_bytes.is_empty() {
        return Err("taskful observer binary is empty".into());
    }
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| error.to_string())?;
    let manifest: TaskfulRunManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| error.to_string())?;
    let profile_path = profile_directory.join("profile.json");
    let profile_bytes = fs::read(&profile_path).map_err(|error| error.to_string())?;
    let compiler_identity = qcore_c_gateway::ClangIdentity::capture()?;
    let source_name = source_path.to_string_lossy().into_owned();
    let binary_name = binary_path.to_string_lossy().into_owned();
    let compiler_name = compiler_path.to_string_lossy().into_owned();
    let expected_compile_argv = vec![
        compiler_name.clone(),
        "-std=c11".into(),
        "-pthread".into(),
        "-Wall".into(),
        "-Wextra".into(),
        "-Werror".into(),
        "-Wno-unused-parameter".into(),
        "-fsanitize=address,undefined".into(),
        source_name,
        "-o".into(),
        binary_name.clone(),
    ];
    if manifest.schema_version != 1
        || manifest.platform != "linux"
        || manifest.threading != "pthreads"
        || manifest.source_sha256 != sha256_hex(&source_bytes)
        || manifest.profile_sha256 != sha256_hex(&profile_bytes)
        || manifest.policy_sha256 != sha256_hex(&policy_bytes)
        || manifest.binary_sha256 != sha256_hex(&binary_bytes)
        || manifest.observations_sha256 != sha256_hex(&observation_bytes)
        || manifest.runtime_stdout_sha256 != manifest.observations_sha256
        || manifest.runtime_stderr_sha256 != sha256_hex(b"")
        || manifest.run_exit != 0
        || manifest.compile_argv != expected_compile_argv
        || manifest.run_argv != [binary_name]
        || manifest.compiler_program != compiler_identity.program()
        || manifest.compiler_version != compiler_identity.version()
        || manifest.compiler_target != compiler_identity.target()
        || manifest.compiler_program != compiler_name
        || manifest.callback_thread == manifest.worker_thread
    {
        return Err("taskful producer manifest does not match configured inputs/run".into());
    }
    let profile = load_source_set_profile(&profile_directory, "cycle18-task-handoff")
        .map_err(|error| error.to_string())?;
    let import = |role| {
        import_source_functions_from_profile(
            &C11TranslationUnit {
                source: source_text,
                ..Default::default()
            },
            "229578d8ad617803bed3f02bcc1f0f59251d560d",
            &profile,
            "fixture",
            role,
        )
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|result| match result {
            SourceResult::Summary(summary) => Ok(summary),
            SourceResult::Unknown { function, reason } => {
                Err(format!("taskful source `{function}` refused: {reason}"))
            }
        })
        .collect::<Result<Vec<_>, String>>()
    };
    let callbacks = import(ImportRole::Callback)?;
    let wrappers = import(ImportRole::Wrapper)?;
    let ([callback], [after_return]) = (callbacks.as_slice(), wrappers.as_slice()) else {
        return Err("taskful fixture did not import exactly one callback and wrapper".into());
    };
    let projection = project_taskful_callback_source(callback, after_return)
        .map_err(|error| format!("taskful source projection refused: {error:?}"))?;
    let certificate = check_taskful_callback_observations(&projection, &observations)
        .map_err(|error| format!("taskful observation refused: {error:?}"))?;
    let callback_threads = observations
        .records
        .iter()
        .filter(|record| !matches!(record.action,
            qcore_verification::callback_task_handoff::CallbackTaskObservationAction::TaskStart { .. }
                | qcore_verification::callback_task_handoff::CallbackTaskObservationAction::TaskCancelAck { .. }))
        .filter_map(|record| record.thread)
        .collect::<std::collections::BTreeSet<_>>();
    let worker_threads = observations
        .records
        .iter()
        .filter(|record| matches!(record.action,
            qcore_verification::callback_task_handoff::CallbackTaskObservationAction::TaskStart { .. }
                | qcore_verification::callback_task_handoff::CallbackTaskObservationAction::TaskCancelAck { .. }))
        .filter_map(|record| record.thread)
        .collect::<std::collections::BTreeSet<_>>();
    if callback_threads != [manifest.callback_thread].into_iter().collect()
        || worker_threads != [manifest.worker_thread].into_iter().collect()
    {
        return Err("taskful producer manifest thread identities do not match observations".into());
    }
    let protected = vec![
        config_path.clone(),
        source_path.clone(),
        policy_path.clone(),
        observation_path.clone(),
        binary_path.clone(),
        compiler_path.clone(),
        manifest_path.clone(),
        profile_path.clone(),
    ];
    reject_report_aliases(
        &report_path,
        &protected,
        std::slice::from_ref(&profile_directory),
    )?;
    if fs::read(&config_path).map_err(|error| error.to_string())? != config_bytes
        || fs::read(&source_path).map_err(|error| error.to_string())? != source_bytes
        || fs::read(&policy_path).map_err(|error| error.to_string())? != policy_bytes
        || fs::read(&observation_path).map_err(|error| error.to_string())? != observation_bytes
        || fs::read(&binary_path).map_err(|error| error.to_string())? != binary_bytes
        || fs::read(&manifest_path).map_err(|error| error.to_string())? != manifest_bytes
        || fs::read(&profile_path).map_err(|error| error.to_string())? != profile_bytes
        || qcore_c_gateway::ClangIdentity::capture()? != compiler_identity
    {
        return Err("taskful source/build/observation changed during audit".into());
    }
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "bounded_taskful_callback_handoff_checked",
        "artifact": config.artifact,
        "property": "callback-task-handoff-and-deferred-work-lifetime",
        "semantic_profile": "cycle18.callback-task-handoff.pthread-fixture.v1",
        "scope": "one checked-in taskful callback fixture and one Linux pthread observation",
        "bounds": ["lifetime/custody only; no payload permission or race-freedom claim",
            "fixture semantics do not imply Monkey or other production callbacks spawn tasks"],
        "assumptions": ["faithful typed C extraction", "declared helper effect contracts",
            "observer capability completeness and producer truth", "Lean-model-to-Rust correspondence"],
        "evidence_version": CALLBACK_TASK_HANDOFF_EVIDENCE_VERSION,
        "identities": {"source_sha256": sha256_hex(&source_bytes),
            "source_summary_sha256": projection.source_digest(),
            "policy_sha256": projection.policy_digest(), "observations_sha256": sha256_hex(&observation_bytes),
            "observer_binary_sha256": sha256_hex(&binary_bytes),
            "producer_manifest_sha256": sha256_hex(&manifest_bytes),
            "compiler": compiler_identity},
        "certificate": certificate
    });
    crate::wait_failures::write_atomic(
        &report_path,
        &serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
}
