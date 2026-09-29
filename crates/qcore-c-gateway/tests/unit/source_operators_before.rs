// CG01-T-OPERATOR-CLOSURE BEFORE: the cited refusals (gap entries E01, E02,
// E03 for T, E14; evidence Q11, Q12) executed on route T through the legacy
// entry point every consumer calls. Each row prints `BEFORE <entry> <op>
// <position> <result>` so the run log is the reproduction record.
use super::*;
#[path = "source_operators_vectors.rs"]
mod vectors;
use vectors::*;

fn legacy(c: &str) -> SourceResult {
    import_source_functions(
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
    )
    .unwrap()
    .remove(0)
}

fn outcome(result: &SourceResult) -> String {
    match result {
        SourceResult::Summary(_) => "summary".into(),
        SourceResult::Unknown { reason, .. } => format!("unknown: {reason}"),
    }
}

/// The legacy entry point refuses every E01/E02/E14 row and every compound row
/// except `int |= int`, which T already admitted as a statement.
#[test]
fn cg01_before_legacy_route_refuses_the_cited_operators() {
    if !crate::test_support::clang_available() {
        return;
    }
    let mut rows = 0;
    for op in E01_BINARY {
        for (position, c) in binary_positions(op) {
            let result = outcome(&legacy(&c));
            println!("BEFORE E01 {op} {position} {result}");
            assert_eq!(
                result,
                "unknown: unmodeled callback expression BinaryOperator",
                "{c}"
            );
            rows += 1;
        }
    }
    for op in E02_UNARY {
        for (position, c) in unary_positions(op) {
            let result = outcome(&legacy(&c));
            println!("BEFORE E02 {op} {position} {result}");
            assert_eq!(
                result,
                "unknown: unmodeled callback expression UnaryOperator",
                "{c}"
            );
            rows += 1;
        }
    }
    for op in E03_COMPOUND {
        for (position, c) in compound_positions(op) {
            let result = outcome(&legacy(&c));
            println!("BEFORE E03 {op} {position} {result}");
            let expected = match (op, position) {
                ("|=", "widened-short" | "long-field") => {
                    "unknown: compound assignment requires int operands"
                }
                ("|=", _) => "summary",
                _ => "unknown: unmodeled callback statement CompoundAssignOperator",
            };
            assert_eq!(result, expected, "{c}");
            rows += 1;
        }
    }
    for (entry, position, c) in literal_positions() {
        let result = outcome(&legacy(&c));
        println!("BEFORE E14 {entry} {position} {result}");
        let kind = if entry == "float" {
            "FloatingLiteral"
        } else {
            "CharacterLiteral"
        };
        assert_eq!(
            result,
            format!("unknown: unmodeled callback expression {kind}"),
            "{c}"
        );
        rows += 1;
    }
    assert_eq!(rows, 6 * 6 + 2 * 6 + 10 * 7 + 6);
}
