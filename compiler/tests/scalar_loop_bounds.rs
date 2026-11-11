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

//! Card `W48-QR-SCALAR-TYPES-R5`, item 1 (`W48-QR-TYPES-LOOP-BOUNDS`): the
//! counted loop's bound and index at every type of the fixed-width table.
//!
//! * A bound of every integer type — a conversion of a value of the lowering's
//!   constant tier — is admitted with the index at that type, the index's
//!   values `0 … n-1` rendered `dec` and `hex` at the type's own width, equal
//!   to an independent reference, and run natively (`-O0`/`-O2` under UBSan
//!   and plain `-O2`) with each stream equal to the reference.
//! * The bound decision at each type's boundary (`MIN`, `MIN+1`, `-1`, `0`,
//!   `1`, `MAX-1`, `MAX`, the ceiling) is the Lean one
//!   (`QCore.ScalarLoopBounds.loopAgreement`), replayed through
//!   `source_loop::typed_bound`, and a source program at each reachable row is
//!   admitted or refused (`Q098`) as the row says.
//! * The checker re-derives a lowered loop's count against its index type and
//!   its named bound, refusing a count the index cannot hold and a count the
//!   bound's value disagrees with.
//! * Negative controls: a `usize` bound with no recorded target; a bound read
//!   out of a field (outside the tier, as before); a typed index at an `i32`
//!   position (`Q052`); a nested bound spelled through an outer index; a
//!   licence the typed bound violates; a typed loop on the composed
//!   string/buffer route (`Q077`).
//!
//! No test here writes output of its own: rendered values reach a stream only
//! through the program's own `print`/`log` lines, compared with the reference.
use qcore_compiler::scalar_types::{self as st, PointerWidth, RecordedTarget, ScalarType};
use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files_for_target,
};
use qcore_core::source_loop::{self, BoundRefusal};
use qcore_core::std_buffer::{self, BufferEvent, BufferOperand, BufferProgram, LoopIndex};
use qcore_core::std_io::{Rendering, StdIoEvent, StdIoOperand};
use qcore_core::typed_errors::EffectRow;

const HEADER: &str =
    "#import std.buffer;\n#import std.fmt;\n#import std.io;\n#import std.logger;\n";

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
        &[SourceFile::new("scalar_loop_bounds.qr", source)],
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

/// The §G02t renderings, spelled independently of `scalar_types::render`.
fn reference_render(hex: bool, value: i128, bits: u32) -> String {
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

/// One loop over a bound of type `ty` and number `n`, reached by a conversion
/// of an `i32` of the tier, rendering the index `dec` and `hex`. A bound above
/// the default budget carries the licence the loop needs.
fn loop_program(ty: ScalarType, n: i128) -> String {
    loop_program_with(ty, n, "        print dec i;\n        log info hex i;\n")
}

fn loop_program_with(ty: ScalarType, n: i128, body: &str) -> String {
    let licence = if n > 256 {
        format!("    [| #unroll, where #bound < {} |]\n", n + 1)
    } else {
        String::new()
    };
    format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let k = {n};\n    let n = k as {ty};\n{licence}    for i in 0..n {{\n{body}    }}\n    drop b;\n    return;\n}}\n"
    )
}

fn expected_lines(ty: ScalarType, n: i128, target: &RecordedTarget) -> Vec<String> {
    let bits = ty.bits(Some(target)).unwrap();
    (0..n)
        .flat_map(|i| {
            [
                format!("out: {}", reference_render(false, i, bits)),
                format!("err: [INFO] {}", reference_render(true, i, bits)),
            ]
        })
        .collect()
}

/// Every integer type is a bound, the index has the bound's type, and its
/// values render at that type — `u8`'s `hex` two digits, `u64`'s sixteen.
#[test]
fn every_integer_type_is_a_bound_and_the_index_has_its_type() {
    let target = host_target();
    for ty in INTEGERS {
        for n in [1, 3, 127] {
            let m = buffer_admitted(&loop_program(ty, n));
            assert_eq!(lines(&m), expected_lines(ty, n, &target), "{ty} {n}");
            let c = m.c_source.clone().unwrap();
            // A derived bound — `i32` included — is named by the loop, so the
            // native run compares against the number the program computed.
            assert!(
                c.contains(&format!("for ({} s_i = ", st::c_type(ty))),
                "{ty}: {c}"
            );
            assert!(c.contains("; s_i < s_n; s_i += "), "{ty}: {c}");
            assert_eq!(
                m.target_assumption().is_some(),
                ty.is_target_width(),
                "{ty}"
            );
        }
    }
}

/// The Lean rows, replayed through `source_loop::typed_bound` at the 64-bit
/// target, and every row an `i32` can reach through a conversion replayed
/// through a whole source program.
#[test]
fn lean_loop_bound_vectors_match_the_decision() {
    let text = include_str!("../../proof/QCore/ScalarLoopBounds.lean");
    let table = text
        .split("def loopAgreement :")
        .nth(1)
        .unwrap()
        .split("def admittedCount")
        .next()
        .unwrap();
    let target = RecordedTarget {
        pointer_width: PointerWidth::W64,
        triple: "x86_64-unknown-linux-gnu".into(),
        compiler: "the host C compiler".into(),
    };
    let (mut rows, mut programs) = (0, 0);
    for row in table.split("(.").skip(1) {
        let fields = row.split(',').take(3).map(str::trim).collect::<Vec<_>>();
        let ty = ScalarType::parse(fields[0]).expect("a Lean type");
        let value: i128 = fields[1]
            .trim_matches(|c| c == '(' || c == ')')
            .parse()
            .unwrap();
        let expected = fields[2]
            .trim_end_matches([')', ']'])
            .strip_prefix("some ")
            .map(|count| count.trim().parse::<u32>().unwrap());
        let decided = source_loop::typed_bound("n", ty, value, Some(&target)).ok();
        assert_eq!(decided, expected, "{row}");
        rows += 1;
        // Through a program: an `i32` literal converted to the type reaches
        // the row when the row's number is an `i32` and a value of the type.
        if ty.is_integer()
            && i32::try_from(value).is_ok()
            && ty.contains(value, Some(&target))
            && cfg!(target_pointer_width = "64")
        {
            let source = loop_program_with(ty, value, "        print dec i;\n");
            match expected {
                Some(count) => {
                    let m = buffer_admitted(&source);
                    assert_eq!(lines(&m).len(), count as usize, "{row}");
                }
                None => {
                    let e = compile_for(&source, Some(&target)).expect_err(&source);
                    assert!(e.iter().any(|d| d.code == "Q098"), "{row}: {e:?}");
                }
            }
            programs += 1;
        }
    }
    assert!(rows >= 30 && programs >= 12, "{rows} {programs}");
}

/// The refusals of the typed decision, one per condition, at `Q098`, and the
/// ceiling and the budget restated at a narrow type.
#[test]
fn typed_bounds_are_refused_by_the_loop_rules() {
    let target = host_target();
    // Negative (`-1` fits `i8`), zero, and above the ceiling (`u16`).
    let negative = format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let k = -1;\n    let n = k as i8;\n    for i in 0..n {{ print dec i; }}\n    drop b;\n    return;\n}}\n"
    );
    refused_at(&negative, Some(&target), "Q098", "-1 is not a bound");
    refused_at(
        &loop_program(ScalarType::U16, 0),
        Some(&target),
        "Q098",
        "a bound of 0",
    );
    refused_at(
        &loop_program(ScalarType::U16, 4097),
        Some(&target),
        "Q098",
        "4097 is above the 4096 iterations",
    );
    // The default budget and the licence, at `u16`.
    let unlicensed =
        loop_program(ScalarType::U16, 300).replace("    [| #unroll, where #bound < 301 |]\n", "");
    refused_at(&unlicensed, Some(&target), "Q098", "#bound < 301");
    let violated = loop_program(ScalarType::U16, 300).replace("< 301", "< 300");
    refused_at(&violated, Some(&target), "Q098", "this loop counts 300");
    // `usize` and `isize` with no recorded target: the bound is refused for
    // its width (`Q098`), never guessed.
    for ty in [ScalarType::Usize, ScalarType::Isize] {
        refused_at(&loop_program(ty, 3), None, "Q098", "recorded target");
    }
}

/// Outside the tier, as before the card: a bound read out of a field, and a
/// nested bound spelled through an outer typed index.
#[test]
fn bounds_outside_the_tier_stay_refused() {
    let target = host_target();
    let field = format!(
        "{HEADER}type A {{ f: u8 }}\nfn main() -> unit effects {{ io }} {{\n    let a = A {{ f: 3 }};\n    let x = a.f;\n    for i in 0..x {{ print dec i; }}\n    return;\n}}\n"
    );
    refused_at(
        &field,
        Some(&target),
        "Q098",
        "`x` is not a check-time-constant",
    );
    let nested = format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let k = 3;\n    let n = k as u8;\n    for i in 0..n {{\n        for j in 0..i {{ print dec j; }}\n    }}\n    drop b;\n    return;\n}}\n"
    );
    refused_at(
        &nested,
        Some(&target),
        "Q098",
        "`i` is not a check-time-constant",
    );
    // A conversion of a byte read is outside the tier: its operand is.
    let byte = format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let v = b[0];\n    let n = v as u8;\n    for i in 0..n {{ print dec i; }}\n    drop b;\n    return;\n}}\n"
    );
    refused_at(
        &byte,
        Some(&target),
        "Q098",
        "`n` is not a check-time-constant",
    );
}

/// A typed index is a value of its type: at an `i32` position it refuses by
/// name, converted it is taken, and it may be copied, converted and rendered.
#[test]
fn a_typed_index_is_a_value_of_its_type() {
    let target = host_target();
    let at_index = format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc, mut }} {{\n    let b = bytes(4);\n    let k = 4;\n    let n = k as u8;\n    for i in 0..n {{ b[i] = 1; }}\n    drop b;\n    return;\n}}\n"
    );
    refused_at(
        &at_index,
        Some(&target),
        "Q052",
        "`i` is a `u8` and this position takes an `i32`",
    );
    let converted = format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc, mut }} {{\n    let b = bytes(4);\n    let k = 4;\n    let n = k as u8;\n    for i in 0..n {{\n        let j = i as i32;\n        let c = i;\n        b[j] = 7;\n        print hex c;\n    }}\n    let v = b[3];\n    print dec v;\n    drop b;\n    return;\n}}\n"
    );
    assert_eq!(
        lines(&buffer_admitted(&converted)),
        ["out: 0x00", "out: 0x01", "out: 0x02", "out: 0x03", "out: 7"]
    );
    // A conversion of the index that does not fit refuses at the iteration
    // that reaches it: `i as i8` over `0..200` at `u8`.
    let narrowing = format!(
        "{HEADER}fn main() -> unit effects {{ io, alloc }} {{\n    let b = bytes(1);\n    let k = 200;\n    let n = k as u8;\n    for i in 0..n {{\n        let j = i as i8;\n        print dec j;\n    }}\n    drop b;\n    return;\n}}\n"
    );
    refused_at(
        &narrowing,
        Some(&target),
        "Q053",
        "`128` (a `u8`) is not a value of `i8`",
    );
}

/// Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-COMPOSED-STRING`) discharged
/// the refusal this test pinned for card R5: a typed loop and a conversion in
/// a loop body are admitted on the composed string/buffer route
/// (`compiler/tests/scalar_composed_string.rs` runs them natively).
#[test]
fn the_composed_string_route_admits_typed_loops() {
    let target = host_target();
    let conversion = "#import std.string;\n#import std.buffer;\n#import std.io;\nfn main() -> unit effects { io, alloc, mut } {\n    let b = bytes(4);\n    for i in 0..4 {\n        let j = i as u8;\n        b[i] = 97;\n    }\n    let all = borrow_utf8(b);\n    print all;\n    drop b;\n    return;\n}\n";
    compile_for(conversion, Some(&target)).expect("a conversion in a composed loop body");
    let typed = "#import std.string;\n#import std.buffer;\n#import std.io;\nfn main() -> unit effects { io, alloc, mut } {\n    let b = bytes(4);\n    for i in 0..1 {\n        let k = 2;\n        let n = k as u8;\n        for j in 0..n {\n            b[0] = 97;\n        }\n    }\n    let all = borrow_utf8(b);\n    print all;\n    drop b;\n    return;\n}\n";
    compile_for(typed, Some(&target)).expect("a typed loop on the composed route");
}

fn repeat_program(index: LoopIndex, count: u32) -> BufferProgram {
    BufferProgram {
        float_effects: Default::default(),
        entry: "main".into(),
        declared_effects: EffectRow::new([qcore_core::typed_errors::Effect::Io]),
        records: Vec::new(),
        events: vec![
            BufferEvent::Bind {
                binding: "k".into(),
                value: 200,
            },
            BufferEvent::Convert {
                binding: "n".into(),
                operand: BufferOperand::Scalar("k".into()),
                to: ScalarType::U8,
            },
            BufferEvent::Repeat {
                variable: "i".into(),
                count,
                index,
                body: vec![BufferEvent::Line(StdIoEvent::Print {
                    rendering: Rendering::Dec,
                    operand: StdIoOperand::Scalar("i".into()),
                })],
            },
            BufferEvent::Return,
        ],
    }
}

/// The checker re-derives the lowering's count: a count the index type cannot
/// hold, and a count the named bound's value disagrees with, are refused; the
/// agreeing loop is admitted and runs every index at its type.
#[test]
fn the_checker_rederives_a_typed_count() {
    let u8_bound = |_count: u32| LoopIndex::Typed {
        ty: ScalarType::U8,
        bound: "n".into(),
    };
    let admitted = std_buffer::check_buffer_program(&repeat_program(u8_bound(200), 200))
        .expect("the agreeing loop");
    assert_eq!(admitted.lines.len(), 200);
    assert_eq!(admitted.lines[199].text, "199");
    let refusals = |program: &BufferProgram| -> Vec<String> {
        std_buffer::check_buffer_program(program)
            .expect_err("refused")
            .iter()
            .map(|e| format!("{} {e}", e.code()))
            .collect()
    };
    let disagrees = refusals(&repeat_program(u8_bound(199), 199));
    assert!(
        disagrees
            .iter()
            .any(|e| e.starts_with("Q098") && e.contains("re-derived")),
        "{disagrees:?}"
    );
    let too_many = refusals(&repeat_program(u8_bound(300), 300));
    assert!(
        too_many
            .iter()
            .any(|e| e.starts_with("Q098") && e.contains("cannot have a `u8` index")),
        "{too_many:?}"
    );
    let bool_index = refusals(&repeat_program(
        LoopIndex::Typed {
            ty: ScalarType::Bool,
            bound: "n".into(),
        },
        1,
    ));
    assert!(
        bool_index.iter().any(|e| e.starts_with("Q098")),
        "{bool_index:?}"
    );
    assert_eq!(
        source_loop::index_admits(256, ScalarType::U8, None),
        Err(BoundRefusal::CountOutsideIndex {
            count: 256,
            ty: ScalarType::U8
        })
    );
    assert_eq!(source_loop::index_admits(255, ScalarType::U8, None), Ok(()));
    assert_eq!(
        source_loop::index_admits(1, ScalarType::Usize, None),
        Err(BoundRefusal::NeedsRecordedTarget {
            ty: ScalarType::Usize
        })
    );
}

/// Every type's loop — its maximum count below the ceiling reachable from an
/// `i32`, and three — runs natively under the three builds with every stream
/// equal to the reference: each index at its type, `u8` through `255`, `i8`
/// through `127`, and no sanitizer report from the step at the top of a narrow
/// type.
#[cfg(unix)]
#[test]
fn every_typed_loop_runs_natively_as_the_reference() {
    let target = host_target();
    let mut rows = Vec::new();
    for ty in INTEGERS {
        let (_, max) = ty.range(Some(&target)).unwrap();
        for n in [3, max.min(4000)] {
            let m = buffer_admitted(&loop_program(ty, n));
            let expected = expected_lines(ty, n, &target);
            assert_eq!(lines(&m), expected, "{ty} {n}");
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
            for (label, flags) in native::BUILDS {
                let built =
                    native::build(&c, flags).unwrap_or_else(|e| panic!("{ty} {n} {label}: {e}"));
                let out = std::process::Command::new(&built.binary).output().unwrap();
                assert_eq!(out.status.code(), Some(0), "{ty} {n} {label}");
                assert_eq!(
                    String::from_utf8_lossy(&out.stdout),
                    stdout,
                    "{ty} {n} {label}"
                );
                assert_eq!(
                    String::from_utf8_lossy(&out.stderr),
                    stderr,
                    "{ty} {n} {label}"
                );
                rows.push(format!(
                    "{{\"position\": \"loop\", \"type\": \"{ty}\", \"bound\": {n}, \"build\": \"{label}\", \"lines\": {}, \"exit\": 0, \"sanitizer_reports\": 0}}",
                    expected.len()
                ));
            }
        }
    }
    native::publish("loops", &rows);
}

// --- Native execution ---------------------------------------------------

#[cfg(unix)]
mod native {
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

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
            "qcore-w48-r5-loops-{}-{}",
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

    /// Write one test's rows to the card's native summary, when
    /// `QCORE_W48_R5_NATIVE_SUMMARY` names a directory: one file per test.
    pub fn publish(name: &str, rows: &[String]) {
        if let Ok(dir) = std::env::var("QCORE_W48_R5_NATIVE_SUMMARY") {
            let _ = std::fs::create_dir_all(&dir);
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{name}.json")),
                format!("[\n  {}\n]\n", rows.join(",\n  ")),
            )
            .unwrap();
        }
    }
}
