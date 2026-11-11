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

//! Finite source-task execution: immutable i32/unit copies, affine owned buffers,
//! counted scoped read shares and leaf workers. Model checking covers
//! allocation/creation branches and checked worker custody effects. A reference
//! run chooses concrete outcomes; it is not the all-path certificate.
//! Source/native refinement, arbitrary owned graphs, disjoint slicing, write
//! shares and synchronization remain separate boundaries.
//!
//! The read loan (card T04) is deliberately minimal: at most one *resource* is
//! lent at a time, at most one loan argument per creation, a loan is never
//! copied, re-lent, moved, returned or dropped, and only the creator's join
//! returns its share. Card T05 makes each loan carry a positive share out of
//! `FULL_READ_AUTHORITY`, so one resource may be lent to at most `MAX_READERS`
//! borrowers whose shares sum to no more than the full authority, and adds the
//! Buffer write whose exclusive-authority requirement the share ledger decides.
//! Every other borrowing form stays outside this profile.
//!
//! The typed error payload (card T06) is equally minimal: a function declares
//! which of its error tags carry a payload, a payload is exactly one owned
//! Buffer, a raise of such a tag must move one in, and the joining creator must
//! take it out of the bound error before that error can be discarded or
//! re-raised. `SpawnError` and `AllocError` never carry one — the runtime
//! raises them and holds nothing — and the task entry declares none, because it
//! has no external custodian. Several payloads, borrowed payloads, scalar
//! payloads and payload-carrying rethrow of a bound error stay outside.
use crate::scalar_types::{self as st, ArithmeticError, RecordedTarget};
use crate::scalar_values::{self, Decider, ValueRefusal};
pub use crate::scoped_tasks::{FULL_READ_AUTHORITY, LeaseRange, MAX_READERS, MAX_WRITERS, ScopeId};
use crate::scoped_tasks::{
    ResourceId, ResourceState, ScopedTaskLedger, TaskId, TaskLedgerError, TaskOutcome,
};
use std::collections::{BTreeMap, BTreeSet};
pub type FunctionId = usize;
pub type SlotId = usize;
pub type BlockId = usize;
pub const SPAWN_ERROR: &str = "SpawnError";
pub const ALLOC_ERROR: &str = "AllocError";
pub type TaskValueType = ScalarType;
pub type TaskOperand = ScalarOperand;
const MAX_CHECK_STATES: usize = 16_384;

/// The byte extent of the one payload shape this route's Buffer has: a single
/// `int32_t` (card `WRITE-LEND-TASKS`). A range write lease's bounds are
/// decided against this extent, so the interval a worker holds names real
/// bytes of the emitted `struct qcore_buffer`'s payload — never a declared
/// axis with no memory behind it. A byte-extent buffer (`bytes(N)` crossing a
/// spawn) is the named residual `WRITE-LEND-TASKS-EXTENT`.
pub const BUFFER_PAYLOAD_BYTES: u64 = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskSpan {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarType {
    Unit,
    I32,
    /// Every other type of the §G02t fixed-width table (card
    /// `W48-QR-SCALAR-TYPES-R4`): `bool`, `i8`, `i16`, `i64`, `isize`, `u8`,
    /// `u16`, `u32`, `u64`, `usize`. `i32` keeps its own variant, so one type
    /// has one representation: `Fixed(i32)` is malformed and refused by the
    /// checker. A value of it is an immutable copy scalar exactly as an `i32`
    /// is — it has no identity and carries no custody.
    Fixed(st::ScalarType),
    /// An owned Buffer whose payload is one value of the carried §G02t type
    /// (card `W48-QR-SCALAR-TYPES-R6`, `W48-QR-TYPES-PAYLOADS`): the payload
    /// type is part of the resource's type, so two Buffers of different
    /// payload types are different types everywhere a type is compared — a
    /// capture, a result, a join. `OwnedBuffer(i32)` is every Buffer written
    /// before that card ([`Self::OWNED_BUFFER`]).
    OwnedBuffer(st::ScalarType),
    /// A scoped read loan of a Buffer the creator keeps owning. It is not a
    /// value type: it cannot be a function result, an entry parameter, a copy
    /// source, a move source or a re-loan source. It carries the lent
    /// Buffer's payload type.
    ReadLoan(st::ScalarType),
    /// A scoped **range write lease** of a Buffer the creator keeps owning
    /// (card `WRITE-LEND-TASKS`): write authority over `[low, high)` of the
    /// payload's [`BUFFER_PAYLOAD_BYTES`], transferred to one worker for
    /// spawn..join. Like a read loan it is not a value type — no result, no
    /// entry parameter, no copy, move or re-lease source. Card
    /// `WRITE-LEND-TASKS-EXERCISE` (wave 36) discharged the residual of that
    /// name: a worker body now **exercises** the authority, through
    /// [`TaskInstructionKind::WriteBufferByte`] and
    /// [`TaskInstructionKind::ReadBufferByte`], at an index relative to this
    /// lease's own `low`. It carries the leased Buffer's payload type, whose
    /// size — not a fixed four — is the extent a lease must stay inside.
    WriteLease(st::ScalarType),
    /// A record, named by its key in [`TaskModule::records`] (card
    /// `W48-QR-TASK-RECORD-CAPTURES`). It is a **result** type only: a record
    /// binding, parameter and capture is its leaves — one slot, parameter or
    /// argument per scalar or owned-Buffer leaf, depth-first in declaration
    /// order — so the one place a record is one value is a worker's result,
    /// which crosses `join` as a single outcome naming every owned leaf and is
    /// unpacked into leaf slots by [`TaskInstructionKind::UnpackRecord`].
    Record(usize),
}
impl ScalarType {
    /// The owned Buffer of an `i32` payload: `own<Buffer>`, every Buffer
    /// written before card `W48-QR-SCALAR-TYPES-R6`.
    pub const OWNED_BUFFER: Self = Self::OwnedBuffer(st::ScalarType::I32);
    /// `borrow<Buffer>` of an `i32` payload.
    pub const READ_LOAN: Self = Self::ReadLoan(st::ScalarType::I32);
    /// `lend<Buffer>` of an `i32` payload.
    pub const WRITE_LEASE: Self = Self::WriteLease(st::ScalarType::I32);
    /// The payload type of a Buffer, a loan or a lease; `None` for a copy
    /// scalar or `unit`.
    pub fn payload(self) -> Option<st::ScalarType> {
        match self {
            Self::OwnedBuffer(ty) | Self::ReadLoan(ty) | Self::WriteLease(ty) => Some(ty),
            Self::Unit | Self::I32 | Self::Fixed(_) | Self::Record(_) => None,
        }
    }
}

/// One record type a task module's results name (card
/// `W48-QR-TASK-RECORD-CAPTURES`): its source name and its **leaves**, each a
/// dotted field path from the record and a leaf type — `i32`, a
/// [`ScalarType::Fixed`] or an [`ScalarType::OwnedBuffer`] — depth-first in
/// declaration order, a nested record contributing its own leaves under its
/// field's name. The lowering flattens; the checker reads only the leaf list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRecord {
    pub name: String,
    pub leaves: Vec<TaskRecordLeaf>,
}

/// One leaf of a [`TaskRecord`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRecordLeaf {
    pub path: String,
    pub ty: ScalarType,
}

impl TaskRecord {
    /// Whether any leaf is an owned Buffer — a record that carries custody.
    pub fn owns(&self) -> bool {
        self.leaves
            .iter()
            .any(|leaf| matches!(leaf.ty, ScalarType::OwnedBuffer(_)))
    }
}

/// The byte extent of a payload of type `ty` (card `W48-QR-SCALAR-TYPES-R6`):
/// the type's size, which for `i32` is [`BUFFER_PAYLOAD_BYTES`]. `None` for an
/// `isize`/`usize` payload with no recorded target, which is refused before
/// any extent is asked for.
pub fn payload_extent(ty: st::ScalarType, target: Option<&RecordedTarget>) -> Option<u64> {
    ty.size_bytes(target).ok()
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarOperand {
    Unit,
    I32(i32),
    /// A literal of a [`ScalarType::Fixed`] type (card
    /// `W48-QR-SCALAR-TYPES-R4`): `value` must be a value of `ty`, and `ty` is
    /// never `i32`, whose literals are [`ScalarOperand::I32`]. `bool` is `0`/`1`.
    Fixed {
        ty: st::ScalarType,
        value: i128,
    },
    Slot(SlotId),
    Move(SlotId),
    /// Lend `share` of an owned Buffer's read authority to one child for the
    /// outstanding creation. The slot keeps its binding; only the share leaves.
    /// `share` is positive and at most `FULL_READ_AUTHORITY`; the ledger
    /// decides whether the *outstanding total* leaves room for it.
    Loan {
        slot: SlotId,
        share: u32,
    },
    /// Lease write authority over the bytes `[low, high)` of an owned
    /// Buffer's payload to one child for the outstanding creation (card
    /// `WRITE-LEND-TASKS`). The slot keeps its binding; only the range's
    /// write authority leaves, and only the creator's successful join
    /// returns it. The bounds are check-time constants of the crossing
    /// construct; whether the range is disjoint from every outstanding
    /// lease is the ledger's decision, by the wave-29 overlap rule.
    Lease {
        slot: SlotId,
        low: u64,
        high: u64,
    },
    /// A whole record, as its leaf operands in leaf order (card
    /// `W48-QR-TASK-RECORD-CAPTURES`): a copy of each scalar leaf and a
    /// [`ScalarOperand::Move`] of each owned leaf. Admitted only as the value
    /// of a [`TaskTerminatorKind::Return`] of a function whose result is a
    /// [`ScalarType::Record`]; at every other position it is refused.
    Record(Vec<ScalarOperand>),
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
    /// The comparison effects the row declares (card `W48-QR-FLOATING-POINT`):
    /// the permission a float comparison of this function's own body needs.
    /// A spawn is not a call — like `alloc`, a worker's comparison effects are
    /// its own row's — so nothing propagates through it.
    pub comparisons: crate::scalar_types::FloatEffects,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskModule {
    pub entry: FunctionId,
    pub functions: BTreeMap<FunctionId, TaskFunction>,
    pub source_len: usize,
    /// The builder's recorded target (card `W48-QR-SCALAR-TYPES-R4`), which
    /// alone gives `isize`/`usize` a width. A module naming either type with
    /// no record is refused; a module naming neither never reads it.
    pub target: Option<RecordedTarget>,
    /// The record types this module's results name, by the key a
    /// [`ScalarType::Record`] carries (card `W48-QR-TASK-RECORD-CAPTURES`).
    /// Empty for every module written before that card.
    pub records: BTreeMap<usize, TaskRecord>,
}

/// The one §G02t operation a task value statement performs (card
/// `W48-QR-SCALAR-TYPES-R4`) — the scalar route's arithmetic, comparison,
/// negation and `as`, one operation per statement as on the extern route. Its
/// operands are copy scalars: a literal (typed by the lowering from its
/// context) or a slot. What it may do wrong is decided by
/// [`crate::scalar_values::Decider`], the decision the extern route uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskValueOperation {
    Binary(st::Op, TaskValueOperand, TaskValueOperand),
    Compare(st::Cmp, TaskValueOperand, TaskValueOperand),
    Negate(TaskValueOperand),
    Convert(TaskValueOperand, st::ScalarType),
}

/// An operand of a [`TaskValueOperation`]: a copy scalar slot or a literal of
/// one §G02t type. Here an `i32` literal is `Literal { ty: I32, .. }` — the one
/// place a task literal carries its type explicitly, because a value
/// statement's literal is typed by its context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskValueOperand {
    Slot(SlotId),
    Literal { ty: st::ScalarType, value: i128 },
}

impl TaskValueOperation {
    /// Every operand, in source order.
    pub fn operands(&self) -> Vec<TaskValueOperand> {
        match *self {
            Self::Binary(_, a, b) | Self::Compare(_, a, b) => vec![a, b],
            Self::Negate(a) | Self::Convert(a, _) => vec![a],
        }
    }
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
    /// Replace a Buffer's payload. This is the operation the share ledger's
    /// exclusive gate exists for: it is admitted only when the writer owns the
    /// resource and no read share is outstanding.
    WriteBuffer {
        buffer: SlotId,
        value: ScalarOperand,
    },
    /// Write one byte of a Buffer's payload through a **range write lease**
    /// (card `WRITE-LEND-TASKS-EXERCISE`). `index` is relative to the
    /// authority's own base — a lease's `low` — so the owner byte this
    /// instruction names is `base + index`, the rebasing law the sequential
    /// lend states as `WRITE-LEND-COMPUTED-INDEX`. The index must be a
    /// check-time constant: an operand this instruction cannot resolve is not
    /// provably inside the authority and is refused rather than approximated.
    /// Only a lease reaches this instruction; the owner's whole-payload write
    /// is unchanged and is still its only write footprint.
    WriteBufferByte {
        authority: SlotId,
        index: ScalarOperand,
        value: ScalarOperand,
    },
    /// Read one byte of a Buffer's payload, through a lease (relative to its
    /// `low`) or through the owner (relative to 0, so absolute). The owner arm
    /// takes the *same* authority gate the whole-payload read takes, so a read
    /// while any lease is outstanding stays refused exactly as card
    /// `WRITE-LEND-TASKS` left it. A read share admits no byte view in this
    /// slice (`WRITE-LEND-EXERCISE-LOAN-BYTE-READ`).
    ReadBufferByte {
        authority: SlotId,
        index: ScalarOperand,
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
    /// `let NAME = VALUE_EXPRESSION;` that cannot raise (card
    /// `W48-QR-SCALAR-TYPES-R4`): every operand known at check time (folded,
    /// a fault refused), a comparison, or a total conversion. An operation
    /// that can raise at run time is [`TaskTerminatorKind::TryCompute`]; the
    /// checker refuses either form where the other is the right one.
    Compute {
        slot: SlotId,
        operation: TaskValueOperation,
    },
    /// Split a whole record value — a record a `join` collected — into one
    /// slot per leaf, in leaf order (card `W48-QR-TASK-RECORD-CAPTURES`). The
    /// record slot is consumed and each leaf slot receives exactly the leaf
    /// the record held: a scalar's type, or an owned leaf's identity and
    /// payload type, so custody is moved and never duplicated.
    UnpackRecord {
        record: SlotId,
        leaves: Vec<SlotId>,
    },
    /// Refill an owned leaf of a record (card `W48-QR-TASK-RECORD-CAPTURES`;
    /// card `W48-QR-USABLE-RECORDS`' `RefillField`): `hole` is the leaf's
    /// current slot, which must be a **hole** — its Buffer moved away or
    /// released — on this path; `value` is a [`ScalarOperand::Move`] of an
    /// owned Buffer of the leaf's payload type, which `slot` receives. A refill
    /// over a live leaf is refused (`OverwriteOfLiveField`): it would leave the
    /// old Buffer with no binding to release it.
    RefillField {
        hole: SlotId,
        slot: SlotId,
        value: ScalarOperand,
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
    /// A value statement that raises `ArithmeticError` at run time (card
    /// `W48-QR-SCALAR-TYPES-R4`): an operand is known only at run time and the
    /// operation can fault, which the function's row must permit. On success
    /// `slot` holds the result and control reaches `success`; on a fault
    /// `error_slot` holds the classified `ArithmeticError::VARIANT` — a bound
    /// typed error exactly like a `TryOutcome`'s — and control reaches
    /// `error`, whose block the lowering makes the route's ordinary error
    /// exit (scope exits, then `raise`).
    TryCompute {
        slot: SlotId,
        operation: TaskValueOperation,
        error_slot: SlotId,
        success: BlockId,
        error: BlockId,
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
    /// The type of every copy-scalar slot the checker bound, per function
    /// (card `W48-QR-SCALAR-TYPES-R4`). One slot has one type on every path —
    /// a disagreement is refused — so this is a fact about the module rather
    /// than about a path. The emitter and the composed route read it.
    slot_types: BTreeMap<(FunctionId, SlotId), ScalarType>,
    /// The payload type of every Buffer, loan and lease slot the checker
    /// bound, per function (card `W48-QR-SCALAR-TYPES-R6`), on the same
    /// one-type-per-slot rule. The emitter reads it to type each payload
    /// operation.
    buffer_payloads: BTreeMap<(FunctionId, SlotId), st::ScalarType>,
    /// The recorded target, exactly when the module names `isize` or `usize`.
    target_assumption: Option<RecordedTarget>,
}
impl CheckedTaskModule {
    /// The checked payload type of a Buffer, loan or lease slot (card
    /// `W48-QR-SCALAR-TYPES-R6`), or `None` for a slot that holds none.
    pub fn buffer_payload(&self, function: FunctionId, slot: SlotId) -> Option<st::ScalarType> {
        self.buffer_payloads.get(&(function, slot)).copied()
    }
    /// Every payload type a Buffer of this module has, `i32` included.
    pub fn payload_types(&self) -> BTreeSet<st::ScalarType> {
        self.buffer_payloads.values().copied().collect()
    }
    pub fn module(&self) -> &TaskModule {
        &self.module
    }
    /// The checked type of a copy-scalar slot, or `None` for a slot that holds
    /// none (a buffer, a loan, a handle, a Result, an error).
    pub fn slot_type(&self, function: FunctionId, slot: SlotId) -> Option<ScalarType> {
        self.slot_types.get(&(function, slot)).copied()
    }
    /// Every copy-scalar slot type of one function.
    pub fn function_slot_types(&self, function: FunctionId) -> BTreeMap<SlotId, ScalarType> {
        self.slot_types
            .iter()
            .filter(|((owner, _), _)| *owner == function)
            .map(|((_, slot), ty)| (*slot, *ty))
            .collect()
    }
    /// The recorded target this result depends on (card
    /// `W48-QR-SCALAR-TYPES-R4`): present exactly when the module names
    /// `isize` or `usize`. A named assumption of the verdict.
    pub fn target_assumption(&self) -> Option<&RecordedTarget> {
        self.target_assumption.as_ref()
    }
}

/// Every §G02t type other than `i32` the module names anywhere: parameter and
/// result types, typed literals, conversion targets, and `bool` for a
/// comparison. A computed value's type is always one of these, so this is the
/// module's whole fixed-width surface.
pub fn fixed_types(module: &TaskModule) -> BTreeSet<st::ScalarType> {
    module
        .functions
        .values()
        .flat_map(function_fixed_types)
        // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result's leaves are
        // values of their types wherever the record is unpacked.
        .chain(module.records.values().flat_map(|record| {
            record.leaves.iter().filter_map(|leaf| match leaf.ty {
                ScalarType::Fixed(ty) => Some(ty),
                ScalarType::OwnedBuffer(ty) if ty != st::ScalarType::I32 => Some(ty),
                _ => None,
            })
        }))
        .collect()
}

/// [`fixed_types`] of one function.
pub fn function_fixed_types(function: &TaskFunction) -> BTreeSet<st::ScalarType> {
    let mut types = BTreeSet::new();
    let kind = |types: &mut BTreeSet<st::ScalarType>, kind: &ScalarType| {
        if let ScalarType::Fixed(ty) = kind {
            types.insert(*ty);
        }
        // Card `W48-QR-SCALAR-TYPES-R6`: a typed payload is part of the
        // fixed-width surface — its reads are values of that type.
        if let Some(ty) = kind.payload()
            && ty != st::ScalarType::I32
        {
            types.insert(ty);
        }
    };
    fn literal(types: &mut BTreeSet<st::ScalarType>, operand: &ScalarOperand) {
        match operand {
            ScalarOperand::Fixed { ty, .. } => {
                types.insert(*ty);
            }
            // Card `W48-QR-TASK-RECORD-CAPTURES`: a returned record's leaves.
            ScalarOperand::Record(leaves) => {
                for leaf in leaves {
                    literal(types, leaf);
                }
            }
            _ => {}
        }
    }
    let operation = |types: &mut BTreeSet<st::ScalarType>, operation: &TaskValueOperation| {
        for operand in operation.operands() {
            if let TaskValueOperand::Literal { ty, .. } = operand
                && ty != st::ScalarType::I32
            {
                types.insert(ty);
            }
        }
        match operation {
            TaskValueOperation::Convert(_, to) if *to != st::ScalarType::I32 => {
                types.insert(*to);
            }
            TaskValueOperation::Compare(..) => {
                types.insert(st::ScalarType::Bool);
            }
            _ => {}
        }
    };
    {
        kind(&mut types, &function.result);
        for (_, parameter) in &function.parameters {
            kind(&mut types, parameter);
        }
        for block in function.blocks.values() {
            for instruction in &block.instructions {
                match &instruction.kind {
                    TaskInstructionKind::BindScalar { value, .. } => literal(&mut types, value),
                    TaskInstructionKind::Compute { operation: op, .. } => operation(&mut types, op),
                    _ => {}
                }
            }
            match &block.terminator.kind {
                TaskTerminatorKind::TryCompute { operation: op, .. } => operation(&mut types, op),
                TaskTerminatorKind::Return { value } => literal(&mut types, value),
                TaskTerminatorKind::TrySpawn { arguments, .. } => {
                    for argument in arguments {
                        literal(&mut types, argument);
                    }
                }
                _ => {}
            }
        }
    }
    types
}

/// Whether the module uses the fixed-width surface at all — a type other than
/// `i32`, or any value statement (whose `i32` arithmetic calls the shared
/// checked helpers). This is the one condition under which a module's
/// emission differs from the task route's historical bytes.
pub fn uses_fixed_width(module: &TaskModule) -> bool {
    !fixed_types(module).is_empty()
        || module.functions.values().any(|function| {
            function.blocks.values().any(|block| {
                block.instructions.iter().any(|instruction| {
                    matches!(instruction.kind, TaskInstructionKind::Compute { .. })
                }) || matches!(block.terminator.kind, TaskTerminatorKind::TryCompute { .. })
            })
        })
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
        TaskTerminatorKind::TryCompute { success, error, .. } => vec![*success, *error],
        _ => vec![],
    }
}

#[derive(Clone, Debug)]
enum AbstractValue {
    Scalar(ScalarType),
    /// An owned Buffer and its payload type (card `W48-QR-SCALAR-TYPES-R6`).
    /// The payload type is fixed at the identity's origin — an allocation's
    /// initializer, a parameter's declared type, a worker's declared result —
    /// and travels with every binding of it.
    Owned(ResourceId, st::ScalarType),
    /// Read authority over a resource this frame does not own. In a worker
    /// frame the lender is outside the frame entirely, so the identity is not
    /// registered in this frame's ledger and carries no discharge duty.
    Loan(ResourceId, st::ScalarType),
    /// Range write authority over a resource this frame does not own (card
    /// `WRITE-LEND-TASKS`). Like a loan it is unregistered in the frame's
    /// ledger and carries no discharge duty. Unlike a loan it is not
    /// *whole-payload* readable: it carries authority over a byte range, and
    /// since card `WRITE-LEND-TASKS-EXERCISE` the operations that reach it are
    /// the byte access pair, never the whole-payload read or write.
    Lease(ResourceId, st::ScalarType),
    /// A whole record — a record result a `join` collected — as its leaves in
    /// leaf order (card `W48-QR-TASK-RECORD-CAPTURES`). It lives only between
    /// the collection that binds it and the [`TaskInstructionKind::UnpackRecord`]
    /// that splits it: no operand reads it, and an exit that finds one live is
    /// refused, so its owned leaves are never discarded unseen.
    Record(usize, Vec<AbstractValue>),
}
impl AbstractValue {
    fn ty(&self) -> ScalarType {
        match self {
            Self::Scalar(ty) => *ty,
            Self::Owned(_, payload) => ScalarType::OwnedBuffer(*payload),
            Self::Loan(_, payload) => ScalarType::ReadLoan(*payload),
            Self::Lease(_, payload) => ScalarType::WriteLease(*payload),
            Self::Record(record, _) => ScalarType::Record(*record),
        }
    }
    /// The owned identities this value carries, in leaf order: one for an
    /// owned Buffer, each owned leaf's for a record, none otherwise.
    fn resources(&self) -> Vec<ResourceId> {
        match self {
            Self::Owned(id, _) => vec![*id],
            Self::Record(_, leaves) => leaves.iter().flat_map(Self::resources).collect(),
            _ => vec![],
        }
    }
    /// The payload type of a Buffer, a loan or a lease.
    fn payload(&self) -> Option<st::ScalarType> {
        self.ty().payload()
    }
    /// Owned identity only. A loan is never an owned identity, so it can never
    /// be returned, dropped or accounted for as one.
    fn resource(&self) -> Option<ResourceId> {
        match self {
            Self::Owned(id, _) => Some(*id),
            _ => None,
        }
    }
    /// The identity a **whole-payload** read is performed against, owned or
    /// borrowed. A write lease is deliberately not among them: it carries
    /// authority over a byte range, not over the whole payload. Its byte
    /// accesses go through [`State::authority`] instead.
    fn readable(&self) -> Option<ResourceId> {
        match self {
            Self::Owned(id, _) | Self::Loan(id, _) => Some(*id),
            Self::Scalar(_) | Self::Lease(..) | Self::Record(..) => None,
        }
    }
}
/// Which authority a byte access is performed under. The distinction is the
/// whole rebasing law: a lease's index counts from its own `low`, an owner's
/// from 0, and a read share carries no byte authority at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ByteAuthority {
    Owner,
    Lease,
    Loan,
}

/// A byte index is a **check-time constant or nothing**. An operand this
/// function cannot resolve to a literal is not provably inside the authority
/// it is offered against, so it is refused rather than approximated — the
/// sequential lend's `WRITE-LEND-COMPUTED-INDEX` arm, decided here in the IR
/// rather than by a route rule.
fn constant_index(state: &State, operand: &ScalarOperand) -> Result<u64, String> {
    match operand {
        ScalarOperand::I32(value) if *value >= 0 => Ok(*value as u64),
        ScalarOperand::I32(value) => Err(format!("a byte index cannot be negative: {value}")),
        other => {
            // Resolving the slot first keeps a stale or mistyped operand
            // reporting what it always reported; only a well-typed but
            // non-constant index reaches the refusal this card adds.
            state.value(other)?;
            Err(
                "a byte index must be a check-time constant, so a computed one is not \
                 provably inside its authority (WRITE-LEND-COMPUTED-INDEX)"
                    .into(),
            )
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
        /// Every owned identity the normal outcome hands back, in leaf order:
        /// none, one Buffer, or each owned leaf of a record result (card
        /// `W48-QR-TASK-RECORD-CAPTURES`).
        returned: Vec<ResourceId>,
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
    /// The owned identities a normal exit hands back, in leaf order: empty for
    /// a scalar or unit result, one for an owned Buffer, and one per owned
    /// leaf for a record result (card `W48-QR-TASK-RECORD-CAPTURES`).
    returned: Vec<WorkerReturn>,
    dropped: Vec<usize>,
    error: Option<String>,
    /// The owned Buffer this error exit carries out of the worker. It is
    /// derived from the worker's own checked exit, exactly like `returned`,
    /// and is `None` for every normal exit and every payload-free error.
    error_payload: Option<WorkerReturn>,
}
/// What one checked function tells its callers. The resource-effect exits are
/// card T04's; the **lease footprint** is card `WRITE-LEND-TASKS-EXERCISE`'s:
/// the exclusive bound `n` such that every byte this function touches through
/// its write lease lies in `[0, n)` of that lease. It is `None` when the
/// function exercises no lease, which is every function written before this
/// card. The creator's spawn site — the only place where a lease's real
/// `[low, high)` is known — decides the fit, so the rebasing law is checked
/// where both halves exist and nowhere else.
#[derive(Clone)]
struct FunctionSummary {
    exits: Vec<WorkerExit>,
    lease_footprint: Option<u64>,
    /// Card `W48-QR-SCALAR-TYPES-R4`: every copy-scalar slot's type.
    slot_types: BTreeMap<SlotId, ScalarType>,
    /// Card `W48-QR-SCALAR-TYPES-R6`: every Buffer, loan and lease slot's
    /// payload type.
    buffer_payloads: BTreeMap<SlotId, st::ScalarType>,
}
#[derive(Clone)]
struct State {
    slots: BTreeMap<SlotId, AbstractSlot>,
    ledger: ScopedTaskLedger,
    scopes: Vec<ScopeId>,
    next_id: u64,
    resources: BTreeSet<ResourceId>,
    /// The number of every copy-scalar slot whose value is known at check
    /// time on this path (card `W48-QR-SCALAR-TYPES-R4`): a literal binding, a
    /// copy of a known slot, or a value statement over known operands. A
    /// parameter, a buffer read and a joined outcome are known only at run
    /// time. It decides whether a value statement folds or raises.
    known: BTreeMap<SlotId, i128>,
    /// The module's recorded target (card `W48-QR-SCALAR-TYPES-R6`), which
    /// alone sizes an `isize`/`usize` payload.
    target: Option<RecordedTarget>,
    /// Every Buffer, loan and lease slot this path has bound, with its payload
    /// type (card `W48-QR-SCALAR-TYPES-R6`). Never removed on consumption, so
    /// the per-block record sees a slot bound and consumed inside one block.
    bound_payloads: BTreeMap<SlotId, st::ScalarType>,
}
impl State {
    /// The byte extent of a payload of type `payload` (card
    /// `W48-QR-SCALAR-TYPES-R6`). The fixed-width validation already refused
    /// a target width with no record, so a missing extent here is a malformed
    /// module and refuses rather than defaulting.
    fn extent(&self, payload: st::ScalarType) -> Result<u64, String> {
        payload_extent(payload, self.target.as_ref()).ok_or_else(|| {
            format!("a `{payload}` payload has no byte extent without the recorded target")
        })
    }
    fn value(&self, operand: &ScalarOperand) -> Result<AbstractValue, String> {
        match operand {
            ScalarOperand::Unit => Ok(AbstractValue::Scalar(ScalarType::Unit)),
            ScalarOperand::I32(_) => Ok(AbstractValue::Scalar(ScalarType::I32)),
            // Its range and representation were validated for the whole
            // function before any path was explored.
            ScalarOperand::Fixed { ty, .. } => Ok(AbstractValue::Scalar(ScalarType::Fixed(*ty))),
            ScalarOperand::Slot(id) => match self.slots.get(id) {
                Some(AbstractSlot::Value(value @ AbstractValue::Scalar(_))) => Ok(value.clone()),
                _ => Err(format!("slot {id} is not an initialized copy scalar")),
            },
            ScalarOperand::Move(id) => match self.slots.get(id) {
                Some(AbstractSlot::Value(value @ AbstractValue::Owned(resource, _))) => {
                    self.ledger
                        .use_resource(0, *resource)
                        .map_err(|e| format!("{e:?}"))?;
                    Ok(value.clone())
                }
                _ => Err(format!("slot {id} is not an initialized owned buffer")),
            },
            // Lending demands ownership plus enough unlent authority. A
            // resource that was moved away, reserved or dropped is not lendable
            // at all, and one whose outstanding shares leave no room is refused
            // with the exact amounts. A loan itself is never lendable: this
            // profile has no transitive or nested borrowing.
            ScalarOperand::Loan { slot, share } => match self.slots.get(slot) {
                Some(AbstractSlot::Value(AbstractValue::Owned(resource, payload))) => {
                    if *share == 0 || *share > FULL_READ_AUTHORITY {
                        return Err(format!(
                            "a read share must be positive and at most {FULL_READ_AUTHORITY}"
                        ));
                    }
                    Ok(AbstractValue::Loan(*resource, *payload))
                }
                _ => Err(format!(
                    "slot {slot} is not an owned buffer this task may lend"
                )),
            },
            // Leasing demands ownership; the bounds are re-decided here so a
            // range no byte of the payload backs cannot reach the ledger, and
            // the ledger separately decides disjointness against everything
            // outstanding. A lease of anything but an owned buffer — a
            // scalar, a loan, a lease — is refused: this profile has no
            // transitive or nested leasing.
            ScalarOperand::Lease { slot, low, high } => match self.slots.get(slot) {
                Some(AbstractSlot::Value(AbstractValue::Owned(resource, payload))) => {
                    if *low >= *high {
                        return Err(format!(
                            "a write lease must cover at least one byte: [{low}, {high}) is empty"
                        ));
                    }
                    // Card `W48-QR-SCALAR-TYPES-R6`: the extent is the
                    // payload type's size, never an assumed four; an `i32`
                    // payload keeps the message it always had.
                    let extent = self.extent(*payload)?;
                    if *high > extent {
                        return Err(if *payload == st::ScalarType::I32 {
                            format!(
                                "a write lease must stay inside the payload's {BUFFER_PAYLOAD_BYTES} \
                                 bytes: [{low}, {high}) ends past it"
                            )
                        } else {
                            format!(
                                "a write lease must stay inside the `{payload}` payload's {extent} \
                                 byte(s): [{low}, {high}) ends past it"
                            )
                        });
                    }
                    Ok(AbstractValue::Lease(*resource, *payload))
                }
                _ => Err(format!(
                    "slot {slot} is not an owned buffer this task may lease for writing"
                )),
            },
            // Card `W48-QR-TASK-RECORD-CAPTURES`: a whole record is an operand
            // of a record-valued `return` only, which takes its leaves itself.
            ScalarOperand::Record(_) => {
                Err("a whole record is an operand only of a record-valued return".into())
            }
        }
    }
    /// The number a copy-scalar operand is known to hold at check time, if any.
    fn known_number(&self, operand: &ScalarOperand) -> Option<i128> {
        match operand {
            ScalarOperand::I32(value) => Some(i128::from(*value)),
            ScalarOperand::Fixed { value, .. } => Some(*value),
            ScalarOperand::Slot(slot) => self.known.get(slot).copied(),
            _ => None,
        }
    }
    /// A value-statement operand as the shared decision sees it.
    fn value_operand(&self, operand: &TaskValueOperand) -> Result<scalar_values::Operand, String> {
        match operand {
            TaskValueOperand::Literal { ty, value } => Ok(scalar_values::Operand::Typed {
                ty: *ty,
                known: Some(*value),
            }),
            TaskValueOperand::Slot(slot) => {
                let ty = match self.value(&ScalarOperand::Slot(*slot))? {
                    AbstractValue::Scalar(ScalarType::I32) => st::ScalarType::I32,
                    AbstractValue::Scalar(ScalarType::Fixed(ty)) => ty,
                    _ => {
                        return Err(format!(
                            "slot {slot} is not a value of the fixed-width table, so no value \
                             statement reads it"
                        ));
                    }
                };
                Ok(scalar_values::Operand::Typed {
                    ty,
                    known: self.known.get(slot).copied(),
                })
            }
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
            AbstractValue::Owned(id, _) => Ok(id),
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
        if matches!(value, AbstractValue::Owned(..)) {
            self.ledger
                .read_resource(0, resource)
                .map_err(|e| format!("{e:?}"))?;
        }
        Ok(resource)
    }
    /// Classify the authority a byte access names. A slot that is none of the
    /// three keeps the original owned-buffer refusal verbatim, so a stale,
    /// moved-away or scalar slot reports what it always reported.
    fn authority(&self, slot: SlotId) -> Result<ByteAuthority, String> {
        match self.slots.get(&slot) {
            Some(AbstractSlot::Value(AbstractValue::Owned(..))) => Ok(ByteAuthority::Owner),
            Some(AbstractSlot::Value(AbstractValue::Lease(..))) => Ok(ByteAuthority::Lease),
            Some(AbstractSlot::Value(AbstractValue::Loan(..))) => Ok(ByteAuthority::Loan),
            _ => Err(format!("slot {slot} is not an initialized owned buffer")),
        }
    }
    fn bind(&mut self, id: SlotId, value: AbstractSlot) -> Result<(), String> {
        if self.slots.contains_key(&id) {
            return Err(format!("slot {id} is written more than once"));
        }
        if let AbstractSlot::Value(value) = &value
            && let Some(payload) = value.payload()
        {
            self.bound_payloads.insert(id, payload);
        }
        self.slots.insert(id, value);
        Ok(())
    }
    fn exit(&self, returned: &[ResourceId]) -> Result<(), String> {
        if !self.scopes.is_empty() {
            return Err("return/error leaves an open thread scope".into());
        }
        if self.slots.values().any(|s| {
            matches!(
                s,
                AbstractSlot::Handle { .. }
                    | AbstractSlot::Result { .. }
                    | AbstractSlot::Value(AbstractValue::Owned(..))
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: a collected record
                    // not yet unpacked is a value this exit would discard.
                    | AbstractSlot::Value(AbstractValue::Record(..))
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
        // An outstanding share is an unfinished lifetime duty, not a leak: name
        // it separately so the refusal says what the exit actually failed to
        // do, which is reclaim the share by joining its borrower.
        if let Some((resource, _)) = self.outstanding_lent_resource() {
            return Err(format!(
                "return/error leaves resource {resource} lent to an unjoined child"
            ));
        }
        // An outstanding write lease is the same unfinished duty on the write
        // side: only the creator's join returns the leased bytes.
        if let Some((resource, _)) = self.outstanding_leased_resource() {
            return Err(format!(
                "return/error leaves resource {resource} write-leased to an unjoined child"
            ));
        }
        for &resource in &self.resources {
            if !returned.contains(&resource)
                && self.ledger.resource_state(resource) != Some(ResourceState::Dropped)
            {
                return Err("return/error leaves unaccounted resource custody".into());
            }
        }
        Ok(())
    }
    /// The exit rule's own loan judgment — the one `exit` names "lent to an
    /// unjoined child": the first frame resource with an outstanding read
    /// share, plus its borrowers. A borrower with an outstanding share is
    /// unjoined by the ledger's own transaction (`join` reclaims the child's
    /// shares in the same transaction that consumes the handle, and a failed
    /// join keeps its scope inexitable), so the borrowers named here are
    /// exactly the unjoined children whose joins this frame still owes.
    fn outstanding_lent_resource(&self) -> Option<(ResourceId, Vec<TaskId>)> {
        self.resources
            .iter()
            .find(|&&resource| self.ledger.lent_share(resource) > 0)
            .map(|&resource| (resource, self.ledger.loan_borrowers(resource)))
    }
    /// The write-side mirror of [`Self::outstanding_lent_resource`]: the first
    /// frame resource with an outstanding range write lease, plus its holders
    /// (card `WRITE-LEND-TASKS`).
    fn outstanding_leased_resource(&self) -> Option<(ResourceId, Vec<TaskId>)> {
        self.resources
            .iter()
            .find(|&&resource| !self.ledger.leased_ranges(resource).is_empty())
            .map(|&resource| (resource, self.ledger.lease_holders(resource)))
    }
    /// Render a refused scope exit (card CT24-CLEANUP-PREEMPTED). The ledger's
    /// `ScopeBusy` obligation is neither weakened nor reordered — it still
    /// refuses exactly the exits it refused, at the same instruction and span,
    /// and the message keeps its `ScopeBusy(..)` prefix. What changes is the
    /// attribution: when the busy scope's preempted teardown would run this
    /// frame's cleanup over an owner a live child still borrows — the
    /// construct CT24 names, cleanup destroying an owner before the child's
    /// join — the refusal carries that fact, read from the same loan judgment
    /// the `exit` backstop reads instead of being discarded in favour of the
    /// bare debug rendering. A busy scope with no lent frame resource (an
    /// unjoined child holding no loan) keeps the bare rendering byte for byte.
    fn scope_exit_refusal(&self, error: TaskLedgerError) -> String {
        if matches!(error, TaskLedgerError::ScopeBusy(_))
            && let Some((resource, borrowers)) = self.outstanding_lent_resource()
        {
            return format!(
                "{error:?}: cleanup preempted — this exit would run the frame's \
                 cleanup over resource {resource} while task(s) {borrowers:?} still \
                 hold a read share of it, destroying the owner before the child's \
                 join; the scope refuses the exit with that loan outstanding"
            );
        }
        format!("{error:?}")
    }
}

fn check_function(
    module: &TaskModule,
    id: FunctionId,
    summaries: &BTreeMap<FunctionId, FunctionSummary>,
) -> Result<FunctionSummary, TaskDiagnostic> {
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
                TaskInstructionKind::BindScalar { slot, .. } => vec![*slot],
                TaskInstructionKind::ReadBuffer { value_slot, .. } => vec![*value_slot],
                TaskInstructionKind::UnwrapInfallible { value_slot, .. } => vec![*value_slot],
                TaskInstructionKind::TakeErrorPayload { buffer_slot, .. } => vec![*buffer_slot],
                TaskInstructionKind::Compute { slot, .. } => vec![*slot],
                // Card `W48-QR-TASK-RECORD-CAPTURES`.
                TaskInstructionKind::UnpackRecord { leaves, .. } => leaves.clone(),
                TaskInstructionKind::RefillField { slot, .. } => vec![*slot],
                TaskInstructionKind::Join { outcome_slot, .. } => {
                    effects.join = true;
                    effects.may_block = true;
                    outcome_slot.iter().copied().collect()
                }
                _ => vec![],
            };
            if defined.into_iter().any(|slot| !definitions.insert(slot)) {
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
            TaskTerminatorKind::TryCompute {
                slot, error_slot, ..
            } => vec![*slot, *error_slot],
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
    if id == module.entry && matches!(function.result, ScalarType::OwnedBuffer(_)) {
        return Err(fail(
            function.span,
            "owned task entry results have no external custodian in this profile".into(),
        ));
    }
    // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result names a declared
    // record whose every leaf is a copy scalar or an owned Buffer, and it is a
    // worker's result — the entry's result is published, never collected.
    // No parameter is a record: a record parameter is its leaves.
    if let ScalarType::Record(record) = function.result {
        let Some(declared) = module.records.get(&record) else {
            return Err(fail(
                function.span,
                format!("record result {record} names no declared record"),
            ));
        };
        if declared.leaves.is_empty()
            || declared.leaves.iter().any(|leaf| {
                !matches!(
                    leaf.ty,
                    ScalarType::I32 | ScalarType::Fixed(_) | ScalarType::OwnedBuffer(_)
                )
            })
        {
            return Err(fail(
                function.span,
                format!(
                    "record `{}`'s leaves must be copy scalars or owned Buffers",
                    declared.name
                ),
            ));
        }
        if id == module.entry {
            return Err(fail(
                function.span,
                "a task entry returns i32, unit or its error; a record is returned by a \
                 worker and collected by join"
                    .into(),
            ));
        }
    }
    if function
        .parameters
        .iter()
        .any(|(_, kind)| matches!(kind, ScalarType::Record(_)))
    {
        return Err(fail(
            function.span,
            "a record parameter is its leaves; no parameter is a whole record".into(),
        ));
    }
    // A read loan is bound to the creation that opened it. Returning one would
    // outlive that creation's join, so it is refused at the type, before any
    // path is explored. A write lease is bound the same way, and refused the
    // same way.
    if matches!(function.result, ScalarType::ReadLoan(_)) {
        return Err(fail(
            function.span,
            "a read loan cannot be a task function result".into(),
        ));
    }
    if matches!(function.result, ScalarType::WriteLease(_)) {
        return Err(fail(
            function.span,
            "a write lease cannot be a task function result".into(),
        ));
    }
    if id == module.entry
        && function
            .parameters
            .iter()
            .any(|(_, kind)| matches!(kind, ScalarType::ReadLoan(_)))
    {
        return Err(fail(
            function.span,
            "the task entry has no lender, so it cannot take a read loan".into(),
        ));
    }
    if id == module.entry
        && function
            .parameters
            .iter()
            .any(|(_, kind)| matches!(kind, ScalarType::WriteLease(_)))
    {
        return Err(fail(
            function.span,
            "the task entry has no lessor, so it cannot take a write lease".into(),
        ));
    }
    // Card `W48-QR-SCALAR-TYPES-R4`: the fixed-width surface of this
    // function, validated once for every path — each type has one
    // representation, each literal is a value of its type, and `isize`/`usize`
    // have a width only under the recorded target.
    validate_fixed_width(module, function).map_err(|message| fail(function.span, message))?;
    if id == module.entry && matches!(function.result, ScalarType::Fixed(_)) {
        return Err(fail(
            function.span,
            "a task entry returns i32, unit or its error; a fixed-width value is \
             returned by a worker and collected by join"
                .into(),
        ));
    }
    // The error row's permission to raise `ArithmeticError`: every variant of
    // the builtin type, which `throws<ArithmeticError>` declares together.
    let raises_permitted = ArithmeticError::ALL
        .iter()
        .all(|error| function.errors.contains(&error.to_string()));
    let comparisons = function.declared_effects.comparisons;
    let target = module.target.as_ref();
    let mut state = State {
        slots: BTreeMap::new(),
        ledger: ScopedTaskLedger::default(),
        scopes: vec![],
        next_id: 1,
        resources: BTreeSet::new(),
        known: BTreeMap::new(),
        target: module.target.clone(),
        bound_payloads: BTreeMap::new(),
    };
    let mut inputs = BTreeMap::new();
    for (index, &(slot, ty)) in function.parameters.iter().enumerate() {
        let value = match ty {
            ScalarType::OwnedBuffer(payload) => {
                let resource = state.next_id;
                state.next_id += 1;
                state
                    .ledger
                    .acquire(0, resource)
                    .map_err(|e| fail(function.span, format!("{e:?}")))?;
                state.resources.insert(resource);
                inputs.insert(resource, index);
                AbstractValue::Owned(resource, payload)
            }
            // A loan is not registered in this frame's ledger and is not an
            // `input`: the borrower owes nothing at exit, and no worker exit
            // can report it as returned or dropped. A lease is on the same
            // footing.
            ScalarType::ReadLoan(payload) => {
                let resource = state.next_id;
                state.next_id += 1;
                AbstractValue::Loan(resource, payload)
            }
            ScalarType::WriteLease(payload) => {
                let resource = state.next_id;
                state.next_id += 1;
                AbstractValue::Lease(resource, payload)
            }
            ScalarType::I32 | ScalarType::Unit | ScalarType::Fixed(_) => AbstractValue::Scalar(ty),
            // Refused above, before any path: a record parameter is its leaves.
            ScalarType::Record(_) => {
                return Err(fail(
                    function.span,
                    "a record parameter is its leaves; no parameter is a whole record".into(),
                ));
            }
        };
        if let Some(payload) = value.payload() {
            state.bound_payloads.insert(slot, payload);
        }
        state.slots.insert(slot, AbstractSlot::Value(value));
    }
    // At most one lease parameter: a creation supplies at most one lease, so a
    // second lease parameter could never be filled, and `lease_footprint`
    // could not say which lease it belongs to. Refusing here keeps the
    // summary's single-authority reading a fact rather than a convention.
    if function
        .parameters
        .iter()
        .filter(|(_, kind)| matches!(kind, ScalarType::WriteLease(_)))
        .count()
        > 1
    {
        return Err(fail(
            function.span,
            "a task function takes at most one write lease".into(),
        ));
    }
    let mut lease_footprint: Option<u64> = None;
    let mut exits = vec![];
    let mut queue = vec![(function.entry, state)];
    let mut reached = BTreeSet::new();
    let mut scope_stacks = BTreeMap::<BlockId, Vec<ScopeId>>::new();
    let mut count = 0usize;
    let mut slot_types = BTreeMap::<SlotId, ScalarType>::new();
    let mut buffer_payloads = BTreeMap::<SlotId, st::ScalarType>::new();
    let record = |state: &State,
                  (slot_types, buffer_payloads): (
        &mut BTreeMap<SlotId, ScalarType>,
        &mut BTreeMap<SlotId, st::ScalarType>,
    )|
     -> Result<(), String> {
        for (slot, payload) in &state.bound_payloads {
            if buffer_payloads
                .insert(*slot, *payload)
                .is_some_and(|earlier| earlier != *payload)
            {
                return Err(format!("slot {slot} has two payload types on two paths"));
            }
        }
        for (slot, value) in &state.slots {
            if let AbstractSlot::Value(AbstractValue::Scalar(ty)) = value
                && *ty != ScalarType::Unit
                && slot_types
                    .insert(*slot, *ty)
                    .is_some_and(|earlier| earlier != *ty)
            {
                return Err(format!("slot {slot} has two types on two paths"));
            }
        }
        Ok(())
    };
    while let Some((block_id, mut state)) = queue.pop() {
        record(&state, (&mut slot_types, &mut buffer_payloads))
            .map_err(|message| fail(function.span, message))?;
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
                    TaskInstructionKind::BindScalar {
                        slot,
                        value: operand,
                    } => {
                        let known = state.known_number(operand);
                        let value = state.take(operand)?;
                        state.bind(*slot, AbstractSlot::Value(value))?;
                        if let Some(known) = known {
                            state.known.insert(*slot, known);
                        }
                    }
                    TaskInstructionKind::Compute { slot, operation } => {
                        let decision =
                            decide_value(&state, operation, raises_permitted, comparisons, target)?;
                        if decision.raises {
                            return Err(
                                "this value statement reads a value known only at run time and \
                                 can raise ArithmeticError, so it must take the checked form \
                                 whose error edge raises it"
                                    .into(),
                            );
                        }
                        state.bind(
                            *slot,
                            AbstractSlot::Value(AbstractValue::Scalar(task_type(decision.ty))),
                        )?;
                        if let Some(known) = decision.known {
                            state.known.insert(*slot, known);
                        }
                    }
                    TaskInstructionKind::ReadBuffer { buffer, value_slot } => {
                        state.readable(*buffer)?;
                        // Card `W48-QR-SCALAR-TYPES-R6`: a read yields a value
                        // of the payload type — `i32` for every Buffer before.
                        let payload = state
                            .slots
                            .get(buffer)
                            .and_then(|slot| match slot {
                                AbstractSlot::Value(value) => value.payload(),
                                _ => None,
                            })
                            .unwrap_or(st::ScalarType::I32);
                        state.bind(
                            *value_slot,
                            AbstractSlot::Value(AbstractValue::Scalar(task_type(payload))),
                        )?;
                    }
                    TaskInstructionKind::WriteBuffer { buffer, value } => {
                        let payload = match state.slots.get(buffer) {
                            Some(AbstractSlot::Value(value)) => value.payload(),
                            _ => None,
                        };
                        let written = state.value(value)?.ty();
                        match payload {
                            Some(payload) if payload != st::ScalarType::I32 => {
                                if written != task_type(payload) {
                                    return Err(format!(
                                        "a write into a `{payload}` payload requires a copy \
                                         `{payload}`: the payload type is part of the Buffer's \
                                         type and nothing converts implicitly"
                                    ));
                                }
                            }
                            _ => {
                                if written != ScalarType::I32 {
                                    return Err("buffer write requires a copy i32".into());
                                }
                            }
                        }
                        // `buffer` demands exclusive authority, which the share
                        // ledger refuses while any read share is outstanding.
                        // A borrower's slot is a loan, never an owned buffer,
                        // so no borrower can reach this instruction at all.
                        state.buffer(*buffer)?;
                    }
                    TaskInstructionKind::WriteBufferByte {
                        authority,
                        index,
                        value,
                    } => {
                        if state.value(value)?.ty() != ScalarType::I32 {
                            return Err("a byte write requires a copy i32 value".into());
                        }
                        let index = constant_index(&state, index)?;
                        // Card `W48-QR-SCALAR-TYPES-R6`: every byte of an
                        // integer payload is a byte of some value of its type,
                        // but a `bool` is one byte whose only values are 0 and
                        // 1 — any other written byte would leave the payload
                        // holding no `bool` at all.
                        if let Some(AbstractSlot::Value(AbstractValue::Lease(
                            _,
                            st::ScalarType::Bool,
                        ))) = state.slots.get(authority)
                        {
                            return Err("a byte write into a `bool` payload is refused: a byte \
                                 other than 0 or 1 is not a `bool`, so the byte view of a \
                                 `bool` payload is read-only"
                                .into());
                        }
                        match state.authority(*authority)? {
                            ByteAuthority::Lease => {
                                lease_footprint = Some(lease_footprint.unwrap_or(0).max(index + 1));
                            }
                            // The owner's whole-payload write stays its only
                            // write footprint in this slice: admitting an
                            // owner byte write would add a second owner-side
                            // footprint that card `WRITE-LEND-TASKS`' "the
                            // owner holds no whole-payload authority while a
                            // lease is out" rule was never measured against.
                            ByteAuthority::Owner => {
                                return Err("the whole-payload write is the owner's only write \
                                     footprint; a byte write requires a lend<Buffer> \
                                     (WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE)"
                                    .into());
                            }
                            ByteAuthority::Loan => {
                                return Err(
                                    "a read share grants no write authority, byte-wise or \
                                     whole-payload"
                                        .into(),
                                );
                            }
                        }
                    }
                    TaskInstructionKind::ReadBufferByte {
                        authority,
                        index,
                        value_slot,
                    } => {
                        let index = constant_index(&state, index)?;
                        match state.authority(*authority)? {
                            ByteAuthority::Lease => {
                                lease_footprint = Some(lease_footprint.unwrap_or(0).max(index + 1));
                            }
                            ByteAuthority::Owner => {
                                // Exactly the whole-payload read's gate, so a
                                // read while any lease is outstanding stays
                                // refused by the ledger rather than by a rule
                                // written a second time here.
                                state.readable(*authority)?;
                                // Card `W48-QR-SCALAR-TYPES-R6`: the payload
                                // type's own extent.
                                let payload = match state.slots.get(authority) {
                                    Some(AbstractSlot::Value(value)) => value.payload(),
                                    _ => None,
                                }
                                .unwrap_or(st::ScalarType::I32);
                                let extent = state.extent(payload)?;
                                if index >= extent {
                                    return Err(format!(
                                        "byte {index} is outside the payload's \
                                         [0, {extent}) extent"
                                    ));
                                }
                            }
                            ByteAuthority::Loan => {
                                return Err("a read share admits the whole-payload read only \
                                     (WRITE-LEND-EXERCISE-LOAN-BYTE-READ)"
                                    .into());
                            }
                        }
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
                        // A declared error payload is an `i32`-payload Buffer:
                        // a typed error payload is refused at its raise below.
                        state.bind(
                            *buffer_slot,
                            AbstractSlot::Value(AbstractValue::Owned(payload, st::ScalarType::I32)),
                        )?;
                    }
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: each leaf slot takes
                    // exactly the leaf the record held; the record is consumed,
                    // so an owned leaf changes binding and never duplicates.
                    TaskInstructionKind::UnpackRecord { record, leaves } => {
                        let Some(AbstractSlot::Value(AbstractValue::Record(_, values))) =
                            state.slots.get(record).cloned()
                        else {
                            return Err(format!(
                                "slot {record} is not a collected record to unpack"
                            ));
                        };
                        if values.len() != leaves.len() {
                            return Err(format!(
                                "unpacking a record of {} leaves into {} slots",
                                values.len(),
                                leaves.len()
                            ));
                        }
                        state.slots.insert(*record, AbstractSlot::Consumed);
                        for (slot, value) in leaves.iter().zip(values) {
                            state.bind(*slot, AbstractSlot::Value(value))?;
                        }
                    }
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: a refill is admitted
                    // exactly into a hole — the leaf's Buffer moved away or
                    // released on this path — and the Buffer moved in has the
                    // leaf's payload type.
                    TaskInstructionKind::RefillField { hole, slot, value } => {
                        match state.slots.get(hole) {
                            Some(AbstractSlot::Consumed) => {}
                            Some(AbstractSlot::Value(AbstractValue::Owned(resource, _))) => {
                                return Err(format!(
                                    "OverwriteOfLiveField: the field still holds resource \
                                     {resource}; a refill moves a Buffer into a hole only, so \
                                     release or move the field's Buffer first"
                                ));
                            }
                            _ => {
                                return Err(format!(
                                    "slot {hole} is not an owned field of a record this task \
                                     holds"
                                ));
                            }
                        }
                        if !matches!(value, ScalarOperand::Move(_)) {
                            return Err("a refill moves exactly one owned Buffer in".into());
                        }
                        let expected = state.bound_payloads.get(hole).copied();
                        let moved = state.take(value)?;
                        match (&moved, expected) {
                            (AbstractValue::Owned(_, payload), Some(expected))
                                if *payload == expected => {}
                            (AbstractValue::Owned(_, payload), Some(expected)) => {
                                return Err(format!(
                                    "a refill of a `{expected}` field requires an \
                                     own<Buffer<{expected}>>, not a `{payload}` payload"
                                ));
                            }
                            _ => {
                                return Err(format!(
                                    "slot {hole} was never an owned field of this task"
                                ));
                            }
                        }
                        state.bind(*slot, AbstractSlot::Value(moved))?;
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
                            .map_err(|error| state.scope_exit_refusal(error))?;
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
                                .iter()
                                .copied()
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
                            let value = if normal {
                                Some(match (returned.as_slice(), worker.result) {
                                    // Card `W48-QR-TASK-RECORD-CAPTURES`: a record
                                    // result, its owned leaves exactly the
                                    // identities the completion carried, in
                                    // leaf order.
                                    (_, ScalarType::Record(record)) => {
                                        record_value(module, record, &returned)?
                                    }
                                    ([resource], ScalarType::OwnedBuffer(payload)) => {
                                        AbstractValue::Owned(*resource, payload)
                                    }
                                    ([resource], _) => {
                                        AbstractValue::Owned(*resource, st::ScalarType::I32)
                                    }
                                    ([], result) => AbstractValue::Scalar(result),
                                    _ => {
                                        return Err("a non-record outcome carries at most one \
                                             owned identity"
                                            .into());
                                    }
                                })
                            } else {
                                None
                            };
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
        record(&state, (&mut slot_types, &mut buffer_payloads))
            .map_err(|message| fail(block.terminator.span, message))?;
        let result: Result<(), String> = (|| {
            match &block.terminator.kind {
                TaskTerminatorKind::Goto { target } => queue.push((*target, state)),
                TaskTerminatorKind::TryCompute {
                    slot,
                    operation,
                    error_slot,
                    success,
                    error,
                } => {
                    let decision =
                        decide_value(&state, operation, raises_permitted, comparisons, target)?;
                    if !decision.raises {
                        return Err(
                            "this value statement cannot raise ArithmeticError here — its \
                             operands are known at check time, or the operation cannot fault — \
                             so it takes no error edge"
                                .into(),
                        );
                    }
                    let variants = raisable_variants(&state, operation)?;
                    let mut failed = state.clone();
                    failed.bind(
                        *error_slot,
                        AbstractSlot::Error {
                            errors: variants,
                            payload: None,
                        },
                    )?;
                    queue.push((*error, failed));
                    state.bind(
                        *slot,
                        AbstractSlot::Value(AbstractValue::Scalar(task_type(decision.ty))),
                    )?;
                    queue.push((*success, state));
                }
                TaskTerminatorKind::TryAllocateBuffer {
                    initial,
                    value_slot,
                    success,
                    allocation_error,
                } => {
                    // Card `W48-QR-SCALAR-TYPES-R6`: the initializer's type is
                    // the payload type "named at acquisition" — an `i32` for
                    // every Buffer before, any §G02t type now.
                    let payload = match state.value(initial)?.ty() {
                        ScalarType::I32 => st::ScalarType::I32,
                        ScalarType::Fixed(ty) => ty,
                        _ => {
                            return Err("buffer initialization requires a copy i32".into());
                        }
                    };
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
                        AbstractSlot::Value(AbstractValue::Owned(resource, payload)),
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
                    let mut leases = vec![];
                    let mut captured = BTreeMap::new();
                    let mut consumed_slots = BTreeSet::new();
                    let mut lent_slots = BTreeSet::new();
                    let mut leased_slots = BTreeSet::new();
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
                        if let AbstractValue::Loan(resource, _) = &value {
                            let ScalarOperand::Loan { slot, share } = argument else {
                                unreachable!()
                            };
                            if !lent_slots.insert(*slot) {
                                return Err("duplicate read loan of one slot".into());
                            }
                            loans.push((*resource, *share));
                        }
                        if let AbstractValue::Lease(resource, _) = &value {
                            let ScalarOperand::Lease { slot, low, high } = argument else {
                                unreachable!()
                            };
                            if !leased_slots.insert(*slot) {
                                return Err("duplicate write lease of one slot".into());
                            }
                            leases.push((
                                *resource,
                                LeaseRange {
                                    low: *low,
                                    high: *high,
                                },
                            ));
                        }
                    }
                    // The card's bounds, enforced structurally rather than
                    // assumed: one loan argument per creation, and one lent
                    // resource at a time. How much authority may be handed out,
                    // and to how many borrowers, is the ledger's decision.
                    if loans.len() > 1 {
                        return Err(
                            "this profile admits at most one read loan per task creation".into(),
                        );
                    }
                    // Card `WRITE-LEND-TASKS`'s bounds, on the loans' pattern:
                    // one lease per creation, one write-leased resource at a
                    // time, and no mixing of the two transfer families across
                    // outstanding creations. Which *ranges* may be out at once
                    // is the ledger's decision, by the wave-29 overlap rule.
                    if leases.len() > 1 {
                        return Err(
                            "this profile admits at most one write lease per task creation".into(),
                        );
                    }
                    if !loans.is_empty() && !leases.is_empty() {
                        return Err(
                            "one creation cannot both lend a read share and lease a write range"
                                .into(),
                        );
                    }
                    if loans.iter().any(|(resource, _)| {
                        state
                            .ledger
                            .outstanding_loans()
                            .iter()
                            .any(|outstanding| outstanding != resource)
                    }) {
                        return Err(
                            "this profile admits at most one outstanding read loan at a time"
                                .into(),
                        );
                    }
                    // Card `W48-QR-TASK-RECORD-CAPTURES` lifts card
                    // `WRITE-LEND-TASKS`' "at most one write-leased resource at
                    // a time" to a **per-resource** rule: the ledger keys every
                    // lease by its resource and decides disjointness per
                    // resource, and two resources are two allocations whose
                    // bytes never coincide, so a lease of one field and a
                    // lease of another — two leaves, two resources — hold no
                    // byte in common. What stays is every per-resource bound:
                    // pairwise-disjoint ranges, `MAX_WRITERS` holders, no
                    // owner authority while any lease of that resource is out
                    // (`QCore.TaskRecordCaptures.no_byte_has_two_write_authorities_per_leaf`).
                    if (!leases.is_empty() && !state.ledger.outstanding_loans().is_empty())
                        || (!loans.is_empty()
                            && !state.ledger.outstanding_write_leases().is_empty())
                    {
                        return Err(
                            "this profile does not mix outstanding read loans and write \
                                    leases"
                                .into(),
                        );
                    }
                    if resources
                        .iter()
                        .any(|resource| loans.iter().any(|(lent, _)| lent == resource))
                    {
                        return Err("one creation cannot both move and lend a resource".into());
                    }
                    if resources
                        .iter()
                        .any(|resource| leases.iter().any(|(leased, _)| leased == resource))
                    {
                        return Err("one creation cannot both move and lease a resource".into());
                    }
                    let reservation = state.next_id;
                    state.next_id += 1;
                    state
                        .ledger
                        .reserve_with_transfers(0, *scope, reservation, &resources, &loans, &leases)
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
                    let worker_summary = summaries
                        .get(worker)
                        .ok_or("worker has no checked resource-effect summary")?;
                    // The rebasing law, decided where both halves are known:
                    // the worker states the exclusive bound it touches through
                    // its lease, and this creation supplies the lease's real
                    // `[low, high)`. Byte `low + i` is authorized exactly when
                    // `i < high - low`, so the whole footprint fits exactly
                    // when `bound <= high - low`. A worker that exercises a
                    // lease it was never handed is refused here too.
                    if let Some(bound) = worker_summary.lease_footprint {
                        let Some((_, range)) = leases.first() else {
                            return Err(format!(
                                "worker exercises bytes [0, {bound}) of a write lease this \
                                 creation does not supply"
                            ));
                        };
                        let extent = range.high.saturating_sub(range.low);
                        if bound > extent {
                            return Err(format!(
                                "worker exercises bytes [0, {bound}) of its lease, which does \
                                 not fit the leased range [{}, {}) of extent {extent}",
                                range.low, range.high
                            ));
                        }
                    }
                    for exit in &worker_summary.exits {
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
                            |carried: Option<&WorkerReturn>,
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
                        // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result
                        // hands back one identity per owned leaf, each resolved
                        // exactly as a single owned result is.
                        let mut returned = Vec::with_capacity(exit.returned.len());
                        for carried in &exit.returned {
                            returned.push(
                                resolve(Some(carried), &mut success_state)?
                                    .ok_or("a returned identity did not resolve")?,
                            );
                        }
                        let error_payload =
                            resolve(exit.error_payload.as_ref(), &mut success_state)?;
                        if !returned.is_empty() && error_payload.is_some() {
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
                                // Like `I32(0)`, a placeholder: the abstract
                                // run tracks the type, not the number, and a
                                // scalar outcome carries no custody.
                                ScalarType::Fixed(ty) => TaskOutcome::Fixed { ty, value: 0 },
                                ScalarType::OwnedBuffer(_) => TaskOutcome::OwnedBuffer(
                                    *returned
                                        .first()
                                        .ok_or("owned worker outcome has no checked identity")?,
                                ),
                                ScalarType::ReadLoan(_) => {
                                    return Err("a read loan is not a worker outcome".into());
                                }
                                ScalarType::WriteLease(_) => {
                                    return Err("a write lease is not a worker outcome".into());
                                }
                                // Card `W48-QR-TASK-RECORD-CAPTURES`: placeholder
                                // scalars, as for `I32(0)`, and every owned leaf's
                                // checked identity.
                                ScalarType::Record(record) => {
                                    abstract_record_outcome(module, record, &returned)?
                                }
                            }
                        };
                        success_state
                            .ledger
                            .complete(
                                reservation,
                                outcome,
                                &returned
                                    .iter()
                                    .copied()
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
                    // Card `W48-QR-TASK-RECORD-CAPTURES`: "owned" is "carries an
                    // identity", which a record carries when any leaf is owned;
                    // for every value written before the card the two agree.
                    if !errors.is_empty()
                        || (!declared_payloads
                            && value.as_ref().is_some_and(|v| v.resources().is_empty()))
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
                    let value = match (value, function.result) {
                        // Card `W48-QR-TASK-RECORD-CAPTURES`: a record-valued
                        // return takes each leaf operand — a copy of a scalar
                        // leaf, a move of an owned one — against the declared
                        // leaf types, in leaf order.
                        (ScalarOperand::Record(operands), ScalarType::Record(record)) => {
                            let declared = module.records.get(&record).ok_or_else(|| {
                                format!("record result {record} names no declared record")
                            })?;
                            if operands.len() != declared.leaves.len() {
                                return Err(format!(
                                    "record `{}` has {} leaves and this return supplies {}",
                                    declared.name,
                                    declared.leaves.len(),
                                    operands.len()
                                ));
                            }
                            let mut leaves = Vec::with_capacity(operands.len());
                            for (operand, leaf) in operands.iter().zip(&declared.leaves) {
                                if matches!(leaf.ty, ScalarType::OwnedBuffer(_))
                                    != matches!(operand, ScalarOperand::Move(_))
                                {
                                    return Err(format!(
                                        "record `{}`'s leaf `{}` returns by {}",
                                        declared.name,
                                        leaf.path,
                                        if matches!(leaf.ty, ScalarType::OwnedBuffer(_)) {
                                            "move of its owned Buffer"
                                        } else {
                                            "copy of its scalar"
                                        }
                                    ));
                                }
                                let taken = state.take(operand)?;
                                if taken.ty() != leaf.ty {
                                    return Err(format!(
                                        "record `{}`'s leaf `{}` has type {:?}, and this \
                                         return supplies {:?}",
                                        declared.name,
                                        leaf.path,
                                        leaf.ty,
                                        taken.ty()
                                    ));
                                }
                                leaves.push(taken);
                            }
                            AbstractValue::Record(record, leaves)
                        }
                        (ScalarOperand::Record(_), _) | (_, ScalarType::Record(_)) => {
                            return Err("a record result returns a whole record, and only a \
                                 record result does"
                                .into());
                        }
                        (value, _) => state.take(value)?,
                    };
                    if value.ty() != function.result {
                        return Err("return scalar/owned type mismatch".into());
                    }
                    let returned = value.resources();
                    state.exit(&returned)?;
                    exits.push(WorkerExit {
                        returned: returned
                            .iter()
                            .map(|r| {
                                inputs
                                    .get(r)
                                    .copied()
                                    .map(WorkerReturn::Input)
                                    .unwrap_or(WorkerReturn::Fresh)
                            })
                            .collect(),
                        dropped: inputs
                            .iter()
                            .filter_map(|(resource, &index)| {
                                (!returned.contains(resource)).then_some(index)
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
                            match value.ty() {
                                ScalarType::OwnedBuffer(st::ScalarType::I32) => {}
                                // Card `W48-QR-SCALAR-TYPES-R6`: a declared
                                // error payload is `own<Buffer>`, the `i32`
                                // payload; a typed one has no declaration
                                // (`W48-QR-TYPES-ERROR-PAYLOAD`).
                                ScalarType::OwnedBuffer(payload) => {
                                    return Err(format!(
                                        "an error payload is an `own<Buffer>` of an i32 \
                                         payload; a `{payload}` payload is not one \
                                         (W48-QR-TYPES-ERROR-PAYLOAD)"
                                    ));
                                }
                                _ => {
                                    return Err("an error payload must be an owned Buffer".into());
                                }
                            }
                            value.resource()
                        }
                        None => None,
                    };
                    state.exit(carried.as_slice())?;
                    let carried_index = carried.and_then(|resource| {
                        inputs
                            .get(&resource)
                            .copied()
                            .map(WorkerReturn::Input)
                            .or(Some(WorkerReturn::Fresh))
                    });
                    for error in errors {
                        exits.push(WorkerExit {
                            returned: vec![],
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
    //
    // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result that owns a Buffer
    // hands identities back exactly as an owned result does, so it keeps its
    // actual exits too; a record of scalars carries no custody and is
    // overapproximated like any scalar result.
    let result_owns = match function.result {
        ScalarType::OwnedBuffer(_) => true,
        ScalarType::Record(record) => module.records.get(&record).is_some_and(TaskRecord::owns),
        _ => false,
    };
    if id != module.entry && !result_owns && function.error_payloads.is_empty() {
        let dropped = inputs.values().copied().collect::<Vec<_>>();
        exits = std::iter::once(WorkerExit {
            returned: vec![],
            dropped: dropped.clone(),
            error: None,
            error_payload: None,
        })
        .chain(function.errors.iter().cloned().map(|error| WorkerExit {
            returned: vec![],
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
    Ok(FunctionSummary {
        exits,
        lease_footprint,
        slot_types,
        buffer_payloads,
    })
}

/// The abstract completion outcome of a record result (card
/// `W48-QR-TASK-RECORD-CAPTURES`): each scalar leaf a placeholder of its type —
/// the abstract run tracks types, not numbers — and each owned leaf the next
/// identity the checked exit hands back.
fn abstract_record_outcome(
    module: &TaskModule,
    record: usize,
    owned: &[ResourceId],
) -> Result<TaskOutcome, String> {
    let AbstractValue::Record(_, leaves) = record_value(module, record, owned)? else {
        unreachable!("record_value builds a record")
    };
    Ok(TaskOutcome::Record(
        leaves
            .into_iter()
            .map(|leaf| match leaf {
                AbstractValue::Owned(resource, _) => TaskOutcome::OwnedBuffer(resource),
                AbstractValue::Scalar(ScalarType::Fixed(ty)) => TaskOutcome::Fixed { ty, value: 0 },
                _ => TaskOutcome::I32(0),
            })
            .collect(),
    ))
}

/// The whole value of a collected record result (card
/// `W48-QR-TASK-RECORD-CAPTURES`): its declared leaves in leaf order, each
/// scalar leaf a value of its type and each owned leaf the next identity the
/// completion carried. Every carried identity is used, exactly once.
fn record_value(
    module: &TaskModule,
    record: usize,
    owned: &[ResourceId],
) -> Result<AbstractValue, String> {
    let declared = module
        .records
        .get(&record)
        .ok_or_else(|| format!("record result {record} names no declared record"))?;
    let mut carried = owned.iter();
    let mut leaves = Vec::with_capacity(declared.leaves.len());
    for leaf in &declared.leaves {
        leaves.push(match leaf.ty {
            ScalarType::OwnedBuffer(payload) => {
                let resource = carried.next().ok_or_else(|| {
                    format!(
                        "record `{}`'s owned leaf `{}` has no carried identity",
                        declared.name, leaf.path
                    )
                })?;
                AbstractValue::Owned(*resource, payload)
            }
            ty => AbstractValue::Scalar(ty),
        });
    }
    if carried.next().is_some() {
        return Err(format!(
            "record `{}` was completed with more identities than it has owned leaves",
            declared.name
        ));
    }
    Ok(AbstractValue::Record(record, leaves))
}

/// The task type of a §G02t type: `i32` keeps its own variant.
fn task_type(ty: st::ScalarType) -> ScalarType {
    if ty == st::ScalarType::I32 {
        ScalarType::I32
    } else {
        ScalarType::Fixed(ty)
    }
}

/// The shared value-statement decision over this path's state.
fn decide_value(
    state: &State,
    operation: &TaskValueOperation,
    raises_permitted: bool,
    comparisons: crate::scalar_types::FloatEffects,
    target: Option<&RecordedTarget>,
) -> Result<scalar_values::Decision, String> {
    let operation = shared_operation(state, operation)?;
    Decider {
        permitted: comparisons,
        ..Decider::default()
    }
    .decide(operation, raises_permitted, target)
    .map_err(|refusal| value_refusal_message(&refusal))
}

fn shared_operation(
    state: &State,
    operation: &TaskValueOperation,
) -> Result<scalar_values::Operation, String> {
    Ok(match operation {
        TaskValueOperation::Binary(op, a, b) => {
            scalar_values::Operation::Binary(*op, state.value_operand(a)?, state.value_operand(b)?)
        }
        TaskValueOperation::Compare(cmp, a, b) => scalar_values::Operation::Compare(
            *cmp,
            state.value_operand(a)?,
            state.value_operand(b)?,
        ),
        TaskValueOperation::Negate(a) => scalar_values::Operation::Negate(state.value_operand(a)?),
        TaskValueOperation::Convert(a, to) => {
            scalar_values::Operation::Convert(state.value_operand(a)?, *to)
        }
    })
}

/// The `ArithmeticError` tags a raising statement's error edge can carry.
fn raisable_variants(
    state: &State,
    operation: &TaskValueOperation,
) -> Result<BTreeSet<String>, String> {
    Ok(scalar_values::raisable(shared_operation(state, operation)?)
        .into_iter()
        .map(|error| error.to_string())
        .collect())
}

/// The task route's words for a refused value statement. The conditions are
/// the shared decision's; only the permission's wording is this route's, whose
/// raise is an ordinary typed worker error rather than an exit status.
fn value_refusal_message(refusal: &ValueRefusal) -> String {
    match refusal {
        ValueRefusal::NeedsPermission { ty } => format!(
            "this `{ty}` operation reads a value known only at run time and can fault; the \
             function's row must declare `throws<ArithmeticError>` for the fault to be raised as \
             a typed error, and without it the operation is refused"
        ),
        other => other.to_string(),
    }
}

/// Validate the fixed-width surface of one function (card
/// `W48-QR-SCALAR-TYPES-R4`): `Fixed(i32)` is malformed — `i32` has its own
/// representation — every typed literal is a value of its type, and
/// `isize`/`usize` need the recorded target.
fn validate_fixed_width(module: &TaskModule, function: &TaskFunction) -> Result<(), String> {
    let target = module.target.as_ref();
    let check_type = |ty: st::ScalarType| -> Result<(), String> {
        if ty.is_target_width() && target.is_none() {
            return Err(ValueRefusal::NoRecordedTarget { ty }.to_string());
        }
        Ok(())
    };
    let check_kind = |kind: &ScalarType| -> Result<(), String> {
        match kind {
            ScalarType::Fixed(st::ScalarType::I32) => {
                Err("the i32 type has one representation; Fixed(i32) is malformed".into())
            }
            ScalarType::Fixed(ty) => check_type(*ty),
            // Card `W48-QR-SCALAR-TYPES-R6`: a typed payload's width is its
            // type's, so an `isize`/`usize` payload needs the record too.
            ScalarType::OwnedBuffer(ty) | ScalarType::ReadLoan(ty) | ScalarType::WriteLease(ty) => {
                check_type(*ty)
            }
            _ => Ok(()),
        }
    };
    let check_literal = |ty: st::ScalarType, value: i128| -> Result<(), String> {
        check_type(ty)?;
        if !ty.contains(value, target) {
            return Err(ValueRefusal::LiteralOutOfRange { literal: value, ty }.to_string());
        }
        Ok(())
    };
    let check_leaf_operand = |operand: &ScalarOperand| -> Result<(), String> {
        match operand {
            ScalarOperand::Fixed {
                ty: st::ScalarType::I32,
                ..
            } => Err("an i32 literal has one representation; Fixed { i32 } is malformed".into()),
            ScalarOperand::Fixed { ty, value } => check_literal(*ty, *value),
            // Card `W48-QR-TASK-RECORD-CAPTURES`: a record's leaves are flat.
            ScalarOperand::Record(_) => Err("a record operand's leaves are not records".into()),
            _ => Ok(()),
        }
    };
    let check_operand = |operand: &ScalarOperand| -> Result<(), String> {
        match operand {
            ScalarOperand::Record(leaves) => leaves.iter().try_for_each(check_leaf_operand),
            other => check_leaf_operand(other),
        }
    };
    // Card `W48-QR-TASK-RECORD-CAPTURES`: a record result's leaf types are part
    // of this function's surface.
    if let ScalarType::Record(record) = function.result
        && let Some(declared) = module.records.get(&record)
    {
        for leaf in &declared.leaves {
            check_kind(&leaf.ty)?;
        }
    }
    let check_operation = |operation: &TaskValueOperation| -> Result<(), String> {
        for operand in operation.operands() {
            if let TaskValueOperand::Literal { ty, value } = operand {
                check_literal(ty, value)?;
            }
        }
        if let TaskValueOperation::Convert(_, to) = operation {
            check_type(*to)?;
        }
        Ok(())
    };
    check_kind(&function.result)?;
    for (_, kind) in &function.parameters {
        check_kind(kind)?;
    }
    for block in function.blocks.values() {
        for instruction in &block.instructions {
            match &instruction.kind {
                TaskInstructionKind::BindScalar { value, .. }
                | TaskInstructionKind::WriteBuffer { value, .. } => check_operand(value)?,
                TaskInstructionKind::WriteBufferByte { index, value, .. } => {
                    check_operand(index)?;
                    check_operand(value)?;
                }
                TaskInstructionKind::ReadBufferByte { index, .. } => check_operand(index)?,
                TaskInstructionKind::Compute { operation, .. } => check_operation(operation)?,
                _ => {}
            }
        }
        match &block.terminator.kind {
            TaskTerminatorKind::TryAllocateBuffer { initial, .. } => check_operand(initial)?,
            TaskTerminatorKind::TrySpawn { arguments, .. } => {
                for argument in arguments {
                    check_operand(argument)?;
                }
            }
            TaskTerminatorKind::Return { value } => check_operand(value)?,
            TaskTerminatorKind::TryCompute { operation, .. } => check_operation(operation)?,
            _ => {}
        }
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
            Ok(summary) => {
                summaries.insert(id, summary);
            }
            Err(error) => errors.push(error),
        }
    }
    // Card `W48-QR-SCALAR-TYPES-R4`: the recorded target is an assumption of
    // the result exactly when the module names `isize` or `usize`.
    let target_assumption = if fixed_types(module).iter().any(|ty| ty.is_target_width()) {
        module.target.clone()
    } else {
        None
    };
    if errors.is_empty() {
        Ok(CheckedTaskModule {
            module: module.clone(),
            slot_types: summaries
                .iter()
                .flat_map(|(function, summary)| {
                    summary
                        .slot_types
                        .iter()
                        .map(|(slot, ty)| ((*function, *slot), *ty))
                })
                .collect(),
            buffer_payloads: summaries
                .iter()
                .flat_map(|(function, summary)| {
                    summary
                        .buffer_payloads
                        .iter()
                        .map(|(slot, ty)| ((*function, *slot), *ty))
                })
                .collect(),
            target_assumption,
        })
    } else {
        Err(errors)
    }
}

mod reference;
pub use reference::*;
#[cfg(test)]
mod tests;
