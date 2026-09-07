//! End-to-end route selection for `source.mutex.guard.v1` (card T03).
//!
//! `compile_files` must reach the checked mutex profile from ordinary source,
//! publish a certificate it independently rechecked, and emit C11 that names
//! real `mtx_t` operations. Rejections must still arrive as located
//! diagnostics on the same route rather than falling through to another one.
use qcore_compiler::{
    CompileOptions, ConcurrencyAssumption, ConcurrencyProfile, SourceExecutionProfile, SourceFile,
    compile_files,
};

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

const GUARDED: &str = "fn main() -> unit {\n\
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
    let source = "fn main() -> unit {\n\
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
    assert_eq!(
        run_native(module.c_source.as_deref().unwrap()),
        reference,
        "native and reference observations must agree line for line"
    );
}

fn run_native(source: &str) -> Vec<String> {
    use std::process::Command;
    let directory = std::env::temp_dir().join(format!(
        "qcore-t03-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
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
        .env("ASAN_OPTIONS", "detect_leaks=1:halt_on_error=1")
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

#[test]
fn an_unguarded_write_to_a_protected_footprint_is_located_and_refused() {
    let source = "fn main() -> unit {\n\
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
    let source = "fn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         with lock m as g { open g; write buf with g; }\n\
         }\n\
         }\n";
    let errors = compile(source).unwrap_err();
    assert_eq!(errors.first().map(String::as_str), Some("Q613@5"));
}
