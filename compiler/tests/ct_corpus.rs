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
//!
//! Two source routes are represented. The task route (`ScopedCopyTasksV1`,
//! `ScopedOwnedTasksV1`, and its `ScopedReadSharesV1` / `ScopedErrorPayloadV1`
//! extensions) is the original eleven cases plus CT03/CT04/CT05/CT17/CT18. The
//! mutex route (`source.mutex.guard.v1` lowering onto the unedited
//! `CheckedMutexInvariantV1` checker, card T03) is CT26/CT27/CT29/CT30. The two
//! routes do not share a stage vocabulary: the mutex route emits directly from
//! its checked event list and has **no CFG stage**, so a mutex case must never
//! claim `cfg: executed`, while a task-route case reads `cfg` as the checked
//! task IR it really constructs (the manifest note states both readings).
//! Rejection cases on either route pin the whole ordered diagnostic cascade
//! rather than its head, because a refusal that silently dropped a custody
//! obligation would still satisfy a head-only assertion, and each carries at
//! least one control so the refusal is not merely "the profile cannot express
//! this at all".

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
    "CT01", "CT02", "CT03", "CT04", "CT05", "CT12", "CT13", "CT14", "CT15", "CT16", "CT17", "CT18",
    "CT19", "CT20", "CT26", "CT27", "CT29", "CT30", "CT37", "CT50",
];
/// The `source.mutex.guard.v1` rejection cases (T03 lowering onto the unedited
/// `CheckedMutexInvariantV1` checker). Each pins its exact ordered diagnostic
/// code vector, not merely its head, and carries at least one control that
/// distinguishes the refused construct from the admitted one.
const MUTEX_REJECT_CASES: &[&str] = &["CT27", "CT29", "CT30"];
/// The one accepted mutex-guard case, executed end to end and re-run under
/// ASan/UBSan/LSan.
const MUTEX_SANITIZED_CASE: &str = "CT26";
/// Task-route rejection cases (cards T05 and T06). Same discipline as the mutex
/// rejections: the exact ordered code vector plus at least one control that
/// isolates the refused element from the surrounding program.
const SOURCE_REJECT_CASES: &[&str] = &["CT04", "CT05", "CT18"];
/// Task-route accept cases whose native evidence is a sanitized run with a heap
/// witness: every Buffer allocation and release is recorded by address, so
/// "counted once" and "reclaims nothing" are checked by identity rather than by
/// the emitted trace alone.
const HEAP_SCENARIO_CASES: &[&str] = &["CT03", "CT17"];
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
    #[serde(default)]
    native_task_scenarios: Option<NativeTaskScenarios>,
    #[serde(default)]
    checked_mutex_sanitized: Option<CheckedMutexSanitized>,
    #[serde(default)]
    checked_mutex_reject: Option<CheckedMutexReject>,
    #[serde(default)]
    native_heap_scenarios: Option<NativeHeapScenarios>,
    #[serde(default)]
    source_reject: Option<SourceReject>,
}

/// Sanitized task-route native evidence with a heap witness (cards T05, T06).
/// Every scenario compiles the case's own emitted C with the recorded sanitizer
/// flags, includes one fixture header, and compares its stdout with the
/// independently computed reference execution of the same checked module. The
/// header prints each Buffer allocation and release with its address, so a
/// scenario states how many objects existed and how many were reclaimed rather
/// than trusting the trace.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeHeapScenarios {
    /// The `SourceExecutionProfile` the module must be published under. A case
    /// that silently changed route would otherwise still pass.
    execution_profile: String,
    alloc_prefix: String,
    free_prefix: String,
    sanitizer_flags: Vec<String>,
    ubsan_options: String,
    rejection_controls: Vec<TaskRejectionControl>,
    scenarios: Vec<HeapScenario>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeapScenario {
    name: String,
    /// Fixture header included with `-include`; other fixture headers are
    /// copied beside it so a header may `#include "track.h"`.
    header: String,
    /// `-D` options selecting the injected failure, if any.
    defines: Vec<String>,
    spawn_outcomes: Vec<bool>,
    allocation_outcomes: Vec<bool>,
    #[serde(default)]
    worker_allocation_outcomes: BTreeMap<u64, Vec<bool>>,
    /// LeakSanitizer. Disabled only where the retained allocation *is* the
    /// observation; at least one scenario per case must enable it.
    detect_leaks: bool,
    /// Process exit code. Zero means the run completed, and exactly then its
    /// stdout must equal the reference observation; a nonzero code is a trapped
    /// abort whose stdout is a truncated prefix, so such a scenario states what
    /// must be absent instead.
    exit_code: i32,
    stdout_contains: Vec<String>,
    stdout_absent: Vec<String>,
    /// Exact stdout lines that must appear in this relative order.
    stdout_order: Vec<String>,
    heap_allocations: usize,
    heap_releases: usize,
}

/// A refused task-route fixture: the exact ordered diagnostic code vector plus
/// the controls that keep the refusal from being vacuous.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceReject {
    diagnostic_sequence: Vec<String>,
    controls: Vec<SourceControl>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceControl {
    source: String,
    /// `accept` or `reject`; any other spelling fails the harness.
    expectation: String,
    diagnostic_sequence: Vec<String>,
    /// For an accepted control, the `SourceExecutionProfile` it must select.
    #[serde(default)]
    execution_profile: Option<String>,
    #[allow(dead_code)]
    note: String,
}

/// Accepted `source.mutex.guard.v1` evidence. The reference observation is
/// computed by `interpret_mutex_program` from the checked event list — it never
/// reads the emitted C — and the sanitized native run must reproduce it line
/// for line. This is the CT corpus's first gated sanitized-native case (T00's
/// finding O4 recorded that the earlier sanitizer evidence was transcript-only).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckedMutexSanitized {
    sections: usize,
    guards: Vec<u64>,
    sanitizer_flags: Vec<String>,
    asan_options: String,
    ubsan_options: String,
    emitted_c_contains: Vec<String>,
    reference_observation: Vec<String>,
}

/// A refused `source.mutex.guard.v1` fixture: the exact ordered diagnostic code
/// vector plus the controls that keep the refusal from being vacuous.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckedMutexReject {
    diagnostic_sequence: Vec<String>,
    controls: Vec<MutexControl>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutexControl {
    source: String,
    /// `accept` or `reject`; any other spelling fails the harness.
    expectation: String,
    diagnostic_sequence: Vec<String>,
    #[allow(dead_code)]
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeTaskScenarios {
    header: String,
    rejection_controls: Vec<TaskRejectionControl>,
    scenarios: Vec<TaskScenario>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskRejectionControl {
    source: String,
    diagnostic: ExpectedDiagnostic,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskScenario {
    name: String,
    failed_allocation: usize,
    #[serde(default)]
    allocation_task: u64,
    #[serde(default)]
    worker_allocation_outcomes: BTreeMap<u64, Vec<bool>>,
    failed_create: usize,
    allocation_outcomes: Vec<bool>,
    spawn_outcomes: Vec<bool>,
    outcome: ExpectedOutcome,
    joined_children: usize,
    owned_child_results: Vec<u64>,
    native_stderr: String,
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
        expected.native_task_scenarios.is_some(),
        expected.verdict == "native_task_scenarios"
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
        expected.checked_mutex_sanitized.is_some(),
        expected.verdict == "checked_mutex_sanitized"
    );
    assert_eq!(
        expected.checked_mutex_sanitized.is_some(),
        case.id == MUTEX_SANITIZED_CASE,
        "{}: exactly one CT case carries gated sanitized mutex evidence",
        case.id
    );
    assert_eq!(
        expected.checked_mutex_reject.is_some(),
        MUTEX_REJECT_CASES.contains(&case.id.as_str()),
        "{}: mutex rejection metadata is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.native_heap_scenarios.is_some(),
        expected.verdict == "native_heap_scenarios"
    );
    assert_eq!(
        expected.native_heap_scenarios.is_some(),
        HEAP_SCENARIO_CASES.contains(&case.id.as_str()),
        "{}: sanitized heap-witness evidence is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.source_reject.is_some(),
        SOURCE_REJECT_CASES.contains(&case.id.as_str()),
        "{}: task-route rejection metadata is pinned to its own case set",
        case.id
    );
    // Target-scoped evidence: the native fault modes, the sanitized mutex run,
    // and any mutex rejection whose control compiles through the Linux-only
    // emission path. Recording the last of these repairs T00's finding O6 for
    // this card's cases rather than repeating it.
    assert_eq!(
        expected.evidence_target.as_deref(),
        (expected.native_join_failure.is_some()
            || expected.native_create_recovery.is_some()
            || expected.native_owned_transfer.is_some()
            || expected.native_task_scenarios.is_some()
            || expected.checked_mutex_sanitized.is_some()
            || expected.native_heap_scenarios.is_some()
            || expected
                .checked_mutex_reject
                .as_ref()
                .is_some_and(|reject| reject
                    .controls
                    .iter()
                    .any(|control| control.expectation == "accept"))
            || expected.source_reject.as_ref().is_some_and(|reject| reject
                .controls
                .iter()
                .any(|control| control.expectation == "accept")))
        .then_some("linux"),
        "{}: a case carries target-scoped evidence exactly when a host-gated \
         runner or control executes for it",
        case.id
    );
    assert_eq!(
        expected.positive_control.is_some(),
        case.id == "CT19",
        "CT19 must retain its explicitly named joined-sibling positive control"
    );
    let mut required_files = BTreeSet::from(["case.qcore".to_owned(), "expected.json".to_owned()]);
    if let Some(native) = &expected.native_task_scenarios {
        assert!(!native.scenarios.is_empty());
        assert!(required_files.insert(native.header.clone()));
        for control in &native.rejection_controls {
            assert!(required_files.insert(control.source.clone()));
        }
        let names = native
            .scenarios
            .iter()
            .map(|scenario| &scenario.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), native.scenarios.len());
        assert!(names.iter().all(|name| !name.is_empty()));
    }
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
    if let Some(mutex) = &expected.checked_mutex_sanitized {
        assert!(mutex.sections >= 2, "CT26 needs two critical sections");
        assert_eq!(mutex.guards.len(), mutex.sections);
        assert!(
            mutex
                .sanitizer_flags
                .iter()
                .any(|flag| flag == "-fsanitize=address,undefined"),
            "the sanitized mutex run must actually request ASan/UBSan"
        );
        assert!(
            mutex.asan_options.contains("detect_leaks=1"),
            "LeakSanitizer stays enabled: reclaiming every emitted allocation \
             is what this run must witness"
        );
        assert!(!mutex.reference_observation.is_empty());
        assert!(!mutex.emitted_c_contains.is_empty());
    }
    if let Some(reject) = &expected.checked_mutex_reject {
        assert!(
            !reject.diagnostic_sequence.is_empty(),
            "{}: a mutex rejection pins its exact ordered code vector",
            case.id
        );
        assert!(
            !reject.controls.is_empty(),
            "{}: a mutex rejection needs at least one control so the refusal \
             is not vacuous",
            case.id
        );
        for control in &reject.controls {
            match control.expectation.as_str() {
                "accept" => assert!(
                    control.diagnostic_sequence.is_empty(),
                    "{}: an accepted control has no diagnostics",
                    case.id
                ),
                "reject" => assert!(
                    !control.diagnostic_sequence.is_empty(),
                    "{}: a refused control pins its codes",
                    case.id
                ),
                other => panic!("{}: unknown control expectation `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(native) = &expected.native_heap_scenarios {
        assert!(!native.scenarios.is_empty());
        assert!(
            native
                .sanitizer_flags
                .iter()
                .any(|flag| flag == "-fsanitize=address,undefined"),
            "{}: the sanitized heap runs must actually request ASan/UBSan",
            case.id
        );
        assert!(
            native.ubsan_options.contains("halt_on_error=1"),
            "{}: UBSan must halt rather than continue past a fault",
            case.id
        );
        assert!(
            !native.alloc_prefix.is_empty() && !native.free_prefix.is_empty(),
            "{}: the heap witness needs both an allocation and a release marker",
            case.id
        );
        assert!(
            native
                .scenarios
                .iter()
                .any(|scenario| scenario.detect_leaks),
            "{}: at least one scenario must run with LeakSanitizer enabled, so \
             `detect_leaks` cannot be switched off case-wide by editing data",
            case.id
        );
        let names = native
            .scenarios
            .iter()
            .map(|scenario| &scenario.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), native.scenarios.len());
        assert!(names.iter().all(|name| !name.is_empty()));
        for scenario in &native.scenarios {
            assert!(
                scenario.heap_releases <= scenario.heap_allocations,
                "{}: {} cannot release more objects than it allocated",
                case.id,
                scenario.name
            );
            assert!(
                scenario
                    .defines
                    .iter()
                    .all(|define| define.starts_with("-D")),
                "{}: {} may only inject failures through `-D` options",
                case.id,
                scenario.name
            );
            assert!(
                scenario.exit_code == 0 || !scenario.stdout_absent.is_empty(),
                "{}: {} is a trapped abort, so it must state what its truncated \
                 observation does not contain",
                case.id,
                scenario.name
            );
            assert!(
                scenario.exit_code == 0 || !scenario.detect_leaks,
                "{}: {} retains its allocations on purpose, so LeakSanitizer \
                 cannot also be asserted clean",
                case.id,
                scenario.name
            );
            required_files.insert(scenario.header.clone());
        }
        for control in &native.rejection_controls {
            assert!(required_files.insert(control.source.clone()));
        }
    }
    if let Some(reject) = &expected.source_reject {
        assert!(
            !reject.diagnostic_sequence.is_empty(),
            "{}: a task-route rejection pins its exact ordered code vector",
            case.id
        );
        assert!(
            !reject.controls.is_empty(),
            "{}: a task-route rejection needs at least one control so the \
             refusal is not vacuous",
            case.id
        );
        for control in &reject.controls {
            match control.expectation.as_str() {
                "accept" => {
                    assert!(
                        control.diagnostic_sequence.is_empty(),
                        "{}: an accepted control has no diagnostics",
                        case.id
                    );
                    assert!(
                        control.execution_profile.is_some(),
                        "{}: an accepted control must name the route it takes, \
                         so a control cannot silently pass through another one",
                        case.id
                    );
                }
                "reject" => {
                    assert!(
                        !control.diagnostic_sequence.is_empty(),
                        "{}: a refused control pins its codes",
                        case.id
                    );
                    assert!(control.execution_profile.is_none());
                }
                other => panic!("{}: unknown control expectation `{other}`", case.id),
            }
            assert!(required_files.insert(control.source.clone()));
        }
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
        // The task route reads `cfg` as the checked task IR it really builds
        // (the manifest note states that reading), which is why every
        // task-route native mode claims it and the mutex route never does.
        "native_join_failure"
        | "native_create_recovery"
        | "native_owned_transfer"
        | "native_task_scenarios"
        | "native_heap_scenarios" => [
            "parser",
            "type_checker",
            "reference_execution",
            "cfg",
            "native_backend",
        ]
        .into_iter()
        .collect(),
        // `cfg` is absent on purpose: `source.mutex.guard.v1` emits directly
        // from the checked event list and has no CFG stage at all, so claiming
        // it would be false (T00's rule: a stage is executed only where a named
        // test runs it on the case's own artifact).
        "checked_mutex_sanitized" => [
            "parser",
            "type_checker",
            "reference_execution",
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
    if let Some(reject) = &expected.checked_mutex_reject {
        run_mutex_controls(&directory, &case.id, reject);
    }
    if let Some(reject) = &expected.source_reject {
        run_source_controls(&directory, &case.id, reject);
    }
    match expected.verdict.as_str() {
        "native_heap_scenarios" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_native_heap_scenarios(&directory, &expected, result);
        }
        "checked_mutex_sanitized" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_checked_mutex_sanitized(&expected, result);
        }
        "native_task_scenarios" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none()
            );
            run_native_task_scenarios(&directory, &expected, result);
        }
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
            if let Some(reject) = &expected.checked_mutex_reject {
                // The whole cascade, in order, not merely its head: a refused
                // release leaves the guard live, which blocks scope-exit
                // destruction and surfaces in the end-of-trace sweep. Pinning
                // every entry is what shows the refusal did not silently
                // release the guard.
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    reject.diagnostic_sequence,
                    "{}: exact diagnostic cascade mismatch; actual:\n{rendered}",
                    case.id
                );
            }
            if let Some(reject) = &expected.source_reject {
                // Same discipline on the task route: the whole vector, in
                // order. A refusal that also silently dropped a custody
                // obligation would still satisfy a head-only assertion.
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    reject.diagnostic_sequence,
                    "{}: exact diagnostic cascade mismatch; actual:\n{rendered}",
                    case.id
                );
            }
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

/// The moving count is the W6 genericity metric: 20 of 50 CT cases are mapped
/// as executed source-level tests; 30 remain unmapped. CT13/CT15/CT26 and the
/// CT03/CT17 sanitized heap scenarios are Linux-scoped, and so are the accepted
/// controls CT04/CT05/CT18/CT27/CT29/CT30 carry. Mapping is not conformance.
/// This test exists so the counts are visible and any drift is a deliberate
/// edit, not an accident.
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

/// The controls a mutex rejection carries. An `accept` control compiles through
/// the Linux-only emission path, so on any other host it must arrive as the
/// same `Q500` refusal every other route uses — never as a silent skip.
fn run_mutex_controls(directory: &Path, id: &str, reject: &CheckedMutexReject) {
    for control in &reject.controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let result = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            "accept" => {
                if !cfg!(target_os = "linux") {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{id} control {}: source/core checked; Linux emission unexecuted on this host",
                        control.source
                    );
                    continue;
                }
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{id} control {} must check, so the case's refusal is not vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                assert!(
                    module.checked.as_checked_mutex_guards().is_some(),
                    "{id} control {} must take the checked mutex route",
                    control.source
                );
            }
            "reject" => {
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!("{id} control {} must be refused", control.source),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{id} control {}: exact diagnostic vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            other => panic!("{id}: unknown control expectation `{other}`"),
        }
    }
}

/// The declared `SourceExecutionProfile` label a fixture must select. An
/// unknown label fails rather than being ignored, so a route cannot be renamed
/// out from under a case.
fn execution_profile(label: &str) -> qcore_compiler::SourceExecutionProfile {
    use qcore_compiler::SourceExecutionProfile as Profile;
    match label {
        "SequentialCleanupV1" => Profile::SequentialCleanupV1,
        "ScopedCopyTasksV1" => Profile::ScopedCopyTasksV1,
        "ScopedOwnedTasksV1" => Profile::ScopedOwnedTasksV1,
        "ScopedReadLoansV1" => Profile::ScopedReadLoansV1,
        "ScopedReadSharesV1" => Profile::ScopedReadSharesV1,
        "ScopedErrorPayloadV1" => Profile::ScopedErrorPayloadV1,
        "CheckedMutexGuardsV1" => Profile::CheckedMutexGuardsV1,
        other => panic!("unknown execution profile `{other}`"),
    }
}

/// The controls a task-route rejection carries. An `accept` control compiles
/// through the Linux-only emission path, so on any other host it must arrive as
/// the same `Q500` refusal every other route uses — never as a silent skip.
fn run_source_controls(directory: &Path, id: &str, reject: &SourceReject) {
    for control in &reject.controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let result = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            "accept" => {
                if !cfg!(target_os = "linux") {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{id} control {}: source/core checked; Linux emission unexecuted on this host",
                        control.source
                    );
                    continue;
                }
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{id} control {} must check, so the case's refusal is not vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                assert_eq!(
                    module.execution_profile,
                    execution_profile(control.execution_profile.as_deref().unwrap()),
                    "{id} control {} must take the route it names",
                    control.source
                );
            }
            "reject" => {
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!("{id} control {} must be refused", control.source),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{id} control {}: exact diagnostic vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            other => panic!("{id}: unknown control expectation `{other}`"),
        }
    }
}

/// CT03 and CT17: the whole task-route stage stack on the case's own fixture —
/// parse, elaborate, lower, check the task IR, reference-execute it, emit C11,
/// then compile that C under ASan/UBSan/LSan and run it once per fixed
/// schedule. The reference observation is computed by
/// `execute_task_module_with_worker_allocations` from the checked task IR and
/// never reads the emitted C. The included header records every Buffer
/// allocation and release by address, so "one object, counted once" and
/// "reclaims nothing" are identity claims rather than trace claims.
fn run_native_heap_scenarios(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module_with_worker_allocations, render_task_observation,
    };
    let native = expected.native_heap_scenarios.as_ref().unwrap();
    for control in &native.rejection_controls {
        let path = directory.join(&control.source);
        let errors = compile_files(
            &[SourceFile::new(
                path.to_string_lossy(),
                fs::read_to_string(&path).unwrap(),
            )],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == control.diagnostic.code
                    && error.message.contains(&control.diagnostic.message_contains)),
            "{}: control {} must be refused with its pinned diagnostic; actual:\n{errors:?}",
            expected.id,
            control.source
        );
    }
    if !cfg!(target_os = "linux") {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: source/core checked; Linux reference/sanitized-native execution unexecuted on \
             this host (manifest evidence_target=linux)",
            expected.id
        );
        return;
    }
    let module = result.expect("the accepted task fixture must pass all-path checks");
    assert_eq!(
        module.execution_profile,
        execution_profile(&native.execution_profile),
        "{}: the case must select the route it names, not another one",
        expected.id
    );
    assert!(
        module.reference_observation.is_none(),
        "{}: do not substitute the sequential execution profile",
        expected.id
    );
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("the checked task artifact is explicit for this route");
    for scenario in &native.scenarios {
        let reference = render_task_observation(
            &execute_task_module_with_worker_allocations(
                checked,
                &TaskExecutionScript {
                    spawn_outcomes: scenario.spawn_outcomes.clone(),
                    allocation_outcomes: scenario.allocation_outcomes.clone(),
                },
                &scenario.worker_allocation_outcomes,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{}: {} reference execution: {error:?}",
                    expected.id, scenario.name
                )
            }),
        );
        let output = sanitized_heap_run(
            module.c_source.as_deref().unwrap(),
            directory,
            native,
            scenario,
        );
        assert_eq!(
            output.status.code(),
            Some(scenario.exit_code),
            "{}: {} exited unexpectedly:\n{}",
            expected.id,
            scenario.name,
            String::from_utf8_lossy(&output.stderr)
        );
        let observed = String::from_utf8(output.stdout.clone()).unwrap();
        if scenario.exit_code == 0 {
            assert_eq!(
                observed, reference,
                "{}: {} native and reference observations must agree line for line",
                expected.id, scenario.name
            );
        }
        for wanted in &scenario.stdout_contains {
            assert!(
                observed.contains(wanted.as_str()),
                "{}: {} is missing `{wanted}` in:\n{observed}",
                expected.id,
                scenario.name
            );
        }
        for forbidden in &scenario.stdout_absent {
            assert!(
                !observed.contains(forbidden.as_str()),
                "{}: {} must not contain `{forbidden}` in:\n{observed}",
                expected.id,
                scenario.name
            );
        }
        let mut previous = None;
        for line in &scenario.stdout_order {
            let position = observed
                .lines()
                .position(|entry| entry == line)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: {} is missing the ordered line `{line}` in:\n{observed}",
                        expected.id, scenario.name
                    )
                });
            if let Some(previous) = previous {
                assert!(
                    previous < position,
                    "{}: {} observed `{line}` out of its pinned order in:\n{observed}",
                    expected.id,
                    scenario.name
                );
            }
            previous = Some(position);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let addresses = |prefix: &str| {
            stderr
                .lines()
                .filter_map(|line| line.strip_prefix(prefix).map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        };
        let allocated = addresses(&native.alloc_prefix);
        let released = addresses(&native.free_prefix);
        assert_eq!(
            allocated.len(),
            scenario.heap_allocations,
            "{}: {} allocated {allocated:?}",
            expected.id,
            scenario.name
        );
        assert_eq!(
            released.len(),
            scenario.heap_releases,
            "{}: {} released {released:?}",
            expected.id,
            scenario.name
        );
        for address in &released {
            assert!(
                allocated.contains(address),
                "{}: {} released an address it never allocated: {address}",
                expected.id,
                scenario.name
            );
            assert_eq!(
                released.iter().filter(|entry| *entry == address).count(),
                1,
                "{}: {} released {address} more than once",
                expected.id,
                scenario.name
            );
        }
    }
}

/// Compiles the emitted C with the recorded sanitizer flags, beside every
/// fixture header so a header may `#include` another, and runs it once.
/// `ASAN_OPTIONS` is computed here from the scenario's `detect_leaks` rather
/// than read from data, so LeakSanitizer cannot be quietly disabled by editing
/// an option string.
fn sanitized_heap_run(
    source: &str,
    fixtures: &Path,
    native: &NativeHeapScenarios,
    scenario: &HeapScenario,
) -> std::process::Output {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "qcore-ct-heap-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let c_path = directory.join("module.c");
    let exe_path = directory.join("module");
    fs::write(&c_path, source).unwrap();
    for entry in fs::read_dir(fixtures).unwrap() {
        let entry = entry.unwrap().path();
        if entry.extension().is_some_and(|extension| extension == "h") {
            fs::copy(&entry, directory.join(entry.file_name().unwrap())).unwrap();
        }
    }
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".to_owned());
    let compiled = Command::new(compiler)
        .args(&native.sanitizer_flags)
        .args(&scenario.defines)
        .arg("-include")
        .arg(directory.join(&scenario.header))
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "the C compiler rejected the emitted program for {}:\n{}",
        scenario.name,
        String::from_utf8_lossy(&compiled.stderr)
    );
    let output = Command::new(&exe_path)
        .env(
            "ASAN_OPTIONS",
            if scenario.detect_leaks {
                "detect_leaks=1:halt_on_error=1"
            } else {
                "detect_leaks=0:halt_on_error=1"
            },
        )
        .env("UBSAN_OPTIONS", &native.ubsan_options)
        .output()
        .unwrap();
    fs::remove_dir_all(directory).unwrap();
    output
}

/// CT26: the whole `source.mutex.guard.v1` stage stack on the case's own
/// fixture — parse, elaborate, lower, check, independently recheck, emit, then
/// compile the emitted C under ASan/UBSan/LSan and run it. The native stdout is
/// compared line for line with `interpret_mutex_program`, which is written
/// against the checked event list and never reads the emitted C.
fn run_checked_mutex_sanitized(
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_compiler::{ConcurrencyAssumption, ConcurrencyProfile, SourceExecutionProfile};
    let mutex = expected.checked_mutex_sanitized.as_ref().unwrap();
    if !cfg!(target_os = "linux") {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: source/core checked; Linux mutex reference/native execution unexecuted on this host \
             (manifest evidence_target=linux)",
            expected.id
        );
        return;
    }
    let module = result.expect("the accepted mutex fixture must check on this route");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::CheckedMutexGuardsV1,
        "{}: the case must select the checked mutex route, not another one",
        expected.id
    );
    assert!(
        module.reference_observation.is_none(),
        "{}: do not substitute the sequential execution profile",
        expected.id
    );
    assert!(module.checked.as_scoped_tasks().is_none());
    assert!(module.checked.as_sequential().is_none());
    let checked = module
        .checked
        .as_checked_mutex_guards()
        .expect("the checked artifact variant is explicit for this route");
    assert_eq!(
        checked.certificate.profile,
        ConcurrencyProfile::CheckedMutexInvariantV1,
        "{}: a trusted-trace certificate can never satisfy this case",
        expected.id
    );
    for forbidden in [
        ConcurrencyAssumption::GuardFootprintAuthority,
        ConcurrencyAssumption::SynchronizedCustodyEdges,
    ] {
        assert!(
            !checked.certificate.assumptions.contains(&forbidden),
            "{}: a checked-profile certificate never carries a trusted-model \
             guard premise",
            expected.id
        );
    }
    let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
        &checked.lowered.program,
        &checked.certificate,
    )
    .expect("the lowered trace is inside the emitted subset");
    assert_eq!(
        reference, mutex.reference_observation,
        "{}: reference observation drifted from the pinned expectation",
        expected.id
    );
    for guard in &mutex.guards {
        assert!(
            reference
                .iter()
                .any(|line| line == &format!("lock 1 {guard}")),
            "{}: guard {guard} is not minted by its own critical section",
            expected.id
        );
    }
    assert_eq!(
        reference
            .iter()
            .filter(|line| line.starts_with("unlock "))
            .count(),
        mutex.sections,
        "{}: every critical section must release",
        expected.id
    );
    let c = module.c_source.as_deref().expect("Linux emission");
    for wanted in &mutex.emitted_c_contains {
        assert!(
            c.contains(wanted.as_str()),
            "{}: emitted C is missing `{wanted}`",
            expected.id
        );
    }
    assert_eq!(
        mutex_sanitized_run(c, mutex),
        reference,
        "{}: native and reference observations must agree line for line",
        expected.id
    );
}

fn mutex_sanitized_run(source: &str, mutex: &CheckedMutexSanitized) -> Vec<String> {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "qcore-ct-mutex-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("module.c");
    let binary = directory.join("module");
    fs::write(&path, source).unwrap();
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
    let compiled = Command::new(&compiler)
        .args(&mutex.sanitizer_flags)
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "C compiler rejected the emitted program:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&binary)
        .env("ASAN_OPTIONS", &mutex.asan_options)
        .env("UBSAN_OPTIONS", &mutex.ubsan_options)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "emitted program failed under ASan/UBSan/LSan:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let observation = String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    fs::remove_dir_all(directory).unwrap();
    observation
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
    native_fault_run_defines(source, header, &[define.to_owned()])
}

fn native_fault_run_defines(
    source: &str,
    header: &Path,
    defines: &[String],
) -> std::process::Output {
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
        .args(defines)
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

fn run_native_task_scenarios(
    directory: &Path,
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_core::scoped_tasks::TaskOutcome;
    use qcore_core::task_execution::{
        TaskExecutionScript, execute_task_module_with_worker_allocations, render_task_observation,
    };
    let native = expected.native_task_scenarios.as_ref().unwrap();
    for control in &native.rejection_controls {
        let path = directory.join(&control.source);
        let errors = compile_files(
            &[SourceFile::new(
                path.to_string_lossy(),
                fs::read_to_string(&path).unwrap(),
            )],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == control.diagnostic.code
                    && error.message.contains(&control.diagnostic.message_contains)),
            "{errors:?}"
        );
    }
    if !cfg!(target_os = "linux") {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: Linux task scenarios unexecuted on this host",
            expected.id
        );
        return;
    }
    let module = result.expect("task scenario source must pass all-path checks");
    let checked = module.checked.as_scoped_tasks().unwrap();
    for scenario in &native.scenarios {
        let selected_allocations = if scenario.allocation_task == 0 {
            &scenario.allocation_outcomes
        } else {
            scenario
                .worker_allocation_outcomes
                .get(&scenario.allocation_task)
                .expect("selected allocation task needs explicit decisions")
        };
        for (failed, outcomes) in [
            (scenario.failed_allocation, selected_allocations),
            (scenario.failed_create, &scenario.spawn_outcomes),
        ] {
            assert!(
                outcomes
                    .iter()
                    .enumerate()
                    .all(|(index, succeeds)| *succeeds == (index + 1 != failed)),
                "{}: inconsistent failure script",
                scenario.name
            );
            assert!(
                failed == 0 || failed <= outcomes.len(),
                "{}: failure must execute",
                scenario.name
            );
        }
        let reference = execute_task_module_with_worker_allocations(
            checked,
            &TaskExecutionScript {
                spawn_outcomes: scenario.spawn_outcomes.clone(),
                allocation_outcomes: scenario.allocation_outcomes.clone(),
            },
            &scenario.worker_allocation_outcomes,
        )
        .unwrap();
        let outcome = match (&scenario.outcome.error, scenario.outcome.ok_value) {
            (Some(error), None) => {
                assert!(scenario.outcome.payload.is_none());
                TaskOutcome::Error(error.clone())
            }
            (None, Some(value)) => {
                assert_eq!(scenario.outcome.payload.as_deref(), Some([].as_slice()));
                TaskOutcome::I32(value)
            }
            _ => panic!("scenario requires exactly one result"),
        };
        assert_eq!(reference.root_outcome, outcome, "{}", scenario.name);
        assert_eq!(reference.tasks.len(), scenario.joined_children + 1);
        assert!(reference.tasks.iter().skip(1).all(|task| task.joined));
        let owned = reference
            .tasks
            .iter()
            .skip(1)
            .filter_map(|task| match task.outcome {
                TaskOutcome::OwnedBuffer(id) => Some(id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(owned, scenario.owned_child_results);
        let output = native_fault_run_defines(
            module.c_source.as_deref().unwrap(),
            &directory.join(&native.header),
            &[
                format!("-DQCORE_CT_ALLOC_TASK={}", scenario.allocation_task),
                format!("-DQCORE_CT_FAIL_ALLOC={}", scenario.failed_allocation),
                format!("-DQCORE_CT_FAIL_CREATE={}", scenario.failed_create),
            ],
        );
        assert!(output.status.success(), "{}: {output:?}", scenario.name);
        assert_eq!(
            output.stdout,
            render_task_observation(&reference).as_bytes(),
            "{}",
            scenario.name
        );
        assert_eq!(
            output.stderr,
            scenario.native_stderr.as_bytes(),
            "{}",
            scenario.name
        );
    }
}

#[test]
fn ct16_retains_sequential_implicit_cleanup_control() {
    let module = compile_files(
        &[SourceFile::new(
            "sequential-worker-error.qcore",
            include_str!("fixtures/task_worker_allocations/sequential_worker_error.qcore"),
        )],
        &CompileOptions {
            entry: Some("worker".into()),
        },
    )
    .unwrap();
    let observation = module.reference_observation.unwrap();
    assert!(
        matches!(&observation.outcome, ExecutionOutcome::Error {name, ..} if name == "WorkerError")
    );
    assert_eq!(
        observation
            .trace
            .iter()
            .map(trace_token)
            .collect::<Vec<_>>(),
        ["acquire:3", "release:3"]
    );
}

#[test]
fn ct37_retains_sequential_transferred_payload_control() {
    let module = compile_files(
        &[SourceFile::new(
            "sequential-completion-payload.qcore",
            include_str!("fixtures/task_worker_allocations/sequential_completion_payload.qcore"),
        )],
        &CompileOptions {
            entry: Some("worker".into()),
        },
    )
    .unwrap();
    let observation = module.reference_observation.unwrap();
    assert!(
        matches!(&observation.outcome, ExecutionOutcome::Ok { value, payload } if *value == 5 && payload == &[6])
    );
    assert_eq!(
        observation
            .trace
            .iter()
            .map(trace_token)
            .collect::<Vec<_>>(),
        ["acquire:5", "acquire:6", "transfer:6", "release:5"]
    );
}
