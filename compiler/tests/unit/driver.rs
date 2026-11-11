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
use crate::{ExecutionOutcome, TraceEvent};
use qcore_core::usage::Usage;
use qcore_frontend::frontend::ExecutableResultType;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn compile(source: &str) -> CompiledModule {
    compile_files(
        &[SourceFile::new("main.qc", source)],
        &CompileOptions::default(),
    )
    .unwrap()
}

#[test]
fn compiles_values_and_maps_emitted_source() {
    let module = compile("fn main() -> i32 { return 42; }");
    assert_eq!(
        module.reference_observation.as_ref().unwrap().outcome,
        ExecutionOutcome::Ok {
            value: 42,
            payload: Vec::new()
        }
    );
    assert!(
        module
            .c_source
            .as_ref()
            .unwrap()
            .contains("#line 1 \"main.qc\"")
    );
    assert_eq!(module.source_map[0].source.path, "main.qc");
}

#[test]
fn combines_module_files_and_checks_calls() {
    let files = [
        SourceFile::new("leaf.qc", "fn leaf() -> i32 { return 1; }"),
        SourceFile::new("main.qc", "fn main() -> i32 { leaf(); return 2; }"),
    ];
    let module = compile_files(&files, &CompileOptions::default()).unwrap();
    assert_eq!(module.checked.function_count(), 2);
    assert_eq!(module.entry.as_deref(), Some("main"));
}

#[test]
fn rejects_observable_calls_that_the_reduced_c_cfg_cannot_encode() {
    let files = [
        SourceFile::new(
            "leaf.qc",
            "error Failure { Failed; } fn leaf() -> i32 { qcore_check(false, Failure); return 1; }",
        ),
        SourceFile::new("main.qc", "fn main() -> i32 { leaf(); return 2; }"),
    ];
    let errors = compile_files(&files, &CompileOptions::default()).unwrap_err();
    assert_eq!(errors[0].code, "Q500");
    assert_eq!(errors[0].location.path, "main.qc");
}

#[test]
fn synthesized_quantitative_bound_covers_the_resource_trace() {
    let module = compile(
        "error Failure { Failed; } fn main() -> i32 { let a = qcore_acquire(3, true, Failure); let b = qcore_acquire(4, true, Failure); drop b; drop a; return 0; }",
    );
    let summary = &module.checked.as_sequential().unwrap().functions
        [&module.checked.as_sequential().unwrap().source.globals[module.entry.as_ref().unwrap()]]
        .summary;
    assert_eq!(summary.flow.peak().get("resources"), 2);
    assert_eq!(summary.flow.total().get("allocations"), 2);
}

#[test]
fn preserves_error_cleanup_and_ownership_transfer_observations() {
    let failed = compile(
        "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(7, true, Failure); qcore_check(false, Failure); drop r; return 1; }",
    );
    assert_eq!(
        failed.reference_observation.as_ref().unwrap().outcome,
        ExecutionOutcome::Error {
            name: "Failure".into(),
            tag: 1
        }
    );
    assert_eq!(
        failed.reference_observation.as_ref().unwrap().trace,
        vec![TraceEvent::Acquire(7), TraceEvent::Release(7)]
    );
    let moved = compile(
        "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(9, true, Failure); move r; return -3; }",
    );
    assert_eq!(
        moved.result_profile.scalar_results["main"],
        ExecutableResultType::I32
    );
    assert_eq!(
        moved.result_profile.custody_payloads,
        CustodyPayloadProfile::SeparateIdentityLedger
    );
    assert_eq!(
        moved.reference_observation.as_ref().unwrap().outcome,
        ExecutionOutcome::Ok {
            value: -3,
            payload: vec![9]
        }
    );
    assert_eq!(
        moved.reference_observation.as_ref().unwrap().trace,
        vec![TraceEvent::Acquire(9), TraceEvent::Transfer(9)]
    );
}

/// Card `IF-STATEMENT` moved this test's *fixture*, deliberately, and left its
/// claim exactly where it was. The claim is that an unsupported structured
/// construct reports `Q500` at the owning file; the fixture used to be
/// `if (true) { return 1; }`, which `source.branch.v1` now reads. `while` is
/// still outside the executable subset and still reports `Q500`, so the pin is
/// unweakened. What the old fixture does now is pinned separately by
/// `an_early_return_from_a_branch_arm_refuses` in `compiler/tests/branch_statement.rs`.
#[test]
fn reports_unsupported_source_at_the_owning_file() {
    let errors = compile_files(
        &[SourceFile::new(
            "bad.qc",
            "fn main() -> i32 { while (true) { return 1; } }",
        )],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].location.path, "bad.qc");
    assert_eq!(errors[0].code, "Q500");
}

#[test]
fn checks_parameter_usage_even_for_uncalled_functions() {
    for grade in ["one", "relevant", "2", "[1, inf]"] {
        let source = format!(
            "fn discard(x: i32 use {grade}) -> i32 {{ return 0; }}\nfn main() -> i32 {{ return 0; }}"
        );
        let errors = compile_files(
            &[SourceFile::new("usage.qc", source)],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q100", "{grade}");
        assert_eq!(errors[0].location.path, "usage.qc");
        assert!(errors[0].message.contains("`x`"));
    }
    for grade in ["zero", "affine", "many", "[0, 3]"] {
        let source = format!(
            "fn discard(x: i32 use {grade}) -> i32 {{ return 0; }}\nfn main() -> i32 {{ return 0; }}"
        );
        assert_eq!(compile(&source).checked.function_count(), 2);
    }
}

#[test]
fn a_shadowing_local_does_not_discharge_a_parameter_usage_obligation() {
    let source = "error Failure { Failed; } fn discard(x: i32 use one) -> i32 { let x = qcore_acquire(1, true, Failure); drop x; return 0; } fn main() -> i32 { return 0; }";
    let errors = compile_files(
        &[SourceFile::new("usage.qc", source)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q100");
}

#[test]
fn records_source_demands_and_rejects_unsupported_local_contracts() {
    let module = compile(
        "error Failure { Failed; } fn main() -> i32 { let x = qcore_acquire(1, true, Failure); drop x; return 0; }",
    );
    let main = &module.checked.as_sequential().unwrap().functions
        [&module.checked.as_sequential().unwrap().source.globals["main"]];
    assert!(
        main.usage
            .iter()
            .any(|(name, demand)| name.ends_with(":x") && demand == Usage::ONE)
    );
    let failed = compile(
        "error Failure { Failed; } fn main() -> i32 { let x = qcore_acquire(1, true, Failure); qcore_check(false, Failure); drop x; return 0; }",
    );
    let main = &failed.checked.as_sequential().unwrap().functions
        [&failed.checked.as_sequential().unwrap().source.globals["main"]];
    assert!(main.usage.iter().next().is_none());
    let errors = compile_files(&[SourceFile::new("usage.qc", "error Failure { Failed; } fn main() -> i32 { let x: own<File> use zero = qcore_acquire(1, true, Failure); drop x; return 0; }")], &CompileOptions::default()).unwrap_err();
    assert_eq!(errors[0].code, "Q100");
}

#[test]
fn call_continuations_have_conservative_usage_lower_bounds() {
    let module = compile(
        "error Failure { Failed; } fn leaf() -> i32 { return 0; } fn main() -> i32 { let x = qcore_acquire(1, true, Failure); leaf(); drop x; return 0; }",
    );
    let main = &module.checked.as_sequential().unwrap().functions
        [&module.checked.as_sequential().unwrap().source.globals["main"]];
    assert!(
        main.usage
            .iter()
            .any(|(name, demand)| name.ends_with(":x") && demand == Usage::AFFINE)
    );
    assert!(
        main.usage
            .iter()
            .any(|(name, demand)| name.starts_with("$function:") && demand == Usage::ONE)
    );
}

#[test]
fn emitted_c_matches_reference_for_value_error_trace_and_transfer() {
    let version = Command::new("rustc").arg("-vV").output().unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap();
    let compiler = cc::Build::new()
        .host(host)
        .target(host)
        .opt_level(0)
        .debug(false)
        .cargo_metadata(false)
        .warnings(true)
        .warnings_into_errors(true)
        .get_compiler();
    for source in [
        "fn main() -> i32 { return 17; }",
        "fn main() -> i32 { return -2147483648; }",
        "fn main() -> i32 { return 2147483647; }",
        "fn main() -> unit { return; }",
        "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(1, true, Failure); qcore_check(false, Failure); drop r; return 0; }",
        "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(2, true, Failure); move r; return 5; }",
    ] {
        let module = compile(source);
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("qcore-cycle12-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let c_path = directory.join("program.c");
        let exe_path = directory.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        fs::write(&c_path, module.c_source.as_ref().unwrap()).unwrap();
        let mut command = compiler.to_command();
        command.current_dir(&directory).arg(&c_path);
        if compiler.is_like_msvc() {
            command
                .args(["/std:c11", "/TC"])
                .arg(format!("/Fe:{}", exe_path.display()));
        } else {
            command.arg("-std=c11").arg("-o").arg(&exe_path);
        }
        let status = command.status().unwrap();
        assert!(status.success());
        let output = Command::new(&exe_path).output().unwrap();
        assert!(output.status.success());
        assert!(module.reference_observation.is_some());
        assert!(output.stdout.is_empty(), "quiet emission wrote stdout");
        assert!(output.stderr.is_empty(), "quiet emission wrote stderr");
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn refuses_unchecked_source_contracts_in_every_input_file() {
    let errors = compile_files(
        &[
            SourceFile::new(
                "bound.qcore",
                "fn leaf() -> i32 contract { quant normal wall_clock <= 5; } { return 0; }",
            ),
            SourceFile::new("main.qcore", "fn main() -> i32 { return 0; }"),
        ],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(errors.iter().any(|error| error.code == "Q021"
        && error.location.path == "bound.qcore"
        && error.message.contains("refusing unchecked contract")));
    compile("fn main() -> i32 contract {} { return 0; }");
}

/// D01: a diagnostic whose span starts at byte 0 of a non-first input file was
/// attributed to the *previous* file, at a line one past that file's end,
/// because `locate` tested an inclusive bound against an exclusive
/// `FileRange::end` and `combine_files` starts the next file at exactly that
/// offset when the previous one ends with a newline. Rejection, code and exit
/// status were always correct; only the named artifact was wrong.
#[test]
fn attributes_diagnostics_at_a_file_boundary_to_the_offending_file() {
    for trailing in ["\n", ""] {
        let errors = compile_files(
            &[
                SourceFile::new(
                    "first.qcore",
                    format!("fn main() -> i32 {{ return 0; }}{trailing}"),
                ),
                SourceFile::new("second.qcore", "fn main() -> i32 { return 1; }"),
            ],
            &CompileOptions::default(),
        )
        .unwrap_err();
        let duplicate = errors
            .iter()
            .find(|error| error.code == "Q010")
            .unwrap_or_else(|| panic!("{errors:?}"));
        assert_eq!(duplicate.location.path, "second.qcore");
        assert_eq!((duplicate.location.line, duplicate.location.column), (1, 1));
        assert_eq!(
            duplicate.location.line_source,
            "fn main() -> i32 { return 1; }"
        );
    }

    // A span strictly inside the first file still resolves to the first file,
    // and an unconditional declaration failure in the second file is no longer
    // blamed on whichever file was listed first.
    let errors = compile_files(
        &[
            SourceFile::new("dimensions.qcore", "dimension Bytes : Idx + Ordered;\n"),
            SourceFile::new("axis.qcore", "quant axis bad : credit<Missing>;\n"),
            SourceFile::new("main.qcore", "fn main() -> i32 { return 0; }\n"),
        ],
        &CompileOptions::default(),
    )
    .unwrap_err();
    let index = errors
        .iter()
        .find(|error| error.code == "Q016")
        .unwrap_or_else(|| panic!("{errors:?}"));
    assert_eq!(index.location.path, "axis.qcore");
    assert_eq!((index.location.line, index.location.column), (1, 1));
}

#[test]
fn resolved_scope_requires_checked_profile_and_guard_still_requires_lowering() {
    for (body, expected_code) in [
        ("thread scope s { }", "Q600"),
        ("let m = 0; with lock m as g { }", "Q500"),
    ] {
        let source = format!("#import std.concurrency; fn main() -> i32 {{ {body} return 0; }}");
        let errors = compile_files(
            &[SourceFile::new("main.qcore", source)],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert!(
            errors.iter().any(|error| error.code == expected_code),
            "{errors:?}"
        );
        assert!(
            !errors.iter().any(|error| error.code == "Q013"),
            "{errors:?}"
        );
    }
}

#[test]
fn review_r04_checks_declared_results_before_lowering() {
    for source in [
        "fn main() -> bool { return 2; }",
        "fn main() -> Missing { return 0; }",
        "fn main() -> own<File> { return 0; }",
        "fn main() -> result<i32, Failure> { return 0; }",
        "fn main() -> i32 { return; }",
        "fn main() -> unit { return 0; }",
        "fn main() -> i32 {}",
        "fn main() -> i32 { return 0; return; }",
        "fn leaf() -> bool { return 1; } fn main() -> i32 { leaf(); return 0; }",
        "fn leaf() -> unit { return; } fn main() -> i32 { return leaf(); }",
    ] {
        assert!(
            compile_files(
                &[SourceFile::new("result.qcore", source)],
                &CompileOptions::default()
            )
            .is_err(),
            "accepted {source}"
        );
    }
    compile("fn main() -> unit { return; }");
    compile("fn leaf() -> unit { return; } fn main() -> i32 { leaf(); return 0; }");
}

#[test]
fn review_r07_driver_signed_literal_boundaries() {
    for value in ["0", "-0", "2147483647", "-2147483648"] {
        let module = compile(&format!("fn main() -> i32 {{ return {value}; }}"));
        assert_eq!(
            module.reference_observation.unwrap().outcome,
            ExecutionOutcome::Ok {
                value: value.parse().unwrap(),
                payload: Vec::new()
            }
        );
    }
    for value in [
        "2147483648",
        "-2147483649",
        "999999999999999999999999999999999999999999",
    ] {
        assert!(
            compile_files(
                &[SourceFile::new(
                    "literal.qcore",
                    format!("fn main() -> i32 {{ return {value}; }}")
                )],
                &CompileOptions::default()
            )
            .is_err()
        );
    }
}

#[test]
fn review_r04_does_not_emit_unencoded_callee_payloads() {
    let source = "error Failure { Failed; } fn leaf() -> i32 { let r = qcore_acquire(8, true, Failure); move r; return 0; } fn main() -> unit { leaf(); return; }";
    let errors = compile_files(
        &[SourceFile::new("payload.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| matches!(error.code.as_str(), "Q200" | "Q500")),
        "{errors:?}"
    );
}
