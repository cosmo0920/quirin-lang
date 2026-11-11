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

//! Card `W48-QR-USABLE-RECORDS` (card 2 of the `.qr` language track): records
//! usable as values.
//!
//! * **Field writes** on `source.buffer.v1` and `source.ffi.v1`, a scalar
//!   field's value checked against its type, an owned field refilled only
//!   into a hole, a write (and a read) through a released record refused.
//! * **Nested records** on `source.buffer.v1`, read and written through a
//!   path, over the ownership IR's generalised owner path.
//! * **Records across calls**: records of scalars as `source.scalar-calls.v1`
//!   parameters and results, by value.
//! * **Imported structs across `.qr` functions** on `source.ffi.v1`, written
//!   and passed by value, every result still under the unsafe-FFI
//!   assumption.
//!
//! Negative controls first, then admissions, then the published examples'
//! native runs. No test here writes output of its own: native results are
//! compared through the programs' own lines and exit status.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
use qcore_core::ffi::UNSAFE_FFI_ASSUMPTION;
use qcore_core::std_io::Stream;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn host_target() -> RecordedTarget {
    let (compiler, triple) = qcore_c_gateway::ffi_layout::running_compiler()
        .expect("Clang is required for the FFI rows");
    RecordedTarget {
        pointer_width: if cfg!(target_pointer_width = "64") {
            PointerWidth::W64
        } else {
            PointerWidth::W32
        },
        triple,
        compiler,
    }
}

fn compile_files(
    files: &[(PathBuf, String)],
    contracts: Option<BTreeMap<String, String>>,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let sources: Vec<SourceFile> = files
        .iter()
        .map(|(path, source)| SourceFile::new(path.display().to_string(), source.clone()))
        .collect();
    compile_files_for_target(
        &sources,
        &CompileOptions {
            extern_contracts: contracts,
            // Card `W48-QRLIB-LINKAGE`: a file binding `qr_point` resolves its
            // calls through the manifest generated from the built library.
            native_libraries: files
                .iter()
                .any(|(_, source)| source.contains("extern library"))
                .then(|| point_library().selection()),
            ..CompileOptions::default()
        },
        target,
    )
}

#[path = "../../tests/support/qrlib.rs"]
mod qrlib;

/// `examples/ffi/qr_point.c` built once into a static library, and its `.qrlib`
/// generated from the archive (card `W48-QRLIB-LINKAGE`).
fn point_library() -> &'static qrlib::Generated {
    static LIBRARY: std::sync::OnceLock<qrlib::Generated> = std::sync::OnceLock::new();
    LIBRARY.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("qcore-w48-records-lib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = qrlib::static_library(
            &dir,
            "qr_point",
            &[ffi_dir().join("qr_point.c")],
            &[&ffi_dir()],
        );
        qrlib::manifest(
            "qr_point",
            qcore_core::native_library::LinkageKind::Static,
            &archive,
            Some(&ffi_dir().join("qr_point.h")),
            &host_target(),
            &point_contracts(),
        )
    })
}

fn compile(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files(
        &[(PathBuf::from("usable_records.qr"), source.to_owned())],
        None,
        None,
    )
}

fn codes(result: Result<CompiledModule, Vec<FileDiagnostic>>) -> Vec<(String, String)> {
    match result {
        Ok(module) => panic!(
            "expected a refusal, admitted with profile {:?}",
            module.execution_profile
        ),
        Err(errors) => errors
            .into_iter()
            .map(|error| (error.code, error.message))
            .collect(),
    }
}

fn refused_with(result: Result<CompiledModule, Vec<FileDiagnostic>>, code: &str, needle: &str) {
    let found = codes(result);
    assert!(
        found
            .iter()
            .any(|(found, message)| found == code && message.contains(needle)),
        "expected {code} mentioning `{needle}`, got {found:#?}"
    );
}

/// The two-stream projection of a route that publishes `out: …`/`err: …`.
fn projections(module: &CompiledModule) -> (String, String) {
    let observation = module
        .extern_observation
        .as_deref()
        .expect("this route publishes a two-stream observation");
    let pick = |prefix: &str| -> String {
        observation
            .iter()
            .filter_map(|line| line.strip_prefix(prefix))
            .map(|line| format!("{line}\n"))
            .collect()
    };
    (pick("out: "), pick("err: "))
}

/// The fenced block after `<!-- MARKER -->` in a page.
fn documented(page: &str, marker: &str) -> String {
    let page = page.replace("\r\n", "\n");
    let start = page
        .find(&format!("<!-- {marker} -->"))
        .unwrap_or_else(|| panic!("the page has no `{marker}` block"));
    let rest = &page[start..];
    let open = rest.find("```text\n").expect("a text block") + "```text\n".len();
    let close = rest[open..].find("```").expect("a closed block");
    rest[open..open + close].to_owned()
}

const BUFFER_HEAD: &str = "#import std.buffer;\n#import std.fmt;\n#import std.io;\n";

fn buffer_program(types: &str, body: &str) -> String {
    format!(
        "{BUFFER_HEAD}{types}\nfn main() -> unit effects {{ io, alloc, mut }} {{\n{body}    return;\n}}\n"
    )
}

// ============================================================ buffer route
// ------------------------------------------------------ negative controls

/// A write of the wrong type: a name of another type (`Q052`), a literal out
/// of the field's range (`Q053`), a `bool` at an integer field (`Q053`).
#[test]
fn a_field_write_of_the_wrong_type_is_refused() {
    let types = "type P { x: i32, flag: bool, small: u8 }";
    refused_with(
        compile(&buffer_program(
            types,
            "    let p = P { x: 1, flag: true, small: 2 };\n    let b = 1 == 1;\n    p.x = b;\n",
        )),
        "Q052",
        "declared `i32` and is supplied a `bool`",
    );
    refused_with(
        compile(&buffer_program(
            types,
            "    let p = P { x: 1, flag: true, small: 2 };\n    p.small = 256;\n",
        )),
        "Q053",
        "`256` is not a value of `u8`",
    );
    refused_with(
        compile(&buffer_program(
            types,
            "    let p = P { x: 1, flag: true, small: 2 };\n    p.x = false;\n",
        )),
        "Q053",
        "not a value of `i32`",
    );
}

/// A write — and, as the card's correction, a read — through a record whose
/// owner was released is a use after drop, decided by the IR.
#[test]
fn a_field_access_through_a_released_record_is_a_use_after_drop() {
    let types = "type F { tag: i32, buf: bytes }";
    for access in ["    f.tag = 3;\n", "    let t = f.tag;\n"] {
        refused_with(
            compile(&buffer_program(
                types,
                &format!(
                    "    let b = bytes(2);\n    let f = F {{ tag: 1, buf: b }};\n    drop f;\n{access}"
                ),
            )),
            "Q054",
            "use of `f` after drop",
        );
    }
}

/// A whole write over an owned field that still holds its buffer would lose
/// that buffer's release, and refuses.
#[test]
fn a_refill_over_a_live_owned_field_is_refused() {
    refused_with(
        compile(&buffer_program(
            "type F { buf: bytes }",
            "    let b = bytes(2);\n    let f = F { buf: b };\n    let c = bytes(2);\n    f.buf = c;\n    drop f;\n",
        )),
        "Q054",
        "still holds its buffer",
    );
}

/// A path continuing through a field that is not a record.
#[test]
fn a_nested_path_through_a_non_record_is_refused() {
    let types = "type I { a: i32 }\ntype O { inner: I, n: i32 }";
    refused_with(
        compile(&buffer_program(
            types,
            "    let i = I { a: 1 };\n    let o = O { inner: i, n: 2 };\n    let v = o.n.a;\n",
        )),
        "Q052",
        "`o.n` is declared `i32`, which has no fields",
    );
    refused_with(
        compile(&buffer_program(
            types,
            "    let i = I { a: 1 };\n    let o = O { inner: i, n: 2 };\n    o.n.a = 3;\n",
        )),
        "Q052",
        "which has no fields",
    );
    refused_with(
        compile(&buffer_program(
            types,
            "    let i = I { a: 1 };\n    let o = O { inner: i, n: 2 };\n    let v = o.inner.b;\n",
        )),
        "Q052",
        "declares no field `b`",
    );
}

/// A record field supplied something other than a record of its type, a
/// record that contains itself, a whole sub-record write, and a whole read of
/// a sub-record that owns.
#[test]
fn the_nested_record_boundaries_refuse_by_name() {
    let types = "type I { a: i32 }\ntype J { a: i32 }\ntype O { inner: I, n: i32 }";
    refused_with(
        compile(&buffer_program(
            types,
            "    let j = J { a: 1 };\n    let o = O { inner: j, n: 2 };\n",
        )),
        "Q052",
        "declared `I` and is supplied a `J`",
    );
    refused_with(
        compile(&buffer_program(
            types,
            "    let o = O { inner: 1, n: 2 };\n",
        )),
        "Q052",
        "takes a `I` record",
    );
    refused_with(
        compile(&buffer_program(
            "type A { b: B }\ntype B { c: C }\ntype C { a: A }",
            "",
        )),
        "Q051",
        "contain itself",
    );
    refused_with(
        compile(&buffer_program(
            types,
            "    let i = I { a: 1 };\n    let k = I { a: 2 };\n    let o = O { inner: i, n: 2 };\n    o.inner = k;\n",
        )),
        "Q051",
        "W48-QR-RECORD-FIELD-WHOLE-WRITE",
    );
    refused_with(
        compile(&buffer_program(
            "type I { buf: bytes }\ntype O { inner: I }",
            "    let b = bytes(2);\n    let i = I { buf: b };\n    let o = O { inner: i };\n    let j = o.inner;\n    drop o;\n",
        )),
        "Q051",
        "W48-QR-NESTED-OWNED-SUBRECORD-MOVE",
    );
}

/// The owned-field window refusals cover a record that owns through a
/// nested record: beside a lend, and inside a loop body.
#[test]
fn a_record_that_owns_through_a_nested_record_keeps_the_window_refusals() {
    let types = "type I { buf: bytes }\ntype O { inner: I }";
    refused_with(
        compile(&buffer_program(
            types,
            "    let b = bytes(2);\n    let i = I { buf: b };\n    let o = O { inner: i };\n    let c = bytes(4);\n    let l = lend(c, 0, 2);\n    l[0] = 1;\n    drop c;\n    drop o;\n",
        )),
        "Q051",
        "OWNED-FIELD-LEND-WINDOW",
    );
    refused_with(
        compile(&buffer_program(
            "type F { tag: i32, buf: bytes }",
            "    let b = bytes(2);\n    let f = F { tag: 1, buf: b };\n    for i in 0..2 {\n        f.tag = i;\n    }\n    drop f;\n",
        )),
        "Q051",
        "OWNED-FIELD-LOOP-BODY",
    );
}

/// A moved sub-record is a use after move from then on, and a record whose
/// nested leaf is never released leaks by its whole path.
#[test]
fn nested_custody_is_the_irs() {
    let types = "type I { buf: bytes }\ntype O { inner: I }";
    refused_with(
        compile(&buffer_program(
            types,
            "    let b = bytes(2);\n    let i = I { buf: b };\n    let o = O { inner: i };\n    i.buf[0] = 1;\n    drop o;\n",
        )),
        "Q054",
        "use of `i.buf` after move",
    );
    refused_with(
        compile(&buffer_program(
            types,
            "    let b = bytes(2);\n    let i = I { buf: b };\n    let o = O { inner: i };\n",
        )),
        "Q054",
        "o.inner.buf",
    );
}

// ------------------------------------------------------------- admissions

/// A write of a scalar field, of every kind of value its type takes.
#[test]
fn a_field_write_replaces_that_field_and_nothing_else() {
    let module = compile(&buffer_program(
        "type P { x: i32, y: u8, z: bool }",
        "    let p = P { x: 1, y: 2, z: false };\n    let v = 9 as u8;\n    p.y = v;\n    p.z = true;\n    p.x = -7;\n    let a = p.x;\n    let b = p.y;\n    let c = p.z;\n    a |> print dec;\n    b |> print dec;\n    c |> print dec;\n",
    ))
    .unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    let (out, _) = projections(&module);
    assert_eq!(out, "-7\n9\ntrue\n");
    let c = module.c_source.as_deref().unwrap();
    assert!(c.contains("r_p.f_x = INT32_C(-7);"), "{c}");
}

/// A record of scalars written in a loop body keeps the body's writes.
#[test]
fn a_record_of_scalars_is_written_in_a_loop_body() {
    let module = compile(&buffer_program(
        "type Acc { total: i32 }",
        "    let acc = Acc { total: 0 };\n    let b = bytes(4);\n    for i in 0..3 {\n        acc.total = i;\n        b[i] = i;\n    }\n    let t = acc.total;\n    t |> print dec;\n    drop b;\n",
    ))
    .unwrap();
    let (out, _) = projections(&module);
    assert_eq!(out, "2\n");
}

// =================================================== scalar-calls route

const IO_HEAD: &str = "#import std.fmt;\n#import std.io;\n";

/// A record parameter whose custody mode is not the route's — by value — and
/// a record that owns crossing a call are refused.
#[test]
fn a_record_parameter_takes_the_record_by_value_only() {
    let types = "type P { x: i32 }\n";
    for parameter in ["p: own<P>", "p: borrow<P>", "p: ptr<const, P>"] {
        refused_with(
            compile(&format!(
                "{IO_HEAD}{types}fn get({parameter}) -> i32 {{\n    return 1;\n}}\nfn main() -> unit effects {{ io }} {{\n    let p = P {{ x: 1 }};\n    let v = get(p);\n    print dec v;\n    return;\n}}\n"
            )),
            "Q020",
            "source.scalar-calls.v1",
        );
    }
    refused_with(
        compile(&format!(
            "{IO_HEAD}{types}fn get(p: P) -> i32 {{\n    return p.x;\n}}\nfn main() -> unit effects {{ io }} {{\n    let p = P {{ x: 1 }};\n    let v = get(move p);\n    print dec v;\n    return;\n}}\n"
        )),
        "Q020",
        "source.scalar-calls.v1",
    );
    refused_with(
        compile(&format!(
            "{IO_HEAD}type F {{ buf: bytes }}\nfn get(f: F) -> i32 {{\n    return 1;\n}}\nfn main() -> unit effects {{ io }} {{\n    let v = get2();\n    return;\n}}\nfn get2() -> i32 {{\n    return 2;\n}}\n"
        )),
        "Q020",
        "W48-QR-OWNED-RECORD-ACROSS-CALL",
    );
}

/// A record of one type where another is taken, a record in arithmetic or a
/// line, an incomplete construction, a field write, and a throwing function
/// returning a record are refused.
#[test]
fn the_scalar_calls_record_boundaries_refuse() {
    let types = "type P { x: i32, y: i32 }\ntype Q { x: i32, y: i32 }\n";
    let program = |functions: &str, body: &str| {
        format!(
            "{IO_HEAD}{types}{functions}fn main() -> unit effects {{ io }} {{\n{body}    return;\n}}\n"
        )
    };
    let getter = "fn get(p: P) -> i32 {\n    return p.x;\n}\n";
    refused_with(
        compile(&program(
            getter,
            "    let q = Q { x: 1, y: 2 };\n    let v = get(q);\n    print dec v;\n",
        )),
        "Q020",
        "expected P expression, found Q",
    );
    refused_with(
        compile(&program(
            getter,
            "    let p = P { x: 1, y: 2 };\n    let v = p + 1;\n    print dec v;\n",
        )),
        "Q020",
        "a record has no arithmetic",
    );
    refused_with(
        compile(&program(
            getter,
            "    let p = P { x: 1, y: 2 };\n    print dec p;\n    let v = get(p);\n    print dec v;\n",
        )),
        "Q020",
        "renders one scalar",
    );
    refused_with(
        compile(&program(
            getter,
            "    let p = P { x: 1 };\n    let v = get(p);\n    print dec v;\n",
        )),
        "Q020",
        "supplies no `y`",
    );
    refused_with(
        compile(&program(
            getter,
            "    let p = P { x: 1, y: 2 };\n    p.x = 3;\n    let v = get(p);\n    print dec v;\n",
        )),
        "Q020",
        "W48-QR-SCALAR-CALLS-FIELD-WRITE",
    );
    refused_with(
        compile(&program(
            "fn make() -> P effects { throws<ArithmeticError> } {\n    return P { x: 1, y: 2 };\n}\n",
            "    let v = 1;\n    print dec v;\n",
        )),
        "Q020",
        "W48-QR-RECORD-THROWS-RESULT",
    );
}

/// A module declaring a record with one function keeps the buffer route —
/// the route-selection change opens only for modules with a second function.
#[test]
fn a_one_function_record_module_keeps_the_buffer_route() {
    let module = compile(&format!(
        "{IO_HEAD}type P {{ x: i32 }}\nfn main() -> unit effects {{ io }} {{\n    let p = P {{ x: 4 }};\n    let v = p.x;\n    print dec v;\n    return;\n}}\n"
    ))
    .unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
}

const RECORD_CALLS: &str = include_str!("../../examples/scalar/record_calls.qr");
const SCALAR_PAGE: &str = include_str!("../../examples/scalar/README.md");

/// The published example: records built, passed, returned and read, the
/// caller's record unchanged by a callee's new one; the page's transcript is
/// the reference execution's.
#[test]
fn the_record_calls_example_checks_and_matches_its_page() {
    let module = compile_files(
        &[(
            PathBuf::from("examples/scalar/record_calls.qr"),
            RECORD_CALLS.to_owned(),
        )],
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ScalarCallsV1
    );
    let (out, err) = projections(&module);
    assert_eq!(out, "9\n14\n25\n0x07\n10\n");
    assert!(err.is_empty());
    assert_eq!(
        out,
        documented(SCALAR_PAGE, "transcript: record_calls stdout")
    );
    let c = module.c_source.as_deref().unwrap();
    assert!(c.contains("struct qcore_record_Point {"), "{c}");
    assert!(c.contains("struct qcore_record_Point f_lo;"), "{c}");
}

// ============================================================ FFI route

fn ffi_dir() -> PathBuf {
    root().join("examples/ffi")
}

fn point_contracts() -> BTreeMap<String, String> {
    BTreeMap::from([(
        "examples/ffi/contracts/qr_point.qv".to_owned(),
        std::fs::read_to_string(ffi_dir().join("contracts/qr_point.qv")).unwrap(),
    )])
}

const FFI_HEAD: &str = "#import std.ffi.unsafe;\n#import std.extern;\n#import std.fmt;\n#import std.io;\nextern library qr_point;\n";
const POINT_IMPORT: &str = "struct! {\n    type Point = c\"struct qr_point\" from \"qr_point.h\";\n    fields { x: i32, y: i32, weight: u16 }\n}\n";

/// An FFI program beside the example's header.
fn ffi(
    declarations: &str,
    body: &str,
    effects: &str,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let source = format!(
        "{FFI_HEAD}{POINT_IMPORT}{declarations}fn main() -> unit effects {{ {effects} }} {{\n{body}    return;\n}}\n"
    );
    compile_files(
        &[(ffi_dir().join("probe.qr"), source)],
        Some(point_contracts()),
        Some(&host_target()),
    )
}

const CONS: &str = "    let p = Point { x: 1, y: 2, weight: 3 };\n";

/// A write of the wrong type, of an opaque member, and of a name that is no
/// member; a write without `mut` in the row.
#[test]
fn an_ffi_field_write_is_checked_and_opaque_members_are_never_written() {
    refused_with(
        ffi("", &format!("{CONS}    p.weight = 70000;\n"), "io, mut"),
        "Q053",
        "not a `u16` value",
    );
    refused_with(
        ffi(
            "",
            &format!("{CONS}    let w = p.weight;\n    p.x = w;\n"),
            "io, mut",
        ),
        "Q052",
        "`w` is a `u16`",
    );
    refused_with(
        ffi("", &format!("{CONS}    p.note = 1;\n"), "io, mut"),
        "Q051",
        "leaves opaque",
    );
    refused_with(
        ffi("", &format!("{CONS}    p.z = 1;\n"), "io, mut"),
        "Q051",
        "`z` is not a field of `Point`",
    );
    refused_with(
        ffi("", &format!("{CONS}    p.x = 5;\n"), "io"),
        "Q029",
        "performs `Mut`",
    );
}

/// A helper nothing calls, a helper calling itself, a helper taking a
/// pointer, and a declared outcome on a `.qr` call.
#[test]
fn an_ffi_helper_is_checked_at_every_call_or_refused() {
    refused_with(
        ffi(
            "fn idle(p: Point) -> i32 {\n    let x = p.x;\n    return x;\n}\n",
            CONS,
            "io",
        ),
        "Q026",
        "never called from the entry",
    );
    refused_with(
        ffi(
            "fn loop_x(p: Point) -> i32 {\n    let x = loop_x(p);\n    return x;\n}\n",
            &format!("{CONS}    let v = loop_x(p);\n"),
            "io",
        ),
        "Q026",
        "calls itself",
    );
    refused_with(
        ffi(
            "fn peek(p: ptr<const, Point>) -> i32 {\n    return 1;\n}\n",
            &format!("{CONS}    let v = peek(p);\n"),
            "io",
        ),
        "Q026",
        "by value",
    );
    refused_with(
        ffi(
            "fn get(p: Point) -> i32 {\n    let x = p.x;\n    return x;\n}\n",
            &format!("{CONS}    let v = get(p) succeeds 1;\n"),
            "io",
        ),
        "Q025",
        "computed rather than declared",
    );
}

/// An imported record reaching a file that is not an FFI file: a function in
/// a file of the bundle that does not import `std.ffi.unsafe` is refused, so
/// no code is checked under a mark its file never took.
#[test]
fn an_ffi_record_never_reaches_a_file_without_the_mark() {
    let import = format!("#import std.ffi.unsafe;\n{POINT_IMPORT}");
    let user = "#import std.fmt;\n#import std.io;\nfn main() -> unit effects { io } {\n    let p = Point { x: 1, y: 2, weight: 3 };\n    let x = p.x;\n    print dec x;\n    return;\n}\n";
    refused_with(
        compile_files(
            &[
                (ffi_dir().join("imports.qr"), import),
                (ffi_dir().join("user.qr"), user.to_owned()),
            ],
            None,
            Some(&host_target()),
        ),
        "Q032",
        "its own file does not import `std.ffi.unsafe`",
    );
}

const RECORD_PASSING: &str = include_str!("../../examples/ffi/record_passing.qr");
const FFI_PAGE: &str = include_str!("../../examples/ffi/README.md");

fn record_passing() -> CompiledModule {
    compile_files(
        &[(
            ffi_dir().join("record_passing.qr"),
            RECORD_PASSING.to_owned(),
        )],
        Some(point_contracts()),
        Some(&host_target()),
    )
    .unwrap_or_else(|errors| panic!("record_passing.qr does not check: {errors:#?}"))
}

/// The example is admitted under the unsafe-FFI assumption — never plain Safe
/// — its certificate names the helpers and the writes, and its page
/// transcript is the reference observation.
#[test]
fn the_record_passing_example_checks_under_the_unsafe_ffi_assumption() {
    let module = record_passing();
    assert_eq!(module.execution_profile, SourceExecutionProfile::FfiV1);
    let verdict = module.published_verdict().expect("a published verdict");
    assert!(!verdict.is_plain_safe());
    assert!(verdict.holds_under(UNSAFE_FFI_ASSUMPTION));
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    assert_eq!(
        checked.certificate.helpers,
        [
            "fn with_weight(p: Point, weight: u16) -> Point",
            "fn show_x(p: Point) -> unit"
        ]
    );
    assert_eq!(
        checked
            .certificate
            .writes
            .iter()
            .map(|write| (write.record.as_str(), write.field.as_str(), write.value))
            .collect::<Vec<_>>(),
        [("p", "weight", Some(5)), ("q", "x", Some(6))]
    );
    let stdout = qcore_core::ffi::reference_stream(&checked.certificate, Stream::Out);
    assert_eq!(stdout, "1\n5\n50\n7\n");
    assert_eq!(
        stdout,
        documented(FFI_PAGE, "transcript: record_passing stdout")
    );
    let c = module.c_source.as_deref().unwrap();
    assert!(
        c.contains(
            "static struct qr_point qr_fn_with_weight(struct qr_point r_p, uint16_t v_weight)"
        ),
        "{c}"
    );
    assert!(c.contains("r_p.weight = v_weight;"), "{c}");
    assert!(!c.contains(".note"), "the opaque member is never touched");
    assert!(qcore_core::io_emit::direct_stdio_calls(c).is_empty());
}

// ================================================================ native

#[cfg(unix)]
mod native {
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    pub const BUILDS: [(&str, &[&str]); 3] = [
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

    pub struct Run {
        pub stdout: String,
        pub stderr: String,
        pub status: Option<i32>,
    }

    /// Build `c` (with `library`, and `include` on the path, when given) and
    /// run it.
    pub fn build_and_run(
        c: &str,
        include: Option<&Path>,
        library: Option<&Path>,
        flags: &[&str],
    ) -> Result<Run, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-records-native-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), c).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let mut command = Command::new(compiler);
        command
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
            .args(flags);
        if let Some(include) = include {
            command.arg("-I").arg(include);
        }
        command.arg(dir.join("main.c"));
        if let Some(library) = library {
            command.arg(library);
        }
        let out = command
            .arg("-o")
            .arg(dir.join("main"))
            .output()
            .expect("Clang is required for this native check");
        if !out.status.success() {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(String::from_utf8_lossy(&out.stderr).into_owned());
        }
        let run = Command::new(dir.join("main"))
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        Ok(Run {
            stdout: String::from_utf8(run.stdout).unwrap(),
            stderr: String::from_utf8(run.stderr).unwrap(),
            status: run.status.code(),
        })
    }
}

/// One native case: the program, its module, its reference stdout and
/// stderr, and the header directory and C library it links, when it has one.
#[cfg(unix)]
type NativeCase<'a> = (
    &'a str,
    &'a CompiledModule,
    String,
    String,
    Option<PathBuf>,
    Option<PathBuf>,
);

/// The three published examples, three builds each: stdout and stderr equal
/// the reference observation, exit 0, no sanitizer report. The summary goes
/// to the report's `native-records.json` when `QCORE_W48_RECORDS_NATIVE_SUMMARY`
/// names a file.
#[cfg(unix)]
#[test]
fn the_published_examples_run_clean_under_three_builds() {
    let nested = compile_files(
        &[(
            root().join("examples/records/nested_writes.qr"),
            std::fs::read_to_string(root().join("examples/records/nested_writes.qr")).unwrap(),
        )],
        None,
        None,
    )
    .unwrap();
    let calls = compile_files(
        &[(
            root().join("examples/scalar/record_calls.qr"),
            RECORD_CALLS.to_owned(),
        )],
        None,
        None,
    )
    .unwrap();
    let passing = record_passing();
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &passing.checked else {
        panic!("not the FFI route");
    };
    let ffi_out = qcore_core::ffi::reference_stream(&checked.certificate, Stream::Out);
    let ffi_err = qcore_core::ffi::reference_stream(&checked.certificate, Stream::Err);
    let (nested_out, nested_err) = projections(&nested);
    let (calls_out, calls_err) = projections(&calls);
    let library = point_library().link_plan().input;
    let cases: [NativeCase; 3] = [
        (
            "examples/records/nested_writes.qr",
            &nested,
            nested_out,
            nested_err,
            None,
            None,
        ),
        (
            "examples/scalar/record_calls.qr",
            &calls,
            calls_out,
            calls_err,
            None,
            None,
        ),
        (
            "examples/ffi/record_passing.qr",
            &passing,
            ffi_out,
            ffi_err,
            Some(ffi_dir()),
            Some(library),
        ),
    ];
    let mut summary = Vec::new();
    for (program, module, stdout, stderr, include, library) in cases {
        let c = module.c_source.as_deref().unwrap();
        for (label, flags) in native::BUILDS {
            let run = native::build_and_run(c, include.as_deref(), library.as_deref(), flags)
                .unwrap_or_else(|error| {
                    panic!("{program} {label}: the emitted C does not build: {error}")
                });
            assert_eq!(run.status, Some(0), "{program} {label}: {}", run.stderr);
            assert_eq!(run.stdout, stdout, "{program} {label}");
            assert_eq!(
                run.stderr, stderr,
                "{program} {label}: a sanitizer report or a changed line"
            );
            summary.push(format!(
                "{{\"program\": \"{program}\", \"build\": \"{label}\", \"exit\": 0, \"stdout_lines\": {}, \"stderr_lines\": {}, \"sanitizer_reports\": 0}}",
                stdout.lines().count(),
                stderr.lines().count()
            ));
        }
    }
    if let Ok(path) = std::env::var("QCORE_W48_RECORDS_NATIVE_SUMMARY") {
        let (compiler, triple) = qcore_c_gateway::ffi_layout::running_compiler().unwrap();
        std::fs::write(
            path,
            format!(
                "{{\n \"compiler\": \"{compiler}\",\n \"triple\": \"{triple}\",\n \"flags\": \"-std=c11 -Wall -Wextra -Werror -pedantic\",\n \"ffi_library\": \"examples/ffi/qr_point.c\",\n \"runs\": [\n  {}\n ]\n}}\n",
                summary.join(",\n  ")
            ),
        )
        .unwrap();
    }
}

/// The nested example also runs clean under AddressSanitizer and
/// LeakSanitizer: the refilled buffer and the moved-out one are each released
/// once.
#[cfg(target_os = "linux")]
#[test]
fn the_nested_example_releases_every_buffer_once_under_asan() {
    let nested = compile_files(
        &[(
            root().join("examples/records/nested_writes.qr"),
            std::fs::read_to_string(root().join("examples/records/nested_writes.qr")).unwrap(),
        )],
        None,
        None,
    )
    .unwrap();
    let (stdout, _) = projections(&nested);
    let run = native::build_and_run(
        nested.c_source.as_deref().unwrap(),
        None,
        None,
        &[
            "-O0",
            "-g",
            "-fsanitize=address,undefined",
            "-fno-omit-frame-pointer",
        ],
    )
    .unwrap();
    assert_eq!(run.status, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, stdout);
    assert!(run.stderr.is_empty(), "{}", run.stderr);
}

const RECORDS_PAGE: &str = include_str!("../../examples/records/README.md");

/// The records page publishes the nested example's transcript.
#[test]
fn the_records_page_publishes_the_nested_transcript() {
    let nested = compile_files(
        &[(
            root().join("examples/records/nested_writes.qr"),
            std::fs::read_to_string(root().join("examples/records/nested_writes.qr")).unwrap(),
        )],
        None,
        None,
    )
    .unwrap();
    let (out, err) = projections(&nested);
    assert_eq!(out, "10\n5\n200\n81\n67\n");
    assert!(err.is_empty());
    assert_eq!(
        out,
        documented(RECORDS_PAGE, "transcript: nested_writes stdout")
    );
}
