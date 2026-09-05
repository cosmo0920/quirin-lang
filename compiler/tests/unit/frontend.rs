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
concurrency checked;
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
    let source = "fn types(a: i32, b: own<File>, c: borrow<'s, const, File>, d: quant<access, p, FULL>, e: result<i32, {Io}>, f: task<'s, i32, {Io}>, g: mutex<File>) -> unit { return; }";
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
