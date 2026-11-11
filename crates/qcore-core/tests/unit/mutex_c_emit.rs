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

//! Reference/native agreement for the checked mutex-guard backend (card T03).
//!
//! Each fixed scenario is compiled and run once under AddressSanitizer,
//! UndefinedBehaviorSanitizer and LeakSanitizer, and its observation compared
//! with the independently computed reference execution. This is a translation
//! check for the selected model and inputs on one Linux target; it establishes
//! nothing about contention, other targets or the invariant predicate.
#[cfg(target_os = "windows")]
#[path = "../../../../tests/support/windows_c.rs"]
mod windows_c;

#[cfg(not(target_os = "windows"))]
use std::fs;
#[cfg(not(target_os = "windows"))]
use std::process::Command;
#[cfg(not(target_os = "windows"))]
use std::sync::atomic::{AtomicU64, Ordering};

use crate::mutex_invariant::{
    AccessFootprint, AccessKind, InvariantWitness, MutexInvariantEvent, MutexInvariantProgram,
    ShareGrant, check_mutex_invariant_program,
};

#[cfg(not(target_os = "windows"))]
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn grant(resource: u32, end: u32) -> ShareGrant {
    ShareGrant {
        task: 0,
        footprint: AccessFootprint::new(resource, 0, end),
        share: crate::mutex_invariant::FULL_ACCESS,
    }
}

/// `open`/`write`/`close` inside one section, then scope exit.
fn open_write_close() -> MutexInvariantProgram {
    let footprint = AccessFootprint::new(1, 0, 4);
    MutexInvariantProgram {
        initial_shares: vec![grant(1, 4)],
        initial_custody: [(1, 0)].into_iter().collect(),
        events: vec![
            MutexInvariantEvent::CreateMutex {
                task: 0,
                mutex: 1,
                protects: vec![footprint],
                invariant: 1,
            },
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 1,
                guard: 1,
            },
            MutexInvariantEvent::OpenInvariant { task: 0, guard: 1 },
            MutexInvariantEvent::Access {
                task: 0,
                footprint,
                kind: AccessKind::Write,
                guard: Some(1),
            },
            MutexInvariantEvent::CloseInvariant {
                task: 0,
                guard: 1,
                witness: InvariantWitness::Declared { invariant: 1 },
            },
            MutexInvariantEvent::Access {
                task: 0,
                footprint,
                kind: AccessKind::Read,
                guard: Some(1),
            },
            MutexInvariantEvent::Unlock { task: 0, guard: 1 },
            MutexInvariantEvent::DestroyMutex { task: 0, mutex: 1 },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 1,
            },
        ],
        ..Default::default()
    }
}

/// Two sections over one generation, a guard-derived borrow released inside its
/// own section, and an unprotected resource beside the protected one.
fn two_sections_with_borrow() -> MutexInvariantProgram {
    let protected = AccessFootprint::new(1, 0, 4);
    let scratch = AccessFootprint::new(2, 0, 2);
    MutexInvariantProgram {
        initial_shares: vec![grant(1, 4), grant(2, 2)],
        initial_custody: [(1, 0), (2, 0)].into_iter().collect(),
        events: vec![
            MutexInvariantEvent::CreateMutex {
                task: 0,
                mutex: 1,
                protects: vec![protected],
                invariant: 1,
            },
            MutexInvariantEvent::Access {
                task: 0,
                footprint: scratch,
                kind: AccessKind::Write,
                guard: None,
            },
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 1,
                guard: 1,
            },
            MutexInvariantEvent::OpenInvariant { task: 0, guard: 1 },
            MutexInvariantEvent::Access {
                task: 0,
                footprint: protected,
                kind: AccessKind::Write,
                guard: Some(1),
            },
            MutexInvariantEvent::CloseInvariant {
                task: 0,
                guard: 1,
                witness: InvariantWitness::Declared { invariant: 1 },
            },
            MutexInvariantEvent::Unlock { task: 0, guard: 1 },
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 1,
                guard: 2,
            },
            MutexInvariantEvent::BorrowUnderGuard {
                task: 0,
                guard: 2,
                footprint: protected,
                borrow: 1,
            },
            MutexInvariantEvent::Access {
                task: 0,
                footprint: protected,
                kind: AccessKind::Read,
                guard: Some(2),
            },
            MutexInvariantEvent::EndBorrow { task: 0, borrow: 1 },
            MutexInvariantEvent::Unlock { task: 0, guard: 2 },
            MutexInvariantEvent::DestroyMutex { task: 0, mutex: 1 },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 1,
            },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 2,
            },
        ],
        ..Default::default()
    }
}

/// A typed-error exit that restores the invariant first. The release is still
/// emitted on that path, and the allocation is still freed.
fn error_exit_after_close() -> MutexInvariantProgram {
    let mut program = open_write_close();
    program.events[6] = MutexInvariantEvent::ErrorExit {
        task: 0,
        guard: 1,
        exit: 0,
    };
    program
}

#[test]
fn checked_mutex_traces_agree_between_the_reference_and_a_sanitized_native_run() {
    for program in [
        open_write_close(),
        two_sections_with_borrow(),
        error_exit_after_close(),
    ] {
        let certificate = check_mutex_invariant_program(&program).expect("fixture checks");
        let reference = crate::mutex_c_emit::interpret_mutex_program(&program, &certificate)
            .expect("fixture is inside the emitted subset");
        let emission =
            crate::mutex_c_emit::emit_mutex_c11(&program, &certificate).expect("emitted");
        assert!(emission.source.contains("mtx_init"));
        assert!(emission.source.contains("mtx_lock"));
        assert!(emission.source.contains("mtx_unlock"));
        assert!(emission.source.contains("mtx_destroy"));
        assert!(!reference.is_empty());
        compile_and_run(&emission.source);
    }
}

#[test]
fn the_emitted_program_routes_events_to_the_typed_diagnostic_logger() {
    let program = error_exit_after_close();
    let certificate = check_mutex_invariant_program(&program).unwrap();
    let emission = crate::mutex_c_emit::emit_mutex_c11(&program, &certificate).unwrap();
    assert!(
        emission
            .source
            .contains("qcore_diagnostic_log(QCORE_LOG_TRACE")
    );
    assert!(
        emission
            .source
            .contains("qcore_diagnostic_log(QCORE_LOG_DEBUG")
    );
    assert!(!emission.source.contains("QCORE_IO_WRITE("));
}

#[test]
fn the_emitted_release_survives_the_typed_error_path() {
    let program = error_exit_after_close();
    let certificate = check_mutex_invariant_program(&program).unwrap();
    let emission = crate::mutex_c_emit::emit_mutex_c11(&program, &certificate).unwrap();
    let unlocks = emission.source.matches("mtx_unlock(&m").count();
    assert_eq!(unlocks, 1, "the error exit releases exactly once");
    let free_index = emission.source.find("free(r1)").expect("resource is freed");
    let unlock_index = emission.source.find("mtx_unlock(&m").unwrap();
    assert!(
        unlock_index < free_index,
        "the section is released before its protected allocation is reclaimed"
    );
}

#[test]
fn emission_refuses_anything_outside_the_single_task_subset() {
    use crate::mutex_c_emit::{MutexEmissionRefusal, emit_mutex_c11, interpret_mutex_program};
    use crate::mutex_invariant::{
        ConcurrencyProfile, MutexInvariantCertificate, SyncWitness, UnsupportedSynchronization,
    };

    let base = open_write_close();
    let certificate = check_mutex_invariant_program(&base).unwrap();
    for event in [
        MutexInvariantEvent::Spawn {
            parent: 0,
            child: 1,
        },
        MutexInvariantEvent::TransferCustody {
            from: 0,
            to: 1,
            resource: 1,
            via: SyncWitness::SpawnEdge { child: 1 },
        },
        MutexInvariantEvent::RegisterWaiter { task: 0, mutex: 1 },
        MutexInvariantEvent::UnsupportedConstruct {
            task: 0,
            construct: UnsupportedSynchronization::ConditionVariable,
        },
    ] {
        let mut program = base.clone();
        program.events.insert(1, event);
        assert_eq!(
            emit_mutex_c11(&program, &certificate),
            Err(MutexEmissionRefusal::UnsupportedEvent { event: 1 })
        );
        assert_eq!(
            interpret_mutex_program(&program, &certificate),
            Err(MutexEmissionRefusal::UnsupportedEvent { event: 1 })
        );
    }

    // A certificate from another profile never authorizes emission.
    let foreign = MutexInvariantCertificate {
        profile: ConcurrencyProfile::TrustedExplicitTrace,
        ..certificate
    };
    assert_eq!(
        emit_mutex_c11(&base, &foreign),
        Err(MutexEmissionRefusal::UncheckedProfile)
    );
    assert_eq!(
        interpret_mutex_program(&base, &foreign),
        Err(MutexEmissionRefusal::UncheckedProfile)
    );
}

fn compile_and_run(source: &str) {
    #[cfg(target_os = "windows")]
    {
        let output = windows_c::run(source, None, &[]);
        assert!(output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty(), "quiet emission wrote stdout");
        assert!(output.stderr.is_empty(), "quiet emission wrote stderr");
    }
    #[cfg(not(target_os = "windows"))]
    {
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "qcore-t03-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let source_path = directory.join("generated.c");
        let binary_path = directory.join("generated");
        fs::write(&source_path, source).unwrap();

        let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
        let compile = Command::new(&compiler)
            .current_dir(&directory)
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
                // links real C11 threads.
                "-pthread",
            ])
            .arg(&source_path)
            .arg("-o")
            .arg(&binary_path)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "C compiler rejected the emitted mutex program:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&binary_path)
            // LeakSanitizer is part of AddressSanitizer here and is left enabled:
            // reclamation of every emitted allocation is exactly what the Linux
            // run has to witness. Apple Clang reports leak detection unsupported,
            // so macOS still runs ASan/UBSan with leak detection disabled.
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
            "emitted mutex program failed under ASan/UBSan/LSan:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(run.stdout.is_empty(), "quiet emission wrote stdout");
        assert!(run.stderr.is_empty(), "quiet emission wrote stderr");
        fs::remove_dir_all(directory).unwrap();
    }
}

// ---------------------------------------------------------------------------
// Card T03-F.
// ---------------------------------------------------------------------------

/// One section that only reads, closing with the **checked** witness, and one
/// guard-scoped fact established and relied on inside it. Both the fact events
/// and the checked witness are static bookkeeping: they are traced so the two
/// observations stay comparable, exactly as the invariant phases are.
fn checked_witness_with_a_fact() -> MutexInvariantProgram {
    let footprint = AccessFootprint::new(1, 0, 4);
    MutexInvariantProgram {
        initial_shares: vec![grant(1, 4)],
        initial_custody: [(1, 0)].into_iter().collect(),
        events: vec![
            MutexInvariantEvent::CreateMutex {
                task: 0,
                mutex: 1,
                protects: vec![footprint],
                invariant: 1,
            },
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 1,
                guard: 1,
            },
            MutexInvariantEvent::AssertFact {
                task: 0,
                guard: 1,
                footprint,
                fact: 1,
            },
            MutexInvariantEvent::UseFact { task: 0, fact: 1 },
            MutexInvariantEvent::OpenInvariant { task: 0, guard: 1 },
            MutexInvariantEvent::Access {
                task: 0,
                footprint,
                kind: AccessKind::Read,
                guard: Some(1),
            },
            MutexInvariantEvent::CloseInvariant {
                task: 0,
                guard: 1,
                witness: InvariantWitness::NoWriteOccurred,
            },
            MutexInvariantEvent::Unlock { task: 0, guard: 1 },
            MutexInvariantEvent::DestroyMutex { task: 0, mutex: 1 },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 1,
            },
        ],
        ..Default::default()
    }
}

#[test]
fn guard_scoped_facts_and_the_checked_witness_run_natively_and_match_the_reference() {
    use crate::mutex_c_emit::{emit_mutex_c11, interpret_mutex_program};

    let program = checked_witness_with_a_fact();
    let certificate = check_mutex_invariant_program(&program).unwrap();
    assert!(
        !certificate
            .assumptions
            .contains(&crate::mutex_invariant::ConcurrencyAssumption::InvariantPredicateDeclared),
        "the checked witness adds no declared premise"
    );
    let reference = interpret_mutex_program(&program, &certificate).unwrap();
    assert!(reference.contains(&"assert 1".to_owned()));
    assert!(reference.contains(&"use-fact 1".to_owned()));
    let emission = emit_mutex_c11(&program, &certificate).unwrap();
    assert!(!reference.is_empty());
    compile_and_run(&emission.source);
}

#[test]
fn a_trace_with_task_structure_is_outside_this_backend_and_says_so_before_emitting() {
    use crate::mutex_c_emit::{MutexEmissionRefusal, emit_mutex_c11, is_single_task_trace};
    use crate::mutex_invariant::{SharedResourceId, SyncWitness};

    let base = open_write_close();
    assert!(is_single_task_trace(&base));
    assert!(is_single_task_trace(&checked_witness_with_a_fact()));

    let certificate = check_mutex_invariant_program(&base).unwrap();
    for event in [
        MutexInvariantEvent::Spawn {
            parent: 0,
            child: 1,
        },
        MutexInvariantEvent::Join {
            parent: 0,
            child: 1,
        },
        MutexInvariantEvent::RegisterWaiter { task: 0, mutex: 1 },
        MutexInvariantEvent::TransferCustody {
            from: 0,
            to: 1,
            resource: 1 as SharedResourceId,
            via: SyncWitness::SpawnEdge { child: 1 },
        },
    ] {
        let mut program = base.clone();
        program.events.insert(1, event);
        assert!(
            !is_single_task_trace(&program),
            "task structure is not a single-task trace"
        );
        // And the backend still refuses it if it is handed one anyway: the
        // classification is a caller's guard, not a replacement for the gate.
        assert_eq!(
            emit_mutex_c11(&program, &certificate),
            Err(MutexEmissionRefusal::UnsupportedEvent { event: 1 })
        );
    }
}

/// Card CT33-RANKS. Two ranked generations over disjoint footprints, nested in
/// declared order, with a write under each guard. Nesting is what the rank
/// discipline makes expressible, so the backend has to render two live guards;
/// this is that rendering, run once under ASan/UBSan/LSan and compared with the
/// independently computed reference execution.
fn ranked_nested_sections() -> MutexInvariantProgram {
    let outer = AccessFootprint::new(1, 0, 4);
    let inner = AccessFootprint::new(2, 0, 4);
    MutexInvariantProgram {
        initial_shares: vec![grant(1, 4), grant(2, 4)],
        initial_custody: [(1, 0), (2, 0)].into_iter().collect(),
        lock_ranks: std::collections::BTreeMap::from([(1, 1), (2, 2)]),
        events: vec![
            MutexInvariantEvent::CreateMutex {
                task: 0,
                mutex: 1,
                protects: vec![outer],
                invariant: 1,
            },
            MutexInvariantEvent::CreateMutex {
                task: 0,
                mutex: 2,
                protects: vec![inner],
                invariant: 2,
            },
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 1,
                guard: 1,
            },
            MutexInvariantEvent::OpenInvariant { task: 0, guard: 1 },
            MutexInvariantEvent::Access {
                task: 0,
                footprint: outer,
                kind: AccessKind::Write,
                guard: Some(1),
            },
            // The nested acquisition, in strictly increasing rank order.
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 2,
                guard: 2,
            },
            MutexInvariantEvent::OpenInvariant { task: 0, guard: 2 },
            MutexInvariantEvent::Access {
                task: 0,
                footprint: inner,
                kind: AccessKind::Write,
                guard: Some(2),
            },
            MutexInvariantEvent::CloseInvariant {
                task: 0,
                guard: 2,
                witness: InvariantWitness::Declared { invariant: 2 },
            },
            MutexInvariantEvent::Unlock { task: 0, guard: 2 },
            MutexInvariantEvent::CloseInvariant {
                task: 0,
                guard: 1,
                witness: InvariantWitness::Declared { invariant: 1 },
            },
            MutexInvariantEvent::Unlock { task: 0, guard: 1 },
            MutexInvariantEvent::DestroyMutex { task: 0, mutex: 2 },
            MutexInvariantEvent::DestroyMutex { task: 0, mutex: 1 },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 2,
            },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 1,
            },
        ],
    }
}

#[test]
fn ranked_nested_sections_agree_between_the_reference_and_a_sanitized_native_run() {
    let program = ranked_nested_sections();
    let certificate = check_mutex_invariant_program(&program).expect("the declared order is kept");
    assert_eq!(
        certificate.declared_lock_ranks,
        std::collections::BTreeMap::from([(1, 1), (2, 2)])
    );
    let reference = crate::mutex_c_emit::interpret_mutex_program(&program, &certificate)
        .expect("a single-task nesting is inside the emitted subset");
    let emission = crate::mutex_c_emit::emit_mutex_c11(&program, &certificate).expect("emitted");
    // Two generations, and the inner section is released before the outer one.
    assert_eq!(
        emission.source.matches("mtx_init(&m").count(),
        2,
        "two generations, not one"
    );
    let inner_unlock = emission
        .source
        .find("mtx_unlock(&m2")
        .expect("the inner section is released");
    let outer_unlock = emission
        .source
        .find("mtx_unlock(&m1")
        .expect("the outer section is released");
    assert!(
        inner_unlock < outer_unlock,
        "the nesting unwinds in reverse acquisition order"
    );
    assert!(!reference.is_empty());
    compile_and_run(&emission.source);
}

/// The backend renders the trace it is given. Reversing the nesting is refused
/// by the checker before emission is ever reached, so there is no ranked
/// program the backend can render out of order.
#[test]
fn the_reversed_nesting_never_reaches_the_backend() {
    let mut program = ranked_nested_sections();
    program.lock_ranks = std::collections::BTreeMap::from([(1, 2), (2, 1)]);
    let diagnostics = check_mutex_invariant_program(&program).expect_err("refused");
    assert!(
        diagnostics.iter().any(|d| d.code() == "Q624"),
        "{diagnostics:?}"
    );
}

/// Card `CT34-SANITIZED`. One generation that grows its declared shape onto a
/// second allocation mid-section, with a real access to the originally declared
/// region *before* the growth and a real access to the newly protected region
/// *after* it, then a second section that reads both under one guard.
///
/// This is the shape the native evidence needs: growth performs no `mtx_t`
/// operation at all, so what a sanitized run can show is that the accesses the
/// checker admitted on the strength of the growth are real, in-bounds accesses
/// to real allocations inside the growing section's own lock/unlock pair, and
/// that both renderings agree line for line. It shows nothing about the C mutex
/// protecting more memory, which no `mtx_t` can express.
fn grown_generation() -> MutexInvariantProgram {
    let declared = AccessFootprint::new(1, 0, 4);
    let grown = AccessFootprint::new(2, 0, 4);
    MutexInvariantProgram {
        initial_shares: vec![grant(1, 4), grant(2, 4)],
        initial_custody: [(1, 0), (2, 0)].into_iter().collect(),
        events: vec![
            MutexInvariantEvent::CreateMutex {
                task: 0,
                mutex: 1,
                protects: vec![declared],
                invariant: 1,
            },
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 1,
                guard: 1,
            },
            MutexInvariantEvent::OpenInvariant { task: 0, guard: 1 },
            // Pre-growth: the originally declared region, under this guard.
            MutexInvariantEvent::Access {
                task: 0,
                footprint: declared,
                kind: AccessKind::Write,
                guard: Some(1),
            },
            MutexInvariantEvent::GrowProtection {
                task: 0,
                guard: 1,
                footprint: grown,
                invariant: 2,
            },
            // Post-growth: the newly protected region, under the same guard.
            MutexInvariantEvent::Access {
                task: 0,
                footprint: grown,
                kind: AccessKind::Write,
                guard: Some(1),
            },
            MutexInvariantEvent::CloseInvariant {
                task: 0,
                guard: 1,
                // The declared identity the growth installed, not the one the
                // generation was created with.
                witness: InvariantWitness::Declared { invariant: 2 },
            },
            MutexInvariantEvent::Unlock { task: 0, guard: 1 },
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 1,
                guard: 2,
            },
            MutexInvariantEvent::Access {
                task: 0,
                footprint: declared,
                kind: AccessKind::Read,
                guard: Some(2),
            },
            MutexInvariantEvent::Access {
                task: 0,
                footprint: grown,
                kind: AccessKind::Read,
                guard: Some(2),
            },
            MutexInvariantEvent::Unlock { task: 0, guard: 2 },
            MutexInvariantEvent::DestroyMutex { task: 0, mutex: 1 },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 1,
            },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 2,
            },
        ],
        ..Default::default()
    }
}

#[test]
fn a_grown_generation_agrees_between_the_reference_and_a_sanitized_native_run() {
    let program = grown_generation();
    let certificate = check_mutex_invariant_program(&program).expect("the growth is admitted");
    assert_eq!(
        certificate.admitted_protection_growths.len(),
        1,
        "the fixture must actually grow, or the run witnesses nothing"
    );
    assert_eq!(
        certificate.protected_shape[&1],
        vec![AccessFootprint::new(1, 0, 4), AccessFootprint::new(2, 0, 4)],
        "the effective shape carries the grown interval"
    );
    let reference = crate::mutex_c_emit::interpret_mutex_program(&program, &certificate)
        .expect("an admitted growth is inside the emitted subset");
    assert!(
        reference.contains(&"grow 1 1 2 0 4".to_owned()),
        "the growth contributes exactly its bookkeeping line: {reference:?}"
    );
    let emission = crate::mutex_c_emit::emit_mutex_c11(&program, &certificate).expect("emitted");
    // The growth performs no mutex operation. Two sections over one generation
    // is exactly six `mtx_` calls; a growth that had quietly acquired, released
    // or re-initialised anything would move one of these counts.
    assert_eq!(emission.source.matches("mtx_init(&m").count(), 1);
    assert_eq!(emission.source.matches("mtx_lock(&m").count(), 2);
    assert_eq!(emission.source.matches("mtx_unlock(&m").count(), 2);
    assert_eq!(emission.source.matches("mtx_destroy(&m").count(), 1);
    // Both allocations are reclaimed, which is what LeakSanitizer then checks.
    assert!(emission.source.contains("free(r1)") && emission.source.contains("free(r2)"));
    assert!(!reference.is_empty());
    compile_and_run(&emission.source);
}

/// The load-bearing half of card CT34-GROWTH's boundary `CT34-GROWTH-NATIVE`,
/// in its new form. The backend no longer refuses every growth — it renders the
/// ones a certificate admits — so what must stay refused is a growth the
/// supplied certificate does *not* admit. A hand-built trace, an edited event
/// and an edited certificate all reach the same refusal in both renderings:
/// there is no path from an unchecked growth to emitted C.
#[test]
fn an_unadmitted_protection_growth_is_refused_by_both_renderings() {
    use crate::mutex_c_emit::{MutexEmissionRefusal, emit_mutex_c11, interpret_mutex_program};

    // 1. A growth spliced into a trace whose certificate never saw one.
    let base = open_write_close();
    let base_certificate = check_mutex_invariant_program(&base).unwrap();
    let mut hand_built = base.clone();
    hand_built.events.insert(
        3,
        MutexInvariantEvent::GrowProtection {
            task: 0,
            guard: 1,
            footprint: AccessFootprint::new(1, 0, 4),
            invariant: 99,
        },
    );
    assert_eq!(
        emit_mutex_c11(&hand_built, &base_certificate),
        Err(MutexEmissionRefusal::UnadmittedProtectionGrowth { event: 3 })
    );
    assert_eq!(
        interpret_mutex_program(&hand_built, &base_certificate),
        Err(MutexEmissionRefusal::UnadmittedProtectionGrowth { event: 3 })
    );

    // 2. An admitted growth whose event was edited after the certificate was
    // issued. Each edit changes exactly one field.
    let program = grown_generation();
    let certificate = check_mutex_invariant_program(&program).unwrap();
    for edited in [
        MutexInvariantEvent::GrowProtection {
            task: 0,
            guard: 1,
            // a narrower interval than the one admitted
            footprint: AccessFootprint::new(2, 0, 2),
            invariant: 2,
        },
        MutexInvariantEvent::GrowProtection {
            task: 0,
            // a guard this section never minted
            guard: 7,
            footprint: AccessFootprint::new(2, 0, 4),
            invariant: 2,
        },
        MutexInvariantEvent::GrowProtection {
            // a task that is not the admitted grower
            task: 3,
            guard: 1,
            footprint: AccessFootprint::new(2, 0, 4),
            invariant: 2,
        },
        MutexInvariantEvent::GrowProtection {
            task: 0,
            guard: 1,
            footprint: AccessFootprint::new(2, 0, 4),
            // a predicate identity the certificate did not record
            invariant: 3,
        },
    ] {
        let mut mutated = program.clone();
        mutated.events[4] = edited;
        assert_eq!(
            emit_mutex_c11(&mutated, &certificate),
            Err(MutexEmissionRefusal::UnadmittedProtectionGrowth { event: 4 })
        );
        assert_eq!(
            interpret_mutex_program(&mutated, &certificate),
            Err(MutexEmissionRefusal::UnadmittedProtectionGrowth { event: 4 })
        );
    }

    // 3. Certificates edited instead of events: an emptied record, a record
    // moved to another event index, and a shape that hides the growth the
    // record admits. The last is a certificate disagreeing with itself.
    let emptied = crate::mutex_invariant::MutexInvariantCertificate {
        admitted_protection_growths: Vec::new(),
        ..certificate.clone()
    };
    let moved = {
        let mut growths = certificate.admitted_protection_growths.clone();
        growths[0].event = 5;
        crate::mutex_invariant::MutexInvariantCertificate {
            admitted_protection_growths: growths,
            ..certificate.clone()
        }
    };
    let hidden = {
        let mut shape = certificate.protected_shape.clone();
        shape.insert(1, vec![AccessFootprint::new(1, 0, 4)]);
        crate::mutex_invariant::MutexInvariantCertificate {
            protected_shape: shape,
            ..certificate.clone()
        }
    };
    for forged in [emptied, moved, hidden] {
        assert_eq!(
            emit_mutex_c11(&program, &forged),
            Err(MutexEmissionRefusal::UnadmittedProtectionGrowth { event: 4 })
        );
        assert_eq!(
            interpret_mutex_program(&program, &forged),
            Err(MutexEmissionRefusal::UnadmittedProtectionGrowth { event: 4 })
        );
    }

    // 4. The certificate the checker really issued still renders.
    assert!(emit_mutex_c11(&program, &certificate).is_ok());
    assert!(interpret_mutex_program(&program, &certificate).is_ok());
}

// ---------------------------------------------------------------------------
// Card `MUTEX-SHARE-CROSS-EMISSION`: the fork/join rendering.
//
// The classification is the whole of the admission decision, so every one of
// its refusing arms is driven here on a hand-built trace rather than left to
// whatever shape a source fixture happens to produce. Each arm was shown
// non-vacuous by mutation: the arm's own condition was inverted and the
// corresponding row went red.

/// CT46's shape, built by hand: a creator, one child that takes the generation
/// itself, and the creator's own section after the join.
fn fork_join_trace() -> MutexInvariantProgram {
    let footprint = AccessFootprint::new(1, 0, 4);
    MutexInvariantProgram {
        initial_shares: vec![grant(1, 4)],
        initial_custody: [(1, 0)].into_iter().collect(),
        events: vec![
            MutexInvariantEvent::CreateMutex {
                task: 0,
                mutex: 1,
                protects: vec![footprint],
                invariant: 1,
            },
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            MutexInvariantEvent::Lock {
                task: 1,
                mutex: 1,
                guard: 1,
            },
            MutexInvariantEvent::OpenInvariant { task: 1, guard: 1 },
            MutexInvariantEvent::Access {
                task: 1,
                footprint,
                kind: AccessKind::Write,
                guard: Some(1),
            },
            MutexInvariantEvent::CloseInvariant {
                task: 1,
                guard: 1,
                witness: InvariantWitness::Declared { invariant: 1 },
            },
            MutexInvariantEvent::Unlock { task: 1, guard: 1 },
            MutexInvariantEvent::Join {
                parent: 0,
                child: 1,
            },
            MutexInvariantEvent::Lock {
                task: 0,
                mutex: 1,
                guard: 2,
            },
            MutexInvariantEvent::Access {
                task: 0,
                footprint,
                kind: AccessKind::Read,
                guard: Some(2),
            },
            MutexInvariantEvent::Unlock { task: 0, guard: 2 },
            MutexInvariantEvent::DestroyMutex { task: 0, mutex: 1 },
            MutexInvariantEvent::Destroy {
                task: 0,
                resource: 1,
            },
        ],
        ..Default::default()
    }
}

#[test]
fn a_fork_join_trace_renders_a_real_child_thread_and_agrees_with_the_reference() {
    use crate::mutex_c_emit::{
        ChildSegment, TraceShape, classify_trace, emit_mutex_c11, interpret_mutex_program,
    };

    let program = fork_join_trace();
    assert_eq!(
        classify_trace(&program),
        TraceShape::ForkJoin {
            children: vec![ChildSegment {
                parent: 0,
                child: 1,
                spawn: 1,
                join: 7,
            }]
        }
    );
    let certificate = check_mutex_invariant_program(&program).unwrap();
    let reference = interpret_mutex_program(&program, &certificate).unwrap();
    // The child's acquisition and release both lie strictly inside its own
    // segment: that is what makes this a crossing rather than a relabelling.
    let at = |needle: &str| reference.iter().position(|line| line == needle).unwrap();
    let (spawn, join) = (at("spawn 0 1"), at("join 0 1"));
    assert!(spawn < at("lock 1 1"));
    assert!(at("unlock 1") < join);
    assert!(join < at("lock 1 2"));

    let emission = emit_mutex_c11(&program, &certificate).unwrap();
    let source = &emission.source;
    assert!(source.contains("static int qcore_mutex_task_1(void *qcore_task_arg)"));
    assert!(source.contains("thrd_create(&qcore_thread_1, qcore_mutex_task_1"));
    assert!(source.contains("thrd_join(qcore_thread_1"));
    // The generation and the protected allocation moved to file scope so the
    // child can name them; both are still initialised in the creator.
    assert!(source.contains("static mtx_t m1;"));
    assert!(source.contains("static unsigned char *r1;"));
    // The child's critical section is not in the creator's body.
    let creator = source.split_once("int main(").unwrap().1;
    assert!(!creator.contains("QCORE_DIAG_INVARIANT_OPEN"));
    compile_and_run(source);
}

#[test]
fn two_children_in_sequence_each_get_their_own_function_and_segment() {
    use crate::mutex_c_emit::{ChildSegment, TraceShape, classify_trace, emit_mutex_c11};

    let mut program = fork_join_trace();
    let second: Vec<MutexInvariantEvent> = vec![
        MutexInvariantEvent::Spawn {
            parent: 0,
            child: 2,
        },
        MutexInvariantEvent::Lock {
            task: 2,
            mutex: 1,
            guard: 3,
        },
        MutexInvariantEvent::Unlock { task: 2, guard: 3 },
        MutexInvariantEvent::Join {
            parent: 0,
            child: 2,
        },
    ];
    // Inserted after the first child's join, so the two segments are siblings
    // and not nested.
    for (offset, event) in second.into_iter().enumerate() {
        program.events.insert(8 + offset, event);
    }
    assert_eq!(
        classify_trace(&program),
        TraceShape::ForkJoin {
            children: vec![
                ChildSegment {
                    parent: 0,
                    child: 1,
                    spawn: 1,
                    join: 7,
                },
                ChildSegment {
                    parent: 0,
                    child: 2,
                    spawn: 8,
                    join: 11,
                },
            ]
        }
    );
    let certificate = check_mutex_invariant_program(&fork_join_trace()).unwrap();
    let source = emit_mutex_c11(&program, &certificate).unwrap().source;
    for child in [1, 2] {
        assert!(source.contains(&format!(
            "static int qcore_mutex_task_{child}(void *qcore_task_arg)"
        )));
        assert!(source.contains(&format!("thrd_join(qcore_thread_{child}")));
    }
}

#[test]
fn every_way_out_of_the_fork_join_shape_is_refused_at_its_own_event() {
    use crate::mutex_c_emit::{
        MutexEmissionRefusal, TraceShape, classify_trace, emit_mutex_c11, interpret_mutex_program,
    };
    use crate::mutex_invariant::{SharedResourceId, SyncWitness};

    let base = fork_join_trace();
    let certificate = check_mutex_invariant_program(&base).unwrap();

    // Each case: a description, the mutated trace, and the event index the
    // refusal must name. A malformed *segment* is reported at its `Spawn`,
    // which is the event whose rendering cannot be constructed; an event that
    // is outside on its own is reported where it stands.
    let mut cases: Vec<(&str, MutexInvariantProgram, usize)> = Vec::new();

    let mut nested = base.clone();
    nested.events.insert(
        3,
        MutexInvariantEvent::Spawn {
            parent: 1,
            child: 2,
        },
    );
    cases.push(("a nested spawn inside an open segment", nested, 1));

    // Two ways a spawn goes unjoined, and they reach *different* arms. Here the
    // creator resumes its own work where the join should have been, so the
    // segment-contiguity arm is what refuses.
    let mut unmatched = base.clone();
    unmatched.events.remove(7);
    cases.push((
        "a spawn whose creator resumes instead of joining",
        unmatched,
        1,
    ));

    // And here the trace simply ends inside the open segment, which is the only
    // way to reach the end-of-events arm: nothing follows to be out of place.
    let mut runs_off_the_end = base.clone();
    runs_off_the_end.events.truncate(7);
    cases.push((
        "a spawn whose segment runs off the end of the trace",
        runs_off_the_end,
        1,
    ));

    let mut interleaved = base.clone();
    interleaved.events.insert(
        3,
        MutexInvariantEvent::AssertFact {
            task: 0,
            guard: 1,
            footprint: AccessFootprint::new(1, 0, 4),
            fact: 1,
        },
    );
    cases.push((
        "the creator's own event between the spawn and the join",
        interleaved,
        1,
    ));

    let mut error_exit = base.clone();
    error_exit.events.insert(
        6,
        MutexInvariantEvent::ErrorExit {
            task: 1,
            guard: 1,
            exit: 3,
        },
    );
    cases.push((
        "a typed-error exit inside a child, whose status the model gives no \
         meaning at the join",
        error_exit,
        1,
    ));

    let mut guard_local = base.clone();
    guard_local.events[7] = MutexInvariantEvent::GuardLocalJoin {
        parent: 0,
        child: 1,
        guard: 1,
    };
    cases.push(("a guard-local join in place of the join", guard_local, 1));

    let mut custody = base.clone();
    custody.events.insert(
        3,
        MutexInvariantEvent::TransferCustody {
            from: 0,
            to: 1,
            resource: 1 as SharedResourceId,
            via: SyncWitness::SpawnEdge { child: 1 },
        },
    );
    cases.push(("a custody transfer inside a segment", custody, 1));

    let mut waiter = base.clone();
    waiter
        .events
        .insert(3, MutexInvariantEvent::RegisterWaiter { task: 1, mutex: 1 });
    cases.push(("a registered waiter inside a segment", waiter, 1));

    let mut wrong_child = base.clone();
    wrong_child.events[7] = MutexInvariantEvent::Join {
        parent: 0,
        child: 9,
    };
    cases.push((
        "a join naming a child this spawn did not create",
        wrong_child,
        1,
    ));

    // A join with no spawn at all, on an otherwise single-task trace — so the
    // join really is the first thing wrong, rather than the first of several.
    let mut stray_join = open_write_close();
    stray_join.events.insert(
        2,
        MutexInvariantEvent::Join {
            parent: 0,
            child: 1,
        },
    );
    cases.push(("a join with no spawn at all", stray_join, 2));

    let mut second_root = base.clone();
    second_root.events.insert(
        8,
        MutexInvariantEvent::AssertFact {
            task: 7,
            guard: 2,
            footprint: AccessFootprint::new(1, 0, 4),
            fact: 1,
        },
    );
    cases.push((
        "an event outside every segment belonging to a task that is not the root",
        second_root,
        8,
    ));

    for (description, program, event) in cases {
        assert_eq!(
            classify_trace(&program),
            TraceShape::Outside { event },
            "{description}: classification"
        );
        // The classification is a caller's guard, never a replacement for the
        // gate: both the backend and the reference interpreter refuse it too,
        // at the same index, so no route can render what neither admits.
        assert_eq!(
            emit_mutex_c11(&program, &certificate),
            Err(MutexEmissionRefusal::UnsupportedEvent { event }),
            "{description}: emission"
        );
        assert_eq!(
            interpret_mutex_program(&program, &certificate),
            Err(MutexEmissionRefusal::UnsupportedEvent { event }),
            "{description}: reference execution"
        );
    }
}

// ---------------------------------------------------------------------------
// Card `TASK-ROUTE-TSAN`: CT46's emitted crossing under ThreadSanitizer.

#[cfg(target_os = "linux")]
#[path = "../../../../tests/support/thread_sanitizer.rs"]
mod thread_sanitizer;

/// **The first thread-race evidence this backend has ever had.**
/// `MUTEX-SHARE-CROSS-EMISSION` recorded that no TSan row existed anywhere in
/// this repository; card `TASK-ROUTE-TSAN` found why — ThreadSanitizer ships
/// no `thrd_create` interceptor, so a thread this backend spawns is never
/// registered with it — and supplied the pthread binding that gets past it
/// without touching one emitted byte.
///
/// **What TSan adds here.** The emitted crossing really does run a second OS
/// thread that locks the mutex, opens the invariant, writes the protected
/// allocation, closes and unlocks, while the creator waits; and a race
/// detector watching both threads reports nothing. Before this row, the
/// child's write and the creator's post-join read were known to agree with the
/// reference execution and to be clean under ASan/UBSan/LSan, neither of which
/// says anything about synchronization.
///
/// **What it still cannot say**, and this is the sentence
/// `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION` already owns: the checked trace
/// *serialises* the child, so the creator's own critical section begins after
/// the join and the two never contend for the mutex. A clean TSan run over a
/// schedule with no contention is evidence about that schedule. It is
/// therefore **not** a claim that the emitted locking would keep two
/// contending threads race-free, and no such claim is made here. What it does
/// establish is that the crossing's real happens-before edges — spawn, the
/// child's lock/unlock pair, join — are edges TSan can see and is satisfied
/// by, which is exactly the part a byte comparison cannot reach.
#[cfg(target_os = "linux")]
#[test]
fn the_fork_join_crossing_runs_clean_under_thread_sanitizer() {
    use crate::mutex_c_emit::emit_mutex_c11;

    let program = fork_join_trace();
    let certificate = check_mutex_invariant_program(&program).unwrap();
    let emission = emit_mutex_c11(&program, &certificate).unwrap();
    // The crossing really is a second thread in the emitted C.
    assert!(emission.source.contains("thrd_create(&qcore_thread_1"));

    // Unbound, this backend's emission dies exactly as the task route's does:
    // the same missing-interceptor fault, so the diagnosis covers both
    // emitters rather than one.
    let unbound = thread_sanitizer::run("ct46-unbound", &emission.source, false).unwrap();
    assert!(
        !unbound.clean && unbound.is_unregistered_thread_fault(),
        "expected the TASK-ROUTE-TSAN-SEGV fault unbound:\n{}",
        unbound.stderr
    );

    let bound = thread_sanitizer::run("ct46-bound", &emission.source, true).unwrap();
    assert_eq!(
        bound.races, 0,
        "ThreadSanitizer reported a race in the emitted crossing:\n{}",
        bound.stderr
    );
    assert!(
        bound.clean,
        "the emitted crossing did not run clean (signal {:?}):\n{}",
        bound.signal, bound.stderr
    );
}
