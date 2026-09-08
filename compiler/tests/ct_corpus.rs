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
//! Three source routes are represented. The task route (`ScopedCopyTasksV1`,
//! `ScopedOwnedTasksV1`, and its `ScopedReadSharesV1` / `ScopedErrorPayloadV1`
//! extensions) is the original eleven cases plus CT03/CT04/CT05/CT17/CT18. The
//! mutex route (`source.mutex.guard.v1` lowering onto the unedited
//! `CheckedMutexInvariantV1` checker, cards T03 and T03-F) is
//! CT26/CT27/CT28/CT29/CT30/CT31/CT32/CT48. The epoch route
//! (`source.epoch.bound.v1` lowering onto the unedited `FlatEpochBoundV1`
//! checker, cards T08-EPOCH-SRC and T08-SRC-CT) is CT35/CT36/CT38/CT39.
//! Rejection cases on any route pin the whole ordered diagnostic cascade rather
//! than its head, because a refusal that silently dropped a custody obligation
//! would still satisfy a head-only assertion, and each carries at least one
//! control so the refusal is not merely "the profile cannot express this at
//! all". The epoch route's *accepted* cases carry the symmetric obligation: an
//! epoch that checks has said nothing until its published certificate is
//! measured, so `epoch_bound_figures` pins every figure on it.
//!
//! Manifest schema 2 (card T00-CT-SCHEMA, tasks T00.1-T00.9) makes the per-case
//! stage record derived rather than declared. Schema 1 wrote a bare word per
//! stage — and two of its six names, `cfg` and `type_checker`, conflated four
//! real passes and had to be *read differently per route*, so a task-route case
//! and a mutex-route case rendered identically while running different
//! machinery. Schema 2 names the passes that exist, gives each route its own
//! stage vocabulary, and requires an `executed` entry to name the runner, the
//! artifact, the invocation count and the exact `target_os` gate behind it.
//! `derive_stage_plan` recomputes every one of those from this harness's own
//! dispatch, the fixture's own scenario data and the diagnostic codes the
//! compiler really produced, and `run_mapped_case` refuses any disagreement, so
//! the manifest is checked evidence and not a self-certifying claim. A route
//! that has no such pass records it as *structurally absent*
//! (`not_applicable`): the mutex route emits directly from its checked event
//! list, so it has no CFG-shaped stage at all, which schema 1 could only spell
//! as an indefinite `cfg: unassessed`. Sanitizer evidence is gated: only a case
//! a runner really compiles with `-fsanitize=…` may read `native_sanitized:
//! executed`, and the five cases whose sanitizer evidence is an archived
//! transcript must read `recorded_not_gated` with a pointer to it.

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

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
    "CT19", "CT20", "CT26", "CT27", "CT28", "CT29", "CT30", "CT31", "CT32", "CT35", "CT36", "CT37",
    "CT38", "CT39", "CT48", "CT50",
];
/// The `source.mutex.guard.v1` rejection cases (T03's lowering onto the unedited
/// `CheckedMutexInvariantV1` checker, extended by T03-F's bounded task
/// composition, `wait` announcement and guard-scoped facts). Each pins its exact
/// ordered diagnostic code vector, not merely its head, and carries at least one
/// control that distinguishes the refused construct from the admitted one.
const MUTEX_REJECT_CASES: &[&str] = &["CT27", "CT28", "CT29", "CT30", "CT31", "CT32", "CT48"];
/// The one accepted mutex-guard case, executed end to end and re-run under
/// ASan/UBSan/LSan.
const MUTEX_SANITIZED_CASE: &str = "CT26";
/// Task-route rejection cases (cards T05 and T06). Same discipline as the mutex
/// rejections: the exact ordered code vector plus at least one control that
/// isolates the refused element from the surrounding program.
const SOURCE_REJECT_CASES: &[&str] = &["CT04", "CT05", "CT18"];
/// The `source.epoch.bound.v1` rejection cases (card T08-EPOCH-SRC's lowering
/// onto the unedited `FlatEpochBoundV1` checker, mapped by card T08-SRC-CT).
/// Same discipline again: the exact ordered code vector plus at least one
/// control that distinguishes the refused epoch from an admitted one.
const EPOCH_REJECT_CASES: &[&str] = &["CT38", "CT39"];
/// The accepted `source.epoch.bound.v1` cases. Their evidence is not the bare
/// verdict — an epoch that checks says nothing on its own — but the exact
/// certificate figures the checker published, which is why they carry their own
/// verdict mode rather than reusing `accept`.
const EPOCH_FIGURE_CASES: &[&str] = &["CT35", "CT36"];
/// Task-route accept cases whose native evidence is a sanitized run with a heap
/// witness: every Buffer allocation and release is recorded by address, so
/// "counted once" and "reclaims nothing" are checked by identity rather than by
/// the emitted trace alone.
const HEAP_SCENARIO_CASES: &[&str] = &["CT03", "CT17"];
const TOTAL_CASES: usize = 50;

/// Manifest schema 2 (card T00-CT-SCHEMA, tasks T00.1-T00.9). Schema 1's `cfg`
/// and `type_checker` are gone: they conflated four real passes and their
/// reading differed per route, so a mutex-route case and a task-route case could
/// render identically while executing different machinery. These are the passes
/// that exist, in pipeline order. Every mapped case declares its `route`, and a
/// stage outside that route's vocabulary is *structurally absent*
/// (`not_applicable`), never "unassessed forever".
const STAGE_NAMES: &[&str] = &[
    "parse",
    "elaborate",
    "task_lowering",
    "task_ledger_check",
    "mutex_lowering",
    "mutex_invariant_check",
    "epoch_lowering",
    "epoch_bound_check",
    "sequential_whole_program",
    "sequential_reference_execution",
    "task_reference_execution",
    "mutex_reference_execution",
    "c_emission",
    "native_execution",
    "native_sanitized",
    "legacy_c_twin",
    "proof",
];
/// `executed` needs an executor, an artifact and an observation count;
/// `recorded_not_gated` needs a `record` pointer and is never counted as
/// executed (T00 finding O4: the ASan transcripts are re-run by nothing);
/// `not_applicable` is a stage this route or this refusal provably never
/// reaches; `unassessed` is an open stage.
const STAGE_STATUSES: &[&str] = &[
    "executed",
    "recorded_not_gated",
    "not_applicable",
    "unassessed",
];
/// Per-route stage vocabulary. A route's list is exactly the stages that route
/// *can* execute; the harness refuses any other stage claiming anything but
/// `not_applicable`. `sequential` and `legacy_c` are declared and currently
/// carry no mapped case — declaring them is what makes "this route has no such
/// stage" a structural statement rather than a note.
const ROUTE_STAGES: &[(&str, &[&str])] = &[
    // `source.epoch.bound.v1` is check-only by construction: an admitted epoch
    // is an accounting statement about declared shapes, so the driver publishes
    // it with `c_source: None` and `reference_observation: None`. There is
    // nothing to execute, nothing to emit and therefore nothing to sanitize, so
    // this route's vocabulary omits the reference/emission/native stages —
    // including `native_sanitized`, which is structurally absent here rather
    // than an open item. That is the one deviation from card T08-EPOCH-SRC's
    // parked rows, which proposed `native_sanitized: unassessed`; recorded in
    // `docs/reports/wave8-epoch-ct/README.md`.
    (
        "epoch",
        &[
            "parse",
            "elaborate",
            "epoch_lowering",
            "epoch_bound_check",
            "proof",
        ],
    ),
    ("legacy_c", &["parse", "legacy_c_twin", "proof"]),
    (
        "mutex_guard",
        &[
            "parse",
            "elaborate",
            "mutex_lowering",
            "mutex_invariant_check",
            "mutex_reference_execution",
            "c_emission",
            "native_execution",
            "native_sanitized",
            "proof",
        ],
    ),
    (
        "sequential",
        &[
            "parse",
            "elaborate",
            "sequential_whole_program",
            "sequential_reference_execution",
            "c_emission",
            "native_execution",
            "native_sanitized",
            "proof",
        ],
    ),
    (
        "task",
        &[
            "parse",
            "elaborate",
            "task_lowering",
            "task_ledger_check",
            "task_reference_execution",
            "c_emission",
            "native_execution",
            "native_sanitized",
            "proof",
        ],
    ),
];
/// The only executors a manifest stage entry may name. A stage cannot be
/// credited to a runner this file does not have.
const RUNNER_REGISTRY: &[&str] = &[
    "compiler/tests/ct_corpus.rs::run_mapped_case",
    "compiler/tests/ct_corpus.rs::run_native_owned_transfer",
    "compiler/tests/ct_corpus.rs::run_native_create_recovery",
    "compiler/tests/ct_corpus.rs::run_native_join_failure",
    "compiler/tests/ct_corpus.rs::run_native_task_scenarios",
    "compiler/tests/ct_corpus.rs::run_native_heap_scenarios",
    "compiler/tests/ct_corpus.rs::run_checked_mutex_sanitized",
];
/// The sanitizer gate (T00.3). A case may claim `native_sanitized: executed`
/// only if a harness runner really compiles its emitted C with `-fsanitize=…`
/// and runs it. Every other case's sanitizer evidence is a transcript.
const SANITIZED_GATED_CASES: &[&str] = &["CT03", "CT17", "CT26"];
/// Cases whose sanitizer evidence is an archived transcript that no workspace
/// test re-runs. They must read `recorded_not_gated` with a `record` pointer,
/// never `executed`.
const SANITIZER_TRANSCRIPT_CASES: &[&str] = &["CT14", "CT16", "CT20", "CT37", "CT50"];
/// Depth rank per stage: the deepest executed stage of a case is the one with
/// the greatest rank. Ranks are shared across routes so a task-route ledger
/// check and a mutex-route invariant check sit at the same depth.
const STAGE_DEPTH: &[(&str, u8)] = &[
    ("parse", 0),
    ("elaborate", 1),
    ("task_lowering", 2),
    ("mutex_lowering", 2),
    ("epoch_lowering", 2),
    ("sequential_whole_program", 2),
    ("task_ledger_check", 3),
    ("mutex_invariant_check", 3),
    ("epoch_bound_check", 3),
    ("task_reference_execution", 4),
    ("mutex_reference_execution", 4),
    ("sequential_reference_execution", 4),
    ("legacy_c_twin", 4),
    ("c_emission", 5),
    ("native_execution", 6),
    ("native_sanitized", 7),
    ("proof", 8),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    #[allow(dead_code)]
    note: String,
    stage_vocabulary: StageVocabulary,
    cases: Vec<ManifestCase>,
}

/// The vocabulary is carried in the manifest so a reader knows what the words
/// mean, and pinned against this file so data cannot widen it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StageVocabulary {
    stages: Vec<String>,
    statuses: Vec<String>,
    routes: BTreeMap<String, Vec<String>>,
}

/// One per-case stage entry. A status is only as good as the evidence named
/// beside it, which is why `executed` cannot be written without an executor, an
/// artifact and a count, and why a host-gated runner must state its gate rather
/// than leaving the status host-unconditional (T00 finding O3).
#[derive(Deserialize, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
struct StageRecord {
    status: String,
    #[serde(default)]
    executor: Option<String>,
    #[serde(default)]
    artifact: Option<String>,
    #[serde(default)]
    observations: Option<usize>,
    #[serde(default)]
    host_gate: Option<Vec<String>>,
    #[serde(default)]
    record: Option<String>,
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
    route: Option<String>,
    source_fixture: Option<String>,
    semantic_profile: Option<String>,
    #[serde(default)]
    harness_verdict_mode: Option<String>,
    stages: BTreeMap<String, StageRecord>,
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
    #[serde(default)]
    epoch_bound_figures: Option<EpochBoundFigures>,
    #[serde(default)]
    epoch_reject: Option<EpochReject>,
}

/// Accepted `source.epoch.bound.v1` evidence (cards T08, T08-EPOCH-SRC,
/// T08-SRC-CT). An admitted epoch is not itself the observation: the route
/// refuses nothing about a program that declares comfortable figures, so
/// "checked" alone would map a case without measuring what CT35 and CT36 ask.
/// What is measured is the certificate the checker published — every nonzero
/// axis of every quantity on it, keyed `field.axis`, plus the task count and
/// the exact premise set — so a changed union rule, a maximized peak or a
/// dropped premise moves a pinned number instead of passing silently.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochBoundFigures {
    /// The `SourceExecutionProfile` the module must be published under, so a
    /// case that silently changed route would not still pass.
    execution_profile: String,
    /// Exactly the certificate's premise set. These are declared premises, not
    /// facts this route establishes, and naming them here keeps that visible.
    assumptions: Vec<String>,
    task_count: u64,
    /// Exactly the nonzero `field.axis` entries of the published certificate —
    /// not a chosen subset, which the runner enforces by rebuilding the whole
    /// map from the certificate and comparing it wholesale.
    figures: BTreeMap<String, u64>,
    controls: Vec<EpochControl>,
}

/// A refused `source.epoch.bound.v1` fixture: the exact ordered diagnostic code
/// vector plus the controls that keep the refusal from being vacuous.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochReject {
    diagnostic_sequence: Vec<String>,
    controls: Vec<EpochControl>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochControl {
    source: String,
    /// `accept` or `reject`; any other spelling fails the harness.
    expectation: String,
    diagnostic_sequence: Vec<String>,
    /// An accepted control states its own certificate, on the same terms as the
    /// case: a control that merely "also checks" would not show what differs.
    #[serde(default)]
    task_count: Option<u64>,
    #[serde(default)]
    figures: BTreeMap<String, u64>,
    #[allow(dead_code)]
    note: String,
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

/// What this harness derives for one stage of one case: the status, the runner
/// that produced it, how many times that runner invoked it, and the exact set of
/// `target_os` values on which it runs. `host_gate: None` means the stage runs
/// on every host.
#[derive(Clone)]
struct StagePlan {
    status: &'static str,
    executor: Option<&'static str>,
    observations: Option<usize>,
    host_gate: Option<Vec<&'static str>>,
}

impl StagePlan {
    fn absent(status: &'static str) -> Self {
        StagePlan {
            status,
            executor: None,
            observations: None,
            host_gate: None,
        }
    }

    fn executed(
        executor: &'static str,
        observations: usize,
        host_gate: Option<Vec<&'static str>>,
    ) -> Self {
        StagePlan {
            status: "executed",
            executor: Some(executor),
            observations: Some(observations),
            host_gate,
        }
    }
}

fn route_stages(route: &str) -> &'static [&'static str] {
    ROUTE_STAGES
        .iter()
        .find(|(name, _)| *name == route)
        .unwrap_or_else(|| panic!("unknown CT route `{route}`"))
        .1
}

/// The route a verdict mode really takes. `reject` is the one mode all three
/// source routes share, and the pinned `MUTEX_REJECT_CASES` / `EPOCH_REJECT_CASES`
/// sets — each cross-checked against its fixture's own rejection metadata —
/// decide it.
fn route_for_case(case_id: &str, verdict: &str) -> &'static str {
    match verdict {
        "checked_mutex_sanitized" => "mutex_guard",
        "epoch_bound_figures" => "epoch",
        "native_join_failure"
        | "native_create_recovery"
        | "native_owned_transfer"
        | "native_task_scenarios"
        | "native_heap_scenarios" => "task",
        "accept" => "sequential",
        "reject" => {
            if MUTEX_REJECT_CASES.contains(&case_id) {
                "mutex_guard"
            } else if EPOCH_REJECT_CASES.contains(&case_id) {
                "epoch"
            } else {
                "task"
            }
        }
        other => panic!("{case_id}: unknown verdict vocabulary `{other}`"),
    }
}

fn diagnostic_band(code: &str) -> u32 {
    code.trim_start_matches('Q')
        .parse::<u32>()
        .unwrap_or_else(|_| panic!("unparsable diagnostic code `{code}`"))
}

/// Derive every stage entry for one case from this harness's dispatch, the
/// fixture's own scenario data, and the diagnostic codes the compiler really
/// produced on the fixture. Nothing here reads the manifest, which is what
/// makes the manifest checkable rather than self-certifying.
fn derive_stage_plan(
    case_id: &str,
    expected: &ExpectedCase,
    observed_codes: &[&str],
) -> BTreeMap<&'static str, StagePlan> {
    let verdict = expected.verdict.as_str();
    let route = route_for_case(case_id, verdict);
    let base = "compiler/tests/ct_corpus.rs::run_mapped_case";
    let deep = match verdict {
        "native_owned_transfer" => Some("compiler/tests/ct_corpus.rs::run_native_owned_transfer"),
        "native_create_recovery" => Some("compiler/tests/ct_corpus.rs::run_native_create_recovery"),
        "native_join_failure" => Some("compiler/tests/ct_corpus.rs::run_native_join_failure"),
        "native_task_scenarios" => Some("compiler/tests/ct_corpus.rs::run_native_task_scenarios"),
        "native_heap_scenarios" => Some("compiler/tests/ct_corpus.rs::run_native_heap_scenarios"),
        "checked_mutex_sanitized" => {
            Some("compiler/tests/ct_corpus.rs::run_checked_mutex_sanitized")
        }
        _ => None,
    };
    // Invocation counts, read from the fixture data each runner loops over — an
    // observation count is a fact about the runner, not a number to be typed in.
    let (reference_runs, native_runs) = match verdict {
        "native_owned_transfer" => {
            let runs = if expected
                .native_owned_transfer
                .as_ref()
                .expect("mode metadata")
                .fail_create
            {
                2
            } else {
                1
            };
            (runs, runs)
        }
        "native_create_recovery" => (3, 3),
        // One reference execution of the control; two compile-and-run cycles,
        // the successful join and the injected failure.
        "native_join_failure" => (1, 2),
        "native_task_scenarios" => {
            let runs = expected
                .native_task_scenarios
                .as_ref()
                .expect("mode metadata")
                .scenarios
                .len();
            (runs, runs)
        }
        "native_heap_scenarios" => {
            let runs = expected
                .native_heap_scenarios
                .as_ref()
                .expect("mode metadata")
                .scenarios
                .len();
            (runs, runs)
        }
        "checked_mutex_sanitized" => (1, 1),
        _ => (0, 0),
    };
    let all_three = vec!["linux", "macos", "windows"];
    // The two sanitized runners return on Windows after asserting that the
    // emitted C exists, so emission is reached on all three hosts while the
    // sanitized compile-and-run is Unix-only.
    let deep_gate = if matches!(verdict, "native_heap_scenarios" | "checked_mutex_sanitized") {
        vec!["linux", "macos"]
    } else {
        all_three.clone()
    };
    // CT50's native observation needs Linux `MAP_FIXED_NOREPLACE`; every other
    // stage of that case runs wherever the threads backend does.
    let native_gate = if case_id == "CT50" {
        vec!["linux"]
    } else {
        deep_gate.clone()
    };

    let refused = !observed_codes.is_empty();
    let refused_before_lowering = refused
        && observed_codes
            .iter()
            .all(|code| diagnostic_band(code) < 500);
    let reached_model_check = !refused
        || observed_codes
            .iter()
            .any(|code| diagnostic_band(code) >= 600);

    // `reference` is `None` for a route that has no reference-execution stage at
    // all. The epoch route is check-only, so it has none: an admitted epoch is a
    // checked accounting statement, not a program with an observation.
    let (lowering, check, reference) = match route {
        "task" => (
            "task_lowering",
            Some("task_ledger_check"),
            Some("task_reference_execution"),
        ),
        "mutex_guard" => (
            "mutex_lowering",
            Some("mutex_invariant_check"),
            Some("mutex_reference_execution"),
        ),
        "sequential" => (
            "sequential_whole_program",
            None,
            Some("sequential_reference_execution"),
        ),
        "epoch" => ("epoch_lowering", Some("epoch_bound_check"), None),
        other => panic!("{case_id}: route `{other}` has no mapped case shape"),
    };

    let mut plan: BTreeMap<&'static str, StagePlan> = STAGE_NAMES
        .iter()
        .map(|stage| (*stage, StagePlan::absent("not_applicable")))
        .collect();
    plan.insert("parse", StagePlan::executed(base, 1, None));
    plan.insert("elaborate", StagePlan::executed(base, 1, None));
    if !refused_before_lowering {
        plan.insert(lowering, StagePlan::executed(base, 1, None));
    }
    if let Some(check) = check
        && !refused_before_lowering
        && reached_model_check
    {
        plan.insert(check, StagePlan::executed(base, 1, None));
    }
    // The plain sequential accept mode runs its reference execution inside
    // `compile_files` itself, so it has no separate runner and no host gate.
    if verdict == "accept"
        && expected.outcome.is_some()
        && let Some(reference) = reference
    {
        plan.insert(reference, StagePlan::executed(base, 1, None));
    }
    if let Some(deep) = deep {
        plan.insert(
            reference.expect("a native runner implies a reference-execution stage"),
            StagePlan::executed(deep, reference_runs, Some(deep_gate.clone())),
        );
        plan.insert(
            "c_emission",
            StagePlan::executed(deep, native_runs, Some(all_three.clone())),
        );
        plan.insert(
            "native_execution",
            StagePlan::executed(deep, native_runs, Some(native_gate)),
        );
    }
    plan.insert(
        "native_sanitized",
        if SANITIZED_GATED_CASES.contains(&case_id) {
            StagePlan::executed(
                deep.expect("a gated sanitized case has a native runner"),
                native_runs,
                Some(deep_gate),
            )
        } else if SANITIZER_TRANSCRIPT_CASES.contains(&case_id) {
            StagePlan::absent("recorded_not_gated")
        } else if !route_stages(route).contains(&"native_sanitized") {
            // A route that emits no C has nothing to sanitize: structurally
            // absent, not an open item somebody could close.
            StagePlan::absent("not_applicable")
        } else {
            StagePlan::absent("unassessed")
        },
    );
    // No CT fixture has a theorem bound to it. Advancing this is card T00.7 and
    // requires editing this line deliberately.
    plan.insert("proof", StagePlan::absent("unassessed"));
    plan
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
    assert_eq!(
        expected.epoch_reject.is_some(),
        EPOCH_REJECT_CASES.contains(&case.id.as_str()),
        "{}: epoch-route rejection metadata is pinned to its own case set",
        case.id
    );
    assert_eq!(
        expected.epoch_bound_figures.is_some(),
        expected.verdict == "epoch_bound_figures"
    );
    assert_eq!(
        expected.epoch_bound_figures.is_some(),
        EPOCH_FIGURE_CASES.contains(&case.id.as_str()),
        "{}: accepted epoch-certificate evidence is pinned to its own case set",
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
    if let Some(figures) = &expected.epoch_bound_figures {
        assert_eq!(
            figures.execution_profile, "FlatEpochBoundV1",
            "{}: the accepted epoch cases take the flat-epoch route",
            case.id
        );
        assert!(
            !figures.figures.is_empty(),
            "{}: an accepted epoch case states the certificate it measured",
            case.id
        );
        check_epoch_controls_shape(&case.id, &figures.controls, &mut required_files);
    }
    if let Some(reject) = &expected.epoch_reject {
        assert!(
            !reject.diagnostic_sequence.is_empty(),
            "{}: an epoch-route rejection pins its exact ordered code vector",
            case.id
        );
        assert!(
            reject
                .controls
                .iter()
                .any(|control| control.expectation == "accept"),
            "{}: an epoch rejection needs an accepted control, or the refusal is \
             only `the profile cannot express this at all`",
            case.id
        );
        check_epoch_controls_shape(&case.id, &reject.controls, &mut required_files);
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

    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", source_path.display()));
    let files = [SourceFile::new(recorded.to_string(), source)];
    let result = compile_files(
        &files,
        &CompileOptions {
            entry: expected.entry.clone(),
            ..CompileOptions::default()
        },
    );

    // Per-stage manifest evidence is derived here, from this harness's own
    // dispatch plus what the compiler actually reported on this fixture, and the
    // manifest must record exactly that. This is the T00.1/T00.4 repair: the
    // manifest no longer carries hand-written status strings whose reading
    // depended on the route.
    let observed_codes: Vec<&str> = match &result {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect(),
    };
    let plan = derive_stage_plan(&case.id, &expected, &observed_codes);
    let route = case
        .route
        .as_deref()
        .unwrap_or_else(|| panic!("{}: a mapped case must declare its route", case.id));
    assert_eq!(
        route,
        route_for_case(&case.id, &expected.verdict),
        "{}: the declared route must be the one this verdict mode really takes",
        case.id
    );
    assert_eq!(
        case.harness_verdict_mode.as_deref(),
        Some(expected.verdict.as_str()),
        "{}: the manifest must record the harness mode that produced its stage evidence",
        case.id
    );
    let route_vocabulary: BTreeSet<&str> = route_stages(route).iter().copied().collect();
    let mut claimed_stages: Vec<&str> = case.stages.keys().map(String::as_str).collect();
    claimed_stages.sort_unstable();
    let mut all_stages: Vec<&str> = STAGE_NAMES.to_vec();
    all_stages.sort_unstable();
    assert_eq!(
        claimed_stages, all_stages,
        "{}: every case states every stage; a missing key is not an absent stage",
        case.id
    );
    for stage in STAGE_NAMES {
        let record = &case.stages[*stage];
        let planned = &plan[stage];
        assert!(
            STAGE_STATUSES.contains(&record.status.as_str()),
            "{}: unknown stage status `{}` for `{stage}`",
            case.id,
            record.status
        );
        if !route_vocabulary.contains(stage) {
            assert_eq!(
                record.status, "not_applicable",
                "{}: `{stage}` is not a stage of route `{route}`, so it is structurally \
                 absent, never unassessed",
                case.id
            );
        }
        assert_eq!(
            record.status, planned.status,
            "{}: stage `{stage}` claims `{}` but this harness derives `{}`",
            case.id, record.status, planned.status
        );
        match record.status.as_str() {
            "executed" => {
                let executor = record.executor.as_deref().unwrap_or_else(|| {
                    panic!("{}: `{stage}` is executed but names no executor", case.id)
                });
                assert!(
                    RUNNER_REGISTRY.contains(&executor),
                    "{}: `{stage}` names executor `{executor}`, which is not a runner in \
                     this harness",
                    case.id
                );
                assert_eq!(
                    Some(executor),
                    planned.executor,
                    "{}: `{stage}` is executed by a different runner than it claims",
                    case.id
                );
                let artifact = record.artifact.as_deref().unwrap_or_else(|| {
                    panic!("{}: `{stage}` is executed but names no artifact", case.id)
                });
                assert_eq!(
                    artifact, recorded,
                    "{}: `{stage}` must be executed on the case's own fixture",
                    case.id
                );
                assert!(
                    repo_root().join(artifact).is_file(),
                    "{}: `{stage}` names artifact `{artifact}`, which does not exist",
                    case.id
                );
                assert_eq!(
                    record.observations, planned.observations,
                    "{}: `{stage}` observation count must equal the runner's invocations",
                    case.id
                );
                assert_eq!(
                    record
                        .host_gate
                        .as_ref()
                        .map(|gate| gate.iter().map(String::as_str).collect::<Vec<_>>()),
                    planned.host_gate.clone(),
                    "{}: `{stage}` must record the exact host gate of its runner, so the \
                     status cannot be read as host-unconditional",
                    case.id
                );
                assert!(record.record.is_none());
            }
            "recorded_not_gated" => {
                assert_eq!(
                    *stage, "native_sanitized",
                    "{}: only sanitizer evidence is currently recorded rather than gated",
                    case.id
                );
                let pointer = record.record.as_deref().unwrap_or_else(|| {
                    panic!(
                        "{}: `{stage}` is recorded_not_gated but names no record",
                        case.id
                    )
                });
                assert!(
                    repo_root().join(pointer).is_file(),
                    "{}: `{stage}` names record `{pointer}`, which does not exist",
                    case.id
                );
                assert!(
                    record.executor.is_none()
                        && record.artifact.is_none()
                        && record.observations.is_none()
                        && record.host_gate.is_none(),
                    "{}: a recorded transcript has no executor, artifact, count or host \
                     gate — that is what distinguishes it from executed evidence",
                    case.id
                );
            }
            _ => assert!(
                record.executor.is_none()
                    && record.artifact.is_none()
                    && record.observations.is_none()
                    && record.host_gate.is_none()
                    && record.record.is_none(),
                "{}: `{stage}` is `{}` and may carry no evidence fields",
                case.id,
                record.status
            ),
        }
    }
    // T00.3: the sanitizer gate. `executed` here is admissible only for a case
    // a runner really compiles with `-fsanitize=…` and runs.
    assert_eq!(
        case.stages["native_sanitized"].status == "executed",
        SANITIZED_GATED_CASES.contains(&case.id.as_str()),
        "{}: a sanitized-native claim requires the harness-gated run",
        case.id
    );
    assert_eq!(
        case.stages["native_sanitized"].status == "recorded_not_gated",
        SANITIZER_TRANSCRIPT_CASES.contains(&case.id.as_str()),
        "{}: transcript-only sanitizer evidence is pinned to its own case set",
        case.id
    );

    // The fixture repeats its executed stage set in the same vocabulary, and it
    // must agree with what the manifest and this harness derived.
    let executed: BTreeSet<&str> = expected
        .stages_executed
        .iter()
        .map(String::as_str)
        .collect();
    for stage in &executed {
        assert!(
            STAGE_NAMES.contains(stage),
            "{}: unknown stage `{stage}` in expected.json",
            case.id
        );
    }
    let derived_executed: BTreeSet<&str> = STAGE_NAMES
        .iter()
        .copied()
        .filter(|stage| plan[stage].status == "executed")
        .collect();
    assert_eq!(
        executed, derived_executed,
        "{}: expected.json's stages_executed must match the derived plan",
        case.id
    );
    assert!(
        executed.contains("parse") && executed.contains("elaborate"),
        "{}: a mapped case must at least parse and elaborate its own fixture",
        case.id
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
    if let Some(reject) = &expected.epoch_reject {
        run_epoch_controls(&directory, &case.id, &reject.controls);
    }
    match expected.verdict.as_str() {
        "epoch_bound_figures" => {
            assert!(
                expected.diagnostics.is_empty()
                    && expected.outcome.is_none()
                    && expected.trace.is_none(),
                "{}: an admitted epoch has no execution post-state; its evidence is \
                 the certificate",
                case.id
            );
            run_epoch_bound_figures(
                &directory,
                &case.id,
                expected
                    .epoch_bound_figures
                    .as_ref()
                    .expect("mode metadata"),
                result,
            );
        }
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
            let wants_execution = executed.contains("sequential_reference_execution");
            assert_eq!(
                wants_execution,
                expected.outcome.is_some(),
                "{}: the reference execution stage and the expected outcome must agree",
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
            if let Some(reject) = &expected.epoch_reject {
                // Same discipline on the epoch route. CT36's own control shows
                // why the whole vector matters here: one byte below the summed
                // peak refuses on the credits *and* on the epoch, so a
                // head-only assertion would not distinguish a per-task check
                // from a whole-epoch one.
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
    assert_eq!(manifest.schema_version, 2);
    assert_eq!(manifest.cases.len(), TOTAL_CASES);
    // The manifest states its own vocabulary so a reader knows what the words
    // mean; this file is the authority, so data can never widen it.
    assert_eq!(
        manifest.stage_vocabulary.stages, STAGE_NAMES,
        "manifest stage vocabulary drifted from the harness"
    );
    assert_eq!(
        manifest.stage_vocabulary.statuses, STAGE_STATUSES,
        "manifest status vocabulary drifted from the harness"
    );
    assert_eq!(
        manifest
            .stage_vocabulary
            .routes
            .iter()
            .map(|(name, stages)| (
                name.as_str(),
                stages.iter().map(String::as_str).collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>(),
        ROUTE_STAGES
            .iter()
            .map(|(name, stages)| (*name, stages.to_vec()))
            .collect::<Vec<_>>(),
        "manifest route vocabulary drifted from the harness"
    );
    for (_, stages) in ROUTE_STAGES {
        for stage in *stages {
            assert!(
                STAGE_NAMES.contains(stage),
                "route vocabulary names `{stage}`, which is not a stage"
            );
        }
    }

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
                        && case.evidence_target.is_none()
                        && case.route.is_none()
                        && case.harness_verdict_mode.is_none(),
                    "{}: an unmapped case must not carry fixture metadata",
                    case.id
                );
                // An unmapped case has no route, so no stage of it is
                // structurally absent either: every stage is genuinely
                // unassessed, and carries no evidence field.
                assert!(
                    case.stages.values().all(|record| record
                        == &StageRecord {
                            status: "unassessed".to_owned(),
                            executor: None,
                            artifact: None,
                            observations: None,
                            host_gate: None,
                            record: None,
                        }),
                    "{}: an unmapped case must keep all stages `unassessed` with no evidence",
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

/// The moving count is the W6 genericity metric: 28 of 50 CT cases are mapped
/// as executed source-level tests; 22 remain unmapped. CT13/CT15/CT26 and the
/// CT03/CT17 sanitized heap scenarios and the accepted controls carried by
/// CT04/CT05/CT18/CT27/CT28/CT29/CT30/CT31/CT32/CT48 execute on Linux and macOS.
/// The manifest's `evidence_target` still identifies the archived Linux evidence
/// run; for CT28/CT31/CT32 it records only the `Q500` host gate, because those
/// accepted controls are multi-task modules this backend renders in no thread at
/// all (published checked with `c_source: None`, card T03-F §3.5). Mapping is not conformance.
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

fn stage_depth(stage: &str) -> u8 {
    STAGE_DEPTH
        .iter()
        .find(|(name, _)| *name == stage)
        .unwrap_or_else(|| panic!("stage `{stage}` has no depth rank"))
        .1
}

/// The stage-derived CT summary (T00.9). Every number here is computed from the
/// manifest's per-stage evidence, which `run_mapped_case` has independently
/// checked against this harness's own dispatch. Nothing is hand-kept, and the
/// `mapped`/`unmapped` pair is retained beside the derived rows rather than
/// standing alone: on its own it treats a source-rejection case and a
/// five-scenario sanitized case as one identical unit of coverage.
fn ct_stage_summary(manifest: &Manifest) -> serde_json::Value {
    use serde_json::{Map, Value, json};
    let mut histogram: BTreeMap<&str, BTreeMap<&str, usize>> = STAGE_NAMES
        .iter()
        .map(|stage| (*stage, BTreeMap::new()))
        .collect();
    let mut observations: BTreeMap<&str, usize> = BTreeMap::new();
    let mut host_scope: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut depth_profile: BTreeMap<&str, usize> = BTreeMap::new();
    let mut case_depth: BTreeMap<&str, &str> = BTreeMap::new();
    let mut mapped = 0usize;
    let mut unmapped = 0usize;
    for case in &manifest.cases {
        if case.mapping_status != "mapped" {
            unmapped += 1;
            continue;
        }
        mapped += 1;
        let mut deepest: Option<&'static str> = None;
        for stage in STAGE_NAMES {
            let record = &case.stages[*stage];
            *histogram
                .get_mut(stage)
                .expect("stage")
                .entry(record.status.as_str())
                .or_insert(0) += 1;
            if record.status != "executed" {
                continue;
            }
            *observations.entry(stage).or_insert(0) += record
                .observations
                .expect("an executed stage counts its runs");
            let hosts = host_scope.entry(stage).or_default();
            match &record.host_gate {
                Some(gate) => hosts.extend(gate.iter().cloned()),
                None => {
                    hosts.insert("any".to_owned());
                }
            }
            if deepest.is_none_or(|current| stage_depth(stage) > stage_depth(current)) {
                deepest = Some(stage);
            }
        }
        let deepest = deepest.expect("a mapped case executes at least the parser");
        *depth_profile.entry(deepest).or_insert(0) += 1;
        case_depth.insert(case.id.as_str(), deepest);
    }
    let mut histogram_json = Map::new();
    for stage in STAGE_NAMES {
        histogram_json.insert(
            (*stage).to_owned(),
            json!(
                histogram[stage]
                    .iter()
                    .map(|(status, count)| ((*status).to_owned(), *count))
                    .collect::<BTreeMap<String, usize>>()
            ),
        );
    }
    json!({
        "mapped": mapped,
        "unmapped": unmapped,
        "stage_histogram": Value::Object(histogram_json),
        "observation_counts": observations,
        "host_scope": host_scope,
        "depth_profile": depth_profile,
        "case_depth": case_depth,
    })
}

/// The consumable acceptance numbers are generated from the manifest, not
/// hand-kept beside it (T00.9 / T00 finding O7). `acceptance.json` names the
/// fields the generator owns; this test recomputes each of them and refuses any
/// drift, so the record cannot claim depth its stage evidence does not carry.
#[test]
fn ct_acceptance_summary_is_derived_from_the_manifest() {
    let manifest = load_manifest();
    let derived = ct_stage_summary(&manifest);
    let path = repo_root().join("docs/reviews/r0_3/acceptance.json");
    let acceptance: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
    let evidence = acceptance
        .get("ct_evidence")
        .expect("acceptance.json carries the stage-derived CT block");
    let generated: Vec<&str> = evidence
        .get("generated_fields")
        .expect("the CT block names the fields its generator owns")
        .as_array()
        .expect("generated_fields is a list")
        .iter()
        .map(|name| name.as_str().expect("field name"))
        .collect();
    assert_eq!(
        generated,
        [
            "ct_mapped",
            "ct_unmapped",
            "stage_histogram",
            "observation_counts",
            "host_scope",
            "depth_profile",
            "case_depth",
        ],
        "the generated field set is itself pinned, so a field cannot quietly \
         become hand-maintained"
    );
    assert_eq!(
        acceptance.get("ct_mapped"),
        Some(&derived["mapped"]),
        "ct_mapped is generated from the manifest"
    );
    assert_eq!(
        acceptance.get("ct_unmapped"),
        Some(&derived["unmapped"]),
        "ct_unmapped is generated from the manifest"
    );
    for field in [
        "stage_histogram",
        "observation_counts",
        "host_scope",
        "depth_profile",
        "case_depth",
    ] {
        assert_eq!(
            evidence.get(field),
            Some(&derived[field]),
            "acceptance.json `{field}` is not what the manifest's stage evidence derives"
        );
    }
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
            .contains("requires a Linux, macOS or Windows threads target"),
        "{diagnostics:?}"
    );
}

/// The controls a mutex rejection carries. An `accept` control compiles through
/// the Linux/macOS/Windows emission path, so on any other host it must arrive as the
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
                if !cfg!(any(
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "windows"
                )) {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{id} control {}: source/core checked; threaded emission unexecuted on this host",
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
/// Shape rules shared by both epoch verdict modes: an accepted control states a
/// whole certificate and no diagnostics, a refused one states codes and no
/// certificate, and every control source joins the fixture inventory.
fn check_epoch_controls_shape(
    case_id: &str,
    controls: &[EpochControl],
    required_files: &mut BTreeSet<String>,
) {
    assert!(
        !controls.is_empty(),
        "{case_id}: an epoch case needs at least one control"
    );
    for control in controls {
        match control.expectation.as_str() {
            "accept" => {
                assert!(
                    control.diagnostic_sequence.is_empty(),
                    "{case_id}: an accepted control has no diagnostics"
                );
                assert!(
                    control.task_count.is_some() && !control.figures.is_empty(),
                    "{case_id}: an accepted epoch control states the certificate it \
                     measured, so `it also checks` is never the whole control"
                );
            }
            "reject" => {
                assert!(
                    !control.diagnostic_sequence.is_empty(),
                    "{case_id}: a refused control pins its codes"
                );
                assert!(
                    control.task_count.is_none() && control.figures.is_empty(),
                    "{case_id}: a refused control publishes no certificate"
                );
            }
            other => panic!("{case_id}: unknown control expectation `{other}`"),
        }
        assert!(required_files.insert(control.source.clone()));
    }
}

/// Every nonzero axis of every quantity on a published epoch certificate, keyed
/// `field.axis`, plus its task count. Rebuilding the whole map here — rather
/// than looking up the keys a fixture happens to name — is what stops a fixture
/// from pinning only the figures that suit it.
fn epoch_certificate_figures(
    certificate: &qcore_core::epoch_accounting::EpochBoundCertificate,
) -> BTreeMap<String, u64> {
    let mut figures = BTreeMap::new();
    for (field, quantity) in [
        ("shared_entry_union", &certificate.shared_entry_union),
        (
            "concurrent_private_peak",
            &certificate.concurrent_private_peak,
        ),
        ("runtime_overhead", &certificate.runtime_overhead),
        ("epoch_peak", &certificate.epoch_peak),
        ("completed_residency", &certificate.completed_residency),
        ("exit_residency", &certificate.exit_residency),
        ("work_total", &certificate.work_total),
        ("granted_credits", &certificate.granted_credits),
        ("allocation", &certificate.allocation),
    ] {
        for (axis, amount) in quantity.iter() {
            figures.insert(format!("{field}.{axis}"), amount);
        }
    }
    figures
}

/// CT35 and CT36: the accepted half of the `source.epoch.bound.v1` route. The
/// case's own fixture must be admitted under the flat-epoch profile with exactly
/// the recorded certificate — every nonzero axis, the task count and the premise
/// set — and each control must land on its own side of the boundary the case is
/// about. The route is check-only, so this runner asserts that the published
/// module carries no emitted C and no reference observation rather than looking
/// for either.
fn run_epoch_bound_figures(
    directory: &Path,
    case_id: &str,
    figures: &EpochBoundFigures,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    let module = result.unwrap_or_else(|diagnostics| {
        panic!(
            "{case_id}: expected an admitted epoch but the checker refused:\n{}",
            diagnostics
                .iter()
                .map(format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    let certificate = epoch_certificate_of(case_id, &module, &figures.execution_profile);
    assert_eq!(
        certificate.task_count, figures.task_count,
        "{case_id}: task count"
    );
    assert_eq!(
        certificate
            .assumption_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        figures.assumptions,
        "{case_id}: the certificate's premise set is part of the measurement — a \
         dropped premise is a changed claim"
    );
    assert_eq!(
        epoch_certificate_figures(certificate),
        figures.figures,
        "{case_id}: published certificate figures"
    );
    run_epoch_controls(directory, case_id, &figures.controls);
}

/// The certificate of an accepted epoch module, with the route and the
/// check-only shape asserted first.
fn epoch_certificate_of<'module>(
    case_id: &str,
    module: &'module qcore_compiler::CompiledModule,
    profile: &str,
) -> &'module qcore_core::epoch_accounting::EpochBoundCertificate {
    assert_eq!(
        module.execution_profile,
        execution_profile(profile),
        "{case_id}: the module must take the route it names"
    );
    assert!(
        module.c_source.is_none() && module.reference_observation.is_none(),
        "{case_id}: the flat-epoch route is check-only — it emits no C and runs no \
         reference execution, which is why its manifest row has no such stage"
    );
    let certificate = &module
        .checked
        .as_checked_epoch_bound()
        .unwrap_or_else(|| panic!("{case_id}: the checked artifact variant is explicit"))
        .certificate;
    // Every certificate this route publishes — a case's or a control's — carries
    // all four premises. They are declared assumptions, not facts the route
    // establishes, so a published certificate that had quietly shed one would be
    // a stronger claim than anything measured here.
    assert_eq!(
        certificate
            .assumption_names()
            .into_iter()
            .collect::<Vec<_>>(),
        [
            "DeclaredRuntimeOverhead",
            "DeclaredTaskFootprints",
            "DisjointSharedStorageCells",
            "NoScheduleModel",
        ],
        "{case_id}: the flat-epoch premise set is fixed"
    );
    certificate
}

/// The controls an epoch case carries, on either verdict mode. An accepted
/// control is held to the same standard as its case: the whole certificate, not
/// merely acceptance.
fn run_epoch_controls(directory: &Path, case_id: &str, controls: &[EpochControl]) {
    for control in controls {
        let path = directory.join(&control.source);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let result = compile_files(
            &[SourceFile::new(path.to_string_lossy(), source)],
            &CompileOptions::default(),
        );
        match control.expectation.as_str() {
            "accept" => {
                let module = result.unwrap_or_else(|diagnostics| {
                    panic!(
                        "{case_id} control {} must check, so the case's measurement is \
                         not vacuous:\n{}",
                        control.source,
                        diagnostics
                            .iter()
                            .map(format_diagnostic)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                });
                let certificate = epoch_certificate_of(case_id, &module, "FlatEpochBoundV1");
                assert_eq!(
                    Some(certificate.task_count),
                    control.task_count,
                    "{case_id} control {}: task count",
                    control.source
                );
                assert_eq!(
                    epoch_certificate_figures(certificate),
                    control.figures,
                    "{case_id} control {}: published certificate figures",
                    control.source
                );
            }
            "reject" => {
                let diagnostics = match result {
                    Err(diagnostics) => diagnostics,
                    Ok(_) => panic!("{case_id} control {} must be refused", control.source),
                };
                assert_eq!(
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect::<Vec<_>>(),
                    control.diagnostic_sequence,
                    "{case_id} control {}: exact diagnostic vector mismatch; actual:\n{}",
                    control.source,
                    diagnostics
                        .iter()
                        .map(format_diagnostic)
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            other => panic!("{case_id}: unknown control expectation `{other}`"),
        }
    }
}

fn execution_profile(label: &str) -> qcore_compiler::SourceExecutionProfile {
    use qcore_compiler::SourceExecutionProfile as Profile;
    match label {
        "FlatEpochBoundV1" => Profile::FlatEpochBoundV1,
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
/// through the Linux/macOS/Windows emission path, so on any other host it must arrive as
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
                if !cfg!(any(
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "windows"
                )) {
                    unsupported_target(&result.unwrap_err());
                    eprintln!(
                        "{id} control {}: source/core checked; threaded emission unexecuted on this host",
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
/// then compile that C under ASan/UBSan (and LSan on Linux) and run it once per fixed
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
    if cfg!(target_os = "windows") {
        assert!(
            result
                .as_ref()
                .expect("Windows source must check and emit")
                .c_source
                .is_some()
        );
        eprintln!(
            "{}: emitted on Windows; Unix sanitizer evidence unexecuted on this host",
            expected.id
        );
        return;
    }
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: source/core checked; reference/sanitized-native execution unexecuted on \
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
    let mut command = Command::new(compiler);
    command
        .args(&native.sanitizer_flags)
        .args(&scenario.defines);
    if cfg!(target_os = "macos") {
        command.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    let compiled = command
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
            if scenario.detect_leaks && !cfg!(target_os = "macos") {
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
/// compile the emitted C under ASan/UBSan (and LSan on Linux) and run it. The native stdout is
/// compared line for line with `interpret_mutex_program`, which is written
/// against the checked event list and never reads the emitted C.
fn run_checked_mutex_sanitized(
    expected: &ExpectedCase,
    result: Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>>,
) {
    use qcore_compiler::{ConcurrencyAssumption, ConcurrencyProfile, SourceExecutionProfile};
    let mutex = expected.checked_mutex_sanitized.as_ref().unwrap();
    if cfg!(target_os = "windows") {
        assert!(
            result
                .as_ref()
                .expect("Windows source must check and emit")
                .c_source
                .is_some()
        );
        eprintln!(
            "{}: emitted on Windows; Unix sanitizer evidence unexecuted on this host",
            expected.id
        );
        return;
    }
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: source/core checked; mutex reference/native execution unexecuted on this host \
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
    let c = module.c_source.as_deref().expect("threaded emission");
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
    let asan_options = if cfg!(target_os = "macos") {
        // Apple's AddressSanitizer does not implement LeakSanitizer.
        "detect_leaks=0:halt_on_error=1"
    } else {
        &mutex.asan_options
    };
    let run = Command::new(&binary)
        .env("ASAN_OPTIONS", asan_options)
        .env("UBSAN_OPTIONS", &mutex.ubsan_options)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "emitted program failed under the platform sanitizers:\n{}",
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
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT19 joined-sibling control: source/core checked; threaded reference/native evidence unexecuted on this host"
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
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT15: source/core checked; native/reference execution unexecuted on this host (manifest evidence_target=linux)"
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
    #[cfg(target_os = "windows")]
    return windows_c::run(
        source,
        Some(&format!(
            "#include \"{}\"",
            header.to_string_lossy().replace('\\', "/")
        )),
        defines,
    );
    #[cfg(not(target_os = "windows"))]
    {
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
        let mut command = Command::new(compiler);
        command
            .args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"])
            .args(defines);
        if cfg!(target_os = "macos") {
            // Fault-injection headers are force-included before the generated
            // source, so make their `<threads.h>` include resolve to the same
            // pthread-backed compatibility surface used by emitted C.
            command.arg("-I").arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/apple-threads"
            ));
        }
        let compiled = command
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
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "CT13: source/core checked; native/reference creation-recovery evidence unexecuted on this host"
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
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: owned-task reference/native evidence unexecuted on this host",
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
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        unsupported_target(&result.unwrap_err());
        eprintln!(
            "{}: threaded task scenarios unexecuted on this host",
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
        if expected.id == "CT50" && !cfg!(target_os = "linux") {
            // CT50 is not a threading fixture: its native evidence requires
            // Linux MAP_FIXED_NOREPLACE. The source, reference, CFG, and C
            // emission stages still execute on macOS; only that OS-specific
            // mapping-address observation remains Linux-scoped.
            eprintln!("CT50: Linux MAP_FIXED_NOREPLACE native observation unexecuted on this host");
            continue;
        }
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
            ..CompileOptions::default()
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
            ..CompileOptions::default()
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
