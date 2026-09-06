use crate::parse_program;

use super::*;

#[test]
fn validates_named_and_count_dimensions() {
    let program = parse_program(
        "\
dimension Bytes : Idx + Ordered;
quant axis memory : live<Bytes>;
quant axis files : live<Count<Fd>>;
",
    )
    .unwrap();
    let environment = StaticEnvironment::check(&program).unwrap();
    assert!(environment.dimensions.contains("Bytes"));
    assert_eq!(environment.axes.len(), 2);
}

#[test]
fn rejects_unknown_named_dimension() {
    let program = parse_program("quant axis memory : live<Bytes>;").unwrap();
    assert_eq!(
        StaticEnvironment::check(&program),
        Err(CheckError::UnknownDimension("Bytes".into()))
    );
}

#[test]
fn allows_forward_dimension_references() {
    let program = parse_program(
        "\
quant axis memory : live<Bytes>;
dimension Bytes : Idx + Ordered;
",
    )
    .unwrap();
    assert!(StaticEnvironment::check(&program).is_ok());
}
