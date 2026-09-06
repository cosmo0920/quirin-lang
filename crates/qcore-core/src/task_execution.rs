//! Finite source-task execution profile: immutable i32/unit copies and leaf workers.
//! Model checking covers every creation and declared worker outcome. A separate
//! reference run chooses creation outcomes; it is not the all-path certificate.
//! Source parsing, native runtime refinement, resources, loans and synchronization
//! are separate boundaries. This IR has no operation that erases those effects.
pub use crate::scoped_tasks::ScopeId;
use crate::scoped_tasks::{ScopedTaskLedger, TaskOutcome};
use std::collections::{BTreeMap, BTreeSet};
pub type FunctionId = usize;
pub type SlotId = usize;
pub type BlockId = usize;
pub const SPAWN_ERROR: &str = "SpawnError";
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
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarOperand {
    Unit,
    I32(i32),
    Slot(SlotId),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorOperand {
    Tag(String),
    Slot(SlotId),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskEffects {
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
        _ => vec![],
    }
}

#[derive(Clone)]
enum AbstractSlot {
    Scalar(ScalarType),
    Result {
        scalar: ScalarType,
        errors: BTreeSet<String>,
    },
    Error(BTreeSet<String>),
    Handle {
        worker: FunctionId,
        child: u64,
        handle: u64,
    },
    Consumed,
}
#[derive(Clone)]
struct State {
    slots: BTreeMap<SlotId, AbstractSlot>,
    ledger: ScopedTaskLedger,
    scopes: Vec<ScopeId>,
    next_id: u64,
}
impl State {
    fn scalar(&self, operand: &ScalarOperand) -> Result<ScalarType, String> {
        match operand {
            ScalarOperand::Unit => Ok(ScalarType::Unit),
            ScalarOperand::I32(_) => Ok(ScalarType::I32),
            ScalarOperand::Slot(id) => match self.slots.get(id) {
                Some(AbstractSlot::Scalar(ty)) => Ok(*ty),
                _ => Err(format!("slot {id} is not an initialized scalar")),
            },
        }
    }
    fn bind(&mut self, id: SlotId, value: AbstractSlot) -> Result<(), String> {
        if self.slots.contains_key(&id) {
            return Err(format!("slot {id} is written more than once"));
        }
        self.slots.insert(id, value);
        Ok(())
    }
    fn exit(&self) -> Result<(), String> {
        if !self.scopes.is_empty() {
            return Err("return/error leaves an open thread scope".into());
        }
        if self
            .slots
            .values()
            .any(|s| matches!(s, AbstractSlot::Handle { .. } | AbstractSlot::Result { .. }))
        {
            return Err("return/error discards a handle or unconsumed Result".into());
        }
        Ok(())
    }
}

fn check_function(module: &TaskModule, id: FunctionId) -> Result<(), TaskDiagnostic> {
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
            "SpawnError is reserved for creation, not worker errors".into(),
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
    if (effects.spawn && !function.declared_effects.spawn)
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
    let state = State {
        slots: function
            .parameters
            .iter()
            .map(|&(id, ty)| (id, AbstractSlot::Scalar(ty)))
            .collect(),
        ledger: ScopedTaskLedger::default(),
        scopes: vec![],
        next_id: 1,
    };
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
                        state.bind(*slot, AbstractSlot::Scalar(state.scalar(value)?))?
                    }
                    TaskInstructionKind::UnwrapInfallible {
                        result_slot,
                        value_slot,
                    } => {
                        let Some(AbstractSlot::Result { scalar, errors }) =
                            state.slots.get(result_slot).cloned()
                        else {
                            return Err(
                                "infallible extraction requires an initialized Result slot".into(),
                            );
                        };
                        if !errors.is_empty() {
                            return Err(
                                "infallible extraction requires an empty declared error row".into(),
                            );
                        }
                        state.slots.insert(*result_slot, AbstractSlot::Consumed);
                        state.bind(*value_slot, AbstractSlot::Scalar(scalar))?;
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
                        // Normal scalar payloads are symbolic here: there is no
                        // scalar-dependent custody operation in this profile.
                        // Check every declared error alternative too, then merge
                        // only identical post-join ledgers. Actual i32 values and
                        // exact tags are retained by the separate reference run.
                        let normal = match worker.result {
                            ScalarType::Unit => TaskOutcome::Unit,
                            ScalarType::I32 => TaskOutcome::I32(0),
                        };
                        let outcomes = std::iter::once(normal)
                            .chain(worker.errors.iter().cloned().map(TaskOutcome::Error));
                        let mut joined = None;
                        for outcome in outcomes {
                            count += 1;
                            if count > MAX_CHECK_STATES {
                                return Err("task path exploration bound exhausted".into());
                            }
                            let mut ledger = state.ledger.clone();
                            ledger
                                .complete(child, outcome.clone(), &[])
                                .map_err(|e| format!("{e:?}"))?;
                            let capsule = ledger
                                .join(0, runtime_handle)
                                .map_err(|e| format!("{e:?}"))?;
                            if capsule.outcome != outcome || !capsule.resources.is_empty() {
                                return Err("join changed the modeled worker outcome".into());
                            }
                            if joined.as_ref().is_some_and(|prior| prior != &ledger) {
                                return Err("worker outcomes disagree on post-join custody".into());
                            }
                            joined = Some(ledger);
                        }
                        state.ledger = joined.expect("normal outcome always modeled");
                        state.slots.insert(*handle, AbstractSlot::Consumed);
                        if let Some(slot) = outcome_slot {
                            state.bind(
                                *slot,
                                AbstractSlot::Result {
                                    scalar: worker.result,
                                    errors: worker.errors.clone(),
                                },
                            )?;
                        }
                    }
                }
                Ok(())
            })();
            result.map_err(|message| fail(instruction.span, message))?;
        }
        let span = block.terminator.span;
        let result: Result<(), String> = (|| {
            match &block.terminator.kind {
                TaskTerminatorKind::Goto { target } => queue.push((*target, state)),
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
                    for (argument, (_, expected)) in arguments.iter().zip(&callee.parameters) {
                        if state.scalar(argument)? != *expected {
                            return Err("worker copy capture type mismatch".into());
                        }
                    }
                    let reservation = state.next_id;
                    state.next_id += 1;
                    state
                        .ledger
                        .reserve(0, *scope, reservation, &[])
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
                    state.bind(
                        *handle_slot,
                        AbstractSlot::Handle {
                            worker: *worker,
                            child: reservation,
                            handle: reservation,
                        },
                    )?;
                    queue.push((*success, state));
                }
                TaskTerminatorKind::TryOutcome {
                    result_slot,
                    value_slot,
                    error_slot,
                    success,
                    error,
                } => {
                    let Some(AbstractSlot::Result { scalar, errors }) =
                        state.slots.get(result_slot).cloned()
                    else {
                        return Err("propagation requires an initialized Result slot".into());
                    };
                    state.slots.insert(*result_slot, AbstractSlot::Consumed);
                    let mut failed = state.clone();
                    failed.bind(*error_slot, AbstractSlot::Error(errors))?;
                    queue.push((*error, failed));
                    state.bind(*value_slot, AbstractSlot::Scalar(scalar))?;
                    queue.push((*success, state));
                }
                TaskTerminatorKind::Return { value } => {
                    if state.scalar(value)? != function.result {
                        return Err("return scalar type mismatch".into());
                    }
                    state.exit()?;
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
                    state.exit()?;
                }
            }
            Ok(())
        })();
        result.map_err(|message| fail(span, message))?;
    }
    if reached.len() != function.blocks.len() {
        return Err(fail(
            function.span,
            "unreachable blocks are outside the finite task profile".into(),
        ));
    }
    Ok(())
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
    for (&id, function) in &module.functions {
        if !names.insert(function.name.clone()) {
            errors.push(problem(id, function.span, "duplicate function name"));
        }
        if let Err(error) = check_function(module, id) {
            errors.push(error);
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
