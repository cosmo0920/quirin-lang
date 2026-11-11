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

//! Card CG20-NUMERIC-DOMAIN (gateway rule 11q): the C integer property per
//! target identity, driven through Clang under an LP64 target
//! (`x86_64-pc-linux-gnu`) and an LLP64 one (`x86_64-pc-windows-msvc`).
//!
//! The property is published beside custody: the CG00 BEFORE controls
//! (`clang_gap_phase4.rs`) keep their custody `Safe` for the same sources,
//! and these tests show the arithmetic obligations those verdicts never
//! covered. Every source is self-contained, so no target header is needed.

use qcore_c_gateway::c_numeric::{self, DataModel, Fault, IntKind, Status};
use qcore_c_gateway::c_numeric_obligations::{
    FunctionNumeric, NumericTarget, NumericVerdict, analyze_numeric_obligations, target_model,
    typed_ast,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use serde_json::Value;

const LP64: NumericTarget<'static> = NumericTarget::Declared("x86_64-pc-linux-gnu");
const LLP64: NumericTarget<'static> = NumericTarget::Declared("x86_64-pc-windows-msvc");

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG20 test: {reason}");
            false
        }
    }
}

fn run(source: &str, target: NumericTarget<'_>) -> FunctionNumeric {
    let unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    let report = analyze_numeric_obligations(&unit, target)
        .unwrap_or_else(|error| panic!("{source}: {error}"));
    report
        .function("subject")
        .unwrap_or_else(|| panic!("no `subject` in {source}"))
        .clone()
}

fn faults(function: &FunctionNumeric, status: Status) -> Vec<Fault> {
    function
        .with_status(status)
        .into_iter()
        .map(|(_, fault)| fault)
        .collect()
}

#[test]
fn the_target_model_is_clangs_not_a_table() {
    if !clang_available() {
        return;
    }
    let unit = C11TranslationUnit::default();
    let lp64 = target_model(&unit, LP64).unwrap();
    let llp64 = target_model(&unit, LLP64).unwrap();
    assert_eq!(lp64.data_model(), DataModel::Lp64);
    assert_eq!(llp64.data_model(), DataModel::Llp64);
    assert_eq!(lp64.ctype(IntKind::Long).width, 64);
    assert_eq!(llp64.ctype(IntKind::Long).width, 32);
    assert_eq!(llp64.ctype(IntKind::LongLong).width, 64);
    assert!(llp64.triple.contains("windows-msvc"), "{}", llp64.triple);
    // The default target is the host's Clang default; the ABI reader's
    // plain-`long` question is answered from it, never from a table.
    let default = target_model(&unit, NumericTarget::Default).unwrap();
    assert_eq!(
        c_numeric::plain_long_is_lp64(),
        default.long_width == 64 && default.pointer_width == 64
    );
}

#[test]
fn long_arithmetic_is_decided_per_target() {
    if !clang_available() {
        return;
    }
    let source = "long subject(void) { return 2147483647L + 1L; }";
    let lp64 = run(source, LP64);
    assert_eq!(lp64.verdict, NumericVerdict::Safe, "{lp64:?}");
    assert_eq!(faults(&lp64, Status::Discharged), [Fault::SignedOverflow]);
    let llp64 = run(source, LLP64);
    assert_eq!(llp64.verdict, NumericVerdict::Unknown, "{llp64:?}");
    assert_eq!(faults(&llp64, Status::Violated), [Fault::SignedOverflow]);
    // An integer-to-double conversion of a `long` is exact under LLP64 (32
    // bits fit 53) and an open rounding obligation under LP64.
    let source = "double subject(long x) { return x; }";
    assert_eq!(
        faults(&run(source, LP64), Status::Open),
        [Fault::InexactIntegerToFloating]
    );
    assert_eq!(run(source, LLP64).verdict, NumericVerdict::Safe);
}

/// Clang's recorded promotions and usual conversions agree with the Lean
/// vectors (`QCore.CNumeric.usualVectors`) under both targets. The consumer
/// reads Clang's; this binds them to the stated C rule.
#[test]
fn clang_records_the_lean_usual_conversions() {
    if !clang_available() {
        return;
    }
    let lean = include_str!("../../../proof/QCore/CNumeric.lean").replace("\r\n", "\n");
    let body = lean
        .split("def usualVectors : List (String × Kind × Kind × Kind) := [")
        .nth(1)
        .unwrap();
    let body = &body[..body.find("\n]").unwrap()];
    let kind = |name: &str| match name.trim().trim_start_matches('.') {
        "char" => "char",
        "ushort" => "unsigned short",
        "int" => "int",
        "uint" => "unsigned int",
        "long" => "long",
        "ulong" => "unsigned long",
        "llong" => "long long",
        "ullong" => "unsigned long long",
        other => panic!("unknown Lean kind {other}"),
    };
    let mut checked = 0;
    for row in body.split("\n  (").skip(1) {
        let fields = row
            .trim_end_matches([',', ')'])
            .split(", ")
            .collect::<Vec<_>>();
        let target = if fields[0] == "\"llp64\"" {
            LLP64
        } else {
            LP64
        };
        let (a, b, expected) = (kind(fields[1]), kind(fields[2]), kind(fields[3]));
        let source = format!("void subject({a} a, {b} b) {{ (void)(a + b); }}");
        let unit = C11TranslationUnit {
            source: &source,
            ..Default::default()
        };
        let ast = typed_ast(&unit, target).unwrap();
        let recorded = find(&ast, &|node| {
            node["kind"] == "BinaryOperator" && node["opcode"] == "+"
        })
        .map(|node| node["type"]["qualType"].as_str().unwrap().to_owned());
        assert_eq!(recorded.as_deref(), Some(expected), "{row}");
        checked += 1;
    }
    assert_eq!(checked, 13);
}

fn find<'a>(node: &'a Value, predicate: &dyn Fn(&Value) -> bool) -> Option<&'a Value> {
    if predicate(node) {
        return Some(node);
    }
    node["inner"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|child| find(child, predicate))
}

#[test]
fn division_shift_and_mixed_comparison_obligations() {
    if !clang_available() {
        return;
    }
    // `x / 0`: undefined whatever x is.
    let run_lp = |source: &str| run(source, LP64);
    let f = run_lp("int subject(int x) { return x / 0; }");
    assert_eq!(f.verdict, NumericVerdict::Unknown);
    assert!(
        faults(&f, Status::Violated).contains(&Fault::DivisionByZero),
        "{f:?}"
    );
    // `INT_MIN / -1`, spelled as Clang folds it.
    let f = run_lp("int subject(void) { return (-2147483647 - 1) / -1; }");
    assert!(
        faults(&f, Status::Violated).contains(&Fault::QuotientOverflow),
        "{f:?}"
    );
    // `1 << 32` at `int`.
    let f = run_lp("int subject(void) { return 1 << 32; }");
    assert_eq!(faults(&f, Status::Violated), [Fault::ShiftCount], "{f:?}");
    // A divisor that excludes 0 and -1 discharges both cases.
    let f = run_lp("enum { K = 7 }; int subject(int a) { return a / K + a % K; }");
    assert_eq!(f.verdict, NumericVerdict::Safe, "{f:?}");
    // `u < -1`: defined, but -1 becomes UINT_MAX; an event, no obligation.
    let f = run_lp("int subject(unsigned u) { return u < -1; }");
    assert_eq!(f.verdict, NumericVerdict::Safe, "{f:?}");
    assert!(
        f.events
            .iter()
            .any(|event| event.kind == "mixed-signedness-comparison"),
        "{f:?}"
    );
}

#[test]
fn overflow_wrap_and_conversion_cases() {
    if !clang_available() {
        return;
    }
    let run_lp = |source: &str| run(source, LP64);
    // A signed parameter + 1 may overflow; a promoted byte + 1 cannot.
    assert_eq!(
        faults(
            &run_lp("int subject(int a) { return a + 1; }"),
            Status::Open
        ),
        [Fault::SignedOverflow]
    );
    assert_eq!(
        run_lp("int subject(unsigned char c) { return c + 1; }").verdict,
        NumericVerdict::Safe
    );
    // Unsigned arithmetic wraps, as C defines: no obligation, an event.
    let f = run_lp("unsigned subject(unsigned a) { return a * 3u + 1u; }");
    assert_eq!(f.verdict, NumericVerdict::Safe, "{f:?}");
    assert!(f.events.iter().any(|event| event.kind == "unsigned-wrap"));
    // Narrowing to a signed type is implementation-defined: open.
    assert_eq!(
        faults(
            &run_lp("signed char subject(int a) { return a; }"),
            Status::Open
        ),
        [Fault::SignedConversion]
    );
    // Narrowing to an unsigned type is defined; widening is exact.
    assert_eq!(
        run_lp("unsigned char subject(int a) { return a; }").verdict,
        NumericVerdict::Safe
    );
    assert_eq!(
        run_lp("long subject(int a) { return a; }").verdict,
        NumericVerdict::Safe
    );
    // Increments: signed open, unsigned defined.
    assert_eq!(
        faults(&run_lp("void subject(int i) { i++; }"), Status::Open),
        [Fault::SignedOverflow]
    );
    assert_eq!(
        run_lp("void subject(unsigned i) { i++; }").verdict,
        NumericVerdict::Safe
    );
    // A compound assignment into `char` converts back: open.
    assert!(
        faults(
            &run_lp("void subject(signed char c) { c += 1; }"),
            Status::Open
        )
        .contains(&Fault::SignedConversion)
    );
    // Shifts: an unsigned right shift by a constant is discharged; a signed
    // left shift of an unknown value is not.
    assert_eq!(
        run_lp("unsigned subject(unsigned a) { return a >> 3; }").verdict,
        NumericVerdict::Safe
    );
    let f = run_lp("int subject(int a) { return a << 3; }");
    assert_eq!(
        faults(&f, Status::Open),
        [Fault::NegativeLeftShift, Fault::SignedOverflow]
    );
    // A signed bit-field store is implementation-defined: open.
    let f = run_lp("struct S { int bf : 3; }; void subject(struct S *s, int v) { s->bf = v; }");
    assert_eq!(faults(&f, Status::Open), [Fault::SignedConversion], "{f:?}");
}

#[test]
fn floating_and_pointer_scope() {
    if !clang_available() {
        return;
    }
    let run_lp = |source: &str| run(source, LP64);
    // N08 transfer: floating-to-integer and floating narrowing are open
    // obligations; floating arithmetic itself is refused by name (N09/N10).
    assert_eq!(
        faults(
            &run_lp("int subject(double d) { return (int)d; }"),
            Status::Open
        ),
        [Fault::FloatingToInteger]
    );
    assert_eq!(
        faults(
            &run_lp("float subject(double d) { return (float)d; }"),
            Status::Open
        ),
        [Fault::FloatingNarrowing]
    );
    let f = run_lp("double subject(double a, double b) { return a + b; }");
    assert_eq!(f.verdict, NumericVerdict::Unknown);
    assert!(
        f.refused.iter().any(|reason| reason.contains("N09/N10")),
        "{f:?}"
    );
    // Pointer arithmetic is the bounds property's (CG16): outside, counted.
    let f = run_lp("char *subject(char *p) { return p + 1; }");
    assert_eq!((f.verdict, f.outside), (NumericVerdict::Safe, 1), "{f:?}");
}

/// The CG00 BEFORE sources (`n04_n05_n06_n07_g_proves_no_arithmetic_property`)
/// keep their custody `Safe`; the arithmetic property decides each of them
/// separately, and none is `Safe` except the mixed comparison.
#[test]
fn the_before_sources_under_the_arithmetic_property() {
    if !clang_available() {
        return;
    }
    for (source, verdict) in [
        (
            "int subject(int a) { return a + 2147483647; }",
            NumericVerdict::Unknown,
        ),
        (
            "int subject(int a, int b) { return a / b; }",
            NumericVerdict::Unknown,
        ),
        (
            "int subject(int a, int b) { return a % b; }",
            NumericVerdict::Unknown,
        ),
        (
            "int subject(int a) { return a << 40; }",
            NumericVerdict::Unknown,
        ),
        (
            "int subject(int a, unsigned b) { return a < b; }",
            NumericVerdict::Safe,
        ),
        (
            "int subject(double d) { return (int)d; }",
            NumericVerdict::Unknown,
        ),
    ] {
        for target in [LP64, LLP64] {
            assert_eq!(run(source, target).verdict, verdict, "{source} {target:?}");
        }
    }
}
