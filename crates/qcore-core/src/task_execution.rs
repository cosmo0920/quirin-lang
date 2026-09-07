//! Finite source-task execution: immutable i32/unit copies, affine owned buffers
//! and leaf workers. Model checking covers allocation/creation branches and
//! checked worker custody effects. A reference run chooses concrete outcomes;
//! it is not the all-path certificate. Source/native refinement, arbitrary owned
//! graphs, loans and synchronization remain separate boundaries.
pub use crate::scoped_tasks::ScopeId;
use crate::scoped_tasks::{ResourceId, ResourceState, ScopedTaskLedger, TaskOutcome};
use std::collections::{BTreeMap, BTreeSet};
pub type FunctionId = usize;
pub type SlotId = usize;
pub type BlockId = usize;
pub const SPAWN_ERROR: &str = "SpawnError";
pub const ALLOC_ERROR: &str = "AllocError";
pub type TaskValueType = ScalarType;
pub type TaskOperand = ScalarOperand;
const MAX_CHECK_STATES: usize = 16_384;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskSpan {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarType {
    Unit,
    I32,
    OwnedBuffer,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarOperand {
    Unit,
    I32(i32),
    Slot(SlotId),
    Move(SlotId),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorOperand {
    Tag(String),
    Slot(SlotId),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskEffects {
    pub alloc: bool,
    pub spawn: bool,
    pub join: bool,
    pub may_block: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskModule {
    pub entry: FunctionId,
    pub functions: BTreeMap<FunctionId, TaskFunction>,
    pub source_len: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskFunction {
    pub name: String,
    pub parameters: Vec<(SlotId, ScalarType)>,
    pub result: ScalarType,
    pub errors: BTreeSet<String>,
    pub declared_effects: TaskEffects,
    pub entry: BlockId,
    pub blocks: BTreeMap<BlockId, TaskBlock>,
    pub span: TaskSpan,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskBlock {
    pub instructions: Vec<TaskInstruction>,
    pub terminator: TaskTerminator,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskInstruction {
    pub span: TaskSpan,
    pub kind: TaskInstructionKind,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskInstructionKind {
    ReadBuffer {
        buffer: SlotId,
        value_slot: SlotId,
    },
    DropBuffer {
        buffer: SlotId,
    },
    BindScalar {
        slot: SlotId,
        value: ScalarOperand,
    },
    /// Eliminate a Result with an empty declared error row. It is still consumed
    /// exactly once; no failure edge or implicit sibling join is introduced.
    UnwrapInfallible {
        result_slot: SlotId,
        value_slot: SlotId,
    },
    EnterScope {
        scope: ScopeId,
    },
    ExitScope {
        scope: ScopeId,
    },
    Join {
        handle: SlotId,
        outcome_slot: Option<SlotId>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskTerminator {
    pub span: TaskSpan,
    pub kind: TaskTerminatorKind,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskTerminatorKind {
    TryAllocateBuffer {
        initial: ScalarOperand,
        value_slot: SlotId,
        success: BlockId,
        allocation_error: BlockId,
    },
    Goto {
        target: BlockId,
    },
    TrySpawn {
        scope: ScopeId,
        worker: FunctionId,
        arguments: Vec<ScalarOperand>,
        handle_slot: SlotId,
        success: BlockId,
        spawn_error: BlockId,
    },
    TryOutcome {
        result_slot: SlotId,
        value_slot: SlotId,
        error_slot: SlotId,
        success: BlockId,
        error: BlockId,
    },
    Return {
        value: ScalarOperand,
    },
    Raise {
        error: ErrorOperand,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskDiagnostic {
    pub function: Option<FunctionId>,
    pub span: TaskSpan,
    pub message: String,
}
impl std::fmt::Display for TaskDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}
impl std::error::Error for TaskDiagnostic {}

/// Constructible only through all-definition, all-path checking.
///
/// ```compile_fail
/// use qcore_core::task_execution::{CheckedTaskModule, TaskModule};
/// fn forge(module: TaskModule) -> CheckedTaskModule { CheckedTaskModule { module } }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedTaskModule {
    module: TaskModule,
}
impl CheckedTaskModule {
    pub fn module(&self) -> &TaskModule {
        &self.module
    }
}

fn problem(function: FunctionId, span: TaskSpan, message: impl Into<String>) -> TaskDiagnostic {
    TaskDiagnostic {
        function: Some(function),
        span,
        message: message.into(),
    }
}
fn error_name(name: &str) -> bool {
    name.split("::").all(|part| {
        let mut bytes = part.bytes();
        bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
    })
}
fn targets(term: &TaskTerminatorKind) -> Vec<BlockId> {
    match term {
        TaskTerminatorKind::Goto { target } => vec![*target],
        TaskTerminatorKind::TrySpawn {
            success,
            spawn_error,
            ..
        } => vec![*success, *spawn_error],
        TaskTerminatorKind::TryOutcome { success, error, .. } => vec![*success, *error],
        TaskTerminatorKind::TryAllocateBuffer {
            success,
            allocation_error,
            ..
        } => vec![*success, *allocation_error],
        _ => vec![],
    }
}

#[derive(Clone, Debug)]
enum AbstractValue {
    Scalar(ScalarType),
    Owned(ResourceId),
}
impl AbstractValue {
    fn ty(&self) -> ScalarType {
        match self {
            Self::Scalar(ty) => *ty,
            Self::Owned(_) => ScalarType::OwnedBuffer,
        }
    }
    fn resource(&self) -> Option<ResourceId> {
        match self {
            Self::Owned(id) => Some(*id),
            _ => None,
        }
    }
}
#[derive(Clone)]
enum AbstractSlot {
    Value(AbstractValue),
    Result {
        value: Option<AbstractValue>,
        errors: BTreeSet<String>,
        declared_errors: BTreeSet<String>,
    },
    Error(BTreeSet<String>),
    Handle {
        worker: FunctionId,
        child: u64,
        handle: u64,
        returned: Option<ResourceId>,
        errors: BTreeSet<String>,
        normal: bool,
    },
    Consumed,
}
/// Each alternative is derived from a checked worker exit, using original
/// parameter positions or a proved fresh local result, never caller assertions.
#[derive(Clone, Debug)]
enum WorkerReturn {
    Input(usize),
    Fresh,
}
#[derive(Clone, Debug)]
struct WorkerExit {
    returned: Option<WorkerReturn>,
    dropped: Vec<usize>,
    error: Option<String>,
}
#[derive(Clone)]
struct State {
    slots: BTreeMap<SlotId, AbstractSlot>,
    ledger: ScopedTaskLedger,
    scopes: Vec<ScopeId>,
    next_id: u64,
    resources: BTreeSet<ResourceId>,
}
impl State {
    fn value(&self, operand: &ScalarOperand) -> Result<AbstractValue, String> {
        match operand {
            ScalarOperand::Unit => Ok(AbstractValue::Scalar(ScalarType::Unit)),
            ScalarOperand::I32(_) => Ok(AbstractValue::Scalar(ScalarType::I32)),
            ScalarOperand::Slot(id) => match self.slots.get(id) {
                Some(AbstractSlot::Value(value @ AbstractValue::Scalar(_))) => Ok(value.clone()),
                _ => Err(format!("slot {id} is not an initialized copy scalar")),
            },
            ScalarOperand::Move(id) => match self.slots.get(id) {
                Some(AbstractSlot::Value(value @ AbstractValue::Owned(resource))) => {
                    self.ledger
                        .use_resource(0, *resource)
                        .map_err(|e| format!("{e:?}"))?;
                    Ok(value.clone())
                }
                _ => Err(format!("slot {id} is not an initialized owned buffer")),
            },
        }
    }
    fn take(&mut self, operand: &ScalarOperand) -> Result<AbstractValue, String> {
        let value = self.value(operand)?;
        if let ScalarOperand::Move(slot) = operand {
            self.slots.insert(*slot, AbstractSlot::Consumed);
        }
        Ok(value)
    }
    fn buffer(&self, slot: SlotId) -> Result<ResourceId, String> {
        match self.value(&ScalarOperand::Move(slot))? {
            AbstractValue::Owned(id) => Ok(id),
            _ => unreachable!(),
        }
    }
    fn bind(&mut self, id: SlotId, value: AbstractSlot) -> Result<(), String> {
        if self.slots.contains_key(&id) {
            return Err(format!("slot {id} is written more than once"));
        }
        self.slots.insert(id, value);
        Ok(())
    }
    fn exit(&self, returned: Option<ResourceId>) -> Result<(), String> {
        if !self.scopes.is_empty() {
            return Err("return/error leaves an open thread scope".into());
        }
        if self.slots.values().any(|s| {
            matches!(
                s,
                AbstractSlot::Handle { .. }
                    | AbstractSlot::Result { .. }
                    | AbstractSlot::Value(AbstractValue::Owned(_))
            )
        }) {
            return Err("return/error discards a handle, unconsumed Result or owned buffer".into());
        }
        for &resource in &self.resources {
            if Some(resource) != returned
                && self.ledger.resource_state(resource) != Some(ResourceState::Dropped)
            {
                return Err("return/error leaves unaccounted resource custody".into());
            }
        }
        Ok(())
    }
}

fn check_function(
    module: &TaskModule,
    id: FunctionId,
    summaries: &BTreeMap<FunctionId, Vec<WorkerExit>>,
) -> Result<Vec<WorkerExit>, TaskDiagnostic> {
    let function = &module.functions[&id];
    let fail = |span, message| problem(id, span, message);
    let span_valid = |span: TaskSpan| {
        span.start <= span.end
            && span.end <= module.source_len
            && span.start >= function.span.start
            && span.end <= function.span.end
    };
    if function.name.is_empty()
        || !span_valid(function.span)
        || function.errors.iter().any(|e| !error_name(e))
    {
        return Err(fail(
            function.span,
            "invalid function name, source span or error label".into(),
        ));
    }
    if id == module.entry && !function.parameters.is_empty() {
        return Err(fail(
            function.span,
            "task entry must have no parameters".into(),
        ));
    }
    if id != module.entry && function.errors.contains(SPAWN_ERROR) {
        return Err(fail(
            function.span,
            "SpawnError is reserved for task creation, not leaf worker errors".into(),
        ));
    }
    // Iterative DAG validation covers even blocks not reached by a particular
    // spawn/result outcome. No cycle is interpreted as successful termination.
    let mut incoming = function
        .blocks
        .keys()
        .map(|&b| (b, 0usize))
        .collect::<BTreeMap<_, _>>();
    if !incoming.contains_key(&function.entry) {
        return Err(fail(function.span, "missing entry block".into()));
    }
    let mut definitions = BTreeSet::new();
    for &(slot, _) in &function.parameters {
        if !definitions.insert(slot) {
            return Err(fail(function.span, "duplicate parameter slot".into()));
        }
    }
    let mut effects = TaskEffects::default();
    for block in function.blocks.values() {
        if !span_valid(block.terminator.span) {
            return Err(fail(
                block.terminator.span,
                "terminator source span is outside its function".into(),
            ));
        }
        for target in targets(&block.terminator.kind) {
            let count = incoming
                .get_mut(&target)
                .ok_or_else(|| fail(block.terminator.span, "missing branch target".into()))?;
            *count += 1;
        }
        for instruction in &block.instructions {
            if !span_valid(instruction.span) {
                return Err(fail(
                    instruction.span,
                    "instruction source span is outside its function".into(),
                ));
            }
            let defined = match &instruction.kind {
                TaskInstructionKind::BindScalar { slot, .. } => Some(*slot),
                TaskInstructionKind::ReadBuffer { value_slot, .. } => Some(*value_slot),
                TaskInstructionKind::UnwrapInfallible { value_slot, .. } => Some(*value_slot),
                TaskInstructionKind::Join { outcome_slot, .. } => {
                    effects.join = true;
                    effects.may_block = true;
                    *outcome_slot
                }
                _ => None,
            };
            if defined.is_some_and(|slot| !definitions.insert(slot)) {
                return Err(fail(
                    instruction.span,
                    "slot identity has multiple definitions".into(),
                ));
            }
        }
        let defined = match &block.terminator.kind {
            TaskTerminatorKind::TryAllocateBuffer { value_slot, .. } => {
                effects.alloc = true;
                vec![*value_slot]
            }
            TaskTerminatorKind::TrySpawn { handle_slot, .. } => {
                effects.spawn = true;
                vec![*handle_slot]
            }
            TaskTerminatorKind::TryOutcome {
                value_slot,
                error_slot,
                ..
            } => vec![*value_slot, *error_slot],
            _ => vec![],
        };
        for slot in defined {
            if !definitions.insert(slot) {
                return Err(fail(
                    block.terminator.span,
                    "slot identity has multiple definitions".into(),
                ));
            }
        }
    }
    if (effects.alloc && !function.declared_effects.alloc)
        || (effects.spawn && !function.declared_effects.spawn)
        || (effects.join && !function.declared_effects.join)
        || (effects.may_block && !function.declared_effects.may_block)
    {
        return Err(fail(
            function.span,
            "declared task effects do not permit body effects".into(),
        ));
    }
    if id != module.entry && (effects.spawn || effects.join) {
        return Err(fail(
            function.span,
            "workers must be nonrecursive task-free leaves".into(),
        ));
    }
    let mut ready = incoming
        .iter()
        .filter_map(|(&b, &n)| (n == 0).then_some(b))
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(block) = ready.pop() {
        visited += 1;
        for target in targets(&function.blocks[&block].terminator.kind) {
            let count = incoming.get_mut(&target).unwrap();
            *count -= 1;
            if *count == 0 {
                ready.push(target);
            }
        }
    }
    if visited != function.blocks.len() {
        return Err(fail(function.span, "task CFG contains a cycle".into()));
    }
    if id == module.entry && function.result == ScalarType::OwnedBuffer {
        return Err(fail(
            function.span,
            "owned task entry results have no external custodian in this profile".into(),
        ));
    }
    let mut state = State {
        slots: BTreeMap::new(),
        ledger: ScopedTaskLedger::default(),
        scopes: vec![],
        next_id: 1,
        resources: BTreeSet::new(),
    };
    let mut inputs = BTreeMap::new();
    for (index, &(slot, ty)) in function.parameters.iter().enumerate() {
        let value = if ty == ScalarType::OwnedBuffer {
            let resource = state.next_id;
            state.next_id += 1;
            state
                .ledger
                .acquire(0, resource)
                .map_err(|e| fail(function.span, format!("{e:?}")))?;
            state.resources.insert(resource);
            inputs.insert(resource, index);
            AbstractValue::Owned(resource)
        } else {
            AbstractValue::Scalar(ty)
        };
        state.slots.insert(slot, AbstractSlot::Value(value));
    }
    let mut exits = vec![];
    let mut queue = vec![(function.entry, state)];
    let mut reached = BTreeSet::new();
    let mut scope_stacks = BTreeMap::<BlockId, Vec<ScopeId>>::new();
    let mut count = 0usize;
    while let Some((block_id, mut state)) = queue.pop() {
        count += 1;
        if count > MAX_CHECK_STATES {
            return Err(fail(
                function.span,
                "task path exploration bound exhausted".into(),
            ));
        }
        reached.insert(block_id);
        if scope_stacks
            .get(&block_id)
            .is_some_and(|s| s != &state.scopes)
        {
            return Err(fail(
                function.span,
                "scope stacks disagree at CFG merge".into(),
            ));
        }
        scope_stacks.insert(block_id, state.scopes.clone());
        let block = &function.blocks[&block_id];
        for instruction in &block.instructions {
            let result: Result<(), String> = (|| {
                match &instruction.kind {
                    TaskInstructionKind::BindScalar { slot, value } => {
                        let value = state.take(value)?;
                        state.bind(*slot, AbstractSlot::Value(value))?;
                    }
                    TaskInstructionKind::ReadBuffer { buffer, value_slot } => {
                        state.buffer(*buffer)?;
                        state.bind(
                            *value_slot,
                            AbstractSlot::Value(AbstractValue::Scalar(ScalarType::I32)),
                        )?;
                    }
                    TaskInstructionKind::DropBuffer { buffer } => {
                        let resource = state.buffer(*buffer)?;
                        state
                            .ledger
                            .drop_resource(0, resource)
                            .map_err(|e| format!("{e:?}"))?;
                        state.slots.insert(*buffer, AbstractSlot::Consumed);
                    }
                    TaskInstructionKind::UnwrapInfallible {
                        result_slot,
                        value_slot,
                    } => {
                        let Some(AbstractSlot::Result {
                            value,
                            declared_errors,
                            ..
                        }) = state.slots.get(result_slot).cloned()
                        else {
                            return Err(
                                "infallible extraction requires an initialized Result slot".into(),
                            );
                        };
                        if !declared_errors.is_empty() {
                            return Err(
                                "infallible extraction requires an empty declared error row".into(),
                            );
                        }
                        let value = value.ok_or("infallible Result has no normal outcome")?;
                        state.slots.insert(*result_slot, AbstractSlot::Consumed);
                        state.bind(*value_slot, AbstractSlot::Value(value))?;
                    }
                    TaskInstructionKind::EnterScope { scope } => {
                        state
                            .ledger
                            .enter_scope(0, *scope)
                            .map_err(|e| format!("{e:?}"))?;
                        state.scopes.push(*scope);
                    }
                    TaskInstructionKind::ExitScope { scope } => {
                        state
                            .ledger
                            .exit_scope(0, *scope)
                            .map_err(|e| format!("{e:?}"))?;
                        state.scopes.pop();
                    }
                    TaskInstructionKind::Join {
                        handle,
                        outcome_slot,
                    } => {
                        let Some(AbstractSlot::Handle {
                            worker,
                            child,
                            handle: runtime_handle,
                            returned,
                            errors,
                            normal,
                        }) = state.slots.get(handle).cloned()
                        else {
                            return Err("join requires a live handle slot".into());
                        };
                        let worker = &module.functions[&worker];
                        if outcome_slot.is_none()
                            && (worker.result != ScalarType::Unit || !worker.errors.is_empty())
                        {
                            return Err("discarded join must have an infallible unit result".into());
                        }
                        let capsule = state
                            .ledger
                            .join(0, runtime_handle)
                            .map_err(|e| format!("{e:?}"))?;
                        if capsule.resources != returned.into_iter().collect::<Vec<_>>() {
                            return Err("join changed returned resource identity".into());
                        }
                        if state.ledger.task_state(child)
                            != Some(crate::scoped_tasks::TaskState::Joined)
                        {
                            return Err("join did not consume child".into());
                        }
                        state.slots.insert(*handle, AbstractSlot::Consumed);
                        if let Some(slot) = outcome_slot {
                            let value = normal.then(|| {
                                returned
                                    .map(AbstractValue::Owned)
                                    .unwrap_or(AbstractValue::Scalar(worker.result))
                            });
                            state.bind(
                                *slot,
                                AbstractSlot::Result {
                                    value,
                                    errors,
                                    declared_errors: worker.errors.clone(),
                                },
                            )?;
                        }
                    }
                }
                Ok(())
            })();
            result.map_err(|message| fail(instruction.span, message))?;
        }
        let result: Result<(), String> = (|| {
            match &block.terminator.kind {
                TaskTerminatorKind::Goto { target } => queue.push((*target, state)),
                TaskTerminatorKind::TryAllocateBuffer {
                    initial,
                    value_slot,
                    success,
                    allocation_error,
                } => {
                    if state.value(initial)?.ty() != ScalarType::I32 {
                        return Err("buffer initialization requires a copy i32".into());
                    }
                    queue.push((*allocation_error, state.clone()));
                    let resource = state.next_id;
                    state.next_id += 1;
                    state
                        .ledger
                        .acquire(0, resource)
                        .map_err(|e| format!("{e:?}"))?;
                    state.resources.insert(resource);
                    state.bind(
                        *value_slot,
                        AbstractSlot::Value(AbstractValue::Owned(resource)),
                    )?;
                    queue.push((*success, state));
                }
                TaskTerminatorKind::TrySpawn {
                    scope,
                    worker,
                    arguments,
                    handle_slot,
                    success,
                    spawn_error,
                } => {
                    if *worker == module.entry {
                        return Err("entry cannot be used as a worker".into());
                    }
                    let callee = module
                        .functions
                        .get(worker)
                        .ok_or("unknown worker function")?;
                    if arguments.len() != callee.parameters.len() {
                        return Err("worker capture arity mismatch".into());
                    }
                    let mut resources = vec![];
                    let mut captured = BTreeMap::new();
                    let mut consumed_slots = BTreeSet::new();
                    for (index, (argument, (_, expected))) in
                        arguments.iter().zip(&callee.parameters).enumerate()
                    {
                        let value = state.value(argument)?;
                        if value.ty() != *expected {
                            return Err("worker copy/move capture type mismatch".into());
                        }
                        if let Some(resource) = value.resource() {
                            let ScalarOperand::Move(slot) = argument else {
                                unreachable!()
                            };
                            if !consumed_slots.insert(*slot) {
                                return Err("duplicate owned capture".into());
                            }
                            resources.push(resource);
                            captured.insert(index, resource);
                        }
                    }
                    let reservation = state.next_id;
                    state.next_id += 1;
                    state
                        .ledger
                        .reserve(0, *scope, reservation, &resources)
                        .map_err(|e| format!("{e:?}"))?;
                    let mut failed = state.clone();
                    failed
                        .ledger
                        .rollback(0, reservation)
                        .map_err(|e| format!("{e:?}"))?;
                    queue.push((*spawn_error, failed));
                    state
                        .ledger
                        .commit(0, reservation, reservation, reservation)
                        .map_err(|e| format!("{e:?}"))?;
                    for slot in consumed_slots {
                        state.slots.insert(slot, AbstractSlot::Consumed);
                    }
                    let worker_exits = summaries
                        .get(worker)
                        .ok_or("worker has no checked resource-effect summary")?;
                    for exit in worker_exits {
                        let mut success_state = state.clone();
                        for input in &exit.dropped {
                            let resource = *captured
                                .get(input)
                                .ok_or("worker drop does not refer to an owned capture")?;
                            success_state
                                .ledger
                                .drop_resource(reservation, resource)
                                .map_err(|e| format!("{e:?}"))?;
                        }
                        let returned = match &exit.returned {
                            Some(WorkerReturn::Input(input)) => Some(
                                *captured
                                    .get(input)
                                    .ok_or("worker return does not refer to an owned capture")?,
                            ),
                            Some(WorkerReturn::Fresh) => {
                                // The worker exit checked every local allocation:
                                // exactly this fresh result remains, all others
                                // are dropped. Materialize its caller-side identity
                                // under child custody before completion/join.
                                let resource = success_state.next_id;
                                success_state.next_id += 1;
                                success_state
                                    .ledger
                                    .acquire(reservation, resource)
                                    .map_err(|e| format!("{e:?}"))?;
                                success_state.resources.insert(resource);
                                Some(resource)
                            }
                            None => None,
                        };
                        let outcome = if let Some(tag) = &exit.error {
                            TaskOutcome::Error(tag.clone())
                        } else {
                            match callee.result {
                                ScalarType::Unit => TaskOutcome::Unit,
                                ScalarType::I32 => TaskOutcome::I32(0),
                                ScalarType::OwnedBuffer => TaskOutcome::OwnedBuffer(
                                    returned
                                        .ok_or("owned worker outcome has no checked identity")?,
                                ),
                            }
                        };
                        success_state
                            .ledger
                            .complete(
                                reservation,
                                outcome,
                                &returned.into_iter().collect::<Vec<_>>(),
                            )
                            .map_err(|e| format!("{e:?}"))?;
                        success_state.bind(
                            *handle_slot,
                            AbstractSlot::Handle {
                                worker: *worker,
                                child: reservation,
                                handle: reservation,
                                returned,
                                errors: exit.error.clone().into_iter().collect(),
                                normal: exit.error.is_none(),
                            },
                        )?;
                        queue.push((*success, success_state));
                    }
                }
                TaskTerminatorKind::TryOutcome {
                    result_slot,
                    value_slot,
                    error_slot,
                    success,
                    error,
                } => {
                    let Some(AbstractSlot::Result { value, errors, .. }) =
                        state.slots.get(result_slot).cloned()
                    else {
                        return Err("propagation requires an initialized Result slot".into());
                    };
                    state.slots.insert(*result_slot, AbstractSlot::Consumed);
                    // Empty-error scalar matches retain the historical conservative
                    // arm check. An owned Result never invents an error capsule.
                    if !errors.is_empty()
                        || value
                            .as_ref()
                            .is_some_and(|v| v.ty() != ScalarType::OwnedBuffer)
                    {
                        let mut failed = state.clone();
                        failed.bind(*error_slot, AbstractSlot::Error(errors))?;
                        queue.push((*error, failed));
                    }
                    if let Some(value) = value {
                        state.bind(*value_slot, AbstractSlot::Value(value))?;
                        queue.push((*success, state));
                    }
                }
                TaskTerminatorKind::Return { value } => {
                    let value = state.take(value)?;
                    if value.ty() != function.result {
                        return Err("return scalar/owned type mismatch".into());
                    }
                    let returned = value.resource();
                    state.exit(returned)?;
                    exits.push(WorkerExit {
                        returned: returned.map(|r| {
                            inputs
                                .get(&r)
                                .copied()
                                .map(WorkerReturn::Input)
                                .unwrap_or(WorkerReturn::Fresh)
                        }),
                        dropped: inputs
                            .iter()
                            .filter_map(|(&resource, &index)| {
                                (Some(resource) != returned).then_some(index)
                            })
                            .collect(),
                        error: None,
                    });
                }
                TaskTerminatorKind::Raise { error } => {
                    let errors = match error {
                        ErrorOperand::Tag(tag) if error_name(tag) => BTreeSet::from([tag.clone()]),
                        ErrorOperand::Slot(slot) => match state.slots.get(slot) {
                            Some(AbstractSlot::Error(errors)) => errors.clone(),
                            _ => {
                                return Err("raise requires an initialized typed error slot".into());
                            }
                        },
                        _ => return Err("invalid error label".into()),
                    };
                    if !errors.is_subset(&function.errors) {
                        return Err("raised errors are absent from the declared row".into());
                    }
                    state.exit(None)?;
                    for error in errors {
                        exits.push(WorkerExit {
                            returned: None,
                            dropped: inputs.values().copied().collect(),
                            error: Some(error),
                        });
                    }
                }
            }
            Ok(())
        })();
        result.map_err(|message| fail(block.terminator.span, message))?;
    }
    if reached.len() != function.blocks.len() {
        return Err(fail(
            function.span,
            "unreachable blocks are outside the finite task profile".into(),
        ));
    }
    // Scalar outcome values cannot affect custody in this profile. Once every
    // actual worker exit drops every input, declared scalar/error alternatives
    // are a safe overapproximation with that same checked resource effect.
    if id != module.entry && function.result != ScalarType::OwnedBuffer {
        let dropped = inputs.values().copied().collect::<Vec<_>>();
        exits = std::iter::once(WorkerExit {
            returned: None,
            dropped: dropped.clone(),
            error: None,
        })
        .chain(function.errors.iter().cloned().map(|error| WorkerExit {
            returned: None,
            dropped: dropped.clone(),
            error: Some(error),
        }))
        .collect();
    }
    Ok(exits)
}

pub fn check_task_module(module: &TaskModule) -> Result<CheckedTaskModule, Vec<TaskDiagnostic>> {
    if !module.functions.contains_key(&module.entry) {
        return Err(vec![problem(
            module.entry,
            TaskSpan::default(),
            "missing task entry function",
        )]);
    }
    let mut names = BTreeSet::new();
    let mut errors = Vec::new();
    let mut summaries = BTreeMap::new();
    let order = module
        .functions
        .keys()
        .copied()
        .filter(|id| *id != module.entry)
        .chain(std::iter::once(module.entry));
    for id in order {
        let function = &module.functions[&id];
        if !names.insert(function.name.clone()) {
            errors.push(problem(id, function.span, "duplicate function name"));
        }
        match check_function(module, id, &summaries) {
            Ok(exits) => {
                summaries.insert(id, exits);
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(CheckedTaskModule {
            module: module.clone(),
        })
    } else {
        Err(errors)
    }
}

mod reference;
pub use reference::*;
#[cfg(test)]
mod tests;
