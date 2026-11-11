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

//! Card `W48-QR-SCALAR-TYPES-R2`, item 0 (`W48-SEQUENTIAL-UNCHECKED-PARAMETER-TYPES`):
//! every route checks every annotation it admits.
//!
//! * The sequential route read no parameter annotation: `x: Widget`,
//!   `f: fn() -> i32`, `x: own<Widget>` and `x: usize` with no recorded target
//!   were admitted. It now checks each against the fixed-width scalar table,
//!   with `isize`/`usize` needing the recorded target, and refuses the rest.
//! * Six routes model the entry only and admitted any other function unread —
//!   its parameters, result, locals and body. Each now refuses such a function
//!   at its own "models the entry only" code.
use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};

fn record() -> RecordedTarget {
    RecordedTarget {
        pointer_width: PointerWidth::W64,
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "the host C compiler".into(),
    }
}
fn compile(
    source: &str,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("unread.qr", source)],
        &CompileOptions::default(),
        target,
    )
}

#[test]
fn the_sequential_route_checks_every_parameter_annotation() {
    let entry = "fn main() -> i32 { return 0; }";
    for (parameter, target, admitted) in [
        ("x: i32", None, true),
        ("x: u8", None, true),
        ("x: bool", None, true),
        ("x: usize", Some(record()), true),
        ("x: isize", Some(record()), true),
        ("x: usize", None, false),
        ("x: isize", None, false),
        ("x: Widget", None, false),
        ("f: fn() -> i32", None, false),
        ("x: own<Widget>", None, false),
        ("x: borrow<Widget>", None, false),
    ] {
        let source = format!("fn helper({parameter}) -> i32 {{ return 0; }}\n{entry}");
        let result = compile(&source, target.as_ref());
        if admitted {
            let m = result.unwrap_or_else(|e| panic!("{source}\n{e:?}"));
            assert_eq!(
                m.execution_profile,
                SourceExecutionProfile::SequentialCleanupV1
            );
        } else {
            let e = result.expect_err(&source);
            assert_eq!(e.len(), 1, "{e:?}");
            assert_eq!(e[0].code, "Q020", "{source}");
            assert!(
                e[0].message
                    .contains(&format!("parameter `{}`", &parameter[..1])),
                "{e:?}"
            );
        }
    }
}

/// The two probes of `docs/reports/remaining-v03` that recorded this as the
/// named boundary B-1 are now refused: the corrections this item makes.
#[test]
fn the_b1_probes_are_corrected() {
    for (source, written) in [
        (
            include_str!("../../docs/reports/remaining-v03/probes/S13_unknown_parameter_type.qr"),
            "`Widget`",
        ),
        (
            include_str!(
                "../../docs/reports/remaining-v03/probes/X14_function_typed_param_helper.qr"
            ),
            "`fn() -> i32`",
        ),
    ] {
        let e = compile(source, None).expect_err(source);
        assert_eq!(e[0].code, "Q020");
        assert!(e[0].message.contains(written), "{e:?}");
    }
}

/// Card `W48-QR-USABLE-RECORDS`: a record module holding no buffer reaches
/// `source.scalar-calls.v1` with a second function, which reads and checks
/// the helper — a valid one is admitted and each ill-typed one is refused at
/// `Q020` — so no helper is admitted unread there either.
#[test]
fn a_record_module_with_a_helper_checks_the_helper() {
    let example = include_str!("../../examples/records/point.qr");
    let source = format!(
        "{}\nfn qprobe_helper(x: i32) -> i32 {{ return 0; }}\n",
        example.trim_end()
    );
    compile(&source, None).unwrap_or_else(|e| panic!("{e:?}"));
    for helper in [
        "fn qprobe_helper(x: Widget) -> Widget { return 0; }",
        "fn qprobe_helper() -> i32 { let q: usize = 7; return 0; }",
        "fn qprobe_helper() -> usize { return 0; }",
    ] {
        let source = format!("{}\n{helper}\n", example.trim_end());
        let e = compile(&source, None).expect_err(&source);
        assert_eq!(e.len(), 1, "{e:?}");
        assert_eq!(e[0].code, "Q020", "{helper}\n{e:?}");
        assert!(e[0].message.contains("source.scalar-calls.v1"), "{e:?}");
    }
}

/// Each entry-only route refuses a helper it would have admitted unread, at
/// that route's own code, whatever the helper's annotations; the example alone
/// is still admitted.
#[test]
fn the_entry_only_routes_refuse_a_function_they_do_not_model() {
    for (example, code) in [
        (include_str!("../../examples/stdio/renderings.qr"), "Q030"),
        // Card `W48-QR-USABLE-RECORDS`: `records/point.qr` holds no buffer, so
        // with a second function it reaches `source.scalar-calls.v1`, which
        // checks the helper — see the test below — and is no longer an
        // entry-only row.
        (include_str!("../../examples/loops/buffer_copy.qr"), "Q051"),
        (
            include_str!("../../examples/strings/status_line.qr"),
            "Q068",
        ),
        (include_str!("../../examples/sockets/udp_echo.qr"), "Q080"),
        (
            include_str!("../../examples/events/socket_server.qr"),
            "Q080",
        ),
        (include_str!("../../examples/http/exchange.qr"), "Q090"),
        (include_str!("../../examples/events/self_pipe.qr"), "Q058"),
    ] {
        compile(example, None).unwrap_or_else(|e| panic!("{e:?}"));
        for helper in [
            "fn qprobe_helper(x: i32) -> i32 { return 0; }",
            "fn qprobe_helper(x: Widget) -> Widget { return 0; }",
            "fn qprobe_helper() -> i32 { let q: usize = 7; return 0; }",
            "fn qprobe_helper() -> usize { return 0; }",
        ] {
            let source = format!("{}\n{helper}\n", example.trim_end());
            let e = compile(&source, None).expect_err(&source);
            assert_eq!(e.len(), 1, "{e:?}");
            // An annotated `let` is scalar-call syntax, so beside `std.io`
            // lines the module reaches the scalar route, which checks the
            // helper and refuses its `usize` for want of a recorded target.
            if code == "Q030" && helper.contains("let q: usize") {
                assert_eq!(e[0].code, "Q020", "{e:?}");
                assert!(e[0].message.contains("recorded target"), "{e:?}");
                continue;
            }
            assert_eq!(e[0].code, code, "{helper}\n{e:?}");
            assert!(
                e[0].message.contains("models the entry only")
                    && e[0].message.contains("`qprobe_helper`"),
                "{e:?}"
            );
        }
    }
}

/// A module with `std.io` lines, scalar-call syntax and a helper reaches the
/// scalar route, which checks the helper — its annotation included.
#[test]
fn a_std_io_module_with_a_helper_reaches_the_route_that_checks_it() {
    let good = "#import std.fmt;\n#import std.io;\nfn h(x: u8) -> u8 { return x; }\nfn main() -> unit effects { io } { let x: u8 = 5; let y = h(x); print dec y; return; }";
    let m = compile(good, None).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(m.execution_profile, SourceExecutionProfile::ScalarCallsV1);
    let bad = good.replace("fn h(x: u8) -> u8", "fn h(x: usize) -> u8");
    let e = compile(&bad, None).expect_err(&bad);
    assert!(e[0].message.contains("recorded target"), "{e:?}");
}
