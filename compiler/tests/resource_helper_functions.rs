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

//! End-to-end checks for bounded ordinary resource helpers.

use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

const ROUND_TRIP: &str = include_str!("fixtures/resource_helpers/round_trip.qr");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("resource_helpers.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn refusal(source: &str) -> String {
    compile(source)
        .expect_err("fixture must refuse")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
        .join("\n")
}

fn refusal_codes(source: &str) -> Vec<String> {
    compile(source)
        .expect_err("fixture must refuse")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn allocation_processing_owned_return_use_and_release_cross_real_functions() {
    let module = compile(ROUND_TRIP).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ResourceCallsV1
    );
    assert_eq!(
        module.result_profile.custody_payloads,
        qcore_compiler::CustodyPayloadProfile::TypedResourceBytes
    );
    assert_eq!(module.checked.function_count(), 4);
    let checked = module.checked.as_resource_calls().unwrap();
    assert_eq!(checked.minimum_extents(0), Some([1].as_slice()));
    assert_eq!(checked.minimum_extents(1), Some([1, 0].as_slice()));
    assert_eq!(checked.certificate().acquisitions.len(), 1);
    assert_eq!(checked.certificate().releases.len(), 1);
    let c = module.c_source.unwrap();
    assert!(c.contains("static int32_t qcore_f0(const qcore_bytes * q_p0)"));
    assert!(c.contains("static void qcore_f1(qcore_bytes * q_p0, int32_t q_p1)"));
    assert!(c.contains("qcore_f2(q_b"));
}

#[test]
fn task_buffer_is_not_the_extent_carrying_bytes_type() {
    let source = ROUND_TRIP.replace("borrow<bytes>", "borrow<Buffer>");
    assert!(refusal(&source).contains("task Buffer is a distinct resource"));
    assert!(refusal_codes(&source).iter().any(|code| code == "Q020"));
}

#[test]
fn spaced_bytes_types_stay_on_the_resource_helper_route() {
    let source = ROUND_TRIP
        .replace("own<bytes>", "own < bytes >")
        .replace("borrow<bytes>", "borrow < bytes >")
        .replace("lend<bytes>", "lend < bytes >");
    assert_eq!(
        compile(&source).unwrap().execution_profile,
        SourceExecutionProfile::ResourceCallsV1
    );
}

#[test]
fn explicit_non_one_and_borrow_usage_grades_refuse() {
    let owned = ROUND_TRIP.replace("own<bytes> use one", "own<bytes> use zero");
    assert!(refusal(&owned).contains("requires grade one"));
    assert!(refusal_codes(&owned).iter().any(|code| code == "Q100"));
    let borrowed = ROUND_TRIP.replace("borrow<bytes>", "borrow<bytes> use zero");
    assert!(refusal(&borrowed).contains("ungraded"));
    assert!(refusal_codes(&borrowed).iter().any(|code| code == "Q100"));
    let scalar = ROUND_TRIP.replace("byte: i32", "byte: i32 use zero");
    assert!(refusal(&scalar).contains("ungraded"));
    assert!(refusal_codes(&scalar).iter().any(|code| code == "Q100"));
}

#[test]
fn equivalent_grade_one_spellings_are_accepted() {
    for grade in ["use 1", "use [1,1]"] {
        let source = ROUND_TRIP.replace("use one", grade);
        assert_eq!(
            compile(&source).unwrap().execution_profile,
            SourceExecutionProfile::ResourceCallsV1
        );
    }
}

#[test]
fn complete_headers_and_token_structural_types_are_required() {
    let cases = [
        ROUND_TRIP.replace(
            "fn main() -> unit effects { alloc, mut }",
            "fn main() -> unit effects { alloc, mut } use zero",
        ),
        ROUND_TRIP.replace(
            "fn main() -> unit effects { alloc, mut }",
            "fn main() -> unit effects { alloc, mut } effects { mut }",
        ),
        ROUND_TRIP.replace("effects { alloc, mut }", "effects { alloc mut }"),
        ROUND_TRIP.replace("effects { alloc, mut }", "effects { alloc, mut, }"),
        ROUND_TRIP.replace(
            "fn main() -> unit effects { alloc, mut }",
            "fn main() -> unit effects { alloc, mut } contract { }",
        ),
        ROUND_TRIP.replacen("borrow<bytes>", "borrow<by tes>", 1),
        ROUND_TRIP.replacen("byte: i32", "byte: i 32", 1),
        ROUND_TRIP.replacen("-> own<bytes>", "-> own<by tes>", 1),
    ];
    for source in cases {
        assert!(compile(&source).is_err(), "malformed header was accepted");
    }
}

#[test]
fn a_parameter_shadowing_a_helper_cannot_be_called_indirectly() {
    let source = r#"
#import std.buffer;
fn target(value: borrow<bytes>) -> unit { return; }
fn caller(target: i32, value: borrow<bytes>) -> unit {
    target();
    return;
}
fn main() -> unit effects { alloc } {
    let value = bytes(1);
    caller(0, borrow value);
    drop value;
    return;
}
"#;
    assert!(refusal(source).contains("shadows this helper"));
}

#[test]
fn indirect_mutation_requires_the_callers_explicit_effect_row() {
    let source = ROUND_TRIP.replace(
        "fn main() -> unit effects { alloc, mut }",
        "fn main() -> unit effects { alloc }",
    );
    assert!(refusal(&source).contains("derived helper effects"));
    assert!(refusal_codes(&source).iter().any(|code| code == "Q057"));
}

#[test]
fn call_site_extent_obligation_is_checked() {
    for extent in [0, 4097, i32::MAX] {
        let source = ROUND_TRIP.replace("bytes(1)", &format!("bytes({extent})"));
        assert!(refusal_codes(&source).iter().any(|code| code == "Q053"));
    }
    let source = ROUND_TRIP.replace("value[0] = byte", "value[1] = byte");
    assert!(refusal(&source).contains("requires extent 2"));
    assert!(refusal_codes(&source).iter().any(|code| code == "Q053"));
}

#[test]
fn moved_caller_name_is_stale() {
    let source = ROUND_TRIP.replace(
        "let output = process(move input);",
        "let output = process(move input);\n    drop input;",
    );
    assert!(refusal(&source).contains("drop after owned move"));
    assert!(refusal_codes(&source).iter().any(|code| code == "Q054"));
}

#[test]
fn same_owner_cannot_be_lent_and_borrowed_in_one_call() {
    let source = ROUND_TRIP
        .replace(
            "fn fill(value: lend<bytes>, byte: i32)",
            "fn fill(value: lend<bytes>, also: borrow<bytes>, byte: i32)",
        )
        .replace(
            "fill(lend(value), seen);",
            "fill(lend(value), borrow value, seen);",
        );
    assert!(refusal(&source).contains("incompatible owned/borrow modes"));
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn emitted_real_helpers_execute_under_address_and_undefined_sanitizers() {
    use std::process::Command;
    let module = compile(ROUND_TRIP).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "qcore-resource-helpers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let generated = directory.join("generated.c");
    let binary = directory.join("generated");
    let source = module
        .c_source
        .unwrap()
        .replace("int main(void)", "int checked_entry(void)")
        + "\nint main(void) {\n\
               qcore_bytes input = qcore_bytes_new((size_t) 1u);\n\
               qcore_bytes_store(input.data, input.len, (size_t) 0u, INT32_C(17));\n\
               qcore_f1(&input, INT32_C(42));\n\
               qcore_bytes output = qcore_f2(input);\n\
               input.data = NULL; input.len = 0u;\n\
               int ok = output.len == (size_t) 1u &&\n\
                   qcore_bytes_load(output.data, output.len, (size_t) 0u) == INT32_C(42);\n\
               qcore_bytes_drop(&output);\n\
               return ok ? 0 : 87;\n\
           }\n";
    std::fs::write(&generated, source).unwrap();
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into());
    let output = Command::new(compiler)
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
        ])
        .arg(&generated)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(&binary)
        .env(
            "ASAN_OPTIONS",
            if cfg!(target_os = "macos") {
                // Apple's sanitizer runtime does not support LeakSanitizer.
                "detect_leaks=0:halt_on_error=1"
            } else {
                "detect_leaks=1:halt_on_error=1"
            },
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    std::fs::remove_dir_all(directory).unwrap();
}
