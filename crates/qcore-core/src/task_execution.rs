//! Finite source-task execution: immutable i32/unit copies, affine owned buffers,
//! one bounded scoped read loan and leaf workers. Model checking covers
//! allocation/creation branches and checked worker custody effects. A reference
//! run chooses concrete outcomes; it is not the all-path certificate.
//! Source/native refinement, arbitrary owned graphs, fractional or multi-reader
//! shares and synchronization remain separate boundaries.
//!
//! The read loan (card T04) is deliberately minimal: at most one loan is
//! outstanding at a time, at most one loan argument per creation, a loan is
//! never copied, re-lent, moved, returned or dropped, and only the creator's
//! join closes it. Every other borrowing form stays outside this profile.
//!
//! The typed error payload (card T06) is equally minimal: a function declares
//! which of its error tags carry a payload, a payload is exactly one owned
//! Buffer, a raise of such a tag must move one in, and the joining creator must
//! take it out of the bound error before that error can be discarded or
//! re-raised. `SpawnError` and `AllocError` never carry one — the runtime
//! raises them and holds nothing — and the task entry declares none, because it
//! has no external custodian. Several payloads, borrowed payloads, scalar
//! payloads and payload-carrying rethrow of a bound error stay outside.
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
    /// A scoped read loan of a Buffer the creator keeps owning. It is not a
    /// value type: it cannot be a function result, an entry parameter, a copy
    /// source, a move source or a re-loan source.
    ReadLoan,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarOperand {
    Unit,
    I32(i32),
    Slot(SlotId),
    Move(SlotId),
    /// Lend an owned Buffer to one child for the outstanding creation. The
    /// slot keeps its binding; only the authority is suspended.
    Loan(SlotId),
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
    /// The declared subset of `errors` whose raise carries exactly one owned
    /// Buffer. Membership is part of the signature, not of a path: a tag either
    /// always carries a payload here or never does.
    pub error_payloads: BTreeSet<String>,
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
    /// Move the owned Buffer out of a bound typed error. This is the only way
    /// a payload leaves an error value, it can happen at most once per error
    /// binding, and until it happens the error can neither be discarded at an
    /// exit nor re-raised.
    TakeErrorPayload {
        error_slot: SlotId,
        buffer_slot: SlotId,
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
        /// Exactly one owned Buffer when the raised tag declares a payload, and
        /// nothing otherwise. Only a literal `Move` is admitted, so a payload
        /// is never a copy, a scalar or a borrowed identity.
        payload: Option<ScalarOperand>,
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
    /// Read authority over a resource this frame does not own. In a worker
    /// frame the lender is outside the frame entirely, so the identity is not
    /// registered in this frame's ledger and carries no discharge duty.
    Loan(ResourceId),
}
impl AbstractValue {
    fn ty(&self) -> ScalarType {
        match self {
            Self::Scalar(ty) => *ty,
            Self::Owned(_) => ScalarType::OwnedBuffer,
            Self::Loan(_) => ScalarType::ReadLoan,
        }
    }
    /// Owned identity only. A loan is never an owned identity, so it can never
    /// be returned, dropped or accounted for as one.
    fn resource(&self) -> Option<ResourceId> {
        match self {
            Self::Owned(id) => Some(*id),
            _ => None,
        }
    }
    /// The identity a read is performed against, owned or borrowed.
    fn readable(&self) -> Option<ResourceId> {
        match self {
            Self::Owned(id) | Self::Loan(id) => Some(*id),
            Self::Scalar(_) => None,
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
        /// Whether the producing worker's declared error row carries payloads.
        /// This is a property of the signature, so it is the same on the normal
        /// and error alternatives of one creation.
        declared_payloads: bool,
        /// The owned Buffer this Result's error alternative carries, if any.
        /// Normal and error alternatives are separate states, so at most one of
        /// `value` and `error_payload` is ever populated.
        error_payload: Option<ResourceId>,
    },
    Error {
        errors: BTreeSet<String>,
        /// Still-undelivered payload. While this is `Some`, the error may not
        /// be discarded at an exit and may not be re-raised.
        payload: Option<ResourceId>,
    },
    Handle {
        worker: FunctionId,
        child: u64,
        handle: u64,
        returned: Option<ResourceId>,
        error_payload: Option<ResourceId>,
        errors: BTreeSet<String>,
        normal: bool,
    },
    Consumed,
}
/// Each alternative is derived from a checked worker exit, using original
/// parameter positions or a proved fresh local result, never caller assertions.
#[derive(Clone, Debug, PartialEq, Eq)]
enum WorkerReturn {
    Input(usize),
    Fresh,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkerExit {
    returned: Option<WorkerReturn>,
    dropped: Vec<usize>,
    error: Option<String>,
    /// The owned Buffer this error exit carries out of the worker. It is
    /// derived from the worker's own checked exit, exactly like `returned`,
    /// and is `None` for every normal exit and every payload-free error.
    error_payload: Option<WorkerReturn>,
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
            // Lending demands exclusive authority at the moment of lending, so
            // a resource that is already loaned, moved away, reserved or
            // dropped cannot be lent again. A loan itself is never lendable:
            // this profile has no transitive or nested borrowing.
            ScalarOperand::Loan(id) => match self.slots.get(id) {
                Some(AbstractSlot::Value(AbstractValue::Owned(resource))) => {
                    self.ledger
                        .use_resource(0, *resource)
                        .map_err(|e| format!("{e:?}"))?;
                    Ok(AbstractValue::Loan(*resource))
                }
                _ => Err(format!(
                    "slot {id} is not an owned buffer this task may lend"
                )),
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
    /// Read authority over a slot: an owned Buffer this frame still has read
    /// authority over — including one it has lent out — or a read loan it
    /// holds. Exclusive authority is deliberately *not* demanded here.
    fn readable(&self, slot: SlotId) -> Result<ResourceId, String> {
        // A slot that is neither an owned buffer nor a loan keeps the original
        // owned-buffer refusal verbatim: adding a borrowing form must not
        // change what a stale, moved-away or scalar slot reports.
        let unavailable = || format!("slot {slot} is not an initialized owned buffer");
        let Some(AbstractSlot::Value(value)) = self.slots.get(&slot) else {
            return Err(unavailable());
        };
        let resource = value.readable().ok_or_else(unavailable)?;
        if matches!(value, AbstractValue::Owned(_)) {
            self.ledger
                .read_resource(0, resource)
                .map_err(|e| format!("{e:?}"))?;
        }
        Ok(resource)
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
        // A payload that is still inside a bound error is a duty this exit did
        // not discharge. The leak check below would also catch it, but naming
        // it here says which value was left holding the Buffer.
        if self.slots.values().any(|s| {
            matches!(
                s,
                AbstractSlot::Error {
                    payload: Some(_),
                    ..
                }
            )
        }) {
            return Err("return/error discards an undelivered error payload".into());
        }
        // An outstanding loan is an unfinished lifetime duty, not a leak: name
        // it separately so the refusal says what the exit actually failed to
        // do, which is close the loan by joining its borrower.
        if let Some(&resource) = self
            .resources
            .iter()
            .find(|&&resource| self.ledger.loan_borrower(resource).is_some())
        {
            return Err(format!(
                "return/error leaves resource {resource} lent to an unjoined child"
            ));
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
    // A declared payload row is checked before any path: an undeclared tag, a
    // runtime-raised tag and the entry function are refused outright rather
    // than becoming a per-path surprise.
    if !function.error_payloads.is_subset(&function.errors) {
        return Err(fail(
            function.span,
            "an error payload is declared for a tag outside the declared error row".into(),
        ));
    }
    if function.error_payloads.contains(SPAWN_ERROR)
        || function.error_payloads.contains(ALLOC_ERROR)
    {
        return Err(fail(
            function.span,
            "creation and allocation failures hold no resource and carry no payload".into(),
        ));
    }
    if id == module.entry && !function.error_payloads.is_empty() {
        return Err(fail(
            function.span,
            "owned task entry error payloads have no external custodian in this profile".into(),
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
                TaskInstructionKind::TakeErrorPayload { buffer_slot, .. } => Some(*buffer_slot),
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
    // A read loan is bound to the creation that opened it. Returning one would
    // outlive that creation's join, so it is refused at the type, before any
    // path is explored.
    if function.result == ScalarType::ReadLoan {
        return Err(fail(
            function.span,
            "a read loan cannot be a task function result".into(),
        ));
    }
    if id == module.entry
        && function
            .parameters
            .iter()
            .any(|(_, kind)| *kind == ScalarType::ReadLoan)
    {
        return Err(fail(
            function.span,
            "the task entry has no lender, so it cannot take a read loan".into(),
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
        let value = match ty {
            ScalarType::OwnedBuffer => {
                let resource = state.next_id;
                state.next_id += 1;
                state
                    .ledger
                    .acquire(0, resource)
                    .map_err(|e| fail(function.span, format!("{e:?}")))?;
                state.resources.insert(resource);
                inputs.insert(resource, index);
                AbstractValue::Owned(resource)
            }
            // A loan is not registered in this frame's ledger and is not an
            // `input`: the borrower owes nothing at exit, and no worker exit
            // can report it as returned or dropped.
            ScalarType::ReadLoan => {
                let resource = state.next_id;
                state.next_id += 1;
                AbstractValue::Loan(resource)
            }
            ScalarType::I32 | ScalarType::Unit => AbstractValue::Scalar(ty),
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
                        state.readable(*buffer)?;
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
                    TaskInstructionKind::TakeErrorPayload {
                        error_slot,
                        buffer_slot,
                    } => {
                        let Some(AbstractSlot::Error { errors, payload }) =
                            state.slots.get(error_slot).cloned()
                        else {
                            return Err(
                                "taking a payload requires an initialized typed error slot".into(),
                            );
                        };
                        let payload =
                            payload.ok_or("this typed error carries no owned payload to take")?;
                        // The error keeps its tags and loses its payload, so a
                        // second take of the same binding is refused here and
                        // not by an accidental double account downstream.
                        state.slots.insert(
                            *error_slot,
                            AbstractSlot::Error {
                                errors,
                                payload: None,
                            },
                        );
                        state.bind(
                            *buffer_slot,
                            AbstractSlot::Value(AbstractValue::Owned(payload)),
                        )?;
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
                            error_payload,
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
                        // Exactly one of the two can be populated on any path,
                        // so this compares the join against the complete set of
                        // identities the completion was recorded to hold.
                        if capsule.resources
                            != returned
                                .into_iter()
                                .chain(error_payload)
                                .collect::<Vec<_>>()
                        {
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
                                    declared_payloads: !worker.error_payloads.is_empty(),
                                    error_payload,
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
                    let mut loans = vec![];
                    let mut captured = BTreeMap::new();
                    let mut consumed_slots = BTreeSet::new();
                    let mut lent_slots = BTreeSet::new();
                    for (index, (argument, (_, expected))) in
                        arguments.iter().zip(&callee.parameters).enumerate()
                    {
                        let value = state.value(argument)?;
                        if value.ty() != *expected {
                            return Err("worker copy/move/loan capture type mismatch".into());
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
                        if let AbstractValue::Loan(resource) = value {
                            let ScalarOperand::Loan(slot) = argument else {
                                unreachable!()
                            };
                            if !lent_slots.insert(*slot) {
                                return Err("duplicate read loan of one slot".into());
                            }
                            loans.push(resource);
                        }
                    }
                    // The card's bound, enforced structurally rather than
                    // assumed: one loan per creation, one outstanding at a
                    // time. Splitting read authority across several borrowers
                    // is a different profile, not a larger version of this one.
                    if loans.len() > 1 {
                        return Err(
                            "this profile admits at most one read loan per task creation".into(),
                        );
                    }
                    if !loans.is_empty() && !state.ledger.outstanding_loans().is_empty() {
                        return Err(
                            "this profile admits at most one outstanding read loan at a time"
                                .into(),
                        );
                    }
                    if resources.iter().any(|resource| loans.contains(resource)) {
                        return Err("one creation cannot both move and lend a resource".into());
                    }
                    let reservation = state.next_id;
                    state.next_id += 1;
                    state
                        .ledger
                        .reserve_with_loans(0, *scope, reservation, &resources, &loans)
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
                        // One resolver for both carried identities: a worker
                        // exit may hand back at most one, either as its normal
                        // result or inside its typed error, and both are the
                        // worker's own checked identity rather than a caller
                        // assertion.
                        let resolve =
                            |carried: &Option<WorkerReturn>,
                             state: &mut State|
                             -> Result<Option<ResourceId>, String> {
                                Ok(match carried {
                                    Some(WorkerReturn::Input(input)) => {
                                        Some(*captured.get(input).ok_or(
                                            "worker return does not refer to an owned capture",
                                        )?)
                                    }
                                    Some(WorkerReturn::Fresh) => {
                                        // The worker exit checked every local allocation:
                                        // exactly this fresh result remains, all others
                                        // are dropped. Materialize its caller-side identity
                                        // under child custody before completion/join.
                                        let resource = state.next_id;
                                        state.next_id += 1;
                                        state
                                            .ledger
                                            .acquire(reservation, resource)
                                            .map_err(|e| format!("{e:?}"))?;
                                        state.resources.insert(resource);
                                        Some(resource)
                                    }
                                    None => None,
                                })
                            };
                        let returned = resolve(&exit.returned, &mut success_state)?;
                        let error_payload = resolve(&exit.error_payload, &mut success_state)?;
                        if returned.is_some() && error_payload.is_some() {
                            return Err(
                                "a worker exit cannot both return and raise a Buffer".into()
                            );
                        }
                        let outcome = if let Some(tag) = &exit.error {
                            match error_payload {
                                Some(buffer) => TaskOutcome::ErrorWithBuffer {
                                    tag: tag.clone(),
                                    buffer,
                                },
                                None => TaskOutcome::Error(tag.clone()),
                            }
                        } else {
                            if error_payload.is_some() {
                                return Err(
                                    "a normal worker exit cannot carry an error payload".into()
                                );
                            }
                            match callee.result {
                                ScalarType::Unit => TaskOutcome::Unit,
                                ScalarType::I32 => TaskOutcome::I32(0),
                                ScalarType::OwnedBuffer => TaskOutcome::OwnedBuffer(
                                    returned
                                        .ok_or("owned worker outcome has no checked identity")?,
                                ),
                                ScalarType::ReadLoan => {
                                    return Err("a read loan is not a worker outcome".into());
                                }
                            }
                        };
                        success_state
                            .ledger
                            .complete(
                                reservation,
                                outcome,
                                &returned
                                    .into_iter()
                                    .chain(error_payload)
                                    .collect::<Vec<_>>(),
                            )
                            .map_err(|e| format!("{e:?}"))?;
                        success_state.bind(
                            *handle_slot,
                            AbstractSlot::Handle {
                                worker: *worker,
                                child: reservation,
                                handle: reservation,
                                returned,
                                error_payload,
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
                    let Some(AbstractSlot::Result {
                        value,
                        errors,
                        declared_payloads,
                        error_payload,
                        ..
                    }) = state.slots.get(result_slot).cloned()
                    else {
                        return Err("propagation requires an initialized Result slot".into());
                    };
                    state.slots.insert(*result_slot, AbstractSlot::Consumed);
                    // Empty-error scalar matches retain the historical conservative
                    // arm check. An owned Result never invents an error capsule,
                    // and neither does a payload-carrying one: synthesizing a
                    // payload-free error there would be a false state, not a
                    // conservative one. That costs no coverage, because a
                    // payload row is nonempty by construction, so every such
                    // creation has a genuine error alternative that checks the
                    // same arm — and if the worker never raises it, the arm is
                    // unreachable and the profile refuses the program.
                    if !errors.is_empty()
                        || (!declared_payloads
                            && value
                                .as_ref()
                                .is_some_and(|v| v.ty() != ScalarType::OwnedBuffer))
                    {
                        let mut failed = state.clone();
                        failed.bind(
                            *error_slot,
                            AbstractSlot::Error {
                                errors,
                                payload: error_payload,
                            },
                        )?;
                        queue.push((*error, failed));
                    } else if error_payload.is_some() {
                        // Unreachable by construction, and refused rather than
                        // assumed: a payload with no error edge would have no
                        // path on which its duty could be discharged.
                        return Err("an error payload has no reachable error edge".into());
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
                        error_payload: None,
                    });
                }
                TaskTerminatorKind::Raise { error, payload } => {
                    let errors = match error {
                        ErrorOperand::Tag(tag) if error_name(tag) => BTreeSet::from([tag.clone()]),
                        ErrorOperand::Slot(slot) => match state.slots.get(slot) {
                            // A bound error must have delivered its payload
                            // before it can be re-raised. Carrying one onward
                            // through a rethrow is a separate profile.
                            Some(AbstractSlot::Error {
                                errors,
                                payload: None,
                            }) => errors.clone(),
                            Some(AbstractSlot::Error {
                                payload: Some(_), ..
                            }) => {
                                return Err(
                                    "a bound error must deliver its payload before it is re-raised"
                                        .into(),
                                );
                            }
                            _ => {
                                return Err("raise requires an initialized typed error slot".into());
                            }
                        },
                        _ => return Err("invalid error label".into()),
                    };
                    if !errors.is_subset(&function.errors) {
                        return Err("raised errors are absent from the declared row".into());
                    }
                    // The declared payload row decides, tag by tag, whether
                    // this raise must move a Buffer in. Both directions are
                    // refusals: a missing payload and an unexpected one.
                    let declared = errors
                        .iter()
                        .filter(|tag| function.error_payloads.contains(*tag))
                        .count();
                    if declared != 0 && declared != errors.len() {
                        return Err(
                            "a raise cannot mix payload-carrying and payload-free tags".into()
                        );
                    }
                    if (declared != 0) != payload.is_some() {
                        return Err(
                            "raised payload presence disagrees with the declared error payload row"
                                .into(),
                        );
                    }
                    let carried = match payload {
                        Some(operand) => {
                            if !matches!(operand, ScalarOperand::Move(_)) {
                                return Err(
                                    "an error payload requires an explicit owned Buffer move"
                                        .into(),
                                );
                            }
                            let value = state.take(operand)?;
                            if value.ty() != ScalarType::OwnedBuffer {
                                return Err("an error payload must be an owned Buffer".into());
                            }
                            value.resource()
                        }
                        None => None,
                    };
                    state.exit(carried)?;
                    let carried_index = carried.and_then(|resource| {
                        inputs
                            .get(&resource)
                            .copied()
                            .map(WorkerReturn::Input)
                            .or(Some(WorkerReturn::Fresh))
                    });
                    for error in errors {
                        exits.push(WorkerExit {
                            returned: None,
                            dropped: inputs
                                .iter()
                                .filter_map(|(&resource, &index)| {
                                    (Some(resource) != carried).then_some(index)
                                })
                                .collect(),
                            error: Some(error),
                            error_payload: carried_index.clone(),
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
    //
    // A declared error payload breaks exactly that premise: such an exit hands
    // an identity back instead of dropping it, so the overapproximation would
    // be wrong rather than conservative. Those functions keep their actual
    // checked exits, which is what an owned result already does.
    if id != module.entry
        && function.result != ScalarType::OwnedBuffer
        && function.error_payloads.is_empty()
    {
        let dropped = inputs.values().copied().collect::<Vec<_>>();
        exits = std::iter::once(WorkerExit {
            returned: None,
            dropped: dropped.clone(),
            error: None,
            error_payload: None,
        })
        .chain(function.errors.iter().cloned().map(|error| WorkerExit {
            returned: None,
            dropped: dropped.clone(),
            error: Some(error),
            error_payload: None,
        }))
        .collect();
    }
    // Identical alternatives explore identical caller states; keeping them
    // would only multiply work. Order is preserved, so nothing is reordered.
    let mut seen = Vec::new();
    exits.retain(|exit| {
        let fresh = !seen.contains(exit);
        if fresh {
            seen.push(exit.clone());
        }
        fresh
    });
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
