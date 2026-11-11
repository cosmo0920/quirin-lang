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

use super::*;
use crate::{ExecutionOutcome, SourceOp, TraceEvent, emit_c11};
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn source(operations: Vec<SourceOp>) -> SourceProgram {
    SourceProgram::new("subject", vec![1, 2], ["Failure".to_owned()], operations).unwrap()
}

#[test]
fn validates_all_three_observations() {
    let source = source(vec![
        SourceOp::Acquire {
            resource: 1,
            succeeds: true,
            error: "Failure".into(),
        },
        SourceOp::TransferToResult(1),
        SourceOp::Return(7),
    ]);
    let cfg = lower_cleanup_cfg(&source).unwrap();
    let certificate = validate_translation(&source, &cfg, "OK 7\nA 1\nM 1\nP 1\n").unwrap();
    assert_eq!(certificate.source, certificate.cfg);
    assert_eq!(certificate.cfg, certificate.c11);
}

#[test]
fn rejects_a_target_observation_that_drops_cleanup() {
    let source = source(vec![
        SourceOp::Acquire {
            resource: 1,
            succeeds: true,
            error: "Failure".into(),
        },
        SourceOp::Check {
            passes: false,
            error: "Failure".into(),
        },
        SourceOp::Drop(1),
        SourceOp::Return(0),
    ]);
    let cfg = lower_cleanup_cfg(&source).unwrap();
    let error = validate_translation(&source, &cfg, "ERR 1\nA 1\n").unwrap_err();
    assert!(matches!(
        error,
        TranslationValidationError::CfgC11Mismatch { .. }
    ));
}

#[test]
fn exhaustive_small_corpus_preserves_source_cfg_semantics() {
    for first_succeeds in [false, true] {
        for second_succeeds in [false, true] {
            for check_passes in [false, true] {
                let source = source(vec![
                    SourceOp::Acquire {
                        resource: 1,
                        succeeds: first_succeeds,
                        error: "Failure".into(),
                    },
                    SourceOp::Acquire {
                        resource: 2,
                        succeeds: second_succeeds,
                        error: "Failure".into(),
                    },
                    SourceOp::Check {
                        passes: check_passes,
                        error: "Failure".into(),
                    },
                    SourceOp::Drop(2),
                    SourceOp::Drop(1),
                    SourceOp::Return(11),
                ]);
                let cfg = lower_cleanup_cfg(&source).unwrap();
                validate_cfg_boundary(&source, &cfg).unwrap();
                assert_eq!(
                    interpret_source(&source).unwrap(),
                    interpret_cfg(&cfg).unwrap()
                );
            }
        }
    }
}

#[test]
fn certificate_retains_error_trace() {
    let source = source(vec![
        SourceOp::Acquire {
            resource: 1,
            succeeds: true,
            error: "Failure".into(),
        },
        SourceOp::Acquire {
            resource: 2,
            succeeds: false,
            error: "Failure".into(),
        },
        SourceOp::Drop(2),
        SourceOp::Drop(1),
        SourceOp::Return(0),
    ]);
    let cfg = lower_cleanup_cfg(&source).unwrap();
    let certificate = validate_translation(&source, &cfg, "ERR 1\nA 1\nR 1\n").unwrap();
    assert_eq!(
        certificate.source.outcome,
        ExecutionOutcome::Error {
            name: "Failure".into(),
            tag: 1
        }
    );
    assert_eq!(
        certificate.source.trace,
        vec![TraceEvent::Acquire(1), TraceEvent::Release(1)]
    );
}

#[test]
fn exhaustive_emitted_c_corpus_passes_available_native_sanitizers() {
    let msvc = cfg!(target_env = "msvc");
    let msvc_compiler = if msvc {
        let rust_target = rust_host_target();
        let mut build = cc::Build::new();
        build
            .host(&rust_target)
            .target(&rust_target)
            .opt_level(0)
            .debug(false)
            .cargo_metadata(false);
        let compiler = build.get_compiler();
        assert!(
            compiler.is_like_msvc() && !compiler.is_like_clang_cl(),
            "native Windows sanitizer validation requires MSVC, got {}",
            compiler.path().display()
        );
        Some(compiler)
    } else {
        if Command::new("clang").arg("--version").output().is_err() {
            return;
        }
        None
    };
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "qcore-cycle13-sanitizers-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    let c_path = directory.join("program.c");
    let executable = directory.join(if msvc { "program.exe" } else { "program" });

    for first_succeeds in [false, true] {
        for second_succeeds in [false, true] {
            for check_passes in [false, true] {
                let source = source(vec![
                    SourceOp::Acquire {
                        resource: 1,
                        succeeds: first_succeeds,
                        error: "Failure".into(),
                    },
                    SourceOp::Acquire {
                        resource: 2,
                        succeeds: second_succeeds,
                        error: "Failure".into(),
                    },
                    SourceOp::Check {
                        passes: check_passes,
                        error: "Failure".into(),
                    },
                    SourceOp::Drop(2),
                    SourceOp::Drop(1),
                    SourceOp::Return(11),
                ]);
                let cfg = lower_cleanup_cfg(&source).unwrap();
                fs::write(&c_path, emit_c11(&cfg).unwrap()).unwrap();
                let compilation = if let Some(compiler) = &msvc_compiler {
                    let mut command = compiler.to_command();
                    command
                        .current_dir(&directory)
                        .args([
                            "/nologo",
                            "/std:c11",
                            "/TC",
                            "/W4",
                            "/WX",
                            "/Zi",
                            "/fsanitize=address",
                        ])
                        .arg(&c_path)
                        .arg(format!("/Fe:{}", executable.display()))
                        .status()
                        .unwrap()
                } else {
                    Command::new("clang")
                        .args([
                            "-std=c11",
                            "-Wall",
                            "-Wextra",
                            "-Werror",
                            "-fsanitize=address,undefined",
                            "-fno-omit-frame-pointer",
                        ])
                        .arg(&c_path)
                        .arg("-o")
                        .arg(&executable)
                        .status()
                        .unwrap()
                };
                assert!(compilation.success());
                let mut run_command = Command::new(&executable);
                if let Some(compiler) = &msvc_compiler {
                    // `cc` discovers MSVC through Visual Studio and retains the
                    // environment that locates its dynamic ASan runtime. The
                    // generated executable needs the same PATH when launched.
                    run_command.envs(compiler.env().iter().cloned());
                }
                let output = run_command
                    // LeakSanitizer cannot run in ptrace-based CI sandboxes;
                    // address and undefined-behavior checks remain enabled.
                    .env("ASAN_OPTIONS", "detect_leaks=0:halt_on_error=1")
                    .env("UBSAN_OPTIONS", "halt_on_error=1")
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "sanitizer failure ({status}): {stderr}",
                    status = output.status,
                    stderr = String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    interpret_cfg(&cfg).unwrap(),
                    interpret_source(&source).unwrap()
                );
                assert!(output.stdout.is_empty(), "quiet emission wrote stdout");
                assert!(output.stderr.is_empty(), "quiet emission wrote stderr");
            }
        }
    }
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
