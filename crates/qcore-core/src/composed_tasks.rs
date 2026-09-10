//! `source.std-io-tasks.v1` — the checked composition of `source.std-io.v1`
//! with the **checked task route** (card `COMPOSE-CONCURRENCY`).
//!
//! # What this model is, stated before anything else
//!
//! A composed program is a [`TaskModule`] — unedited, checked by
//! [`check_task_module`] exactly as the task route checks it — plus an ordered
//! list of [`TaskIoSite`]s naming **where in the entry function's own CFG** each
//! `source.std-io.v1` event stands. Nothing here re-implements a custody rule, a
//! share rule, a join rule or a rendering rule: the custody answers are the task
//! checker's and the line bytes are [`crate::std_io::rendered_line`]'s. What
//! this module owns is exactly what neither composed model can state — where a
//! line may stand, whether its operand is readable there, and whether the entry
//! declared the effect the composition performs.
//!
//! # The v1 boundary, and the measurement behind it
//!
//! **A line may stand anywhere in the parent's body. A line inside a worker
//! body is refused.**
//!
//! That split is not a guess about scheduling; it is read off the emitted
//! runtime. In `crates/qcore-core/src/task_c_emit.rs` every write is performed
//! by the thread whose context identity is `0` — the parent:
//!
//! * `qcore_buffer_event` writes immediately only when `context->identity == 0`,
//!   and otherwise appends to the worker's **private** event array;
//! * `qcore_task_collect` replays that array, on the parent's thread, after a
//!   successful `thrd_join`;
//! * `qcore_task_start`, `qcore_task_collect` and `qcore_task_print_result` are
//!   called from the parent, and a worker never calls them because the source
//!   lowering refuses a spawn inside a worker (leaf workers only).
//!
//! So each stream of an emitted task program is written by exactly one thread,
//! in the parent's own statement order, and a source line placed at any point of
//! the parent body has a determined position relative to every trace line. The
//! conservative reading — "before any spawn, between joins, after all joins" —
//! is *subsumed* by that: every parent point is join-ordered, because a worker's
//! output does not exist as bytes until its parent replays it at the join.
//!
//! A line inside a **worker** body would be written by that worker's own thread,
//! concurrently with the parent's writes and with its siblings', and no model in
//! this repository orders those. Admitting it would make the per-stream byte
//! comparison every route here rests on compare against something that is not a
//! function of the program. It is refused by name
//! ([`ComposedTaskRule::SiteOutsideEntry`]), and buffering worker lines the way
//! the trace events are buffered is the recorded future path — it needs the F02
//! comparison-up-to-an-admitted-reordering treatment, or `STD-LOGGER-ASYNC`'s
//! queue, neither of which is built here.
//!
//! # What a rendering may read
//!
//! An operand is an integer literal or a name the **task** model bound to an
//! `i32` — a copied parameter, a `let`-bound literal, the value a
//! `qcore_buffer_read` computed out of a live buffer, or the outcome a `join`
//! collected. The value is not statically known: the task model computes it, and
//! the composed reference execution reads it out of the slot the task model
//! wrote. That is the one structural difference from `source.std-io-extern.v1`,
//! whose numbers are declared at the site, and it is why
//! [`crate::std_io::StdIoProgram::scalars`] cannot carry them.
//!
//! **Whether a slot is readable at a site is asked of the task checker, in the
//! task model's own language, rather than answered here.** [`readable_probe`]
//! inserts a `BindScalar { value: Slot(s) }` — an ordinary copy of an immutable
//! scalar, an operation the task model already has — at exactly the rendering's
//! position and re-runs [`check_task_module`]. If the copy checks, the slot
//! holds a copyable scalar there on every path; if it does not, the rendering is
//! refused. The composition therefore owns no liveness rule, and the must-fact
//! bar of `BORROW-LIVENESS` is met by the task checker's own all-path check
//! rather than by a meet this module computes.
//!
//! # What is deliberately not admitted in v1
//!
//! * A `borrow NAME` rendering ([`crate::std_io::StdIoOperand::Borrowed`]).
//!   `STD-FMT-BORROW` admits one on the extern composition because that model
//!   publishes a ledger the join can read; the task ledger's parent/child
//!   custody edges, share arithmetic and join discipline are a different
//!   question, and refusing is the fail-closed answer until a card makes it.
//! * A rendering that touches custody at all. The operand forms carry an `i32`
//!   or a literal, so a rendering cannot consume, release, lend or retain
//!   anything — the same structural discipline `STD-MODULES` established, over a
//!   different value namespace.
//!
//! # What is not claimed
//!
//! No order between the two streams is modeled, exactly as on every other route
//! that writes. Each stream's own order is checked. `flush` is sink handoff, not
//! durability.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::std_io::{
    self, Level, ObservedLine, Rendering, StdIoEvent, StdIoOperand, StdIoProgram, Stream,
};
use crate::task_execution::{
    BlockId, CheckedTaskModule, FunctionId, ScalarOperand, SlotId, TaskDiagnostic, TaskInstruction,
    TaskInstructionKind, TaskModule, TaskSpan, TaskTerminatorKind, check_task_module,
};
use crate::typed_errors::{Effect, EffectRow};

/// One `source.std-io.v1` event, placed in the entry function's CFG.
///
/// The position is "immediately before instruction `index` of block `block`",
/// with `index == instructions.len()` meaning "immediately before that block's
/// terminator". Two events written one after the other in the source therefore
/// share a position and are ordered by their position in
/// [`ComposedTaskProgram::sites`], which is source order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskIoSite {
    pub block: BlockId,
    pub index: usize,
    pub event: StdIoEvent,
    /// The task slot a `Scalar` operand names. `None` for a literal operand and
    /// for `flush`, which renders nothing.
    pub operand: Option<SlotId>,
    pub span: TaskSpan,
}

/// A task program and the `source.std-io.v1` events placed in its entry.
///
/// The two halves are carried side by side and never merged: `tasks` is exactly
/// the module the task route would check, byte for byte, and a program with no
/// sites is that route's program with an empty list beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedTaskProgram {
    pub tasks: TaskModule,
    /// In source order.
    pub sites: Vec<TaskIoSite>,
    /// Whether the entry's declared `effects { … }` row admits `io`.
    pub declares_io: bool,
}

/// The join rules this composition owns, and nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposedTaskRule {
    /// A line placed outside the entry function — in a worker body. See the
    /// module header: a worker's writes are performed by its own thread, and no
    /// model here orders them against the parent's or its siblings'.
    SiteOutsideEntry,
    /// A position that is not a position of that block.
    SiteOutOfRange,
    /// Two lines in one block whose source order disagrees with their positions,
    /// so the order the program performs is not the order that was checked.
    SiteOrder,
    /// An operand this route does not render: a `borrow NAME`, or a name for
    /// which the lowering resolved no task slot.
    UnrenderableOperand,
    /// The task checker refuses to read that slot at that position.
    OperandNotReadable,
    /// The composed program writes but the entry does not declare `io`.
    EffectNotDeclared,
}

impl ComposedTaskRule {
    pub fn describe(self) -> &'static str {
        match self {
            Self::SiteOutsideEntry => "a line inside a worker body",
            Self::SiteOutOfRange => "a line at a position its block does not have",
            Self::SiteOrder => "two lines whose source order is not the order performed",
            Self::UnrenderableOperand => "an operand this route does not render",
            Self::OperandNotReadable => "an operand the task model cannot read here",
            Self::EffectNotDeclared => "an undeclared `io` effect",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedTaskDiagnostic {
    pub rule: ComposedTaskRule,
    pub site: usize,
    pub span: TaskSpan,
    pub message: String,
}

impl fmt::Display for ComposedTaskDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ComposedTaskDiagnostic {}

/// A refusal of a composed program, told apart by which model refused.
///
/// The task half's refusals are the task route's own diagnostics, reported at
/// the task route's own code: composing buys no admission, and a program whose
/// task half does not check is refused before this module states a rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposedTaskRefusal {
    Tasks(Vec<TaskDiagnostic>),
    Join(Vec<ComposedTaskDiagnostic>),
}

/// The published evidence of a composed program.
///
/// Constructible only through [`check_composed_task_program`].
///
/// ```compile_fail
/// use qcore_core::composed_tasks::{CheckedComposedTaskModule, ComposedTaskProgram};
/// fn forge(p: ComposedTaskProgram) -> CheckedComposedTaskModule {
///     CheckedComposedTaskModule { program: p }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedComposedTaskModule {
    program: ComposedTaskProgram,
    tasks: CheckedTaskModule,
    certificate: ComposedTaskCertificate,
}

impl CheckedComposedTaskModule {
    pub fn program(&self) -> &ComposedTaskProgram {
        &self.program
    }

    /// The task half, checked by the task route's own unedited checker. Every
    /// consumer of a task module — the reference execution, the C backend, the
    /// profile classification — takes it from here.
    pub fn tasks(&self) -> &CheckedTaskModule {
        &self.tasks
    }

    pub fn sites(&self) -> &[TaskIoSite] {
        &self.program.sites
    }

    pub fn certificate(&self) -> &ComposedTaskCertificate {
        &self.certificate
    }
}

/// What a composed publication carries beside the task half's own evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedTaskCertificate {
    pub entry: FunctionId,
    /// `{Io}` when the program writes or flushes, empty otherwise — derived by
    /// `source.std-io.v1`'s own [`std_io::derived_effects`], never recomputed
    /// here.
    pub effects: EffectRow,
    /// Every placed event, in source order, with its position.
    pub placements: Vec<(BlockId, usize, StdIoEvent)>,
    /// The `std.fmt` surface this program actually uses.
    pub renderings: BTreeSet<Rendering>,
    /// The `std.logger` severities this program actually uses.
    pub levels: BTreeSet<Level>,
    /// The streams this program actually writes.
    pub streams: BTreeSet<Stream>,
    pub flushes: usize,
}

/// The `source.std-io.v1` projection of a composed program: the same events, in
/// source order, with the entry's declared row.
///
/// It is built so this module can call `source.std-io.v1`'s own
/// [`std_io::derived_effects`] rather than restate the effect rule. Its
/// `scalars` map is deliberately **empty**: a composed operand's number is
/// computed by the task model at the rendering's own step, so it is not a
/// static binding and must not look like one.
pub fn std_io_projection(program: &ComposedTaskProgram) -> StdIoProgram {
    StdIoProgram {
        entry: program.tasks.entry.to_string(),
        scalars: BTreeMap::new(),
        identities: BTreeMap::new(),
        declared_effects: if program.declares_io {
            EffectRow::new([Effect::Io])
        } else {
            EffectRow::default()
        },
        events: program
            .sites
            .iter()
            .map(|site| site.event.clone())
            .chain(std::iter::once(StdIoEvent::Return))
            .collect(),
    }
}

fn max_slot(module: &TaskModule) -> SlotId {
    let mut max = 0;
    for function in module.functions.values() {
        for (slot, _) in &function.parameters {
            max = max.max(*slot);
        }
        for block in function.blocks.values() {
            for instruction in &block.instructions {
                for slot in instruction_slots(&instruction.kind) {
                    max = max.max(slot);
                }
            }
            for slot in terminator_slots(&block.terminator.kind) {
                max = max.max(slot);
            }
        }
    }
    max
}

fn operand_slots(operand: &ScalarOperand) -> Vec<SlotId> {
    match operand {
        ScalarOperand::Slot(slot)
        | ScalarOperand::Move(slot)
        | ScalarOperand::Loan { slot, .. } => {
            vec![*slot]
        }
        ScalarOperand::Unit | ScalarOperand::I32(_) => Vec::new(),
    }
}

fn instruction_slots(kind: &TaskInstructionKind) -> Vec<SlotId> {
    match kind {
        TaskInstructionKind::ReadBuffer { buffer, value_slot } => vec![*buffer, *value_slot],
        TaskInstructionKind::WriteBuffer { buffer, value } => {
            let mut slots = vec![*buffer];
            slots.extend(operand_slots(value));
            slots
        }
        TaskInstructionKind::DropBuffer { buffer } => vec![*buffer],
        TaskInstructionKind::BindScalar { slot, value } => {
            let mut slots = vec![*slot];
            slots.extend(operand_slots(value));
            slots
        }
        TaskInstructionKind::UnwrapInfallible {
            result_slot,
            value_slot,
        } => vec![*result_slot, *value_slot],
        TaskInstructionKind::EnterScope { .. } | TaskInstructionKind::ExitScope { .. } => {
            Vec::new()
        }
        TaskInstructionKind::Join {
            handle,
            outcome_slot,
        } => {
            let mut slots = vec![*handle];
            slots.extend(outcome_slot.iter().copied());
            slots
        }
        TaskInstructionKind::TakeErrorPayload {
            error_slot,
            buffer_slot,
        } => vec![*error_slot, *buffer_slot],
    }
}

fn terminator_slots(kind: &TaskTerminatorKind) -> Vec<SlotId> {
    match kind {
        TaskTerminatorKind::TryAllocateBuffer {
            initial,
            value_slot,
            ..
        } => {
            let mut slots = vec![*value_slot];
            slots.extend(operand_slots(initial));
            slots
        }
        TaskTerminatorKind::Goto { .. } => Vec::new(),
        TaskTerminatorKind::TrySpawn {
            arguments,
            handle_slot,
            ..
        } => {
            let mut slots = vec![*handle_slot];
            for argument in arguments {
                slots.extend(operand_slots(argument));
            }
            slots
        }
        TaskTerminatorKind::TryOutcome {
            result_slot,
            value_slot,
            error_slot,
            ..
        } => vec![*result_slot, *value_slot, *error_slot],
        TaskTerminatorKind::Return { value } => operand_slots(value),
        TaskTerminatorKind::Raise { payload, .. } => {
            payload.iter().flat_map(operand_slots).collect()
        }
    }
}

/// Ask the **task checker** whether `slot` holds a copyable immutable scalar at
/// this position, by putting the question in the task model's own language.
///
/// The probe is an ordinary `BindScalar { value: Slot(slot) }` — a copy of an
/// immutable scalar, which the task model already admits and which moves no
/// custody — inserted at exactly the rendering's position. If the resulting
/// module checks, the copy is admitted on every path that reaches the site, and
/// a rendering reads no more than the copy does. If it does not, the rendering
/// is refused. Nothing about liveness is decided in this file.
pub fn readable_probe(
    module: &TaskModule,
    block: BlockId,
    index: usize,
    slot: SlotId,
    fresh: SlotId,
) -> Option<TaskModule> {
    let mut probe = module.clone();
    let entry = probe.functions.get_mut(&module.entry)?;
    let target = entry.blocks.get_mut(&block)?;
    if index > target.instructions.len() {
        return None;
    }
    let span = target
        .instructions
        .get(index)
        .map_or(target.terminator.span, |instruction| instruction.span);
    target.instructions.insert(
        index,
        TaskInstruction {
            span,
            kind: TaskInstructionKind::BindScalar {
                slot: fresh,
                value: ScalarOperand::Slot(slot),
            },
        },
    );
    Some(probe)
}

fn refuse(
    errors: &mut Vec<ComposedTaskDiagnostic>,
    rule: ComposedTaskRule,
    site: usize,
    span: TaskSpan,
    message: impl Into<String>,
) {
    errors.push(ComposedTaskDiagnostic {
        rule,
        site,
        span,
        message: message.into(),
    });
}

/// Check a composed program: the task half by the task route's own unedited
/// checker, then the three things neither model can state.
pub fn check_composed_task_program(
    program: &ComposedTaskProgram,
) -> Result<CheckedComposedTaskModule, ComposedTaskRefusal> {
    // The task half first, and unedited. A program whose task half does not
    // check is refused before this module states any rule of its own, so
    // composing can never admit a task program the task route refuses.
    let tasks = check_task_module(&program.tasks).map_err(ComposedTaskRefusal::Tasks)?;

    let mut errors = Vec::new();
    let entry_blocks = program.tasks.functions[&program.tasks.entry]
        .blocks
        .iter()
        .map(|(id, block)| (*id, block.instructions.len()))
        .collect::<BTreeMap<_, _>>();
    let fresh_base = max_slot(&program.tasks) + 1;

    let mut previous: BTreeMap<BlockId, usize> = BTreeMap::new();
    for (ordinal, site) in program.sites.iter().enumerate() {
        let Some(&length) = entry_blocks.get(&site.block) else {
            refuse(
                &mut errors,
                ComposedTaskRule::SiteOutsideEntry,
                ordinal,
                site.span,
                "`source.std-io-tasks.v1` writes a line only from the parent's own body: a \
                 worker's writes would be performed by that worker's thread, concurrently with \
                 the parent's and with its siblings', and no checked model here orders them, so \
                 the per-stream comparison this route rests on would compare against something \
                 that is not a function of the program",
            );
            continue;
        };
        if site.index > length {
            refuse(
                &mut errors,
                ComposedTaskRule::SiteOutOfRange,
                ordinal,
                site.span,
                "a line is placed at a position its block does not have",
            );
            continue;
        }
        if let Some(earlier) = previous.insert(site.block, site.index)
            && earlier > site.index
        {
            refuse(
                &mut errors,
                ComposedTaskRule::SiteOrder,
                ordinal,
                site.span,
                "two lines in one block are ordered differently from the order the emitted \
                 program performs them in",
            );
        }
        match (&site.event, site.operand) {
            (StdIoEvent::Flush | StdIoEvent::Return, _) => {}
            (
                StdIoEvent::Print {
                    operand: StdIoOperand::Literal(_),
                    ..
                }
                | StdIoEvent::Log {
                    operand: StdIoOperand::Literal(_),
                    ..
                },
                _,
            ) => {}
            (
                StdIoEvent::Print {
                    operand: StdIoOperand::Borrowed(name),
                    ..
                }
                | StdIoEvent::Log {
                    operand: StdIoOperand::Borrowed(name),
                    ..
                },
                _,
            ) => refuse(
                &mut errors,
                ComposedTaskRule::UnrenderableOperand,
                ordinal,
                site.span,
                format!(
                    "`borrow {name}` renders the model identity a checked custody model published \
                     for a resource, and `source.std-io-tasks.v1` publishes none: the task \
                     ledger's parent/child custody edges, read shares and join discipline are a \
                     different question from the extern route's straight-line ledger, and \
                     admitting a borrowed render across a task boundary needs its own custody \
                     argument"
                ),
            ),
            (
                StdIoEvent::Print {
                    operand: StdIoOperand::Scalar(name),
                    ..
                }
                | StdIoEvent::Log {
                    operand: StdIoOperand::Scalar(name),
                    ..
                },
                operand,
            ) => match operand {
                None => refuse(
                    &mut errors,
                    ComposedTaskRule::UnrenderableOperand,
                    ordinal,
                    site.span,
                    format!("`{name}` resolves to no task slot in this entry"),
                ),
                Some(slot) => {
                    let admitted = readable_probe(
                        &program.tasks,
                        site.block,
                        site.index,
                        slot,
                        fresh_base + ordinal,
                    )
                    .is_some_and(|probe| check_task_module(&probe).is_ok());
                    if !admitted {
                        refuse(
                            &mut errors,
                            ComposedTaskRule::OperandNotReadable,
                            ordinal,
                            site.span,
                            format!(
                                "the checked task model does not admit a copy of `{name}` here, \
                                 so a rendering may not read it either: a rendering reads no more \
                                 than the model's own copy of an immutable scalar does"
                            ),
                        );
                    }
                }
            },
        }
    }

    // The effect rule is `source.std-io.v1`'s, applied to the composed
    // projection: this module states which events exist and that route derives
    // what performing them means.
    let projection = std_io_projection(program);
    let derived = std_io::derived_effects(&projection);
    for effect in derived.iter() {
        if !projection.declared_effects.contains(effect) {
            refuse(
                &mut errors,
                ComposedTaskRule::EffectNotDeclared,
                program.sites.len(),
                program.tasks.functions[&program.tasks.entry].span,
                "this entry writes a line but its declared `effects { … }` row does not admit `io`",
            );
        }
    }

    if !errors.is_empty() {
        return Err(ComposedTaskRefusal::Join(errors));
    }

    let mut renderings = BTreeSet::new();
    let mut levels = BTreeSet::new();
    let mut streams = BTreeSet::new();
    let mut flushes = 0;
    for site in &program.sites {
        match &site.event {
            StdIoEvent::Print { rendering, .. } => {
                renderings.insert(*rendering);
                streams.insert(Stream::Out);
            }
            StdIoEvent::Log {
                level, rendering, ..
            } => {
                renderings.insert(*rendering);
                levels.insert(*level);
                streams.insert(Stream::Err);
            }
            StdIoEvent::Flush => flushes += 1,
            StdIoEvent::Return => {}
        }
    }

    Ok(CheckedComposedTaskModule {
        certificate: ComposedTaskCertificate {
            entry: program.tasks.entry,
            effects: derived,
            placements: program
                .sites
                .iter()
                .map(|site| (site.block, site.index, site.event.clone()))
                .collect(),
            renderings,
            levels,
            streams,
            flushes,
        },
        program: program.clone(),
        tasks,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposedTaskRecheckFailure {
    Entry,
    Effects,
    Placements,
    Renderings,
    Levels,
    Streams,
    Flushes,
    Refused,
}

impl fmt::Display for ComposedTaskRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Self::Entry => "the entry function",
            Self::Effects => "the derived effect row",
            Self::Placements => "the placed events and their positions",
            Self::Renderings => "the used renderings",
            Self::Levels => "the used severities",
            Self::Streams => "the written streams",
            Self::Flushes => "the barrier count",
            Self::Refused => "the program itself, which the recheck refuses",
        };
        write!(f, "{what} does not survive an independent rederivation")
    }
}

/// Rederive the composed certificate from the program alone and compare it field
/// for field. A published certificate is never one the driver did not recompute.
pub fn recheck_composed_task_certificate(
    program: &ComposedTaskProgram,
    certificate: &ComposedTaskCertificate,
) -> Result<(), ComposedTaskRecheckFailure> {
    let rederived = check_composed_task_program(program)
        .map_err(|_| ComposedTaskRecheckFailure::Refused)?
        .certificate;
    if rederived.entry != certificate.entry {
        return Err(ComposedTaskRecheckFailure::Entry);
    }
    if rederived.effects != certificate.effects {
        return Err(ComposedTaskRecheckFailure::Effects);
    }
    if rederived.placements != certificate.placements {
        return Err(ComposedTaskRecheckFailure::Placements);
    }
    if rederived.renderings != certificate.renderings {
        return Err(ComposedTaskRecheckFailure::Renderings);
    }
    if rederived.levels != certificate.levels {
        return Err(ComposedTaskRecheckFailure::Levels);
    }
    if rederived.streams != certificate.streams {
        return Err(ComposedTaskRecheckFailure::Streams);
    }
    if rederived.flushes != certificate.flushes {
        return Err(ComposedTaskRecheckFailure::Flushes);
    }
    Ok(())
}

/// The composed reference observation: the task route's own observation,
/// **unchanged**, and the source lines beside it with the position each one is
/// written at.
///
/// The two are kept apart rather than merged so that "the task evidence is the
/// task route's" is a fact about the data and not a claim: `tasks` is exactly
/// what `execute_task_module` would have produced for the same module, and
/// [`crate::task_execution::render_task_observation`] of it is exactly the
/// program's standard output with this route's own lines removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedTaskObservation {
    pub tasks: crate::task_execution::TaskExecutionObservation,
    /// `(position, line)` — the line is written immediately before
    /// `tasks.events[position]`, and `position == tasks.events.len()` means
    /// after every event.
    pub lines: Vec<(usize, ObservedLine)>,
}

impl ComposedTaskObservation {
    /// The two streams, each in its own order. No order between them is claimed.
    pub fn projections(&self) -> (String, String) {
        let mut out = String::new();
        let mut err = String::new();
        let mut cursor = 0usize;
        for (position, event) in self.tasks.events.iter().enumerate() {
            while cursor < self.lines.len() && self.lines[cursor].0 <= position {
                let line = &self.lines[cursor].1;
                match line.stream {
                    Stream::Out => out.push_str(&format!("{}\n", line.text)),
                    Stream::Err => err.push_str(&format!("{}\n", line.text)),
                }
                cursor += 1;
            }
            out.push_str(&crate::task_execution::render_task_event(event));
        }
        for (_, line) in &self.lines[cursor..] {
            match line.stream {
                Stream::Out => out.push_str(&format!("{}\n", line.text)),
                Stream::Err => err.push_str(&format!("{}\n", line.text)),
            }
        }
        (out, err)
    }
}

#[cfg(test)]
#[path = "../tests/unit/composed_tasks.rs"]
mod tests;
