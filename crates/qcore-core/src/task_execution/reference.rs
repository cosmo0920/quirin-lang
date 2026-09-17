use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarValue {
    Unit,
    I32(i32),
    OwnedBuffer(ResourceId),
    /// A borrowed identity. The reference run refuses to treat it as an
    /// outcome, independently of the checker's type rule.
    ReadLoan(ResourceId),
    /// A write-leased identity (card `WRITE-LEND-TASKS`). Refused as an
    /// outcome exactly as a loan is, independently of the checker's rule.
    /// It carries its own `low` since card `WRITE-LEND-TASKS-EXERCISE`: a
    /// lease-relative byte index means nothing without the base it counts
    /// from, and the base is the one fact the holder must not be able to
    /// choose.
    WriteLease { resource: ResourceId, low: u64 },
}
impl ScalarValue {
    fn outcome(self) -> Result<TaskOutcome, String> {
        match self {
            Self::Unit => Ok(TaskOutcome::Unit),
            Self::I32(value) => Ok(TaskOutcome::I32(value)),
            Self::OwnedBuffer(id) => Ok(TaskOutcome::OwnedBuffer(id)),
            Self::ReadLoan(_) => Err("a read loan is not a reference outcome".into()),
            Self::WriteLease { .. } => {
                Err("a write lease is not a reference outcome".into())
            }
        }
    }
}
/// Missing decisions select success; unused supplied decisions reject. These
/// choices drive one reference run and never change the checked source module.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskExecutionScript {
    pub spawn_outcomes: Vec<bool>,
    /// Root-task Buffer allocation decisions; worker decisions are separate.
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
    BufferWritten {
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
    /// A read loan opened to a child that was actually created.
    BufferLoaned {
        borrower: u64,
        buffer: ResourceId,
    },
    /// A read loan closed by a successful creator join.
    BufferReturned {
        borrower: u64,
        buffer: ResourceId,
    },
    /// A range write lease opened to a child that was actually created (card
    /// `WRITE-LEND-TASKS`): write authority over the payload bytes
    /// `[low, high)`, held by `borrower` until its creator's join.
    BufferLeased {
        borrower: u64,
        buffer: ResourceId,
        low: u64,
        high: u64,
    },
    /// A range write lease closed by a successful creator join.
    BufferLeaseReturned {
        borrower: u64,
        buffer: ResourceId,
    },
    /// One byte of a payload written **through a lease** (card
    /// `WRITE-LEND-TASKS-EXERCISE`). `index` is the owner-side byte, already
    /// rebased through the lease's `low`, so the event names the memory the
    /// emitted C writes rather than the holder's private coordinate.
    BufferByteWritten {
        task: u64,
        buffer: ResourceId,
        index: u64,
        byte: u8,
    },
    /// One byte of a payload read, through a lease or through the owner.
    BufferByteRead {
        task: u64,
        buffer: ResourceId,
        index: u64,
        byte: u8,
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
    Error {
        tag: String,
        payload: Option<ResourceId>,
    },
    Handle {
        task: u64,
        handle: u64,
    },
    Consumed,
}
// Each finite leaf invocation owns its allocation counter. The task namespace
// avoids a shared native counter and makes traces independent of worker order.
pub(super) fn buffer_identity(task: u64, ordinal: u64) -> Result<ResourceId, String> {
    if task > u64::from(u32::MAX) || ordinal == 0 || ordinal > u64::from(u32::MAX) {
        return Err("task Buffer identity range exhausted".into());
    }
    Ok(task * 4_294_967_296 + ordinal)
}

struct Runtime<'a> {
    module: &'a TaskModule,
    script: &'a TaskExecutionScript,
    decision: usize,
    worker_allocations: &'a BTreeMap<u64, Vec<bool>>,
    allocation_decisions: BTreeMap<u64, usize>,
    buffers: BTreeMap<ResourceId, i32>,
    buffer_events: BTreeMap<u64, Vec<TaskEvent>>,
    next_task: u64,
    next_reservation: u64,
    next_scope: u64,
    ledger: ScopedTaskLedger,
    observations: Vec<TaskObservation>,
    events: Vec<TaskEvent>,
    /// The `source.std-io.v1` events placed in this module's CFGs by card
    /// `COMPOSE-CONCURRENCY` — in the entry's, and since card `OBS-REORDERING`
    /// in a spawned worker's too. Empty on every task-route run, which is what
    /// makes that route's observation byte-identical: the whole composed
    /// apparatus below is guarded by `is_empty()`.
    io_sites: &'a [crate::composed_tasks::TaskIoSite],
    io_lines: Vec<crate::composed_tasks::ComposedLine>,
    /// `(event index, child)` at each point the parent's collect replay begins
    /// (card `OBS-REORDERING`).
    collects: Vec<(usize, u64)>,
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
        // Lending neither consumes nor copies the binding; it produces a
        // borrowed identity for exactly one creation. The share travels with
        // the reservation, not with the value: a borrower cannot read its own
        // share out of the value it was handed.
        ScalarOperand::Loan { slot, .. } => match slots.get(slot) {
            Some(Value::Scalar(ScalarValue::OwnedBuffer(resource))) => {
                Ok(ScalarValue::ReadLoan(*resource))
            }
            _ => Err("reference lender slot is unavailable".into()),
        },
        // Leasing, like lending, neither consumes nor copies the binding; the
        // range travels with the reservation, not with the value.
        ScalarOperand::Lease { slot, low, .. } => match slots.get(slot) {
            Some(Value::Scalar(ScalarValue::OwnedBuffer(resource))) => {
                Ok(ScalarValue::WriteLease {
                    resource: *resource,
                    low: *low,
                })
            }
            _ => Err("reference lessor slot is unavailable".into()),
        },
    }
}

/// The value a byte access names, read straight out of the slot. `scalar`'s
/// `Move` arm deliberately admits only an owned buffer, so it cannot be reused
/// here: a lease is neither moved nor copied by being exercised.
fn authority_value(
    slots: &BTreeMap<SlotId, Value>,
    slot: SlotId,
) -> Result<ScalarValue, String> {
    match slots.get(&slot) {
        Some(Value::Scalar(
            value @ (ScalarValue::OwnedBuffer(_)
            | ScalarValue::ReadLoan(_)
            | ScalarValue::WriteLease { .. }),
        )) => Ok(value.clone()),
        _ => Err("reference byte-access slot is unavailable".into()),
    }
}

/// Rebase a lease-relative index onto the payload and re-decide its extent,
/// independently of the checker. `base` is the lease's `low`, or 0 for an
/// owner. A non-constant index is refused here as it is in the checker: the
/// reference run may not resolve what the checker could not.
fn byte_offset(
    slots: &BTreeMap<SlotId, Value>,
    index: &ScalarOperand,
    base: u64,
) -> Result<u64, String> {
    let ScalarOperand::I32(value) = index else {
        // Touch the slot so an unavailable one reports as unavailable rather
        // than as a non-constant index.
        scalar(slots, index)?;
        return Err("reference byte index is not a check-time constant".into());
    };
    if *value < 0 {
        return Err("reference byte index is negative".into());
    }
    let offset = base + *value as u64;
    if offset >= BUFFER_PAYLOAD_BYTES {
        return Err(format!(
            "reference byte {offset} is outside the payload's \
             [0, {BUFFER_PAYLOAD_BYTES}) extent"
        ));
    }
    Ok(offset)
}

/// The identity a reference read is performed against, owned or borrowed.
fn readable(slots: &BTreeMap<SlotId, Value>, slot: SlotId) -> Result<ResourceId, String> {
    match slots.get(&slot) {
        Some(Value::Scalar(
            ScalarValue::OwnedBuffer(resource) | ScalarValue::ReadLoan(resource),
        )) => Ok(*resource),
        _ => Err("reference buffer read requires an owner or borrower".into()),
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
    /// Write the source lines standing immediately before position `index` of
    /// `block` of `function`, if any.
    ///
    /// Guarded by the model's own statements rather than by convenience: sites
    /// exist only on the composed route, and a site is performed only by the
    /// function that carries it. Since card `OBS-REORDERING` that function may
    /// be a spawned worker, in which case the writer is that worker's own task
    /// — which is exactly the emitter recorded here, and exactly what makes the
    /// declared reordering relation derivable.
    fn source_lines(
        &mut self,
        function: FunctionId,
        task: u64,
        block: BlockId,
        index: usize,
        slots: &BTreeMap<SlotId, Value>,
    ) -> Result<(), String> {
        if self.io_sites.is_empty() {
            return Ok(());
        }
        let emitter = if task == 0 {
            crate::obs_reordering::Emitter::Parent
        } else {
            crate::obs_reordering::Emitter::Worker(task)
        };
        let sites = self.io_sites;
        for site in sites
            .iter()
            .filter(|site| site.function == function && site.block == block && site.index == index)
        {
            let value = match &site.event {
                crate::std_io::StdIoEvent::LogLevel { .. }
                | crate::std_io::StdIoEvent::Flush
                | crate::std_io::StdIoEvent::Return => continue,
                crate::std_io::StdIoEvent::Print { operand, .. }
                | crate::std_io::StdIoEvent::Log { operand, .. } => match operand {
                    crate::std_io::StdIoOperand::Literal(value) => *value,
                    crate::std_io::StdIoOperand::Scalar(name) => {
                        let slot = site
                            .operand
                            .ok_or_else(|| format!("`{name}` resolves to no task slot"))?;
                        match slots.get(&slot) {
                            Some(Value::Scalar(ScalarValue::I32(value))) => *value,
                            _ => {
                                return Err(format!(
                                    "reference rendering of `{name}` requires a readable i32"
                                ));
                            }
                        }
                    }
                    crate::std_io::StdIoOperand::Borrowed(name) => {
                        return Err(format!(
                            "`borrow {name}` is not rendered by `source.std-io-tasks.v1`"
                        ));
                    }
                },
            };
            if let Some(line) = crate::std_io::rendered_line(&site.event, value) {
                let position = self.events.len();
                self.io_lines.push(crate::composed_tasks::ComposedLine {
                    position,
                    emitter,
                    line,
                });
            }
        }
        Ok(())
    }

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
        let mut next_buffer = 1u64;
        loop {
            let block = &function.blocks[&block_id];
            for (index, instruction) in block.instructions.iter().enumerate() {
                self.source_lines(id, task, block_id, index, &slots)
                    .map_err(|message| problem(id, instruction.span, message))?;
                let result: Result<(), String> = (|| {
                    match &instruction.kind {
                        TaskInstructionKind::ReadBuffer { buffer, value_slot } => {
                            let resource = readable(&slots, *buffer)?;
                            self.ledger
                                .read_resource(task, resource)
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
                        TaskInstructionKind::WriteBuffer { buffer, value } => {
                            let ScalarValue::I32(written) = scalar(&slots, value)? else {
                                return Err("reference buffer write requires i32".into());
                            };
                            // Independently of the checker: a write demands
                            // exclusive authority, which the ledger refuses
                            // while any read share is outstanding.
                            let ScalarValue::OwnedBuffer(resource) =
                                scalar(&slots, &ScalarOperand::Move(*buffer))?
                            else {
                                return Err("reference buffer write requires owner".into());
                            };
                            self.ledger
                                .use_resource(task, resource)
                                .map_err(|e| format!("{e:?}"))?;
                            *self
                                .buffers
                                .get_mut(&resource)
                                .ok_or("reference buffer payload missing")? = written;
                            self.buffer_event(
                                task,
                                TaskEvent::BufferWritten {
                                    task,
                                    buffer: resource,
                                    value: written,
                                },
                            );
                        }
                        TaskInstructionKind::WriteBufferByte {
                            authority,
                            index,
                            value,
                        } => {
                            let ScalarValue::I32(written) = scalar(&slots, value)? else {
                                return Err("reference byte write requires i32".into());
                            };
                            let ScalarValue::WriteLease { resource, low } =
                                authority_value(&slots, *authority)?
                            else {
                                return Err("reference byte write requires a write lease".into());
                            };
                            let offset = byte_offset(&slots, index, low)?;
                            let payload = self
                                .buffers
                                .get_mut(&resource)
                                .ok_or("reference buffer payload missing")?;
                            // The payload's own object representation, which
                            // is what the emitted C indexes. Reference and C
                            // therefore agree by construction on any host;
                            // a program whose *observation* depends on that
                            // representation is the recorded boundary
                            // `WRITE-LEND-EXERCISE-BYTE-ORDER`.
                            let mut bytes = payload.to_ne_bytes();
                            let byte = (written & 0xff) as u8;
                            bytes[offset as usize] = byte;
                            *payload = i32::from_ne_bytes(bytes);
                            self.buffer_event(
                                task,
                                TaskEvent::BufferByteWritten {
                                    task,
                                    buffer: resource,
                                    index: offset,
                                    byte,
                                },
                            );
                        }
                        TaskInstructionKind::ReadBufferByte {
                            authority,
                            index,
                            value_slot,
                        } => {
                            let (resource, offset) =
                                match authority_value(&slots, *authority)? {
                                    ScalarValue::WriteLease { resource, low } => {
                                        (resource, byte_offset(&slots, index, low)?)
                                    }
                                    ScalarValue::OwnedBuffer(resource) => {
                                        // Independently of the checker: the
                                        // owner's byte read takes the same
                                        // ledger gate its whole-payload read
                                        // takes, so a lease outstanding
                                        // refuses it here too.
                                        self.ledger
                                            .read_resource(task, resource)
                                            .map_err(|e| format!("{e:?}"))?;
                                        (resource, byte_offset(&slots, index, 0)?)
                                    }
                                    _ => {
                                        return Err(
                                            "reference byte read requires an owner or a write \
                                             lease"
                                                .into(),
                                        );
                                    }
                                };
                            let payload = *self
                                .buffers
                                .get(&resource)
                                .ok_or("reference buffer payload missing")?;
                            let byte = payload.to_ne_bytes()[offset as usize];
                            self.buffer_event(
                                task,
                                TaskEvent::BufferByteRead {
                                    task,
                                    buffer: resource,
                                    index: offset,
                                    byte,
                                },
                            );
                            slots.insert(
                                *value_slot,
                                Value::Scalar(ScalarValue::I32(i32::from(byte))),
                            );
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
                                TaskOutcome::Error(_) | TaskOutcome::ErrorWithBuffer { .. } => {
                                    return Err(
                                        "reference error contradicts checked infallible Result"
                                            .into(),
                                    );
                                }
                            };
                            slots.insert(*result_slot, Value::Consumed);
                            slots.insert(*value_slot, Value::Scalar(value));
                        }
                        TaskInstructionKind::TakeErrorPayload {
                            error_slot,
                            buffer_slot,
                        } => {
                            let Some(Value::Error {
                                tag,
                                payload: Some(resource),
                            }) = slots.get(error_slot).cloned()
                            else {
                                return Err(
                                    "reference typed error carries no payload to take".into()
                                );
                            };
                            // The reference run checks the ledger itself rather
                            // than trusting the checked module: only the task
                            // that actually holds the identity may take it.
                            self.ledger
                                .use_resource(task, resource)
                                .map_err(|e| format!("{e:?}"))?;
                            slots.insert(*error_slot, Value::Error { tag, payload: None });
                            slots.insert(
                                *buffer_slot,
                                Value::Scalar(ScalarValue::OwnedBuffer(resource)),
                            );
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
                            // Read the shares this borrower holds before the
                            // join returns them, so the returned identities are
                            // the ledger's, not the caller's claim.
                            let returned_loans = self
                                .ledger
                                .outstanding_loans()
                                .into_iter()
                                .filter(|&buffer| {
                                    self.ledger.loan_borrowers(buffer).contains(&child)
                                })
                                .collect::<Vec<_>>();
                            let returned_leases = self
                                .ledger
                                .outstanding_write_leases()
                                .into_iter()
                                .filter(|&buffer| {
                                    self.ledger.lease_holders(buffer).contains(&child)
                                })
                                .collect::<Vec<_>>();
                            let completion = self
                                .ledger
                                .join(task, actual)
                                .map_err(|e| format!("{e:?}"))?;
                            slots.insert(*handle, Value::Consumed);
                            // Card `OBS-REORDERING`: the point at which
                            // `thrd_join` has returned and the child is no
                            // longer a live writer. Recorded here, by the model
                            // that performs the join.
                            self.collects.push((self.events.len(), child));
                            self.events
                                .extend(self.buffer_events.remove(&child).unwrap_or_default());
                            self.events.push(TaskEvent::Result {
                                task: child,
                                outcome: completion.outcome.clone(),
                            });
                            self.events.push(TaskEvent::Join { task: child });
                            for buffer in returned_loans {
                                self.events.push(TaskEvent::BufferReturned {
                                    borrower: child,
                                    buffer,
                                });
                            }
                            for buffer in returned_leases {
                                self.events.push(TaskEvent::BufferLeaseReturned {
                                    borrower: child,
                                    buffer,
                                });
                            }
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
            self.source_lines(id, task, block_id, block.instructions.len(), &slots)
                .map_err(|message| problem(id, block.terminator.span, message))?;
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
                        let choices = if task == 0 {
                            self.script.allocation_outcomes.as_slice()
                        } else {
                            self.worker_allocations
                                .get(&task)
                                .map(Vec::as_slice)
                                .unwrap_or(&[])
                        };
                        let decision = self.allocation_decisions.entry(task).or_default();
                        let succeeds = choices.get(*decision).copied().unwrap_or(true);
                        *decision += 1;
                        if succeeds {
                            let resource = buffer_identity(task, next_buffer)?;
                            next_buffer += 1;
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
                            .filter_map(|arg: &ScalarOperand| {
                                if let ScalarOperand::Move(slot) = arg {
                                    Some(*slot)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        let arguments_syntax = arguments;
                        let arguments = arguments_syntax
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
                        let shares = arguments_syntax
                            .iter()
                            .filter_map(|argument| match argument {
                                ScalarOperand::Loan { share, .. } => Some(*share),
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        let lent_resources = arguments
                            .iter()
                            .filter_map(|value| {
                                if let ScalarValue::ReadLoan(id) = value {
                                    Some(*id)
                                } else {
                                    None
                                }
                            })
                            .zip(shares)
                            .collect::<Vec<_>>();
                        let ranges = arguments_syntax
                            .iter()
                            .filter_map(|argument| match argument {
                                ScalarOperand::Lease { low, high, .. } => Some(LeaseRange {
                                    low: *low,
                                    high: *high,
                                }),
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        let leased_resources = arguments
                            .iter()
                            .filter_map(|value| {
                                if let ScalarValue::WriteLease { resource, .. } = value {
                                    Some(*resource)
                                } else {
                                    None
                                }
                            })
                            .zip(ranges)
                            .collect::<Vec<_>>();
                        let reservation = self.next_reservation;
                        self.next_reservation += 1;
                        let actual_scope = *scopes
                            .get(scope)
                            .ok_or("reference spawn scope unavailable")?;
                        self.ledger
                            .reserve_with_transfers(
                                task,
                                actual_scope,
                                reservation,
                                &moved_resources,
                                &lent_resources,
                                &leased_resources,
                            )
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
                            for &(buffer, _) in &lent_resources {
                                self.events.push(TaskEvent::BufferLoaned {
                                    borrower: child,
                                    buffer,
                                });
                            }
                            for &(buffer, range) in &leased_resources {
                                self.events.push(TaskEvent::BufferLeased {
                                    borrower: child,
                                    buffer,
                                    low: range.low,
                                    high: range.high,
                                });
                            }
                            // Leaf workers have no shared state. Eager reference
                            // evaluation selects a legal schedule, not a claim
                            // that the native worker completed before spawn returned.
                            let outcome = self
                                .execute(*worker, child, arguments.clone())
                                .map_err(|e| e.to_string())?;
                            let returned = match outcome {
                                TaskOutcome::OwnedBuffer(resource)
                                | TaskOutcome::ErrorWithBuffer {
                                    buffer: resource, ..
                                } => vec![resource],
                                _ => vec![],
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
                                slots.insert(*error_slot, Value::Error { tag, payload: None });
                                block_id = *error;
                            }
                            TaskOutcome::ErrorWithBuffer { tag, buffer } => {
                                slots.insert(
                                    *error_slot,
                                    Value::Error {
                                        tag,
                                        payload: Some(buffer),
                                    },
                                );
                                block_id = *error;
                            }
                        }
                    }
                    TaskTerminatorKind::Return { value } => {
                        return Ok(Some(take(&mut slots, value)?.outcome()?));
                    }
                    TaskTerminatorKind::Raise { error, payload } => {
                        let tag = match error {
                            ErrorOperand::Tag(tag) => tag.clone(),
                            ErrorOperand::Slot(slot) => match slots.get(slot) {
                                Some(Value::Error { tag, payload: None }) => tag.clone(),
                                Some(Value::Error {
                                    payload: Some(_), ..
                                }) => {
                                    return Err(
                                        "reference rethrow of an undelivered payload is refused"
                                            .into(),
                                    );
                                }
                                _ => return Err("reference error slot unavailable".into()),
                            },
                        };
                        return Ok(Some(match payload {
                            None => TaskOutcome::Error(tag),
                            Some(operand) => {
                                let ScalarValue::OwnedBuffer(buffer) = take(&mut slots, operand)?
                                else {
                                    return Err(
                                        "reference error payload requires an owned Buffer".into()
                                    );
                                };
                                TaskOutcome::ErrorWithBuffer { tag, buffer }
                            }
                        }));
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
    execute_task_module_with_worker_allocations(checked, script, &BTreeMap::new())
}

/// Worker Buffer allocation decisions keyed by successful runtime task identity.
/// Missing decisions select success. Root decisions stay in the original script;
/// zero keys, absent workers and unused supplied worker decisions reject. This
/// prevents native scheduling from deciding which worker receives an injected
/// allocation failure. It is an execution script, not checked program evidence.
pub fn execute_task_module_with_worker_allocations(
    checked: &CheckedTaskModule,
    script: &TaskExecutionScript,
    worker_allocations: &BTreeMap<u64, Vec<bool>>,
) -> Result<TaskExecutionObservation, TaskDiagnostic> {
    execute_with_source_lines(checked, script, worker_allocations, &[])
        .map(|composed| composed.tasks)
}

/// One reference run of a composed program (card `COMPOSE-CONCURRENCY`).
///
/// The **empty** site list is the task route's own entry point above, so a
/// module with no source lines travels this identical code path and produces a
/// byte-identical [`TaskExecutionObservation`]: there is no second reference
/// interpreter to drift.
pub fn execute_with_source_lines(
    checked: &CheckedTaskModule,
    script: &TaskExecutionScript,
    worker_allocations: &BTreeMap<u64, Vec<bool>>,
    io_sites: &[crate::composed_tasks::TaskIoSite],
) -> Result<crate::composed_tasks::ComposedTaskObservation, TaskDiagnostic> {
    if worker_allocations.contains_key(&0) {
        return Err(problem(
            checked.module().entry,
            TaskSpan::default(),
            "root allocation decisions belong in TaskExecutionScript",
        ));
    }
    let mut runtime = Runtime {
        module: checked.module(),
        script,
        decision: 0,
        worker_allocations,
        allocation_decisions: BTreeMap::new(),
        buffers: BTreeMap::new(),
        buffer_events: BTreeMap::new(),
        next_task: 1,
        next_reservation: 1,
        next_scope: 1,
        ledger: ScopedTaskLedger::default(),
        observations: vec![],
        events: vec![],
        io_sites,
        io_lines: vec![],
        collects: vec![],
    };
    let root_outcome = runtime.execute(runtime.module.entry, 0, vec![])?;
    if runtime.decision < script.spawn_outcomes.len() {
        return Err(problem(
            runtime.module.entry,
            TaskSpan::default(),
            "reference script contains unused creation outcomes",
        ));
    }
    if runtime.allocation_decisions.get(&0).copied().unwrap_or(0) < script.allocation_outcomes.len()
    {
        return Err(problem(
            runtime.module.entry,
            TaskSpan::default(),
            "unused allocation outcomes",
        ));
    }
    for (&task, outcomes) in worker_allocations {
        if !runtime
            .observations
            .iter()
            .any(|observation| observation.task == task)
            || runtime
                .allocation_decisions
                .get(&task)
                .copied()
                .unwrap_or(0)
                < outcomes.len()
        {
            return Err(problem(
                runtime.module.entry,
                TaskSpan::default(),
                "absent worker or unused worker allocation outcomes",
            ));
        }
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
    Ok(crate::composed_tasks::ComposedTaskObservation {
        tasks: TaskExecutionObservation {
            root_outcome,
            tasks: runtime.observations,
            events: runtime.events,
        },
        lines: runtime.io_lines,
        collects: runtime.collects,
    })
}

pub fn render_task_observation(observation: &TaskExecutionObservation) -> String {
    let mut output = String::new();
    for event in &observation.events {
        output.push_str(&render_task_event(event));
    }
    output
}

/// The bytes of one trace event, newline included.
///
/// Factored out of [`render_task_observation`] byte for byte so the composed
/// task route (card `COMPOSE-CONCURRENCY`) can interleave source lines with
/// these without a second rendering existing. `render_task_observation` is the
/// concatenation of this over the observation's events, which is what makes
/// "the composed program's standard output with its source lines removed **is**
/// the task rendering" a fact rather than an agreement between two renderers.
pub fn render_task_event(event: &TaskEvent) -> String {
    let mut output = String::new();
    use std::fmt::Write;
    {
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
            TaskEvent::BufferWritten {
                task,
                buffer,
                value,
            } => writeln!(output, "QCORE_BUFFER_WRITE {task} {buffer} {value}").unwrap(),
            TaskEvent::BufferDropped { task, buffer } => {
                writeln!(output, "QCORE_BUFFER_DROP {task} {buffer}").unwrap()
            }
            TaskEvent::Spawn { task, worker } => {
                writeln!(output, "QCORE_TASK_SPAWN {task} {worker}").unwrap()
            }
            TaskEvent::CreateFailed { worker } => {
                writeln!(output, "QCORE_TASK_CREATE_FAILED {worker}").unwrap()
            }
            TaskEvent::BufferLoaned { borrower, buffer } => {
                writeln!(output, "QCORE_BUFFER_LOAN {borrower} {buffer}").unwrap()
            }
            TaskEvent::BufferReturned { borrower, buffer } => {
                writeln!(output, "QCORE_BUFFER_RETURN {borrower} {buffer}").unwrap()
            }
            TaskEvent::BufferLeased {
                borrower,
                buffer,
                low,
                high,
            } => writeln!(
                output,
                "QCORE_BUFFER_LEASE {borrower} {buffer} {low} {high}"
            )
            .unwrap(),
            TaskEvent::BufferLeaseReturned { borrower, buffer } => {
                writeln!(output, "QCORE_BUFFER_LEASE_RETURN {borrower} {buffer}").unwrap()
            }
            TaskEvent::BufferByteWritten {
                task,
                buffer,
                index,
                byte,
            } => writeln!(
                output,
                "QCORE_BUFFER_BYTE_WRITE {task} {buffer} {index} {byte}"
            )
            .unwrap(),
            TaskEvent::BufferByteRead {
                task,
                buffer,
                index,
                byte,
            } => writeln!(
                output,
                "QCORE_BUFFER_BYTE_READ {task} {buffer} {index} {byte}"
            )
            .unwrap(),
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
                TaskOutcome::ErrorWithBuffer { tag, buffer } => writeln!(
                    output,
                    "QCORE_TASK_RESULT {task} ERROR_BUFFER {tag} {buffer}"
                )
                .unwrap(),
            },
        }
    }
    output
}
