//! Reference/native agreement for the checked mutex-guard backend (card T03).
//!
//! Each fixed scenario is compiled and run once under AddressSanitizer,
//! UndefinedBehaviorSanitizer and LeakSanitizer, and its observation compared
//! with the independently computed reference execution. This is a translation
//! check for the selected model and inputs on one Linux target; it establishes
//! nothing about contention, other targets or the invariant predicate.
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::mutex_invariant::{
    AccessFootprint, AccessKind, InvariantWitness, MutexInvariantEvent, MutexInvariantProgram,
    ShareGrant, check_mutex_invariant_program,
};

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
        // reclamation of every emitted allocation is exactly what this run has
        // to witness.
        .env("ASAN_OPTIONS", "detect_leaks=1:halt_on_error=1")
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
