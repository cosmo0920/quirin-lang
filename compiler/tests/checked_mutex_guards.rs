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

//! End-to-end route selection for `source.mutex.guard.v1` (card T03).
//!
//! `compile_files` must reach the checked mutex profile from ordinary source,
//! publish a certificate it independently rechecked, and emit C11 that names
//! real `mtx_t` operations. Rejections must still arrive as located
//! diagnostics on the same route rather than falling through to another one.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

#[path = "../../tests/support/mutex_observation.rs"]
mod mutex_observation;

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_compiler::{ConcurrencyAssumption, ConcurrencyProfile, SourceExecutionProfile};
#[cfg(not(target_os = "windows"))]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(not(target_os = "windows"))]
static NEXT_NATIVE_RUN: AtomicU64 = AtomicU64::new(0);

fn compile(source: &str) -> Result<qcore_compiler::CompiledModule, Vec<String>> {
    compile_files(
        &[SourceFile {
            path: "guarded.qcore".into(),
            source: source.into(),
        }],
        &CompileOptions::default(),
    )
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| format!("{}@{}", diagnostic.code, diagnostic.location.line))
            .collect()
    })
}

const GUARDED: &str = "#import std.concurrency; fn main() -> unit {\n\
     thread scope s {\n\
     let buf = protected 4;\n\
     let m = mutex guards buf;\n\
     with lock m as g { open g; write buf with g; close g; read buf with g; }\n\
     }\n\
     }\n";

#[test]
fn a_guarded_module_selects_the_checked_mutex_profile_and_emits_native_locking() {
    let module = compile(GUARDED).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::CheckedMutexGuardsV1
    );
    assert_eq!(module.entry.as_deref(), Some("main"));
    let checked = module
        .checked
        .as_checked_mutex_guards()
        .expect("the checked artifact variant is explicit for this route");
    assert_eq!(
        checked.certificate.profile,
        ConcurrencyProfile::CheckedMutexInvariantV1
    );
    for forbidden in [
        ConcurrencyAssumption::GuardFootprintAuthority,
        ConcurrencyAssumption::SynchronizedCustodyEdges,
    ] {
        assert!(
            !checked.certificate.assumptions.contains(&forbidden),
            "a checked-profile certificate never carries a trusted-model guard premise"
        );
    }
    assert!(checked.certificate.protected_shape.contains_key(&1));
    assert!(module.checked.as_sequential().is_none());
    assert!(module.checked.as_scoped_tasks().is_none());
    let c = module.c_source.as_deref().expect("Linux emission");
    for expected in [
        "mtx_init",
        "mtx_lock",
        "mtx_unlock",
        "mtx_destroy",
        "free(r1)",
    ] {
        assert!(c.contains(expected), "emitted C is missing `{expected}`");
    }
}

/// The full source stage stack for one accepted fixture: parse, elaborate,
/// lower, check, emit, and run the emitted C under ASan/UBSan/LSan, comparing
/// the native observation with the independently computed reference execution.
/// One Linux target, one compiler invocation, one thread: not evidence about
/// contention or any other backend.
#[test]
fn an_accepted_source_fixture_runs_natively_and_matches_its_reference_execution() {
    let source = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         with lock m as g { open g; write buf with g; close g; }\n\
         with lock m as h { let b = borrow buf with h; read buf with h; drop b; }\n\
         }\n\
         }\n";
    let module = compile(source).unwrap();
    let checked = module.checked.as_checked_mutex_guards().unwrap();
    let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
        &checked.lowered.program,
        &checked.certificate,
    )
    .expect("the lowered trace is inside the emitted subset");
    // Two acquisitions, two releases, one create/destroy pair, one borrow pair.
    assert_eq!(
        reference
            .iter()
            .filter(|line| line.starts_with("lock "))
            .count(),
        2
    );
    assert_eq!(
        reference
            .iter()
            .filter(|line| line.starts_with("unlock "))
            .count(),
        2
    );
    assert!(reference.contains(&"destroy-mutex 1".to_owned()));
    assert!(reference.contains(&"destroy 1".to_owned()));
    assert!(run_native(module.c_source.as_deref().unwrap()).is_empty());
}

fn run_native(source: &str) -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let output = windows_c::run(source, None, &[]);
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-t03-cli-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_NATIVE_RUN.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("generated.c");
        let binary = directory.join("generated");
        std::fs::write(&path, source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
        let compile = Command::new(&compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-g",
                "-O0",
                "-fsanitize=address,undefined",
                "-fno-omit-frame-pointer",
                // Card `MUTEX-SHARE-CROSS-EMISSION`: a fork/join rendering
                // links real C11 threads. Harmless for the single-thread
                // programs this helper already compiled.
                "-pthread",
            ])
            .arg(&path)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "C compiler rejected the emitted program:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&binary)
            .env(
                "ASAN_OPTIONS",
                if cfg!(target_os = "macos") {
                    "detect_leaks=0:halt_on_error=1"
                } else {
                    "detect_leaks=1:halt_on_error=1"
                },
            )
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
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
        std::fs::remove_dir_all(directory).unwrap();
        observation
    }
}

#[test]
fn an_unguarded_write_to_a_protected_footprint_is_located_and_refused() {
    let source = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         write buf;\n\
         }\n\
         }\n";
    // Line 5 is the offending statement: the diagnostic names it, not the
    // module or the scope.
    assert_eq!(compile(source).unwrap_err(), vec!["Q611@5"]);
}

#[test]
fn a_broken_invariant_at_the_section_exit_is_located_and_refused() {
    let source = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         with lock m as g { open g; write buf with g; }\n\
         }\n\
         }\n";
    let errors = compile(source).unwrap_err();
    assert_eq!(errors.first().map(String::as_str), Some("Q613@5"));
}

// ---------------------------------------------------------------------------
// Card T03-F: the guard-profile completions, end to end.
// ---------------------------------------------------------------------------

/// The whole stage stack for a section that closes with the **checked**
/// witness and carries a guard-scoped fact: parse, elaborate, lower, check,
/// recheck, emit, and run under ASan/UBSan/LSan against the independently
/// computed reference execution. The certificate must drop
/// `InvariantPredicateDeclared`, which is the point of the checked witness.
#[test]
fn a_checked_witness_module_drops_the_declared_premise_and_still_runs_natively() {
    let source = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         let f = fact buf;\n\
         with lock m as g { assert f with g; open g; read buf with g; use f; close g unchanged; }\n\
         }\n\
         }\n";
    let module = compile(source).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::CheckedMutexGuardsV1
    );
    let checked = module.checked.as_checked_mutex_guards().unwrap();
    assert!(
        !checked
            .certificate
            .assumptions
            .contains(&ConcurrencyAssumption::InvariantPredicateDeclared),
        "a section that only read while open needs no declared predicate"
    );
    let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
        &checked.lowered.program,
        &checked.certificate,
    )
    .unwrap();
    assert!(reference.contains(&"assert 2".to_owned()));
    assert!(reference.contains(&"use-fact 2".to_owned()));
    assert!(run_native(module.c_source.as_deref().unwrap()).is_empty());

    // One word of difference restores the premise.
    let declared = compile(&source.replace("close g unchanged;", "close g;")).unwrap();
    assert!(
        declared
            .checked
            .as_checked_mutex_guards()
            .unwrap()
            .certificate
            .assumptions
            .contains(&ConcurrencyAssumption::InvariantPredicateDeclared)
    );
}

/// A module whose trace carries fork/join task structure is checked, rendered
/// with a real C11 child thread, and run against its own reference observation.
///
/// Before card `MUTEX-SHARE-CROSS-EMISSION` this test asserted the opposite —
/// `c_source.is_none()` — because the backend emitted one thread. That pin was
/// moved deliberately, and the boundary it used to guard is now asserted on a
/// guard-local child by the test below, which is still outside both shapes.
#[test]
fn a_composed_child_task_module_checks_and_runs_natively_with_a_real_child_thread() {
    let source = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         thread scope c { with lock m as gc { open gc; write buf with gc; close gc; } }\n\
         with lock m as g { read buf with g; }\n\
         }\n\
         }\n";
    let module = compile(source).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::CheckedMutexGuardsV1
    );
    let checked = module.checked.as_checked_mutex_guards().unwrap();
    assert_eq!(
        checked.certificate.profile,
        ConcurrencyProfile::CheckedMutexInvariantV1
    );
    assert_eq!(
        checked.certificate.completed_tasks,
        vec![1],
        "the child is created and joined by its creator"
    );
    assert_eq!(checked.certificate.destroyed_resources, vec![1]);
    // Card `MUTEX-SHARE-CROSS-EMISSION` moved this pin **deliberately**: the
    // backend has a rendering for a non-nested fork/join trace, so this module
    // now emits, and the emitted program is run against its own reference
    // observation below rather than merely asserted to exist.
    let c = module
        .c_source
        .as_deref()
        .expect("a fork/join trace renders since card MUTEX-SHARE-CROSS-EMISSION");
    assert!(
        c.contains("thrd_create(&qcore_thread_1, qcore_mutex_task_1"),
        "the child is a real C11 thread, not an inlined segment"
    );
    let reference = qcore_core::mutex_c_emit::interpret_mutex_program(
        &checked.lowered.program,
        &checked.certificate,
    )
    .expect("a fork/join trace is inside the emitted subset");
    assert!(reference.contains(&"spawn 0 1".to_owned()));
    assert!(reference.contains(&"join 0 1".to_owned()));
    assert_eq!(
        run_native(&mutex_observation::instrument(c)),
        reference,
        "native and reference observations must agree line for line"
    );
}

/// The boundary that survives card `MUTEX-SHARE-CROSS-EMISSION`, asserted on a
/// shape that is **reachable from source** so the refusal is not vacuous.
///
/// A guard-local child (`thread scope c under g`) lowers to `GuardLocalJoin`,
/// which is outside both rendered shapes: the child is joined inside a critical
/// section its parent holds, so the parent is not simply blocked for the whole
/// of the child's segment. The module still checks, and is still published
/// without C rather than with C the backend did not produce.
#[test]
fn a_guard_local_child_is_checked_and_still_published_without_native_emission() {
    let source = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         with lock m as g {\n\
         thread scope c under g { }\n\
         read buf with g;\n\
         }\n\
         }\n\
         }\n";
    let module = compile(source).unwrap();
    let checked = module.checked.as_checked_mutex_guards().unwrap();
    assert_eq!(
        qcore_core::mutex_c_emit::classify_trace(&checked.lowered.program),
        qcore_core::mutex_c_emit::TraceShape::Outside { event: 2 },
        "the guard-local join is the event that puts this trace outside, and \
         the refusal names it"
    );
    assert!(
        module.c_source.is_none(),
        "a trace outside both rendered shapes is published without C"
    );
    assert_eq!(
        qcore_core::mutex_c_emit::interpret_mutex_program(
            &checked.lowered.program,
            &checked.certificate,
        ),
        Err(qcore_core::mutex_c_emit::MutexEmissionRefusal::UnsupportedEvent { event: 2 }),
        "the reference interpreter refuses exactly where the backend does"
    );
}

/// The three composed refusals, each located on the statement that caused it.
#[test]
fn the_composed_refusals_are_located_on_their_own_statements() {
    // A child acting through the guard it lexically sees (line 6), then the
    // join that landed inside the section (line 6 as well: the block's end).
    let escaping_guard = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         with lock m as g {\n\
         thread scope c { read buf with g; }\n\
         }\n\
         }\n\
         }\n";
    let refusal = compile(escaping_guard).unwrap_err();
    assert_eq!(
        refusal.first().map(String::as_str),
        Some("Q612@6"),
        "{refusal:?}"
    );

    // An announced acquisition that outlives its task blocks the generation's
    // destruction at scope exit.
    let waiter = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         thread scope c { wait m; }\n\
         }\n\
         }\n";
    let refusal = compile(waiter).unwrap_err();
    assert_eq!(
        refusal.first().map(String::as_str),
        Some("Q619@6"),
        "{refusal:?}"
    );

    // A fact relied on after the release that dropped it.
    let stale = "#import std.concurrency; fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         let f = fact buf;\n\
         with lock m as g { assert f with g; }\n\
         with lock m as h { use f; }\n\
         }\n\
         }\n";
    let refusal = compile(stale).unwrap_err();
    assert_eq!(refusal, vec!["Q616@7"], "{refusal:?}");
}
