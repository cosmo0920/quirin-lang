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

//! Card `W48-QR-SCALAR-TYPES-R6`, item 4 (`W48-QR-TYPES-COMPOSED-STRING`): the
//! fixed-width tier on the composed string/buffer route (`source.string.v1`
//! holding a buffer).
//!
//! * A top-level conversion and value statement, a loop whose index has any
//!   integer type, and conversions and value statements inside a loop body are
//!   read by `source.buffer.v1`'s reader, decided by its runner over its typed
//!   environment under the recorded target, and emitted by its emitter with
//!   one typed environment for the whole entry; a typed name renders at its
//!   type at the top level and in a body.
//! * The borrow window is decided exactly as before: a typed statement stores
//!   no byte, so a loop writing outside a live loan is admitted and one writing
//!   inside it is refused.
//! * Native: every integer type's program under Clang `-O0` and `-O2` with
//!   `-fsanitize=undefined,integer` and plain `-O2` prints exactly the
//!   reference's streams.
//! * Negative controls: a typed name at an `i32` position of the string route,
//!   a fault folded at check time, a target width with no record, `bool` under
//!   `hex`, a typed byte index, and a body write into a live loan.

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};

const HEADER: &str = "#import std.string;\n#import std.buffer;\n#import std.fmt;\n#import std.io;\n#import std.logger;\n";

const INTEGERS: [ScalarType; 10] = [
    ScalarType::I8,
    ScalarType::I16,
    ScalarType::I32,
    ScalarType::I64,
    ScalarType::Isize,
    ScalarType::U8,
    ScalarType::U16,
    ScalarType::U32,
    ScalarType::U64,
    ScalarType::Usize,
];

fn host_target() -> RecordedTarget {
    RecordedTarget {
        pointer_width: if cfg!(target_pointer_width = "64") {
            PointerWidth::W64
        } else {
            PointerWidth::W32
        },
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "the host C compiler".into(),
    }
}

fn compile_for(
    source: &str,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("composed_typed.qr", source)],
        &CompileOptions::default(),
        target,
    )
}

fn admitted(source: &str) -> CompiledModule {
    let m = compile_for(source, Some(&host_target()))
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"));
    assert_eq!(
        m.execution_profile,
        SourceExecutionProfile::StandardStringV1
    );
    m
}

fn refused_at(source: &str, target: Option<&RecordedTarget>, code: &str, needle: &str) {
    let errors = compile_for(source, target).expect_err(source);
    assert!(
        errors
            .iter()
            .any(|d| d.code == code && d.message.contains(needle)),
        "expected {code} `{needle}` for\n{source}\n{errors:?}"
    );
}

fn lines(m: &CompiledModule) -> Vec<String> {
    m.extern_observation.clone().unwrap_or_default()
}

/// The whole tier in one composed entry at type `ty`.
fn program(ty: ScalarType) -> String {
    format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc, mut }} {{
    let b = bytes(4);
    for p in 0..4 {{
        b[p] = 97;
    }}
    let all = borrow_utf8(b);
    print all;
    let k = 3;
    let n = k as {ty};
    let m = n + n;
    let lt = n < m;
    print dec m;
    log info hex m;
    print dec lt;
    for i in 0..n {{
        let j = i * 2;
        let w = j as i64;
        print dec w;
    }}
    let head = borrow_utf8(b, 0, 2);
    for i in 0..n {{
        b[3] = 98;
        let t = i + 1;
        log info dec t;
    }}
    print head;
    let again = borrow_utf8(b);
    print again;
    drop b;
    return;
}}
"
    )
}

/// The streams an independent reading of [`program`] prints.
fn expected(ty: ScalarType) -> Vec<String> {
    let bits = ty.bits(Some(&host_target())).unwrap();
    let mut out = vec!["out: aaaa".to_owned(), "out: 6".to_owned()];
    out.push(format!(
        "err: [INFO] 0x{:0width$X}",
        6,
        width = (bits / 4) as usize
    ));
    out.push("out: true".to_owned());
    for i in 0..3 {
        out.push(format!("out: {}", i * 2));
    }
    for i in 0..3 {
        out.push(format!("err: [INFO] {}", i + 1));
    }
    out.push("out: aa".to_owned());
    out.push("out: aaab".to_owned());
    out
}

#[test]
fn the_typed_tier_is_admitted_on_the_composed_route_at_every_type() {
    for ty in INTEGERS {
        let m = admitted(&program(ty));
        assert_eq!(lines(&m), expected(ty), "{ty}");
        let c = m.c_source.clone().unwrap();
        if ty != ScalarType::I32 {
            assert!(
                c.contains(&format!("{} s_n", ty_c(ty))),
                "{ty}: the typed binding"
            );
            assert!(
                c.contains(&format!("for ({} s_i", ty_c(ty))),
                "{ty}: the typed loop"
            );
        }
        assert!(c.contains("q_scalar_add_"), "{ty}: the shared helper");
        assert_eq!(
            m.target_assumption().is_some(),
            ty.is_target_width(),
            "{ty}"
        );
    }
}

fn ty_c(ty: ScalarType) -> &'static str {
    qcore_compiler::scalar_types::c_type(ty)
}

#[test]
fn the_typed_tier_keeps_every_refusal_on_the_composed_route() {
    let base = |statements: &str| {
        format!(
            "{HEADER}fn main() -> unit effects {{ io, alloc, mut }} {{
    let b = bytes(4);
    for p in 0..4 {{
        b[p] = 97;
    }}
    let all = borrow_utf8(b);
    print all;
    let k = 3;
    let n = k as u8;
{statements}    drop b;
    return;
}}
"
        )
    };
    let target = Some(host_target());
    // A typed name at an `i32` position of the string route.
    refused_at(
        &base("    let s = all[0:n];\n    print s;\n"),
        target.as_ref(),
        "Q077",
        "`n` is a `u8` and this position takes an `i32`",
    );
    // A fault folded at check time.
    refused_at(
        &base("    let z = 200 as u8;\n    let w = z + z;\n    print dec w;\n"),
        target.as_ref(),
        "Q077",
        "Overflow",
    );
    // `bool` has no `hex` rendering.
    refused_at(
        &base("    let t = n < n;\n    log info hex t;\n"),
        target.as_ref(),
        "Q077",
        "no `hex` rendering",
    );
    // A typed byte index.
    refused_at(
        &base("    b[n] = 1;\n"),
        target.as_ref(),
        "Q077",
        "`n` is a `u8`",
    );
    // A target width with no record.
    refused_at(
        &base("    let u = k as usize;\n    print dec u;\n"),
        None,
        "Q077",
        "usize",
    );
    // The window is decided as before: a typed loop's store inside a live loan.
    let window = base(
        "    let head = borrow_utf8(b, 0, 2);\n    for i in 0..n {\n        let t = i + 1;\n        b[1] = 98;\n        print dec t;\n    }\n    print head;\n",
    );
    let errors = compile_for(&window, target.as_ref()).expect_err(&window);
    assert!(errors.iter().any(|d| d.code == "Q078"), "{errors:?}");
}

#[cfg(unix)]
#[test]
fn composed_typed_programs_run_natively_as_the_reference() {
    use std::process::Command;
    let builds: [(&str, &[&str]); 3] = [
        (
            "O0-ubsan",
            &[
                "-O0",
                "-fsanitize=undefined,integer",
                "-fno-sanitize-recover=all",
            ],
        ),
        (
            "O2-ubsan",
            &[
                "-O2",
                "-fsanitize=undefined,integer",
                "-fno-sanitize-recover=all",
            ],
        ),
        ("O2", &["-O2"]),
    ];
    let mut index = 0;
    for ty in INTEGERS {
        let m = admitted(&program(ty));
        let expected = lines(&m);
        let stdout: String = expected
            .iter()
            .filter_map(|l| l.strip_prefix("out: "))
            .map(|l| format!("{l}\n"))
            .collect();
        let stderr: String = expected
            .iter()
            .filter_map(|l| l.strip_prefix("err: "))
            .map(|l| format!("{l}\n"))
            .collect();
        let c = m.c_source.clone().unwrap();
        for (label, flags) in builds {
            index += 1;
            let dir = std::env::temp_dir().join(format!(
                "qcore-w48r6-composed-{}-{index}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("main.c"), &c).unwrap();
            let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
            let out = Command::new(compiler)
                .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
                .args(flags)
                .arg(dir.join("main.c"))
                .arg("-o")
                .arg(dir.join("main"))
                .output()
                .expect("Clang is required for this native check");
            assert!(
                out.status.success(),
                "{ty} {label}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let run = Command::new(dir.join("main"))
                .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
                .output()
                .unwrap();
            let _ = std::fs::remove_dir_all(&dir);
            assert_eq!(run.status.code(), Some(0), "{ty} {label}");
            assert_eq!(String::from_utf8_lossy(&run.stdout), stdout, "{ty} {label}");
            assert_eq!(String::from_utf8_lossy(&run.stderr), stderr, "{ty} {label}");
        }
    }
}
