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

//! Card `W48-QRLIB-LINKAGE` (card 5 of the `.qr` track): `.qrlib` as the C
//! linkage manifest of the FFI route, generated from the artifact on every
//! build, with the artifact's symbol table read on every platform.
//!
//! * **Negative controls** (Unix, a card-built C library): an FFI call with no
//!   manifest binding, a binding to a hand-written v1 manifest, a callee the
//!   bound manifest does not bind, a manifest whose entry disagrees with the
//!   contract, a header digest that differs from the `struct!` header, a
//!   stored manifest made stale by a rebuilt artifact, a build asked to link a
//!   different artifact or one whose bytes changed, and the publication
//!   tripwire.
//! * **Linux**: libm end to end, static (through `libm.a`'s `ld` script) and
//!   dynamic (through `libm.so`'s), at `-O0` and `-O2` under UBSan and at
//!   plain `-O2`, `-ffp-contract=off`, all `-Wall -Wextra -Werror -pedantic`,
//!   the linker's own `--trace` naming every pinned file; `ldexp` refused from
//!   the static libm, which does not define it; a stored manifest whose symbol
//!   version was edited refused as stale.
//! * **Unix**: a `#![no_std]` Rust `staticlib` exporting `extern "C"`
//!   functions, linked through its generated manifest.
//! * **Windows (MSVC)**, wired into the `msvc-core` job: an MSVC static `.lib`
//!   and one `ntdll.dll` function through the SDK's `ntdll.lib`.
//! * **macOS**, run by card 5m (`W48-QRLIB-MACOS`): libm through the SDK's `libm.tbd`.
//!
//! No test here writes output of its own: native results are compared through
//! the program's own `print` lines and its exit status.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceFile, compile_files_for_target,
};
use qcore_core::native_library::LinkageKind;
use qcore_core::std_io::Stream;

#[path = "../../tests/support/qrlib.rs"]
mod qrlib;

#[cfg(all(windows, target_env = "msvc"))]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

/// A scratch directory, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-qrlib-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One `#c_ffi` contract borrowing every parameter and returning data.
fn contract(name: &str, parameters: &[(&str, &str)], result: &str) -> String {
    let mut text = format!(
        "[| quant #c_ffi |]\nextern fn {name}({}) -> {result}\ncontract {{\n    requires resource_contract_v1;\n",
        parameters
            .iter()
            .map(|(p, ty)| format!("{p}: {ty}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for (p, _) in parameters {
        text.push_str(&format!("    ensures normal resource_borrowed({p});\n"));
    }
    text.push_str("    ensures normal resource_return_none();\n};\n");
    text
}

fn module(contracts: &[String]) -> String {
    format!("module contract;\n\n{}", contracts.join("\n"))
}

/// A `.qr` program binding `library`, declaring `declarations` and running
/// `body` in an entry with row `io`.
fn program(library: Option<&str>, prelude: &str, declarations: &str, body: &str) -> String {
    format!(
        "#import std.ffi.unsafe;\n#import std.extern;\n#import std.fmt;\n#import std.io;\n{}{prelude}{declarations}fn main() -> unit effects {{ io }} {{\n{body}    return;\n}}\n",
        library.map_or_else(String::new, |name| format!("extern library {name};\n"))
    )
}

fn compile(
    path: &Path,
    source: &str,
    contracts: &BTreeMap<String, String>,
    manifests: Option<BTreeMap<String, String>>,
    target: &RecordedTarget,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new(path.display().to_string(), source)],
        &CompileOptions {
            extern_contracts: Some(contracts.clone()),
            native_libraries: manifests,
            ..CompileOptions::default()
        },
        Some(target),
    )
}

#[cfg(unix)]
fn refused(result: Result<CompiledModule, Vec<FileDiagnostic>>, code: &str, needle: &str) {
    match result {
        Ok(_) => panic!("expected {code} mentioning `{needle}`, admitted"),
        Err(errors) => assert!(
            errors
                .iter()
                .any(|error| error.code == code && error.message.contains(needle)),
            "expected {code} mentioning `{needle}`, got {errors:#?}"
        ),
    }
}

fn reference(module: &CompiledModule) -> (String, String) {
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    (
        qcore_core::ffi::reference_stream(&checked.certificate, Stream::Out),
        qcore_core::ffi::reference_stream(&checked.certificate, Stream::Err),
    )
}

#[cfg(unix)]
mod unix {
    use super::*;

    /// The recorded target naming the Clang that runs here.
    pub fn host_target() -> RecordedTarget {
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

    pub fn clang() -> String {
        std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into())
    }

    /// The three builds of every native row: `-O0` and `-O2` under UBSan (IEEE
    /// division intended, so `float-divide-by-zero` off) and plain `-O2`.
    pub const BUILDS: [(&str, &[&str]); 3] = [
        (
            "O0-ubsan",
            &[
                "-O0",
                "-fsanitize=undefined,integer",
                "-fno-sanitize=float-divide-by-zero",
                "-fno-sanitize-recover=all",
            ],
        ),
        (
            "O2-ubsan",
            &[
                "-O2",
                "-fsanitize=undefined,integer",
                "-fno-sanitize=float-divide-by-zero",
                "-fno-sanitize-recover=all",
            ],
        ),
        ("O2", &["-O2"]),
    ];

    pub struct Built {
        pub stdout: String,
        pub stderr: String,
        pub status: Option<i32>,
        /// What the linker printed for `--trace` (GNU `ld` only), else empty;
        /// only the Linux row reads it.
        #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
        pub trace: String,
    }

    /// Build `c` against exactly `libraries`, with `includes`, and run it.
    pub fn build_and_run(
        dir: &Path,
        c: &str,
        includes: &[&Path],
        libraries: &[&Path],
        flags: &[&str],
    ) -> Result<Built, String> {
        std::fs::write(dir.join("main.c"), c).unwrap();
        let mut command = Command::new(clang());
        command
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-ffp-contract=off",
            ])
            .args(flags);
        if cfg!(target_os = "linux") {
            command.arg("-Wl,--trace");
        }
        for include in includes {
            command.arg("-I").arg(include);
        }
        command.arg(dir.join("main.c"));
        command.args(libraries);
        let out = command
            .arg("-o")
            .arg(dir.join("main"))
            .output()
            .expect("Clang is required for this native row");
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into_owned());
        }
        let run = Command::new(dir.join("main"))
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        Ok(Built {
            stdout: String::from_utf8(run.stdout).unwrap(),
            stderr: String::from_utf8(run.stderr).unwrap(),
            status: run.status.code(),
            trace: String::from_utf8_lossy(&out.stdout).into_owned(),
        })
    }

    // ------------------------------------------------ a card-built C library

    pub const DEMO_H: &str = "#ifndef QR_DEMO_H\n#define QR_DEMO_H\n#include <stdint.h>\nstruct qr_pair {\n    int32_t a;\n    int32_t b;\n};\nint32_t qr_pair_sum(const struct qr_pair *pair);\nint32_t qr_twice(int32_t x);\n#endif\n";
    pub const DEMO_C: &str = "#include \"qr_demo.h\"\nint32_t qr_pair_sum(const struct qr_pair *pair) { return pair->a + pair->b; }\nint32_t qr_twice(int32_t x) { return x * 2; }\n";

    pub fn demo_contracts() -> BTreeMap<String, String> {
        BTreeMap::from([(
            "demo.qv".to_owned(),
            module(&[
                contract(
                    "qr_pair_sum",
                    &[("pair", "ptr<const, c\"struct qr_pair\">")],
                    "i32",
                ),
                contract("qr_twice", &[("x", "i32")], "i32"),
            ]),
        )])
    }

    pub const DEMO_IMPORT: &str = "struct! { type Pair = c\"struct qr_pair\" from \"qr_demo.h\"; fields { a: i32, b: i32 } }\n";
    pub const DEMO_DECLARATIONS: &str = "extern fn qr_pair_sum(pair: ptr<const, Pair>) -> i32;\nextern fn qr_twice(x: i32) -> i32;\n";
    pub const DEMO_BODY: &str = "    let p = Pair { a: 20, b: 22 };\n    let s = qr_pair_sum(p) succeeds 42;\n    print dec s;\n    let t = qr_twice(21) succeeds 42;\n    print dec t;\n";

    pub struct Demo {
        pub scratch: Scratch,
        pub archive: PathBuf,
        pub generated: qrlib::Generated,
    }

    impl Demo {
        pub fn new(label: &str) -> Self {
            let scratch = Scratch::new(label);
            std::fs::write(scratch.path().join("qr_demo.h"), DEMO_H).unwrap();
            std::fs::write(scratch.path().join("qr_demo.c"), DEMO_C).unwrap();
            let archive = qrlib::static_library(
                scratch.path(),
                "qr_demo",
                &[scratch.path().join("qr_demo.c")],
                &[scratch.path()],
            );
            let generated = qrlib::manifest(
                "qr_demo",
                LinkageKind::Static,
                &archive,
                Some(&scratch.path().join("qr_demo.h")),
                &host_target(),
                &demo_contracts(),
            );
            Self {
                scratch,
                archive,
                generated,
            }
        }

        pub fn compile(
            &self,
            source: &str,
            manifests: Option<BTreeMap<String, String>>,
        ) -> Result<CompiledModule, Vec<FileDiagnostic>> {
            compile(
                &self.scratch.path().join("main.qr"),
                source,
                &demo_contracts(),
                manifests,
                &host_target(),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Negative controls first
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn an_ffi_call_with_no_manifest_binding_refuses() {
    use unix::*;
    let demo = Demo::new("unbound");
    // No `extern library` at all.
    refused(
        demo.compile(
            &program(None, DEMO_IMPORT, DEMO_DECLARATIONS, DEMO_BODY),
            Some(demo.generated.selection()),
        ),
        "Q065",
        "binds no library",
    );
    // A binding and no manifest selected.
    refused(
        demo.compile(
            &program(Some("qr_demo"), DEMO_IMPORT, DEMO_DECLARATIONS, DEMO_BODY),
            None,
        ),
        "Q065",
        "no native-library manifest is selected",
    );
}

#[cfg(unix)]
#[test]
fn a_hand_written_v1_manifest_does_not_link_an_ffi_call() {
    use unix::*;
    let demo = Demo::new("v1");
    let v1 = format!(
        "native_library manifest v1;\nlibrary qr_demo;\nversion 0.1.0;\nartifact {};\nhash sha256:{};\ndata_model lp64;\nunwinding abort;\nallocator_custody closed;\nthread_posture single_caller;\ncontract fn qr_twice(int32_t) -> int32_t from demo.qv;\ncontract fn qr_pair_sum(uint8_t *) -> int32_t from demo.qv;\n",
        demo.archive.display(),
        "0".repeat(64)
    );
    refused(
        demo.compile(
            &program(
                Some("qr_demo"),
                "",
                "extern fn qr_twice(x: i32) -> i32;\n",
                "    let t = qr_twice(21) succeeds 42;\n    print dec t;\n",
            ),
            Some(BTreeMap::from([("hand.qrlib".to_owned(), v1)])),
        ),
        "Q065",
        "hand-written v1 manifest",
    );
}

#[cfg(unix)]
#[test]
fn a_callee_the_bound_manifest_does_not_bind_refuses() {
    use unix::*;
    let demo = Demo::new("unbound-callee");
    // A manifest generated from a module holding `qr_twice` alone; the program
    // also calls `qr_pair_sum` from a second selected module.
    let twice_only = BTreeMap::from([(
        "twice.qv".to_owned(),
        module(&[contract("qr_twice", &[("x", "i32")], "i32")]),
    )]);
    let generated = qrlib::manifest(
        "qr_demo",
        LinkageKind::Static,
        &demo.archive,
        Some(&demo.scratch.path().join("qr_demo.h")),
        &host_target(),
        &twice_only,
    );
    let mut selection = twice_only.clone();
    selection.insert(
        "pair.qv".to_owned(),
        module(&[contract(
            "qr_pair_sum",
            &[("pair", "ptr<const, c\"struct qr_pair\">")],
            "i32",
        )]),
    );
    refused(
        compile(
            &demo.scratch.path().join("main.qr"),
            &program(Some("qr_demo"), DEMO_IMPORT, DEMO_DECLARATIONS, DEMO_BODY),
            &selection,
            Some(generated.selection()),
            &host_target(),
        ),
        "Q066",
        "`qr_pair_sum` is bound by no generated manifest",
    );
}

#[cfg(unix)]
#[test]
fn a_manifest_entry_that_disagrees_with_the_contract_refuses() {
    use unix::*;
    let demo = Demo::new("disagree");
    // Generated from a module spelling `qr_twice` over `i64`; the program's
    // selection spells it over `i32` under the same module key.
    let wide = BTreeMap::from([(
        "demo.qv".to_owned(),
        module(&[
            contract(
                "qr_pair_sum",
                &[("pair", "ptr<const, c\"struct qr_pair\">")],
                "i32",
            ),
            contract("qr_twice", &[("x", "i64")], "i64"),
        ]),
    )]);
    let generated = qrlib::manifest(
        "qr_demo",
        LinkageKind::Static,
        &demo.archive,
        Some(&demo.scratch.path().join("qr_demo.h")),
        &host_target(),
        &wide,
    );
    refused(
        demo.compile(
            &program(Some("qr_demo"), DEMO_IMPORT, DEMO_DECLARATIONS, DEMO_BODY),
            Some(generated.selection()),
        ),
        "Q041",
        "parameter 0 is `int32_t` in the contract and `int64_t` in the manifest",
    );
}

#[cfg(unix)]
#[test]
fn a_header_digest_that_differs_from_the_struct_header_refuses() {
    use unix::*;
    let demo = Demo::new("header-digest");
    // The manifest pins a header whose bytes differ from the one `struct!`
    // reads (one comment line more): the two are not the same header.
    let other = demo.scratch.path().join("qr_demo_other.h");
    std::fs::write(&other, format!("/* another build */\n{DEMO_H}")).unwrap();
    let generated = qrlib::manifest(
        "qr_demo",
        LinkageKind::Static,
        &demo.archive,
        Some(&other),
        &host_target(),
        &demo_contracts(),
    );
    refused(
        demo.compile(
            &program(Some("qr_demo"), DEMO_IMPORT, DEMO_DECLARATIONS, DEMO_BODY),
            Some(generated.selection()),
        ),
        "Q064",
        "the two must be the same bytes",
    );
}

#[cfg(unix)]
#[test]
fn a_stored_manifest_of_a_rebuilt_artifact_is_stale_and_a_changed_artifact_does_not_link() {
    use qcore_c_gateway::linkage::generate::{GenerationInputs, GenerationRefusal, check_stale};
    use qcore_c_gateway::linkage::link::{self, LinkRefusal};
    use unix::*;
    let demo = Demo::new("stale");
    let stored = demo.generated.text.clone();
    // Rebuild the library from changed source: same symbols, other bytes.
    std::fs::write(
        demo.scratch.path().join("qr_demo.c"),
        format!("{DEMO_C}int32_t qr_unused(void) {{ return 7; }}\n"),
    )
    .unwrap();
    let rebuilt = qrlib::static_library(
        demo.scratch.path(),
        "qr_demo",
        &[demo.scratch.path().join("qr_demo.c")],
        &[demo.scratch.path()],
    );
    assert_eq!(rebuilt, demo.archive);
    let artifact = demo.archive.display().to_string();
    let header_path = demo.scratch.path().join("qr_demo.h");
    let header_ref = header_path.display().to_string();
    let header = std::fs::read(&header_path).unwrap();
    let contracts = demo_contracts();
    let target = host_target();
    let inputs = GenerationInputs {
        library: "qr_demo",
        version: "0.1.0",
        linkage: LinkageKind::Static,
        artifact: &artifact,
        header: Some((&header_ref, &header)),
        target: &target,
        unwinding: "abort",
        allocator_custody: "closed",
        thread_posture: "single_caller",
        contracts: &contracts,
        os_build: None,
    };
    let loader = |reference: &str| std::fs::read(reference).map_err(|e| e.to_string());
    assert!(matches!(
        check_stale(&stored, &inputs, &loader, None),
        Err(GenerationRefusal::Stale { .. })
    ));
    // The build refuses to link the changed artifact against the old manifest,
    // and refuses any artifact but the pinned one.
    let manifest = qcore_core::native_library::load_native_library_manifest(&stored).unwrap();
    let resolve = |reference: &str| PathBuf::from(reference);
    assert!(matches!(
        link::plan(&manifest, &resolve),
        Err(LinkRefusal::DigestMismatch { .. })
    ));
    let fresh = qrlib::manifest(
        "qr_demo",
        LinkageKind::Static,
        &demo.archive,
        Some(&header_path),
        &target,
        &contracts,
    );
    let plan = fresh.link_plan();
    assert!(plan.admit(&demo.archive).is_ok());
    assert!(matches!(
        plan.admit(&demo.scratch.path().join("qr_demo.c")),
        Err(LinkRefusal::OtherArtifact { .. })
    ));
}

#[cfg(unix)]
#[test]
fn a_certificate_that_drops_its_linkage_premise_is_not_published() {
    use unix::*;
    let demo = Demo::new("tripwire");
    let module = demo
        .compile(
            &program(Some("qr_demo"), DEMO_IMPORT, DEMO_DECLARATIONS, DEMO_BODY),
            Some(demo.generated.selection()),
        )
        .unwrap();
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    assert!(qcore_core::ffi::certificate_carries_linkage(
        &checked.certificate
    ));
    let mut edited = checked.certificate.clone();
    edited
        .assumptions
        .retain(|assumption| !matches!(assumption, qcore_core::ffi::FfiAssumption::Linked(_)));
    assert!(!qcore_core::ffi::certificate_carries_linkage(&edited));
    // The verdict names every linkage premise and is never plain Safe.
    let verdict = module.published_verdict().unwrap();
    assert!(!verdict.is_plain_safe());
    assert!(verdict.holds_under(qcore_core::ffi::UNSAFE_FFI_ASSUMPTION));
    assert!(verdict.holds_under("linked:qr_twice"));
    assert!(verdict.holds_under("linked:qr_pair_sum"));
}

// ---------------------------------------------------------------------------
// The card-built C library, linked through its manifest
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn a_card_built_c_archive_links_through_its_generated_manifest_under_three_builds() {
    use unix::*;
    let demo = Demo::new("c-archive");
    let module = demo
        .compile(
            &program(Some("qr_demo"), DEMO_IMPORT, DEMO_DECLARATIONS, DEMO_BODY),
            Some(demo.generated.selection()),
        )
        .unwrap();
    let linked: Vec<String> = module
        .ffi_assumptions()
        .iter()
        .filter_map(|assumption| match assumption {
            qcore_core::ffi::FfiAssumption::Linked(link) => Some(link.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(linked.len(), 2, "{linked:#?}");
    assert!(
        linked
            .iter()
            .all(|line| line.contains("from the static artifact")
                && line.contains("which code is linked, not that the code is safe")),
        "{linked:#?}"
    );
    let (stdout, stderr) = reference(&module);
    assert_eq!(stdout, "42\n42\n");
    let plan = demo.generated.link_plan();
    for (label, flags) in BUILDS {
        let run = build_and_run(
            demo.scratch.path(),
            module.c_source.as_deref().unwrap(),
            &[demo.scratch.path()],
            &[&plan.input],
            flags,
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(run.status, Some(0), "{label}: {}", run.stderr);
        assert_eq!(run.stdout, stdout, "{label}");
        assert_eq!(run.stderr, stderr, "{label}");
    }
}

// ---------------------------------------------------------------------------
// libm on Linux, static and dynamic
// ---------------------------------------------------------------------------

/// `(function, parameters, result, arguments, outcome, rendering)`: each call
/// is an exact IEEE operation, so this platform's own libm must return the
/// declared outcome bit for bit.
#[cfg(any(target_os = "linux", target_os = "macos"))]
type LibmRow = (
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

#[cfg(any(target_os = "linux", target_os = "macos"))]
const LIBM: [LibmRow; 6] = [
    ("ilogb", &[("x", "f64")], "i32", "1024.0", "10", "dec"),
    (
        "sqrt",
        &[("x", "f64")],
        "f64",
        "2.0",
        "1.4142135623730951",
        "hex",
    ),
    ("fabs", &[("x", "f64")], "f64", "-3.5", "3.5", "dec"),
    (
        "fmax",
        &[("x", "f64"), ("y", "f64")],
        "f64",
        "1.0, 2.0",
        "2.0",
        "dec",
    ),
    (
        "nextafter",
        &[("x", "f64"), ("y", "f64")],
        "f64",
        "1.0, 2.0",
        "1.0000000000000002",
        "hex",
    ),
    ("floor", &[("x", "f64")], "f64", "2.5", "2.0", "dec"),
];

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn libm_program(functions: &[&str]) -> (BTreeMap<String, String>, String) {
    let rows: Vec<_> = LIBM
        .iter()
        .filter(|row| functions.contains(&row.0))
        .collect();
    let contracts = BTreeMap::from([(
        "libm.qv".to_owned(),
        module(
            &rows
                .iter()
                .map(|(name, parameters, result, ..)| contract(name, parameters, result))
                .collect::<Vec<_>>(),
        ),
    )]);
    let declarations: String = rows
        .iter()
        .map(|(name, parameters, result, ..)| {
            format!(
                "extern fn {name}({}) -> {result};\n",
                parameters
                    .iter()
                    .map(|(p, ty)| format!("{p}: {ty}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    let body: String = rows
        .iter()
        .map(|(name, _, _, arguments, outcome, rendering)| {
            format!("    let r_{name} = {name}({arguments}) succeeds {outcome};\n    print {rendering} r_{name};\n")
        })
        .collect();
    (contracts, program(Some("libm"), "", &declarations, &body))
}

#[cfg(target_os = "linux")]
fn libm_path(file: &str) -> PathBuf {
    let out = Command::new(unix::clang())
        .arg(format!("-print-file-name={file}"))
        .output()
        .unwrap();
    let path = PathBuf::from(String::from_utf8(out.stdout).unwrap().trim());
    assert!(
        path.is_absolute() && path.is_file(),
        "`{file}` is not installed where Clang links it from: {}",
        path.display()
    );
    path
}

/// libm, static through `libm.a` and dynamic through `libm.so`, both GNU `ld`
/// scripts on glibc. The static row leaves out `floor`: glibc's static
/// `floor` is an `ifunc` whose resolver reads `_dl_x86_cpu_features`, which
/// only a fully static libc defines (specification §L14.8).
#[cfg(target_os = "linux")]
#[test]
fn libm_links_statically_and_dynamically_and_runs_under_three_builds() {
    use unix::*;
    let target = host_target();
    let mut summary = Vec::new();
    for (kind, file, functions) in [
        (
            LinkageKind::Static,
            "libm.a",
            &["ilogb", "sqrt", "fabs", "fmax", "nextafter"][..],
        ),
        (
            LinkageKind::Dynamic,
            "libm.so",
            &["ilogb", "sqrt", "fabs", "fmax", "nextafter", "floor"][..],
        ),
    ] {
        let scratch = Scratch::new(&format!("libm-{}", kind.spelling()));
        let artifact = libm_path(file);
        let (contracts, source) = libm_program(functions);
        let generated = qrlib::manifest("libm", kind, &artifact, None, &target, &contracts);
        let manifest =
            qcore_core::native_library::load_native_library_manifest(&generated.text).unwrap();
        let linkage = manifest.linkage.as_ref().unwrap();
        // glibc's `libm.a`/`libm.so` are `ld` scripts: every file they name is
        // pinned beside them.
        assert!(!linkage.inputs.is_empty(), "{}", generated.text);
        for entry in manifest.contracted.values() {
            let export = entry.export.as_ref().unwrap();
            assert_eq!(
                export.binding,
                qcore_core::native_library::ExportBinding::Weak,
                "glibc exports `{}` weak",
                entry.symbol
            );
            match kind {
                LinkageKind::Dynamic => {
                    assert!(
                        export
                            .version
                            .as_deref()
                            .is_some_and(|v| v.starts_with("GLIBC_")),
                        "{}",
                        generated.text
                    );
                }
                LinkageKind::Static => assert_eq!(export.version, None),
            }
        }
        if kind == LinkageKind::Dynamic {
            assert!(manifest.contracted["floor"].export.as_ref().unwrap().ifunc);
            assert!(generated.text.contains("runtime soname libm.so.6 in "));
        }
        let module = compile(
            &scratch.path().join("main.qr"),
            &source,
            &contracts,
            Some(generated.selection()),
            &target,
        )
        .unwrap_or_else(|errors| panic!("{errors:#?}"));
        let (stdout, stderr) = reference(&module);
        assert_eq!(stdout.lines().count(), functions.len());
        let plan = generated.link_plan();
        for (label, flags) in BUILDS {
            let run = build_and_run(
                scratch.path(),
                module.c_source.as_deref().unwrap(),
                &[],
                &[&plan.input],
                flags,
            )
            .unwrap_or_else(|error| panic!("{} {label}: {error}", kind.spelling()));
            assert_eq!(
                run.status,
                Some(0),
                "{} {label}: {}",
                kind.spelling(),
                run.stderr
            );
            assert_eq!(run.stdout, stdout, "{} {label}", kind.spelling());
            assert_eq!(run.stderr, stderr, "{} {label}", kind.spelling());
            // The linker's own trace names the pinned artifact and every file
            // its script names (an `AS_NEEDED` one only if it was needed).
            for input in std::iter::once(manifest.identity.artifact.as_str()).chain(
                linkage
                    .inputs
                    .iter()
                    .filter(|input| !input.as_needed)
                    .map(|input| input.reference.as_str()),
            ) {
                assert!(
                    run.trace.contains(input),
                    "{} {label}: the link did not read `{input}`:\n{}",
                    kind.spelling(),
                    run.trace
                );
            }
            summary.push(format!(
                "{{\"linkage\": \"{}\", \"build\": \"{label}\", \"exit\": 0, \"stdout\": {:?}, \"sanitizer_reports\": 0, \"pinned\": {}}}",
                kind.spelling(),
                stdout,
                plan.verified.len()
            ));
        }
    }
    if let Ok(path) = std::env::var("QCORE_W48_QRLIB_NATIVE_SUMMARY") {
        std::fs::write(
            path,
            format!(
                "{{\n \"compiler\": \"{}\",\n \"triple\": \"{}\",\n \"flags\": \"-std=c11 -Wall -Wextra -Werror -pedantic -ffp-contract=off\",\n \"runs\": [\n  {}\n ]\n}}\n",
                target.compiler,
                target.triple,
                summary.join(",\n  ")
            ),
        )
        .unwrap();
    }
}

/// glibc's static libm does not define `ldexp` (it lives in `libc.a`): a
/// manifest binding it through `libm.a` is not generated.
#[cfg(target_os = "linux")]
#[test]
fn ldexp_is_not_generated_from_the_static_libm() {
    let contracts = BTreeMap::from([(
        "libm.qv".to_owned(),
        module(&[contract("ldexp", &[("x", "f64"), ("e", "i32")], "f64")]),
    )]);
    let artifact = libm_path("libm.a").display().to_string();
    let target = unix::host_target();
    let inputs = qcore_c_gateway::linkage::generate::GenerationInputs {
        library: "libm",
        version: "system",
        linkage: LinkageKind::Static,
        artifact: &artifact,
        header: None,
        target: &target,
        unwinding: "abort",
        allocator_custody: "closed",
        thread_posture: "single_caller",
        contracts: &contracts,
        os_build: None,
    };
    let loader = |reference: &str| std::fs::read(reference).map_err(|e| e.to_string());
    let refusal = qcore_c_gateway::linkage::generate::generate(&inputs, &loader, None).unwrap_err();
    assert!(
        refusal
            .to_string()
            .contains("`ldexp` does not link from the pinned artifact"),
        "{refusal}"
    );
}

/// A symbol version edited in a stored libm manifest is refused as stale.
#[cfg(target_os = "linux")]
#[test]
fn a_symbol_version_that_differs_from_the_artifact_is_stale() {
    let (contracts, _) = libm_program(&["ilogb"]);
    let artifact = libm_path("libm.so");
    let target = unix::host_target();
    let generated = qrlib::manifest(
        "libm",
        LinkageKind::Dynamic,
        &artifact,
        None,
        &target,
        &contracts,
    );
    let version = generated
        .text
        .split(" version ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap()
        .to_owned();
    let edited = generated
        .text
        .replace(&format!(" version {version} "), " version GLIBC_0.0 ");
    assert_ne!(edited, generated.text);
    let artifact_ref = artifact.display().to_string();
    let inputs = qcore_c_gateway::linkage::generate::GenerationInputs {
        library: "libm",
        version: "0.1.0",
        linkage: LinkageKind::Dynamic,
        artifact: &artifact_ref,
        header: None,
        target: &target,
        unwinding: "abort",
        allocator_custody: "closed",
        thread_posture: "single_caller",
        contracts: &contracts,
        os_build: None,
    };
    let loader = |reference: &str| std::fs::read(reference).map_err(|e| e.to_string());
    let refusal = qcore_c_gateway::linkage::generate::check_stale(&edited, &inputs, &loader, None)
        .unwrap_err();
    assert!(refusal.to_string().contains("is stale"), "{refusal}");
}

// ---------------------------------------------------------------------------
// A Rust staticlib exporting `extern "C"`
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn a_rust_staticlib_exporting_extern_c_links_through_its_generated_manifest() {
    use unix::*;
    let scratch = Scratch::new("rust-staticlib");
    std::fs::write(
        scratch.path().join("qr_rust.rs"),
        "#![no_std]\n\
         #[panic_handler]\n\
         fn panic(_: &core::panic::PanicInfo<'_>) -> ! { loop {} }\n\
         #[unsafe(no_mangle)]\n\
         pub extern \"C\" fn qr_rust_add(a: i32, b: i32) -> i32 { a.wrapping_add(b) }\n\
         #[unsafe(no_mangle)]\n\
         pub extern \"C\" fn qr_rust_scale(x: f64, k: f64) -> f64 { x * k }\n",
    )
    .unwrap();
    let archive = scratch.path().join("libqr_rust.a");
    let rustc = Command::new("rustc")
        .args([
            "--edition",
            "2024",
            "--crate-type",
            "staticlib",
            "-C",
            "panic=abort",
            "-C",
            "opt-level=2",
        ])
        .arg(scratch.path().join("qr_rust.rs"))
        .arg("-o")
        .arg(&archive)
        .output()
        .unwrap();
    assert!(
        rustc.status.success(),
        "{}",
        String::from_utf8_lossy(&rustc.stderr)
    );
    let contracts = BTreeMap::from([(
        "rust.qv".to_owned(),
        module(&[
            contract("qr_rust_add", &[("a", "i32"), ("b", "i32")], "i32"),
            contract("qr_rust_scale", &[("x", "f64"), ("k", "f64")], "f64"),
        ]),
    )]);
    let target = host_target();
    // No header: the library is published only through its C symbol table.
    let generated = qrlib::manifest(
        "qr_rust",
        LinkageKind::Static,
        &archive,
        None,
        &target,
        &contracts,
    );
    assert!(generated.text.contains("export "), "{}", generated.text);
    let source = program(
        Some("qr_rust"),
        "",
        "extern fn qr_rust_add(a: i32, b: i32) -> i32;\nextern fn qr_rust_scale(x: f64, k: f64) -> f64;\n",
        "    let s = qr_rust_add(40, 2) succeeds 42;\n    print dec s;\n    let k = qr_rust_scale(1.5, 4.0) succeeds 6.0;\n    print dec k;\n",
    );
    let module = compile(
        &scratch.path().join("main.qr"),
        &source,
        &contracts,
        Some(generated.selection()),
        &target,
    )
    .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let (stdout, stderr) = reference(&module);
    assert_eq!(stdout, "42\n6\n");
    let plan = generated.link_plan();
    for (label, flags) in BUILDS {
        let run = build_and_run(
            scratch.path(),
            module.c_source.as_deref().unwrap(),
            &[],
            &[&plan.input],
            flags,
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(run.status, Some(0), "{label}: {}", run.stderr);
        assert_eq!(run.stdout, stdout, "{label}");
        assert_eq!(run.stderr, stderr, "{label}");
    }
}

// ---------------------------------------------------------------------------
// macOS (card 5m): libm through the SDK's text stub. Run by W48-QRLIB-MACOS.
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
#[test]
fn libm_links_through_the_sdk_text_stub_under_three_builds() {
    use unix::*;
    let sdk = Command::new("xcrun")
        .arg("--show-sdk-path")
        .output()
        .unwrap();
    assert!(sdk.status.success(), "xcrun --show-sdk-path failed");
    let stub =
        PathBuf::from(String::from_utf8(sdk.stdout).unwrap().trim()).join("usr/lib/libm.tbd");
    assert!(stub.is_file(), "{} is absent", stub.display());
    let target = host_target();
    let functions = ["ilogb", "sqrt", "fabs", "fmax", "nextafter", "floor"];
    let (contracts, source) = libm_program(&functions);
    let generated = qrlib::manifest(
        "libm",
        LinkageKind::Dynamic,
        &stub,
        None,
        &target,
        &contracts,
    );
    assert!(
        generated
            .text
            .contains("runtime install_name /usr/lib/libSystem.B.dylib"),
        "{}",
        generated.text
    );
    let scratch = Scratch::new("libm-tbd");
    let module = compile(
        &scratch.path().join("main.qr"),
        &source,
        &contracts,
        Some(generated.selection()),
        &target,
    )
    .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let (stdout, stderr) = reference(&module);
    let plan = generated.link_plan();
    for (label, flags) in BUILDS {
        let run = build_and_run(
            scratch.path(),
            module.c_source.as_deref().unwrap(),
            &[],
            &[&plan.input],
            flags,
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(run.status, Some(0), "{label}: {}", run.stderr);
        assert_eq!(run.stdout, stdout, "{label}");
        assert_eq!(run.stderr, stderr, "{label}");
    }
}

// ---------------------------------------------------------------------------
// Windows (MSVC), wired into the `msvc-core` job. Not run on this card's host.
// ---------------------------------------------------------------------------

#[cfg(all(windows, target_env = "msvc"))]
mod windows {
    use super::*;

    /// The recorded target of the MSVC build. No `struct!` is read here, so no
    /// layout is taken from it; the triple derives the COFF format and the
    /// LLP64 data model.
    pub fn msvc_target() -> RecordedTarget {
        let version = Command::new("rustc").arg("-vV").output().unwrap();
        assert!(version.status.success());
        let triple = String::from_utf8(version.stdout)
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .expect("rustc reports its host triple")
            .to_owned();
        assert!(triple.ends_with("-pc-windows-msvc"), "{triple}");
        RecordedTarget {
            pointer_width: if cfg!(target_pointer_width = "64") {
                PointerWidth::W64
            } else {
                PointerWidth::W32
            },
            triple,
            compiler: "Microsoft C/C++ (cl.exe)".into(),
        }
    }

    /// The first directory of `LIB` (set by the Visual Studio environment)
    /// holding `name`.
    pub fn sdk_library(name: &str) -> PathBuf {
        // Cargo can find MSVC without a Developer Command Prompt. Use the same
        // discovered compiler environment as the native C test harness.
        let target = msvc_target();
        let compiler = cc::Build::new()
            .host(&target.triple)
            .target(&target.triple)
            .out_dir(std::env::temp_dir())
            .opt_level(1)
            .cargo_metadata(false)
            .get_compiler();
        let lib = std::env::var_os("LIB")
            .or_else(|| {
                compiler
                    .env()
                    .iter()
                    .find(|(key, _)| key == "LIB")
                    .map(|(_, value)| value.clone())
            })
            .expect("the discovered MSVC environment sets LIB");
        std::env::split_paths(&lib)
            .map(|directory| directory.join(name))
            .find(|path| path.is_file())
            .unwrap_or_else(|| panic!("`{name}` is in no LIB directory"))
    }

    pub fn system32() -> PathBuf {
        PathBuf::from(std::env::var("SystemRoot").expect("SystemRoot is set")).join("System32")
    }

    /// `cmd /c ver`'s version number: the OS build the runtime DLL is checked on.
    pub fn os_build() -> String {
        let out = Command::new("cmd").args(["/c", "ver"]).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let start = text
            .find("Version ")
            .map(|at| at + "Version ".len())
            .unwrap();
        text[start..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
            .collect()
    }
}

#[cfg(all(windows, target_env = "msvc"))]
#[test]
fn an_msvc_static_library_links_through_its_generated_manifest() {
    use windows::*;
    let scratch = Scratch::new("msvc-lib");
    std::fs::write(
        scratch.path().join("qr_win.c"),
        "#include <stdint.h>\nint32_t qr_win_add(int32_t a, int32_t b) { return a + b; }\nuint32_t qr_win_mask(uint32_t x) { return x & 0xffu; }\n",
    )
    .unwrap();
    let archive = qrlib::static_library(
        scratch.path(),
        "qr_win",
        &[scratch.path().join("qr_win.c")],
        &[],
    );
    let contracts = BTreeMap::from([(
        "win.qv".to_owned(),
        module(&[
            contract("qr_win_add", &[("a", "i32"), ("b", "i32")], "i32"),
            contract("qr_win_mask", &[("x", "u32")], "u32"),
        ]),
    )]);
    let target = msvc_target();
    let generated = qrlib::manifest(
        "qr_win",
        LinkageKind::Static,
        &archive,
        None,
        &target,
        &contracts,
    );
    assert!(generated.text.contains("format coff;"));
    assert!(generated.text.contains("data_model llp64;"));
    let source = program(
        Some("qr_win"),
        "",
        "extern fn qr_win_add(a: i32, b: i32) -> i32;\nextern fn qr_win_mask(x: u32) -> u32;\n",
        "    let s = qr_win_add(40, 2) succeeds 42;\n    print dec s;\n    let m = qr_win_mask(4660) succeeds 52;\n    print dec m;\n",
    );
    let module = compile(
        &scratch.path().join("main.qr"),
        &source,
        &contracts,
        Some(generated.selection()),
        &target,
    )
    .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let (stdout, _) = reference(&module);
    let plan = generated.link_plan();
    let run =
        windows_c::run_msvc_with_libraries(module.c_source.as_deref().unwrap(), &[&plan.input]);
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    assert_eq!(String::from_utf8_lossy(&run.stdout), stdout);
}

/// One `ntdll.dll` function with an integer-only signature, through the SDK's
/// `ntdll.lib`. The name is not assumed: the first candidate that `ntdll.lib`
/// imports **by name** and this image's `ntdll.dll` exports **by name and not
/// as a forwarder** is the one used, and the generator re-checks both.
#[cfg(all(windows, target_env = "msvc"))]
#[test]
fn one_ntdll_function_links_through_the_sdk_import_library() {
    use qcore_c_gateway::linkage::{artifact, coff, resolve};
    use windows::*;
    let scratch = Scratch::new("ntdll");
    let sdk_import_library = sdk_library("ntdll.lib");
    // A manifest token cannot carry the spaces in the SDK installation path.
    // Pin and link an exact copy of that SDK artifact in the fixture directory.
    let import_library = scratch.path().join("ntdll.lib");
    let sdk_bytes = std::fs::read(&sdk_import_library).unwrap();
    std::fs::write(&import_library, &sdk_bytes).unwrap();
    assert_eq!(std::fs::read(&import_library).unwrap(), sdk_bytes);
    let runtime = system32().join("ntdll.dll");
    let facts = artifact::TargetFacts::from_triple(&msvc_target().triple).unwrap();
    let loader = |reference: &str| std::fs::read(reference).map_err(|e| e.to_string());
    let read =
        artifact::read_artifact(&import_library.display().to_string(), &loader, &facts).unwrap();
    let exports = coff::pe_exports(&std::fs::read(&runtime).unwrap()).unwrap();
    // `RtlNtStatusToDosError(NTSTATUS) -> ULONG`: `0` maps to `0`, and
    // `STATUS_ACCESS_DENIED` (0xC0000022) to `ERROR_ACCESS_DENIED` (5).
    let name = ["RtlNtStatusToDosError", "RtlNtStatusToDosErrorNoTeb"]
        .into_iter()
        .find(|name| {
            resolve(&read.table, name).is_ok_and(|resolved| {
                resolved
                    .import
                    .as_ref()
                    .is_some_and(|import| import.by_name)
            }) && exports
                .find(name)
                .is_some_and(|export| export.forwarder.is_none())
        })
        .expect("no candidate ntdll function is imported by name and exported directly");
    let contracts = BTreeMap::from([(
        "ntdll.qv".to_owned(),
        module(&[contract(name, &[("status", "i32")], "u32")]),
    )]);
    let target = msvc_target();
    let artifact_ref = import_library.display().to_string();
    let build = os_build();
    let inputs = qcore_c_gateway::linkage::generate::GenerationInputs {
        library: "ntdll",
        version: "sdk",
        linkage: LinkageKind::Dynamic,
        artifact: &artifact_ref,
        header: None,
        target: &target,
        unwinding: "abort",
        allocator_custody: "closed",
        thread_posture: "single_caller",
        contracts: &contracts,
        os_build: Some(&build),
    };
    let system = system32();
    let runtime_loader =
        move |dll: &str| std::fs::read(system.join(dll)).map_err(|e| e.to_string());
    let generated =
        qcore_c_gateway::linkage::generate::generate(&inputs, &loader, Some(&runtime_loader))
            .unwrap_or_else(|refusal| panic!("{refusal}"));
    assert!(
        generated.text.contains("runtime dll ntdll.dll os_build "),
        "{}",
        generated.text
    );
    let selection = BTreeMap::from([(
        import_library
            .with_file_name("ntdll.qrlib")
            .display()
            .to_string(),
        generated.text.clone(),
    )]);
    let source = program(
        Some("ntdll"),
        "",
        &format!("extern fn {name}(status: i32) -> u32;\n"),
        &format!(
            "    let ok = {name}(0) succeeds 0;\n    print dec ok;\n    let denied = {name}(-1073741790) succeeds 5;\n    print dec denied;\n"
        ),
    );
    let module = compile(
        &scratch.path().join("main.qr"),
        &source,
        &contracts,
        Some(selection),
        &target,
    )
    .unwrap_or_else(|errors| panic!("{errors:#?}"));
    let manifest =
        qcore_core::native_library::load_native_library_manifest(&generated.text).unwrap();
    let plan = qcore_c_gateway::linkage::link::plan(&manifest, &|reference: &str| {
        PathBuf::from(reference)
    })
    .unwrap();
    let run =
        windows_c::run_msvc_with_libraries(module.c_source.as_deref().unwrap(), &[&plan.input]);
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    assert_eq!(String::from_utf8_lossy(&run.stdout), "0\n5\n");
}
