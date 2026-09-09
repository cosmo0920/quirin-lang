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
        assert_eq!(compile_and_run(&emission.source), reference);
    }
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

fn compile_and_run(source: &str) -> Vec<String> {
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
        let observation = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        fs::remove_dir_all(directory).unwrap();
        observation
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
    assert_eq!(compile_and_run(&emission.source), reference);
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
