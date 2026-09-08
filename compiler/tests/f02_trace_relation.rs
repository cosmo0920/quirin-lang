//! F02 — the source-to-native side of one trace relation.
//!
//! Two admitted source programs of the `ScopedOwnedTasksV1` owned-Buffer
//! worker/capsule/join path are compiled, executed natively, and the native
//! rendering is related to the reference run's admitted causal order by
//! [`qcore_core::task_trace_relation::relate`] — not by string equality.
//!
//! What this establishes: on these two runs the native rendering reports the
//! same facts, in each actor's own program order, respecting every admitted
//! causal constraint. What it does **not** establish: that the native execution
//! took the order it rendered (no shared trace clock exists across C11
//! threads), nor anything about programs outside this profile. Native trace
//! replay is not general C refinement.
#![cfg(target_os = "linux")]

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ObservedEvent, TRACE_RELATION_IDENTITY, parse_observed_trace, relate,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const ROUND_TRIP_SOURCE: &str = include_str!("fixtures/ct/CT01/case.qcore");
const CONCURRENT_SOURCE: &str = include_str!("fixtures/f02/concurrent_round_trip.qcore");
const ROUND_TRIP_TRACE: &str =
    include_str!("../../crates/qcore-core/tests/fixtures/f02/round_trip.trace");
const CONCURRENT_TRACE: &str =
    include_str!("../../crates/qcore-core/tests/fixtures/f02/concurrent_round_trip.trace");

/// The exact compiler arguments bound into the recorded evidence, before the
/// input and output paths.
const COMPILE_FLAGS: [&str; 7] = [
    "-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror", "-g", "-O1",
];

/// Digests of the generated C11 for the two selected programs. A change to
/// emission retires the recorded native evidence rather than silently
/// reinterpreting it, so updating these is a deliberate act that must be
/// recorded in `docs/reports/remaining-f02/README.md`.
const ROUND_TRIP_C_DIGEST: &str = "7d39f7c9157db610";
const CONCURRENT_C_DIGEST: &str = "3cb6a22168b481b3";

static NEXT: AtomicU64 = AtomicU64::new(0);

fn digest(bytes: &[u8]) -> String {
    let mut accumulator: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        accumulator ^= u64::from(*byte);
        accumulator = accumulator.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{accumulator:016x}")
}

fn clang() -> String {
    std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into())
}

fn clang_identity() -> (String, String) {
    let version = Command::new(clang()).arg("--version").output().unwrap();
    let target = Command::new(clang()).arg("-dumpmachine").output().unwrap();
    (
        String::from_utf8_lossy(&version.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned(),
        String::from_utf8_lossy(&target.stdout).trim().to_owned(),
    )
}

struct NativeRun {
    module: CompiledModule,
    c_digest: String,
    stdout: String,
    executable_digest: String,
}

fn build_and_run(name: &str, source: &str) -> NativeRun {
    let module = compile_files(
        &[SourceFile::new(format!("{name}.qcore"), source)],
        &CompileOptions::default(),
    )
    .unwrap();
    assert!(module.checked.as_scoped_tasks().is_some());
    let generated = module.c_source.as_ref().unwrap().clone();
    let directory = std::env::temp_dir().join(format!(
        "qcore-f02-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    fs::write(&c, &generated).unwrap();
    let compiled = Command::new(clang())
        .args(COMPILE_FLAGS)
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let executable_digest = digest(&fs::read(&executable).unwrap());
    let run = Command::new(&executable).output().unwrap();
    assert!(run.status.success(), "{run:?}");
    fs::remove_dir_all(&directory).unwrap();
    NativeRun {
        module,
        c_digest: digest(generated.as_bytes()),
        stdout: String::from_utf8(run.stdout).unwrap(),
        executable_digest,
    }
}

fn reference_events(module: &CompiledModule) -> Vec<ObservedEvent> {
    let observation = execute_task_module(
        module.checked.as_scoped_tasks().unwrap(),
        &TaskExecutionScript::default(),
    )
    .unwrap();
    let order = AdmittedCausalOrder::derive(&observation).unwrap();
    order.events().to_vec()
}

/// The declared domain fixtures in `qcore-core` are the real runs, not a
/// hand-written approximation of them.
#[test]
fn the_declared_trace_fixtures_are_the_reference_runs() {
    for (source, fixture) in [
        (ROUND_TRIP_SOURCE, ROUND_TRIP_TRACE),
        (CONCURRENT_SOURCE, CONCURRENT_TRACE),
    ] {
        let module = compile_files(
            &[SourceFile::new("f02.qcore", source)],
            &CompileOptions::default(),
        )
        .unwrap();
        assert_eq!(
            reference_events(&module),
            parse_observed_trace(fixture).unwrap()
        );
    }
}

#[test]
fn the_native_rendering_relates_to_the_admitted_causal_order() {
    for (name, source, expected_digest) in [
        ("round-trip", ROUND_TRIP_SOURCE, ROUND_TRIP_C_DIGEST),
        ("concurrent", CONCURRENT_SOURCE, CONCURRENT_C_DIGEST),
    ] {
        let run = build_and_run(name, source);
        assert_eq!(run.c_digest, expected_digest, "generated C changed: {name}");
        let observation = execute_task_module(
            run.module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript::default(),
        )
        .unwrap();
        let order = AdmittedCausalOrder::derive(&observation).unwrap();
        let observed = parse_observed_trace(&run.stdout).unwrap();
        let linearization = relate(&order, &observed).expect("native rendering is admitted");
        // The shipped backend buffers a child's facts and prints them from the
        // parent at join, so today it renders exactly the reference order. That
        // is a measurement about this backend, not a requirement of the
        // relation: the concurrent program has six admitted renderings.
        assert!(linearization.is_reference_order());
        assert_eq!(
            linearization.reference_positions,
            (0..observed.len()).collect::<Vec<_>>()
        );
    }
}

/// Fail-closed: a native rendering that broke an admitted constraint would be
/// refused with that constraint, not normalised into agreement.
#[test]
fn a_causally_impossible_native_rendering_would_be_refused() {
    let run = build_and_run("concurrent-adversarial", CONCURRENT_SOURCE);
    let observation = execute_task_module(
        run.module.checked.as_scoped_tasks().unwrap(),
        &TaskExecutionScript::default(),
    )
    .unwrap();
    let order = AdmittedCausalOrder::derive(&observation).unwrap();
    let mut observed = parse_observed_trace(&run.stdout).unwrap();
    // The child's read moved before the creation that produced the child.
    let read = observed.remove(5);
    observed.insert(1, read);
    let refusal = relate(&order, &observed).unwrap_err();
    assert_eq!(refusal.code(), 14);
    // A rendering that silently loses the release is refused too.
    let mut truncated = parse_observed_trace(&run.stdout).unwrap();
    truncated.remove(9);
    assert_eq!(relate(&order, &truncated).unwrap_err().code(), 3);
}

/// Acceptance 2: generated C, compiler identity, flags, target, runtime and
/// observer identity are all bound into one record.
#[test]
fn the_native_evidence_binds_its_whole_identity() {
    let (compiler_version, target) = clang_identity();
    assert!(compiler_version.to_lowercase().contains("clang"));
    assert!(!target.is_empty());
    let mut records = Vec::new();
    for (name, source, source_digest_pin) in [
        ("round-trip", ROUND_TRIP_SOURCE, ROUND_TRIP_C_DIGEST),
        ("concurrent", CONCURRENT_SOURCE, CONCURRENT_C_DIGEST),
    ] {
        let run = build_and_run(name, source);
        assert_eq!(run.c_digest, source_digest_pin);
        let observed = parse_observed_trace(&run.stdout).unwrap();
        let observation = execute_task_module(
            run.module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript::default(),
        )
        .unwrap();
        let order = AdmittedCausalOrder::derive(&observation).unwrap();
        let linearization = relate(&order, &observed).unwrap();
        let flags = COMPILE_FLAGS
            .iter()
            .map(|flag| format!("\"{flag}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let positions = format!("{:?}", linearization.reference_positions);
        let source_digest = digest(source.as_bytes());
        records.push(format!(
            concat!(
                "    {{\n",
                "      \"program\": \"{name}\",\n",
                "      \"source_digest\": \"{source}\",\n",
                "      \"generated_c_digest\": \"{c}\",\n",
                "      \"executable_digest\": \"{exe}\",\n",
                "      \"compiler\": \"{compiler}\",\n",
                "      \"target\": \"{target}\",\n",
                "      \"compile_flags\": [{flags}],\n",
                "      \"runtime\": \"c11-threads/qcore_task_start+qcore_task_collect\",\n",
                "      \"observer\": \"qcore_core::task_trace_relation::parse_observed_trace\",\n",
                "      \"relation\": \"{relation}\",\n",
                "      \"events\": {events},\n",
                "      \"admitted_edges\": {edges},\n",
                "      \"reference_positions\": {positions},\n",
                "      \"is_reference_order\": {reference}\n",
                "    }}"
            ),
            name = name,
            source = source_digest,
            c = run.c_digest,
            exe = run.executable_digest,
            compiler = compiler_version,
            target = target,
            flags = flags,
            relation = TRACE_RELATION_IDENTITY,
            events = observed.len(),
            edges = order.edges().len(),
            positions = positions,
            reference = linearization.is_reference_order(),
        ));
    }
    let record = format!("{{\n  \"runs\": [\n{}\n  ]\n}}\n", records.join(",\n"));
    if let Ok(path) = std::env::var("QCORE_F02_EVIDENCE") {
        fs::write(Path::new(&path), &record).unwrap();
    }
    assert!(record.contains(TRACE_RELATION_IDENTITY));
    assert!(record.contains("\"is_reference_order\": true"));
}
