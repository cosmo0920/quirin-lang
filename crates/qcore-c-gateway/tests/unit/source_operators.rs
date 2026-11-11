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

// CG01-T-OPERATOR-CLOSURE AFTER: route T reads every CG01 node, admits only
// what the calling consumer classifies (rule 10r), and keeps its sequencing
// refusals. Rows are the shared vectors the BEFORE test executed at base.
use super::*;
use crate::typed_source_nodes::{
    Classification, Fault, NODE_TABLE, NodeAdmission, checked_binary, classification_allowed, facts,
};
#[path = "source_operators_vectors.rs"]
mod vectors;
use vectors::*;

/// A test consumer that classifies every node as strongly as its facts allow.
const ALL: NodeAdmission = NodeAdmission::new(
    "CG01 test consumer",
    &[
        ("binary -", Classification::ObligationBearing),
        ("binary <=", Classification::CustodyNeutral),
        ("binary >=", Classification::CustodyNeutral),
        ("binary ^", Classification::CustodyNeutral),
        ("binary <<", Classification::NumericallyEvaluated),
        ("binary >>", Classification::NumericallyEvaluated),
        ("unary +", Classification::CustodyNeutral),
        ("unary ~", Classification::CustodyNeutral),
        ("compound +=", Classification::ObligationBearing),
        ("compound -=", Classification::ObligationBearing),
        ("compound *=", Classification::ObligationBearing),
        ("compound /=", Classification::ObligationBearing),
        ("compound %=", Classification::ObligationBearing),
        ("compound <<=", Classification::ObligationBearing),
        ("compound >>=", Classification::ObligationBearing),
        ("compound &=", Classification::ObligationBearing),
        ("compound ^=", Classification::ObligationBearing),
        ("compound |=", Classification::ObligationBearing),
        ("literal floating", Classification::CustodyNeutral),
        ("literal character", Classification::CustodyNeutral),
    ],
);

fn import_with(c: &str, admission: &NodeAdmission) -> Result<SourceResult, String> {
    import_source_functions_admitting(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        "cg01-fixture",
        &["f"],
        &SourceImportProfile {
            records: &["item"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
        admission,
    )
    .map(|mut results| results.remove(0))
}

fn summary(c: &str) -> SourceSummary {
    match import_with(c, &ALL).unwrap() {
        SourceResult::Summary(summary) => summary,
        SourceResult::Unknown { reason, .. } => panic!("admitted row refused: {reason}: {c}"),
    }
}

fn refusal(c: &str, admission: &NodeAdmission) -> String {
    match import_with(c, admission).unwrap() {
        SourceResult::Summary(_) => panic!("expected a refusal: {c}"),
        SourceResult::Unknown { reason, .. } => reason,
    }
}

fn named(spelling: &str) -> String {
    format!("typed-source node `{spelling}` is not classified by the legacy typed-source consumer")
}

/// Every operator and literal spelling appearing anywhere in a summary.
fn spellings(summary: &SourceSummary) -> Vec<String> {
    let mut found = Vec::new();
    let mut visit = |value: &SourceValue| {
        let _ = visit_value(value, |value| {
            match &value.expression {
                SourceExpression::Binary { operator, .. } => {
                    found.push(format!("binary {operator}"))
                }
                SourceExpression::Unary { operator, .. } => found.push(format!("unary {operator}")),
                SourceExpression::Floating { .. } => found.push("literal floating".into()),
                SourceExpression::Character { .. } => found.push("literal character".into()),
                _ => {}
            }
            Ok(())
        });
    };
    for path in summary.paths() {
        for operation in &path.operations {
            match operation {
                SourceOperation::Read { place } => visit(place),
                SourceOperation::Write { place, value } => {
                    visit(place);
                    visit(value);
                }
                SourceOperation::LocalWrite { value, .. }
                | SourceOperation::ReturnValue { value }
                | SourceOperation::Guard {
                    condition: value, ..
                } => visit(value),
                SourceOperation::Call { arguments, .. } => arguments.iter().for_each(&mut visit),
                _ => {}
            }
        }
    }
    found
}

#[test]
fn cg01_legacy_route_refuses_each_node_by_name() {
    if !crate::test_support::clang_available() {
        return;
    }
    let legacy = &NodeAdmission::LEGACY;
    for op in E01_BINARY {
        for (_, c) in binary_positions(op) {
            assert_eq!(refusal(&c, legacy), named(&format!("binary {op}")), "{c}");
        }
    }
    for op in E02_UNARY {
        for (_, c) in unary_positions(op) {
            assert_eq!(refusal(&c, legacy), named(&format!("unary {op}")), "{c}");
        }
    }
    for op in E03_COMPOUND {
        for (position, c) in compound_positions(op) {
            match (op, position) {
                ("|=", "widened-short" | "long-field") => assert_eq!(
                    refusal(&c, legacy),
                    "compound assignment requires int operands"
                ),
                ("|=", _) => assert!(matches!(
                    import_with(&c, legacy).unwrap(),
                    SourceResult::Summary(_)
                )),
                _ => assert_eq!(refusal(&c, legacy), named(&format!("compound {op}")), "{c}"),
            }
        }
    }
    for (entry, _, c) in literal_positions() {
        let spelling = if entry == "float" {
            "literal floating"
        } else {
            "literal character"
        };
        assert_eq!(refusal(&c, legacy), named(spelling), "{c}");
    }
}

#[test]
fn cg01_admitting_route_imports_every_node_in_every_position() {
    if !crate::test_support::clang_available() {
        return;
    }
    let mut rows = 0;
    for op in E01_BINARY {
        for (position, c) in binary_positions(op) {
            let s = summary(&c);
            assert!(
                spellings(&s).contains(&format!("binary {op}")),
                "{position}: {c}"
            );
            s.paths()
                .iter()
                .for_each(|p| check_path_symbols(p).unwrap());
            rows += 1;
        }
    }
    for op in E02_UNARY {
        for (position, c) in unary_positions(op) {
            let s = summary(&c);
            assert!(
                spellings(&s).contains(&format!("unary {op}")),
                "{position}: {c}"
            );
            rows += 1;
        }
    }
    for (entry, position, c) in literal_positions() {
        let s = summary(&c);
        let spelling = if entry == "float" {
            "literal floating"
        } else {
            "literal character"
        };
        assert!(
            spellings(&s).iter().any(|found| found == spelling),
            "{position}: {c}"
        );
        rows += 1;
    }
    for op in E03_COMPOUND {
        let operator = op.strip_suffix('=').unwrap();
        for (position, c) in compound_positions(op) {
            let s = summary(&c);
            for path in s.paths() {
                check_path_symbols(path).unwrap();
            }
            assert!(
                spellings(&s).contains(&format!("binary {operator}")),
                "{position}: {c}"
            );
            rows += 1;
        }
    }
    assert_eq!(rows, 6 * 6 + 2 * 6 + 6 + 10 * 7);
}

/// A non-local place is evaluated once: one Read of the resolved place, the
/// operation at Clang's computation types with explicit conversions, and one
/// Write of the same place.
#[test]
fn cg01_compound_reads_once_converts_and_writes_back() {
    if !crate::test_support::clang_available() {
        return;
    }
    let s = summary(&format!(
        "{RECORD}void f(struct item *p,int a){{p->h -= a;}}"
    ));
    let [path] = s.paths() else { panic!() };
    let reads: Vec<_> = path
        .operations
        .iter()
        .filter_map(|o| match o {
            SourceOperation::Read { place } => Some(place),
            _ => None,
        })
        .collect();
    let writes: Vec<_> = path
        .operations
        .iter()
        .filter_map(|o| match o {
            SourceOperation::Write { place, value } => Some((place, value)),
            _ => None,
        })
        .collect();
    let ([read], [(written, stored)]) = (reads.as_slice(), writes.as_slice()) else {
        panic!("one read and one write expected: {:?}", path.operations)
    };
    assert_eq!(*read, *written, "the place is evaluated once");
    assert_eq!(read.c_type, "short");
    // short -> int (computation), `-` at int, int -> short (writeback).
    let SourceExpression::Cast {
        cast,
        value: result,
    } = &stored.expression
    else {
        panic!("conversion back to the place type: {stored:?}")
    };
    assert_eq!(
        (cast.as_str(), stored.c_type.as_str()),
        ("IntegralCast", "short")
    );
    let SourceExpression::Binary { operator, left, .. } = &result.expression else {
        panic!()
    };
    assert_eq!((operator.as_str(), result.c_type.as_str()), ("-", "int"));
    let SourceExpression::Cast { value: old, .. } = &left.expression else {
        panic!("old value converted to the computation type: {left:?}")
    };
    assert!(matches!(old.expression, SourceExpression::Symbol { .. }));
    assert_eq!(
        (left.c_type.as_str(), old.c_type.as_str()),
        ("int", "short")
    );
}

/// `int |= int` takes the CG01 route under an admission and the legacy route
/// without one; the two projections are identical.
#[test]
fn cg01_int_or_assign_matches_the_legacy_projection() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (position, c) in compound_positions("|=") {
        if matches!(position, "widened-short" | "long-field") {
            continue;
        }
        let SourceResult::Summary(legacy) = import_with(&c, &NodeAdmission::LEGACY).unwrap() else {
            panic!("{c}")
        };
        assert_eq!(summary(&c), legacy, "{position}");
    }
}

#[test]
fn cg01_refusal_controls() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (c, expected) in [
        // Unsequenced: `i++` is no expression of T, and a call beside a read
        // or in a compound assignment has no invented order.
        // (CG02 names the form: formerly `… UnaryOperator`.)
        (
            "void f(int *a,int i){a[i] -= i++;}",
            "typed-source value form `post increment`",
        ),
        (
            "int g(void);void f(int *p){*p += g();}",
            "unsequenced call/read operands",
        ),
        (
            "int g(void);int f(int a){int x = a; x -= g(); return x;}",
            "compound assignment with an opaque call",
        ),
        (
            "int g(void);void f(int *p){p[g()] ^= 1;}",
            "compound assignment with an effectful address",
        ),
        (
            "int g(void);int f(int *p,int a){return p[a] ^ g();}",
            "unsequenced call/read operands",
        ),
        (
            "int g(int);int f(int a,int b){return g(a) - g(b);}",
            "unsequenced call/read operands",
        ),
        (
            "int g(void);int f(int a){if((a << 1) >= g())return 1;return 0;}",
            "unsequenced call/read operands",
        ),
        // A compound assignment used as a value is CG02's, which names it
        // (flipped from `unmodeled callback expression CompoundAssignOperator`).
        (
            "int f(int a){int x = a; return x += 1;}",
            "typed-source value form `compound assignment value`",
        ),
        // Pointer arithmetic and comparison (M15/M16) and floating arithmetic.
        (
            "void g(long);void f(int *p,int *q){g(p - q);}",
            "requires promoted integer operands",
        ),
        (
            "char *f(char *p){return p - 1;}",
            "requires promoted integer operands",
        ),
        (
            "int f(int *p,int *q){return p <= q;}",
            "requires promoted integer operands",
        ),
        (
            "void g(double);void f(void){g(1.5 - 0.5);}",
            "requires promoted integer operands",
        ),
        (
            "void f(int *p){*p += 1.5;}",
            "requires admitted integer types",
        ),
    ] {
        let reason = refusal(c, &ALL);
        assert!(reason.contains(expected), "{c}: {reason}");
    }
    // A consumer that classifies only `^` refuses `-` by name.
    let partial = NodeAdmission::new(
        "partial test consumer",
        &[("binary ^", Classification::CustodyNeutral)],
    );
    assert_eq!(
        refusal("int f(int a,int b){return a - b;}", &partial),
        "typed-source node `binary -` is not classified by the partial test consumer"
    );
    assert!(matches!(
        import_with("int f(int a,int b){return a ^ b;}", &partial).unwrap(),
        SourceResult::Summary(_)
    ));
}

/// `x /= 0` and `a << -1` are read by the projection, where they are only
/// obligation-bearing; a numeric consumer's kernel refuses both.
#[test]
fn cg01_fault_cases_are_obligations_or_refused() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (c, spelling) in [
        ("int f(int a){int x = a; x /= 0; return x;}", "compound /="),
        ("int f(int a){return a << -1;}", "binary <<"),
    ] {
        summary(c);
        assert_ne!(ALL.admit(spelling), Ok(Classification::CustodyNeutral));
    }
    assert_eq!(checked_binary("/", 32, true, 7, 0), None);
    assert_eq!(checked_binary("<<", 32, true, 1, -1), None);
}

#[test]
fn cg01_admissions_cannot_misclassify_a_node() {
    for (rows, fragment) in [
        (
            &[("binary -", Classification::CustodyNeutral)][..],
            "cannot be CustodyNeutral",
        ),
        (
            &[("compound &=", Classification::CustodyNeutral)][..],
            "cannot be CustodyNeutral",
        ),
        (
            &[("compound ^=", Classification::NumericallyEvaluated)][..],
            "cannot be NumericallyEvaluated",
        ),
        (
            &[("binary ,", Classification::ObligationBearing)][..],
            "is not a CG01 node",
        ),
        (
            &[
                ("binary ^", Classification::CustodyNeutral),
                ("binary ^", Classification::ObligationBearing),
            ][..],
            "listed twice",
        ),
    ] {
        let rows: &'static [(&str, Classification)] = Box::leak(rows.to_vec().into_boxed_slice());
        let error = NodeAdmission::new("bad", rows).validate().unwrap_err();
        assert!(error.contains(fragment), "{error}");
        // The importer refuses the whole call before reading any function.
        let bad = NodeAdmission::new("bad", rows);
        assert!(import_with("int f(int a){return a;}", &bad).is_err());
    }
    ALL.validate().unwrap();
    NodeAdmission::LEGACY.validate().unwrap();
    for facts in NODE_TABLE {
        assert_eq!(
            classification_allowed(&facts, Classification::CustodyNeutral),
            facts.faults.is_empty() && !facts.store,
            "{}",
            facts.spelling
        );
    }
    assert_eq!(facts("binary -").unwrap().faults, [Fault::SignedOverflow]);
}

/// The unsigned kernel wraps as C defines; the signed one refuses overflow.
#[test]
fn cg01_checked_binary_matches_c_integer_faults() {
    let int_min = -(1i128 << 31);
    for (op, width, signed, a, b, expected) in [
        ("-", 32, true, int_min, 1, None),
        ("-", 32, false, 0, 1, Some((1i128 << 32) - 1)),
        ("/", 32, true, int_min, -1, None),
        ("%", 32, true, int_min, -1, None),
        ("%", 32, true, 7, 0, None),
        ("/", 32, true, -7, 2, Some(-3)),
        ("%", 32, true, -7, 2, Some(-1)),
        ("<<", 32, true, 1, 31, None),
        ("<<", 32, true, 1, 30, Some(1 << 30)),
        ("<<", 32, true, -1, 1, None),
        ("<<", 32, false, 1, 31, Some(1 << 31)),
        ("<<", 64, false, 1, 64, None),
        (">>", 32, true, -8, 1, None),
        (">>", 32, true, 8, 1, Some(4)),
        (">>", 32, true, 8, 32, None),
        ("^", 32, true, 5, 3, Some(6)),
        ("*", 64, true, 1 << 32, 1 << 31, None),
        ("?", 32, true, 1, 1, None),
    ] {
        assert_eq!(
            checked_binary(op, width, signed, a, b),
            expected,
            "{op} {width} {signed} {a} {b}"
        );
    }
}

/// The mechanical binding of `proof/QCore/TypedSourceOperators.lean`: its
/// grammar lists, node table and kernel vectors are the Rust ones.
#[test]
fn cg01_lean_vectors_match_the_rust_tables() {
    use crate::typed_source_nodes::{LEGACY_BINARY, LEGACY_UNARY};
    let lean =
        include_str!("../../../../proof/QCore/TypedSourceOperators.lean").replace("\r\n", "\n");
    let section = |start: &str| {
        let body = lean.split(start).nth(1).expect(start);
        body[..body.find("]\n").unwrap() + 1].to_owned()
    };
    let quoted = |text: &str| {
        text.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        quoted(&section("def legacyBinary : List String :=")),
        LEGACY_BINARY
    );
    assert_eq!(
        quoted(&section("def legacyUnary : List String :=")),
        LEGACY_UNARY
    );
    let fault = |name: &str| match name.trim().trim_start_matches('.') {
        "signedOverflow" => Fault::SignedOverflow,
        "divisionByZero" => Fault::DivisionByZero,
        "quotientOverflow" => Fault::QuotientOverflow,
        "shiftCount" => Fault::ShiftCount,
        "negativeLeftShift" => Fault::NegativeLeftShift,
        other => panic!("unknown Lean fault {other}"),
    };
    let table = lean
        .split("def nodeTable : List Facts := [")
        .nth(1)
        .unwrap();
    let table = &table[..table.find("\n]").unwrap()];
    let rows = table
        .split('⟨')
        .skip(1)
        .map(|row| {
            let (spelling, rest) = row.trim_start_matches('"').split_once('"').unwrap();
            let (faults, rest) = rest.split_once('[').unwrap().1.split_once(']').unwrap();
            let faults = faults
                .split(',')
                .filter(|name| !name.trim().is_empty())
                .map(fault)
                .collect::<Vec<_>>();
            (spelling.to_owned(), faults, rest.contains("true"))
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), NODE_TABLE.len());
    for ((spelling, faults, store), facts) in rows.iter().zip(NODE_TABLE) {
        assert_eq!(
            (spelling.as_str(), faults.as_slice(), *store),
            (facts.spelling, facts.faults, facts.store)
        );
    }
    let vectors = lean.split("def kernelVectors").nth(1).unwrap();
    let vectors = &vectors[..vectors.find("\n]").unwrap()];
    let mut count = 0;
    for row in vectors.split("\n  (.").skip(1) {
        let fields = row
            .trim_end_matches([',', ')'])
            .split(", ")
            .collect::<Vec<_>>();
        let op = match fields[0] {
            "add" => "+",
            "sub" => "-",
            "mul" => "*",
            "div" => "/",
            "rem" => "%",
            "shl" => "<<",
            "shr" => ">>",
            other => panic!("unknown Lean operation {other}"),
        };
        let number = |text: &str| text.trim_matches(['(', ')']).parse::<i128>().unwrap();
        let expected = match fields[5] {
            "none" => None,
            some => Some(number(some.strip_prefix("some ").unwrap())),
        };
        assert_eq!(
            checked_binary(
                op,
                fields[1].parse().unwrap(),
                fields[2] == "true",
                number(fields[3]),
                number(fields[4]),
            ),
            expected,
            "{row}"
        );
        count += 1;
    }
    assert_eq!(count, 19);
}
