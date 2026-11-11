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

//! Exact source lowering for the bounded flat-epoch profile
//! `source.epoch.bound.v1`, over the unedited `FlatEpochBoundV1` checker
//! (`qcore_core::epoch_accounting`).
//!
//! The admitted surface is small, fully enumerated below, and decided here in
//! full: a module either lies inside it and lowers to a complete
//! [`FlatEpochProgram`] that the core checker validates independently, or it is
//! refused. There is no partially lowered middle state and no optimistic
//! fallback. Constructs the *profile* models as refusals become real program
//! content — an `UnsupportedEpochShape`, a cell identity the storage list does
//! not contain, a `DroppedAtCompletion` disposal, an unjoined task — so the
//! core issues the refusal from its own `Q630`-`Q641` band; constructs this
//! grammar cannot even represent are refused here at `Q600`.
//!
//! ```text
//! module      ::= entry
//! entry       ::= "fn" NAME "(" ")" "->" "unit" "{" epoch-scope "}"
//! epoch-scope ::= "thread" "scope" NAME "epoch" "{" (header | worker)* "}"
//!
//! header      ::= "let" NAME "=" "share" INT ";"     -- one shared cell, INT bytes
//!               | "bound" INT "effort" INT ";"          -- the epoch allocation, both axes
//!               | "overhead" "scope" INT "thread" INT ("measured" INT)? ";"
//!               | "bound" "effort" "as" "latency" ";"   -- read the work bound as a time
//!
//! worker      ::= "thread" "scope" NAME "{" clause* "}"
//! clause      ::= "read" NAME ";"                     -- shared entry read, repeatable
//!               | "write" NAME ";"                    -- shared mutation
//!               | "peak" INT ";"                      -- private peak bytes
//!               | "effort" INT ";"                      -- work units
//!               | "credit" INT "effort" INT ";"         -- this task's credit, both axes
//!               | "retained" INT ";"                  -- result footprint bytes
//!               | "join" "(" "move" ")" ";"           -- creator joins and takes it
//!               | "join" "(" "drop" ")" ";"           -- creator joins and releases it
//!               | "join" "(" "close" ")" ";"          -- claim: it ends at completion
//!               | "thread" "scope" NAME "{" clause* "}"   -- a nested task
//! ```
//!
//! Two axes are declared: `memory` carries every byte figure and `effort`
//! carries T08's work quantity (`work_total` on the certificate).
//! `bound 65536 effort 1000;` is the whole epoch allocation on both;
//! `credit 8192 effort 100;` is one task's share of it. The clause keyword is
//! `effort` rather than `work` deliberately: `work` is a plausible function or
//! place name in existing source — the pinned golden surface declares
//! `fn work[...]` — and reserving it would stop that source parsing.
//!
//! **The join clause is the creator's act, written in the child's block.**
//! `join(move)` transfers the completed child's result to the creator,
//! `join(drop)` releases it there, and `join(close)` is the claim that the
//! result stopped being live at completion with no transfer and no release —
//! representable precisely so the core refuses it (`Q634`) instead of it being
//! a case nobody checks. A worker with no join clause lowers to an unjoined
//! task, which is `Q635`.
//!
//! **`bound effort as latency;` is a request, not a grant.** The lowering records
//! the claim and the driver puts it to `epoch_wall_clock_bound`, which always
//! refuses (`Q639`). This profile has no schedule model — no cores, no
//! queueing, no preemption — so no quantity it derives is ever a time.
//!
//! What this lowering does **not** establish: that the declared peaks, results,
//! work, credits, cell footprints or overhead figures describe any real
//! allocation, allocator or runtime. Every one of them is a declared premise of
//! the profile (`EpochAssumption::DeclaredTaskFootprints`,
//! `DeclaredRuntimeOverhead`, `DisjointSharedStorageCells`, `NoScheduleModel`),
//! and this route imports none of them from source, from C or from a measured
//! run. There is no reference execution and no C emission on this route: an
//! admitted epoch is a checked accounting statement about a declared shape, not
//! a program this backend runs.
use std::collections::BTreeMap;

use crate::frontend::*;
use crate::lexer::{Span, Symbol, Token, TokenKind};
use qcore_core::prelude::concurrency::epoch_accounting::{
    EpochDiagnostic, EpochTask, EpochTaskId, FlatEpochProgram, OverheadBasis, ResultDisposal,
    RuntimeOverhead, SharedCellId, SharedStorageCell, TaskResult, UnsupportedEpochShape,
};
use qcore_core::prelude::concurrency::epoch_credit_recovery::{
    CreationErrorArm, CreationErrorEdges, EpochCreditDiagnostic, EpochCreditRecoveryPlan,
    ErrorArmCredit,
};
use qcore_core::resource_flow::Quantity;

/// Every byte figure of this profile lands on one axis, and every work figure
/// on the other. The checker compares componentwise, so the two never mix.
pub const MEMORY_AXIS: &str = "memory";
pub const EFFORT_AXIS: &str = "effort";

/// One shared storage cell declared by `let NAME = share INT;`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochSourceCell {
    pub name: String,
    pub cell: SharedCellId,
    pub bytes: u64,
}

/// One worker declared by a nested `thread scope NAME { … }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochSourceTask {
    pub name: String,
    pub task: EpochTaskId,
}

/// A complete lowering of one entry function. The core checker still has to
/// validate `program`; nothing here is evidence on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredEpochModule {
    pub entry: String,
    pub program: FlatEpochProgram,
    pub cells: Vec<EpochSourceCell>,
    pub tasks: Vec<EpochSourceTask>,
    /// Where each task's `thread scope` header is, for locating refusals.
    pub task_spans: BTreeMap<EpochTaskId, Span>,
    /// Where each cell identity was written. An identity that no `share`
    /// declaration introduced is recorded at its `read` statement.
    pub cell_spans: BTreeMap<SharedCellId, Span>,
    /// The epoch scope statement, the fallback span for a refusal that names
    /// neither a task nor a cell.
    pub scope_span: Span,
    /// `bound effort as latency;`, if the module asked for one. The driver must
    /// put this to `epoch_wall_clock_bound`; it is never granted here.
    pub latency_claim: Option<Span>,
    /// The error path over this epoch's credits: one arm per task, in task
    /// order (card `CT23-CREDIT-ERROR-PATH`). On **this** route every arm's
    /// [`CreationErrorEdges`] is empty, because a declared worker is not a
    /// creation the creator could fail to make; a row that nevertheless spells
    /// `error(…)` therefore reaches
    /// [`EpochCreditDiagnostic::ErrorArmWithoutErrorEdge`] from the core's own
    /// band, which is where that boundary belongs. The composed route
    /// (`source.epoch.tasks.v1`) fills the edges from the checked task IR.
    pub recovery: EpochCreditRecoveryPlan,
    /// Where each task's `error(…)` clause was written, for locating a credit
    /// refusal at the clause rather than at the row header.
    pub error_arm_spans: BTreeMap<EpochTaskId, Span>,
}

impl LoweredEpochModule {
    /// Present one core diagnostic at its own source span, under the code the
    /// core assigns it. The code is the core's; only the location is added.
    pub fn locate(&self, diagnostic: &EpochDiagnostic) -> SourceDiagnostic {
        let span = self.span_of(diagnostic);
        SourceDiagnostic {
            code: diagnostic.code(),
            pass: SemanticPass::Concurrency,
            span,
            // `Display` already prefixes the code and the presentation boundary
            // adds it again; carry the structured refusal only.
            message: format!("{diagnostic:?}"),
        }
    }

    /// Present one error-path credit refusal at its own source span, under the
    /// code the core assigns it. The code is the core's; only the location is
    /// added, exactly as in [`Self::locate`].
    ///
    /// A refusal naming a task lands on that task's `error(…)` clause when the
    /// row wrote one, and on the row header otherwise — the same rule
    /// [`Self::span_of`] uses, one level finer.
    pub fn locate_credit(&self, diagnostic: &EpochCreditDiagnostic) -> SourceDiagnostic {
        use EpochCreditDiagnostic as D;
        let task = match diagnostic {
            D::UnknownTask { task }
            | D::DuplicateArm { task }
            | D::ErrorArmWithoutErrorEdge { task }
            | D::CreditAbandonedAtErrorArm { task, .. }
            | D::RestoredCreditExceedsGrant { task, .. } => Some(task),
            D::ArmCountMismatch { .. } | D::Arithmetic { .. } => None,
        };
        let span = task
            .and_then(|task| {
                self.error_arm_spans
                    .get(task)
                    .or_else(|| self.task_spans.get(task))
                    .copied()
            })
            .unwrap_or(self.scope_span);
        SourceDiagnostic {
            code: diagnostic.code(),
            pass: SemanticPass::Concurrency,
            span,
            message: format!("{diagnostic:?}"),
        }
    }

    fn span_of(&self, diagnostic: &EpochDiagnostic) -> Span {
        use EpochDiagnostic as D;
        let by_task = |task: &EpochTaskId| self.task_spans.get(task).copied();
        let located = match diagnostic {
            D::DuplicateTask { task }
            | D::EmptyRetainedResult { task }
            | D::RetainedResultExceedsTaskPeak { task, .. }
            | D::ResultDiscardedAtCompletion { task }
            | D::UntransferredResultAtEpochExit { task }
            | D::UnjoinedTask { task }
            | D::TaskExceedsItsCredit { task, .. } => by_task(task),
            D::DuplicateSharedEntry { task, cell } | D::UnknownSharedCell { task, cell } => {
                self.cell_spans.get(cell).copied().or_else(|| by_task(task))
            }
            D::DuplicateSharedCell { cell } => self.cell_spans.get(cell).copied(),
            D::WallClockUnavailable => self.latency_claim,
            D::EmptyEpoch
            | D::TaskCountMismatch { .. }
            | D::UnsupportedEpochShape { .. }
            | D::MeasuredOverheadIsNotADeclaredBound { .. }
            | D::CreditsExceedAllocation { .. }
            | D::EpochExceedsAllocation { .. }
            | D::Arithmetic { .. }
            | D::PhaseVectorMismatch { .. } => None,
        };
        located.unwrap_or(self.scope_span)
    }
}

/// Select this route. Detection is syntactic and route-selecting only: it
/// grants no support, and a detected module that leaves the admitted grammar is
/// refused rather than handed to another route.
///
/// Card `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`: `epoch {` is not read at a
/// declaration's own name; an error named `epoch` (`error epoch { … }`)
/// selected this route before. A record so named still does (constraint 14's
/// `AGGREGATE-SELECTOR-WORDS`).
pub fn has_source_epoch(source: &SourceProgram) -> bool {
    let scope = crate::route_selection::SelectorScope::of(source);
    source.lexical_tokens().windows(2).any(|pair| {
        word(pair.first(), "epoch")
            && symbol(pair.get(1), Symbol::LeftBrace)
            && scope.reads(source, pair[0].span)
    })
}

pub(crate) fn failure(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q600",
        pass: SemanticPass::Concurrency,
        span,
        message: message.into(),
    }
}

pub(crate) fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Word(w)) if w == expected)
}

pub(crate) fn symbol(token: Option<&Token>, expected: Symbol) -> bool {
    matches!(token.map(|t| &t.kind), Some(TokenKind::Symbol(s)) if *s == expected)
}

pub(crate) fn name(token: Option<&Token>) -> Option<&str> {
    match &token?.kind {
        TokenKind::Word(w) => Some(w),
        _ => None,
    }
}

/// Integers are read exactly, at the full width the checker computes in. An
/// out-of-range literal is a refusal here, never a wrapped or clamped figure.
pub(crate) fn integer(token: Option<&Token>) -> Option<u64> {
    match &token?.kind {
        TokenKind::Integer(text) => text.parse().ok(),
        _ => None,
    }
}

/// Statement tokens, excluding any nested block: a structured statement's
/// header ends at its first `{`.
pub(crate) fn header_tokens(source: &SourceProgram, span: Span) -> Vec<Token> {
    let mut tokens = source
        .lexical_tokens()
        .iter()
        .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
        .cloned()
        .collect::<Vec<_>>();
    if let Some(brace) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Symbol(Symbol::LeftBrace))
    {
        tokens.truncate(brace);
    }
    tokens
}

/// An `error(…)` clause as written, before the row's own `credit` is known.
///
/// The clause and the credit may be written in either order, so `error(move);`
/// — "restore the whole grant" — cannot be turned into a quantity until the row
/// is complete. [`resolve_error_arm`] does that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ErrorArmClause {
    /// `error(move);` — the whole grant returns to the epoch.
    RestoreAll,
    /// `error(move BYTES effort UNITS);` — that much returns and the rest is
    /// spent. Restoring more than the grant is the mint direction and is refused
    /// by the core (`Q636`), not here: the figure is admitted as written and
    /// compared against the grant the epoch actually makes.
    Restore { bytes: u64, units: u64 },
    /// `error(drop);` — the creator's failure handling spends the whole grant.
    SpendAll,
    /// `error(close);` — neither restored nor spent. Represented so the core
    /// refuses it (`Q635`) rather than the case being one nobody checks, exactly
    /// as `join(close);` is represented for a result.
    Abandon,
}

/// `error(move|drop|close);`, or `error(move BYTES effort UNITS);`.
///
/// **No word here is new** (constraint 14): `error`, `move`, `drop`, `close` and
/// `effort` are all already reserved — the first four by the language, the fifth
/// by `T08-EPOCH-SRC` — so this clause reserves nothing and the fixture census
/// it would otherwise require is vacuous.
pub(crate) fn parse_error_arm(
    tokens: &[Token],
    span: Span,
) -> Result<ErrorArmClause, SourceDiagnostic> {
    if tokens.len() == 5
        && symbol(tokens.get(1), Symbol::LeftParen)
        && symbol(tokens.get(3), Symbol::RightParen)
        && symbol(tokens.get(4), Symbol::Semicolon)
    {
        return match name(tokens.get(2)) {
            Some("move") => Ok(ErrorArmClause::RestoreAll),
            Some("drop") => Ok(ErrorArmClause::SpendAll),
            Some("close") => Ok(ErrorArmClause::Abandon),
            _ => Err(error_arm_form(span)),
        };
    }
    if tokens.len() == 8
        && symbol(tokens.get(1), Symbol::LeftParen)
        && word(tokens.get(2), "move")
        && word(tokens.get(4), "effort")
        && symbol(tokens.get(6), Symbol::RightParen)
        && symbol(tokens.get(7), Symbol::Semicolon)
        && let (Some(bytes), Some(units)) = (integer(tokens.get(3)), integer(tokens.get(5)))
    {
        return Ok(ErrorArmClause::Restore { bytes, units });
    }
    Err(error_arm_form(span))
}

pub(crate) fn error_arm_form(span: Span) -> SourceDiagnostic {
    failure(
        span,
        "the creator's error path over this row's credit is exactly `error(move);`, \
         `error(move BYTES effort UNITS);`, `error(drop);` or `error(close);`",
    )
}

/// Resolve a written clause against the grant the row declares. `error(move);`
/// with no figures is the full rollback, so it *is* the grant; every other form
/// carries its own figures or none.
pub(crate) fn resolve_error_arm(
    clause: ErrorArmClause,
    credit: (u64, u64),
    span: Span,
) -> Result<ErrorArmCredit, SourceDiagnostic> {
    Ok(match clause {
        ErrorArmClause::RestoreAll => ErrorArmCredit::RestoredToEpoch {
            restored: both(credit.0, credit.1, span)?,
        },
        ErrorArmClause::Restore { bytes, units } => ErrorArmCredit::RestoredToEpoch {
            restored: both(bytes, units, span)?,
        },
        ErrorArmClause::SpendAll => ErrorArmCredit::SpentByCreator,
        ErrorArmClause::Abandon => ErrorArmCredit::AbandonedAtErrorArm,
    })
}

/// One task's error arm, with the edges the route supplies. The pure route
/// supplies none; the composed route derives both from the checked task IR.
pub(crate) fn error_arm(
    task: EpochTaskId,
    edges: CreationErrorEdges,
    declared: Option<ErrorArmCredit>,
) -> CreationErrorArm {
    CreationErrorArm {
        task,
        edges,
        declared,
    }
}

pub(crate) fn memory(bytes: u64) -> Quantity {
    Quantity::single(MEMORY_AXIS, bytes)
}

pub(crate) fn work(units: u64) -> Quantity {
    Quantity::single(EFFORT_AXIS, units)
}

pub(crate) fn both(bytes: u64, units: u64, span: Span) -> Result<Quantity, SourceDiagnostic> {
    Quantity::from_axes([
        (MEMORY_AXIS.to_owned(), bytes),
        (EFFORT_AXIS.to_owned(), units),
    ])
    .map_err(|error| failure(span, format!("quantity does not fit its axes: {error}")))
}

#[derive(Default)]
struct Lowering {
    cells: Vec<EpochSourceCell>,
    cell_of: BTreeMap<String, SharedCellId>,
    cell_spans: BTreeMap<SharedCellId, Span>,
    tasks: Vec<EpochTask>,
    named_tasks: Vec<EpochSourceTask>,
    task_spans: BTreeMap<EpochTaskId, Span>,
    unsupported: Vec<UnsupportedEpochShape>,
    allocation: Option<Quantity>,
    overhead: Option<RuntimeOverhead>,
    latency_claim: Option<Span>,
    /// One error arm per worker, in worker order (card `CT23-CREDIT-ERROR-PATH`).
    recovery: Vec<CreationErrorArm>,
    error_arm_spans: BTreeMap<EpochTaskId, Span>,
    next_cell: SharedCellId,
    next_task: EpochTaskId,
}

impl Lowering {
    fn fresh_cell(&mut self) -> SharedCellId {
        self.next_cell += 1;
        self.next_cell
    }
}

/// Lower one entry function to a checked-profile program.
///
/// The returned program still requires
/// `qcore_core::prelude::concurrency::epoch_accounting::check_flat_epoch`;
/// lowering establishes nothing on its own.
pub fn lower_source_epoch(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredEpochModule, Vec<SourceDiagnostic>> {
    crate::module_imports::check_source_imports(source)?;
    lower(source, typed, entry).map_err(|diagnostic| vec![diagnostic])
}

fn lower(
    source: &SourceProgram,
    typed: &TypedSourceProgram,
    entry: &str,
) -> Result<LoweredEpochModule, SourceDiagnostic> {
    let module_span = typed
        .declarations
        .first()
        .map_or(Span { start: 0, end: 0 }, |declaration| declaration.span);
    if let Some(expression) = source.task_expressions().first() {
        return Err(failure(
            expression.span,
            "the scoped task profile's `spawn`/`join` expressions are outside \
             source.epoch.bound.v1; an epoch's tasks are its nested `thread scope` blocks and \
             the creator's join is written `join(move);`, `join(drop);` or `join(close);`",
        ));
    }
    let mut entry_declaration = None;
    for declaration in &source.declarations {
        match declaration.value.kind {
            DeclarationKind::Function if declaration.value.name == entry => {
                entry_declaration = Some(declaration);
            }
            _ => {
                return Err(failure(
                    declaration.span,
                    format!(
                        "source.epoch.bound.v1 admits one entry function only; `{}` is outside \
                         the profile",
                        declaration.value.name
                    ),
                ));
            }
        }
    }
    let Some(declaration) = entry_declaration else {
        return Err(failure(
            module_span,
            format!("entry function `{entry}` has no source body in this module"),
        ));
    };
    if declaration.value.function_contract.is_some() {
        return Err(failure(
            declaration.span,
            "source contracts require an implemented consumer; the flat-epoch profile does not \
             enforce them",
        ));
    }
    if !declaration.value.parameters.is_empty()
        || declaration
            .value
            .result_type
            .as_ref()
            .and_then(SourceResultType::executable_type)
            != Some(ExecutableResultType::Unit)
    {
        return Err(failure(
            declaration.span,
            "the flat-epoch entry must be a parameter-free `-> unit` function",
        ));
    }
    let body = declaration
        .value
        .body
        .as_ref()
        .ok_or_else(|| failure(declaration.span, "entry function requires a source body"))?;
    let [scope_statement] = body.statements.as_slice() else {
        return Err(failure(
            body.span,
            "the flat-epoch entry body must be exactly one `thread scope NAME epoch { … }` \
             statement",
        ));
    };
    let StatementKind::ThreadScope {
        body: scope_body, ..
    } = &scope_statement.value
    else {
        return Err(failure(
            scope_statement.span,
            "the flat-epoch entry body must be exactly one `thread scope NAME epoch { … }` \
             statement",
        ));
    };
    let scope_tokens = header_tokens(source, scope_statement.span);
    if !word(scope_tokens.get(3), "epoch") || scope_tokens.len() != 4 {
        return Err(failure(
            scope_statement.span,
            "an epoch scope is exactly `thread scope NAME epoch { … }`",
        ));
    }

    let mut state = Lowering::default();
    for statement in &scope_body.statements {
        lower_epoch_statement(source, statement, &mut state)?;
    }

    let Some(allocation) = state.allocation.clone() else {
        return Err(failure(
            scope_statement.span,
            "an epoch declares its whole allocation exactly once, as `bound BYTES effort UNITS;`",
        ));
    };
    let Some(overhead) = state.overhead.clone() else {
        return Err(failure(
            scope_statement.span,
            "an epoch declares its runtime overhead exactly once, as \
             `overhead scope BYTES thread BYTES;`",
        ));
    };

    let program = FlatEpochProgram {
        declared_task_count: state.tasks.len() as u64,
        shared_storage: state
            .cells
            .iter()
            .map(|cell| SharedStorageCell {
                cell: cell.cell,
                footprint: memory(cell.bytes),
            })
            .collect(),
        overhead,
        allocation,
        tasks: state.tasks,
        unsupported: state.unsupported,
    };
    Ok(LoweredEpochModule {
        entry: entry.to_owned(),
        program,
        cells: state.cells,
        tasks: state.named_tasks,
        task_spans: state.task_spans,
        cell_spans: state.cell_spans,
        scope_span: scope_statement.span,
        latency_claim: state.latency_claim,
        recovery: EpochCreditRecoveryPlan {
            arms: state.recovery,
        },
        error_arm_spans: state.error_arm_spans,
    })
}

/// One statement of the epoch scope: a header the creator declares, or a worker.
fn lower_epoch_statement(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let span = statement.span;
    let tokens = header_tokens(source, span);
    match &statement.value {
        StatementKind::Let {
            binding: Some(binding),
        } => lower_cell_declaration(&tokens, span, binding, state),
        StatementKind::ThreadScope { name, body } => lower_worker(source, span, name, body, state),
        StatementKind::Expression if word(tokens.first(), "bound") => {
            lower_bound(&tokens, span, state)
        }
        StatementKind::Expression if word(tokens.first(), "overhead") => {
            lower_overhead(&tokens, span, state)
        }
        _ => Err(failure(
            span,
            "an epoch scope admits `let NAME = share BYTES;`, `bound BYTES effort UNITS;`, \
             `overhead scope BYTES thread BYTES;`, `bound effort as latency;` and worker \
             `thread scope NAME { … }` blocks only",
        )),
    }
}

fn lower_cell_declaration(
    tokens: &[Token],
    span: Span,
    binding: &str,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let Some(bytes) = integer(tokens.get(4)) else {
        return Err(failure(
            span,
            "a shared storage cell is exactly `let NAME = share BYTES;`",
        ));
    };
    if tokens.len() != 6
        || !word(tokens.get(3), "share")
        || !symbol(tokens.get(5), Symbol::Semicolon)
        || name(tokens.get(1)) != Some(binding)
    {
        return Err(failure(
            span,
            "a shared storage cell is exactly `let NAME = share BYTES;`",
        ));
    }
    if state.cell_of.contains_key(binding) {
        // Name resolution already refuses a duplicate local binding (`Q012`);
        // this keeps the map's invariant explicit rather than silently rebinding.
        return Err(failure(
            span,
            format!("shared storage cell `{binding}` is already declared"),
        ));
    }
    let cell = state.fresh_cell();
    state.cell_of.insert(binding.to_owned(), cell);
    state.cell_spans.insert(cell, span);
    state.cells.push(EpochSourceCell {
        name: binding.to_owned(),
        cell,
        bytes,
    });
    Ok(())
}

/// `bound BYTES effort UNITS;` is the epoch allocation; `bound effort as latency;`
/// asks for the work total as a completion time, which nothing here grants.
fn lower_bound(tokens: &[Token], span: Span, state: &mut Lowering) -> Result<(), SourceDiagnostic> {
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
        if state.latency_claim.is_some() {
            return Err(failure(span, "the latency claim is written at most once"));
        }
        state.latency_claim = Some(span);
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
    if state.allocation.is_some() {
        return Err(failure(span, "an epoch declares one allocation"));
    }
    state.allocation = Some(both(bytes, units, span)?);
    Ok(())
}

/// `overhead scope BYTES thread BYTES;` declares the figures; the optional
/// `measured RUNS` suffix offers a native measurement in their place, which is
/// exactly what the profile refuses (`Q640`) — an observation of some runs is
/// evidence about those runs, not a bound on every admitted epoch.
fn lower_overhead(
    tokens: &[Token],
    span: Span,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
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
    if state.overhead.is_some() {
        return Err(failure(span, "an epoch declares one runtime overhead"));
    }
    state.overhead = Some(RuntimeOverhead {
        per_task: memory(per_task),
        per_epoch: memory(per_epoch),
        basis,
    });
    Ok(())
}

/// What a worker's clauses said, before the task is assembled.
#[derive(Default)]
struct WorkerClauses {
    shared_entry: Vec<SharedCellId>,
    peak: Option<u64>,
    work: Option<u64>,
    credit: Option<(u64, u64)>,
    retained: Option<u64>,
    join: Option<ResultDisposal>,
    error_arm: Option<(ErrorArmClause, Span)>,
}

fn lower_worker(
    source: &SourceProgram,
    span: Span,
    name: &str,
    body: &SourceBlock,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    state.next_task += 1;
    let task = state.next_task;
    state.task_spans.insert(task, span);
    state.named_tasks.push(EpochSourceTask {
        name: name.to_owned(),
        task,
    });
    let mut clauses = WorkerClauses::default();
    for statement in &body.statements {
        lower_worker_clause(source, statement, &mut clauses, state)?;
    }
    let Some((credit_bytes, credit_work)) = clauses.credit else {
        return Err(failure(
            span,
            "every worker declares its share of the epoch allocation exactly once, as \
             `credit BYTES effort UNITS;`",
        ));
    };
    let result = match (clauses.retained, clauses.join) {
        (Some(bytes), disposal) => TaskResult::Retained {
            footprint: memory(bytes),
            // With no join clause the task is unjoined and the core refuses it
            // there; the disposal it would have had is not a decision this
            // lowering may invent.
            disposal: disposal.unwrap_or(ResultDisposal::ReleasedByCreator),
        },
        (None, Some(ResultDisposal::DroppedAtCompletion)) => {
            return Err(failure(
                span,
                "`join(close);` claims a retained result stopped being live at completion; this \
                 worker declares no `retained BYTES;`",
            ));
        }
        (None, _) => TaskResult::ReleasedInWorker,
    };
    // The error path over this row's credit. A declared worker has no creation
    // the creator could fail to make, so the edges stay empty here and a spelled
    // `error(…)` reaches the core's own `ErrorArmWithoutErrorEdge` refusal;
    // an elided one declares nothing about a path and spends the grant, which is
    // the accounting every epoch written before card `CT23-CREDIT-ERROR-PATH`
    // already had.
    let declared = match clauses.error_arm {
        None => None,
        Some((clause, clause_span)) => {
            state.error_arm_spans.insert(task, clause_span);
            Some(resolve_error_arm(
                clause,
                (credit_bytes, credit_work),
                clause_span,
            )?)
        }
    };
    state
        .recovery
        .push(error_arm(task, CreationErrorEdges::default(), declared));
    state.tasks.push(EpochTask {
        task,
        shared_entry: clauses.shared_entry,
        private_peak: memory(clauses.peak.unwrap_or(0)),
        work: work(clauses.work.unwrap_or(0)),
        result,
        credit: both(credit_bytes, credit_work, span)?,
        joined: clauses.join.is_some(),
    });
    Ok(())
}

fn lower_worker_clause(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    clauses: &mut WorkerClauses,
    state: &mut Lowering,
) -> Result<(), SourceDiagnostic> {
    let span = statement.span;
    let tokens = header_tokens(source, span);
    // A worker that creates a worker is not a flat epoch. Represented, so the
    // core refuses the shape rather than this grammar hiding it.
    if let StatementKind::ThreadScope { .. } = &statement.value {
        state.unsupported.push(UnsupportedEpochShape::NestedTask);
        return Ok(());
    }
    if !matches!(statement.value, StatementKind::Expression) {
        return Err(unknown_clause(span));
    }
    match name(tokens.first()) {
        Some("read") => {
            let cell = require_cell(&tokens, span, state)?;
            state.cell_spans.entry(cell).or_insert(span);
            // A repeated entry is not deduplicated: the union is the accounting
            // rule, and a repeat is evidence the plan is summing (`Q630`).
            clauses.shared_entry.push(cell);
            Ok(())
        }
        Some("write") => {
            // This profile admits shared *reads* only; a write is a represented
            // refusal, and the cell still has to name something.
            require_cell(&tokens, span, state)?;
            state
                .unsupported
                .push(UnsupportedEpochShape::SharedMutation);
            Ok(())
        }
        Some("peak") => {
            let value = single_integer(&tokens, span, "peak")?;
            set_once(&mut clauses.peak, value, span, "peak")
        }
        Some("effort") => {
            let value = single_integer(&tokens, span, "effort")?;
            set_once(&mut clauses.work, value, span, "effort")
        }
        Some("retained") => {
            let value = single_integer(&tokens, span, "retained")?;
            set_once(&mut clauses.retained, value, span, "retained")
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
            set_once(&mut clauses.credit, (bytes, units), span, "credit")
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
            set_once(&mut clauses.join, disposal, span, "join")
        }
        Some("error") => {
            let clause = parse_error_arm(&tokens, span)?;
            set_once(&mut clauses.error_arm, (clause, span), span, "error")
        }
        _ => Err(unknown_clause(span)),
    }
}

fn unknown_clause(span: Span) -> SourceDiagnostic {
    failure(
        span,
        "a worker admits `read NAME;`, `write NAME;`, `peak BYTES;`, `effort UNITS;`, \
         `credit BYTES effort UNITS;`, `retained BYTES;`, one `join(move|drop|close);` and one \
         `error(move|drop|close);` only",
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
            format!("a worker declares `{clause}` at most once"),
        ));
    }
    *slot = Some(value);
    Ok(())
}

/// The cell identity a `read`/`write` clause names. A name that no `share`
/// declaration introduced becomes a fresh identity the storage list does not
/// contain, so the core reports `UnknownSharedCell` (`Q631`) instead of this
/// grammar deciding what an unmodeled name meant.
fn require_cell(
    tokens: &[Token],
    span: Span,
    state: &mut Lowering,
) -> Result<SharedCellId, SourceDiagnostic> {
    let Some(cell_name) = name(tokens.get(1)) else {
        return Err(failure(
            span,
            "a shared entry clause names one declared cell, as `read NAME;`",
        ));
    };
    if tokens.len() != 3 || !symbol(tokens.get(2), Symbol::Semicolon) {
        return Err(failure(
            span,
            "a shared entry clause names one declared cell, as `read NAME;`",
        ));
    }
    Ok(match state.cell_of.get(cell_name) {
        Some(cell) => *cell,
        None => state.fresh_cell(),
    })
}
