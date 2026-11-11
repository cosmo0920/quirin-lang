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

//! Card `W48-QR-FLOATING-POINT`, item 2 (extern values) and item 3 (the
//! extern contract type table): `f32`/`f64` on `source.extern-call.v1` and the
//! composed extern + std.io route.
//!
//! * A declaration may state `f32`/`f64` parameters and results (the extern
//!   route's `AbiType` now has them); a bound float result is a value of the
//!   namespace; a float literal is an argument at a float parameter, a declared
//!   outcome of a float result (`succeeds 1.5`) and an `f64` binding
//!   (`let x = 1.5;`); value statements compute with floats under the float
//!   rules, their comparisons gated by the entry's `partial_eq`/`partial_ord`.
//! * **Known path.** Declared outcomes and statements over them fold at check
//!   time and render at their type; the emitted program, linked against the
//!   card's own C callees taking and returning `double`/`float`, compares every
//!   real result with its declared outcome bit for bit and prints the
//!   reference's bytes under three builds.
//! * **Dynamic path.** An undeclared float result enters arithmetic,
//!   comparisons and a float → int conversion; the linked callee checks each
//!   result bit for bit (a NaN as a NaN), and a conversion of NaN, ±inf or an
//!   out-of-range value exits with `ArithmeticError::OutOfRange`'s `80 + 3`.
//! * Negative controls: an integer literal at a float parameter and a float
//!   literal at an integer one, an integer outcome of a float result, mixed
//!   `f32`/`f64` operands, a float comparison without its effect, `%` of
//!   floats, a literal that rounds to infinity.
//!
//! No test here writes output of its own.
use std::collections::BTreeMap;

use qcore_compiler::scalar_floats as sf;
use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};

fn host_target() -> RecordedTarget {
    RecordedTarget {
        pointer_width: if cfg!(target_pointer_width = "64") {
            PointerWidth::W64
        } else {
            PointerWidth::W32
        },
        // The host the native rows build and run on; the float route records
        // the triple and reads only the pointer width (card W48-QRLIB-MACOS).
        triple: if cfg!(target_os = "macos") {
            format!("{}-apple-darwin", std::env::consts::ARCH)
        } else {
            "x86_64-unknown-linux-gnu".into()
        },
        compiler: "the host C compiler".into(),
    }
}

#[cfg(unix)]
const FLOATS: [ScalarType; 2] = [ScalarType::F32, ScalarType::F64];

/// Per float type: `q_seq_T()` returns the next bit pattern of `QSEQ`,
/// `q_expect_T(v)` exits 99 unless `v` has the next pattern (a NaN matching any
/// NaN), `q_half_T(x)` returns `x / 2`; and `q_seq_i32`/`q_expect_i32` for the
/// conversions' results.
fn contracts() -> String {
    let mut text = String::from("module contract;\n\n");
    for ty in ["f32", "f64", "i32"] {
        text.push_str(&format!(
            "extern fn q_seq_{ty}() -> {ty}\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_return_none();\n}};\n\nextern fn q_expect_{ty}(value: {ty}) -> unit\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(value);\n    ensures normal resource_return_none();\n}};\n\n"
        ));
    }
    for ty in ["f32", "f64"] {
        text.push_str(&format!(
            "extern fn q_half_{ty}(x: {ty}) -> {ty}\ncontract {{\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(x);\n    ensures normal resource_return_none();\n}};\n\n"
        ));
    }
    text
}

const DECLARATIONS: &str = "extern fn q_seq_f32() -> f32;\nextern fn q_expect_f32(value: f32) -> unit;\nextern fn q_seq_f64() -> f64;\nextern fn q_expect_f64(value: f64) -> unit;\nextern fn q_seq_i32() -> i32;\nextern fn q_expect_i32(value: i32) -> unit;\nextern fn q_half_f32(x: f32) -> f32;\nextern fn q_half_f64(x: f64) -> f64;\n";

/// The card's own C callees.
#[cfg(unix)]
fn callees_c() -> String {
    let mut text = String::from(
        "#include <stdint.h>\n#include <stdlib.h>\n#include <string.h>\n\nstatic const char *q_next(void)\n{\n    static const char *cursor = NULL;\n    static int started = 0;\n    if (!started) {\n        cursor = getenv(\"QSEQ\");\n        started = 1;\n    }\n    if (cursor == NULL || *cursor == '\\0') {\n        exit(98);\n    }\n    const char *token = cursor;\n    const char *comma = strchr(cursor, ',');\n    cursor = comma != NULL ? comma + 1 : cursor + strlen(cursor);\n    return token;\n}\nint32_t q_seq_i32(void);\nvoid q_expect_i32(int32_t value);\nint32_t q_seq_i32(void) { return (int32_t)strtoll(q_next(), NULL, 10); }\nvoid q_expect_i32(int32_t value) { if (value != (int32_t)strtoll(q_next(), NULL, 10)) { exit(99); } }\n",
    );
    for (ty, c, word, exponent, fraction) in [
        ("f32", "float", "uint32_t", "0x7F800000u", "0x007FFFFFu"),
        (
            "f64",
            "double",
            "uint64_t",
            "UINT64_C(0x7FF0000000000000)",
            "UINT64_C(0x000FFFFFFFFFFFFF)",
        ),
    ] {
        text.push_str(&format!(
            "{c} q_seq_{ty}(void);\nvoid q_expect_{ty}({c} value);\n{c} q_half_{ty}({c} x);\n{c} q_seq_{ty}(void)\n{{\n    {word} bits = ({word})strtoull(q_next(), NULL, 16);\n    {c} v;\n    memcpy(&v, &bits, sizeof v);\n    return v;\n}}\nstatic int q_nan_{ty}({word} b) {{ return (b & {exponent}) == {exponent} && (b & {fraction}) != 0; }}\nvoid q_expect_{ty}({c} value)\n{{\n    {word} got;\n    {word} want = ({word})strtoull(q_next(), NULL, 16);\n    memcpy(&got, &value, sizeof got);\n    if (!(got == want || (q_nan_{ty}(got) && q_nan_{ty}(want)))) {{\n        exit(99);\n    }}\n}}\n{c} q_half_{ty}({c} x) {{ return x / 2; }}\n"
        ));
    }
    text
}

fn compile(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("float_extern.qr", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: Some(BTreeMap::from([(
                "float_extern_contracts.qcore".to_owned(),
                contracts(),
            )])),
            native_libraries: None,
            native_library_headers: None,
        },
        Some(&host_target()),
    )
}

fn admitted(source: &str, profile: SourceExecutionProfile) -> CompiledModule {
    let m = compile(source).unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"));
    assert_eq!(m.execution_profile, profile, "{source}");
    m
}

fn refused_at(source: &str, code: &str, needle: &str) {
    let e = compile(source).expect_err(source);
    assert!(
        e.iter()
            .any(|d| d.code == code && d.message.contains(needle)),
        "expected {code} `{needle}` for\n{source}\n{e:?}"
    );
}

fn lines(m: &CompiledModule) -> Vec<String> {
    m.extern_observation.clone().unwrap_or_default()
}

const COMPOSED: &str = "#import std.extern;\n#import std.fmt;\n#import std.io;\n#import std.logger;\nerror Failure { Failed; }\n";
const EXTERN: &str = "#import std.extern;\nerror Failure { Failed; }\n";

/// The known-path program: literal arguments, declared outcomes, an `f64`
/// literal binding, an `f32` binding through `as`, arithmetic, a comparison
/// under `partial_ord`, a conversion, each rendered.
fn known_source() -> String {
    format!(
        "{COMPOSED}{DECLARATIONS}fn main() -> unit effects {{ io, partial_ord }} {{\n    let h = q_half_f64(3.0) succeeds 1.5;\n    let q = q_half_f32(-0.0) succeeds -0.0;\n    let x = 0.1;\n    let s = h + x;\n    let t = 0.1 as f32;\n    let u = t * 3.0;\n    let big = s > 1.5;\n    let n = h as i32;\n    let w = u as f64;\n    print dec h;\n    print hex h;\n    print dec q;\n    print dec s;\n    print hex t;\n    print dec u;\n    print dec big;\n    print dec n;\n    log info dec w;\n    flush;\n    return;\n}}\n"
    )
}

#[test]
fn known_float_values_fold_and_render_at_their_type() {
    let m = admitted(
        &known_source(),
        SourceExecutionProfile::ComposedStdIoExternV1,
    );
    let t = 0.1f32;
    let u = t * 3.0;
    assert_eq!(
        lines(&m),
        [
            "out: 1.5".to_owned(),
            "out: 0x1.8p+0".to_owned(),
            "out: -0".to_owned(),
            format!(
                "out: {}",
                sf::render_dec(ScalarType::F64, sf::of_f64(1.5 + 0.1)).unwrap()
            ),
            "out: 0x1.99999ap-4".to_owned(),
            format!(
                "out: {}",
                sf::render_dec(ScalarType::F32, sf::of_f32(u)).unwrap()
            ),
            "out: true".to_owned(),
            "out: 1".to_owned(),
            format!(
                "err: [INFO] {}",
                sf::render_dec(ScalarType::F64, sf::of_f64(f64::from(u))).unwrap()
            ),
        ]
    );
    assert_eq!(m.float_assumption(), Some(sf::ASSUMPTION));
}

#[test]
fn negative_controls_refuse() {
    for (body, code, needle) in [
        (
            "let h = q_half_f64(3) succeeds 1.5;",
            "Q024",
            "an integer literal is not a `f64`",
        ),
        (
            "q_expect_i32(1.5);",
            "Q024",
            "is the float literal `1.5` and the parameter is declared `i32`",
        ),
        (
            "let h = q_half_f64(3.0) succeeds 2;",
            "Q024",
            "`succeeds` names an integer literal",
        ),
        (
            "let n = q_seq_i32() succeeds 2.0;",
            "Q024",
            "`succeeds` names a float literal",
        ),
        (
            "let a = q_seq_f32(); let b = q_seq_f64(); let c = a + b;",
            "Q024",
            "the operands are a `f32` and a `f64`",
        ),
        (
            "let a = q_seq_f64(); let c = a == a;",
            "Q024",
            "needs the `partial_eq` effect",
        ),
        (
            "let a = q_seq_f64(); let c = a < 1.0;",
            "Q024",
            "needs the `partial_ord` effect",
        ),
        (
            "let a = q_seq_f64(); let c = a % 2.0;",
            "Q024",
            "`%` is an integer operation",
        ),
        ("let a = q_half_f64(1e400);", "Q024", "rounds to infinity"),
        (
            "let a = q_half_f64(1.5); let b = a + 1;",
            "Q024",
            "an integer literal is not a `f64`",
        ),
        (
            "let a = q_seq_f64(); q_expect_f32(a);",
            "Q024",
            "argument 0 is a `f64` and the parameter is declared `f32`",
        ),
    ] {
        refused_at(
            &format!(
                "{EXTERN}{DECLARATIONS}fn main() -> unit effects {{ io }} {{\n    {body}\n    return;\n}}\n"
            ),
            code,
            needle,
        );
    }
    // A run-time float → int conversion without `throws<ArithmeticError>`.
    refused_at(
        &format!(
            "{EXTERN}{DECLARATIONS}fn main() -> unit effects {{ io }} {{\n    let a = q_seq_f64();\n    let n = a as i32;\n    return;\n}}\n"
        ),
        "Q024",
        "`throws<ArithmeticError>`",
    );
    // A known float → int conversion out of range is refused at check time.
    refused_at(
        &format!(
            "{EXTERN}{DECLARATIONS}fn main() -> unit effects {{ io }} {{\n    let a = q_half_f64(1e300) succeeds 5e299;\n    let n = a as i32;\n    return;\n}}\n"
        ),
        "Q024",
        "`ArithmeticError::OutOfRange`",
    );
}

#[cfg(unix)]
mod native {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

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
        dir: std::path::PathBuf,
        pub binary: std::path::PathBuf,
    }
    impl Drop for Built {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    pub fn build(source: &str, callees: &str, flags: &[&str]) -> Result<Built, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-float-extern-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), source).unwrap();
        std::fs::write(dir.join("callees.c"), callees).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let out = Command::new(compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-ffp-contract=off",
            ])
            .args(flags)
            .arg(dir.join("main.c"))
            .arg(dir.join("callees.c"))
            .arg("-o")
            .arg(dir.join("main"))
            .output()
            .expect("Clang is required for this native check");
        let binary = dir.join("main");
        let built = Built { dir, binary };
        if out.status.success() {
            Ok(built)
        } else {
            Err(String::from_utf8_lossy(&out.stderr).into_owned())
        }
    }
}

/// The known program linked against the card's own C functions taking and
/// returning `double` and `float`: each real result is compared with its
/// declared outcome bit for bit before anything reads it, and the streams are
/// the reference's bytes.
#[cfg(unix)]
#[test]
fn known_program_links_real_float_callees_under_three_builds() {
    let m = admitted(
        &known_source(),
        SourceExecutionProfile::ComposedStdIoExternV1,
    );
    let reference = lines(&m);
    let project = |stream: &str| {
        reference
            .iter()
            .filter_map(|line| line.strip_prefix(&format!("{stream}: ")))
            .map(|line| format!("{line}\n"))
            .collect::<String>()
    };
    let c = m.c_source.clone().unwrap();
    for (label, flags) in native::BUILDS {
        let built =
            native::build(&c, &callees_c(), flags).unwrap_or_else(|e| panic!("{label}: {e}"));
        let out = std::process::Command::new(&built.binary).output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "{label}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            project("out"),
            "{label}"
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            project("err"),
            "{label}"
        );
    }
    // A callee that disagrees with its declared outcome stops the run at 70
    // before the value is read: `q_half_f64(3.0)` declared `-0.0`'s twin.
    let wrong = known_source().replace("succeeds 1.5", "succeeds 1.25");
    let m = admitted(&wrong, SourceExecutionProfile::ComposedStdIoExternV1);
    let built = native::build(&m.c_source.unwrap(), &callees_c(), &["-O0"]).unwrap();
    let out = std::process::Command::new(&built.binary).output().unwrap();
    assert_eq!(out.status.code(), Some(70));
    assert!(out.stdout.is_empty());
}

/// The boundary table of each float type, as bit patterns.
#[cfg(unix)]
fn boundary(ty: ScalarType) -> Vec<u64> {
    let (sign, positives): (u64, Vec<u64>) = if ty == ScalarType::F32 {
        (
            0x8000_0000,
            vec![
                0,
                1,
                0x007F_FFFF,
                0x0080_0000,
                0x7F7F_FFFF,
                0x3F80_0000,
                0x7F80_0000,
                0x4B80_0001,
            ],
        )
    } else {
        (
            0x8000_0000_0000_0000,
            vec![
                0,
                1,
                0x000F_FFFF_FFFF_FFFF,
                0x0010_0000_0000_0000,
                0x7FEF_FFFF_FFFF_FFFF,
                0x3FF0_0000_0000_0000,
                0x7FF0_0000_0000_0000,
                0x4340_0000_0000_0001,
            ],
        )
    };
    let mut values = Vec::new();
    for p in positives {
        values.push(p);
        values.push(p | sign);
    }
    values.push(if ty == ScalarType::F32 {
        0x7FC0_0000
    } else {
        0x7FF8_0000_0000_0000
    });
    values
}

/// Undeclared float results through arithmetic, negation and the conversion to
/// `i32`, over every boundary pair: the callee checks every result against the
/// Rust reference (the host's IEEE operation), and a conversion of NaN, ±inf or
/// an out-of-range value exits `80 + 3` (`OutOfRange`).
#[cfg(unix)]
#[test]
fn dynamic_float_values_take_the_reference_under_three_builds() {
    use qcore_compiler::scalar_types::{self as st, Op};
    for ty in FLOATS {
        let symbols = [
            (Op::Add, "+"),
            (Op::Sub, "-"),
            (Op::Mul, "*"),
            (Op::Div, "/"),
        ];
        let mut body = String::new();
        for (op, symbol) in symbols {
            body.push_str(&format!(
                "    let a{0} = q_seq_{ty}();\n    let b{0} = q_seq_{ty}();\n    let z{0} = a{0} {symbol} b{0};\n    q_expect_{ty}(z{0});\n",
                op.name()
            ));
        }
        body.push_str(&format!("    let an = q_seq_{ty}();\n    let zn = -an;\n    q_expect_{ty}(zn);\n    let ac = q_seq_{ty}();\n    let bc = q_seq_{ty}();\n    let lt = ac < bc;\n    let eq = ac == bc;\n    let ai = q_seq_{ty}();\n    let zi = ai as i32;\n    q_expect_i32(zi);\n"));
        let source = format!(
            "{EXTERN}{DECLARATIONS}fn main() -> unit effects {{ io, partial_eq, partial_ord, throws<ArithmeticError> }} {{\n{body}    return;\n}}\n"
        );
        let m = admitted(&source, SourceExecutionProfile::ExternCallsV1);
        let c = m.c_source.clone().unwrap();
        let hex = |bits: u64| format!("{bits:x}");
        let values = boundary(ty);
        let mut runs: Vec<(String, i32)> = Vec::new();
        for &a in &values {
            for &b in &values {
                let mut sequence = Vec::new();
                for (op, _) in symbols {
                    let z = st::checked(op, ty, i128::from(a), i128::from(b), None).unwrap();
                    sequence.extend([hex(a), hex(b), hex(z as u64)]);
                }
                let z = st::negate(ty, i128::from(a), None).unwrap();
                sequence.extend([hex(a), hex(z as u64), hex(a), hex(b), hex(a)]);
                match st::convert(ty, ScalarType::I32, i128::from(a), None) {
                    Some(n) => {
                        sequence.push(n.to_string());
                        runs.push((sequence.join(","), 0));
                    }
                    None => runs.push((sequence.join(","), 83)),
                }
            }
        }
        for (label, flags) in native::BUILDS {
            let built = native::build(&c, &callees_c(), flags)
                .unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
            for (sequence, status) in &runs {
                let out = std::process::Command::new(&built.binary)
                    .env("QSEQ", sequence)
                    .output()
                    .unwrap();
                assert!(
                    out.stdout.is_empty() && out.stderr.is_empty(),
                    "{ty} {label} {sequence}: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                assert_eq!(out.status.code(), Some(*status), "{ty} {label} {sequence}");
            }
        }
    }
}
