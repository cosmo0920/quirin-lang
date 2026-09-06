#[cfg(target_os = "linux")]
use qcore_compiler::SourceExecutionProfile;
use qcore_compiler::frontend::{elaborate_source, parse_source_program};
use qcore_compiler::{CompileOptions, SourceFile, compile_files};
#[cfg(target_os = "linux")]
use qcore_core::scoped_tasks::TaskOutcome;
#[cfg(target_os = "linux")]
use qcore_core::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::process::{Command, Output};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};

const COPY_TASK: &str = r#"
concurrency checked;
fn worker(x: i32) -> i32 { return x; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let x = 42;
    thread scope s {
        let h = spawn s worker(copy x)?;
        let r = join move h;
        let y = r?;
        return y;
    }
}
"#;

#[test]
fn resolved_spawn_and_join_require_the_checked_source_profile() {
    let source = "fn worker() -> unit { return; } fn main() -> unit { thread scope s { let h = spawn s worker()?; let r = join move h; } return; }";
    let parsed = parse_source_program(source).unwrap();
    assert_eq!(parsed.task_expressions().len(), 2);
    let typed = elaborate_source(&parsed).unwrap();
    let main = typed
        .declarations
        .iter()
        .find(|d| d.name == "main")
        .unwrap();
    let tasks = &main.statements[0].nested;
    assert!(
        tasks
            .iter()
            .all(|s| s.binding.is_some() && s.task_expressions.len() == 1)
    );
    assert_ne!(tasks[0].binding, tasks[1].binding);
    let errors = compile_files(
        &[SourceFile::new("tasks.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("concurrency checked")),
        "{errors:?}"
    );
    assert!(!errors.iter().any(|e| e.code == "Q013"), "{errors:?}");
}

#[cfg(target_os = "linux")]
static NEXT: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "linux")]
fn native(source: &str, header: Option<&str>) -> (qcore_compiler::CompiledModule, Output) {
    let module = compile_files(
        &[SourceFile::new("native-tasks.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ScopedCopyTasksV1
    );
    assert!(module.checked.as_sequential().is_none());
    assert!(module.reference_observation.is_none());
    assert!(!module.source_map.is_empty());
    let directory = std::env::temp_dir().join(format!(
        "qcore-native-tasks-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    let c = directory.join("module.c");
    let exe = directory.join("module");
    fs::write(&c, module.c_source.as_ref().unwrap()).unwrap();
    let mut compiler =
        Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"]);
    if let Some(header) = header {
        let path = directory.join("fault.h");
        fs::write(&path, header).unwrap();
        compiler.arg("-include").arg(path);
    }
    let output = compiler.arg(&c).arg("-o").arg(&exe).output().unwrap();
    assert!(
        output.status.success(),
        "{}\nartifact: {}",
        String::from_utf8_lossy(&output.stderr),
        c.display()
    );
    let output = Command::new(exe).output().unwrap();
    fs::remove_dir_all(directory).unwrap();
    (module, output)
}

#[cfg(target_os = "linux")]
fn matches_reference(
    module: &qcore_compiler::CompiledModule,
    output: &Output,
    decisions: Vec<bool>,
) -> TaskOutcome {
    let reference = execute_task_module(
        module.checked.as_scoped_tasks().unwrap(),
        &TaskExecutionScript {
            spawn_outcomes: decisions,
        },
    )
    .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout.clone()).unwrap(),
        render_task_observation(&reference)
    );
    reference.root_outcome
}

#[test]
#[cfg(target_os = "linux")]
fn actual_native_copy_worker_matches_reference_and_retains_scalar_extrema() {
    for value in ["42", "-2147483648", "2147483647"] {
        let source = COPY_TASK.replace("let x = 42", &format!("let x = {value}"));
        let (module, output) = native(&source, None);
        assert_eq!(
            matches_reference(&module, &output, vec![true]),
            TaskOutcome::I32(value.parse().unwrap())
        );
        assert!(module.c_source.as_ref().unwrap().contains("thrd_create"));
    }
}

#[test]
#[cfg(target_os = "linux")]
fn allocation_and_native_creation_failure_follow_the_checked_error_edge() {
    for (header, released) in [
        (
            r#"
#include <stdlib.h>
static void *fail_alloc(size_t n, size_t s) { (void)n; (void)s; return NULL; }
static void forbidden_free(void *p) { (void)p; abort(); }
#define QCORE_TASK_CALLOC fail_alloc
#define QCORE_TASK_FREE forbidden_free
"#,
            false,
        ),
        (
            r#"
#include <threads.h>
#include <stdio.h>
#include <stdlib.h>
static int fail_create(thrd_t *t, thrd_start_t f, void *p) { (void)t; (void)f; (void)p; return thrd_error; }
static void record_free(void *p) { fputs("FREE\n", stderr); free(p); }
#define QCORE_TASK_CREATE fail_create
#define QCORE_TASK_FREE record_free
"#,
            true,
        ),
    ] {
        let (module, output) = native(COPY_TASK, Some(header));
        assert_eq!(
            matches_reference(&module, &output, vec![false]),
            TaskOutcome::Error("SpawnError".into())
        );
        assert_eq!(
            output.stderr,
            if released {
                b"FREE\n".as_slice()
            } else {
                b"".as_slice()
            }
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("QCORE_TASK_SPAWN"));
    }
}

#[test]
#[cfg(target_os = "linux")]
fn unexpected_native_join_failure_aborts_before_environment_reclamation() {
    let header = r#"
#include <threads.h>
#include <signal.h>
#include <stdlib.h>
static void observed_abort(int signal_number) { (void)signal_number; _Exit(77); }
static int fail_join(thrd_t t, int *result) { (void)t; (void)result; signal(SIGABRT, observed_abort); return thrd_error; }
static void forbidden_free(void *p) { (void)p; _Exit(91); }
#define QCORE_TASK_JOIN fail_join
#define QCORE_TASK_FREE forbidden_free
"#;
    let (_, output) = native(COPY_TASK, Some(header));
    assert_eq!(output.status.code(), Some(77), "{output:?}");
}

#[test]
#[cfg(target_os = "linux")]
fn empty_scopes_and_uncalled_workers_emit_complete_warning_clean_c() {
    let source = "concurrency checked; fn unused(x: i32) -> i32 { return x; } fn main() -> unit { thread scope s { } return; }";
    let (module, output) = native(source, None);
    assert_eq!(module.checked.function_count(), 2);
    assert_eq!(
        matches_reference(&module, &output, vec![]),
        TaskOutcome::Unit
    );
}

#[test]
fn dynamic_creation_failure_cannot_strand_a_previous_sibling() {
    let source = COPY_TASK.replace("let r = join move h;", "let second = spawn s worker(copy x)?; let second_result = join move second?; let r = join move h;");
    let errors = compile_files(
        &[SourceFile::new("siblings.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(errors.iter().any(|e| e.code == "Q600"), "{errors:?}");
}

#[test]
#[cfg(target_os = "linux")]
fn worker_errors_and_sequential_copy_results_preserve_exact_outcomes() {
    let failure = r#"
concurrency checked;
error WorkerFailure { Broken; Other; }
fn worker() -> i32 effects { throws<WorkerFailure> } { raise WorkerFailure::Broken; }
fn main() -> i32 effects { throws<SpawnError>, throws<WorkerFailure>, spawn, join, may_block } {
    thread scope s {
        let h = spawn s worker()?;
        let collected = join move h;
        let value = collected?;
        return value;
    }
}
"#;
    let (module, output) = native(failure, None);
    assert_eq!(
        matches_reference(&module, &output, vec![true]),
        TaskOutcome::Error("WorkerFailure::Broken".into())
    );
    let sequential = COPY_TASK.replace(
        "return y;",
        "let next = spawn s worker(copy y)?; let z = join move next?; return z;",
    );
    let (module, output) = native(&sequential, None);
    assert_eq!(
        matches_reference(&module, &output, vec![true, true]),
        TaskOutcome::I32(42)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("QCORE_TASK_SPAWN 2 "));
    let (module, output) = native(
        &sequential,
        Some(
            r#"
#include <threads.h>
static int attempts;
static int second_create_fails(thrd_t *t, thrd_start_t f, void *p) {
    if (++attempts == 2) return thrd_error;
    return thrd_create(t, f, p);
}
#define QCORE_TASK_CREATE second_create_fails
"#,
        ),
    );
    assert_eq!(
        matches_reference(&module, &output, vec![true, false]),
        TaskOutcome::Error("SpawnError".into())
    );
}

#[test]
#[cfg(not(target_os = "linux"))]
fn unsupported_native_target_rejects_explicitly() {
    let errors = compile_files(
        &[SourceFile::new("tasks.qcore", COPY_TASK)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("Linux C11 threads target")),
        "{errors:?}"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn explicit_creation_recovery_joins_prior_sibling_on_each_dynamic_outcome() {
    let source = include_str!("fixtures/ct/CT13/case.qcore");
    let (module, output) = native(source, None);
    assert_eq!(
        matches_reference(&module, &output, vec![true, true]),
        TaskOutcome::I32(7)
    );
    let header = r#"
#include <threads.h>
static int count;
static int fail_second(thrd_t *thread, thrd_start_t worker, void *arg) {
    if (++count == 2) return thrd_error;
    return thrd_create(thread, worker, arg);
}
#define QCORE_TASK_CREATE fail_second
"#;
    let (module, output) = native(source, Some(header));
    assert_eq!(
        matches_reference(&module, &output, vec![true, false]),
        TaskOutcome::I32(42)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("QCORE_TASK_JOIN 1\n"));
    assert!(!text.contains("QCORE_TASK_SPAWN 2 "));
    let rethrow = source.replace("return first_result;", "raise failure;");
    let (module, output) = native(&rethrow, Some(header));
    assert_eq!(
        matches_reference(&module, &output, vec![true, false]),
        TaskOutcome::Error("SpawnError".into())
    );
}

#[test]
fn failed_creation_handler_cannot_drop_an_existing_sibling() {
    let errors = compile_files(
        &[SourceFile::new(
            "unjoined-recovery.qcore",
            include_str!("fixtures/ct/CT13/unjoined_failure.qcore"),
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
}

#[test]
#[cfg(target_os = "linux")]
fn result_handlers_collect_siblings_before_rethrowing_exact_worker_errors() {
    let direct = r#"
concurrency checked;
error WorkerFailure { First; Second; }
fn worker(x: i32) -> i32 { return x; }
fn failing() -> i32 effects { throws<WorkerFailure> } { raise WorkerFailure::Second; }
fn main() -> i32 effects { throws<SpawnError>, throws<WorkerFailure>, spawn, join, may_block } {
    let x = 42;
    thread scope s {
        let first = spawn s worker(copy x)?;
        match spawn s failing() {
            Result::Ok { value: second } => {
                match join move second {
                    Result::Ok { value: value } => {
                        let saved = join move first?;
                        return value;
                    }
                    Result::Err { error: worker_error } => {
                        let saved = join move first?;
                        raise worker_error;
                    }
                }
            }
            Result::Err { error: creation } => {
                let saved = join move first?;
                raise creation;
            }
        }
    }
}
"#;
    for source in [
        direct.to_owned(),
        direct.replace(
            "match join move second {",
            "let collected = join move second; match collected {",
        ),
    ] {
        let (module, output) = native(&source, None);
        assert_eq!(
            matches_reference(&module, &output, vec![true, true]),
            TaskOutcome::Error("WorkerFailure::Second".into())
        );
        let transcript = String::from_utf8(output.stdout).unwrap();
        assert!(transcript.contains("QCORE_TASK_JOIN 2\n"));
        assert!(transcript.contains("QCORE_TASK_JOIN 1\n"));
    }
}

#[test]
#[cfg(target_os = "linux")]
fn second_start_allocation_failure_preserves_the_prior_native_task() {
    let header = r#"
#include <stdlib.h>
static int allocations;
static void *fail_second_allocation(size_t n, size_t size) {
    if (++allocations == 2) return NULL;
    return calloc(n, size);
}
#define QCORE_TASK_CALLOC fail_second_allocation
"#;
    let (module, output) = native(include_str!("fixtures/ct/CT13/case.qcore"), Some(header));
    assert_eq!(
        matches_reference(&module, &output, vec![true, false]),
        TaskOutcome::I32(42)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("QCORE_TASK_JOIN 1\n"));
}
