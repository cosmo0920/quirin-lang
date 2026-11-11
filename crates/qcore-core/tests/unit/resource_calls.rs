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
        Type::I32,
        vec![
            Statement::Buffer(BufferEvent::Load {
                binding: "seen".into(),
                name: "x".into(),
                index: BufferOperand::Literal(0),
            }),
            Statement::Return(ReturnValue::Scalar(BufferOperand::Scalar("seen".into()))),
        ],
    );
    let fill = function(
        "fill",
        vec![
            parameter("x", Type::WriteBytes),
            parameter("value", Type::I32),
        ],
        Type::Unit,
        vec![
            Statement::Buffer(BufferEvent::Store {
                name: "x".into(),
                index: BufferOperand::Literal(0),
                value: BufferOperand::Scalar("value".into()),
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
                binding: Some("seen".into()),
                callee: 0,
                arguments: vec![Argument::Borrow("x".into())],
            },
            Statement::Call {
                binding: None,
                callee: 1,
                arguments: vec![
                    Argument::Lend("x".into()),
                    Argument::Scalar(BufferOperand::Scalar("seen".into())),
                ],
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
    assert_eq!(checked.minimum_extents(1), Some([1, 0].as_slice()));
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
    assert_eq!(refusal.category, RefusalCategory::Ownership);
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
    let refusal = check(Program {
        functions: vec![helper, main],
        entry: 1,
    })
    .unwrap_err();
    assert!(refusal.message.contains("requires extent 3"));
    assert_eq!(refusal.category, RefusalCategory::Extent);
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
    assert_eq!(
        refusal.message,
        "resource helper expanded-item bound exceeded"
    );
}

#[test]
fn high_arity_repeated_dag_charges_generated_events_and_operations() {
    let parameters = (0..MAX_PARAMETERS)
        .map(|index| parameter(&format!("p{index}"), Type::I32))
        .collect::<Vec<_>>();
    let arguments = (0..MAX_PARAMETERS)
        .map(|_| Argument::Scalar(BufferOperand::Literal(0)))
        .collect::<Vec<_>>();
    let mut functions = vec![function(
        "leaf",
        parameters,
        Type::Unit,
        vec![Statement::Return(ReturnValue::Unit)],
    )];
    functions.push(function(
        "fan",
        vec![],
        Type::Unit,
        vec![
            Statement::Call {
                binding: None,
                callee: 0,
                arguments: arguments.clone(),
            },
            Statement::Call {
                binding: None,
                callee: 0,
                arguments,
            },
            Statement::Return(ReturnValue::Unit),
        ],
    ));
    for level in 2..=7 {
        functions.push(function(
            &format!("fan{level}"),
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
    let entry = functions.len() - 1;
    assert_eq!(
        check(Program { functions, entry }).unwrap_err().message,
        "resource helper expanded-item bound exceeded"
    );
}

#[test]
fn declaration_order_cannot_hide_a_call_chain_over_the_depth_bound() {
    let mut functions = vec![function(
        "leaf",
        vec![],
        Type::Unit,
        vec![Statement::Return(ReturnValue::Unit)],
    )];
    for level in 1..=MAX_CALL_DEPTH {
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
                Statement::Return(ReturnValue::Unit),
            ],
        ));
    }
    let entry = functions.len() - 1;
    assert_eq!(
        check(Program { functions, entry }).unwrap_err().message,
        "resource helper call depth bound exceeded"
    );
}

#[test]
fn caller_owned_value_may_not_survive_an_allocating_helper_failure() {
    let make = function(
        "make",
        vec![],
        Type::OwnedBytes,
        vec![
            Statement::Buffer(BufferEvent::Acquire {
                name: "x".into(),
                extent: 1,
            }),
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
                binding: Some("x".into()),
                callee: 0,
                arguments: vec![],
            },
            Statement::Buffer(BufferEvent::Release { name: "x".into() }),
            Statement::Buffer(BufferEvent::Release { name: "b".into() }),
            Statement::Return(ReturnValue::Unit),
        ],
    );
    assert!(
        check(Program {
            functions: vec![make, main],
            entry: 1
        })
        .unwrap_err()
        .message
        .contains("strand caller-owned bytes")
    );
}

#[test]
fn borrowed_owner_may_not_survive_allocation_failure_inside_helper() {
    let helper = function(
        "helper",
        vec![parameter("view", Type::ReadBytes)],
        Type::OwnedBytes,
        vec![
            Statement::Buffer(BufferEvent::Acquire {
                name: "x".into(),
                extent: 1,
            }),
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
                binding: Some("x".into()),
                callee: 0,
                arguments: vec![Argument::Borrow("b".into())],
            },
            Statement::Buffer(BufferEvent::Release { name: "x".into() }),
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
        .contains("strand caller-owned bytes")
    );
}

#[test]
fn scalar_return_must_name_a_live_scalar_even_when_the_caller_discards_it() {
    let bad_helper = function(
        "bad",
        vec![],
        Type::I32,
        vec![Statement::Return(ReturnValue::Scalar(
            BufferOperand::Scalar("missing".into()),
        ))],
    );
    let main = function(
        "main",
        vec![],
        Type::Unit,
        vec![
            Statement::Call {
                binding: Some("ignored".into()),
                callee: 0,
                arguments: vec![],
            },
            Statement::Return(ReturnValue::Unit),
        ],
    );
    assert!(
        check(Program {
            functions: vec![bad_helper, main],
            entry: 1
        })
        .unwrap_err()
        .message
        .contains("live i32 binding")
    );
}

#[test]
fn acquisition_extents_are_refused_before_value_allocation() {
    let program = |extent| Program {
        functions: vec![function(
            "main",
            vec![],
            Type::Unit,
            vec![
                Statement::Buffer(BufferEvent::Acquire {
                    name: "b".into(),
                    extent,
                }),
                Statement::Buffer(BufferEvent::Release { name: "b".into() }),
                Statement::Return(ReturnValue::Unit),
            ],
        )],
        entry: 0,
    };
    for extent in [0, std_buffer::MAX_EXTENT + 1, u32::MAX] {
        assert_eq!(
            check(program(extent)).unwrap_err().category,
            RefusalCategory::Extent
        );
    }
    assert!(check(program(std_buffer::MAX_EXTENT)).is_ok());
}
