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

//! Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: `f32`/`f64` on `source.buffer.v1`,
//! on the composed string route, in `Buffer<T>` payloads and in `protected T`.
//!
//! * **The buffer route** (§G02k, §G07.12, §G07.13): float record fields
//!   (float literals at a float field, field reads and writes), conversions
//!   (`as f32`/`as f64`, float → integer truncating and refused when it does
//!   not fit), the literal binding `let x = 1.5;` (an `f64`), value statements
//!   with float literals and operands, and loop bodies; comparisons under the
//!   entry row's `partial_eq`/`partial_ord`. Every value of the route is a
//!   check-time constant, so every statement is folded by the shared decision
//!   and the emitted C computes it again through §G02t's helpers; the native
//!   run prints what the reference prints, bit for bit through `hex`.
//! * **The composed string route**: the same tier through the buffer route's
//!   reader and emitter, with the entry row's comparison effects.
//! * **Payloads** (§G02b, §G02c): `Buffer<f32>`/`Buffer<f64>`, whose lease
//!   extents are 4 and 8 bytes; a byte view of a float payload follows the
//!   byte rules, and every byte pattern is a value; `protected f32`/`f64`.
//! * **Native**: Clang `-O0` and `-O2` under
//!   `-fsanitize=undefined,integer -fno-sanitize=float-divide-by-zero` and
//!   plain `-O2`, all `-std=c11 -Wall -Wextra -Werror -pedantic
//!   -ffp-contract=off`; ThreadSanitizer over the payload rows with workers,
//!   through the pthread binding (`TASK-ROUTE-TSAN-PTHREAD-BINDING`).
//! * **Negative controls**: a comparison without its effect, a mixed
//!   `f32`/`f64` operation, an implicit int ↔ float, a byte write or lease that
//!   splits a float of the other size, a float loop bound.
//!
//! No program writes a debug print: output is `std.fmt`/`std.io`/`std.logger`
//! on the buffer and string routes and the maintained diagnostic sink on the
//! task route.

#[cfg(unix)]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;
#[cfg(unix)]
#[path = "../../tests/support/mutex_observation.rs"]
mod mutex_observation;
#[cfg(target_os = "linux")]
#[path = "../../tests/support/thread_sanitizer.rs"]
mod thread_sanitizer;

use qcore_compiler::scalar_types::{PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_task_module, render_task_observation,
};
use qcore_core::scalar_types as st;

const FLOATS: [ScalarType; 2] = [ScalarType::F32, ScalarType::F64];
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
const BUFFER: &str =
    "#import std.buffer;\n#import std.fmt;\n#import std.io;\n#import std.logger;\n";
const STRING: &str = "#import std.string;\n#import std.buffer;\n#import std.fmt;\n#import std.io;\n#import std.logger;\n";
const TASKS: &str = "#import std.concurrency;\nconcurrency checked;\n";
const ENTRY: &str = "alloc, throws<AllocError>, spawn, join, may_block";

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

fn compile_for(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files_for_target(
        &[SourceFile::new("float_buffer_payloads.qr", source)],
        &CompileOptions::default(),
        Some(&host_target()),
    )
}

fn admitted(source: &str) -> CompiledModule {
    compile_for(source).unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"))
}

fn refused(source: &str, code: &str, needle: &str) {
    let errors = compile_for(source).expect_err(source);
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

/// The boundary table of a float type, as bit patterns: ±0, ±min subnormal,
/// ±max subnormal, ±min normal, ±max, ±1, ±inf, 2^24−1, 2^24, 2^24+2 (`f32`)
/// or 2^53−1, 2^53, 2^53+2 (`f64`), each signed, and one NaN — the float
/// card's table (`float_types.rs`), 21 values.
fn boundary(ty: ScalarType) -> Vec<i128> {
    let (sign, positives): (u64, Vec<u64>) = match ty {
        ScalarType::F32 => (
            0x8000_0000,
            vec![
                0x0000_0000,
                0x0000_0001,
                0x007F_FFFF,
                0x0080_0000,
                0x7F7F_FFFF,
                0x3F80_0000,
                0x7F80_0000,
                0x4B7F_FFFF,
                0x4B80_0000,
                0x4B80_0001,
            ],
        ),
        _ => (
            0x8000_0000_0000_0000,
            vec![
                0x0000_0000_0000_0000,
                0x0000_0000_0000_0001,
                0x000F_FFFF_FFFF_FFFF,
                0x0010_0000_0000_0000,
                0x7FEF_FFFF_FFFF_FFFF,
                0x3FF0_0000_0000_0000,
                0x7FF0_0000_0000_0000,
                0x433F_FFFF_FFFF_FFFF,
                0x4340_0000_0000_0000,
                0x4340_0000_0000_0001,
            ],
        ),
    };
    let mut values = Vec::new();
    for p in positives {
        values.push(i128::from(p));
        values.push(i128::from(p | sign));
    }
    values.push(qcore_core::scalar_floats::canonical_nan(ty));
    values
}

fn wide(ty: ScalarType, bits: i128) -> f64 {
    match ty {
        ScalarType::F32 => f64::from(f32::from_bits(bits as u32)),
        _ => f64::from_bits(bits as u64),
    }
}

/// Statements binding `name` to the value `bits` of `ty` on the buffer route:
/// a finite value is its shortest literal converted to `ty` (read once at
/// `ty`), an infinity and a NaN are quotients.
fn bind(ty: ScalarType, bits: i128, name: &str) -> String {
    let x = wide(ty, bits);
    if x.is_nan() {
        return format!("    let {name}_z = 0.0 as {ty};\n    let {name} = {name}_z / {name}_z;\n");
    }
    if x.is_infinite() {
        let one = if x < 0.0 { "-1.0" } else { "1.0" };
        return format!(
            "    let {name}_z = 0.0 as {ty};\n    let {name}_o = {one} as {ty};\n    let {name} = {name}_o / {name}_z;\n"
        );
    }
    let literal = match ty {
        ScalarType::F32 => format!("{:e}", f32::from_bits(bits as u32).abs()),
        _ => format!("{:e}", x.abs()),
    };
    let sign = if x.is_sign_negative() { "-" } else { "" };
    format!("    let {name} = {sign}{literal} as {ty};\n")
}

/// What the reference renders for a value: `hex`, exact but for a NaN's
/// payload and sign.
fn hex(ty: ScalarType, bits: i128) -> String {
    st::render(true, ty, bits, None).unwrap()
}

/// One program per left operand `a`: every value bound, then `a OP b` for
/// every `b` and every operation (`hex`), `a CMP b` for every comparison under
/// both effects (`dec`), `-a`, and `a as` every integer type where the
/// truncation fits and the other float type. Returns the program and the
/// stdout an independent reading of it prints.
fn table_program(ty: ScalarType, a: usize) -> (String, Vec<String>) {
    let values = boundary(ty);
    let other = if ty == ScalarType::F32 {
        ScalarType::F64
    } else {
        ScalarType::F32
    };
    let mut body = String::new();
    let mut expected = Vec::new();
    for (i, bits) in values.iter().enumerate() {
        body.push_str(&bind(ty, *bits, &format!("v{i}")));
    }
    let x = values[a];
    for (j, y) in values.iter().enumerate() {
        for (op, symbol) in [
            (st::Op::Add, "+"),
            (st::Op::Sub, "-"),
            (st::Op::Mul, "*"),
            (st::Op::Div, "/"),
        ] {
            let name = format!("r{j}{}", op.name());
            body.push_str(&format!(
                "    let {name} = v{a} {symbol} v{j};\n    print hex {name};\n"
            ));
            let r = st::checked(op, ty, x, *y, None).unwrap();
            expected.push(hex(ty, r));
        }
        for (cmp, symbol) in [
            (st::Cmp::Eq, "=="),
            (st::Cmp::Ne, "!="),
            (st::Cmp::Lt, "<"),
            (st::Cmp::Le, "<="),
            (st::Cmp::Gt, ">"),
            (st::Cmp::Ge, ">="),
        ] {
            let name = format!("c{j}{}", symbol.len() * 10 + cmp as usize);
            body.push_str(&format!(
                "    let {name} = v{a} {symbol} v{j};\n    print dec {name};\n"
            ));
            expected.push(st::compare(cmp, ty, x, *y).unwrap().to_string());
        }
    }
    body.push_str(&format!("    let neg = -v{a};\n    print hex neg;\n"));
    expected.push(hex(ty, st::negate(ty, x, None).unwrap()));
    for to in INTEGERS {
        if let Some(n) = st::convert(ty, to, x, Some(&host_target())) {
            body.push_str(&format!(
                "    let t{to} = v{a} as {to};\n    print dec t{to};\n"
            ));
            expected.push(n.to_string());
        }
    }
    let converted = st::convert(ty, other, x, None).unwrap();
    body.push_str(&format!("    let w = v{a} as {other};\n    print hex w;\n"));
    expected.push(hex(other, converted));
    let source = format!(
        "{BUFFER}fn main() -> unit effects {{ io, alloc, mut, partial_eq, partial_ord }} {{\n    let b = bytes(1);\n{body}    drop b;\n    return;\n}}\n"
    );
    (
        source,
        expected
            .into_iter()
            .map(|line| format!("out: {line}"))
            .collect(),
    )
}

/// Integer → float on the buffer route: every integer type's boundary values,
/// held in a record field of their type, converted to `ty` (`hex`).
fn integer_program(ty: ScalarType) -> (String, Vec<String>) {
    let target = host_target();
    let mut types = String::new();
    let mut body = String::new();
    let mut expected = Vec::new();
    for from in INTEGERS {
        let (mut fields, mut values) = (Vec::new(), Vec::new());
        let (lo, hi) = from.range(Some(&target)).unwrap();
        let mut picks = vec![lo, hi, 0, 1];
        let precision: i128 = if ty == ScalarType::F32 {
            1 << 24
        } else {
            1 << 53
        };
        for delta in [-1, 1, 3] {
            for v in [precision + delta, -(precision + delta)] {
                if lo <= v && v <= hi {
                    picks.push(v);
                }
            }
        }
        picks.sort();
        picks.dedup();
        for (k, v) in picks.iter().enumerate() {
            let field = format!("f{from}{k}");
            fields.push(format!("{field}: {from}"));
            values.push(format!("{field}: {v}"));
            body.push_str(&format!(
                "    let n{from}{k} = r{from}.{field};\n    let x{from}{k} = n{from}{k} as {ty};\n    print hex x{from}{k};\n"
            ));
            expected.push(format!(
                "out: {}",
                hex(ty, st::convert(from, ty, *v, Some(&target)).unwrap())
            ));
        }
        // One record per integer type, inside the 32-field bound.
        types.push_str(&format!("type R{from} {{ {} }}\n", fields.join(", ")));
        body.insert_str(
            0,
            &format!("    let r{from} = R{from} {{ {} }};\n", values.join(", ")),
        );
    }
    let source = format!(
        "{BUFFER}{types}fn main() -> unit effects {{ io, alloc, mut }} {{\n    let b = bytes(1);\n{body}    drop b;\n    return;\n}}\n"
    );
    (source, expected)
}

/// Records, the literal binding and a loop body at `ty`: float fields
/// constructed from literals and names, read, written and read again; a loop
/// computing a float per iteration and comparing it; logged lines on stderr.
fn record_loop_program(ty: ScalarType) -> (String, Vec<String>) {
    let source = format!(
        "{BUFFER}type Sample {{ value: {ty}, weight: {ty}, count: i32 }}
fn main() -> unit effects {{ io, alloc, mut, partial_ord }} {{
    let b = bytes(4);
    let w = 0.25 as {ty};
    let s = Sample {{ value: 1.5e1, weight: w, count: 3 }};
    let v = s.value;
    let half = v / 2.0;
    print dec half;
    s.weight = -1.25e-2;
    let w2 = s.weight;
    print hex w2;
    let wide = 2.5;
    print dec wide;
    for i in 0..4 {{
        let f = i as {ty};
        let g = f * 0.5;
        let h = g - 1.25;
        let lt = h < g;
        log info dec h;
        log info dec lt;
        b[i] = 1;
    }}
    let t = -2.75 as i32;
    print dec t;
    let n = half as u8;
    print dec n;
    drop b;
    flush;
    return;
}}
"
    );
    let at = |x: f64| -> i128 {
        if ty == ScalarType::F32 {
            i128::from((x as f32).to_bits())
        } else {
            i128::from(x.to_bits())
        }
    };
    let dec = |t: ScalarType, bits: i128| st::render(false, t, bits, None).unwrap();
    let mut expected = vec![
        format!("out: {}", dec(ty, at(7.5))),
        format!(
            "out: {}",
            hex(
                ty,
                if ty == ScalarType::F32 {
                    i128::from((-1.25e-2f32).to_bits())
                } else {
                    i128::from((-1.25e-2f64).to_bits())
                }
            )
        ),
        "out: 2.5".to_owned(),
    ];
    for i in 0..4 {
        let h = f64::from(i) * 0.5 - 1.25;
        expected.push(format!("err: [INFO] {}", dec(ty, at(h))));
        expected.push("err: [INFO] true".to_owned());
    }
    expected.push("out: -2".to_owned());
    expected.push("out: 7".to_owned());
    (source, expected)
}

/// The composed string route's typed tier at `ty`.
fn composed_program(ty: ScalarType) -> (String, Vec<String>) {
    let source = format!(
        "{STRING}fn main() -> unit effects {{ io, alloc, mut, partial_ord, partial_eq }} {{
    let b = bytes(4);
    for p in 0..4 {{
        b[p] = 97;
    }}
    let all = borrow_utf8(b);
    print all;
    let x = 2.5 as {ty};
    let k = 3;
    let f = k as {ty};
    let y = x * 4.0;
    let lt = x < y;
    let eq = y == y;
    print dec y;
    log info hex f;
    print dec lt;
    print dec eq;
    for i in 0..k {{
        let g = i as {ty};
        let h = g / 3.0;
        print hex h;
    }}
    let n = y as u8;
    print dec n;
    let z = 0.0;
    print dec z;
    drop b;
    return;
}}
"
    );
    let third = |i: i32| -> i128 {
        if ty == ScalarType::F32 {
            i128::from((i as f32 / 3.0f32).to_bits())
        } else {
            i128::from((f64::from(i) / 3.0).to_bits())
        }
    };
    let mut expected = vec![
        "out: aaaa".to_owned(),
        "out: 10".to_owned(),
        format!(
            "err: [INFO] {}",
            if ty == ScalarType::F32 {
                hex(ty, i128::from(3.0f32.to_bits()))
            } else {
                hex(ty, i128::from(3.0f64.to_bits()))
            }
        ),
        "out: true".to_owned(),
        "out: true".to_owned(),
    ];
    for i in 0..3 {
        expected.push(format!("out: {}", hex(ty, third(i))));
    }
    expected.push("out: 10".to_owned());
    expected.push("out: 0".to_owned());
    (source, expected)
}

// ------------------------------------------------------------ the buffer route

/// Every boundary program is admitted, publishes the IEEE premise, and its
/// reference lines are an independent reading of it: the kernel's operations,
/// conversions and renderings, value by value.
#[test]
fn the_buffer_route_decides_the_boundary_table() {
    for ty in FLOATS {
        for a in 0..boundary(ty).len() {
            let (source, expected) = table_program(ty, a);
            let m = admitted(&source);
            assert_eq!(
                m.execution_profile,
                SourceExecutionProfile::StandardBufferV1
            );
            assert_eq!(lines(&m), expected, "{ty} a={a}");
            assert!(m.float_assumption().is_some(), "{ty}: the IEEE premise");
        }
        let (source, expected) = integer_program(ty);
        assert_eq!(lines(&admitted(&source)), expected, "{ty} from integers");
        let (source, expected) = record_loop_program(ty);
        let m = admitted(&source);
        assert_eq!(lines(&m), expected, "{ty} records and loops");
        let certificate = &m.checked.as_standard_buffer().unwrap().certificate;
        assert!(certificate.float_comparisons.partial_ord);
        assert!(!certificate.float_comparisons.partial_eq);
    }
}

/// A program holding no float keeps its bytes: no float header, environment
/// or assumption (the fixture freeze is the whole-corpus form of this).
#[test]
fn a_buffer_program_without_floats_carries_no_float_text() {
    let m = admitted(&format!(
        "{BUFFER}fn main() -> unit effects {{ io, alloc, mut }} {{\n    let b = bytes(2);\n    let n = 3;\n    let w = n as u8;\n    print dec w;\n    drop b;\n    return;\n}}\n"
    ));
    let c = m.c_source.clone().unwrap();
    assert!(!c.contains("float.h") && !c.contains("FLT_EVAL_METHOD"));
    assert!(m.float_assumption().is_none());
}

#[test]
fn buffer_route_negative_controls() {
    let entry = |row: &str, body: &str| {
        format!(
            "{BUFFER}type P {{ x: f64, y: f32 }}\nfn main() -> unit effects {{ io, alloc, mut{row} }} {{\n    let b = bytes(4);\n{body}    drop b;\n    return;\n}}\n"
        )
    };
    // A comparison without its effect; `==` under `partial_ord` only; an
    // ordering under `partial_eq` only.
    refused(
        &entry("", "    let z = 0.0;\n    let c = z == z;\n"),
        "Q052",
        "needs the `partial_eq` effect",
    );
    refused(
        &entry(", partial_ord", "    let z = 0.0;\n    let c = z != z;\n"),
        "Q052",
        "needs the `partial_eq` effect",
    );
    refused(
        &entry(", partial_eq", "    let z = 0.0;\n    let c = z >= z;\n"),
        "Q052",
        "needs the `partial_ord` effect",
    );
    // A mixed `f32`/`f64` operation, at a statement and at a field.
    refused(
        &entry(
            "",
            "    let a = 1.0 as f32;\n    let z = 0.0;\n    let c = a * z;\n",
        ),
        "Q052",
        "the operands are a `f32` and a `f64`",
    );
    refused(
        &entry(
            "",
            "    let a = 1.0 as f32;\n    let p = P { x: a, y: a };\n",
        ),
        "Q052",
        "`P.x` is declared `f64` and is supplied a `f32`",
    );
    // An implicit int <-> float: an integer literal where a float is wanted,
    // a float literal where an integer is, an integer name at a float field,
    // a float at a byte.
    refused(
        &entry("", "    let z = 0.0;\n    let c = z + 1;\n"),
        "Q052",
        "an integer literal is not a `f64`",
    );
    refused(
        &entry("", "    let n = 3;\n    let c = n + 1.5;\n"),
        "Q052",
        "a float literal is not a `i32`",
    );
    refused(
        &entry("", "    let p = P { x: 1, y: 2.0 };\n"),
        "Q052",
        "an integer literal is not a `f64`",
    );
    refused(
        &entry(
            "",
            "    let p = P { x: 1.0, y: 2.0 };\n    let n = 3;\n    p.x = n;\n",
        ),
        "Q052",
        "is declared `f64` and is supplied a `i32`",
    );
    refused(
        &entry("", "    let f = 1.5;\n    b[0] = f;\n"),
        "Q052",
        "`f` is a `f64` and this position takes an `i32`",
    );
    // A float loop bound, directly and through the lowering's tier.
    refused(
        &entry(
            "",
            "    let f = 4.0;\n    for i in 0..f {\n        print dec i;\n    }\n",
        ),
        "Q098",
        "a float loop bound is refused on every route",
    );
    refused(
        &entry(
            "",
            "    let n = 4;\n    let f = n as f32;\n    for i in 0..f {\n        print dec i;\n    }\n",
        ),
        "Q098",
        "a float loop bound is refused on every route",
    );
    // Float -> integer that does not fit, NaN, an infinity: refused now.
    refused(
        &entry("", "    let f = 3.0e9;\n    let n = f as i32;\n"),
        "Q053",
        "truncated toward zero is not a value of `i32`",
    );
    refused(
        &entry(
            "",
            "    let z = 0.0;\n    let q = z / z;\n    let n = q as u64;\n",
        ),
        "Q053",
        "`nan` (a `f64`)",
    );
    refused(
        &entry(
            "",
            "    let z = 0.0;\n    let o = 1.0;\n    let q = o / z;\n    let n = q as i64;\n",
        ),
        "Q053",
        "`inf` (a `f64`)",
    );
    refused(
        &entry("", "    let n = -1.0e10 as i32;\n"),
        "Q053",
        "OutOfRange",
    );
    // A literal outside its type; no float remainder; no `bool` conversion.
    refused(
        &entry("", "    let x = 1e400;\n"),
        "Q053",
        "rounds to infinity",
    );
    refused(
        &entry("", "    let p = P { x: 1.0, y: 3.5e38 };\n"),
        "Q053",
        "rounds to infinity",
    );
    refused(
        &entry("", "    let f = 1.5;\n    let r = f % 2.0;\n"),
        "Q052",
        "`%` is an integer operation",
    );
    refused(
        &entry("", "    let f = 1.5;\n    let r = f as bool;\n"),
        "Q052",
        "`f64 as bool` is not a conversion",
    );
}

#[test]
fn composed_string_route_negative_controls() {
    let entry = |row: &str, body: &str| {
        format!(
            "{STRING}fn main() -> unit effects {{ io, alloc, mut{row} }} {{\n    let b = bytes(4);\n    for p in 0..4 {{\n        b[p] = 97;\n    }}\n    let all = borrow_utf8(b);\n    print all;\n{body}    drop b;\n    return;\n}}\n"
        )
    };
    refused(
        &entry("", "    let z = 0.0;\n    let c = z < z;\n"),
        "Q077",
        "needs the `partial_ord` effect",
    );
    refused(
        &entry(
            "",
            "    let a = 1.0 as f32;\n    let z = 0.0;\n    let c = a - z;\n",
        ),
        "Q077",
        "the operands are a `f32` and a `f64`",
    );
    refused(
        &entry("", "    let z = 0.0;\n    let c = z + 2;\n"),
        "Q077",
        "an integer literal is not a `f64`",
    );
    refused(
        &entry(
            "",
            "    let f = 2.0;\n    for i in 0..f {\n        print dec i;\n    }\n",
        ),
        "Q098",
        "a float loop bound is refused on every route",
    );
    // A float name at an `i32` position of the string route.
    refused(
        &entry(
            "",
            "    let f = 2.0;\n    let s = all[0:f];\n    print s;\n",
        ),
        "Q077",
        "`f` is a `f64`",
    );
}

#[test]
fn the_composed_string_route_admits_floats() {
    for ty in FLOATS {
        let (source, expected) = composed_program(ty);
        let m = admitted(&source);
        assert_eq!(
            m.execution_profile,
            SourceExecutionProfile::StandardStringV1
        );
        assert_eq!(lines(&m), expected, "{ty}");
        assert!(m.float_assumption().is_some());
    }
}

/// `examples/scalar/float_buffer.qr`: the page publishes the reference's
/// lines, per stream, and the native run under the three builds prints them.
#[test]
fn the_float_buffer_example_prints_its_transcript() {
    let m = admitted(include_str!("../../examples/scalar/float_buffer.qr"));
    assert_eq!(
        m.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    let page = include_str!("../../examples/scalar/README.md").replace("\r\n", "\n");
    for (stream, marker) in [
        ("out", "float_buffer stdout"),
        ("err", "float_buffer stderr"),
    ] {
        let block = page
            .split(&format!("<!-- transcript: {marker} -->"))
            .nth(1)
            .and_then(|rest| rest.split("```text\n").nth(1))
            .and_then(|rest| rest.split("```").next())
            .unwrap_or_else(|| panic!("the page has no `{marker}` transcript"));
        let published = block
            .lines()
            .map(|line| format!("{stream}: {line}"))
            .collect::<Vec<_>>();
        let observed = lines(&m)
            .into_iter()
            .filter(|line| line.starts_with(&format!("{stream}: ")))
            .collect::<Vec<_>>();
        assert_eq!(published, observed, "{marker}");
    }
    #[cfg(unix)]
    {
        use native::*;
        let (out, err) = streams(&lines(&m));
        for (label, flags) in BUILDS {
            let run = build_and_run(m.c_source.as_deref().unwrap(), flags)
                .unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(
                (run.stdout.as_str(), run.stderr.as_str()),
                (out.as_str(), err.as_str())
            );
            assert_eq!(run.status, Some(0), "{label}");
        }
    }
}

// ------------------------------------------------------------ payloads

fn float_literal(ty: ScalarType, bits: i128) -> Option<String> {
    let x = wide(ty, bits);
    if !x.is_finite() {
        return None;
    }
    let literal = match ty {
        ScalarType::F32 => format!("{:e}", f32::from_bits(bits as u32).abs()),
        _ => format!("{:e}", x.abs()),
    };
    Some(format!(
        "{}{literal}",
        if x.is_sign_negative() { "-" } else { "" }
    ))
}

/// Task-route statements binding `name: ty` to `bits` — a literal, or a
/// quotient for a NaN and an infinity (a task value statement).
fn task_bind(ty: ScalarType, bits: i128, name: &str) -> String {
    match float_literal(ty, bits) {
        Some(literal) => format!("    let {name}: {ty} = {literal};\n"),
        None => {
            let x = wide(ty, bits);
            let numerator = if x.is_nan() {
                "0.0"
            } else if x < 0.0 {
                "-1.0"
            } else {
                "1.0"
            };
            format!(
                "    let {name}_n: {ty} = {numerator};\n    let {name}_z: {ty} = 0.0;\n    let {name} = {name}_n / {name}_z;\n"
            )
        }
    }
}

/// A float payload holding `value` moves to a worker that reads it and writes
/// `next`, returns through join, is split between two disjoint 4/8-byte
/// halves' leases whose workers write one byte each, and is read whole.
fn payload_program(ty: ScalarType, value: i128, next: i128) -> String {
    let n = ty.size_bytes(None).unwrap();
    let half = n / 2;
    format!(
        "{TASKS}fn bump(b: own<Buffer<{ty}>> use one, next: {ty}) -> own<Buffer<{ty}>> {{
    let before = qcore_buffer_read(b);
    let wrote = qcore_buffer_write(b, next);
    return move b;
}}
fn poke(part: lend<Buffer<{ty}>>, byte: i32) -> i32 {{
    let wrote = qcore_buffer_write(part, 0, byte);
    let seen = qcore_buffer_read(part, 0);
    return seen;
}}
fn main() -> i32 effects {{ {ENTRY} }} {{
{v}{w}    let x = 171;
    let y = 205;
    let b = qcore_buffer_new(v)?;
    thread scope s {{
        match spawn s bump(move b, copy w) {{
            Result::Ok {{ value: h }} => {{
                let r = join move h?;
                match spawn s poke(lend r 0..{half}, copy x) {{
                    Result::Ok {{ value: first }} => {{
                        match spawn s poke(lend r {half}..{n}, copy y) {{
                            Result::Ok {{ value: second }} => {{
                                let a = join move first?;
                                let c = join move second?;
                                let last = qcore_buffer_read(r, {last});
                                let whole = qcore_buffer_read(r);
                                drop r;
                                return 0;
                            }}
                            Result::Err {{ error: f }} => {{
                                let a = join move first?;
                                drop r;
                                return 1;
                            }}
                        }}
                    }}
                    Result::Err {{ error: f }} => {{
                        drop r;
                        return 1;
                    }}
                }}
            }}
            Result::Err {{ error: f }} => {{
                drop b;
                return 1;
            }}
        }}
    }}
}}
",
        v = task_bind(ty, value, "v"),
        w = task_bind(ty, next, "w"),
        last = n - 1
    )
}

/// The trace spelling of a float payload: `TYPE BITS`, `nan` for every NaN.
fn traced(ty: ScalarType, bits: i128) -> String {
    let number = if qcore_core::scalar_floats::is_nan(ty, bits) {
        "nan".to_owned()
    } else {
        bits.to_string()
    };
    format!("{} {number}", ty.name().to_ascii_uppercase())
}

/// The whole payload after the two byte writes, reconstructed independently:
/// the little-endian bytes of `next`, byte 0 set to 171 and byte `n/2` to 205.
fn after_bytes(ty: ScalarType, next: i128) -> i128 {
    let n = ty.size_bytes(None).unwrap() as usize;
    let mut bytes = (next as u128).to_le_bytes()[..n].to_vec();
    bytes[0] = 171;
    bytes[n / 2] = 205;
    let mut full = [0u8; 16];
    full[..n].copy_from_slice(&bytes);
    u128::from_le_bytes(full) as i128
}

fn reference(module: &CompiledModule) -> String {
    let tasks = module.checked.as_scoped_tasks().expect("a task module");
    render_task_observation(&execute_task_module(tasks, &TaskExecutionScript::default()).unwrap())
}

#[test]
fn float_payloads_round_trip_and_split_into_leases() {
    for ty in FLOATS {
        let values = boundary(ty);
        for (index, value) in values.iter().enumerate() {
            let next = values[(index + 1) % values.len()];
            let module = admitted(&payload_program(ty, *value, next));
            assert!(module.float_assumption().is_some());
            let trace = reference(&module);
            let whole = after_bytes(ty, next);
            for line in [
                format!("QCORE_BUFFER_ALLOC 0 1 {}\n", traced(ty, *value)),
                format!("QCORE_BUFFER_READ 1 1 {}\n", traced(ty, *value)),
                format!("QCORE_BUFFER_WRITE 1 1 {}\n", traced(ty, next)),
                format!("QCORE_BUFFER_READ 0 1 {}\n", traced(ty, whole)),
            ] {
                assert!(trace.contains(&line), "{ty} {value}: {line} in\n{trace}");
            }
            let c = module.c_source.as_deref().unwrap();
            let size = ty.size_bytes(None).unwrap();
            assert!(c.contains(&format!("\"a {ty} payload is {size} byte(s)\"")));
            assert!(c.contains(&format!("qcore_buffer_bits_{ty}(")));
        }
    }
}

#[test]
fn float_payload_negative_controls() {
    for ty in FLOATS {
        let n = ty.size_bytes(None).unwrap();
        let other = if ty == ScalarType::F32 {
            ScalarType::F64
        } else {
            ScalarType::F32
        };
        let reader = |range: &str, index: u64| {
            format!(
                "{TASKS}fn look(part: lend<Buffer<{ty}>>) -> i32 {{\n    let seen = qcore_buffer_write(part, {index}, 7);\n    return 0;\n}}\nfn main() -> i32 effects {{ {ENTRY} }} {{\n    let v: {ty} = 1.5;\n    let b = qcore_buffer_new(v)?;\n    thread scope s {{\n        match spawn s look(lend b {range}) {{\n            Result::Ok {{ value: h }} => {{\n                let a = join move h?;\n                drop b;\n                return a;\n            }}\n            Result::Err {{ error: f }} => {{ drop b; return 0; }}\n        }}\n    }}\n}}\n"
            )
        };
        admitted(&reader(&format!("0..{n}"), n - 1));
        // A lease that splits the payload as a float of the other size would:
        // past the end of an `f32`, or an `f64` lease written past its bytes.
        refused(
            &reader(
                &format!("0..{}", other.size_bytes(None).unwrap().max(n + 1)),
                0,
            ),
            "Q600",
            &format!("stay inside the `{ty}` payload's {n} byte(s)"),
        );
        let entry = |body: &str| {
            format!(
                "{TASKS}fn main() -> i32 effects {{ {ENTRY} }} {{\n    let v: {ty} = 1.5;\n    let b = qcore_buffer_new(v)?;\n{body}    drop b;\n    return 0;\n}}\n"
            )
        };
        // An owner byte read and write past the payload.
        refused(
            &entry(&format!("    let x = qcore_buffer_read(b, {n});\n")),
            "Q600",
            &format!("outside the payload's [0, {n}) extent"),
        );
        // A write of the other float type, and an integer literal (no
        // implicit int -> float), into a float payload.
        refused(
            &entry(&format!(
                "    let x: {other} = 2.5;\n    let w = qcore_buffer_write(b, x);\n"
            )),
            "Q500",
            &format!("requires a `{ty}` value"),
        );
        refused(
            &entry("    let w = qcore_buffer_write(b, 2);\n"),
            "Q020",
            &format!("an integer literal is not a `{ty}`"),
        );
        // A float payload read is not an integer.
        refused(
            &entry("    let x: i32 = qcore_buffer_read(b);\n"),
            "Q020",
            "local result does not match",
        );
        // No conversion at the boundary: a payload into the other float
        // type's parameter.
        refused(
            &format!(
                "{TASKS}fn w(p: own<Buffer<{other}>> use one) -> i32 {{ drop p; return 0; }}\nfn main() -> i32 effects {{ {ENTRY} }} {{\n    let v: {ty} = 1.5;\n    let b = qcore_buffer_new(v)?;\n    thread scope s {{\n        match spawn s w(move b) {{\n            Result::Ok {{ value: h }} => {{\n                let a = join move h?;\n                return 0;\n            }}\n            Result::Err {{ error: f }} => {{ drop b; return 0; }}\n        }}\n    }}\n}}\n"
            ),
            "Q020",
            "capture type does not match",
        );
    }
}

fn protected_program(ty: &str) -> String {
    format!(
        "#import std.concurrency;\nfn main() -> unit {{\n    thread scope s {{\n        let r = protected {ty};\n        let m = mutex guards r;\n        with lock m as g {{ open g; write r with g; close g; read r with g; }}\n        with lock m as h {{ read r with h; }}\n    }}\n}}\n"
    )
}

#[test]
fn a_protected_float_is_its_size() {
    for ty in FLOATS {
        let module = admitted(&protected_program(ty.name()));
        let checked = module.checked.as_checked_mutex_guards().unwrap();
        let size = ty.size_bytes(None).unwrap();
        assert_eq!(u64::from(checked.lowered.resources[0].bytes), size);
        assert_eq!(checked.lowered.resources[0].payload, Some(ty));
        let c = module.c_source.as_deref().unwrap();
        assert!(c.contains(&format!("malloc({size})")));
        assert!(
            c.contains("FLT_EVAL_METHOD == 0"),
            "{ty}: the IEEE environment"
        );
        assert!(module.float_assumption().is_some());
    }
    // `protected f32` is `protected 4` to the model, byte for byte.
    let typed = admitted(&protected_program("f32"));
    let bytes = admitted(&protected_program("4"));
    let (typed, bytes) = (
        typed.checked.as_checked_mutex_guards().unwrap(),
        bytes.checked.as_checked_mutex_guards().unwrap(),
    );
    assert_eq!(typed.lowered.program, bytes.lowered.program);
    assert_eq!(typed.certificate, bytes.certificate);
}

// ------------------------------------------------------------ the Lean rows

/// `QCore.FloatPayloads.floatSizeAgreement`, replayed against the Rust extent.
#[test]
fn lean_float_payload_sizes_match_the_rust_extent() {
    let text = include_str!("../../proof/QCore/FloatPayloads.lean");
    let table = text
        .split("def floatSizeAgreement")
        .nth(1)
        .unwrap()
        .split("theorem floatSizeAgreement_holds")
        .next()
        .unwrap();
    let mut rows = 0;
    for row in table.split("(.").skip(1) {
        let fields = row.split(',').take(2).map(str::trim).collect::<Vec<_>>();
        let ty = ScalarType::parse_admitting_floats(fields[0]).expect("a Lean float type");
        let expected = fields[1]
            .trim_end_matches([')', ']', '\n', ' '])
            .parse::<u64>()
            .unwrap();
        assert_eq!(ty.size_bytes(None), Ok(expected), "{row}");
        assert_eq!(
            qcore_core::task_execution::payload_extent(ty, None),
            Some(expected),
            "{row}"
        );
        rows += 1;
    }
    assert_eq!(rows, 2);
}

// ------------------------------------------------------------ native

#[cfg(unix)]
mod native {
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

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

    pub struct Run {
        pub stdout: String,
        pub stderr: String,
        pub status: Option<i32>,
    }

    pub fn build_and_run(c: &str, flags: &[&str]) -> Result<Run, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48fbp-{}-{}",
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
                "-ffp-contract=off",
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
            .arg("-lm")
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

    /// A buffer/string-route run's two streams against the reference lines.
    pub fn streams(lines: &[String]) -> (String, String) {
        let mut out = String::new();
        let mut err = String::new();
        for line in lines {
            if let Some(text) = line.strip_prefix("out: ") {
                out.push_str(text);
                out.push('\n');
            } else if let Some(text) = line.strip_prefix("err: ") {
                err.push_str(text);
                err.push('\n');
            } else {
                panic!("unframed reference line {line}");
            }
        }
        (out, err)
    }
}

/// The buffer route's boundary table, integer conversions, records and loops,
/// and the composed string route, natively under the three builds: both
/// streams equal the reference, exit 0. Summary rows for the report when
/// `QCORE_W48_FBP_NATIVE_SUMMARY` names a file.
#[cfg(unix)]
#[test]
fn native_buffer_and_string_routes_match_the_reference() {
    use native::*;
    let summary = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for ty in FLOATS {
            for part in 0..4usize {
                let summary = &summary;
                scope.spawn(move || {
                    let mut programs = Vec::new();
                    let count = boundary(ty).len();
                    for a in (0..count).filter(|a| a % 4 == part) {
                        programs.push(("table", table_program(ty, a)));
                    }
                    if part == 0 {
                        programs.push(("integers", integer_program(ty)));
                        programs.push(("records-loops", record_loop_program(ty)));
                        programs.push(("composed-string", composed_program(ty)));
                    }
                    let (mut runs, mut checked_lines) = (0usize, 0usize);
                    for (kind, (source, expected)) in &programs {
                        let m = admitted(source);
                        assert_eq!(&lines(&m), expected, "{ty} {kind}");
                        let (out, err) = streams(expected);
                        let c = m.c_source.as_deref().unwrap();
                        for (label, flags) in BUILDS {
                            let run = build_and_run(c, flags)
                                .unwrap_or_else(|e| panic!("{ty} {kind} {label}: {e}"));
                            assert_eq!(run.stdout, out, "{ty} {kind} {label}: stdout");
                            assert_eq!(run.stderr, err, "{ty} {kind} {label}: stderr");
                            assert_eq!(run.status, Some(0), "{ty} {kind} {label}");
                            runs += 1;
                            checked_lines += expected.len();
                        }
                    }
                    summary.lock().unwrap().push(format!(
                        "{{\"type\": \"{ty}\", \"part\": {part}, \"programs\": {}, \"runs\": {runs}, \"lines\": {checked_lines}, \"builds\": [\"O0-ubsan\", \"O2-ubsan\", \"O2\"], \"sanitizer_reports\": 0}}",
                        programs.len()
                    ));
                });
            }
        }
    });
    let mut rows = summary.into_inner().unwrap();
    rows.sort();
    assert_eq!(rows.len(), 8);
    if let Ok(path) = std::env::var("QCORE_W48_FBP_NATIVE_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// The payload's trace sink: the typed-payload arms, the fixed result arm and
/// the float payload branch.
#[cfg(unix)]
fn instrumented(module: &CompiledModule) -> String {
    qcore_compiler::diagnostic_log_c_emit::with_float_payloads(&diagnostic_sink::instrument_typed(
        module.c_source.as_deref().unwrap(),
        true,
    ))
}

/// Whether a native trace is the reference trace, where the one line that can
/// expose a NaN's unspecified bits — the owner's byte read of the payload's
/// top byte after a NaN was written — may differ in the sign bit, and nowhere
/// else. Measured: the reference's `0.0 / 0.0` (the host's run-time division)
/// is x86-64's default NaN, sign set, and Clang at `-O2` folds the same
/// division to the sign-clear quiet NaN; §L14.7 states that a byte view of a
/// NaN payload has unspecified sign and payload bits.
#[cfg(unix)]
fn same_trace_but_nan_sign(native: &str, reference: &str, top_byte_of_a_nan: Option<u64>) -> bool {
    let (native, reference): (Vec<&str>, Vec<&str>) =
        (native.lines().collect(), reference.lines().collect());
    native.len() == reference.len()
        && native.iter().zip(&reference).all(|(n, r)| {
            if n == r {
                return true;
            }
            let Some(last) = top_byte_of_a_nan else {
                return false;
            };
            let prefix = format!("QCORE_BUFFER_BYTE_READ 0 1 {last} ");
            match (n.strip_prefix(&prefix), r.strip_prefix(&prefix)) {
                (Some(a), Some(b)) => match (a.parse::<u32>(), b.parse::<u32>()) {
                    (Ok(a), Ok(b)) => a ^ b == 0x80,
                    _ => false,
                },
                _ => false,
            }
        })
}

/// Every float payload row, natively under the three builds: stdout is the
/// reference trace byte for byte (every NaN `nan`) — but for a NaN's sign bit
/// in a byte view, which is unspecified — stderr empty, exit 0.
#[cfg(unix)]
#[test]
fn native_float_payloads_match_the_reference() {
    use native::*;
    let summary = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for ty in FLOATS {
            let summary = &summary;
            scope.spawn(move || {
                let values = boundary(ty);
                let (mut runs, mut lines, mut nan_sign_differences) = (0usize, 0usize, 0usize);
                for (index, value) in values.iter().enumerate() {
                    let next = values[(index + 1) % values.len()];
                    let module = admitted(&payload_program(ty, *value, next));
                    let trace = reference(&module);
                    let c = instrumented(&module);
                    let nan = qcore_core::scalar_floats::is_nan(ty, next)
                        .then(|| ty.size_bytes(None).unwrap() - 1);
                    for (label, flags) in BUILDS {
                        let run = build_and_run(&c, flags)
                            .unwrap_or_else(|e| panic!("{ty} {value} {label}: {e}"));
                        assert_eq!(run.stderr, "", "{ty} {value} {label}: sanitizer report");
                        assert!(
                            same_trace_but_nan_sign(&run.stdout, &trace, nan),
                            "{ty} {value} {label}: stdout\n{}\nreference\n{trace}",
                            run.stdout
                        );
                        if run.stdout != trace {
                            nan_sign_differences += 1;
                        }
                        assert_eq!(run.status, Some(0), "{ty} {value} {label}");
                        runs += 1;
                        lines += trace.lines().count();
                    }
                }
                summary.lock().unwrap().push(format!(
                    "{{\"type\": \"{ty}\", \"programs_per_build\": {}, \"runs\": {runs}, \"lines\": {lines}, \"builds\": [\"O0-ubsan\", \"O2-ubsan\", \"O2\"], \"sanitizer_reports\": 0, \"runs_differing_only_in_a_nan_sign_bit\": {nan_sign_differences}}}",
                    values.len()
                ));
            });
        }
    });
    let mut rows = summary.into_inner().unwrap();
    rows.sort();
    if let Ok(path) = std::env::var("QCORE_W48_FBP_PAYLOAD_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// ThreadSanitizer over the payload rows with workers, through the pthread
/// binding: no report of any kind, the reference trace on stdout — unforced,
/// and with the two lease workers forced live together and in reverse.
#[cfg(target_os = "linux")]
#[test]
fn float_payload_rows_are_clean_under_thread_sanitizer() {
    use thread_sanitizer::{Row, Schedule};
    let mut rows = Vec::new();
    for ty in FLOATS {
        let values = boundary(ty);
        // ±0, a subnormal, a NaN and an infinity as the payloads.
        for index in [0usize, 1, 2, 12, 20] {
            let next = values[(index + 1) % values.len()];
            let module = admitted(&payload_program(ty, values[index], next));
            let trace = reference(&module);
            let c = instrumented(&module);
            let nan = qcore_core::scalar_floats::is_nan(ty, next)
                .then(|| ty.size_bytes(None).unwrap() - 1);
            for schedule in Schedule::ALL {
                let tag = format!("fbp-{ty}-{index}");
                let run = Row {
                    tag: &tag,
                    source: &c,
                    schedule,
                    // The join-only gate: the two lease workers are parked at
                    // creation and released together (or in reverse) at the
                    // creator's first join.
                    cohort: 0,
                    ..Row::default()
                }
                .execute()
                .unwrap_or_else(|error| panic!("{ty} {index} tsan: {error}"));
                assert!(
                    run.clean,
                    "{ty} {index} {}: ThreadSanitizer reported:\n{}",
                    schedule.tag(),
                    run.stderr
                );
                assert_eq!(run.races, 0);
                assert!(
                    same_trace_but_nan_sign(&run.stdout, &trace, nan),
                    "{ty} {index} {}",
                    schedule.tag()
                );
                rows.push(format!(
                    "{{\"type\": \"{ty}\", \"payload\": {index}, \"build\": \"tsan-{}\", \"exit\": 0, \"stdout_lines\": {}, \"sanitizer_reports\": 0}}",
                    schedule.tag(),
                    trace.lines().count()
                ));
            }
        }
    }
    if let Ok(path) = std::env::var("QCORE_W48_FBP_TSAN_SUMMARY") {
        std::fs::write(path, format!("[\n  {}\n]\n", rows.join(",\n  "))).unwrap();
    }
}

/// Every `protected f32`/`f64` program under the three builds: the native
/// mutex observation is the typed reference observation line for line.
#[cfg(unix)]
#[test]
fn native_protected_floats_match_the_reference() {
    use native::*;
    for ty in FLOATS {
        let module = admitted(&protected_program(ty.name()));
        let checked = module.checked.as_checked_mutex_guards().unwrap();
        let reference = qcore_core::mutex_c_emit::interpret_typed_mutex_program(
            &checked.lowered.program,
            &checked.certificate,
            &checked.lowered.payloads(),
        )
        .unwrap();
        let expected = reference
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        let c = mutex_observation::instrument(module.c_source.as_deref().unwrap());
        for (label, flags) in BUILDS {
            let run = build_and_run(&c, flags).unwrap_or_else(|e| panic!("{ty} {label}: {e}"));
            assert_eq!(run.stdout, expected, "{ty} {label}");
            assert_eq!(run.stderr, "", "{ty} {label}");
            assert_eq!(run.status, Some(0), "{ty} {label}");
        }
    }
}
