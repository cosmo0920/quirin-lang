use super::*;

#[test]
fn parses_and_pretty_prints_initial_declarations() {
    let source = "\
dimension Bytes : Idx + Ordered;
quant axis memory : live<Bytes>;
quant axis allocation : cost.total<Bytes>;
quant axis descriptors : credit<Count<Fd>>;
";
    let program = parse_program(source).unwrap();
    assert_eq!(program.declarations.len(), 4);
    assert_eq!(parse_program(&program.to_string()).unwrap(), program);
}

#[test]
fn rejects_variable_multiplication_as_a_declaration_start() {
    let error = parse_program("n * m").unwrap_err();
    assert!(error.message.contains("expected `dimension`"));
}
