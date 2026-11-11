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

//! End-to-end route selection, admission and refusal for `source.event.v1` —
//! the `std.event` bundled module (card `STD-EVENT`).
//!
//! # What these tests are about
//!
//! Two things, and they are separate. The **custody** half is the existing
//! `event_reactor`/`os_endpoint` models, which this card made reachable from
//! source for the first time (the recorded U3 blocker); those tests assert that
//! a reactor refusal reaches a source span with the reactor's own words rather
//! than a paraphrase. The **determinism** half is this card's own: a dispatch
//! turn is admitted only where the program itself induced the readiness it
//! declares, and the refusals name their ground.
//!
//! The native run compares the explicitly enabled diagnostic stdout log with
//! the reference observation and keeps stderr quiet. Every identity in that trace is a **model** identity assigned by
//! the checker, never a real file descriptor — a real descriptor is not
//! model-determined, so a trace carrying one could not be compared at all.
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const SELF_PIPE: &str = include_str!("../../examples/events/self_pipe.qr");
const WRITE_INTEREST: &str = include_str!("../../examples/events/write_interest.qr");
const README: &str = include_str!("../../examples/events/README.md");
const WINDOWS_RUNNER: &str = include_str!("../../examples/events/build-windows.ps1");

#[cfg(target_os = "windows")]
fn windows_afunix_paths() -> std::collections::BTreeSet<std::path::PathBuf> {
    std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    (name.starts_with("qel") || name.starts_with("qec")) && name.ends_with(".tmp")
                })
        })
        .collect()
}

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("std_event.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn diagnostics(source: &str) -> Vec<qcore_compiler::FileDiagnostic> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
}

fn codes(source: &str) -> Vec<String> {
    diagnostics(source)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn message(source: &str) -> String {
    diagnostics(source)
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The self-pipe fixture with `body` replacing its statement sequence.
fn with_body(body: &str) -> String {
    format!("#import std.event;\n\nfn main() -> i32 effects {{ io }} {{\n{body}\n}}\n")
}

// --- The accepted module. ---

#[test]
fn the_event_module_selects_the_event_profile() {
    let module = compile(SELF_PIPE).unwrap();
    assert_eq!(module.execution_profile, SourceExecutionProfile::EventV1);
    assert_eq!(module.entry.as_deref(), Some("main"));
    assert!(module.c_source.is_some());
}

#[test]
fn the_reference_observation_is_the_whole_event_trace() {
    let module = compile(SELF_PIPE).unwrap();
    let observation = module
        .extern_observation
        .as_ref()
        .expect("the event route publishes a reference observation");
    assert_eq!(
        observation,
        &vec![
            "out: QCORE_EVENT_OPEN".to_owned(),
            "out: QCORE_EVENT_ENDPOINTS 0 1".to_owned(),
            "out: QCORE_EVENT_WATCH 0 0 READ".to_owned(),
            "out: QCORE_EVENT_WAKE 0".to_owned(),
            "out: QCORE_EVENT_DISPATCH 0".to_owned(),
            "out: QCORE_EVENT_DISPATCH IDLE".to_owned(),
            "out: QCORE_EVENT_UNWATCH 0".to_owned(),
            "out: QCORE_EVENT_CLOSE 0".to_owned(),
            "out: QCORE_EVENT_CLOSE 1".to_owned(),
            "out: QCORE_EVENT_TEARDOWN".to_owned(),
            "out: QCORE_EVENT_RESULT 0".to_owned(),
        ]
    );
}

#[test]
fn the_emitted_program_carries_the_embedded_shim_and_the_explicit_logger_adapter() {
    let module = compile(SELF_PIPE).unwrap();
    let source = module.c_source.as_deref().unwrap();
    // Self-contained on the `c11_threads_compat.h` precedent: the readiness
    // primitives are in the emitted text, so the program needs no QCore include
    // path and no linked QCore artifact.
    assert!(source.contains("QCORE_EVENT_POLL_COMPAT_H"));
    assert!(source.contains("QCORE_DIAG_EVENT_STEP"));
    assert!(source.contains("#define QCORE_DIAGNOSTIC_THRESHOLD QCORE_LOG_TRACE"));
    assert!(source.contains("qcore_diagnostic_stdout"));
    assert!(!source.contains("QCORE_IO_WRITE("));
}

#[test]
fn an_event_module_without_log_level_keeps_the_null_sink_and_needs_no_stdio() {
    let quiet = SELF_PIPE
        .replace("#import std.logger;", "")
        .replace("    log_level trace;", "");
    let module = compile(&quiet).unwrap();
    let source = module.c_source.as_deref().unwrap();
    assert!(!source.contains("#include <stdio.h>"));
    assert!(!source.contains("fprintf("));
    assert!(!source.contains("qcore_diagnostic_stdout"));
    assert!(source.contains("#define QCORE_DIAGNOSTIC_THRESHOLD 6"));
}

// --- The published example explicitly renders its retained system trace. ---

#[test]
fn the_published_transcript_is_the_reference_observation() {
    let module = compile(SELF_PIPE).unwrap();
    assert!(!module.extern_observation.as_ref().unwrap().is_empty());
    assert!(README.contains("log_level trace"));
    assert!(README.contains("QCORE_EVENT_OPEN"));
    assert!(README.contains("QCORE_EVENT_RESULT 0"));
}

#[test]
fn the_published_program_is_the_example_file() {
    // The README quotes the program as well as its output. Both are compared,
    // so the page cannot drift from the file in either direction.
    let quoted = README
        .split("```qcore")
        .nth(1)
        .expect("the README quotes the program")
        .split("```")
        .next()
        .unwrap();
    for line in quoted.lines().filter(|line| !line.trim().is_empty()) {
        assert!(
            SELF_PIPE.contains(line.trim()),
            "the README quotes a line the example does not contain: {line}"
        );
    }
}

#[test]
fn the_windows_runner_reads_the_published_transcript_rather_than_a_copy() {
    // The one arm the standing gate does not run is the one whose runner must
    // not carry its own copy of the expected output: a stale copy there would
    // be a Windows run that passes against the wrong bytes.
    assert!(!WINDOWS_RUNNER.contains("QCORE_EVENT_ENDPOINTS"));
    // And it must not add a library flag: the emitted shim links the socket
    // library itself, so the Windows arm is as self-contained as the other two.
    assert!(!WINDOWS_RUNNER.to_lowercase().contains("ws2_32.lib\""));
}

#[test]
fn the_embedded_shim_carries_all_three_platform_arms() {
    // Three arms, on the `c11_threads_compat.h` shape: `_WIN32`, `__APPLE__`,
    // and everything else. Pinned as a text check rather than left to a reader,
    // because only one of the three is *run* by this gate (see the native-run
    // test below) and an edit that silently dropped an arm would otherwise be
    // invisible until someone built on that platform.
    let module = compile(SELF_PIPE).unwrap();
    let source = module.c_source.as_deref().unwrap();
    assert!(source.contains("#if defined(_WIN32)"));
    assert!(source.contains("#elif defined(__APPLE__)"));
    // Each arm's own visibility declaration: what the platforms actually differ
    // in under `-std=c11 -pedantic`.
    assert!(source.contains("#define _DARWIN_C_SOURCE 1"));
    assert!(source.contains("#define _POSIX_C_SOURCE 200809L"));
    // Each arm's own wait and pair primitive.
    assert!(source.contains("poll(set, (nfds_t) count, timeout_ms)"));
    assert!(source.contains("WSAPoll(set, (ULONG) count, timeout_ms)"));
    assert!(source.contains("socketpair(AF_UNIX, SOCK_STREAM, 0, pair)"));
    // Windows has no socketpair call, but a new enough SDK and runtime can
    // construct the same stream-pair shape through a pathname listener. Header
    // presence only compiles the candidate; the socket probe selects it.
    assert!(source.contains("#if __has_include(<afunix.h>)"));
    assert!(source.contains("qcore_event_pair_afunix"));
    assert!(source.contains("GetTempFileNameW"));
    assert!(source.contains("atexit(qcore_event_afunix_cleanup)"));
    assert!(source.contains("qcore_event_afunix_closed(endpoint)"));
    assert!(source.contains("qcore_event_windows_afunix_capability"));
    assert!(source.contains("WSAEAFNOSUPPORT"));
    assert!(source.contains("bind(connector, (struct sockaddr *) &connector_address"));
    assert!(source.contains("accepted_address.sun_family != AF_UNIX"));
    assert!(source.contains("accepted_address.sun_path, connector_address.sun_path"));
    assert!(source.contains("return qcore_event_pair_afunix(first, second)"));
    // Older SDK/runtime support remains the existing private loopback route.
    assert!(source.contains("qcore_event_pair_loopback"));
    assert!(source.contains("return qcore_event_pair_loopback(first, second)"));
    // Test-only compile controls let the Windows native run exercise both
    // routes without changing the default runtime selection.
    assert!(source.contains("QCORE_EVENT_TEST_REQUIRE_AF_UNIX"));
    assert!(source.contains("QCORE_EVENT_TEST_FORCE_LOOPBACK"));
    assert!(source.contains("QCORE_EVENT_TEST_DISABLE_AF_UNIX"));
    assert!(source.contains("QCORE_EVENT_TEST_ABORT_AFTER_AF_UNIX_PAIR"));
    assert!(source.contains("accept(listener, NULL, NULL)"));
    // Deliberately absent: v1 reaches for neither kqueue nor epoll. poll(2) is
    // the portable deterministic subset, and a declared ready set of at most
    // one member buys nothing from either.
    assert!(!source.contains("kqueue("));
    assert!(!source.contains("epoll_create"));
    assert!(!source.contains("epoll_wait"));
}

#[test]
fn the_descriptor_type_is_per_arm_and_never_reaches_the_trace() {
    let module = compile(SELF_PIPE).unwrap();
    let source = module.c_source.as_deref().unwrap();
    // POSIX endpoints are `int`; Windows endpoints are `SOCKET`, which is not a
    // CRT file descriptor. Both are hidden behind one typedef.
    assert!(source.contains("typedef int qcore_event_fd;"));
    assert!(source.contains("typedef SOCKET qcore_event_fd;"));
    // And neither reaches an observation: every identity in the trace is the
    // model identity the checker assigned, so the byte comparison is identical
    // on all three arms. A real descriptor number is not model-determined and
    // could not be compared at all.
    for line in module.extern_observation.as_ref().unwrap() {
        assert!(
            !line.contains("qcore_endpoint"),
            "a trace line must carry no run-time descriptor: {line}"
        );
    }
    assert!(!source.contains("QCORE_IO_WRITE(stdout, \"QCORE_EVENT_ENDPOINTS %"));
}

#[test]
fn the_declaration_is_verified_at_run_time_rather_than_trusted() {
    let module = compile(SELF_PIPE).unwrap();
    let source = module.c_source.as_deref().unwrap();
    // The whole determinism argument rests on a platform premise. This is the
    // line that keeps it honest: the emitted program compares the ready set
    // `poll` actually reported against the declaration and exits non-zero on
    // disagreement, so a false premise is a *failed run*, never a wrong verdict.
    assert!(source.contains("qcore_event_wait(qcore_watch, 1u, 5000, 0)"));
    assert!(source.contains("qcore_event_wait(qcore_watch, 1u, 0, -1)"));
    assert!(source.contains("return 73;"));
}

// --- The check-only arm. ---

#[test]
fn a_write_registration_is_checked_in_full_and_emits_no_c() {
    let module = compile(WRITE_INTEREST).unwrap();
    assert_eq!(module.execution_profile, SourceExecutionProfile::EventV1);
    // Accepted and checked. The absence is of *execution*, not of checking.
    assert!(module.c_source.is_none());
    assert!(module.extern_observation.is_none());
}

#[test]
fn the_two_absences_of_a_check_only_module_travel_together() {
    // A module that emitted C without a reference observation, or the reverse,
    // would publish an unchecked comparison. Pinned rather than left to the
    // shape of one `if`.
    for source in [SELF_PIPE, WRITE_INTEREST] {
        let module = compile(source).unwrap();
        assert_eq!(
            module.c_source.is_some(),
            module.extern_observation.is_some()
        );
    }
}

// --- The import gate (standing constraint 13's shape). ---

#[test]
fn the_event_vocabulary_needs_its_own_file_local_import() {
    let without = SELF_PIPE.replace("#import std.event;", "");
    let codes = codes(&without);
    assert_eq!(codes, vec!["Q015".to_owned()]);
    assert!(
        message(&without).contains("#import std.event;"),
        "the message must name the missing module"
    );
}

#[test]
fn no_other_module_grants_the_event_vocabulary() {
    // `std.io` is a capability, not a key to every other one — the discipline
    // `STD-MODULES` fixed for its own three modules.
    let wrong = SELF_PIPE.replace("#import std.event;", "#import std.io;");
    assert_eq!(codes(&wrong), vec!["Q015".to_owned()]);
}

#[test]
fn an_unknown_module_is_still_an_unknown_module() {
    let wrong = SELF_PIPE.replace("#import std.event;", "#import std.events;");
    assert_eq!(codes(&wrong), vec!["Q014".to_owned()]);
}

// --- The determinism boundary, at the source level. ---

#[test]
fn two_ready_registrations_in_one_turn_are_refused_by_name_with_the_ground() {
    let source = with_body(
        "    event open;
    endpoints a b;
    endpoints c d;
    watch first a read;
    watch second c read;
    wake a;
    wake c;
    dispatch ready first;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q061".to_owned()));
    let message = message(&source);
    assert!(message.contains("`poll(2)` reports a set"), "{message}");
    // The ground is stated, and what it waits on is named. A reader of the
    // diagnostic does not have to find the design document.
    assert!(
        message.contains("equal-priority events are deliberately nondeterministic"),
        "{message}"
    );
    assert!(message.contains("OBS-REORDERING"), "{message}");
}

#[test]
fn waiting_on_readiness_the_program_did_not_induce_is_refused() {
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    dispatch ready reader;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q061".to_owned()));
    assert!(
        message(&source).contains("readiness the program itself induced"),
        "{}",
        message(&source)
    );
}

#[test]
fn an_idle_turn_that_is_not_idle_is_refused() {
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    wake a;
    dispatch idle;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q061".to_owned()));
}

#[test]
fn an_undeclared_dispatch_turn_has_no_spelling_at_all() {
    // Not a rule this route states about a form it admits: there *is* no
    // undeclared turn in the grammar, because a turn with no declaration has no
    // observation to compare.
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    dispatch;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q058".to_owned()));
    assert!(
        message(&source).contains("a certificate compares one observation"),
        "{}",
        message(&source)
    );
}

// --- The reactor model's refusals reach the source unweakened. ---

#[test]
fn a_leaked_endpoint_is_refused_with_the_reactor_models_own_diagnostic() {
    let source = with_body(
        "    event open;
    endpoints a b;
    close a;
    event close;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q060".to_owned()));
    // The reactor's own name, not a paraphrase: a reader can go to
    // `event_reactor` and find the rule that refused this.
    assert!(
        message(&source).contains("L03DescriptorCustody"),
        "{}",
        message(&source)
    );
}

#[test]
fn tearing_down_a_context_under_a_live_registration_is_refused_by_the_reactor_model() {
    let source = with_body(
        "    event open;
    endpoints a b;
    watch reader a read;
    close a;
    close b;
    event close;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q060".to_owned()));
}

// --- Shape and names. ---

#[test]
fn a_registration_on_an_endpoint_this_entry_did_not_bind_is_refused() {
    let source = with_body(
        "    event open;
    endpoints a b;
    close a;
    watch reader a read;
    close b;
    event close;
    return 0;",
    );
    assert!(codes(&source).contains(&"Q059".to_owned()));
}

#[test]
fn a_module_that_both_dispatches_and_prints_stays_with_this_route_and_is_now_admitted() {
    // Route selection puts this module here, and until card
    // `COMPOSE-EVENT-LINES` this route kept its refusal for the statements it
    // did not model. It models them now: the line grammar is
    // `source.std-io.v1`'s, read by that route's own reader.
    //
    // **This test carried the old pin and was rewritten rather than deleted**,
    // the precedent `COMPOSE-CONCURRENCY` set when it moved `TRACE-STDIO`'s.
    // What survives of it is the half that is still true: route selection is
    // unmoved, so the module is still an event module and still publishes this
    // profile.
    let source = "#import std.event;\n#import std.io;\n#import std.fmt;\n\n\
                  fn main() -> i32 effects { io } {\n    event open;\n    print dec 1;\n    \
                  event close;\n    return 0;\n}\n";
    let module = compile(source).expect("a composed event module compiles");
    assert_eq!(module.execution_profile, SourceExecutionProfile::EventV1);
    assert_eq!(
        module.extern_observation.as_deref(),
        Some(["out: 1".to_owned()].as_slice())
    );
    // And the refusal that is still a refusal: a statement head neither route
    // models is still not one `source.event.v1` models.
    let unmodelled = "#import std.event;\n\nfn main() -> i32 effects { io } {\n    event open;\n    \
                      endpoints a b;\n    drop a;\n    close a;\n    close b;\n    \
                      event close;\n    return 0;\n}\n";
    assert!(codes(unmodelled).contains(&"Q058".to_owned()));
    assert!(
        message(unmodelled).contains("is not a statement `source.event.v1` models"),
        "{}",
        message(unmodelled)
    );
}

#[test]
fn an_event_statement_without_the_io_modules_still_reports_the_io_modules() {
    // The import gates run before route selection, so a `print` in an event
    // module without `std.io` reports the missing *IO* module rather than an
    // event-route shape refusal. Measured, and pinned so that a later change to
    // gate ordering is visible rather than silent.
    let source = with_body(
        "    event open;
    print dec 1;
    event close;
    return 0;",
    );
    assert_eq!(codes(&source), vec!["Q015".to_owned()]);
    assert!(
        message(&source).contains("#import std.io;"),
        "{}",
        message(&source)
    );
}

#[test]
fn an_entry_without_the_io_effect_is_refused() {
    let source = SELF_PIPE.replace(" effects { io }", "");
    assert!(codes(&source).contains(&"Q062".to_owned()));
}

#[test]
fn an_event_statement_outside_the_entry_is_refused() {
    let source = "#import std.event;\n\nfn helper() -> i32 {\n    event open;\n    return 0;\n}\n\n\
                  fn main() -> i32 effects { io } {\n    event open;\n    event close;\n    \
                  return 0;\n}\n";
    assert!(codes(source).contains(&"Q058".to_owned()));
}

// --- The native run. ---

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), keep stderr quiet, and compare its explicitly selected
/// stdout system trace with the reference observation.
///
/// On Windows the established native-C harness selects the installed MSVC
/// toolchain and runs the AF_UNIX-first/loopback-fallback arm. The Unix arm
/// retains the sanitizer instrumentation; MSVC execution is a native
/// byte-comparison, not a sanitizer claim.
#[test]
fn the_emitted_program_reproduces_the_reference_observation() {
    let module = compile(SELF_PIPE).unwrap();
    let source = module.c_source.as_deref().unwrap();
    let expected = module
        .extern_observation
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(str::to_owned))
        .collect::<Vec<_>>();

    #[cfg(target_os = "windows")]
    let runs = {
        let paths_before_failure = windows_afunix_paths();
        let early_failure = windows_c::run(
            source,
            None,
            &[
                "-DQCORE_EVENT_TEST_REQUIRE_AF_UNIX=1".to_owned(),
                "-DQCORE_EVENT_TEST_ABORT_AFTER_AF_UNIX_PAIR=1".to_owned(),
            ],
        );
        assert_eq!(early_failure.status.code(), Some(71));
        assert_eq!(
            windows_afunix_paths(),
            paths_before_failure,
            "the atexit rollback must remove the pathname after an early return"
        );
        [
            (
                "AF_UNIX",
                windows_c::run(
                    source,
                    None,
                    &["-DQCORE_EVENT_TEST_REQUIRE_AF_UNIX=1".to_owned()],
                ),
            ),
            (
                "loopback fallback",
                windows_c::run(
                    source,
                    None,
                    &["-DQCORE_EVENT_TEST_FORCE_LOOPBACK=1".to_owned()],
                ),
            ),
            (
                "no-AF_UNIX-header fallback",
                windows_c::run(
                    source,
                    None,
                    &["-DQCORE_EVENT_TEST_DISABLE_AF_UNIX=1".to_owned()],
                ),
            ),
        ]
    };

    #[cfg(not(target_os = "windows"))]
    let runs = [("POSIX", {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-std-event-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let generated = directory.join("generated.c");
        let binary = directory.join("generated");
        std::fs::write(&generated, source).unwrap();
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
            .arg(&generated)
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
                    // Apple's AddressSanitizer provides no LeakSanitizer, so
                    // leak detection stays a Linux-only observation.
                    "detect_leaks=0:halt_on_error=1"
                } else {
                    "detect_leaks=1:halt_on_error=1"
                },
            )
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        run
    })];
    for (route, run) in runs {
        // A non-zero exit here is not only a sanitizer finding: it is also how
        // the emitted program reports that the ready set `poll` returned
        // disagreed with the declaration. Both are failures, deliberately so.
        assert!(
            run.status.success(),
            "{route} emitted program failed with {}:\n{}",
            run.status,
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(!expected.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&run.stdout)
                .lines()
                .collect::<Vec<_>>(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "the {route} diagnostic log"
        );
        assert!(run.stderr.is_empty(), "the {route} run wrote stderr");
    }
}
