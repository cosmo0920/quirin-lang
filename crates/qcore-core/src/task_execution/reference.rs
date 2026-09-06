use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarValue {
    Unit,
    I32(i32),
}
impl ScalarValue {
    fn outcome(self) -> TaskOutcome {
        match self {
            Self::Unit => TaskOutcome::Unit,
            Self::I32(value) => TaskOutcome::I32(value),
        }
    }
}
/// Missing decisions select success; unused supplied decisions reject. These
/// choices drive one reference run and never change the checked source module.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskExecutionScript {
    pub spawn_outcomes: Vec<bool>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskObservation {
    pub task: u64,
    pub function: FunctionId,
    pub parent: Option<u64>,
    pub arguments: Vec<ScalarValue>,
    pub outcome: TaskOutcome,
    pub joined: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskEvent {
    Spawn { task: u64, worker: FunctionId },
    CreateFailed { worker: FunctionId },
    Result { task: u64, outcome: TaskOutcome },
    Join { task: u64 },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskExecutionObservation {
    pub root_outcome: TaskOutcome,
    pub tasks: Vec<TaskObservation>,
    pub events: Vec<TaskEvent>,
}
#[derive(Clone)]
enum Value {
    Scalar(ScalarValue),
    Result(TaskOutcome),
    Error(String),
    Handle { task: u64, handle: u64 },
    Consumed,
}
struct Runtime<'a> {
    module: &'a TaskModule,
    script: &'a TaskExecutionScript,
    decision: usize,
    next_task: u64,
    next_reservation: u64,
    next_scope: u64,
    ledger: ScopedTaskLedger,
    observations: Vec<TaskObservation>,
    events: Vec<TaskEvent>,
}
fn scalar(slots: &BTreeMap<SlotId, Value>, operand: &ScalarOperand) -> Result<ScalarValue, String> {
    match operand {
        ScalarOperand::Unit => Ok(ScalarValue::Unit),
        ScalarOperand::I32(value) => Ok(ScalarValue::I32(*value)),
        ScalarOperand::Slot(id) => match slots.get(id) {
            Some(Value::Scalar(value)) => Ok(value.clone()),
            _ => Err("reference scalar slot is unavailable".into()),
        },
    }
}
impl Runtime<'_> {
    fn execute(
        &mut self,
        id: FunctionId,
        task: u64,
        args: Vec<ScalarValue>,
    ) -> Result<TaskOutcome, TaskDiagnostic> {
        let function = &self.module.functions[&id];
        let mut slots = function
            .parameters
            .iter()
            .zip(args)
            .map(|(&(slot, _), value)| (slot, Value::Scalar(value)))
            .collect::<BTreeMap<_, _>>();
        let mut scopes = BTreeMap::<ScopeId, u64>::new();
        let mut block_id = function.entry;
        loop {
            let block = &function.blocks[&block_id];
            for instruction in &block.instructions {
                let result: Result<(), String> = (|| {
                    match &instruction.kind {
                        TaskInstructionKind::BindScalar { slot, value } => {
                            slots.insert(*slot, Value::Scalar(scalar(&slots, value)?));
                        }
                        TaskInstructionKind::UnwrapInfallible {
                            result_slot,
                            value_slot,
                        } => {
                            let Some(Value::Result(outcome)) = slots.get(result_slot).cloned()
                            else {
                                return Err("reference infallible Result unavailable".into());
                            };
                            let value = match outcome {
                                TaskOutcome::Unit => ScalarValue::Unit,
                                TaskOutcome::I32(value) => ScalarValue::I32(value),
                                TaskOutcome::Error(_) => {
                                    return Err(
                                        "reference error contradicts checked infallible Result"
                                            .into(),
                                    );
                                }
                            };
                            slots.insert(*result_slot, Value::Consumed);
                            slots.insert(*value_slot, Value::Scalar(value));
                        }
                        TaskInstructionKind::EnterScope { scope } => {
                            let actual = self.next_scope;
                            self.next_scope += 1;
                            self.ledger
                                .enter_scope(task, actual)
                                .map_err(|e| format!("{e:?}"))?;
                            scopes.insert(*scope, actual);
                        }
                        TaskInstructionKind::ExitScope { scope } => {
                            let actual =
                                scopes.remove(scope).ok_or("reference scope unavailable")?;
                            self.ledger
                                .exit_scope(task, actual)
                                .map_err(|e| format!("{e:?}"))?;
                        }
                        TaskInstructionKind::Join {
                            handle,
                            outcome_slot,
                        } => {
                            let Some(Value::Handle {
                                task: child,
                                handle: actual,
                            }) = slots.get(handle).cloned()
                            else {
                                return Err("reference handle unavailable".into());
                            };
                            let completion = self
                                .ledger
                                .join(task, actual)
                                .map_err(|e| format!("{e:?}"))?;
                            slots.insert(*handle, Value::Consumed);
                            self.events.push(TaskEvent::Result {
                                task: child,
                                outcome: completion.outcome.clone(),
                            });
                            self.events.push(TaskEvent::Join { task: child });
                            self.observations
                                .iter_mut()
                                .find(|o| o.task == child)
                                .ok_or("reference child observation missing")?
                                .joined = true;
                            if let Some(slot) = outcome_slot {
                                slots.insert(*slot, Value::Result(completion.outcome));
                            }
                        }
                    }
                    Ok(())
                })();
                result.map_err(|message| problem(id, instruction.span, message))?;
            }
            let result: Result<Option<TaskOutcome>, String> = (|| {
                match &block.terminator.kind {
                    TaskTerminatorKind::Goto { target } => block_id = *target,
                    TaskTerminatorKind::TrySpawn {
                        scope,
                        worker,
                        arguments,
                        handle_slot,
                        success,
                        spawn_error,
                    } => {
                        let arguments = arguments
                            .iter()
                            .map(|value| scalar(&slots, value))
                            .collect::<Result<Vec<_>, _>>()?;
                        let reservation = self.next_reservation;
                        self.next_reservation += 1;
                        let actual_scope = *scopes
                            .get(scope)
                            .ok_or("reference spawn scope unavailable")?;
                        self.ledger
                            .reserve(task, actual_scope, reservation, &[])
                            .map_err(|e| format!("{e:?}"))?;
                        let succeeds = self
                            .script
                            .spawn_outcomes
                            .get(self.decision)
                            .copied()
                            .unwrap_or(true);
                        self.decision += 1;
                        if succeeds {
                            let child = self.next_task;
                            self.next_task += 1;
                            self.ledger
                                .commit(task, reservation, child, child)
                                .map_err(|e| format!("{e:?}"))?;
                            self.events.push(TaskEvent::Spawn {
                                task: child,
                                worker: *worker,
                            });
                            // Leaf workers have no shared state. Eager reference
                            // evaluation selects a legal schedule, not a claim
                            // that the native worker completed before spawn returned.
                            let outcome = self
                                .execute(*worker, child, arguments.clone())
                                .map_err(|e| e.to_string())?;
                            self.ledger
                                .complete(child, outcome.clone(), &[])
                                .map_err(|e| format!("{e:?}"))?;
                            self.observations.push(TaskObservation {
                                task: child,
                                function: *worker,
                                parent: Some(task),
                                arguments,
                                outcome,
                                joined: false,
                            });
                            slots.insert(
                                *handle_slot,
                                Value::Handle {
                                    task: child,
                                    handle: child,
                                },
                            );
                            block_id = *success;
                        } else {
                            self.ledger
                                .rollback(task, reservation)
                                .map_err(|e| format!("{e:?}"))?;
                            self.events
                                .push(TaskEvent::CreateFailed { worker: *worker });
                            block_id = *spawn_error;
                        }
                    }
                    TaskTerminatorKind::TryOutcome {
                        result_slot,
                        value_slot,
                        error_slot,
                        success,
                        error,
                    } => {
                        let Some(Value::Result(outcome)) = slots.get(result_slot).cloned() else {
                            return Err("reference Result unavailable".into());
                        };
                        slots.insert(*result_slot, Value::Consumed);
                        match outcome {
                            TaskOutcome::Unit => {
                                slots.insert(*value_slot, Value::Scalar(ScalarValue::Unit));
                                block_id = *success;
                            }
                            TaskOutcome::I32(value) => {
                                slots.insert(*value_slot, Value::Scalar(ScalarValue::I32(value)));
                                block_id = *success;
                            }
                            TaskOutcome::Error(tag) => {
                                slots.insert(*error_slot, Value::Error(tag));
                                block_id = *error;
                            }
                        }
                    }
                    TaskTerminatorKind::Return { value } => {
                        return Ok(Some(scalar(&slots, value)?.outcome()));
                    }
                    TaskTerminatorKind::Raise { error } => {
                        let tag = match error {
                            ErrorOperand::Tag(tag) => tag.clone(),
                            ErrorOperand::Slot(slot) => match slots.get(slot) {
                                Some(Value::Error(tag)) => tag.clone(),
                                _ => return Err("reference error slot unavailable".into()),
                            },
                        };
                        return Ok(Some(TaskOutcome::Error(tag)));
                    }
                }
                Ok(None)
            })();
            if let Some(outcome) =
                result.map_err(|message| problem(id, block.terminator.span, message))?
            {
                return Ok(outcome);
            }
        }
    }
}
pub fn execute_task_module(
    checked: &CheckedTaskModule,
    script: &TaskExecutionScript,
) -> Result<TaskExecutionObservation, TaskDiagnostic> {
    let mut runtime = Runtime {
        module: checked.module(),
        script,
        decision: 0,
        next_task: 1,
        next_reservation: 1,
        next_scope: 1,
        ledger: ScopedTaskLedger::default(),
        observations: vec![],
        events: vec![],
    };
    let root_outcome = runtime.execute(runtime.module.entry, 0, vec![])?;
    if runtime.decision < script.spawn_outcomes.len() {
        return Err(problem(
            runtime.module.entry,
            TaskSpan::default(),
            "reference script contains unused creation outcomes",
        ));
    }
    runtime.events.push(TaskEvent::Result {
        task: 0,
        outcome: root_outcome.clone(),
    });
    runtime.observations.push(TaskObservation {
        task: 0,
        function: runtime.module.entry,
        parent: None,
        arguments: vec![],
        outcome: root_outcome.clone(),
        joined: false,
    });
    runtime.observations.sort_by_key(|o| o.task);
    Ok(TaskExecutionObservation {
        root_outcome,
        tasks: runtime.observations,
        events: runtime.events,
    })
}

pub fn render_task_observation(observation: &TaskExecutionObservation) -> String {
    let mut output = String::new();
    use std::fmt::Write;
    for event in &observation.events {
        match event {
            TaskEvent::Spawn { task, worker } => {
                writeln!(output, "QCORE_TASK_SPAWN {task} {worker}").unwrap()
            }
            TaskEvent::CreateFailed { worker } => {
                writeln!(output, "QCORE_TASK_CREATE_FAILED {worker}").unwrap()
            }
            TaskEvent::Join { task } => writeln!(output, "QCORE_TASK_JOIN {task}").unwrap(),
            TaskEvent::Result { task, outcome } => match outcome {
                TaskOutcome::Unit => writeln!(output, "QCORE_TASK_RESULT {task} UNIT").unwrap(),
                TaskOutcome::I32(value) => {
                    writeln!(output, "QCORE_TASK_RESULT {task} I32 {value}").unwrap()
                }
                TaskOutcome::Error(tag) => {
                    writeln!(output, "QCORE_TASK_RESULT {task} ERROR {tag}").unwrap()
                }
            },
        }
    }
    output
}
