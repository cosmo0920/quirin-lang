use super::*;

#[test]
fn symbolic_references_require_prior_producers_with_matching_types() {
    let read = SourceOperation::Read {
        place: value("int".into(), SourceExpression::Parameter { index: 0 }),
    };
    for (operation, c_type) in [(0, "int"), (1, "int"), (0, "short")] {
        let path = SourcePath {
            operations: vec![
                read.clone(),
                SourceOperation::Guard {
                    condition: value(c_type.into(), SourceExpression::Symbol { operation }),
                    taken: true,
                },
            ],
        };
        assert_eq!(
            check_path_symbols(&path).is_ok(),
            operation == 0 && c_type == "int"
        );
    }
    let path = SourcePath {
        operations: vec![
            SourceOperation::Return,
            SourceOperation::Guard {
                condition: value("int".into(), SourceExpression::Symbol { operation: 0 }),
                taken: true,
            },
        ],
    };
    assert!(check_path_symbols(&path).is_err());
}

#[test]
fn aggregate_symbol_validation_rejects_nested_forward_reference() {
    let path = SourcePath {
        operations: vec![SourceOperation::LocalWrite {
            local: 0,
            value: value(
                "struct sample_pair".into(),
                SourceExpression::Aggregate {
                    elements: vec![value(
                        "long".into(),
                        SourceExpression::Symbol { operation: 0 },
                    )],
                },
            ),
        }],
    };
    assert!(check_path_symbols(&path).is_err());
}

#[test]
fn induction_value_cannot_escape_its_scan() {
    let invalid = SourcePath {
        operations: vec![SourceOperation::ReturnValue {
            value: value("int".into(), SourceExpression::LoopIndex { local: 3 }),
        }],
    };
    assert!(check_path_symbols(&invalid).is_err());
}
