//! `source.epoch.tasks.v1` — a flat-epoch declaration composed with a **real
//! checked task region**.
//!
//! `source.epoch.bound.v1` (see [`crate::source_epoch`]) checks an epoch whose
//! workers are declarations: a worker block declares quantities and has no body,
//! no captures and no completion capsule. That was recorded as its first open
//! boundary. This module composes the same unedited `FlatEpochBoundV1` checker
//! with the same unedited scoped-task profile: the entry's first statement is an
//! epoch *plan*, and the rest of the entry is an ordinary checked task region
//! that `qcore_core::task_execution::check_task_module` accepts on its own.
//!
//! ```text
//! entry-body  ::= plan , task-region
//! plan        ::= "thread" "scope" NAME "epoch" "{" (header | row)* "}"
//! header      ::= "bound" INT "effort" INT ";"
//!               | "overhead" "scope" INT "thread" INT ("measured" INT)? ";"
//!               | "bound" "effort" "as" "latency" ";"
//! row         ::= "thread" "scope" WORKER "{" clause* "}"
//! clause      ::= "move" ";" | "copy" ";"          -- ordered captures
//!               | "peak" INT ";" | "effort" INT ";"
//!               | "credit" INT "effort" INT ";"
//!               | "retained" INT ";"
//!               | "join" "(" ("move" | "drop" | "close") ")" ";"   -- optional
//! ```
//!
//! **The composition is checked, never assumed.** Nothing about the task region
//! is read out of the plan, and nothing about the plan is read out of the task
//! region without being compared. Concretely:
//!
//! * The **rows** and the **spawn sites** must correspond exactly. Rows naming
//!   worker `W`, in declaration order, are matched one-to-one with the entry's
//!   `TrySpawn` sites for `W`, in source order. A declared row with no matching
//!   ledger entry, and a ledger entry with no declared row, are both refusals —
//!   in either direction, and per worker, so an over-declared row cannot be
//!   paid for by an undeclared one.
//! * The **captures** must correspond exactly. A row's ordered `move;`/`copy;`
//!   clauses must equal that spawn site's ordered argument operands. A missing,
//!   extra or wrongly-moded capture is a refusal.
//! * The **result** must correspond. `retained BYTES;` is required exactly when
//!   the spawned worker's checked result type is an owned Buffer, and refused
//!   otherwise. The *footprint* stays declared — see below.
//! * The **disposal** is **derived** from the checked task IR, not declared. A
//!   `join(...)` clause is optional; when it is written it must equal the
//!   derived disposal, and when it is omitted the derived disposal is used.
//!
//! **What is derived and what stays declared.** The card asked for elided
//! declarations wherever every figure of a row is derivable from checked
//! structure, using capture sizes from declared Buffer extents as the example.
//! That example is not reachable and the reason is structural, not an omission:
//! this backend's `Buffer` has **no extent**. `TryAllocateBuffer` takes an
//! initial *scalar value*, `qcore_buffer_read` returns one scalar, and no source
//! form declares a size. So no byte figure — `peak`, `retained`, `credit`,
//! `bound`, `overhead` — is derivable here, and neither is `effort`, because the
//! profile has no cost model. What *is* derivable is the epoch's **shape**: the
//! task count, each task's identity, its capture list, whether it retains a
//! result, whether it is joined, and what the creator then does with the result.
//! Those are derived, and the two the plan may still spell (the result kind
//! through `retained`, the disposal through `join`) are compared rather than
//! believed.
//!
//! **Shapes this composition refuses.** Shared storage cells (`let NAME = share
//! N;`, `read NAME;`, `write NAME;`) have no referent in the copy/owned task
//! forms, so they are refused rather than admitted as free-floating declarations
//! (`T08-COMPOSE-SHARED`). Read loans and counted shares are refused as captures
//! (`T08-COMPOSE-LOAN`). A mutex generation in the composed body is refused
//! (`T08-COMPOSE-GUARD`): the epoch models no guard and the mutex ledger models
//! no accounting, and admitting the pair would be asserting a composition
//! neither checker was given. A joined owned result that the creator then moves
//! on is refused (`T08-COMPOSE-RESULT-REUSE`), because it is neither released at
//! the join nor live in the creator at the epoch exit.
//!
//! Every refusal decided here is `Q600`, the carried-over structural refusal the
//! epoch and task routes already share; every refusal the *profile* models still
//! comes from the core's own `Q630`-`Q641` band, because the assembled program
//! is handed to the unedited `check_flat_epoch`.
use std::collections::{BTreeMap, BTreeSet};

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token};
use crate::source_epoch::{
    EpochSourceTask, LoweredEpochModule, both, failure, header_tokens, integer, memory, name,
    symbol, word, work,
};
use qcore_core::prelude::concurrency::epoch_accounting::{
    EpochTask, EpochTaskId, FlatEpochProgram, OverheadBasis, ResultDisposal, RuntimeOverhead,
    TaskResult,
};
use qcore_core::task_execution::{
    CheckedTaskModule, ScalarOperand, ScalarType, TaskInstructionKind, TaskTerminatorKind,
};

/// A capture as either side of the composition describes it. The declared and
/// the derived list are compared elementwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposedCapture {
    /// `copy;` — a scalar copied into the child.
    Copy,
    /// `move;` — an owned Buffer consumed by the creation.
    Move,
}

impl ComposedCapture {
    pub const fn clause(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }
}

/// Where a row's disposal came from. `Declared` was written and then checked
/// against the derived one; `Derived` was elided and filled from the ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisposalSource {
    Declared,
    Derived,
}

/// One spawn site as the checked task IR describes it. Every field is read out
/// of `CheckedTaskModule`; none is read out of the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerSpawn {
    pub worker: String,
    pub span: Span,
    pub captures: Vec<ComposedCapture>,
    /// The worker's checked result type is an owned Buffer, so a completed
    /// child's result is live until the creator disposes of it.
    pub retains_result: bool,
    /// What the creator does with that result, derived from the IR. `None` when
    /// the worker retains nothing.
    pub disposal: Option<ResultDisposal>,
}

/// One reconciled row: a declared plan row and the ledger spawn it agreed with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedRow {
    pub task: EpochTaskId,
    pub worker: String,
    pub captures: Vec<ComposedCapture>,
    pub result: TaskResult,
    pub disposal_source: DisposalSource,
    /// The row's `thread scope` header, for locating a refusal.
    pub row_span: Span,
    /// The spawn site this row agreed with.
    pub spawn_span: Span,
}

/// The composed lowering: an assembled [`FlatEpochProgram`] the caller still has
/// to hand to `check_flat_epoch`, plus the reconciliation record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredComposedEpoch {
    /// Reuses the pure route's locating record, so a core refusal lands on the
    /// row that caused it under exactly the same rules.
    pub plan: LoweredEpochModule,
    pub rows: Vec<ComposedRow>,
    pub ledger: Vec<LedgerSpawn>,
}

/// The syntactic split: where the plan is, and the task source with the plan
/// blanked out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedSplit {
    pub plan_span: Span,
    /// The original bytes with the plan statement replaced by spaces and
    /// newlines. Byte offsets are preserved exactly, so every span the task
    /// route reports still points at the original file.
    pub task_source: String,
}

/// Select the composed route. Detection is syntactic and route-selecting only:
/// an epoch scope *and* the scoped-task profile's own `concurrency checked;`
/// marker. A plain epoch declaration has no such marker and keeps the pure
/// `source.epoch.bound.v1` route; a plain task module has no epoch scope.
pub fn has_composed_epoch(source: &SourceProgram) -> bool {
    crate::source_epoch::has_source_epoch(source)
        && source
            .declarations
            .iter()
            .any(|declaration| declaration.value.kind == DeclarationKind::ConcurrencyProfile)
}

fn entry_body<'a>(
    source: &'a SourceProgram,
    entry: &str,
) -> Result<(&'a Located<SourceDeclaration>, &'a SourceBlock), SourceDiagnostic> {
    let declaration = source
        .declarations
        .iter()
        .find(|declaration| {
            declaration.value.kind == DeclarationKind::Function && declaration.value.name == entry
        })
        .ok_or_else(|| {
            failure(
                Span { start: 0, end: 0 },
                format!("entry function `{entry}` has no source body in this module"),
            )
        })?;
    let body = declaration
        .value
        .body
        .as_ref()
        .ok_or_else(|| failure(declaration.span, "entry function requires a source body"))?;
    Ok((declaration, body))
}

/// Split the composed entry into its plan and its task region.
///
/// The plan must be the entry's **first** statement and the module's only epoch
/// scope; the check that it is the only one is made by re-detecting the route on
/// the blanked text rather than by scanning for a second marker, so a marker
/// this function did not remove cannot be missed.
pub fn split_composed_epoch(
    source: &SourceProgram,
    entry: &str,
) -> Result<ComposedSplit, Vec<SourceDiagnostic>> {
    split(source, entry).map_err(|diagnostic| vec![diagnostic])
}

fn split(source: &SourceProgram, entry: &str) -> Result<ComposedSplit, SourceDiagnostic> {
    let (_, body) = entry_body(source, entry)?;
    let Some(first) = body.statements.first() else {
        return Err(failure(
            body.span,
            "a composed epoch entry begins with a `thread scope NAME epoch { … }` plan and \
             continues with the task region it accounts for",
        ));
    };
    if !matches!(first.value, StatementKind::ThreadScope { .. })
        || !word(header_tokens(source, first.span).get(3), "epoch")
    {
        return Err(failure(
            first.span,
            "the epoch plan is the entry's first statement; a `thread scope NAME epoch { … }` \
             elsewhere in the body is outside source.epoch.tasks.v1",
        ));
    }
    if body.statements.len() < 2 {
        return Err(failure(
            body.span,
            "a composed epoch accounts for a real task region; this entry declares a plan and \
             then does nothing",
        ));
    }
    let plan_span = first.span;
    let mut task_source = source.source().to_owned();
    let blank: String = task_source.as_bytes()[plan_span.start..plan_span.end]
        .iter()
        .map(|byte| if *byte == b'\n' { '\n' } else { ' ' })
        .collect();
    task_source.replace_range(plan_span.start..plan_span.end, &blank);
    let reparsed = parse_source_program(&task_source)
        .map_err(|error| failure(error.span, error.message.clone()))?;
    if crate::source_epoch::has_source_epoch(&reparsed) {
        return Err(failure(
            plan_span,
            "source.epoch.tasks.v1 admits exactly one epoch plan; a second epoch scope remains \
             after the plan was removed",
        ));
    }
    if crate::source_mutex::has_source_mutex(&reparsed) {
        // Measured, then refused deliberately. The mutex profile checks guard
        // authority and no accounting; the epoch profile checks accounting and
        // models no guard. Handing one body to both and publishing the pair
        // would assert a composition neither checker was given. Open as
        // `T08-COMPOSE-GUARD`.
        return Err(failure(
            plan_span,
            "source.epoch.tasks.v1 does not compose with a mutex generation (T08-COMPOSE-GUARD): \
             the epoch profile models no guard and the mutex profile models no accounting, so \
             their composition is unchecked rather than checked here",
        ));
    }
    Ok(ComposedSplit {
        plan_span,
        task_source,
    })
}

/// Read the epoch's spawn sites out of the checked task IR.
///
/// Only the entry function creates tasks: the scoped-task profile already
/// refuses a worker that spawns ("spawned workers must be first-order leaf
/// functions"), so a flat epoch is what a checked module of this profile is.
pub fn derive_ledger(
    checked: &CheckedTaskModule,
) -> Result<Vec<LedgerSpawn>, Vec<SourceDiagnostic>> {
    ledger(checked).map_err(|diagnostic| vec![diagnostic])
}

fn ledger(checked: &CheckedTaskModule) -> Result<Vec<LedgerSpawn>, SourceDiagnostic> {
    let module = checked.module();
    let creator = &module.functions[&module.entry];
    let mut spawns = Vec::new();
    for block in creator.blocks.values() {
        let TaskTerminatorKind::TrySpawn {
            worker,
            arguments,
            handle_slot,
            ..
        } = &block.terminator.kind
        else {
            continue;
        };
        let span = Span {
            start: block.terminator.span.start,
            end: block.terminator.span.end,
        };
        let worker_function = &module.functions[worker];
        let mut captures = Vec::new();
        for argument in arguments {
            captures.push(match argument {
                ScalarOperand::Slot(_) => ComposedCapture::Copy,
                ScalarOperand::Move(_) => ComposedCapture::Move,
                ScalarOperand::Loan { .. } => {
                    return Err(failure(
                        span,
                        "source.epoch.tasks.v1 admits copy and move captures only \
                         (T08-COMPOSE-LOAN): a read loan's residency is shared with the lender, \
                         which this flat accounting does not represent",
                    ));
                }
                ScalarOperand::Unit | ScalarOperand::I32(_) => {
                    return Err(failure(
                        span,
                        "a spawn argument that is neither a place copy nor a move has no \
                         composed capture spelling",
                    ));
                }
            });
        }
        let retains_result = worker_function.result == ScalarType::OwnedBuffer;
        let disposal = if retains_result {
            Some(derive_disposal(creator, *handle_slot, span)?)
        } else {
            None
        };
        spawns.push(LedgerSpawn {
            worker: worker_function.name.clone(),
            span,
            captures,
            retains_result,
            disposal,
        });
    }
    spawns.sort_by_key(|spawn| (spawn.span.start, spawn.span.end));
    Ok(spawns)
}

/// What the creator does with one completed child's owned result, read out of
/// the checked IR: every join of that handle, every elimination of the resulting
/// outcome, and then whether the creator releases each bound value.
///
/// One creation can be joined on several paths — a spawn inside a `Result`
/// match is joined in both arms — so the derivation is over the *set* of joins
/// and the *set* of values they bind, and every one of them must yield the same
/// disposal. A creation whose paths disagree has no single figure to account
/// for, and is refused rather than resolved by picking one.
///
/// Completion is never a disposal here, matching the profile: a result released
/// by the creator is live until that join, and a result the creator neither
/// releases nor hands on is live at the epoch exit.
fn derive_disposal(
    creator: &qcore_core::task_execution::TaskFunction,
    handle: usize,
    span: Span,
) -> Result<ResultDisposal, SourceDiagnostic> {
    let instructions = || {
        creator
            .blocks
            .values()
            .flat_map(|block| block.instructions.iter())
    };
    let outcomes = instructions()
        .filter_map(|instruction| match instruction.kind {
            TaskInstructionKind::Join {
                handle: joined,
                outcome_slot: Some(outcome),
            } if joined == handle => Some(outcome),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if outcomes.is_empty() {
        return Err(failure(
            span,
            "this creation's result is never bound by a join, so the composed profile cannot \
             derive its disposal",
        ));
    }
    let mut values = BTreeSet::new();
    for block in creator.blocks.values() {
        for instruction in &block.instructions {
            if let TaskInstructionKind::UnwrapInfallible {
                result_slot,
                value_slot,
            } = instruction.kind
                && outcomes.contains(&result_slot)
            {
                values.insert(value_slot);
            }
        }
        if let TaskTerminatorKind::TryOutcome {
            result_slot,
            value_slot,
            ..
        } = block.terminator.kind
            && outcomes.contains(&result_slot)
        {
            values.insert(value_slot);
        }
    }
    if values.is_empty() {
        return Err(failure(
            span,
            "this creation's outcome is never eliminated, so the composed profile cannot derive \
             its disposal",
        ));
    }
    let mut derived: Option<ResultDisposal> = None;
    for value in values {
        let released = instructions().any(|instruction| {
            matches!(instruction.kind, TaskInstructionKind::DropBuffer { buffer } if buffer == value)
        });
        let moved_on = creator
            .blocks
            .values()
            .any(|block| match &block.terminator.kind {
                TaskTerminatorKind::TrySpawn { arguments, .. } => arguments.iter().any(
                    |argument| matches!(argument, ScalarOperand::Move(slot) if *slot == value),
                ),
                TaskTerminatorKind::Return { value: returned } => {
                    matches!(returned, ScalarOperand::Move(slot) if *slot == value)
                }
                TaskTerminatorKind::Raise {
                    payload: Some(payload),
                    ..
                } => matches!(payload, ScalarOperand::Move(slot) if *slot == value),
                _ => false,
            });
        let disposal = match (released, moved_on) {
            (true, false) => ResultDisposal::ReleasedByCreator,
            (false, false) => ResultDisposal::TransferredToCreator,
            // Neither released at the join nor live in the creator at the epoch
            // exit. `T08-COMPOSE-RESULT-REUSE`.
            (_, true) => {
                return Err(failure(
                    span,
                    "the creator hands this joined result on rather than releasing it or keeping \
                     it (T08-COMPOSE-RESULT-REUSE); the flat epoch has no residency for a result \
                     that is neither",
                ));
            }
        };
        match derived {
            None => derived = Some(disposal),
            Some(existing) if existing == disposal => {}
            Some(existing) => {
                return Err(failure(
                    span,
                    format!(
                        "the creator performs `{}` on one path and `{}` on another for this \
                         creation, so the epoch has no single disposal to account for",
                        disposal_clause(existing),
                        disposal_clause(disposal),
                    ),
                ));
            }
        }
    }
    Ok(derived.expect("a nonempty value set decides one disposal"))
}

struct PlanRow {
    captures: Vec<ComposedCapture>,
    peak: Option<u64>,
    work: Option<u64>,
    credit: Option<(u64, u64)>,
    retained: Option<u64>,
    join: Option<ResultDisposal>,
    span: Span,
    worker: String,
}

#[derive(Default)]
struct Plan {
    rows: Vec<PlanRow>,
    allocation: Option<Quantity>,
    overhead: Option<RuntimeOverhead>,
    latency_claim: Option<Span>,
}

use qcore_core::resource_flow::Quantity;

/// Lower a composed entry: parse the plan, derive the ledger, reconcile the two,
/// and assemble the flat epoch. The returned program still requires
/// `check_flat_epoch`; agreement is not a bound.
pub fn lower_composed_epoch(
    source: &SourceProgram,
    entry: &str,
    plan_span: Span,
    checked: &CheckedTaskModule,
) -> Result<LoweredComposedEpoch, Vec<SourceDiagnostic>> {
    crate::module_imports::check_source_imports(source)?;
    compose(source, entry, plan_span, checked).map_err(|diagnostic| vec![diagnostic])
}

fn compose(
    source: &SourceProgram,
    entry: &str,
    plan_span: Span,
    checked: &CheckedTaskModule,
) -> Result<LoweredComposedEpoch, SourceDiagnostic> {
    let (_, body) = entry_body(source, entry)?;
    let plan_statement = body
        .statements
        .iter()
        .find(|statement| statement.span == plan_span)
        .ok_or_else(|| failure(plan_span, "the epoch plan is not a statement of this entry"))?;
    let StatementKind::ThreadScope {
        body: plan_body, ..
    } = &plan_statement.value
    else {
        return Err(failure(plan_span, "the epoch plan is not a thread scope"));
    };
    let mut plan = Plan::default();
    for statement in &plan_body.statements {
        lower_plan_statement(source, statement, &mut plan)?;
    }
    let Some(allocation) = plan.allocation.clone() else {
        return Err(failure(
            plan_span,
            "an epoch declares its whole allocation exactly once, as `bound BYTES effort UNITS;`",
        ));
    };
    let Some(overhead) = plan.overhead.clone() else {
        return Err(failure(
            plan_span,
            "an epoch declares its runtime overhead exactly once, as \
             `overhead scope BYTES thread BYTES;`",
        ));
    };
    let ledger = ledger(checked)?;
    let rows = reconcile(plan.rows, &ledger, plan_span)?;

    let tasks = rows
        .iter()
        .map(|row| {
            Ok(EpochTask {
                task: row.task,
                shared_entry: Vec::new(),
                private_peak: memory(row.peak),
                work: work(row.effort),
                result: row.result.clone(),
                credit: both(row.credit.0, row.credit.1, row.row_span)?,
                // Derived, never declared: the scoped-task ledger requires every
                // created task to be joined before its scope exits, so a checked
                // module has no unjoined creation to record.
                joined: true,
            })
        })
        .collect::<Result<Vec<_>, SourceDiagnostic>>()?;
    let program = FlatEpochProgram {
        // Derived from the ledger, never declared: the plan cannot disagree with
        // the number of creations the checked module makes.
        declared_task_count: tasks.len() as u64,
        shared_storage: Vec::new(),
        overhead,
        allocation,
        tasks,
        unsupported: Vec::new(),
    };
    let plan_record = LoweredEpochModule {
        entry: entry.to_owned(),
        program,
        cells: Vec::new(),
        tasks: rows
            .iter()
            .map(|row| EpochSourceTask {
                name: row.worker.clone(),
                task: row.task,
            })
            .collect(),
        task_spans: rows.iter().map(|row| (row.task, row.row_span)).collect(),
        cell_spans: BTreeMap::new(),
        scope_span: plan_span,
        latency_claim: plan.latency_claim,
    };
    Ok(LoweredComposedEpoch {
        plan: plan_record,
        rows: rows
            .into_iter()
            .map(|row| ComposedRow {
                task: row.task,
                worker: row.worker,
                captures: row.captures,
                result: row.result,
                disposal_source: row.disposal_source,
                row_span: row.row_span,
                spawn_span: row.spawn_span,
            })
            .collect(),
        ledger,
    })
}

/// A reconciled row before the epoch task is built from it.
struct Reconciled {
    task: EpochTaskId,
    worker: String,
    captures: Vec<ComposedCapture>,
    result: TaskResult,
    disposal_source: DisposalSource,
    peak: u64,
    effort: u64,
    credit: (u64, u64),
    row_span: Span,
    spawn_span: Span,
}

/// The agreement relation. Both directions are checked per worker, so an
/// over-declared row for one worker cannot be paid for by an undeclared spawn of
/// another.
fn reconcile(
    rows: Vec<PlanRow>,
    ledger: &[LedgerSpawn],
    plan_span: Span,
) -> Result<Vec<Reconciled>, SourceDiagnostic> {
    let mut by_worker: BTreeMap<&str, Vec<&LedgerSpawn>> = BTreeMap::new();
    for spawn in ledger {
        by_worker
            .entry(spawn.worker.as_str())
            .or_default()
            .push(spawn);
    }
    let mut declared: BTreeMap<&str, usize> = BTreeMap::new();
    for row in &rows {
        *declared.entry(row.worker.as_str()).or_default() += 1;
    }
    for (worker, sites) in &by_worker {
        let count = declared.get(worker).copied().unwrap_or(0);
        if count != sites.len() {
            return Err(failure(
                if count == 0 {
                    plan_span
                } else {
                    rows.iter()
                        .find(|row| row.worker == *worker)
                        .map_or(plan_span, |row| row.span)
                },
                format!(
                    "the plan declares {count} row(s) for worker `{worker}` and the checked task \
                     region creates {} task(s) with it; every creation is declared exactly once",
                    sites.len()
                ),
            ));
        }
    }
    for (worker, count) in &declared {
        if !by_worker.contains_key(worker) {
            let row = rows
                .iter()
                .find(|row| row.worker == **worker)
                .expect("the count came from a row");
            return Err(failure(
                row.span,
                format!(
                    "the plan declares {count} row(s) for worker `{worker}`, which the checked \
                     task region never creates"
                ),
            ));
        }
    }
    let mut taken: BTreeMap<&str, usize> = BTreeMap::new();
    let mut ordered = Vec::new();
    for row in &rows {
        let index = taken.entry(row.worker.as_str()).or_default();
        let spawn = by_worker[row.worker.as_str()][*index];
        *index += 1;
        ordered.push((row, spawn));
    }
    // Rows are numbered in the order of the creations they agreed with, so the
    // epoch's task identities follow the checked region rather than the plan's
    // text order.
    ordered.sort_by_key(|(_, spawn)| (spawn.span.start, spawn.span.end));

    let mut reconciled = Vec::new();
    for (task, (row, spawn)) in ordered.into_iter().enumerate() {
        if row.captures != spawn.captures {
            return Err(failure(
                row.span,
                format!(
                    "the plan declares captures [{}] for `{}` and the checked creation passes \
                     [{}]",
                    clause_list(&row.captures),
                    row.worker,
                    clause_list(&spawn.captures),
                ),
            ));
        }
        let Some((peak, effort, credit)) = row
            .peak
            .zip(row.work)
            .zip(row.credit)
            .map(|((peak, effort), credit)| (peak, effort, credit))
        else {
            return Err(failure(
                row.span,
                "every composed row declares `peak BYTES;`, `effort UNITS;` and \
                 `credit BYTES effort UNITS;` exactly once; no byte or work figure is derivable \
                 from this backend's Buffer, which has no extent",
            ));
        };
        let (result, disposal_source) = match (row.retained, spawn.retains_result) {
            (Some(footprint), true) => {
                let derived = spawn
                    .disposal
                    .expect("a retained ledger result carries its derived disposal");
                let source = match row.join {
                    None => DisposalSource::Derived,
                    Some(declared) if declared == derived => DisposalSource::Declared,
                    Some(declared) => {
                        return Err(failure(
                            row.span,
                            format!(
                                "the plan declares `{}` for `{}` and the checked task region \
                                 performs `{}`",
                                disposal_clause(declared),
                                row.worker,
                                disposal_clause(derived),
                            ),
                        ));
                    }
                };
                (
                    TaskResult::Retained {
                        footprint: memory(footprint),
                        disposal: derived,
                    },
                    source,
                )
            }
            (Some(_), false) => {
                return Err(failure(
                    row.span,
                    format!(
                        "the plan declares a retained result for `{}`, whose checked result type \
                         is not an owned Buffer",
                        row.worker
                    ),
                ));
            }
            (None, true) => {
                return Err(failure(
                    row.span,
                    format!(
                        "`{}` returns an owned Buffer that is live from its completion until the \
                         creator's join, so its row declares `retained BYTES;`",
                        row.worker
                    ),
                ));
            }
            (None, false) => {
                if let Some(declared) = row.join {
                    return Err(failure(
                        row.span,
                        format!(
                            "the plan declares `{}` for `{}`, which retains no result to dispose \
                             of",
                            disposal_clause(declared),
                            row.worker
                        ),
                    ));
                }
                (TaskResult::ReleasedInWorker, DisposalSource::Derived)
            }
        };
        reconciled.push(Reconciled {
            task: task as EpochTaskId + 1,
            worker: row.worker.clone(),
            captures: row.captures.clone(),
            result,
            disposal_source,
            peak,
            effort,
            credit,
            row_span: row.span,
            spawn_span: spawn.span,
        });
    }
    Ok(reconciled)
}

fn clause_list(captures: &[ComposedCapture]) -> String {
    captures
        .iter()
        .map(|capture| capture.clause())
        .collect::<Vec<_>>()
        .join(", ")
}

fn disposal_clause(disposal: ResultDisposal) -> &'static str {
    match disposal {
        ResultDisposal::TransferredToCreator => "join(move)",
        ResultDisposal::ReleasedByCreator => "join(drop)",
        ResultDisposal::DroppedAtCompletion => "join(close)",
    }
}

fn lower_plan_statement(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    plan: &mut Plan,
) -> Result<(), SourceDiagnostic> {
    let span = statement.span;
    let tokens = header_tokens(source, span);
    match &statement.value {
        StatementKind::ThreadScope { name, body } => lower_row(source, span, name, body, plan),
        StatementKind::Expression if word(tokens.first(), "bound") => {
            lower_bound(&tokens, span, plan)
        }
        StatementKind::Expression if word(tokens.first(), "overhead") => {
            lower_overhead(&tokens, span, plan)
        }
        StatementKind::Let { .. } => Err(failure(
            span,
            "source.epoch.tasks.v1 has no shared storage (T08-COMPOSE-SHARED): the copy and \
             owned task forms have no shared cell for a `share` declaration to name",
        )),
        _ => Err(failure(
            span,
            "an epoch plan admits `bound BYTES effort UNITS;`, \
             `overhead scope BYTES thread BYTES;`, `bound effort as latency;` and worker \
             `thread scope WORKER { … }` rows only",
        )),
    }
}

fn lower_bound(tokens: &[Token], span: Span, plan: &mut Plan) -> Result<(), SourceDiagnostic> {
    if word(tokens.get(1), "effort") {
        if tokens.len() != 5
            || !word(tokens.get(2), "as")
            || !word(tokens.get(3), "latency")
            || !symbol(tokens.get(4), Symbol::Semicolon)
        {
            return Err(failure(
                span,
                "reading the work bound as a time is exactly `bound effort as latency;`",
            ));
        }
        if plan.latency_claim.is_some() {
            return Err(failure(span, "the latency claim is written at most once"));
        }
        plan.latency_claim = Some(span);
        return Ok(());
    }
    let (Some(bytes), Some(units)) = (integer(tokens.get(1)), integer(tokens.get(3))) else {
        return Err(failure(
            span,
            "an epoch allocation is exactly `bound BYTES effort UNITS;`",
        ));
    };
    if tokens.len() != 5
        || !word(tokens.get(2), "effort")
        || !symbol(tokens.get(4), Symbol::Semicolon)
    {
        return Err(failure(
            span,
            "an epoch allocation is exactly `bound BYTES effort UNITS;`",
        ));
    }
    if plan.allocation.is_some() {
        return Err(failure(span, "an epoch declares one allocation"));
    }
    plan.allocation = Some(both(bytes, units, span)?);
    Ok(())
}

fn lower_overhead(tokens: &[Token], span: Span, plan: &mut Plan) -> Result<(), SourceDiagnostic> {
    let (Some(per_epoch), Some(per_task)) = (integer(tokens.get(2)), integer(tokens.get(4))) else {
        return Err(failure(
            span,
            "declared overhead is exactly `overhead scope BYTES thread BYTES;`",
        ));
    };
    if !word(tokens.get(1), "scope") || !word(tokens.get(3), "thread") {
        return Err(failure(
            span,
            "declared overhead is exactly `overhead scope BYTES thread BYTES;`",
        ));
    }
    let basis = if symbol(tokens.get(5), Symbol::Semicolon) && tokens.len() == 6 {
        OverheadBasis::Declared
    } else if tokens.len() == 8
        && word(tokens.get(5), "measured")
        && symbol(tokens.get(7), Symbol::Semicolon)
        && let Some(runs) = integer(tokens.get(6))
    {
        OverheadBasis::MeasuredNativeRun { runs }
    } else {
        return Err(failure(
            span,
            "declared overhead is `overhead scope BYTES thread BYTES;`, optionally followed by \
             `measured RUNS` before the semicolon",
        ));
    };
    if plan.overhead.is_some() {
        return Err(failure(span, "an epoch declares one runtime overhead"));
    }
    plan.overhead = Some(RuntimeOverhead {
        per_task: memory(per_task),
        per_epoch: memory(per_epoch),
        basis,
    });
    Ok(())
}

fn lower_row(
    source: &SourceProgram,
    span: Span,
    worker: &str,
    body: &SourceBlock,
    plan: &mut Plan,
) -> Result<(), SourceDiagnostic> {
    let mut row = PlanRow {
        captures: Vec::new(),
        peak: None,
        work: None,
        credit: None,
        retained: None,
        join: None,
        span,
        worker: worker.to_owned(),
    };
    for statement in &body.statements {
        lower_row_clause(source, statement, &mut row)?;
    }
    plan.rows.push(row);
    Ok(())
}

fn lower_row_clause(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    row: &mut PlanRow,
) -> Result<(), SourceDiagnostic> {
    let span = statement.span;
    let tokens = header_tokens(source, span);
    // `move;` is the parser's own `move` statement with no place, so it arrives
    // as `StatementKind::Move`, not as an expression. A capture clause carries
    // no place on purpose: what the creator captured is the ledger's fact, and
    // repeating its name here would be a second spelling of the same thing that
    // this composition would then have to reconcile against nothing.
    if let StatementKind::Move { place } = &statement.value {
        if place.is_some() || tokens.len() != 2 || !symbol(tokens.get(1), Symbol::Semicolon) {
            return Err(failure(
                span,
                "a composed capture clause is exactly `move;`; the captured place is the checked \
                 task region's, not the plan's, to name",
            ));
        }
        row.captures.push(ComposedCapture::Move);
        return Ok(());
    }
    if !matches!(statement.value, StatementKind::Expression) {
        return Err(unknown_clause(span));
    }
    match name(tokens.first()) {
        Some("copy") => {
            if tokens.len() != 2 || !symbol(tokens.get(1), Symbol::Semicolon) {
                return Err(failure(
                    span,
                    "a composed capture clause is exactly `copy;`",
                ));
            }
            row.captures.push(ComposedCapture::Copy);
            Ok(())
        }
        Some("read" | "write") => Err(failure(
            span,
            "source.epoch.tasks.v1 has no shared storage (T08-COMPOSE-SHARED) and admits no read \
             loan capture (T08-COMPOSE-LOAN)",
        )),
        Some("peak") => {
            let value = single_integer(&tokens, span, "peak")?;
            set_once(&mut row.peak, value, span, "peak")
        }
        Some("effort") => {
            let value = single_integer(&tokens, span, "effort")?;
            set_once(&mut row.work, value, span, "effort")
        }
        Some("retained") => {
            let value = single_integer(&tokens, span, "retained")?;
            set_once(&mut row.retained, value, span, "retained")
        }
        Some("credit") => {
            let (Some(bytes), Some(units)) = (integer(tokens.get(1)), integer(tokens.get(3)))
            else {
                return Err(failure(
                    span,
                    "a worker credit is exactly `credit BYTES effort UNITS;`",
                ));
            };
            if tokens.len() != 5
                || !word(tokens.get(2), "effort")
                || !symbol(tokens.get(4), Symbol::Semicolon)
            {
                return Err(failure(
                    span,
                    "a worker credit is exactly `credit BYTES effort UNITS;`",
                ));
            }
            set_once(&mut row.credit, (bytes, units), span, "credit")
        }
        Some("join") => {
            if tokens.len() != 5
                || !symbol(tokens.get(1), Symbol::LeftParen)
                || !symbol(tokens.get(3), Symbol::RightParen)
                || !symbol(tokens.get(4), Symbol::Semicolon)
            {
                return Err(join_form(span));
            }
            let disposal = match name(tokens.get(2)) {
                Some("move") => ResultDisposal::TransferredToCreator,
                Some("drop") => ResultDisposal::ReleasedByCreator,
                Some("close") => ResultDisposal::DroppedAtCompletion,
                _ => return Err(join_form(span)),
            };
            set_once(&mut row.join, disposal, span, "join")
        }
        _ => Err(unknown_clause(span)),
    }
}

fn unknown_clause(span: Span) -> SourceDiagnostic {
    failure(
        span,
        "a composed row admits `move;`, `copy;`, `peak BYTES;`, `effort UNITS;`, \
         `credit BYTES effort UNITS;`, `retained BYTES;` and at most one \
         `join(move|drop|close);` only",
    )
}

fn join_form(span: Span) -> SourceDiagnostic {
    failure(
        span,
        "the creator's join is exactly `join(move);`, `join(drop);` or `join(close);`",
    )
}

fn single_integer(tokens: &[Token], span: Span, clause: &str) -> Result<u64, SourceDiagnostic> {
    match (tokens.len(), integer(tokens.get(1))) {
        (3, Some(value)) if symbol(tokens.get(2), Symbol::Semicolon) => Ok(value),
        _ => Err(failure(
            span,
            format!("a `{clause}` clause is exactly `{clause} N;`"),
        )),
    }
}

fn set_once<T>(
    slot: &mut Option<T>,
    value: T,
    span: Span,
    clause: &str,
) -> Result<(), SourceDiagnostic> {
    if slot.is_some() {
        return Err(failure(
            span,
            format!("a composed row declares `{clause}` at most once"),
        ));
    }
    *slot = Some(value);
    Ok(())
}
