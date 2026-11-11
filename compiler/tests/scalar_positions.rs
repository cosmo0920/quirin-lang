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

//! Card `W48-QR-SCALAR-TYPES-R3`: the fixed-width scalar table at the positions
//! card 0 left at `i32` — here, item 1, the record fields of `source.buffer.v1`
//! and that route's now-typed value environment.
//!
//! * Every §G02t type is a field type: constructed from boundary literals,
//!   read back at its own type, rendered `dec` and `hex` through `std.fmt`
//!   against an independent reference, and run natively (`-O0`/`-O2` under
//!   UBSan and plain `-O2`) with each stream equal to the reference.
//! * The conversion statement `let NAME = OPERAND as TYPE;` over every pair of
//!   integer types and every boundary value is admitted exactly when the value
//!   fits the target, refused at `Q053` otherwise, and the admitted ones run
//!   natively with the reference value.
//! * The Lean vectors of `QCore.ScalarPositions.fieldAgreement` are replayed
//!   against the checker.
//! * Negative controls: a literal outside its field's type at both ends, a
//!   number at a `bool` field and `true` at an integer field, a name of another
//!   type at a field, a typed name at every `i32` position (index, stored byte,
//!   lend bound, loop bound), `hex` of a `bool`, a conversion out of range and
//!   to or from `bool`, and `isize`/`usize` with no recorded target.
//!
//! No test here writes output of its own: rendered values reach a stream only
//! through the program's own `print`/`log` lines, compared with the reference.
use qcore_compiler::scalar_types::{self as st, PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
use qcore_core::std_buffer;

const HEADER: &str = "#import std.fmt;\n#import std.io;\n#import std.logger;\n";

const ALL: [ScalarType; 11] = st::ALL;

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
        &[SourceFile::new("scalar_positions.qr", source)],
        &CompileOptions::default(),
        target,
    )
}

fn buffer_admitted(source: &str) -> CompiledModule {
    let m = compile_for(source, Some(&host_target()))
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{source}"));
    assert_eq!(
        m.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    m
}

fn refused_at(source: &str, target: Option<&RecordedTarget>, code: &str, needle: &str) {
    let e = compile_for(source, target).expect_err(source);
    assert!(
        e.iter()
            .any(|d| d.code == code && d.message.contains(needle)),
        "expected {code} `{needle}` for\n{source}\n{e:?}"
    );
}

fn lines(m: &CompiledModule) -> Vec<String> {
    m.extern_observation.clone().unwrap_or_default()
}

fn certificate(m: &CompiledModule) -> &std_buffer::BufferCertificate {
    match &m.checked {
        qcore_compiler::CheckedSourceModule::StandardBuffer(checked) => &checked.certificate,
        other => panic!("expected a byte-buffer module, got {other:?}"),
    }
}

fn boundary(ty: ScalarType, target: Option<&RecordedTarget>) -> Vec<i128> {
    let (lo, hi) = ty.range(target).unwrap();
    let mut values = vec![lo, lo + 1, -1, 0, 1, hi - 1, hi];
    values.retain(|v| lo <= *v && *v <= hi);
    values.sort();
    values.dedup();
    values
}

/// The §G02t renderings, spelled independently of `scalar_types::render`:
/// `dec` the number, `hex` `0x` and the two's-complement pattern at a quarter
/// of the width in upper-case digits, `bool` `true`/`false`.
fn reference_render(hex: bool, ty: ScalarType, value: i128, bits: u32) -> String {
    if ty == ScalarType::Bool {
        return if value == 1 { "true" } else { "false" }.to_owned();
    }
    if !hex {
        return value.to_string();
    }
    let pattern = if value < 0 {
        (value + (1i128 << bits)) as u128
    } else {
        value as u128
    };
    format!("0x{pattern:0width$X}", width = (bits / 4) as usize)
}

fn width(ty: ScalarType, target: &RecordedTarget) -> u32 {
    ty.bits(Some(target)).unwrap_or(1)
}

/// A literal as a field value: `true`/`false` for `bool`, the number otherwise.
fn field_literal(ty: ScalarType, value: i128) -> String {
    if ty == ScalarType::Bool {
        (value == 1).to_string()
    } else {
        value.to_string()
    }
}

// --- Item 1: every type is a field type --------------------------------------

/// One record type per §G02t type, one field per boundary value; each field is
/// read back into a binding of its own type and rendered `dec` and (for an
/// integer) `hex`, `print` and `log`. The certificate publishes every typed
/// field and read; the lines equal the independent reference.
fn every_type_program(target: &RecordedTarget) -> (String, Vec<String>) {
    let mut source = String::from(HEADER);
    let mut expected = Vec::new();
    for ty in ALL {
        let values = boundary(ty, Some(target));
        let fields: Vec<String> = (0..values.len()).map(|i| format!("f{i}: {ty}")).collect();
        source.push_str(&format!("type R{ty} {{ {} }}\n", fields.join(", ")));
    }
    source.push_str("fn main() -> unit effects { io } {\n");
    for ty in ALL {
        let values = boundary(ty, Some(target));
        let supplied: Vec<String> = values
            .iter()
            .enumerate()
            .map(|(i, v)| format!("f{i}: {}", field_literal(ty, *v)))
            .collect();
        source.push_str(&format!(
            "    let r{ty} = R{ty} {{ {} }};\n",
            supplied.join(", ")
        ));
        for (i, v) in values.iter().enumerate() {
            source.push_str(&format!("    let v{ty}{i} = r{ty}.f{i};\n"));
            source.push_str(&format!("    print dec v{ty}{i};\n"));
            expected.push(format!(
                "out: {}",
                reference_render(false, ty, *v, width(ty, target))
            ));
            if ty != ScalarType::Bool {
                source.push_str(&format!("    log info hex v{ty}{i};\n"));
                expected.push(format!(
                    "err: [INFO] {}",
                    reference_render(true, ty, *v, width(ty, target))
                ));
            }
        }
    }
    source.push_str("    flush;\n    return;\n}\n");
    (source, expected)
}

#[test]
fn every_scalar_type_is_a_field_type_read_back_at_its_own_type() {
    let target = host_target();
    let (source, expected) = every_type_program(&target);
    let m = buffer_admitted(&source);
    assert_eq!(lines(&m), expected);
    let c = certificate(&m);
    // `isize`/`usize` fields make the result depend on the recorded target,
    // and the certificate says so.
    assert_eq!(c.target_assumption.as_ref(), Some(&target));
    assert_eq!(m.target_assumption(), Some(&target));
    // Every typed read is published with its type; `i32` reads keep their
    // own list, as before the card.
    let typed_reads: usize = ALL
        .iter()
        .filter(|ty| **ty != ScalarType::I32)
        .map(|ty| boundary(*ty, Some(&target)).len())
        .sum();
    assert_eq!(c.typed_projections.len(), typed_reads);
    assert_eq!(
        c.projections.len(),
        boundary(ScalarType::I32, Some(&target)).len()
    );
    for read in &c.typed_projections {
        assert!(read.ty.contains(read.value, Some(&target)), "{read:?}");
    }
    // Each struct member is the type's own C11 spelling.
    let emitted = m.c_source.as_deref().unwrap();
    for ty in ALL {
        assert!(
            emitted.contains(&format!("    {} f_f0;", st::c_type(ty))),
            "{ty}: {emitted}"
        );
    }
    assert!(emitted.contains("_Static_assert(SIZE_MAX == UINT"));
}

#[cfg(unix)]
#[test]
fn every_field_type_renders_natively_as_the_reference_under_three_builds() {
    let target = host_target();
    let (source, expected) = every_type_program(&target);
    let m = buffer_admitted(&source);
    let c = m.c_source.clone().unwrap();
    let project = |stream: &str| {
        expected
            .iter()
            .filter_map(|line| line.strip_prefix(&format!("{stream}: ")))
            .map(|line| format!("{line}\n"))
            .collect::<String>()
    };
    let mut rows = Vec::new();
    for (label, flags) in native::BUILDS {
        let built = native::build(&c, flags).unwrap_or_else(|e| panic!("{label}: {e}"));
        let out = std::process::Command::new(&built.binary).output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{label}");
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
        rows.push(format!(
            "{{\"position\": \"record-fields\", \"build\": \"{label}\", \"lines\": {}, \"exit\": 0, \"sanitizer_reports\": 0}}",
            expected.len()
        ));
    }
    native::publish("fields", &rows);
}

/// A field's literal is range-checked against the field's type, at both ends,
/// the minimum included; a number at a `bool` field and `true` at an integer
/// field are not values of those types.
#[test]
fn a_field_literal_outside_its_type_is_refused_at_both_ends() {
    let target = host_target();
    for ty in INTEGERS {
        let (lo, hi) = ty.range(Some(&target)).unwrap();
        for (value, fits) in [(lo, true), (lo - 1, false), (hi, true), (hi + 1, false)] {
            let source = format!(
                "{HEADER}type R {{ f: {ty} }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ f: {value} }};\n    let v = r.f;\n    print dec v;\n    return;\n}}\n"
            );
            if fits {
                let m = buffer_admitted(&source);
                assert_eq!(lines(&m), [format!("out: {value}")], "{ty} {value}");
            } else if ty == ScalarType::I32 {
                // An `i32` field reads its literal with the route's own `i32`
                // reader, exactly as before the card: out of range there is the
                // shape refusal it always was.
                refused_at(&source, Some(&target), "Q051", "R.f");
            } else {
                refused_at(&source, Some(&target), "Q053", "is not a value of");
            }
        }
    }
    let number_at_bool = format!(
        "{HEADER}type R {{ f: bool }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ f: 1 }};\n    return;\n}}\n"
    );
    refused_at(
        &number_at_bool,
        Some(&target),
        "Q053",
        "is not a value of `bool`",
    );
    let bool_at_integer = format!(
        "{HEADER}type R {{ f: u8 }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ f: true }};\n    return;\n}}\n"
    );
    refused_at(
        &bool_at_integer,
        Some(&target),
        "Q053",
        "is not a value of `u8`",
    );
}

/// There is no implicit conversion at a field: a name of another type is
/// refused in both directions, `i32` included.
#[test]
fn a_name_of_another_type_at_a_field_is_refused() {
    let target = host_target();
    let cases = [
        // A `u8` read into an `i32` field.
        "type A { f: u8 }\ntype B { g: i32 }\nfn main() -> unit effects { io } {\n    let a = A { f: 7 };\n    let x = a.f;\n    let b = B { g: x };\n    return;\n}\n",
        // An `i32` name into a `u16` field.
        "type B { g: u16 }\nfn main() -> unit effects { io } {\n    let x = 7;\n    let b = B { g: x };\n    return;\n}\n",
        // A `bool` into an `i8` field.
        "type A { f: bool }\ntype B { g: i8 }\nfn main() -> unit effects { io } {\n    let a = A { f: true };\n    let x = a.f;\n    let b = B { g: x };\n    return;\n}\n",
    ];
    for case in cases {
        refused_at(
            &format!("{HEADER}{case}"),
            Some(&target),
            "Q052",
            "there is no implicit conversion",
        );
    }
    // The same number converted explicitly is admitted.
    let converted = format!(
        "{HEADER}type A {{ f: u8 }}\ntype B {{ g: i32 }}\nfn main() -> unit effects {{ io }} {{\n    let a = A {{ f: 7 }};\n    let x = a.f;\n    let y = x as i32;\n    let b = B {{ g: y }};\n    let z = b.g;\n    print dec z;\n    return;\n}}\n"
    );
    assert_eq!(lines(&buffer_admitted(&converted)), ["out: 7"]);
}

/// Every position of the route that takes an `i32` refuses a name of another
/// type by name, and takes it once it is converted.
#[test]
fn a_typed_name_at_every_i32_position_is_refused() {
    let target = host_target();
    let prefix = "#import std.buffer;\n#import std.fmt;\n#import std.io;\ntype A { f: u8 }\nfn main() -> unit effects { io, alloc, mut } {\n    let b = bytes(4);\n    let a = A { f: 2 };\n    let x = a.f;\n";
    let suffix = "    drop b;\n    return;\n}\n";
    for statement in ["    b[x] = 1;\n", "    b[0] = x;\n", "    let y = b[x];\n"] {
        refused_at(
            &format!("{prefix}{statement}{suffix}"),
            Some(&target),
            "Q052",
            "`x` is a `u8` and this position takes an `i32`",
        );
    }
    // A lend bound and a loop bound are the lowering-visible tier, which a
    // field read never enters: refused by name, as before the card.
    let lend = format!("{prefix}    let l = lend(b, 0, x);\n    l[0] = 1;\n{suffix}");
    refused_at(&lend, Some(&target), "Q053", "`x` is not a bound");
    let bound = format!("{prefix}    for i in 0..x {{ b[0] = 1; }}\n{suffix}");
    assert!(compile_for(&bound, Some(&target)).is_err());
    // Converted to `i32`, the index position takes it.
    let converted = format!(
        "{prefix}    let i = x as i32;\n    b[i] = 9;\n    let y = b[2];\n    print dec y;\n{suffix}"
    );
    assert_eq!(lines(&buffer_admitted(&converted)), ["out: 9"]);
}

/// `hex` of a `bool` is refused; `dec` renders `true`/`false`. A copy keeps its
/// type, so its `hex` is still the type's own width.
#[test]
fn bool_has_no_hex_and_a_copy_keeps_its_type() {
    let target = host_target();
    let hex_bool = format!(
        "{HEADER}type A {{ f: bool }}\nfn main() -> unit effects {{ io }} {{\n    let a = A {{ f: false }};\n    let x = a.f;\n    print hex x;\n    return;\n}}\n"
    );
    refused_at(&hex_bool, Some(&target), "Q052", "no `hex`");
    let copy = format!(
        "{HEADER}type A {{ f: u16, t: bool }}\nfn main() -> unit effects {{ io }} {{\n    let a = A {{ f: 65535, t: true }};\n    let x = a.f;\n    let y = x;\n    let t = a.t;\n    let u = t;\n    print hex y;\n    print dec u;\n    return;\n}}\n"
    );
    assert_eq!(lines(&buffer_admitted(&copy)), ["out: 0xFFFF", "out: true"]);
}

/// `isize`/`usize` take their width from the recorded target: a field or a
/// conversion target of either type refuses without one, and a program naming
/// neither carries no target assumption even when one is supplied.
#[test]
fn target_width_types_need_the_recorded_target() {
    let field = format!(
        "{HEADER}type A {{ f: usize }}\nfn main() -> unit effects {{ io }} {{\n    let a = A {{ f: 7 }};\n    let x = a.f;\n    print dec x;\n    return;\n}}\n"
    );
    refused_at(&field, None, "Q051", "recorded target");
    let m = buffer_admitted(&field);
    assert_eq!(lines(&m), ["out: 7"]);
    assert!(m.target_assumption().is_some());
    let conversion = format!(
        "{HEADER}type A {{ f: i32 }}\nfn main() -> unit effects {{ io }} {{\n    let a = A {{ f: 7 }};\n    let x = a.f;\n    let y = x as isize;\n    print dec y;\n    return;\n}}\n"
    );
    refused_at(&conversion, None, "Q051", "recorded target");
    assert_eq!(lines(&buffer_admitted(&conversion)), ["out: 7"]);
    let neither = format!(
        "{HEADER}type A {{ f: u8 }}\nfn main() -> unit effects {{ io }} {{\n    let a = A {{ f: 7 }};\n    let x = a.f;\n    print dec x;\n    return;\n}}\n"
    );
    assert!(buffer_admitted(&neither).target_assumption().is_none());
    assert!(compile_for(&neither, None).is_ok());
}

// --- Item 1: the conversion statement at the boundary ------------------------

fn conversion_program(from: ScalarType, value: i128, to: ScalarType) -> String {
    format!(
        "{HEADER}type R {{ f: {from} }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ f: {value} }};\n    let x = r.f;\n    let y = x as {to};\n    print dec y;\n    return;\n}}\n"
    )
}

/// Every integer pair, every boundary value of the source: admitted exactly
/// when the value is a value of the target (the reference), with that value
/// rendered, and refused at `Q053` otherwise. A conversion to or from `bool`
/// is refused at `Q052`.
#[test]
fn a_conversion_is_admitted_exactly_when_the_value_fits() {
    let target = host_target();
    let (mut admitted, mut refused) = (0, 0);
    for from in INTEGERS {
        for value in boundary(from, Some(&target)) {
            for to in INTEGERS {
                let source = conversion_program(from, value, to);
                let (lo, hi) = to.range(Some(&target)).unwrap();
                if lo <= value && value <= hi {
                    assert_eq!(
                        lines(&buffer_admitted(&source)),
                        [format!("out: {value}")],
                        "{from} {value} as {to}"
                    );
                    admitted += 1;
                } else {
                    refused_at(&source, Some(&target), "Q053", "is not a value of");
                    refused += 1;
                }
            }
        }
    }
    assert!(admitted > 0 && refused > 0);
    for (from, to) in [
        (ScalarType::Bool, ScalarType::U8),
        (ScalarType::I32, ScalarType::Bool),
    ] {
        let value = if from == ScalarType::Bool {
            "true"
        } else {
            "1"
        };
        let source = format!(
            "{HEADER}type R {{ f: {from} }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ f: {value} }};\n    let x = r.f;\n    let y = x as {to};\n    return;\n}}\n"
        );
        refused_at(&source, Some(&target), "Q052", "is not a conversion");
    }
    // A literal source is an `i32`.
    let literal = format!(
        "#import std.buffer;\n{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let y = 300 as u8;\n    drop b;\n    return;\n}}\n"
    );
    refused_at(
        &literal,
        Some(&target),
        "Q053",
        "`300` (a `i32`) is not a value of `u8`",
    );
}

/// The admitted boundary conversions, one program per source type, run
/// natively under the three builds: every line is the reference value, and
/// the checked helpers' witnesses never fire.
#[cfg(unix)]
#[test]
fn admitted_conversions_run_natively_as_the_reference() {
    let target = host_target();
    let mut rows = Vec::new();
    for from in INTEGERS {
        let values = boundary(from, Some(&target));
        let fields: Vec<String> = (0..values.len()).map(|i| format!("f{i}: {from}")).collect();
        let supplied: Vec<String> = values
            .iter()
            .enumerate()
            .map(|(i, v)| format!("f{i}: {v}"))
            .collect();
        let mut source = format!(
            "{HEADER}type R {{ {} }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ {} }};\n",
            fields.join(", "),
            supplied.join(", ")
        );
        let mut expected = String::new();
        let mut n = 0;
        for (i, value) in values.iter().enumerate() {
            source.push_str(&format!("    let x{i} = r.f{i};\n"));
            for to in INTEGERS {
                if to.contains(*value, Some(&target)) {
                    source.push_str(&format!(
                        "    let y{n} = x{i} as {to};\n    print dec y{n};\n    print hex y{n};\n"
                    ));
                    expected.push_str(&format!(
                        "{}\n{}\n",
                        reference_render(false, to, *value, width(to, &target)),
                        reference_render(true, to, *value, width(to, &target))
                    ));
                    n += 1;
                }
            }
        }
        source.push_str("    return;\n}\n");
        let m = buffer_admitted(&source);
        let c = m.c_source.clone().unwrap();
        for (label, flags) in native::BUILDS {
            let built = native::build(&c, flags).unwrap_or_else(|e| panic!("{from} {label}: {e}"));
            let out = std::process::Command::new(&built.binary).output().unwrap();
            assert_eq!(out.status.code(), Some(0), "{from} {label}");
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                expected,
                "{from} {label}"
            );
            assert!(out.stderr.is_empty(), "{from} {label}");
            rows.push(format!(
                "{{\"position\": \"conversion\", \"from\": \"{from}\", \"build\": \"{label}\", \"conversions\": {n}, \"exit\": 0, \"sanitizer_reports\": 0}}"
            ));
        }
    }
    native::publish("conversions", &rows);
}

/// `QCore.ScalarPositions.fieldAgreement`: the Lean statement of what a
/// construction admits at a field, replayed against the checker. Each row is
/// `(type, literal, admitted)` at the 64-bit target.
#[test]
fn lean_field_vectors_match_the_checker() {
    let text = include_str!("../../proof/QCore/ScalarPositions.lean");
    let table = text
        .split("def fieldAgreement :")
        .nth(1)
        .unwrap()
        .split("theorem fieldAgreement_holds")
        .next()
        .unwrap();
    let target = RecordedTarget {
        pointer_width: PointerWidth::W64,
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "the host C compiler".into(),
    };
    let mut count = 0;
    for row in table.split("(.").skip(1) {
        let fields = row.split(',').take(3).map(str::trim).collect::<Vec<_>>();
        let ty = ScalarType::parse(fields[0]).expect("a Lean type");
        let value: i128 = fields[1]
            .trim_matches(|c| c == '(' || c == ')')
            .parse()
            .unwrap();
        let admitted = fields[2].starts_with("true");
        let source = format!(
            "{HEADER}type R {{ f: {ty} }}\nfn main() -> unit effects {{ io }} {{\n    let r = R {{ f: {value} }};\n    return;\n}}\n"
        );
        let checked = compile_for(&source, Some(&target));
        // The Lean row speaks of the literal at the type; an `i32` field
        // refuses an out-of-range literal at the reader rather than the
        // checker, and both are refusals.
        assert_eq!(checked.is_ok(), admitted, "{row}");
        count += 1;
    }
    assert!(count >= 20, "{count}");
}

/// The published example's lines are the ones its page shows, per stream, and
/// its native run under the three builds writes exactly those.
#[test]
fn the_record_fields_example_publishes_its_transcript() {
    let m = buffer_admitted(include_str!("../../examples/scalar/record_fields.qr"));
    let page = include_str!("../../examples/scalar/README.md").replace("\r\n", "\n");
    let mut published = PublishedStreams::default();
    for (stream, marker) in [
        ("out", "record_fields stdout"),
        ("err", "record_fields stderr"),
    ] {
        let block = page
            .split(&format!("<!-- transcript: {marker} -->"))
            .nth(1)
            .and_then(|rest| rest.split("```text\n").nth(1))
            .and_then(|rest| rest.split("```").next())
            .unwrap_or_else(|| panic!("the page has no `{marker}` transcript"));
        let observed = lines(&m)
            .into_iter()
            .filter(|line| line.starts_with(&format!("{stream}: ")))
            .collect::<Vec<_>>();
        let shown = block
            .lines()
            .map(|line| format!("{stream}: {line}"))
            .collect::<Vec<_>>();
        assert_eq!(shown, observed, "{marker}");
        published.push(stream, block);
    }
    #[cfg(unix)]
    {
        let c = m.c_source.clone().unwrap();
        for (label, flags) in native::BUILDS {
            let built = native::build(&c, flags).unwrap_or_else(|e| panic!("{label}: {e}"));
            let out = std::process::Command::new(&built.binary).output().unwrap();
            assert_eq!(out.status.code(), Some(0), "{label}");
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                published.out,
                "{label}"
            );
            assert_eq!(
                String::from_utf8_lossy(&out.stderr),
                published.err,
                "{label}"
            );
        }
    }
}

/// The two stream projections a page publishes.
#[derive(Default)]
#[cfg_attr(not(unix), allow(dead_code))]
struct PublishedStreams {
    out: String,
    err: String,
}

impl PublishedStreams {
    fn push(&mut self, stream: &str, block: &str) {
        match stream {
            "out" => self.out.push_str(block),
            _ => self.err.push_str(block),
        }
    }
}

// --- Native execution ---------------------------------------------------

#[cfg(unix)]
mod native {
    use std::process::Command;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    static SUMMARY: Mutex<Vec<String>> = Mutex::new(Vec::new());

    /// The three builds the card measures: `-O0` and `-O2` under UBSan
    /// (`undefined,integer`, no recovery), and plain `-O2`.
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

    pub struct Built {
        dir: std::path::PathBuf,
        pub binary: std::path::PathBuf,
    }
    impl Drop for Built {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    pub fn build(source: &str, flags: &[&str]) -> Result<Built, String> {
        let dir = std::env::temp_dir().join(format!(
            "qcore-w48-r3-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.c"), source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or("clang".into());
        let out = Command::new(compiler)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
            .args(flags)
            .arg(dir.join("main.c"))
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

    /// Append one test's rows to the card's native summary, when
    /// `QCORE_W48_R3_NATIVE_SUMMARY` names a directory: one file per test.
    pub fn publish(name: &str, rows: &[String]) {
        SUMMARY.lock().unwrap().extend(rows.iter().cloned());
        if let Ok(dir) = std::env::var("QCORE_W48_R3_NATIVE_SUMMARY") {
            let _ = std::fs::create_dir_all(&dir);
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{name}.json")),
                format!("[\n  {}\n]\n", rows.join(",\n  ")),
            )
            .unwrap();
        }
    }
}
