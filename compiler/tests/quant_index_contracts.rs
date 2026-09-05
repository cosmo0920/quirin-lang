use qcore_compiler::{
    CompileOptions, IndexExpression, SemanticPass, SourceFile, compile_files, elaborate_source,
    parse_source_program,
};

fn source(quant: &str) -> String {
    format!(
        "dimension Bytes : Idx + Ordered; quant axis memory : credit<Bytes>; extern fn f(q: {quant} use one, tail: i32) -> unit; fn main() -> i32 {{ return 0; }}"
    )
}

#[test]
fn shorthand_and_vector_where_bindings_elaborate_to_dimensioned_values() {
    for quant in [
        "quant<n where n := 5>",
        "quant<credit, { memory = n } where n := m + 2, m := 3, n < 6, n > 4>",
    ] {
        let source = source(quant);
        let parsed = parse_source_program(&source).unwrap();
        let typed = elaborate_source(&parsed).unwrap();
        let function = typed
            .declarations
            .iter()
            .find(|decl| decl.name == "f")
            .unwrap();
        assert_eq!(function.parameters.len(), 2);
        let quant = &function.quant_index_types[0];
        assert_eq!(
            quant.fields["memory"].expression(),
            &IndexExpression::Literal(5)
        );
        assert_eq!(
            quant.bindings["n"].expression(),
            &IndexExpression::Literal(5)
        );
        assert_eq!(quant.fields["memory"].dimension().to_string(), "Bytes");
        assert!(parsed.syntax_equivalent(&parse_source_program(&parsed.to_string()).unwrap()));
        compile_files(
            &[SourceFile::new("where.qcore", source)],
            &CompileOptions::default(),
        )
        .unwrap();
    }
}

#[test]
fn rejects_false_cyclic_runtime_and_overflowing_index_bindings() {
    for quant in [
        "quant<n where n := 5, n <= 4>",
        "quant<n where n := n + 1>",
        "quant<n where n := m, m := n>",
        "quant<n where n := tail>",
        "quant<n where n := 5, n := 6>",
        "quant<q where q := 5>",
        "quant<n where n := 340282366920938463463374607431768211455 + 1>",
        "quant<n where n := monus(2, 3)>",
        "quant<n where n := 5, unused := 7>",
    ] {
        let errors = compile_files(
            &[SourceFile::new("bad.qcore", source(quant))],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.pass == SemanticPass::Index && error.code == "Q016"),
            "{quant}: {errors:?}"
        );
    }
}

#[test]
fn rejects_effectful_and_nonlinear_where_syntax() {
    for quant in [
        "quant<n where n := allocate()>",
        "quant<n where n := object.field>",
        "quant<n where n := *pointer>",
        "quant<n where n := -5>",
        "quant<n where n := 1.5>",
        "quant<n where n := m * k, m := 2, k := 3>",
    ] {
        assert!(
            compile_files(
                &[SourceFile::new("bad.qcore", source(quant))],
                &CompileOptions::default()
            )
            .is_err(),
            "{quant}"
        );
    }
}

#[test]
fn shorthand_is_ambiguous_with_two_axes_and_binders_cannot_cross_dimensions() {
    let preamble = "dimension Bytes : Idx + Ordered; dimension Seconds : Idx + Ordered; quant axis memory : credit<Bytes>; quant axis time : credit<Seconds>;";
    for quant in [
        "quant<n where n := 5>",
        "quant<credit, { memory = n, time = n } where n := 5>",
        "quant<credit, { memory = n, time = m } where n := 5, m := 5, n == m>",
    ] {
        let parsed =
            parse_source_program(&format!("{preamble} extern fn f(q: {quant}) -> unit;")).unwrap();
        assert!(elaborate_source(&parsed).is_err(), "{quant}");
    }
    let parsed = parse_source_program(&format!("{preamble} extern fn f(q: quant<credit, {{ memory = n, time = m }} where n := 5, m := 5>) -> unit;")).unwrap();
    assert!(elaborate_source(&parsed).is_ok());
}

#[test]
fn bindings_are_type_local_and_natural_operators_are_checked() {
    let text =
        source("quant<credit, { memory = n } where n := max(min(8, 6), monus(8, 2)) + 2 * 3>");
    let typed = elaborate_source(&parse_source_program(&text).unwrap()).unwrap();
    assert_eq!(
        typed.declarations[2].quant_index_types[0].bindings["n"].expression(),
        &IndexExpression::Literal(12)
    );
    let parsed = parse_source_program("quant axis slots : credit<Count<Slot>>; extern fn f(a: quant<n where n := 5>, b: quant<n where n := 7>) -> unit;").unwrap();
    let typed = elaborate_source(&parsed).unwrap();
    assert_eq!(
        typed.declarations[1].quant_index_types[0].bindings["n"].expression(),
        &IndexExpression::Literal(5)
    );
    assert_eq!(
        typed.declarations[1].quant_index_types[1].bindings["n"].expression(),
        &IndexExpression::Literal(7)
    );
    let leak = source("quant<n where n := 5>").replace("return 0;", "return n;");
    assert!(elaborate_source(&parse_source_program(&leak).unwrap()).is_err());
}

#[test]
fn shared_definition_dependencies_are_evaluated_without_exponential_expansion() {
    let mut bindings = vec!["n0 := 1".to_owned()];
    for n in 1..50 {
        bindings.push(format!("n{n} := n{} + n{}", n - 1, n - 1));
    }
    let text = source(&format!("quant<n49 where {}>", bindings.join(", ")));
    let typed = elaborate_source(&parse_source_program(&text).unwrap()).unwrap();
    assert_eq!(
        typed.declarations[2].quant_index_types[0].fields["memory"].expression(),
        &IndexExpression::Literal(1u128 << 49)
    );
}

#[test]
fn where_binders_cannot_shadow_visible_runtime_locals() {
    let text = "quant axis slots : credit<Count<Slot>>; fn main() -> i32 { let n: i32 = 3; let q: quant<n where n := 5>; return 0; }";
    let errors = compile_files(
        &[SourceFile::new("local.qcore", text)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors.iter().any(|error| error.code == "Q016"),
        "{errors:?}"
    );
}
