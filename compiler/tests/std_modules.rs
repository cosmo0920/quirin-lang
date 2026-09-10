//! End-to-end route selection, admission and refusal for `source.std-io.v1` —
//! the `std.fmt`, `std.io` and `std.logger` bundled modules (card
//! `STD-MODULES`).
//!
//! The native runs compare the emitted program's **stdout** with the reference
//! observation's stdout projection and its **stderr** with the stderr
//! projection. No order between the two streams is asserted anywhere, because
//! none is modeled: C gives no order between two separately buffered streams
//! and this card does not measure one.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;
use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

const LINES: &str = include_str!("fixtures/std_modules/lines.qcore");
const LOGGER_ONLY: &str = include_str!("fixtures/std_modules/logger_only.qcore");

// Exercise both checkout styles regardless of the host's Git configuration.
fn with_line_endings(source: &str) -> [String; 2] {
    let lf = source.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf, crlf]
}

fn without_line(source: &str, line: &str) -> String {
    source
        .replace(&format!("{line}\r\n"), "")
        .replace(&format!("{line}\n"), "")
}

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("std_modules.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
        },
    )
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

// --- The accepted module. ---

#[test]
fn the_three_modules_select_the_standard_io_profile() {
    let module = compile(LINES).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardIoV1
    );
    assert_eq!(module.entry.as_deref(), Some("main"));
    let checked = module
        .checked
        .as_standard_io()
        .expect("this route publishes standard-IO evidence");
    assert_eq!(checked.certificate.flushes, 1);
    // The certificate names its own `std.fmt` surface, not the whole vocabulary.
    assert_eq!(checked.certificate.renderings.len(), 2);
}

#[test]
fn the_derived_effect_row_is_exactly_io() {
    use qcore_core::typed_errors::Effect;
    let module = compile(LINES).unwrap();
    let checked = module.checked.as_standard_io().unwrap();
    assert!(checked.certificate.effects.contains(&Effect::Io));
    // Never Alloc, never Mut: the operand grammar admits no resource, so this
    // route cannot allocate or mutate one. This is the effect discipline.
    assert!(!checked.certificate.effects.contains(&Effect::Alloc));
    assert!(!checked.certificate.effects.contains(&Effect::Mut));
    assert_eq!(checked.certificate.effects.iter().count(), 1);
}

#[test]
fn the_published_certificate_is_independently_rederivable() {
    use qcore_core::std_io;
    let module = compile(LINES).unwrap();
    let checked = module.checked.as_standard_io().unwrap();
    std_io::recheck_std_io_certificate(&checked.lowered.program, &checked.certificate).unwrap();

    let mut tampered = checked.certificate.clone();
    tampered.lines.clear();
    assert!(
        std_io::recheck_std_io_certificate(&checked.lowered.program, &tampered).is_err(),
        "a tampered certificate must not survive an independent recheck"
    );
    let mut renamed = checked.certificate.clone();
    renamed.flushes += 1;
    assert!(
        std_io::recheck_std_io_certificate(&checked.lowered.program, &renamed).is_err(),
        "a tampered barrier count must not survive an independent recheck"
    );
}

#[test]
fn the_reference_observation_separates_the_two_streams() {
    let module = compile(LINES).unwrap();
    assert_eq!(
        module.extern_observation.as_deref().unwrap(),
        [
            "out: 42",
            "out: 0x000000FF",
            "err: [INFO] 42",
            "err: [WARN] 0x000000FF",
            "err: [FATAL] -7",
            "out: -7",
        ]
    );
    // The sequential observation field models something else and stays empty.
    assert!(module.reference_observation.is_none());
}

#[test]
fn emission_renders_both_streams_and_the_barrier() {
    let module = compile(LINES).unwrap();
    let c = module.c_source.as_deref().expect("this route emits C11");
    // Card `TRACE-STDIO` moved this route onto the shared emission route that
    // the four synthesized trace backends now also use, so an emitted program
    // has one printing mechanism rather than two. Everything this pin is
    // actually about — which stream, which rendering, which level tag, which
    // operand expression, and that the barrier is a real flush — is unchanged;
    // only the spelling of the call moved.
    for expected in [
        "QCORE_IO_WRITE(stdout, \"%\" PRId32 \"\\n\", s_total);",
        "QCORE_IO_WRITE(stdout, \"0x%08\" PRIX32 \"\\n\", (uint32_t)(s_mask));",
        "QCORE_IO_WRITE(stderr, \"[INFO] %\" PRId32 \"\\n\", s_total);",
        "QCORE_IO_WRITE(stderr, \"[FATAL] %\" PRId32 \"\\n\", INT32_C(-7));",
        "QCORE_IO_FLUSH(stderr);",
    ] {
        assert!(c.contains(expected), "emitted C is missing `{expected}`");
    }
    // Card `TRACE-STDIO`: this route and the four synthesized trace backends
    // share one C-level printing route, so an emitted program has no
    // standard-output call of its own.
    assert_eq!(qcore_core::io_emit::direct_stdio_calls(c), Vec::new());
}

#[test]
fn a_logger_only_module_needs_no_rendering_import() {
    let module = compile(LOGGER_ONLY).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardIoV1
    );
    let checked = module.checked.as_standard_io().unwrap();
    assert_eq!(checked.certificate.flushes, 1);
    assert!(
        checked.certificate.renderings.is_empty(),
        "a module that renders nothing names no rendering"
    );
    assert!(module.extern_observation.as_deref().unwrap().is_empty());
}

// --- Q015: each capability is gated by its own module, and by no other. ---

#[test]
fn printing_without_the_io_import_refuses() {
    for source in with_line_endings(&without_line(LINES, "import std.io;")) {
        assert_eq!(codes(&source), ["Q015"]);
    }
}

#[test]
fn logging_without_the_logger_import_refuses() {
    for source in with_line_endings(&without_line(LINES, "import std.logger;")) {
        assert_eq!(codes(&source), ["Q015"]);
    }
}

#[test]
fn flushing_without_the_logger_import_refuses() {
    let source = without_line(LOGGER_ONLY, "import std.logger;");
    assert_eq!(codes(&source), ["Q015"]);
}

#[test]
fn rendering_without_the_fmt_import_refuses() {
    for source in with_line_endings(&without_line(LINES, "import std.fmt;")) {
        assert_eq!(codes(&source), ["Q015"]);
    }
}

#[test]
fn the_io_import_alone_does_not_grant_the_logger() {
    let source = "import std.fmt;\n\
                  import std.io;\n\
                  fn main() -> unit effects { io } {\n    \
                      print dec 1;\n    \
                      log info dec 1;\n    \
                      return;\n\
                  }\n";
    assert_eq!(codes(source), ["Q015"]);
}

#[test]
fn the_logger_import_alone_does_not_grant_standard_output() {
    let source = "import std.fmt;\n\
                  import std.logger;\n\
                  fn main() -> unit effects { io } {\n    \
                      log info dec 1;\n    \
                      print dec 1;\n    \
                      return;\n\
                  }\n";
    assert_eq!(codes(source), ["Q015"]);
}

#[test]
fn a_misspelled_module_refuses_as_an_unknown_module() {
    for name in ["std.fmtt", "std.stdio", "std.log"] {
        let source = LINES.replace("import std.fmt;", &format!("import {name};"));
        assert_eq!(codes(&source), ["Q014"], "`{name}` must be unknown");
    }
}

#[test]
fn importing_the_modules_in_another_file_does_not_grant_this_one_the_capability() {
    let granting = "import std.fmt;\nimport std.io;\nimport std.logger;\n";
    let diagnostics = compile_files(
        &[
            SourceFile::new("granting.qcore", granting),
            SourceFile::new("using.qcore", without_line(LINES, "import std.io;")),
        ],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
        },
    )
    .expect_err("imports are file-local");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "Q015")
    );
}

#[test]
fn a_module_that_imports_without_printing_needs_no_route() {
    // Importing vocabulary is not using it: this file stays on the sequential
    // route and is not dragged onto `source.std-io.v1`.
    let source = "import std.fmt;\n\
                  import std.io;\n\
                  import std.logger;\n\
                  fn main() -> i32 { return 0; }\n";
    let module = compile(source).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
}

// --- Q028: the operand rule is the effect discipline. ---

#[test]
fn rendering_a_name_nothing_binds_refuses_at_name_resolution() {
    // Measured, and recorded rather than worked around: the established
    // name-resolution gate runs before this route and refuses an unbound
    // operand at `Q013`, so `Q028` is not the code for this case. The refusal
    // is correct and fail-closed; only the message is less specific.
    let source = LINES.replace("print dec total;", "print dec missing;");
    assert_eq!(codes(&source), ["Q013"]);
}

#[test]
fn rendering_a_resource_refuses() {
    // The whole point of the v1 operand grammar: there is no form that could
    // denote a resource. `qcore_acquire` is a resource acquisition reachable
    // without the concurrency import, so this is a real resource binding — the
    // `let` refuses it as a binding, and the operand refuses it as a rendering.
    // Both halves matter: neither alone would keep a resource unrenderable.
    let source = "import std.fmt;\n\
                  import std.io;\n\
                  fn main() -> unit effects { io } {\n    \
                      let r = qcore_acquire(true);\n    \
                      print dec r;\n    \
                      return;\n\
                  }\n";
    assert_eq!(codes(source), ["Q030", "Q028"]);
}

#[test]
fn rendering_an_owned_buffer_refuses() {
    // The concurrency-gated allocation refuses even earlier, at its own import
    // gate. Pinned so the buffer form cannot quietly become renderable if that
    // gate ever moves.
    let source = "import std.fmt;\n\
                  import std.io;\n\
                  fn main() -> unit effects { io } {\n    \
                      let b = qcore_buffer_new(42);\n    \
                      print dec b;\n    \
                      return;\n\
                  }\n";
    assert_eq!(codes(source), ["Q015"]);
}

#[test]
fn a_moved_operand_refuses() {
    let source = LINES.replace("print dec total;", "print dec move total;");
    assert_eq!(codes(&source), ["Q030"]);
}

// --- Q029: the declared effect row must admit the derived one. ---

#[test]
fn writing_without_declaring_io_refuses() {
    let source = LINES.replace("effects { io }", "effects { alloc }");
    assert_eq!(codes(&source), ["Q029"]);
}

#[test]
fn writing_with_no_effect_row_refuses() {
    let source = LINES.replace(" effects { io }", "");
    assert_eq!(codes(&source), ["Q029"]);
}

// --- Q030: the flat entry shape. ---

#[test]
fn an_unknown_rendering_refuses() {
    // Measured: an unreserved word in a rendering position is collected as a
    // referenced name, so the existing gate refuses it at `Q013` before this
    // route's `Q030` message is reached. Fail-closed and correct; the message
    // is less specific. Named follow-on `STD-FMT-DIAGNOSTIC`.
    let source = LINES.replace("print dec total;", "print oct total;");
    assert_eq!(codes(&source), ["Q013"]);
}

#[test]
fn an_unknown_level_refuses() {
    let source = LINES.replace("log info dec total;", "log trace dec total;");
    assert_eq!(codes(&source), ["Q013"]);
}

#[test]
fn a_reserved_word_in_a_rendering_position_reaches_this_routes_refusal() {
    // The complement of the two tests above: when the word *is* reserved, name
    // resolution does not claim it and this route's own `Q030` is what refuses.
    // Both halves are pinned so the boundary between the two gates is recorded
    // rather than inferred.
    let source = LINES.replace("print dec total;", "print read total;");
    assert_eq!(codes(&source), ["Q030"]);
}

#[test]
fn the_error_level_spelling_refuses_because_error_is_reserved() {
    // Recorded deliberately: `error` is the error-type declaration keyword, so
    // this card's severity is spelled `fatal`. The refusal is the record.
    let source = LINES.replace("log fatal dec -7;", "log error dec -7;");
    assert_eq!(codes(&source), ["Q030"]);
}

#[test]
fn a_flush_with_an_operand_refuses() {
    let source = LINES.replace("flush;", "flush 1;");
    assert_eq!(codes(&source), ["Q030"]);
}

#[test]
fn printing_outside_the_entry_refuses() {
    let source = LINES.replace(
        "fn main()",
        "fn helper() -> unit effects { io } { print dec 1; return; }\nfn main()",
    );
    assert!(codes(&source).contains(&"Q030".to_owned()));
}

#[test]
fn an_entry_that_never_returns_refuses() {
    let source = without_line(LINES, "    return;");
    assert_eq!(codes(&source), ["Q030"]);
}

// --- Route composition stays fail-closed. ---

#[test]
fn printing_inside_a_thread_scope_keeps_the_task_routes_refusal() {
    let source = "import std.concurrency;\n\
                  import std.fmt;\n\
                  import std.io;\n\
                  concurrency checked;\n\
                  fn main() -> i32 effects { alloc } {\n    \
                      thread scope s {\n        \
                          print dec 1;\n    \
                      }\n    \
                      return 0;\n\
                  }\n";
    assert!(
        compile(source).is_err(),
        "a printing thread scope must stay refused, not silently unmodeled"
    );
}

// --- Native evidence, under ASan/UBSan/LSan. ---

#[test]
fn the_emitted_program_matches_the_reference_per_stream() {
    let module = compile(LINES).unwrap();
    let reference = module.extern_observation.as_deref().unwrap();
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "lines");

    let expected_out = reference
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    let expected_err = reference
        .iter()
        .filter_map(|line| line.strip_prefix("err: ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();

    assert_eq!(out, expected_out, "stdout must match the reference");
    assert_eq!(err, expected_err, "stderr must match the reference");
}

#[test]
fn the_logger_only_program_runs_natively_and_writes_nothing() {
    let module = compile(LOGGER_ONLY).unwrap();
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "logger-only");
    assert!(out.is_empty(), "a bare barrier writes no line: {out:?}");
    assert!(err.is_empty(), "a bare barrier writes no line: {err:?}");
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
///
/// The sanitizer runtimes write their own findings to stderr and abort, so a
/// clean exit with the expected stderr lines is itself the evidence that no
/// finding was reported.
fn run_native(source: &str, _label: &str) -> (Vec<String>, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let run = windows_c::run(source, None, &[]);
        assert!(
            run.status.success(),
            "emitted program failed:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        (out, err)
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-std-modules-{_label}-{}-{}",
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
                    // Apple's AddressSanitizer provides no LeakSanitizer, so leak
                    // detection stays a Linux-only observation.
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
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        std::fs::remove_dir_all(directory).unwrap();
        (out, err)
    }
}
