use super::*;

fn bytes() -> DimensionRef {
    DimensionRef::Named("Bytes".into())
}

#[test]
fn same_dimension_addition_is_constructible() {
    let n = IndexTerm::variable("n", bytes());
    let term = n.checked_add(IndexTerm::literal(8, bytes())).unwrap();
    assert_eq!(term.dimension(), &bytes());
    assert!(matches!(term.expression(), IndexExpression::Add(_, _)));
}

#[test]
fn cross_dimension_addition_is_rejected() {
    let memory = IndexTerm::variable("memory", bytes());
    let files = IndexTerm::variable("files", DimensionRef::Count("Fd".into()));
    let error = memory.checked_add(files).unwrap_err();
    assert_eq!(error.operation, "addition");
    assert_eq!(error.left, bytes());
    assert_eq!(error.right, DimensionRef::Count("Fd".into()));
}

#[test]
fn scaling_is_constant_by_construction() {
    let term = IndexTerm::variable("n", bytes()).scale(16);
    assert!(matches!(term.expression(), IndexExpression::Scale(16, _)));
}
