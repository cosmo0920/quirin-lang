//! Windows-native observations; these do not substitute for Unix sanitizer runs.
#![cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::task_execution::{
    TaskExecutionScript, execute_task_module, execute_task_module_with_worker_allocations,
    render_task_observation,
};

#[test]
fn winpthreads_subset_preserves_results_failures_and_mutex_exclusion() {
    let output = windows_c::run(include_str!("fixtures/windows_threads.c"), None, &[]);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[test]
fn windows_f02_renderings_satisfy_the_incoming_trace_relation_and_emission_pins() {
    use qcore_core::task_trace_relation::{AdmittedCausalOrder, parse_observed_trace, relate};
    // Deliberate pin update (card `TRACE-STDIO`, 2026-09-10), matching
    // `f02_trace_relation.rs`: the synthesized trace instrumentation moved onto
    // the shared `qcore_core::io_emit` route, so the generated C moved while
    // the printed bytes did not. Measured on Linux from the same emission this
    // target compiles; **not re-measured on Windows here** — no MSVC host was
    // available to this card, and the emission is host-independent apart from
    // the embedded compatibility header, which this card did not touch.
    for (name, source, pin) in [
        (
            "round-trip",
            include_str!("fixtures/ct/CT01/case.qcore"),
            "e33534d5c7da5c6c",
        ),
        (
            "concurrent",
            include_str!("fixtures/f02/concurrent_round_trip.qcore"),
            "910e7f975a896ff1",
        ),
    ] {
        let module = compile_files(
            &[SourceFile::new(format!("{name}.qcore"), source)],
            &CompileOptions::default(),
        )
        .unwrap();
        let c = module.c_source.as_ref().unwrap();
        // Reproduce the LF checkout used by the Linux evidence pins.
        let digest = c
            .replace("\r\n", "\n")
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, byte| {
                (h ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
            });
        assert_eq!(format!("{digest:016x}"), pin);
        let reference = execute_task_module(
            module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript::default(),
        )
        .unwrap();
        let order = AdmittedCausalOrder::derive(&reference).unwrap();
        let output = windows_c::run(c, None, &[]);
        assert!(output.status.success(), "{output:?}");
        let mut events = parse_observed_trace(&String::from_utf8(output.stdout).unwrap()).unwrap();
        assert!(relate(&order, &events).unwrap().is_reference_order());
        events.pop();
        assert!(relate(&order, &events).is_err());
    }
}

#[test]
fn windows_worker_failure_returns_and_releases_the_owned_error_payload() {
    let module = compile_files(
        &[SourceFile::new(
            "payload.qcore",
            include_str!("fixtures/ct/CT17/case.qcore"),
        )],
        &CompileOptions::default(),
    )
    .unwrap();
    for attempt in 1..=2 {
        let mut decisions = vec![true; attempt - 1];
        decisions.push(false);
        let reference = execute_task_module_with_worker_allocations(
            module.checked.as_scoped_tasks().unwrap(),
            &TaskExecutionScript::default(),
            &[(1, decisions)].into_iter().collect(),
        )
        .unwrap();
        let output = windows_c::run(
            module.c_source.as_ref().unwrap(),
            Some(include_str!("fixtures/ct/CT17/track.h")),
            &[
                "-DPAYLOAD_FAIL_TASK=1".to_owned(),
                format!("-DPAYLOAD_FAIL_ATTEMPT={attempt}"),
            ],
        );
        assert!(output.status.success(), "{output:?}");
        let trace = render_task_observation(&reference);
        assert!(trace.contains("ERROR_BUFFER"));
        assert_eq!(output.stdout, trace.as_bytes());
        let heap = String::from_utf8(output.stderr).unwrap();
        let mut allocations = std::collections::BTreeMap::new();
        let mut releases = std::collections::BTreeMap::new();
        for line in heap.lines() {
            let (kind, pointer) = line.split_once(' ').unwrap();
            let records = match kind {
                "PAYLOAD_ALLOC" => &mut allocations,
                "PAYLOAD_FREE" => &mut releases,
                _ => panic!("unexpected heap event: {line}"),
            };
            *records.entry(pointer).or_insert(0) += 1;
        }
        assert_eq!(allocations.len(), attempt);
        assert!(allocations.values().all(|count| *count == 1));
        assert_eq!(
            allocations, releases,
            "every original allocation must be released exactly once"
        );
    }
}

#[test]
fn windows_read_loan_shares_and_owned_error_match_reference() {
    // Other Windows task profiles and failure paths run in source_tasks/ct_corpus.
    for source in [
        include_str!("fixtures/ct/CT03/case.qcore"),
        include_str!("fixtures/ct/CT17/case.qcore"),
        "import std.concurrency; concurrency checked;
         fn reader(p: borrow<Buffer>) -> i32 { let x = qcore_buffer_read(p); return x; }
         fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
           let b = qcore_buffer_new(42)?;
           thread scope s { match spawn s reader(read b share 1024) {
             Result::Ok { value: h } => { let x = join move h?; drop b; return x; }
             Result::Err { error: failed } => { drop b; return 0; }
           } }
         }",
    ] {
        let module = compile_files(&[SourceFile::new("windows.qcore", source)], &CompileOptions::default()).unwrap();
        let reference = execute_task_module(module.checked.as_scoped_tasks().unwrap(), &TaskExecutionScript::default()).unwrap();
        let output = windows_c::run(module.c_source.as_ref().unwrap(), None, &[]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, render_task_observation(&reference).as_bytes());
    }
}
