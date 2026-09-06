use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarValue {
    Unit,
    I32(i32),
    OwnedBuffer(ResourceId),
}
impl ScalarValue {
    fn outcome(self) -> TaskOutcome {
        match self {
            Self::Unit => TaskOutcome::Unit,
            Self::I32(value) => TaskOutcome::I32(value),
            Self::OwnedBuffer(id) => TaskOutcome::OwnedBuffer(id),
        }
    }
}
/// Missing decisions select success; unused supplied decisions reject. These
/// choices drive one reference run and never change the checked source module.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskExecutionScript {
    pub spawn_outcomes: Vec<bool>,
    pub allocation_outcomes: Vec<bool>,
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
    BufferAllocated {
        task: u64,
        buffer: ResourceId,
        initial: i32,
    },
    BufferAllocationFailed {
        task: u64,
    },
    BufferRead {
        task: u64,
        buffer: ResourceId,
        value: i32,
    },
    BufferDropped {
        task: u64,
        buffer: ResourceId,
    },
    Spawn {
        task: u64,
        worker: FunctionId,
    },
    CreateFailed {
        worker: FunctionId,
    },
    Result {
        task: u64,
        outcome: TaskOutcome,
    },
    Join {
        task: u64,
    },
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
    allocation_decision: usize,
    next_buffer: u64,
    buffers: BTreeMap<ResourceId, i32>,
    buffer_events: BTreeMap<u64, Vec<TaskEvent>>,
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
        ScalarOperand::Move(id) => match slots.get(id) {
            Some(Value::Scalar(value @ ScalarValue::OwnedBuffer(_))) => Ok(value.clone()),
            _ => Err("reference owned slot is unavailable".into()),
        },
        ScalarOperand::Slot(id) => match slots.get(id) {
            Some(Value::Scalar(value @ (ScalarValue::I32(_) | ScalarValue::Unit))) => {
                Ok(value.clone())
            }
            _ => Err("reference scalar slot is unavailable".into()),
        },
    }
}
fn take(
    slots: &mut BTreeMap<SlotId, Value>,
    operand: &ScalarOperand,
) -> Result<ScalarValue, String> {
    let value = scalar(slots, operand)?;
    if let ScalarOperand::Move(id) = operand {
        slots.insert(*id, Value::Consumed);
    }
    Ok(value)
}
impl Runtime<'_> {
    fn buffer_event(&mut self, task: u64, event: TaskEvent) {
        if task == 0 {
            self.events.push(event);
        } else {
            self.buffer_events.entry(task).or_default().push(event);
        }
    }

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
                        TaskInstructionKind::ReadBuffer { buffer, value_slot } => {
                            let ScalarValue::OwnedBuffer(resource) =
                                scalar(&slots, &ScalarOperand::Move(*buffer))?
                            else {
                                return Err("reference buffer read requires owner".into());
                            };
                            self.ledger
                                .use_resource(task, resource)
                                .map_err(|e| format!("{e:?}"))?;
                            let value = *self
                                .buffers
                                .get(&resource)
                                .ok_or("reference buffer payload missing")?;
                            self.buffer_event(
                                task,
                                TaskEvent::BufferRead {
                                    task,
                                    buffer: resource,
                                    value,
                                },
                            );
                            slots.insert(*value_slot, Value::Scalar(ScalarValue::I32(value)));
                        }
                        TaskInstructionKind::DropBuffer { buffer } => {
                            let ScalarValue::OwnedBuffer(resource) =
                                take(&mut slots, &ScalarOperand::Move(*buffer))?
                            else {
                                return Err("reference buffer drop requires owner".into());
                            };
                            self.ledger
                                .drop_resource(task, resource)
                                .map_err(|e| format!("{e:?}"))?;
                            self.buffers
                                .remove(&resource)
                                .ok_or("reference buffer payload missing")?;
                            self.buffer_event(
                                task,
                                TaskEvent::BufferDropped {
                                    task,
                                    buffer: resource,
                                },
                            );
                        }
                        TaskInstructionKind::BindScalar { slot, value } => {
                            let value = take(&mut slots, value)?;
                            slots.insert(*slot, Value::Scalar(value));
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
                                TaskOutcome::OwnedBuffer(id) => ScalarValue::OwnedBuffer(id),
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
                            self.events
                                .extend(self.buffer_events.remove(&child).unwrap_or_default());
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
                    TaskTerminatorKind::TryAllocateBuffer {
                        initial,
                        value_slot,
                        success,
                        allocation_error,
                    } => {
                        let ScalarValue::I32(initial) = scalar(&slots, initial)? else {
                            return Err("reference allocation requires i32".into());
                        };
                        if task != 0 {
                            return Err("reference allocation requires root".into());
                        }
                        let succeeds = self
                            .script
                            .allocation_outcomes
                            .get(self.allocation_decision)
                            .copied()
                            .unwrap_or(true);
                        self.allocation_decision += 1;
                        if succeeds {
                            let resource = self.next_buffer;
                            self.next_buffer = self
                                .next_buffer
                                .checked_add(1)
                                .ok_or("reference resource identity exhausted")?;
                            self.ledger
                                .acquire(task, resource)
                                .map_err(|e| format!("{e:?}"))?;
                            self.buffers.insert(resource, initial);
                            slots.insert(
                                *value_slot,
                                Value::Scalar(ScalarValue::OwnedBuffer(resource)),
                            );
                            self.buffer_event(
                                task,
                                TaskEvent::BufferAllocated {
                                    task,
                                    buffer: resource,
                                    initial,
                                },
                            );
                            block_id = *success;
                        } else {
                            self.buffer_event(task, TaskEvent::BufferAllocationFailed { task });
                            block_id = *allocation_error;
                        }
                    }
                    TaskTerminatorKind::Goto { target } => block_id = *target,
                    TaskTerminatorKind::TrySpawn {
                        scope,
                        worker,
                        arguments,
                        handle_slot,
                        success,
                        spawn_error,
                    } => {
                        let moved_slots = arguments
                            .iter()
                            .filter_map(|arg| {
                                if let ScalarOperand::Move(slot) = arg {
                                    Some(*slot)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        let arguments = arguments
                            .iter()
                            .map(|value| scalar(&slots, value))
                            .collect::<Result<Vec<_>, _>>()?;
                        let moved_resources = arguments
                            .iter()
                            .filter_map(|value| {
                                if let ScalarValue::OwnedBuffer(id) = value {
                                    Some(*id)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        let reservation = self.next_reservation;
                        self.next_reservation += 1;
                        let actual_scope = *scopes
                            .get(scope)
                            .ok_or("reference spawn scope unavailable")?;
                        self.ledger
                            .reserve(task, actual_scope, reservation, &moved_resources)
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
                            for slot in moved_slots {
                                slots.insert(slot, Value::Consumed);
                            }
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
                            let returned = if let TaskOutcome::OwnedBuffer(resource) = outcome {
                                vec![resource]
                            } else {
                                vec![]
                            };
                            self.ledger
                                .complete(child, outcome.clone(), &returned)
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
                            TaskOutcome::OwnedBuffer(id) => {
                                slots.insert(
                                    *value_slot,
                                    Value::Scalar(ScalarValue::OwnedBuffer(id)),
                                );
                                block_id = *success;
                            }
                            TaskOutcome::Error(tag) => {
                                slots.insert(*error_slot, Value::Error(tag));
                                block_id = *error;
                            }
                        }
                    }
                    TaskTerminatorKind::Return { value } => {
                        return Ok(Some(take(&mut slots, value)?.outcome()));
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
        allocation_decision: 0,
        next_buffer: 1,
        buffers: BTreeMap::new(),
        buffer_events: BTreeMap::new(),
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
    if runtime.allocation_decision < script.allocation_outcomes.len() {
        return Err(problem(
            runtime.module.entry,
            TaskSpan::default(),
            "unused allocation outcomes",
        ));
    }
    if !runtime.buffers.is_empty() || !runtime.buffer_events.is_empty() {
        return Err(problem(
            runtime.module.entry,
            TaskSpan::default(),
            "reference exit retains buffer custody or unjoined observations",
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
            TaskEvent::BufferAllocated {
                task,
                buffer,
                initial,
            } => writeln!(output, "QCORE_BUFFER_ALLOC {task} {buffer} {initial}").unwrap(),
            TaskEvent::BufferAllocationFailed { task } => {
                writeln!(output, "QCORE_BUFFER_ALLOC_FAILED {task}").unwrap()
            }
            TaskEvent::BufferRead {
                task,
                buffer,
                value,
            } => writeln!(output, "QCORE_BUFFER_READ {task} {buffer} {value}").unwrap(),
            TaskEvent::BufferDropped { task, buffer } => {
                writeln!(output, "QCORE_BUFFER_DROP {task} {buffer}").unwrap()
            }
            TaskEvent::Spawn { task, worker } => {
                writeln!(output, "QCORE_TASK_SPAWN {task} {worker}").unwrap()
            }
            TaskEvent::CreateFailed { worker } => {
                writeln!(output, "QCORE_TASK_CREATE_FAILED {worker}").unwrap()
            }
            TaskEvent::Join { task } => writeln!(output, "QCORE_TASK_JOIN {task}").unwrap(),
            TaskEvent::Result { task, outcome } => match outcome {
                TaskOutcome::OwnedBuffer(buffer) => {
                    writeln!(output, "QCORE_TASK_RESULT {task} BUFFER {buffer}").unwrap()
                }
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
