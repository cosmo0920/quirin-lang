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

//! Card `W48-QR-SCALAR-TYPES-R6`, item 5: the two emitted-C defects card R5
//! measured at its base, and the unused-binding class they belong to.
//!
//! * `W48-QR-LOOP-BOUND-NAME-UNUSED-C`: an `i32` loop bound named only by its
//!   loop — and, generally, any scalar binding of `source.buffer.v1` nothing
//!   after it reads, at the top level or in a loop body, on that route and on
//!   the composed string/buffer route — emitted an unused C local, which
//!   `-Wall -Wextra -Werror` refuses. It is now discarded with `(void)`.
//! * `W48-QR-INLINE-COMPARISON-WERROR`: a value compared with itself emitted
//!   `(x == x)`, which `-Wtautological-compare` refuses under `-Werror`, on the
//!   scalar, extern and task routes. Every value-statement comparison is now
//!   the shared two-parameter function `q_scalar_CMP_T` (the extern route's
//!   case is `scalar_extern_values.rs`'s `a_self_comparison_builds_under_werror_and_runs`).
//! * Every program below builds under Clang `-O0` and `-O2` with
//!   `-fsanitize=undefined,integer` and plain `-O2`, all with `-Werror`, and
//!   prints its reference streams.

#[cfg(unix)]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget};
use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files_for_target};

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

fn admitted(source: &str) -> CompiledModule {
    compile_files_for_target(
        &[SourceFile::new("werror.qr", source)],
        &CompileOptions::default(),
        Some(&host_target()),
    )
    .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"))
}

#[cfg(unix)]
const BUILDS: [(&str, &[&str]); 3] = [
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

/// Build `c` under each build with `-Werror` and run it; every run's streams.
#[cfg(unix)]
fn run(c: &str, label: &str) -> Vec<(String, String, Option<i32>)> {
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let mut runs = Vec::new();
    for (build, flags) in BUILDS {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48r6-werror-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), c).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let mut command = Command::new(compiler);
        command
            .args([
                "-std=c11",
                "-pthread",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
            ])
            .args(flags);
        if cfg!(target_os = "macos") {
            command.arg("-I").arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/apple-threads"
            ));
        }
        let out = command
            .arg(dir.join("main.c"))
            .arg("-o")
            .arg(dir.join("main"))
            .output()
            .expect("Clang is required for this native check");
        assert!(
            out.status.success(),
            "{label} {build}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result = Command::new(dir.join("main"))
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        runs.push((
            String::from_utf8(result.stdout).unwrap(),
            String::from_utf8(result.stderr).unwrap(),
            result.status.code(),
        ));
    }
    runs
}

/// The two streams of a route's labelled reference observation.
#[cfg(unix)]
fn streams(observation: &[String]) -> (String, String) {
    let project = |stream: &str| {
        observation
            .iter()
            .filter_map(|line| line.strip_prefix(&format!("{stream}: ")))
            .map(|line| format!("{line}\n"))
            .collect::<String>()
    };
    (project("out"), project("err"))
}

#[cfg(unix)]
#[test]
fn unused_buffer_route_bindings_are_discarded_and_build() {
    for (label, source, stdout) in [
        (
            "the R5 loop bound",
            "#import std.buffer;\n#import std.fmt;\n#import std.io;\nfn main() -> unit effects { io, alloc } {\n    let b = bytes(1);\n    let n = 3;\n    for i in 0..n {\n        print dec i;\n    }\n    drop b;\n    return;\n}\n",
            "0\n1\n2\n",
        ),
        (
            "an unused top-level binding and an unused body conversion",
            "#import std.buffer;\n#import std.fmt;\n#import std.io;\nfn main() -> unit effects { io, alloc, mut } {\n    let b = bytes(4);\n    let unused = 5;\n    for i in 0..4 {\n        let j = i as u8;\n        let k = b[i];\n        b[i] = 97;\n    }\n    let x = b[0];\n    print dec x;\n    drop b;\n    return;\n}\n",
            "97\n",
        ),
        (
            "the composed route's unused body binding",
            "#import std.string;\n#import std.buffer;\n#import std.io;\nfn main() -> unit effects { io, alloc, mut } {\n    let b = bytes(4);\n    for i in 0..4 {\n        let j = i as u8;\n        b[i] = 97;\n    }\n    let all = borrow_utf8(b);\n    print all;\n    drop b;\n    return;\n}\n",
            "aaaa\n",
        ),
    ] {
        let m = admitted(source);
        let (out, err) = streams(&m.extern_observation.clone().unwrap_or_default());
        assert_eq!(out, stdout, "{label}: the reference");
        let c = m.c_source.clone().unwrap();
        assert!(c.contains("(void) s_"), "{label}: a discard");
        for (run_out, run_err, status) in run(&c, label) {
            assert_eq!(
                (run_out.as_str(), run_err.as_str(), status),
                (out.as_str(), err.as_str(), Some(0)),
                "{label}"
            );
        }
    }
}

/// A binding read later gets no discard, so a program whose every binding is
/// read keeps its bytes.
#[test]
fn a_read_binding_gets_no_discard() {
    let m = admitted(
        "#import std.buffer;\n#import std.fmt;\n#import std.io;\nfn main() -> unit effects { io, alloc } {\n    let b = bytes(1);\n    let n = 3;\n    print dec n;\n    for i in 0..3 {\n        let j = i as u8;\n        print dec j;\n    }\n    drop b;\n    return;\n}\n",
    );
    assert!(!m.c_source.unwrap().contains("(void) s_"));
}

#[cfg(unix)]
#[test]
fn a_self_comparison_builds_on_the_scalar_route() {
    let source = "#import std.fmt;\n#import std.io;\nfn same(x: u8) -> bool {\n    let c = x == x;\n    return c;\n}\nfn main() -> unit {\n    let v: u8 = 7;\n    let c = same(v);\n    let d = v < v;\n    print dec c;\n    print dec d;\n    return;\n}\n";
    let m = admitted(source);
    let c = m.c_source.clone().unwrap();
    assert!(
        c.contains("q_scalar_eq_u8(") && c.contains("q_scalar_lt_u8("),
        "{c}"
    );
    for (out, err, status) in run(&c, "scalar") {
        assert_eq!(
            (out.as_str(), err.as_str(), status),
            ("true\nfalse\n", "", Some(0))
        );
    }
}

#[cfg(unix)]
#[test]
fn a_self_comparison_builds_on_the_task_route() {
    let source = "#import std.concurrency;\nconcurrency checked;\nfn same(x: u8) -> bool {\n    let c = x == x;\n    return c;\n}\nfn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {\n    let v: u8 = 7;\n    thread scope s {\n        let h = spawn s same(copy v)?;\n        let r = join move h?;\n        let d = v < v;\n        return 0;\n    }\n}\n";
    let m = admitted(source);
    let c = diagnostic_sink::instrument_fixed(m.c_source.as_deref().unwrap());
    assert!(c.contains("q_scalar_eq_u8(") && c.contains("q_scalar_lt_u8("));
    for (out, err, status) in run(&c, "task") {
        assert!(out.contains("QCORE_TASK_RESULT 1 BOOL true\n"), "{out}");
        assert_eq!((err.as_str(), status), ("", Some(0)));
    }
}
