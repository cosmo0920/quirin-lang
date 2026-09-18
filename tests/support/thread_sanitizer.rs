//! Card `TASK-ROUTE-TSAN`: run a program the checked task route emitted under
//! ThreadSanitizer.
//!
//! **Why this module exists at all.** The emitted task runtime creates threads
//! with C11 `thrd_create`, which is the right primitive for a route that must
//! also emit for MSVC. ThreadSanitizer's runtime ships an interceptor for
//! `pthread_create` and **none for `thrd_create`** — verified on the binary,
//! `nm` finding `__interceptor_pthread_create` and no `thrd_create`
//! counterpart. On glibc 2.34 and later `thrd_create` reaches `start_thread`
//! without passing through that interceptor, so the new thread is never
//! registered with TSan, `cur_thread()` is null in it, and the first
//! instrumented event in the worker faults reading offset `0x18` of that null
//! pointer — `__tsan_func_entry+19`, `mov 0x18(%rdi),%rax`. That is the whole
//! of `TASK-ROUTE-TSAN-SEGV`, and it is a property of the sanitizer runtime
//! rather than of anything QCore emits: a fifteen-line C file containing no
//! QCore code at all, which calls `thrd_create` on a body holding one
//! instrumented access, dies at the identical address and pc.
//!
//! **What this module does about it.** [`PTHREAD_BINDING`] replaces the two
//! C11 thread primitives the emitted programs call — `thrd_create` and
//! `thrd_join` — with pthread-backed equivalents, which TSan *does* intercept.
//! **No emitted byte changes**: the C the compiler produced is compiled
//! verbatim and the binding arrives through `-include`, the same mechanism the
//! existing allocation-tracking harnesses use for `QCORE_BUFFER_MALLOC`. One
//! binding covers both emitters, because they reach the same two names from
//! different directions — the task runtime through its own documented
//! `#ifndef QCORE_TASK_CREATE` hook, whose default *is* `thrd_create`, and the
//! mutex-invariant backend through a literal call.
//!
//! The replacement bodies are not invented here: they are the pthread-backed
//! `thrd_create`/`thrd_join` the shipped `c11_threads_compat.h` already
//! compiles on Apple hosts, so every task program in this repository is
//! already built this way on one supported platform.
//!
//! **What that costs, stated rather than elided.** The program TSan observes
//! differs from the published program in which symbol spawns the thread. It is
//! not a different concurrency structure: glibc implements `thrd_create` by
//! calling `pthread_create`'s implementation, so both reach the same
//! `start_thread` — visible in the faulting backtrace itself, where the
//! unbound `thrd_create` frame sits directly on `start_thread`. Every
//! happens-before edge TSan reasons about (thread creation, thread join) is
//! therefore the same edge in both builds. What TSan cannot speak for is the
//! `thrd_create` call itself and any Windows or Apple thread arm. Registered
//! as `TASK-ROUTE-TSAN-PTHREAD-BINDING`.
//!
//! Linux only, and gated as such. `run` is `#[cfg(target_os = "linux")]`
//! because the diagnosis, the binding and every measurement above are
//! x86-64-Linux/glibc observations; a caller on another platform gets no row
//! rather than a vacuous pass, and must say so in its own test.

#![allow(dead_code)]

/// Replace `thrd_create`/`thrd_join` with pthread-backed equivalents.
/// Prepended to the emitted C through `-include`, so the emitted translation
/// unit itself is unmodified; `<threads.h>` is pulled in *before* the macros
/// are defined, so the real declarations are never rewritten and only call
/// sites are affected.
pub const PTHREAD_BINDING: &str = r#"
#include <pthread.h>
#include <stdint.h>
#include <stdlib.h>
#include <threads.h>

/* The two casts below are the whole of the binding's representation
 * assumption, and it is checked rather than assumed. */
_Static_assert(sizeof(thrd_t) == sizeof(pthread_t),
    "the pthread binding requires thrd_t and pthread_t to be one representation");

struct qcore_tsan_start { int (*function)(void *); void *argument; };

static void *qcore_tsan_trampoline(void *opaque) {
    struct qcore_tsan_start start = *(struct qcore_tsan_start *)opaque;
    free(opaque);
    return (void *)(intptr_t)start.function(start.argument);
}

/* Signature-compatible with thrd_create, so no emitted call site changes. */
__attribute__((unused))
static int qcore_tsan_thread_create(thrd_t *thread, int (*function)(void *), void *argument) {
    struct qcore_tsan_start *start = malloc(sizeof(*start));
    pthread_t created;
    if (start == NULL) return thrd_error;
    start->function = function;
    start->argument = argument;
    if (pthread_create(&created, NULL, qcore_tsan_trampoline, start) != 0) {
        free(start);
        return thrd_error;
    }
    *thread = (thrd_t)created;
    return thrd_success;
}

__attribute__((unused))
static int qcore_tsan_thread_join(thrd_t thread, int *result) {
    void *value = NULL;
    if (pthread_join((pthread_t)thread, &value) != 0) return thrd_error;
    if (result != NULL) *result = (int)(intptr_t)value;
    return thrd_success;
}

/* Call sites only: the task runtime reaches these through its own
 * QCORE_TASK_CREATE/QCORE_TASK_JOIN hooks, whose defaults are these names, and
 * the mutex-invariant backend reaches them directly. */
#define thrd_create qcore_tsan_thread_create
#define thrd_join qcore_tsan_thread_join
"#;

/// One ThreadSanitizer run of one emitted program.
#[derive(Debug)]
pub struct Sanitized {
    pub stdout: String,
    pub stderr: String,
    /// `true` when the process exited 0 with no diagnostic on stderr.
    pub clean: bool,
    /// Number of `data race` reports ThreadSanitizer printed.
    pub races: usize,
    /// `Some(signal)` when the process died on a signal.
    pub signal: Option<i32>,
}

impl Sanitized {
    /// The `TASK-ROUTE-TSAN-SEGV` shape: the null-`ThreadState` fault.
    pub fn is_unregistered_thread_fault(&self) -> bool {
        self.stderr.contains("SEGV on unknown address 0x000000000018")
    }
}

/// Whether ThreadSanitizer binds `thrd_create`. The whole reason this module
/// exists is that it does not; a toolchain where it does needs no binding, and
/// a caller can pin that with this.
#[cfg(target_os = "linux")]
pub fn interceptor_for_thrd_create(executable: &std::path::Path) -> bool {
    let symbols = std::process::Command::new("nm")
        .arg(executable)
        .output()
        .expect("nm is available");
    String::from_utf8_lossy(&symbols.stdout)
        .lines()
        .any(|line| line.contains("__interceptor_thrd_create"))
}

/// Compile `c_source` under ThreadSanitizer and run it. `bind_threads` selects
/// whether [`PTHREAD_BINDING`] is prepended; `false` reproduces the unbound
/// failure, which is what keeps the diagnosis pinned rather than remembered.
///
/// Returns `Err` only when the C compiler rejected the program — a program
/// that builds and then dies is a [`Sanitized`] with `clean == false`, because
/// dying is a measurement here and not a harness failure.
#[cfg(target_os = "linux")]
pub fn run(tag: &str, c_source: &str, bind_threads: bool) -> Result<Sanitized, String> {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    // Run-scoped by tag and pid, so two runs cannot truncate each other.
    let directory = std::env::temp_dir().join(format!(
        "qcore-task-route-tsan-{tag}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let c = directory.join("module.c");
    let binding = directory.join("bind.h");
    let executable = directory.join("module");
    std::fs::write(&c, c_source).unwrap();
    std::fs::write(&binding, PTHREAD_BINDING).unwrap();

    let mut compiler = Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args([
        "-std=c11",
        "-pthread",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-g",
        "-O0",
        "-fno-omit-frame-pointer",
        "-fsanitize=thread",
    ]);
    if bind_threads {
        compiler.arg("-include").arg(&binding);
    }
    let built = compiler
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .map_err(|error| format!("the C compiler could not be run: {error}"))?;
    if !built.status.success() {
        return Err(format!(
            "the C compiler rejected the emitted program:\n{}",
            String::from_utf8_lossy(&built.stderr)
        ));
    }

    let output = Command::new(&executable)
        .env("TSAN_OPTIONS", "halt_on_error=1")
        .output()
        .map_err(|error| format!("the built program could not be run: {error}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let races = stderr.matches("ThreadSanitizer: data race").count();
    let sanitized = Sanitized {
        clean: output.status.success() && !stderr.contains("ThreadSanitizer:"),
        races,
        signal: output.status.signal(),
        stdout,
        stderr,
    };
    std::fs::remove_dir_all(&directory).ok();
    Ok(sanitized)
}
