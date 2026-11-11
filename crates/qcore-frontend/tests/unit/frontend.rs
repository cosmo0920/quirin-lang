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

use super::*;

const GOLDEN: &str = r#"dimension Bytes : Idx + Ordered;
quant axis memory : live<Bytes>;
type Buffer = own<array<u8, 16>>;
resource type File { repr: i32; state: Open; footprint: { files = 1 }; destructor: close noexcept; copy: forbidden; move: allowed; }
error Io { Failed { code: i32 }; }
const limit: usize = 16;
extern fn close(file: own<File> use one) -> unit effects { unsafe } contract { requires true; };
fn work[file_count: Idx<Count<File>>](file: own<File> use one, flag: bool) -> result<i32, {Io}> effects { throws<Io> } {
  let idx n = limit;
  let alias: borrow<'scope, const, File> = &file;
  guard n <= limit else raise Io::Failed { code: 1 };
  if (flag) { move alias; } else { defer close(move file); }
  match flag { true => { return 1; } false => { raise Io::Failed { code: 2 }; } }
  while (flag) invariant true quant invariant zero == zero bound 1 { continue; }
  error region cleanup { let temp = move file; raise Io::Failed { code: 3 }; } catch (Io::Failed { code }) quant open cleanup as pack { drop temp; discharge pack; }
  unsafe { return 0; }
}
#import std.concurrency; concurrency checked;
fn future(shared: i32) -> unit { footprint cell = shared; thread scope s { let task = spawn s work(copy shared); join move task; } with lock shared as guard { guard.value = 1; } }
"#;

#[test]
fn parses_complete_golden_surface_and_preserves_exact_text() {
    let program = parse_source_program(GOLDEN).unwrap();
    assert_eq!(program.declarations.len(), 10);
    assert_eq!(program.to_string(), GOLDEN);
    let reparsed = parse_source_program(&program.to_string()).unwrap();
    assert!(program.syntax_equivalent(&reparsed));
    assert!(
        program
            .declarations
            .iter()
            .all(|declaration| declaration.span.start < declaration.span.end)
    );
}

#[test]
fn retains_function_contracts_as_structured_source() {
    let source = "extern fn transfer(value: own<Item> use one) -> result<own<Item>, TransferError> contract { requires caller_owns_value; ensures normal custody_transferred; ensures error(TransferError) caller_owns_value; quant any live_resources == one; };";
    let program = parse_source_program(source).unwrap();
    let contract = program.declarations[0]
        .value
        .function_contract
        .as_ref()
        .unwrap();
    assert_eq!(contract.clauses.len(), 4);
    assert_eq!(
        contract.clauses[0].value.kind,
        SourceContractClauseKind::Requires
    );
    assert_eq!(
        contract.clauses[1].value.kind,
        SourceContractClauseKind::Ensures(SourceContractExitSelector::Normal)
    );
    assert_eq!(
        contract.clauses[2].value.kind,
        SourceContractClauseKind::Ensures(SourceContractExitSelector::Error(
            "TransferError".into()
        ))
    );
    assert_eq!(
        contract.clauses[3].value.kind,
        SourceContractClauseKind::Quant(SourceContractExitSelector::Any)
    );
    assert_eq!(
        contract.clauses[2].value.expression_source,
        "caller_owns_value"
    );
    let typed = elaborate_source(&program).unwrap();
    assert_eq!(
        typed.declarations[0]
            .function_contract
            .as_ref()
            .unwrap()
            .clauses,
        contract.clauses
    );
}

#[test]
fn rejects_malformed_function_contract_clauses() {
    for source in [
        "extern fn bad() -> unit contract { unknown true; };",
        "extern fn bad() -> unit contract { ensures normal; };",
        "extern fn bad() -> unit contract { ensures error() failed; };",
        "extern fn bad() -> unit contract { quant mystery zero == zero; };",
    ] {
        assert!(parse_source_program(source).is_err(), "accepted {source}");
    }
}

#[test]
fn classifies_all_sequential_statement_families() {
    let program = parse_source_program(GOLDEN).unwrap();
    let kinds = program
        .declarations
        .iter()
        .filter_map(|declaration| declaration.value.body.as_ref())
        .flat_map(|body| body.statements.iter().map(|statement| &statement.value))
        .collect::<Vec<_>>();
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, StatementKind::LetIndex { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, StatementKind::Guard))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, StatementKind::If { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, StatementKind::Match { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, StatementKind::While { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, StatementKind::ErrorRegion { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, StatementKind::Unsafe { .. }))
    );
}

#[test]
fn classifies_parameter_types_for_typed_source_ir() {
    let source = "#import std.concurrency; fn types(a: i32, b: own<File>, c: borrow<'s, const, File>, d: quant<access, p, FULL>, e: result<i32, {Io}>, f: task<'s, i32, {Io}>, g: mutex<File>) -> unit { return; }";
    let program = parse_source_program(source).unwrap();
    let parameters = &program.declarations[0].value.parameters;
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| parameter.type_kind)
            .collect::<Vec<_>>(),
        vec![
            SourceTypeKind::Atomic,
            SourceTypeKind::Owned,
            SourceTypeKind::Borrow,
            SourceTypeKind::AccessQuant,
            SourceTypeKind::Result,
            SourceTypeKind::Task,
            SourceTypeKind::Mutex,
        ]
    );
    let typed = elaborate_source(&program).unwrap();
    assert_eq!(
        typed.declarations[0].parameters[1].type_kind,
        SourceTypeKind::Owned
    );
}

#[test]
fn elaborates_names_and_attributes_statements_to_passes() {
    let source = "error Problem { Failed; } extern fn cleanup() -> unit; fn consume(x: i32) -> unit { let y: i32 = x; move y; drop x; raise Problem::Failed; defer cleanup(); unsafe { return; } }";
    let program = parse_source_program(source).unwrap();
    let typed = elaborate_source(&program).unwrap();
    let passes = covered_semantic_passes(&typed);
    assert!(passes.contains(&SemanticPass::NameResolution));
    assert!(passes.contains(&SemanticPass::OrdinaryTypes));
    assert!(passes.contains(&SemanticPass::Ownership));
    assert!(passes.contains(&SemanticPass::ResourceFlow));
    assert!(passes.contains(&SemanticPass::TypedErrors));
    assert!(passes.contains(&SemanticPass::CleanupLowering));
    assert!(passes.contains(&SemanticPass::CEmission));
}

#[test]
fn reports_duplicate_and_unknown_names_at_source_spans() {
    let source = "fn bad(x: i32, x: i32) -> unit { move missing; }";
    let program = parse_source_program(source).unwrap();
    let errors = elaborate_source(&program).unwrap_err();
    assert!(errors.iter().any(|error| error.code == "Q011"));
    assert!(errors.iter().any(|error| error.code == "Q013"));
    assert!(errors.iter().all(|error| error.span.end <= source.len()));
}

#[test]
fn rejects_unbalanced_and_unterminated_syntax() {
    assert!(parse_source_program("fn bad() -> unit { return;").is_err());
    assert!(parse_source_program("const x: i32 = 1").is_err());
    assert!(parse_source_program("dimension Bad: Idx;").is_err());
    assert!(parse_source_program("const x = 1;").is_err());
    assert!(parse_source_program("fn missing_return_type() { return; }").is_err());
}

#[test]
fn preserves_concrete_parameter_usage_grades_and_spans() {
    for (annotation, expected) in [
        ("zero", Usage::ZERO),
        ("one", Usage::ONE),
        ("affine", Usage::AFFINE),
        ("relevant", Usage::RELEVANT),
        ("many", Usage::MANY),
        ("2", Usage::finite(2, 2)),
        ("[0, 3]", Usage::finite(0, 3)),
        ("[2, inf]", Usage::unbounded(2)),
    ] {
        let source = format!("fn f(x: usize as idx n use {annotation}) -> unit {{ return; }}");
        let parsed = parse_source_program(&source).unwrap();
        let typed = elaborate_source(&parsed).unwrap();
        let grade = typed.declarations[0].parameters[0].usage.as_ref().unwrap();
        assert_eq!(grade.value, expected);
        assert_eq!(
            &source[grade.span.start..grade.span.end],
            format!("use {annotation}")
        );
    }
}

#[test]
fn rejects_invalid_and_unsupported_usage_annotations() {
    for annotation in [
        "",
        "mystery",
        "[2, 1]",
        "[0, n]",
        "-1",
        "one use many",
        "18446744073709551616",
        "[0, 18446744073709551616]",
    ] {
        let source = format!("fn f(x: i32 use {annotation}) -> unit {{ return; }}");
        assert!(
            parse_source_program(&source).is_err(),
            "accepted {annotation:?}"
        );
    }
}

#[test]
fn maps_every_existing_semantic_boundary_to_a_source_diagnostic() {
    #[derive(Debug)]
    struct TestError;
    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("test failure")
        }
    }
    impl std::error::Error for TestError {}

    let passes = [
        SemanticPass::NameResolution,
        SemanticPass::OrdinaryTypes,
        SemanticPass::Index,
        SemanticPass::Usage,
        SemanticPass::Ownership,
        SemanticPass::ResourceFlow,
        SemanticPass::TypedErrors,
        SemanticPass::ErrorRegion,
        SemanticPass::CleanupLowering,
        SemanticPass::CEmission,
        SemanticPass::Concurrency,
    ];
    for pass in passes {
        let diagnostic = source_diagnostic(pass, Span { start: 2, end: 4 }, &TestError);
        assert_eq!(diagnostic.pass, pass);
        assert_eq!(diagnostic.span, Span { start: 2, end: 4 });
        assert!(!diagnostic.code.is_empty());
    }
}

#[test]
fn concurrency_header_binders_have_body_local_symbol_identity() {
    let source = "#import std.concurrency; fn main(m: i32) -> unit { with lock m as g { g; } thread scope s { s; } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let function = &typed.declarations[0];
    let lock = &function.statements[0];
    let scope = &function.statements[1];
    assert_eq!(lock.referenced_symbols, vec![function.parameters[0].symbol]);
    assert!(scope.referenced_symbols.is_empty());
    assert_eq!(lock.nested[0].referenced_symbols.len(), 1);
    assert_eq!(scope.nested[0].referenced_symbols.len(), 1);
    assert_ne!(lock.nested[0].referenced_symbols, lock.referenced_symbols);
    assert_ne!(
        lock.nested[0].referenced_symbols,
        scope.nested[0].referenced_symbols
    );

    let source = "#import std.concurrency; fn main(m: i32) -> unit { with lock m as m { m; } m; }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let function = &typed.declarations[0];
    assert_ne!(
        function.statements[0].referenced_symbols,
        function.statements[0].nested[0].referenced_symbols
    );
    assert_eq!(
        function.statements[0].referenced_symbols,
        function.statements[1].referenced_symbols
    );
}

#[test]
fn concurrency_binders_do_not_hide_unresolved_inputs_or_escape_their_body() {
    for source in [
        "#import std.concurrency; fn main() -> unit { with lock missing as g { g; } }",
        "#import std.concurrency; fn main() -> unit { with lock m as m { m; } }",
        "#import std.concurrency; fn main(m: i32) -> unit { with lock m as g { } g; }",
        "#import std.concurrency; fn main() -> unit { thread scope s { } s; }",
        "#import std.concurrency; fn main() -> unit { thread scope s { } thread scope t { s; } }",
    ] {
        let errors = elaborate_source(&parse_source_program(source).unwrap()).unwrap_err();
        assert!(
            errors.iter().any(|error| error.code == "Q013"),
            "{source}: {errors:?}"
        );
    }
    for source in [
        "#import std.concurrency; fn main(m: i32) -> unit { with lock m as g { let g = 0; } }",
        "#import std.concurrency; fn main() -> unit { thread scope s { let s = 0; } }",
    ] {
        let errors = elaborate_source(&parse_source_program(source).unwrap()).unwrap_err();
        assert!(
            errors.iter().any(|error| error.code == "Q012"),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn review_r05_checks_axes_even_without_quantitative_types() {
    for suffix in [
        "",
        " quant axis slots : credit<Count<Slot>>; type T = quant<credit, { slots = n } where n := 1>;",
    ] {
        let source = format!("quant axis bad : credit<Missing>;{suffix}");
        let parsed = parse_source_program(&source).unwrap();
        let errors = elaborate_source(&parsed).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "Q016" && error.message.contains("unknown dimension")),
            "{errors:?}"
        );
        for prefix in [
            "dimension Bytes : Idx + Ordered; quant axis x : credit<Bytes>; quant axis x : credit<Bytes>;",
            "quant axis bad : missing<Count<T>>;",
            "dimension Bytes : Idx + Ordered; dimension Bytes : Idx + Ordered;",
        ] {
            let source = format!("{prefix}{suffix}");
            assert!(
                parse_source_program(&source)
                    .map(|program| elaborate_source(&program).is_err())
                    .unwrap_or(true),
                "accepted {source}"
            );
        }
        for prefix in [
            "quant axis objects : live<Count<UndeclaredNominal>>;",
            "quant axis memory : credit<Bytes>; dimension Bytes : Idx + Ordered;",
        ] {
            let source = format!("{prefix}{suffix}");
            assert!(
                elaborate_source(&parse_source_program(&source).unwrap()).is_ok(),
                "{source}"
            );
        }
    }
}

/// D01: the unconditional declaration check must attribute `Q016` to the
/// declaration that actually failed. It previously emitted `Span { 0, 0 }` for
/// every failure, which rendered as line 1 of the first input file, and it
/// forwarded parse-failure byte offsets that addressed the reconstructed
/// dimension/axis buffer rather than the caller's source. Rejection coverage is
/// unchanged; only the span and the reported offsets move.
#[test]
fn unconditional_declaration_checks_report_the_offending_span() {
    // Failures found by `StaticEnvironment::check` carry the offending
    // declaration's own span, even when earlier declarations are valid and the
    // offender is neither first nor last.
    let source = "\
dimension Bytes : Idx + Ordered;
quant axis good : credit<Bytes>;

quant axis bad : credit<Missing>;
fn main() -> i32 { return 0; }
";
    let errors = elaborate_source(&parse_source_program(source).unwrap()).unwrap_err();
    let index = errors
        .iter()
        .find(|error| error.code == "Q016")
        .unwrap_or_else(|| panic!("{errors:?}"));
    assert_eq!(index.pass, SemanticPass::Index);
    assert_eq!(index.message, "unknown dimension `Missing`");
    assert_eq!(
        &source[index.span.start..index.span.end],
        "quant axis bad : credit<Missing>;"
    );

    // The duplicate-declaration pre-pass and the unconditional index pass now
    // agree on where the same duplicate lives.
    let source = "\
dimension Bytes : Idx + Ordered;
quant axis good : credit<Bytes>;

quant axis good : credit<Bytes>;
fn main() -> i32 { return 0; }
";
    let errors = elaborate_source(&parse_source_program(source).unwrap()).unwrap_err();
    let spans = errors
        .iter()
        .filter(|error| error.code == "Q010" || error.code == "Q016")
        .map(|error| error.span.start)
        .collect::<Vec<_>>();
    assert_eq!(spans.len(), 2, "{errors:?}");
    assert_eq!(spans[0], spans[1]);
    assert_eq!(&source[spans[0]..spans[0] + 5], "quant");

    // Parse failures inside the reconstructed buffer report the caller's own
    // byte offsets. `missing` sits at byte 48 of this source; the reconstructed
    // buffer places it at 17.
    let source = "\
fn pad() -> i32 { return 0; }

quant axis bad : missing<Count<T>>;
fn main() -> i32 { return 0; }
";
    assert_eq!(source.find("missing"), Some(48));
    let errors = elaborate_source(&parse_source_program(source).unwrap()).unwrap_err();
    let index = errors
        .iter()
        .find(|error| error.code == "Q016")
        .unwrap_or_else(|| panic!("{errors:?}"));
    assert_eq!(&source[index.span.start..index.span.end], "missing");
    assert_eq!(index.message, "expected axis mode at bytes 48..55");
}

#[test]
fn review_r04_retains_result_annotation_and_profile() {
    let source = "extern fn external() -> result<own<Item>, {Failure}> effects { unsafe }; fn main() -> i32 /* retained */ { return 0; }";
    let parsed = parse_source_program(source).unwrap();
    let typed = elaborate_source(&parsed).unwrap();
    for (declaration, typed) in parsed.declarations.iter().zip(&typed.declarations) {
        let result = declaration.value.result_type.as_ref().unwrap();
        assert_eq!(&source[result.span.start..result.span.end], result.source);
        assert_eq!(typed.result_type.as_ref(), Some(result));
    }
    assert_eq!(
        typed.declarations[0].result_type.as_ref().unwrap().source,
        "result<own<Item>, {Failure}>"
    );
    assert_eq!(
        typed.declarations[0]
            .result_type
            .as_ref()
            .unwrap()
            .executable_type(),
        None
    );
    assert_eq!(
        typed.declarations[1]
            .result_type
            .as_ref()
            .unwrap()
            .executable_type(),
        Some(ExecutableResultType::I32)
    );
    for source in [
        "fn main() -> { return 0; }",
        "extern fn f() -> ;",
        "fn main() -> own<Item { return 0; }",
    ] {
        assert!(parse_source_program(source).is_err(), "accepted {source}");
    }
}

#[test]
fn review_r04_preserves_quantitative_result_comparison_syntax() {
    let source = "quant axis slots : credit<Count<Slot>>; extern fn measured() -> quant<n where n := 4, n > 3> effects { unsafe };";
    let parsed = parse_source_program(source).unwrap();
    let typed = elaborate_source(&parsed).unwrap();
    let result = typed
        .declarations
        .last()
        .unwrap()
        .result_type
        .as_ref()
        .unwrap();
    assert_eq!(result.source, "quant<n where n := 4, n > 3>");
    assert_eq!(result.executable_type(), None);
}

#[test]
fn retains_formal_proposition_kind_and_source_without_reserving_relation_names() {
    let source = "module contract; extern fn proposition() -> unit contract { quant proposition #p(#r: predicate<>) = #r(); };";
    let program = parse_source_program(source).unwrap();
    let clause = &program.declarations[0]
        .value
        .function_contract
        .as_ref()
        .unwrap()
        .clauses[0]
        .value;
    assert_eq!(clause.kind, SourceContractClauseKind::QuantProposition);
    assert_eq!(clause.expression_source, "#p(#r: predicate<>) = #r()");
    assert_eq!(program.to_string(), format!("{source}\n"));
}
