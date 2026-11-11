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

use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::cfg::interpret_cfg;
use crate::cleanup_lowering::{SourceOp, SourceProgram, interpret_source, lower_cleanup_cfg};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn corpus() -> Vec<SourceProgram> {
    vec![
        SourceProgram::new(
            "normal_transfer",
            vec![1],
            vec!["IoError".into()],
            vec![
                SourceOp::Acquire {
                    resource: 1,
                    succeeds: true,
                    error: "IoError".into(),
                },
                SourceOp::TransferToResult(1),
                SourceOp::Return(23),
            ],
        )
        .unwrap(),
        SourceProgram::new(
            "partial_failure",
            vec![1, 2],
            vec!["IoError".into()],
            vec![
                SourceOp::Acquire {
                    resource: 1,
                    succeeds: true,
                    error: "IoError".into(),
                },
                SourceOp::Acquire {
                    resource: 2,
                    succeeds: false,
                    error: "IoError".into(),
                },
                SourceOp::Drop(2),
                SourceOp::Drop(1),
                SourceOp::Return(0),
            ],
        )
        .unwrap(),
        SourceProgram::new(
            "lifo_failure",
            vec![7, 9],
            vec!["IoError".into(), "ParseError".into()],
            vec![
                SourceOp::Acquire {
                    resource: 7,
                    succeeds: true,
                    error: "IoError".into(),
                },
                SourceOp::Acquire {
                    resource: 9,
                    succeeds: true,
                    error: "IoError".into(),
                },
                SourceOp::Check {
                    passes: false,
                    error: "ParseError".into(),
                },
                SourceOp::Drop(9),
                SourceOp::Drop(7),
                SourceOp::Return(0),
            ],
        )
        .unwrap(),
        SourceProgram::new(
            "signed_value",
            vec![],
            vec!["IoError".into()],
            vec![SourceOp::Return(i32::MIN)],
        )
        .unwrap(),
    ]
}

#[test]
fn emitted_c_is_readable_and_has_tagged_abi_and_cleanup_labels() {
    let cfg = lower_cleanup_cfg(&corpus().remove(1)).unwrap();
    let c = emit_c11(&cfg).unwrap();
    assert!(c.contains("enum qcore_error_tag"));
    assert!(c.contains("union qcore_error_payload"));
    assert!(c.contains("q_cleanup_"));
    assert!(c.contains("q_release(&q_res_1"));
}

#[test]
fn the_emitted_program_routes_trace_events_only_to_the_diagnostic_logger() {
    for source in corpus() {
        let cfg = lower_cleanup_cfg(&source).unwrap();
        let c = emit_c11(&cfg).unwrap();
        assert!(c.contains("qcore_diagnostic_log(QCORE_LOG_TRACE"));
        assert!(!c.contains("QCORE_IO_WRITE("));
    }
}

#[test]
fn emitted_c_keeps_helper_bodies_and_report_block_indented() {
    let cfg = lower_cleanup_cfg(&corpus().remove(1)).unwrap();
    let c = emit_c11(&cfg).unwrap();
    // Rust string-literal continuations once stripped this indentation, leaving
    // the runtime helpers and main's report block flush left. The emitted unit
    // is meant to be readable, so each nesting level stays four-space indented.
    for expected in [
        "    enum qcore_diagnostic_kind diagnostic_kind;",
        "        abort();",
        "    q_trace('A', identity);",
        "        QCORE_DIAGNOSTIC_SINK(&event);",
    ] {
        assert!(
            c.lines().any(|line| line == expected),
            "missing indented line in emitted C: {expected:?}"
        );
    }
    // Statements never sit at column 0: only preprocessor lines, file-scope
    // declarations, function-scope braces and goto labels start there.
    for line in c.lines() {
        if line.is_empty() || line.starts_with(' ') {
            continue;
        }
        assert!(
            line.starts_with('#')
                // A file-scope block comment opens at column 0; the single
                // emission route's prelude is introduced by one. The rule this
                // test enforces is that no *statement* sits at column 0, and
                // that half is unchanged.
                || line.starts_with("/*")
                || line.starts_with("enum ")
                || line.starts_with("union ")
                || line.starts_with("struct ")
                || line.starts_with("static ")
                || line.starts_with("int main")
                || line.ends_with(':')
                || line == "{"
                || line == "}"
                || line == "};",
            "unindented statement in emitted C: {line:?}"
        );
    }
}

#[test]
fn compiled_quiet_c_matches_source_and_cfg_and_emits_no_stream_bytes() {
    for source in corpus() {
        let cfg = lower_cleanup_cfg(&source).unwrap();
        let reference = interpret_source(&source).unwrap();
        assert_eq!(interpret_cfg(&cfg).unwrap(), reference);
        compile_and_run(&cfg);
    }
}

fn compile_and_run(cfg: &TypedCfg) {
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "qcore-cycle7-{}-{timestamp}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let source_path = directory.join("generated.c");
    let binary_path = directory.join(if cfg!(target_env = "msvc") {
        "generated.exe"
    } else {
        "generated"
    });
    fs::write(&source_path, emit_c11(cfg).unwrap()).unwrap();
    let flags: &[&str] = if cfg!(target_env = "msvc") {
        // /RTC1 enables the native debug runtime's stack-frame and
        // uninitialized-use checks. /sdl adds Microsoft's stricter
        // security diagnostics and runtime checks. The cc configuration
        // below adds /W4 and /WX without first injecting /W0.
        &["/std:c11", "/TC", "/sdl", "/RTC1"]
    } else {
        &[
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-fsanitize=address,undefined",
            "-fno-omit-frame-pointer",
        ]
    };
    let rust_target = rust_host_target();
    let mut build = cc::Build::new();
    build
        .host(&rust_target)
        .target(&rust_target)
        .file(&source_path)
        .out_dir(&directory)
        .opt_level(0)
        .debug(false)
        .cargo_metadata(false)
        .flags(flags);
    if cfg!(target_env = "msvc") {
        build.warnings(true).warnings_into_errors(true);
    } else {
        build.warnings(false);
    }
    let compiler = build.get_compiler();
    if cfg!(target_env = "msvc") {
        assert!(
            compiler.is_like_msvc() && !compiler.is_like_clang_cl(),
            "Cycle 7.5 requires the native MSVC compiler selected by cc, got {}",
            compiler.path().display()
        );
    }
    build.try_compile("qcore_generated").unwrap();

    let mut compile_command = compiler.to_command();
    compile_command.current_dir(&directory).arg(&source_path);
    if compiler.is_like_msvc() {
        compile_command.arg(format!("/Fe:{}", binary_path.display()));
    } else {
        compile_command.arg("-o").arg(&binary_path);
    }
    let compile = compile_command.output().unwrap();
    assert!(
        compile.status.success(),
        "C compiler rejected generated code:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );
    let mut run_command = Command::new(&binary_path);
    if !cfg!(target_env = "msvc") {
        // LeakSanitizer cannot run under the process supervisor used by
        // some CI sandboxes; AddressSanitizer and UBSan remain enabled.
        run_command
            .env("ASAN_OPTIONS", "detect_leaks=0:halt_on_error=1")
            .env("UBSAN_OPTIONS", "halt_on_error=1");
    }
    let run = run_command.output().unwrap();
    assert!(
        run.status.success(),
        "generated code failed under runtime instrumentation:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.stdout.is_empty(), "quiet emission wrote stdout");
    assert!(run.stderr.is_empty(), "quiet emission wrote stderr");
    fs::remove_dir_all(directory).unwrap();
}

fn rust_host_target() -> String {
    let version = Command::new("rustc").arg("-vV").output().unwrap();
    assert!(version.status.success());
    std::str::from_utf8(&version.stdout)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .expect("rustc -vV must report its host target")
        .into()
}
