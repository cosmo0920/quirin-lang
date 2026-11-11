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

//! Card `W48-QR-FLOATING-POINT`, items 2 and 3 on `source.ffi.v1`: `struct!`
//! fields of C type `float`/`double` listed as `f32`/`f64`, `#c_ffi` contract
//! parameters and results of `f32`/`f64`, float literals at every scalar
//! position of the route, float outcomes compared bit for bit, and float
//! renderings — against the card's own small C library, built by the recorded
//! compiler at `-O0` and `-O2` under UBSan and at plain `-O2`,
//! `-ffp-contract=off`, all `-Wall -Wextra -Werror -pedantic`.
//!
//! No test here writes output of its own.
use std::collections::BTreeMap;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;

use qcore_compiler::scalar_floats as sf;
use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
use qcore_core::std_io::Stream;

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

const HEADER: &str = "#ifndef QR_POINT_H\n#define QR_POINT_H\n#include <stdint.h>\nstruct qr_point {\n    double x;\n    float y;\n    int32_t n;\n};\ndouble qr_point_sum(const struct qr_point *p);\nstruct qr_point qr_point_scale(struct qr_point p, double k);\nfloat qr_half(float v);\n#endif\n";

const LIBRARY: &str = "#include \"qr_point.h\"\ndouble qr_point_sum(const struct qr_point *p) { return p->x + (double)p->y + (double)p->n; }\nstruct qr_point qr_point_scale(struct qr_point p, double k) { struct qr_point q = p; q.x = p.x * k; q.y = p.y * (float)k; return q; }\nfloat qr_half(float v) { return v / 2; }\n";

const CONTRACTS: &str = "module contract;\n\n[| quant #c_ffi |]\nextern fn qr_point_sum(p: ptr<const, c\"struct qr_point\">) -> f64\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(p);\n    ensures normal resource_return_none();\n};\n\n[| quant #c_ffi |]\nextern fn qr_point_scale(p: c\"struct qr_point\", k: f64) -> c\"struct qr_point\"\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(p);\n    ensures normal resource_borrowed(k);\n    ensures normal resource_return_none();\n};\n\n[| quant #c_ffi |]\nextern fn qr_half(v: f32) -> f32\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(v);\n    ensures normal resource_return_none();\n};\n";

// Card `W48-QRLIB-LINKAGE`: every `#c_ffi` call resolves through the
// generated `.qrlib` the file binds.
const HEAD: &str = "#import std.ffi.unsafe;\n#import std.extern;\n#import std.fmt;\n#import std.io;\n#import std.logger;\nextern library qr_point;\n";
const IMPORT: &str = "struct! { type Point = c\"struct qr_point\" from \"qr_point.h\"; fields { x: f64, y: f32, n: i32 } }\n";
const DECLARATIONS: &str = "extern fn qr_point_sum(p: ptr<const, Point>) -> f64;\nextern fn qr_point_scale(p: Point, k: f64) -> Point;\nextern fn qr_half(v: f32) -> f32;\n";

#[path = "../../tests/support/qrlib.rs"]
mod qrlib;

struct Scratch(PathBuf, qrlib::Generated);

fn contracts() -> BTreeMap<String, String> {
    BTreeMap::from([("qr_point.qv".to_owned(), CONTRACTS.to_owned())])
}

impl Scratch {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-float-ffi-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("qr_point.h"), HEADER).unwrap();
        std::fs::write(dir.join("qr_point.c"), LIBRARY).unwrap();
        // Card `W48-QRLIB-LINKAGE`: the library is built, and its manifest
        // generated from the archive, before anything is checked.
        let archive = qrlib::static_library(&dir, "qr_point", &[dir.join("qr_point.c")], &[&dir]);
        let generated = qrlib::manifest(
            "qr_point",
            qcore_core::native_library::LinkageKind::Static,
            &archive,
            Some(&dir.join("qr_point.h")),
            &host_target(),
            &contracts(),
        );
        Self(dir, generated)
    }

    fn compile(&self, source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
        compile_files_for_target(
            &[SourceFile::new(
                self.0.join("main.qr").display().to_string(),
                source,
            )],
            &CompileOptions {
                extern_contracts: Some(contracts()),
                native_libraries: Some(self.1.selection()),
                ..CompileOptions::default()
            },
            Some(&host_target()),
        )
    }

    /// The pinned archive, verified: the only library the build links.
    #[cfg(unix)]
    fn archive(&self) -> PathBuf {
        self.1.link_plan().input
    }

    #[cfg(unix)]
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A `.qr` helper of the FFI file taking an imported record and a float by
/// value and returning a float (card `W48-QR-USABLE-RECORDS`'s helpers).
const HELPER: &str = "fn pick(p: Point, k: f64) -> f64 {\n    let x = p.x;\n    return k;\n}\n";

fn program(body: &str) -> String {
    format!(
        "{HEAD}{IMPORT}{DECLARATIONS}{HELPER}fn main() -> unit effects {{ io }} {{\n{body}    return;\n}}\n"
    )
}

const BODY: &str = "    let p = Point { x: 1.5, y: -0.25, n: 3 };\n    let x = p.x;\n    print dec x;\n    let y = p.y;\n    print hex y;\n    let s = qr_point_sum(p) succeeds 4.25;\n    print dec s;\n    let q = qr_point_scale(p, 2.0) succeeds Point { x: 3.0, y: -0.5, n: 3 };\n    let qx = q.x;\n    log info dec qx;\n    let h = qr_half(1e-45) succeeds 0.0;\n    print dec h;\n    let k = pick(p, -2.5);\n    print dec k;\n    flush;\n";

#[test]
fn float_fields_calls_and_outcomes_check_and_render() {
    let scratch = Scratch::new();
    let module = scratch
        .compile(&program(BODY))
        .unwrap_or_else(|e| panic!("refused: {e:#?}"));
    assert_eq!(module.execution_profile, SourceExecutionProfile::FfiV1);
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    assert_eq!(
        qcore_core::ffi::reference_stream(&checked.certificate, Stream::Out),
        format!(
            "1.5\n{}\n4.25\n0\n-2.5\n",
            sf::render_hex(ScalarType::F32, sf::of_f32(-0.25)).unwrap()
        )
    );
    assert_eq!(
        qcore_core::ffi::reference_stream(&checked.certificate, Stream::Err),
        "[INFO] 3\n"
    );
    assert_eq!(module.float_assumption(), Some(sf::ASSUMPTION));
}

#[test]
fn negative_controls_refuse() {
    let scratch = Scratch::new();
    for (body, needle) in [
        (
            "    let p = Point { x: 1, y: 0.5, n: 3 };\n",
            "is not a `f64` value",
        ),
        (
            "    let p = Point { x: 1.0, y: 0.5, n: 3.0 };\n",
            "is not a `i32` value",
        ),
        (
            "    let h = qr_half(1.0) succeeds 1;\n",
            "is not a `f32` literal",
        ),
        ("    let h = qr_half(4e38);\n", "is not a `f32` value"),
    ] {
        // Without the helper, which only the admitted program calls.
        let source = program(body).replace(HELPER, "");
        let errors = scratch.compile(&source).expect_err(body);
        assert!(
            errors.iter().any(|error| error.message.contains(needle)),
            "`{needle}` for\n{body}\n{errors:#?}"
        );
    }
    // A `double` member listed as `f32` disagrees with the recorded compiler.
    let wrong = program("    return;\n").replace("x: f64", "x: f32");
    assert!(scratch.compile(&wrong).is_err());
}

#[cfg(unix)]
#[test]
fn the_program_runs_against_the_card_library_under_three_builds() {
    use std::process::Command;
    let scratch = Scratch::new();
    let module = scratch.compile(&program(BODY)).unwrap();
    let qcore_compiler::CheckedSourceModule::Ffi(checked) = &module.checked else {
        panic!("not the FFI route");
    };
    let stdout = qcore_core::ffi::reference_stream(&checked.certificate, Stream::Out);
    let stderr = qcore_core::ffi::reference_stream(&checked.certificate, Stream::Err);
    let c = module.c_source.clone().unwrap();
    let builds: [&[&str]; 3] = [
        &[
            "-O0",
            "-fsanitize=undefined,integer",
            "-fno-sanitize=float-divide-by-zero",
            "-fno-sanitize-recover=all",
        ],
        &[
            "-O2",
            "-fsanitize=undefined,integer",
            "-fno-sanitize=float-divide-by-zero",
            "-fno-sanitize-recover=all",
        ],
        &["-O2"],
    ];
    std::fs::write(scratch.path().join("main.c"), &c).unwrap();
    for flags in builds {
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let out = Command::new(&compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-ffp-contract=off",
            ])
            .args(flags)
            .arg("-I")
            .arg(scratch.path())
            .arg(scratch.path().join("main.c"))
            .arg(scratch.archive())
            .arg("-o")
            .arg(scratch.path().join("main"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{flags:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let run = Command::new(scratch.path().join("main")).output().unwrap();
        assert_eq!(
            run.status.code(),
            Some(0),
            "{flags:?}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), stdout, "{flags:?}");
        assert_eq!(String::from_utf8_lossy(&run.stderr), stderr, "{flags:?}");
    }
    // A float outcome the library does not produce stops the run at 70 before
    // its line: `-0.0` is not `0.0`, bit for bit.
    let wrong = scratch
        .compile(&program(&BODY.replace("succeeds 0.0", "succeeds -0.0")))
        .unwrap();
    std::fs::write(scratch.path().join("main.c"), wrong.c_source.unwrap()).unwrap();
    let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
    let out = Command::new(&compiler)
        .args([
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
            "-ffp-contract=off",
            "-O0",
        ])
        .arg("-I")
        .arg(scratch.path())
        .arg(scratch.path().join("main.c"))
        .arg(scratch.archive())
        .arg("-o")
        .arg(scratch.path().join("main"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let run = Command::new(scratch.path().join("main")).output().unwrap();
    assert_eq!(run.status.code(), Some(70));
}
