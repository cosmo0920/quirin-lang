use super::*;
fn span() -> TaskSpan {
    TaskSpan { start: 0, end: 1 }
}
fn instruction(kind: TaskInstructionKind) -> TaskInstruction {
    TaskInstruction { span: span(), kind }
}
fn block(instructions: Vec<TaskInstructionKind>, kind: TaskTerminatorKind) -> TaskBlock {
    TaskBlock {
        instructions: instructions.into_iter().map(instruction).collect(),
        terminator: TaskTerminator { span: span(), kind },
    }
}
fn module(worker_error: Option<&str>) -> TaskModule {
    use TaskInstructionKind as I;
    use TaskTerminatorKind as T;
    let mut errors = BTreeSet::from([SPAWN_ERROR.into()]);
    if let Some(error) = worker_error {
        errors.insert(error.into());
    }
    let main = TaskFunction {
        name: "main".into(),
        parameters: vec![],
        result: ScalarType::I32,
        errors,
        declared_effects: TaskEffects {
            spawn: true,
            join: true,
            may_block: true,
        },
        entry: 0,
        span: span(),
        blocks: [
            (
                0,
                block(
                    vec![
                        I::BindScalar {
                            slot: 0,
                            value: ScalarOperand::I32(-42),
                        },
                        I::EnterScope { scope: 1 },
                    ],
                    T::TrySpawn {
                        scope: 1,
                        worker: 1,
                        arguments: vec![ScalarOperand::Slot(0)],
                        handle_slot: 1,
                        success: 1,
                        spawn_error: 2,
                    },
                ),
            ),
            (
                1,
                block(
                    vec![I::Join {
                        handle: 1,
                        outcome_slot: Some(2),
                    }],
                    T::TryOutcome {
                        result_slot: 2,
                        value_slot: 3,
                        error_slot: 4,
                        success: 3,
                        error: 4,
                    },
                ),
            ),
            (
                2,
                block(
                    vec![I::ExitScope { scope: 1 }],
                    T::Raise {
                        error: ErrorOperand::Tag(SPAWN_ERROR.into()),
                    },
                ),
            ),
            (
                3,
                block(
                    vec![I::ExitScope { scope: 1 }],
                    T::Return {
                        value: ScalarOperand::Slot(3),
                    },
                ),
            ),
            (
                4,
                block(
                    vec![I::ExitScope { scope: 1 }],
                    T::Raise {
                        error: ErrorOperand::Slot(4),
                    },
                ),
            ),
        ]
        .into(),
    };
    let worker = TaskFunction {
        name: "worker".into(),
        parameters: vec![(0, ScalarType::I32)],
        result: ScalarType::I32,
        errors: worker_error
            .map(|tag| BTreeSet::from([tag.into()]))
            .unwrap_or_default(),
        declared_effects: TaskEffects::default(),
        entry: 0,
        span: span(),
        blocks: [(
            0,
            block(
                vec![],
                if let Some(tag) = worker_error {
                    T::Raise {
                        error: ErrorOperand::Tag(tag.into()),
                    }
                } else {
                    T::Return {
                        value: ScalarOperand::Slot(0),
                    }
                },
            ),
        )]
        .into(),
    };
    TaskModule {
        entry: 0,
        functions: [(0, main), (1, worker)].into(),
        source_len: 1,
    }
}
#[test]
fn checked_creation_outcomes_and_copied_scalar_reference_are_distinct() {
    let checked = check_task_module(&module(None)).unwrap();
    let success = execute_task_module(&checked, &TaskExecutionScript::default()).unwrap();
    assert_eq!(success.root_outcome, TaskOutcome::I32(-42));
    assert_eq!(success.tasks[1].arguments, [ScalarValue::I32(-42)]);
    assert!(success.tasks[1].joined);
    assert_eq!(
        render_task_observation(&success),
        "QCORE_TASK_SPAWN 1 1\nQCORE_TASK_RESULT 1 I32 -42\nQCORE_TASK_JOIN 1\nQCORE_TASK_RESULT 0 I32 -42\n"
    );
    let failed = execute_task_module(
        &checked,
        &TaskExecutionScript {
            spawn_outcomes: vec![false],
        },
    )
    .unwrap();
    assert_eq!(failed.root_outcome, TaskOutcome::Error(SPAWN_ERROR.into()));
    assert_eq!(failed.tasks.len(), 1);
    assert_eq!(
        render_task_observation(&failed),
        "QCORE_TASK_CREATE_FAILED 1\nQCORE_TASK_RESULT 0 ERROR SpawnError\n"
    );
    assert!(
        execute_task_module(
            &checked,
            &TaskExecutionScript {
                spawn_outcomes: vec![false, true]
            }
        )
        .is_err()
    );
}
#[test]
fn joined_worker_error_keeps_its_exact_tag() {
    let checked = check_task_module(&module(Some("Worker::Failed"))).unwrap();
    let result = execute_task_module(&checked, &TaskExecutionScript::default()).unwrap();
    assert_eq!(
        result.root_outcome,
        TaskOutcome::Error("Worker::Failed".into())
    );
    assert_eq!(result.tasks[1].outcome, result.root_outcome);
    assert!(result.tasks[1].joined);
}
#[test]
fn absence_and_consumption_of_handle_are_checked_on_every_path() {
    use TaskInstructionKind as I;
    use TaskTerminatorKind as T;
    for mutation in 0..5 {
        let mut input = module(None);
        let main = input.functions.get_mut(&0).unwrap();
        match mutation {
            0 => main.blocks.get_mut(&2).unwrap().instructions.insert(
                0,
                instruction(I::Join {
                    handle: 1,
                    outcome_slot: None,
                }),
            ),
            1 => main
                .blocks
                .get_mut(&1)
                .unwrap()
                .instructions
                .push(instruction(I::Join {
                    handle: 1,
                    outcome_slot: None,
                })),
            2 => {
                main.blocks.get_mut(&1).unwrap().terminator.kind = T::Return {
                    value: ScalarOperand::I32(0),
                }
            }
            3 => {
                main.blocks.get_mut(&1).unwrap().instructions = vec![instruction(I::BindScalar {
                    slot: 9,
                    value: ScalarOperand::Slot(1),
                })]
            }
            4 => {
                main.blocks.get_mut(&1).unwrap().instructions[0].kind = I::Join {
                    handle: 1,
                    outcome_slot: None,
                }
            }
            _ => unreachable!(),
        }
        assert!(check_task_module(&input).is_err(), "mutation {mutation}");
    }
}
#[test]
fn second_creation_failure_cannot_strand_a_sibling() {
    let mut input = module(None);
    let main = input.functions.get_mut(&0).unwrap();
    main.blocks.insert(
        5,
        block(
            vec![],
            TaskTerminatorKind::TrySpawn {
                scope: 1,
                worker: 1,
                arguments: vec![ScalarOperand::Slot(0)],
                handle_slot: 10,
                success: 1,
                spawn_error: 2,
            },
        ),
    );
    let TaskTerminatorKind::TrySpawn { success, .. } =
        &mut main.blocks.get_mut(&0).unwrap().terminator.kind
    else {
        unreachable!()
    };
    *success = 5;
    assert!(check_task_module(&input).is_err());
}
#[test]
fn all_definitions_types_rows_and_source_spans_are_required() {
    for mutation in 0..8 {
        let mut input = module(None);
        match mutation {
            0 => input.functions.get_mut(&1).unwrap().result = ScalarType::Unit,
            1 => {
                input
                    .functions
                    .get_mut(&0)
                    .unwrap()
                    .declared_effects
                    .may_block = false
            }
            2 => {
                input.functions.get_mut(&0).unwrap().errors.clear();
            }
            3 => {
                input
                    .functions
                    .get_mut(&1)
                    .unwrap()
                    .blocks
                    .get_mut(&0)
                    .unwrap()
                    .terminator
                    .span
                    .end = 2
            }
            4 => {
                input
                    .functions
                    .get_mut(&1)
                    .unwrap()
                    .blocks
                    .get_mut(&0)
                    .unwrap()
                    .terminator
                    .kind = TaskTerminatorKind::Goto { target: 0 }
            }
            5 => {
                input
                    .functions
                    .get_mut(&1)
                    .unwrap()
                    .blocks
                    .get_mut(&0)
                    .unwrap()
                    .terminator
                    .kind = TaskTerminatorKind::Return {
                    value: ScalarOperand::Slot(99),
                }
            }
            6 => {
                let mut unused = input.functions[&1].clone();
                unused.name = "unused".into();
                unused.result = ScalarType::Unit;
                input.functions.insert(2, unused);
            }
            7 => {
                input
                    .functions
                    .get_mut(&0)
                    .unwrap()
                    .blocks
                    .get_mut(&0)
                    .unwrap()
                    .instructions
                    .insert(
                        0,
                        instruction(TaskInstructionKind::BindScalar {
                            slot: 0,
                            value: ScalarOperand::I32(0),
                        }),
                    );
            }
            _ => unreachable!(),
        }
        assert!(check_task_module(&input).is_err(), "mutation {mutation}");
    }
    assert!(check_task_module(&module(Some(SPAWN_ERROR))).is_err());
    assert!(check_task_module(&module(Some("bad\ntrace"))).is_err());
}

#[test]
fn declared_rows_permit_unused_operations_and_multiple_error_variants() {
    let mut input = module(Some("Worker::First"));
    input
        .functions
        .get_mut(&1)
        .unwrap()
        .errors
        .insert("Worker::Second".into());
    input
        .functions
        .get_mut(&0)
        .unwrap()
        .errors
        .insert("Worker::Second".into());
    input.functions.get_mut(&1).unwrap().declared_effects = TaskEffects {
        spawn: true,
        join: true,
        may_block: true,
    };
    let checked = check_task_module(&input).unwrap();
    let observation = execute_task_module(&checked, &TaskExecutionScript::default()).unwrap();
    assert_eq!(
        observation.root_outcome,
        TaskOutcome::Error("Worker::First".into())
    );
    input
        .functions
        .get_mut(&1)
        .unwrap()
        .blocks
        .get_mut(&0)
        .unwrap()
        .terminator
        .kind = TaskTerminatorKind::Raise {
        error: ErrorOperand::Tag("Worker::Second".into()),
    };
    let checked = check_task_module(&input).unwrap();
    assert_eq!(
        execute_task_module(&checked, &TaskExecutionScript::default())
            .unwrap()
            .root_outcome,
        TaskOutcome::Error("Worker::Second".into())
    );
    input
        .functions
        .get_mut(&0)
        .unwrap()
        .errors
        .remove("Worker::Second");
    assert!(check_task_module(&input).is_err());
}

#[test]
fn collecting_worker_error_is_distinct_from_propagation() {
    let mut input = module(Some("Worker::Failed"));
    let main = input.functions.get_mut(&0).unwrap();
    main.errors.remove("Worker::Failed");
    main.blocks.get_mut(&4).unwrap().terminator.kind = TaskTerminatorKind::Return {
        value: ScalarOperand::I32(7),
    };
    let checked = check_task_module(&input).unwrap();
    let observation = execute_task_module(&checked, &TaskExecutionScript::default()).unwrap();
    assert_eq!(
        observation.tasks[1].outcome,
        TaskOutcome::Error("Worker::Failed".into())
    );
    assert_eq!(observation.root_outcome, TaskOutcome::I32(7));
    assert!(observation.tasks[1].joined);
}

#[test]
fn finite_outcome_exploration_exhaustion_rejects_instead_of_sampling_success() {
    use TaskInstructionKind as I;
    use TaskTerminatorKind as T;
    let mut input = module(None);
    let worker = input.functions.get_mut(&1).unwrap();
    worker.parameters.clear();
    worker.result = ScalarType::Unit;
    worker.blocks.get_mut(&0).unwrap().terminator.kind = T::Return {
        value: ScalarOperand::Unit,
    };
    let main = input.functions.get_mut(&0).unwrap();
    main.errors.clear();
    main.result = ScalarType::Unit;
    main.blocks.clear();
    main.blocks.insert(
        0,
        block(vec![I::EnterScope { scope: 1 }], T::Goto { target: 1 }),
    );
    // Each creation can fail and continue, or succeed and join before continuing.
    // The graph and every path are finite, but their outcome combinations exceed
    // this profile's checker budget. None may be silently omitted.
    for i in 0..20 {
        let spawn_block = 1 + 2 * i;
        let join_block = spawn_block + 1;
        let next = spawn_block + 2;
        main.blocks.insert(
            spawn_block,
            block(
                vec![],
                T::TrySpawn {
                    scope: 1,
                    worker: 1,
                    arguments: vec![],
                    handle_slot: 100 + i,
                    success: join_block,
                    spawn_error: next,
                },
            ),
        );
        main.blocks.insert(
            join_block,
            block(
                vec![I::Join {
                    handle: 100 + i,
                    outcome_slot: None,
                }],
                T::Goto { target: next },
            ),
        );
    }
    main.blocks.insert(
        41,
        block(
            vec![I::ExitScope { scope: 1 }],
            T::Return {
                value: ScalarOperand::Unit,
            },
        ),
    );
    let errors = check_task_module(&input).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("exploration bound exhausted")),
        "{errors:?}"
    );
}
