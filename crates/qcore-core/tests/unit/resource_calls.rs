use super::*;

fn parameter(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.into(),
        ty,
    }
}

fn function(
    name: &str,
    parameters: Vec<Parameter>,
    result: Type,
    statements: Vec<Statement>,
) -> Function {
    Function {
        name: name.into(),
        parameters,
        result,
        declared_effects: None,
        statements,
    }
}

#[test]
fn owned_round_trip_and_borrowed_processing_use_both_existing_checkers() {
    let inspect = function(
        "inspect",
        vec![parameter("x", Type::ReadBytes)],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Load {
                binding: "seen".into(),
                name: "x".into(),
                index: BufferOperand::Literal(0),
            }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    let fill = function(
        "fill",
        vec![parameter("x", Type::WriteBytes)],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Store {
                name: "x".into(),
                index: BufferOperand::Literal(0),
                value: BufferOperand::Literal(42),
            }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    let pass = function(
        "pass",
        vec![parameter("x", Type::OwnedBytes)],
        Type::OwnedBytes,
        vec![
            Statement::Call {
                binding: None,
                callee: 0,
                arguments: vec![Argument::Borrow("x".into())],
            },
            Statement::Call {
                binding: None,
                callee: 1,
                arguments: vec![Argument::Lend("x".into())],
            },
            Statement::Return(ReturnValue::Move("x".into())),
        ],
    );
    let main = function(
        "main",
        vec![],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Acquire {
                name: "b".into(),
                extent: 1,
            }),
            Statement::Call {
                binding: Some("out".into()),
                callee: 2,
                arguments: vec![Argument::Move("b".into())],
            },
            Statement::Buffer(BufferEvent::Load {
                binding: "answer".into(),
                name: "out".into(),
                index: BufferOperand::Literal(0),
            }),
            Statement::Buffer(BufferEvent::Release { name: "out".into() }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    let checked = check(Program {
        functions: vec![inspect, fill, pass, main],
        entry: 3,
    })
    .unwrap();
    assert_eq!(checked.minimum_extents(0), Some([1].as_slice()));
    assert_eq!(checked.minimum_extents(1), Some([1].as_slice()));
    assert_eq!(checked.certificate().releases.len(), 1);
    assert!(
        checked
            .ownership_operations()
            .iter()
            .any(|op| matches!(op, OwnershipOp::Move { .. }))
    );
}

#[test]
fn one_owner_cannot_be_moved_and_borrowed_in_the_same_call() {
    let helper = function(
        "helper",
        vec![
            parameter("a", Type::OwnedBytes),
            parameter("b", Type::ReadBytes),
        ],
        Type::OwnedBytes,
        vec![Statement::Return(ReturnValue::Move("a".into()))],
    );
    let main = function(
        "main",
        vec![],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Acquire {
                name: "b".into(),
                extent: 1,
            }),
            Statement::Call {
                binding: Some("out".into()),
                callee: 0,
                arguments: vec![Argument::Move("b".into()), Argument::Borrow("b".into())],
            },
            Statement::Buffer(BufferEvent::Release { name: "out".into() }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    let refusal = check(Program {
        functions: vec![helper, main],
        entry: 1,
    })
    .unwrap_err();
    assert!(refusal.message.contains("incompatible owned/borrow modes"));
}

#[test]
fn caller_binding_is_stale_after_move() {
    let helper = function(
        "helper",
        vec![parameter("x", Type::OwnedBytes)],
        Type::OwnedBytes,
        vec![Statement::Return(ReturnValue::Move("x".into()))],
    );
    let main = function(
        "main",
        vec![],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Acquire {
                name: "b".into(),
                extent: 1,
            }),
            Statement::Call {
                binding: Some("out".into()),
                callee: 0,
                arguments: vec![Argument::Move("b".into())],
            },
            Statement::Buffer(BufferEvent::Release { name: "b".into() }),
            Statement::Buffer(BufferEvent::Release { name: "out".into() }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    assert!(
        check(Program {
            functions: vec![helper, main],
            entry: 1
        })
        .unwrap_err()
        .message
        .contains("drop after owned move")
    );
}

#[test]
fn allocation_with_a_live_input_refuses_the_failure_exit() {
    let helper = function(
        "helper",
        vec![parameter("x", Type::OwnedBytes)],
        Type::OwnedBytes,
        vec![
            Statement::Buffer(BufferEvent::Acquire {
                name: "fresh".into(),
                extent: 1,
            }),
            Statement::Buffer(BufferEvent::Release { name: "x".into() }),
            Statement::Return(ReturnValue::Move("fresh".into())),
        ],
    );
    let main = function(
        "main",
        vec![],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Acquire {
                name: "b".into(),
                extent: 1,
            }),
            Statement::Call {
                binding: Some("out".into()),
                callee: 0,
                arguments: vec![Argument::Move("b".into())],
            },
            Statement::Buffer(BufferEvent::Release { name: "out".into() }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    let refusal = check(Program {
        functions: vec![helper, main],
        entry: 1,
    })
    .unwrap_err();
    assert!(refusal.message.contains("allocation failure would leave"));
}

#[test]
fn uncalled_resource_helpers_refuse_in_the_closed_profile() {
    let helper = function(
        "helper",
        vec![parameter("x", Type::ReadBytes)],
        Type::Unit,
        vec![Statement::Return(ReturnValue::Unit)],
    );
    let main = function(
        "main",
        vec![],
        Type::Unit,
        vec![Statement::Return(ReturnValue::Unit)],
    );
    assert!(
        check(Program {
            functions: vec![helper, main],
            entry: 1
        })
        .unwrap_err()
        .message
        .contains("every resource helper must be instantiated")
    );
}

#[test]
fn call_site_must_discharge_the_helpers_minimum_extent() {
    let helper = function(
        "helper",
        vec![parameter("x", Type::ReadBytes)],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Load {
                binding: "v".into(),
                name: "x".into(),
                index: BufferOperand::Literal(2),
            }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    let main = function(
        "main",
        vec![],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Acquire {
                name: "b".into(),
                extent: 2,
            }),
            Statement::Call {
                binding: None,
                callee: 0,
                arguments: vec![Argument::Borrow("b".into())],
            },
            Statement::Buffer(BufferEvent::Release { name: "b".into() }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    assert!(
        check(Program {
            functions: vec![helper, main],
            entry: 1
        })
        .unwrap_err()
        .message
        .contains("requires extent 3")
    );
}

#[test]
fn acyclic_dag_expansion_is_bounded_before_it_can_grow_exponentially() {
    let mut functions = vec![function(
        "leaf",
        vec![],
        Type::Unit,
        vec![Statement::Return(ReturnValue::Unit)],
    )];
    for level in 1..16 {
        functions.push(function(
            &format!("f{level}"),
            vec![],
            Type::Unit,
            vec![
                Statement::Call {
                    binding: None,
                    callee: level - 1,
                    arguments: vec![],
                },
                Statement::Call {
                    binding: None,
                    callee: level - 1,
                    arguments: vec![],
                },
                Statement::Return(ReturnValue::Unit),
            ],
        ));
    }
    let refusal = check(Program {
        functions,
        entry: 15,
    })
    .unwrap_err();
    assert_eq!(refusal.message, "resource helper expansion bound exceeded");
}
