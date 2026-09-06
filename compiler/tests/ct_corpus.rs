//! W6 CT conformance harness (`docs/GENERALIZATION_PLAN.md` §W6 task 1).
//!
//! The harness is keyed to `docs/reevaluation/ct_manifest.json`, the tracking
//! table for the spec §55 language conformance corpus (CT01–CT50). Every case
//! the manifest claims is `mapped` must be backed by a real QCore source
//! fixture under `compiler/tests/fixtures/ct/<ID>/` and is executed through
//! the actual source pipeline (`compile_files`: parse → elaborate → check,
//! plus the CFG reference execution for accepted executables). Reevaluation
//! spec §5.1: hand-constructed Rust events do not mark a source conformance
//! case complete, so this harness never builds semantics by hand.
//!
//! Unmapped cases are counted explicitly, never silently skipped, and the
//! pinned `MAPPED_CASES` table below is an intentional expectation: advancing
//! a case requires updating the manifest, adding an executed fixture, and
//! editing this table deliberately. A manifest status the fixture set cannot
//! back (mapped without a fixture, a fixture without a mapped status, or an
//! unknown status vocabulary) fails the harness.
//!
//! Namespace discipline (§W6 task 3): CT is the language corpus only. WM/VP/LC
//! probe regressions and per-library provider cases must not be added here.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_compiler::{
    CompileOptions, ExecutionOutcome, SourceFile, TraceEvent, compile_files, format_diagnostic,
};
use serde::Deserialize;

/// Pinned mapping expectation. Exactly these manifest entries carry
/// `mapping_status: "mapped"`; everything else must still read `unmapped`.
const MAPPED_CASES: &[&str] = &[
    "CT01", "CT02", "CT12", "CT13", "CT15", "CT16", "CT19", "CT37",
];
const TOTAL_CASES: usize = 50;
const STAGE_NAMES: &[&str] = &[
    "parser",
    "type_checker",
    "reference_execution",
    "cfg",
    "native_backend",
    "proof",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    #[allow(dead_code)]
    note: String,
    cases: Vec<ManifestCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestCase {
    id: String,
    #[allow(dead_code)]
    scenario: String,
    #[allow(dead_code)]
    expected: String,
    spec_section: String,
    mapping_status: String,
    source_fixture: Option<String>,
    semantic_profile: Option<String>,
    stages: BTreeMap<String, String>,
    #[serde(default)]
    evidence_target: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCase {
    id: String,
    verdict: String,
    entry: Option<String>,
    diagnostics: Vec<ExpectedDiagnostic>,
    outcome: Option<ExpectedOutcome>,
    trace: Option<Vec<String>>,
    stages_executed: Vec<String>,
    #[allow(dead_code)]
    notes: String,
    #[serde(default)]
    evidence_target: Option<String>,
    #[serde(default)]
    native_join_failure: Option<NativeJoinFailure>,
    #[serde(default)]
    native_create_recovery: Option<NativeCreateRecovery>,
    #[serde(default)]
    native_owned_transfer: Option<NativeOwnedTransfer>,
    #[serde(default)]
    positive_control: Option<PositiveControl>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeOwnedTransfer {
    header: String,
    fail_create: bool,
    expected_i32: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeCreateRecovery {
    header: String,
    unjoined_source: String,
    success_i32: i32,
    recovery_i32: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeJoinFailure {
    header: String,
    control_i32: i32,
    abort_exit_code: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PositiveControl {
    source: String,
    evidence_target: String,
    expected_i32: i32,
    joined_children: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedDiagnostic {
    code: String,
    message_contains: String,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ExpectedOutcome {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    ok_value: Option<i32>,
    /// Completion-record identities still live at a normal exit (spec §43.5,
    /// §50.3). Mandatory for an `ok_value` outcome so that a case cannot claim
    /// a normal exit without stating what the capsule still carries; an error
    /// exit carries no completion payload in this model.
    #[serde(default)]
    payload: Option<Vec<u64>>,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler crate lives one level below the repository root")
        .to_path_buf()
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ct")
}

fn load_manifest() -> Manifest {
    let path = repo_root().join("docs/reevaluation/ct_manifest.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()))
}

fn trace_token(event: &TraceEvent) -> String {
    match event {
        TraceEvent::Acquire(resource) => format!("acquire:{resource}"),
        TraceEvent::Release(resource) => format!("release:{resource}"),
        TraceEvent::Transfer(resource) => format!("transfer:{resource}"),
    }
}

fn run_mapped_case(case: &ManifestCase) {
    let directory = fixture_root().join(&case.id);
    assert!(
        directory.is_dir(),
        "{}: manifest claims `mapped` but fixture directory {} does not exist",
        case.id,
        directory.display()
    );
    let expected_path = directory.join("expected.json");
    let expected: ExpectedCase = serde_json::from_str(
        &fs::read_to_string(&expected_path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", expected_path.display())),
    )
    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", expected_path.display()));
    assert_eq!(expected.id, case.id, "expected.json id mismatch");
    assert_eq!(
        expected.evidence_target, case.evidence_target,
        "{}: target metadata mismatch",
        case.id
    );
    assert_eq!(
        expected.native_join_failure.is_some(),
        expected.verdict == "native_join_failure",
        "{}: native fault mode and fixture metadata must agree",
        case.id
    );
    assert_eq!(
        expected.native_create_recovery.is_some(),
        expected.verdict == "native_create_recovery"
    );
    assert_eq!(
        expected.native_owned_transfer.is_some(),
        expected.verdict == "native_owned_transfer"
    );
    assert_eq!(
        expected.evidence_target.as_deref(),
        (expected.native_join_failure.is_some()
            || expected.native_create_recovery.is_some()
            || expected.native_owned_transfer.is_some())
        .then_some("linux"),
        "{}: only the native fault mode carries target-scoped evidence",
        case.id
    );
    assert_eq!(
        expected.positive_control.is_some(),
        case.id == "CT19",
        "CT19 must retain its explicitly named joined-sibling positive control"
    );
    let mut required_files = BTreeSet::from(["case.qcore".to_owned(), "expected.json".to_owned()]);
    if let Some(native) = &expected.native_owned_transfer {
        assert!(["CT01", "CT12"].contains(&case.id.as_str()));
        assert_eq!(native.header, "owned_transfer.h");
        assert_eq!(native.fail_create, case.id == "CT12");
        assert_eq!(native.expected_i32, 42);
        required_files.insert(native.header.clone());
    }
    if let Some(native) = &expected.native_join_failure {
        assert_eq!(case.id, "CT15");
        assert_eq!(native.header, "join_failure.h");
        assert_eq!(native.abort_exit_code, 77);
        required_files.insert(native.header.clone());
    }
    if let Some(native) = &expected.native_create_recovery {
        assert_eq!(case.id, "CT13");
        assert_eq!(native.header, "create_failure.h");
        assert_eq!(native.unjoined_source, "unjoined_failure.qcore");
        assert_eq!(native.success_i32, 7);
        assert_eq!(native.recovery_i32, 42);
        required_files.insert(native.header.clone());
        required_files.insert(native.unjoined_source.clone());
    }
    if let Some(control) = &expected.positive_control {
        assert_eq!(control.source, "joined_sibling.qcore");
        assert_eq!(control.evidence_target, "linux");
        assert_eq!(control.joined_children, 2);
        required_files.insert(control.source.clone());
    }
    let actual_files = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(
                entry.file_type().unwrap().is_file(),
                "CT fixtures cannot hide nested artifacts"
            );
            entry.file_name().into_string().unwrap()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual_files, required_files,
        "{}: fixture inventory differs from executed inputs",
        case.id
    );

    let source_path = directory.join("case.qcore");
    assert!(
        source_path.is_file(),
        "{}: fixture source {} is missing",
        case.id,
        source_path.display()
    );
    let recorded = case.source_fixture.as_deref().unwrap_or_else(|| {
        panic!(
            "{}: a mapped case must record its source fixture path in the manifest",
            case.id
        )
    });
    assert_eq!(
        repo_root().join(recorded).canonicalize().ok(),
        source_path.canonicalize().ok(),
        "{}: manifest source_fixture `{recorded}` does not name the fixture source",
        case.id
    );
    let profile = case.semantic_profile.as_deref().unwrap_or_else(|| {
        panic!(
            "{}: a mapped case must declare its semantic profile in the manifest",
            case.id
        )
    });
    assert!(
        !profile.trim().is_empty(),
        "{}: empty semantic profile",
        case.id
    );

    // Per-stage manifest statuses must match exactly what this harness runs.
    let executed: BTreeSet<&str> = expected
        .stages_executed
        .iter()
        .map(String::as_str)
        .collect();
    assert!(
        executed.contains("parser") && executed.contains("type_checker"),
        "{}: a mapped case must at least execute the parser and type checker",
        case.id
    );
    let allowed_stages: BTreeSet<&str> = match expected.verdict.as_str() {
        "native_join_failure" | "native_create_recovery" | "native_owned_transfer" => [
            "parser",
            "type_checker",
            "reference_execution",
            "cfg",
            "native_backend",
        ]
        .into_iter()
        .collect(),
        "accept" if expected.outcome.is_some() => {
            ["parser", "type_checker", "reference_execution", "cfg"]
                .into_iter()
                .collect()
        }
        "accept" | "reject" => ["parser", "type_checker"].into_iter().collect(),
        other => panic!("{}: unknown verdict vocabulary `{other}`", case.id),
    };
    assert_eq!(
        executed, allowed_stages,
        "{}: claimed stages must match this mode's actual runner",
        case.id
    );
    for stage in &executed {
        assert!(
            STAGE_NAMES.contains(stage),
            "{}: unknown stage `{stage}` in expected.json",
            case.id
        );
    }
    for (stage, status) in &case.stages {
        let want = if executed.contains(stage.as_str()) {
            "executed"
        } else {
            "unassessed"
        };
        assert_eq!(
            status, want,
            "{}: manifest stage `{stage}` must be `{want}` to match the executed fixture",
            case.id
        );
    }

    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", source_path.display()));
    let files = [SourceFile::new(recorded.to_string(), source)];
    let result = compile_files(
        &files,
        &CompileOptions {
            entry: expected.entry.clone(),
        },
    );

    if let Some(control) = &expected.positive_control {
        run_positive_control(&directory, control);
    }
    match expected.verdict.as_str() {
        "native_owned_transfer" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_native_owned_transfer(&directory, &expected, result);
        }
        "native_create_recovery" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_native_create_recovery(&directory, &expected, result);
        }
        "native_join_failure" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none(),
                "native fault mode has its own explicit outcome expectations"
            );
            run_native_join_failure(&directory, &expected, result);
        }
        "accept" => {
            assert!(
                expected.diagnostics.is_empty(),
                "{}: an accept case must not expect diagnostics",
                case.id
            );
            let module = result.unwrap_or_else(|diagnostics| {
                panic!(
                    "{}: expected acceptance but the checker rejected:\n{}",
                    case.id,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            });
            let wants_execution = executed.contains("reference_execution");
            assert_eq!(
                wants_execution,
                expected.outcome.is_some(),
                "{}: reference_execution stage and expected outcome must agree",
                case.id
            );
            if let Some(outcome) = &expected.outcome {
                let observation = module.reference_observation.as_ref().unwrap_or_else(|| {
                    panic!(
                        "{}: expected a reference execution observation (executable module)",
                        case.id
                    )
                });
                match &observation.outcome {
                    ExecutionOutcome::Ok { value, payload } => {
                        assert_eq!(outcome.error, None, "{}: expected error outcome", case.id);
                        assert_eq!(
                            outcome.ok_value,
                            Some(*value),
                            "{}: reference execution returned {value}",
                            case.id
                        );
                        let expected_payload = outcome.payload.as_deref().unwrap_or_else(|| {
                            panic!(
                                "{}: a normal-exit case must pin the completion-record payload",
                                case.id
                            )
                        });
                        assert_eq!(
                            expected_payload,
                            payload.as_slice(),
                            "{}: completion-record payload mismatch",
                            case.id
                        );
                    }
                    ExecutionOutcome::Error { name, .. } => {
                        assert_eq!(
                            outcome.ok_value, None,
                            "{}: expected ok outcome, got error `{name}`",
                            case.id
                        );
                        assert!(
                            outcome.payload.is_none(),
                            "{}: an error exit carries no completion-record payload",
                            case.id
                        );
                        assert_eq!(
                            outcome.error.as_deref(),
                            Some(name.as_str()),
                            "{}: reference execution raised `{name}`",
                            case.id
                        );
                    }
                }
                let actual_trace: Vec<String> = observation.trace.iter().map(trace_token).collect();
                let expected_trace = expected.trace.as_ref().unwrap_or_else(|| {
                    panic!(
                        "{}: an executed accept case must pin its acquisition/release trace",
                        case.id
                    )
                });
                assert_eq!(
                    &actual_trace, expected_trace,
                    "{}: reference execution trace mismatch",
                    case.id
                );
            }
        }
        "reject" => {
            assert!(
                expected.outcome.is_none() && expected.trace.is_none(),
                "{}: a reject case has no execution post-state",
                case.id
            );
            assert!(
                !expected.diagnostics.is_empty(),
                "{}: a reject case must pin at least one expected diagnostic",
                case.id
            );
            let diagnostics = match result {
                Err(diagnostics) => diagnostics,
                Ok(_) => panic!(
                    "{}: expected rejection but the checker accepted the fixture",
                    case.id
                ),
            };
            let rendered = diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n");
            for want in &expected.diagnostics {
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == want.code
                            && diagnostic.message.contains(&want.message_contains)),
                    "{}: no diagnostic with code `{}` containing `{}`; actual:\n{rendered}",
                    case.id,
                    want.code,
                    want.message_contains
                );
            }
        }
        other => panic!("{}: unknown verdict vocabulary `{other}`", case.id),
    }
}

#[test]
fn ct_manifest_and_fixture_set_agree_and_mapped_cases_execute() {
    let manifest = load_manifest();
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.cases.len(), TOTAL_CASES);

    let mut mapped = Vec::new();
    let mut unmapped = Vec::new();
    for (index, case) in manifest.cases.iter().enumerate() {
        assert_eq!(
            case.id,
            format!("CT{:02}", index + 1),
            "manifest cases must stay CT01..CT50 in order"
        );
        assert_eq!(
            case.spec_section, "55",
            "{}: CT cases pin spec §55",
            case.id
        );
        let mut stage_keys: Vec<&str> = case.stages.keys().map(String::as_str).collect();
        stage_keys.sort_unstable();
        let mut expected_keys: Vec<&str> = STAGE_NAMES.to_vec();
        expected_keys.sort_unstable();
        assert_eq!(stage_keys, expected_keys, "{}: stage schema drift", case.id);

        let directory = fixture_root().join(&case.id);
        match case.mapping_status.as_str() {
            "unmapped" => {
                assert!(
                    !directory.exists(),
                    "{}: fixture directory exists but the manifest still says `unmapped` — \
                     advance the manifest (and MAPPED_CASES) deliberately or remove the fixture",
                    case.id
                );
                assert!(
                    case.source_fixture.is_none()
                        && case.semantic_profile.is_none()
                        && case.evidence_target.is_none(),
                    "{}: an unmapped case must not carry fixture metadata",
                    case.id
                );
                assert!(
                    case.stages.values().all(|status| status == "unassessed"),
                    "{}: an unmapped case must keep all stages `unassessed`",
                    case.id
                );
                unmapped.push(case.id.clone());
            }
            "mapped" => {
                run_mapped_case(case);
                mapped.push(case.id.clone());
            }
            other => panic!(
                "{}: unknown mapping_status `{other}` — the harness fails closed on \
                 unrecognized status vocabulary",
                case.id
            ),
        }
    }

    assert_eq!(
        mapped, MAPPED_CASES,
        "mapped case set drifted from the pinned expectation"
    );
    assert_eq!(
        unmapped.len(),
        TOTAL_CASES - MAPPED_CASES.len(),
        "unmapped CT cases ({}): {unmapped:?}",
        unmapped.len()
    );
}

/// The moving count is the W6 genericity metric: 8 of 50 CT cases are mapped
/// as executed source-level tests; 42 remain unmapped. CT13/CT15 native evidence is Linux-scoped. This test exists so the
/// counts are visible and any drift is a deliberate edit, not an accident.
#[test]
fn ct_mapping_progress_is_pinned_and_counted() {
    let manifest = load_manifest();
    let mapped = manifest
        .cases
        .iter()
        .filter(|case| case.mapping_status == "mapped")
        .count();
    let unmapped = manifest
        .cases
        .iter()
        .filter(|case| case.mapping_status == "unmapped")
        .count();
    assert_eq!(mapped, MAPPED_CASES.len(), "mapped CT case count");
    assert_eq!(
        unmapped,
        TOTAL_CASES - MAPPED_CASES.len(),
        "unmapped CT case count"
    );
    assert_eq!(mapped + unmapped, TOTAL_CASES, "every case is counted");
}

fn unsupported_target(diagnostics: &[qcore_compiler::FileDiagnostic]) {
    assert_eq!(
        diagnostics.len(),
        1,
        "unsupported target must follow successful source/core checks"
    );
    assert_eq!(diagnostics[0].code, "Q500");
    assert!(
        diagnostics[0]
            .message
            .contains("requires a Linux C11 threads target"),
        "{diagnostics:?}"
    );
}

fn run_positive_control(directory: &Path, expected: &PositiveControl) {
    let path = directory.join(&expected.source);
    let source = fs::read_to_string(&path).unwrap();
    let result = compile_files(
        &[SourceFile::new(path.to_string_lossy(), source)],
        &CompileOptions::default(),
    );
    if !cfg!(target_os = "linux") {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT19 joined-sibling control: source/core checked; Linux reference/native evidence unexecuted on this host"
        );
        return;
    }
    let module =
        result.expect("joined siblings must be accepted, so rejection is not a syntax gate");
    let reference = qcore_core::task_execution::execute_task_module(
        module
            .checked
            .as_scoped_tasks()
            .expect("control must use actual tasks"),
        &qcore_core::task_execution::TaskExecutionScript::default(),
    )
    .unwrap();
    assert_eq!(
        reference.root_outcome,
        qcore_core::scoped_tasks::TaskOutcome::I32(expected.expected_i32)
    );
    let children = reference
        .tasks
        .iter()
        .filter(|task| task.parent.is_some())
        .collect::<Vec<_>>();
    assert_eq!(children.len(), expected.joined_children);
    assert!(
        children.iter().all(|task| task.joined),
        "positive control must explicitly collect both siblings"
    );
}

fn run_native_join_failure(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    let native = expected.native_join_failure.as_ref().unwrap();
    assert!(directory.join(&native.header).is_file());
    if !cfg!(target_os = "linux") {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT15: source/core checked; Linux native/reference execution unexecuted on this host (manifest evidence_target=linux)"
        );
        return;
    }
    let module = result.expect("CT15 source must check before native fault injection");
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("CT15 must use the task execution artifact");
    assert!(
        module.reference_observation.is_none(),
        "do not substitute the sequential execution profile"
    );
    let reference = qcore_core::task_execution::execute_task_module(
        checked,
        &qcore_core::task_execution::TaskExecutionScript::default(),
    )
    .unwrap();
    assert_eq!(
        reference.root_outcome,
        qcore_core::scoped_tasks::TaskOutcome::I32(native.control_i32)
    );
    let children = reference
        .tasks
        .iter()
        .filter(|task| task.parent.is_some())
        .collect::<Vec<_>>();
    assert_eq!(children.len(), 1);
    assert!(children[0].joined);
    let transcript = qcore_core::task_execution::render_task_observation(&reference);
    let normal = native_join_run(
        module.c_source.as_deref().unwrap(),
        &directory.join(&native.header),
        false,
    );
    assert!(normal.status.success(), "CT15 control failed: {normal:?}");
    assert_eq!(normal.stdout, transcript.as_bytes());
    assert_eq!(
        normal.stderr, b"JOIN_OK\nFREE\n",
        "control must really join and reclaim its environment"
    );
    let failed = native_join_run(
        module.c_source.as_deref().unwrap(),
        &directory.join(&native.header),
        true,
    );
    assert_eq!(
        failed.status.code(),
        Some(native.abort_exit_code),
        "CT15 must take process abort policy: {failed:?}"
    );
    assert_eq!(
        failed.stderr, b"JOIN_FAILED\n",
        "failed join must not reclaim its environment"
    );
    let spawn = transcript
        .lines()
        .next()
        .expect("control must publish spawn");
    assert!(spawn.starts_with("QCORE_TASK_SPAWN "));
    assert_eq!(
        failed.stdout,
        format!("{spawn}\n").as_bytes(),
        "failed join must not publish initialized completion, quiescence or a parent outcome"
    );
}

fn native_join_run(source: &str, header: &Path, fail_join: bool) -> std::process::Output {
    native_fault_run(
        source,
        header,
        if fail_join {
            "-DQCORE_CT_FAIL_JOIN=1"
        } else {
            "-DQCORE_CT_FAIL_JOIN=0"
        },
    )
}

fn native_fault_run(source: &str, header: &Path, define: &str) -> std::process::Output {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "qcore-ct-native-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let c_path = directory.join("module.c");
    let exe_path = directory.join("module");
    fs::write(&c_path, source).unwrap();
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".to_owned());
    let compiled = Command::new(compiler)
        .args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"])
        .arg(define)
        .arg("-include")
        .arg(header)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "CT emitted C failed: {} (artifact {})",
        String::from_utf8_lossy(&compiled.stderr),
        c_path.display()
    );
    let output = Command::new(&exe_path).output().unwrap();
    fs::remove_dir_all(directory).unwrap();
    output
}

fn run_native_create_recovery(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::scoped_tasks::TaskOutcome;
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module, render_task_observation,
    };
    let native = expected.native_create_recovery.as_ref().unwrap();
    // The corresponding failure arm that omits its join must reject at the
    // task-state gate, not merely because matching syntax is unsupported.
    let bad_path = directory.join(&native.unjoined_source);
    let errors = compile_files(
        &[SourceFile::new(
            bad_path.to_string_lossy(),
            fs::read_to_string(&bad_path).unwrap(),
        )],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "Q600" && error.message.contains("ScopeBusy")),
        "{errors:?}"
    );
    if !cfg!(target_os = "linux") {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT13: source/core checked; Linux native/reference creation-recovery evidence unexecuted on this host"
        );
        return;
    }
    let module = result.expect("CT13 recovery source must check before fault injection");
    let checked = module.checked.as_scoped_tasks().unwrap();
    for (failed_create, decisions, outcome, children, stderr) in [
        (
            0,
            vec![true, true],
            TaskOutcome::I32(native.success_i32),
            2,
            "CREATE_OK:1\nCREATE_OK:2\nJOIN_OK:2\nFREE\nJOIN_OK:1\nFREE\n",
        ),
        (
            1,
            vec![false],
            TaskOutcome::Error("SpawnError".into()),
            0,
            "CREATE_FAILED:1\nFREE\n",
        ),
        (
            2,
            vec![true, false],
            TaskOutcome::I32(native.recovery_i32),
            1,
            "CREATE_OK:1\nCREATE_FAILED:2\nFREE\nJOIN_OK:1\nFREE\n",
        ),
    ] {
        let reference = execute_task_module(
            checked,
            &TaskExecutionScript {
                spawn_outcomes: decisions,
                allocation_outcomes: vec![],
            },
        )
        .unwrap();
        assert_eq!(reference.root_outcome, outcome);
        let actual_children = reference
            .tasks
            .iter()
            .filter(|task| task.parent.is_some())
            .collect::<Vec<_>>();
        assert_eq!(actual_children.len(), children);
        assert!(actual_children.iter().all(|task| task.joined));
        let run = native_fault_run(
            module.c_source.as_deref().unwrap(),
            &directory.join(&native.header),
            &format!("-DQCORE_CT_FAIL_CREATE={failed_create}"),
        );
        assert!(
            run.status.success(),
            "CT13 native mode {failed_create} failed: {run:?}"
        );
        assert_eq!(run.stdout, render_task_observation(&reference).as_bytes());
        assert_eq!(
            run.stderr,
            stderr.as_bytes(),
            "must join the exact earlier native thread after second creation failure"
        );
    }
}

fn run_native_owned_transfer(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::scoped_tasks::TaskOutcome;
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module, render_task_observation,
    };
    let native = expected.native_owned_transfer.as_ref().unwrap();
    if !cfg!(target_os = "linux") {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: Linux owned-task reference/native evidence unexecuted on this host",
            expected.id
        );
        return;
    }
    let module = result.expect("owned-task source must pass all-path ownership checks");
    assert!(module.reference_observation.is_none());
    let checked = module.checked.as_scoped_tasks().unwrap();
    let scripts = if native.fail_create {
        vec![false, true]
    } else {
        vec![true]
    };
    for succeeds in scripts {
        let reference = execute_task_module(
            checked,
            &TaskExecutionScript {
                spawn_outcomes: vec![succeeds],
                allocation_outcomes: vec![true],
            },
        )
        .unwrap();
        assert_eq!(
            reference.root_outcome,
            TaskOutcome::I32(native.expected_i32)
        );
        assert_eq!(reference.tasks.len(), if succeeds { 2 } else { 1 });
        if succeeds {
            assert_eq!(reference.tasks[1].outcome, TaskOutcome::OwnedBuffer(1));
            assert!(reference.tasks[1].joined);
        }
        let output = native_fault_run(
            module.c_source.as_deref().unwrap(),
            &directory.join(&native.header),
            if succeeds {
                "-DQCORE_CT_FAIL_CREATE=0"
            } else {
                "-DQCORE_CT_FAIL_CREATE=1"
            },
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            render_task_observation(&reference).as_bytes()
        );
        assert_eq!(
            output.stderr,
            if succeeds {
                b"BUFFER_ALLOC\nCREATE_OK\nJOIN_OK\nENV_FREE\nBUFFER_FREE\n".as_slice()
            } else {
                b"BUFFER_ALLOC\nCREATE_FAILED\nENV_FREE\nBUFFER_FREE\n".as_slice()
            }
        );
    }
}
