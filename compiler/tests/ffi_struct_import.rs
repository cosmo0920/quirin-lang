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

//! Card `W48-QR-C-STRUCT-IMPORT` (card 1 of the `.qr` language track):
//! `#import std.ffi.unsafe;`, `struct! { … }` checked against the recorded
//! compiler, `[| quant #c_ffi |]` contracts, and the file-wide trust mark.
//!
//! * The published example `examples/ffi/header_sum.qr` against the real C
//!   library beside it (`qr_header.h`, `qr_header.c`), checked, emitted, built
//!   by the recorded compiler at `-O0` and `-O2` under UBSan and at plain
//!   `-O2`, all `-Wall -Wextra -Werror -pedantic`, and run: both streams equal
//!   the reference observation, exit 0, no sanitizer report.
//! * A layout changed after checking: the emitted C no longer builds.
//! * Negative controls, one row each.
//!
//! No test here writes output of its own: native results are compared through
//! the program's own `print`/`log` lines and its exit status.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
use qcore_core::ffi::{FfiAssumption, UNSAFE_FFI_ASSUMPTION};
use qcore_core::std_io::Stream;

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/ffi")
}

/// The recorded target naming the Clang that runs here: the layout is read
/// only by the recorded compiler, so the tests record the one they build with.
fn host_target() -> RecordedTarget {
    let (compiler, triple) = qcore_c_gateway::ffi_layout::running_compiler()
        .expect("Clang is required for the FFI route");
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

fn example_contracts() -> BTreeMap<String, String> {
    let path = examples().join("contracts/qr_header.qv");
    BTreeMap::from([(
        "examples/ffi/contracts/qr_header.qv".to_owned(),
        std::fs::read_to_string(path).unwrap(),
    )])
}

#[path = "../../tests/support/qrlib.rs"]
mod qrlib;

/// Card `W48-QRLIB-LINKAGE`: `qr_header.c` built once into a static library,
/// and its `.qrlib` generated from the archive, binding the example contracts.
fn example_library() -> &'static qrlib::Generated {
    static LIBRARY: std::sync::OnceLock<qrlib::Generated> = std::sync::OnceLock::new();
    LIBRARY.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("qcore-w48-ffi-lib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = qrlib::static_library(
            &dir,
            "qr_header",
            &[examples().join("qr_header.c")],
            &[&examples()],
        );
        qrlib::manifest(
            "qr_header",
            qcore_core::native_library::LinkageKind::Static,
            &archive,
            Some(&examples().join("qr_header.h")),
            &host_target(),
            &example_contracts(),
        )
    })
}

fn compile_at(
    path: &Path,
    source: &str,
    contracts: Option<BTreeMap<String, String>>,
    target: Option<&RecordedTarget>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new(path.display().to_string(), source)],
        &CompileOptions {
            extern_contracts: contracts,
            // Selected for a file that binds the library, and only then, so a
            // row about another route selects exactly what it did before.
            native_libraries: source
                .contains("extern library")
                .then(|| example_library().selection()),
            ..CompileOptions::default()
        },
        target,
    )
}

fn example() -> CompiledModule {
    let path = examples().join("header_sum.qr");
    let source = std::fs::read_to_string(&path).unwrap();
    compile_at(
        &path,
        &source,
        Some(example_contracts()),
        Some(&host_target()),
    )
    .unwrap_or_else(|errors| panic!("examples/ffi/header_sum.qr does not check: {errors:#?}"))
}

/// A scratch directory holding the example header under `qr_header.h`.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str, header: &str) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-ffi-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("qr_header.h"), header).unwrap();
        Self(dir)
    }

    fn compile(
        &self,
        source: &str,
        contracts: Option<BTreeMap<String, String>>,
    ) -> Result<CompiledModule, Vec<FileDiagnostic>> {
        compile_at(
            &self.0.join("main.qr"),
            source,
            contracts,
            Some(&host_target()),
        )
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn example_header() -> String {
    std::fs::read_to_string(examples().join("qr_header.h")).unwrap()
}

const HEAD: &str = "#import std.ffi.unsafe;\n#import std.extern;\n#import std.fmt;\n#import std.io;\n#import std.logger;\nextern library qr_header;\n";

const IMPORT: &str = "struct! { type Header = c\"struct qr_header\" from \"qr_header.h\"; fields { len: u32, flags: u8, id: i64, valid: bool } }\n";

fn program(import: &str, declarations: &str, body: &str) -> String {
    format!(
        "{HEAD}{import}{declarations}fn main() -> unit effects {{ io }} {{\n{body}    return;\n}}\n"
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

// ---------------------------------------------------------------- admission

#[test]
fn the_example_checks_under_the_unsafe_ffi_assumption() {
    let module = example();
    assert_eq!(module.execution_profile, SourceExecutionProfile::FfiV1);
    let verdict = module
        .published_verdict()
        .expect("the FFI route publishes a verdict");
    assert!(!verdict.is_plain_safe());
    assert!(verdict.holds_under(UNSAFE_FFI_ASSUMPTION));
    assert!(
        verdict.render().starts_with("Safe under the "),
        "{}",
        verdict.render()
    );
    let Some(FfiAssumption::UnsafeFfi { structs, functions }) = module.ffi_assumptions().first()
    else {
        panic!("the trust mark is the first assumption");
    };
    assert_eq!(structs.len(), 1);
    assert!(
        structs[0].starts_with("`Header = struct qr_header` from \"qr_header.h\""),
        "{structs:?}"
    );
    assert_eq!(functions.len(), 2, "{functions:?}");
    assert!(functions.iter().any(|f| f.starts_with("`qr_header_total`")));
    assert!(functions.iter().any(|f| f.starts_with("`qr_header_make`")));
    assert!(
        module
            .ffi_assumptions()
            .iter()
            .any(|assumption| matches!(assumption, FfiAssumption::RecordedCompiler { .. }))
    );
}

#[test]
fn the_certificate_publishes_the_recorded_layout() {
    let module = example();
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    let record = &checked.certificate.structs[0];
    let listed: Vec<(&str, u64)> = record
        .listed
        .iter()
        .map(|field| (field.name.as_str(), field.offset))
        .collect();
    if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        assert_eq!(listed, [("len", 0), ("flags", 4), ("id", 8), ("valid", 16)]);
        assert_eq!(record.facts.size, 32);
        assert_eq!(record.facts.align, 8);
    }
    assert_eq!(record.opaque, ["reserved", "label"]);
    assert_eq!(record.facts.header_sha256.len(), 64);
    assert_eq!(checked.certificate.functions.len(), 2);
    assert!(
        checked
            .certificate
            .functions
            .iter()
            .all(|function| function.marked)
    );
}

#[test]
fn the_emitted_c_includes_the_real_header_and_asserts_the_layout() {
    let module = example();
    let c = module.c_source.as_deref().unwrap();
    assert!(c.contains("#include \"qr_header.h\"\n"), "{c}");
    assert!(
        !c.contains("struct qr_header {"),
        "the struct is never re-declared"
    );
    assert!(
        c.contains("_Static_assert(sizeof(struct qr_header) == "),
        "{c}"
    );
    for field in ["len", "flags", "id", "valid"] {
        assert!(
            c.contains(&format!(
                "_Static_assert(offsetof(struct qr_header, {field}) == "
            )),
            "{field}"
        );
        assert!(
            c.contains(&format!("((struct qr_header *) 0)->{field}, ")),
            "{field}"
        );
    }
    for field in ["reserved", "label"] {
        assert!(
            !c.contains(&format!(".{field}")),
            "opaque `{field}` is never touched"
        );
    }
    assert!(
        c.contains("uint32_t qr_header_total(const struct qr_header *header);"),
        "{c}"
    );
    assert!(
        c.contains("struct qr_header qr_header_make(uint32_t len, uint8_t flags);"),
        "{c}"
    );
    assert!(qcore_core::io_emit::direct_stdio_calls(c).is_empty());
}

// ---------------------------------------------------------------- native

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

    /// Build `c` with the library `library_c`, the header directory on the
    /// include path, and run it.
    pub fn build_and_run(
        c: &str,
        include: &Path,
        library_c: &Path,
        flags: &[&str],
    ) -> Result<Run, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-ffi-native-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), c).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let out = Command::new(compiler)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
            .args(flags)
            .arg("-I")
            .arg(include)
            .arg(dir.join("main.c"))
            .arg(library_c)
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

/// The published example against the real C library, three builds. The
/// summary folds into the report's `native-ffi.json` when
/// `QCORE_W48_FFI_NATIVE_SUMMARY` names a file.
#[cfg(unix)]
#[test]
fn the_example_runs_against_the_real_library_under_three_builds() {
    let module = example();
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    let stdout = qcore_core::ffi::reference_stream(&checked.certificate, Stream::Out);
    let stderr = qcore_core::ffi::reference_stream(&checked.certificate, Stream::Err);
    assert_eq!(stdout, "40\n0x02\n42\ntrue\n");
    assert_eq!(stderr, "[INFO] 30\n");
    let c = module.c_source.as_deref().unwrap();
    let mut summary = Vec::new();
    for (label, flags) in native::BUILDS {
        let run =
            native::build_and_run(c, &examples(), &example_library().link_plan().input, flags)
                .unwrap_or_else(|error| panic!("{label}: the emitted C does not build: {error}"));
        assert_eq!(run.status, Some(0), "{label}: {}", run.stderr);
        assert_eq!(run.stdout, stdout, "{label}");
        assert_eq!(
            run.stderr, stderr,
            "{label}: a sanitizer report or a changed line"
        );
        summary.push(format!(
            "{{\"build\": \"{label}\", \"exit\": 0, \"stdout_lines\": {}, \"stderr_lines\": {}, \"sanitizer_reports\": 0}}",
            stdout.lines().count(),
            stderr.lines().count()
        ));
    }
    if let Ok(path) = std::env::var("QCORE_W48_FFI_NATIVE_SUMMARY") {
        let (compiler, triple) = qcore_c_gateway::ffi_layout::running_compiler().unwrap();
        std::fs::write(
            path,
            format!(
                "{{\n \"program\": \"examples/ffi/header_sum.qr\",\n \"library\": \"examples/ffi/qr_header.c\",\n \"compiler\": \"{compiler}\",\n \"triple\": \"{triple}\",\n \"flags\": \"-std=c11 -Wall -Wextra -Werror -pedantic\",\n \"builds\": [\n  {}\n ]\n}}\n",
                summary.join(",\n  ")
            ),
        )
        .unwrap();
    }
}

/// A declared outcome the library does not produce: the emitted C exits 70
/// before writing the line that would have published it.
#[cfg(unix)]
#[test]
fn a_false_declared_outcome_exits_70_before_its_line() {
    let path = examples().join("header_sum.qr");
    let source = std::fs::read_to_string(&path)
        .unwrap()
        .replace("succeeds 42", "succeeds 43");
    let module = compile_at(
        &path,
        &source,
        Some(example_contracts()),
        Some(&host_target()),
    )
    .expect("a declared outcome is a claim the C checks, not the checker");
    let run = native::build_and_run(
        module.c_source.as_deref().unwrap(),
        &examples(),
        &example_library().link_plan().input,
        &[
            "-O0",
            "-fsanitize=undefined,integer",
            "-fno-sanitize-recover=all",
        ],
    )
    .unwrap();
    assert_eq!(run.status, Some(70));
    assert_eq!(run.stdout, "40\n0x02\n");
}

/// The emitted C is checked against one header; a header whose layout
/// changed afterwards does not build with it.
#[cfg(unix)]
#[test]
fn a_layout_changed_after_checking_fails_the_build() {
    let module = example();
    let c = module.c_source.as_deref().unwrap();
    for (label, from, to) in [
        (
            "offset",
            "    uint8_t flags;\n    char reserved[3];",
            "    char reserved[3];\n    uint8_t flags;",
        ),
        ("type", "    uint32_t len;", "    uint16_t len;"),
        (
            "size",
            "    const char *label;",
            "    const char *label;\n    uint64_t extra;",
        ),
    ] {
        let drifted = example_header().replacen(from, to, 1);
        assert_ne!(drifted, example_header(), "{label}");
        let scratch = Scratch::new(&format!("drift-{label}"), &drifted);
        std::fs::write(
            scratch.0.join("qr_header.c"),
            std::fs::read_to_string(examples().join("qr_header.c")).unwrap(),
        )
        .unwrap();
        let error = native::build_and_run(c, &scratch.0, &scratch.0.join("qr_header.c"), &["-O0"])
            .err()
            .unwrap_or_else(|| panic!("{label}: a drifted header built"));
        assert!(
            error.contains("static assertion failed") || error.contains("static_assert"),
            "{label}: {error}"
        );
    }
}

// ---------------------------------------------------------------- refusals

#[test]
fn ffi_without_its_import_refuses() {
    let scratch = Scratch::new("no-import", &example_header());
    let source = format!(
        "#import std.fmt;\n#import std.io;\n{IMPORT}fn main() -> unit effects {{ io }} {{\n    let h = Header {{ len: 1, flags: 2, id: 3, valid: false }};\n    return;\n}}\n"
    );
    refused_with(
        scratch.compile(&source, None),
        "Q032",
        "#import std.ffi.unsafe;",
    );
}

#[test]
fn a_field_type_mismatch_refuses() {
    let scratch = Scratch::new("type", &example_header());
    let source = program(
        "struct! { type Header = c\"struct qr_header\" from \"qr_header.h\"; fields { len: u16 } }\n",
        "",
        "",
    );
    refused_with(
        scratch.compile(&source, None),
        "Q064",
        "`len` is listed `u16`",
    );
}

#[test]
fn a_field_offset_mismatch_refuses() {
    let scratch = Scratch::new("offset", &example_header());
    let source = program(
        "struct! { type Header = c\"struct qr_header\" from \"qr_header.h\"; fields { flags: u8, len: u32 } }\n",
        "",
        "",
    );
    refused_with(
        scratch.compile(&source, None),
        "Q064",
        "places it at offset 0",
    );
}

#[test]
fn an_unknown_c_tag_refuses() {
    let scratch = Scratch::new("tag", &example_header());
    let source = program(
        "struct! { type Header = c\"struct qr_nothing\" from \"qr_header.h\"; fields { len: u32 } }\n",
        "",
        "",
    );
    refused_with(
        scratch.compile(&source, None),
        "Q063",
        "no complete definition",
    );
}

#[test]
fn an_opaque_field_cannot_be_read_written_or_listed_without_an_equivalent() {
    let scratch = Scratch::new("opaque", &example_header());
    let read = program(
        IMPORT,
        "",
        "    let h = Header { len: 1, flags: 2, id: 3, valid: false };\n    let r = h.reserved;\n",
    );
    refused_with(scratch.compile(&read, None), "Q051", "opaque bytes");
    let write = program(
        IMPORT,
        "",
        "    let h = Header { len: 1, flags: 2, id: 3, valid: false, label: 0 };\n",
    );
    refused_with(scratch.compile(&write, None), "Q051", "opaque bytes");
    let listed = program(
        "struct! { type Header = c\"struct qr_header\" from \"qr_header.h\"; fields { len: u32, reserved: u8 } }\n",
        "",
        "",
    );
    refused_with(
        scratch.compile(&listed, None),
        "Q063",
        "no §G02t equivalent",
    );
}

#[test]
fn a_struct_call_through_an_unmarked_contract_refuses() {
    let scratch = Scratch::new("unmarked", &example_header());
    let source = program(
        IMPORT,
        "extern fn qr_header_total(header: ptr<const, Header>) -> u32;\n",
        "    let h = Header { len: 1, flags: 2, id: 3, valid: false };\n    let t = qr_header_total(h);\n",
    );
    // Unmarked, with a pointer the ordinary vocabulary has: refused by name.
    let unmarked = "module contract;\nextern fn qr_header_total(header: ptr<const, u8>) -> u32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(header);\n    ensures normal resource_return_none();\n};\n";
    refused_with(
        scratch.compile(
            &source,
            Some(BTreeMap::from([(
                "unmarked.qv".to_owned(),
                unmarked.to_owned(),
            )])),
        ),
        "Q033",
        "not marked `[| quant #c_ffi |]`",
    );
    // Unmarked, naming the C struct: the loader refuses the selection.
    let named = unmarked.replace("ptr<const, u8>", "ptr<const, c\"struct qr_header\">");
    refused_with(
        scratch.compile(
            &source,
            Some(BTreeMap::from([("named.qv".to_owned(), named)])),
        ),
        "Q023",
        "admitted only on a declaration marked `[| quant #c_ffi |]`",
    );
}

#[test]
fn a_header_the_context_cannot_resolve_refuses() {
    let scratch = Scratch::new("header", &example_header());
    let source = program(
        "struct! { type Header = c\"struct qr_header\" from \"missing.h\"; fields { len: u32 } }\n",
        "",
        "",
    );
    refused_with(scratch.compile(&source, None), "Q063", "does not resolve");
}

#[test]
fn no_recorded_target_or_another_compiler_refuses() {
    let scratch = Scratch::new("target", &example_header());
    let source = program(IMPORT, "", "");
    let path = scratch.0.join("main.qr");
    refused_with(
        compile_at(&path, &source, None, None),
        "Q063",
        "no recorded target",
    );
    let mut other = host_target();
    other.compiler = "some other compiler 1.0".into();
    refused_with(
        compile_at(&path, &source, None, Some(&other)),
        "Q063",
        "not the recorded",
    );
}

#[test]
fn shapes_the_reader_does_not_state_refuse() {
    for (label, body, needle) in [
        (
            "union",
            "union qr_header { uint32_t len; uint8_t flags; };",
            "union",
        ),
        (
            "bitfield",
            "struct qr_header { uint32_t len : 4; };",
            "bit-field",
        ),
        (
            "flexible",
            "struct qr_header { uint32_t len; uint8_t data[]; };",
            "flexible array",
        ),
        (
            "packed",
            "struct __attribute__((packed)) qr_header { uint8_t flags; uint32_t len; };",
            "attribute",
        ),
        (
            "aligned",
            "struct qr_header { _Alignas(16) uint32_t len; };",
            "attribute",
        ),
        (
            "anonymous",
            "struct qr_header { uint32_t len; struct { int a; }; };",
            "anonymous",
        ),
    ] {
        let scratch = Scratch::new(label, &format!("#include <stdint.h>\n{body}\n"));
        let source = program(
            "struct! { type Header = c\"struct qr_header\" from \"qr_header.h\"; fields { len: u32 } }\n",
            "",
            "",
        );
        refused_with(scratch.compile(&source, None), "Q063", needle);
    }
}

#[test]
fn a_plain_safe_claim_from_an_ffi_file_is_refused() {
    let module = example();
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    assert!(qcore_compiler::driver::ffi_verdict(&checked.certificate).is_ok());
    // The trust mark dropped: no verdict is published.
    let mut stripped = checked.certificate.clone();
    stripped
        .assumptions
        .retain(|assumption| !matches!(assumption, FfiAssumption::UnsafeFfi { .. }));
    assert!(qcore_compiler::driver::ffi_verdict(&stripped).is_err());
    // Every assumption dropped: a plain `Safe` is never published.
    let mut bare = checked.certificate.clone();
    bare.assumptions.clear();
    assert!(qcore_compiler::driver::ffi_verdict(&bare).is_err());
    // The mark kept but a struct dropped from it: refused too.
    let mut partial = checked.certificate.clone();
    if let Some(FfiAssumption::UnsafeFfi { structs, .. }) = partial.assumptions.first_mut() {
        structs.clear();
    }
    assert!(qcore_compiler::driver::ffi_verdict(&partial).is_err());
    // A certificate whose published offset was edited fails its recheck.
    let mut moved = checked.certificate.clone();
    moved.structs[0].listed[1].offset += 1;
    assert!(matches!(
        qcore_core::ffi::recheck_ffi_certificate(&checked.lowered.program, &moved),
        Err(qcore_core::ffi::FfiRecheckFailure::LayoutDiffers { .. })
    ));
    assert!(
        qcore_core::ffi::recheck_ffi_certificate(&checked.lowered.program, &checked.certificate)
            .is_ok()
    );
}

#[test]
fn a_c_result_with_no_declared_outcome_is_not_rendered() {
    let path = examples().join("header_sum.qr");
    let source = std::fs::read_to_string(&path)
        .unwrap()
        .replace(" succeeds 42;", ";");
    refused_with(
        compile_at(
            &path,
            &source,
            Some(example_contracts()),
            Some(&host_target()),
        ),
        "Q043",
        "no number",
    );
}

#[test]
fn a_call_still_needs_std_extern_and_a_line_its_effect() {
    let path = examples().join("header_sum.qr");
    let source = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n");
    assert!(source.contains("#import std.extern;\n"));
    assert!(source.contains("effects { io }"));
    refused_with(
        compile_at(
            &path,
            &source.replace("#import std.extern;\n", ""),
            Some(example_contracts()),
            Some(&host_target()),
        ),
        "Q022",
        "#import std.extern;",
    );
    refused_with(
        compile_at(
            &path,
            &source.replace("effects { io }", "effects { }"),
            Some(example_contracts()),
            Some(&host_target()),
        ),
        "Q029",
        "does not admit it",
    );
}

#[test]
fn the_marker_belongs_to_the_contract_side() {
    let scratch = Scratch::new("marker", &example_header());
    let source = format!(
        "{HEAD}[| quant #c_ffi |]\nextern fn f(x: u32) -> u32;\nfn main() -> unit effects {{ io }} {{\n    return;\n}}\n"
    );
    refused_with(
        scratch.compile(&source, None),
        "Q033",
        "`.qv` contract module",
    );
}

#[test]
fn a_marked_contract_is_withheld_from_every_other_route() {
    let source = "#import std.extern;\nextern fn qr_header_make(len: u32, flags: u8) -> u32;\nfn main() -> unit effects { io } {\n    let x = qr_header_make(1, 2);\n    return;\n}\n";
    let marked = "module contract;\n[| quant #c_ffi |]\nextern fn qr_header_make(len: u32, flags: u8) -> u32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(len);\n    ensures normal resource_borrowed(flags);\n    ensures normal resource_return_none();\n};\n";
    refused_with(
        compile_at(
            Path::new("main.qr"),
            source,
            Some(BTreeMap::from([(
                "marked.qv".to_owned(),
                marked.to_owned(),
            )])),
            None,
        ),
        "Q023",
        "no admitted declared contract",
    );
}

#[test]
fn a_marked_declaration_outside_the_ffi_table_does_not_load() {
    for (label, contract) in [
        (
            "custody",
            "extern fn f(header: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_consumed(header, nonnull);\n    ensures normal resource_return_none();\n};\n",
        ),
        (
            "mut-pointer",
            "extern fn f(header: ptr<mut, c\"struct qr_header\">) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(header);\n    ensures normal resource_return_none();\n};\n",
        ),
        (
            "union",
            "extern fn f(header: c\"union qr_header\") -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(header);\n    ensures normal resource_return_none();\n};\n",
        ),
        (
            "spaced",
            "extern fn f(header: c \"struct qr_header\") -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(header);\n    ensures normal resource_return_none();\n};\n",
        ),
    ] {
        let text = format!("module contract;\n[| quant #c_ffi |]\n{contract}");
        let sources = BTreeMap::from([(format!("{label}.qv"), text)]);
        assert!(
            qcore_contract::contract_loader::load_resource_contract_sources(&sources).is_err(),
            "{label}"
        );
    }
    // The published module loads, withholds both marked functions from the
    // ordinary map, and rederives them.
    let sources = example_contracts();
    let contracts =
        qcore_contract::contract_loader::load_resource_contract_sources(&sources).unwrap();
    assert!(contracts.contract("qr_header_total").is_none());
    let marked = qcore_contract::contract_loader::ffi_declarations(&sources).unwrap();
    assert_eq!(
        marked.keys().cloned().collect::<Vec<_>>(),
        ["qr_header_make", "qr_header_total"]
    );
}

/// The family page prints the transcript the program produces.
#[test]
fn the_ffi_page_publishes_the_transcript_its_program_prints() {
    let page = std::fs::read_to_string(examples().join("README.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let block = page
        .split("```text\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("the page prints a transcript");
    let module = example();
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    assert_eq!(
        block,
        qcore_core::ffi::reference_stream(&checked.certificate, Stream::Out)
    );
    assert!(page.contains(&format!(
        "logs `{}` on standard error",
        qcore_core::ffi::reference_stream(&checked.certificate, Stream::Err).trim_end()
    )));
}

/// Card 4's closure capture list `|copy x, …|` keeps its lexical space: the
/// marker's `[|`/`|]` are two-byte tokens that only an adjacent `[` or `]`
/// forms, and a capture list lexes as single bars.
#[test]
fn the_marker_tokens_leave_the_capture_list_spelling_free() {
    use qcore_compiler::lexer::{Symbol, TokenKind, lex};
    let kinds = |text: &str| {
        lex(text)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .filter(|kind| *kind != TokenKind::Eof)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kinds("|copy x, y|"),
        [
            TokenKind::Symbol(Symbol::Pipe),
            TokenKind::Word("copy".into()),
            TokenKind::Word("x".into()),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::Word("y".into()),
            TokenKind::Symbol(Symbol::Pipe),
        ]
    );
    assert_eq!(
        kinds("[| quant #c_ffi |]"),
        [
            TokenKind::Symbol(Symbol::QuantOpen),
            TokenKind::Word("quant".into()),
            TokenKind::Symbol(Symbol::Hash),
            TokenKind::Word("c_ffi".into()),
            TokenKind::Symbol(Symbol::QuantClose),
        ]
    );
}
