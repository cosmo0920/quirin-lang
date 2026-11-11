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
//! clause      ::= "move" ";" | "copy" ";" | "read" ";"   -- ordered captures
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
//! (`T08-COMPOSE-SHARED`). A read-loan capture — the task route's own
//! `read PLACE share N` at a spawn argument — is **admitted** since card
//! `REGION-JOINS-LOAN` (wave 30), declared as a place-free `read;` clause: the
//! wave-15 refusal reason ("a read loan's residency is shared with the lender,
//! which this flat accounting does not represent") dissolves on this route
//! because the lender is always the **creator** — a spawned worker is a leaf
//! function and lends nothing — and creator-scope bytes are already outside the
//! epoch's allocation (`REGION-JOINS-EPOCH-AXIS`), so the loan contributes no
//! epoch byte and no shared cell; the share's arithmetic, the write exclusion
//! while a share is out and the reclamation at the join stay the **unedited**
//! task ledger's own rules on the task half (`T08-COMPOSE-LOAN` discharged by
//! that card). A mutex generation in the composed body is refused
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
    EpochSourceTask, ErrorArmClause, LoweredEpochModule, both, error_arm, failure, header_tokens,
    integer, memory, name, parse_error_arm, resolve_error_arm, symbol, word, work,
};
use qcore_core::error_region::{QuantOpen as RegionQuantOpen, RegionCatchClause, RegionOp};
use qcore_core::prelude::concurrency::epoch_accounting::{
    EpochTask, EpochTaskId, FlatEpochProgram, OverheadBasis, ResultDisposal, RuntimeOverhead,
    TaskResult,
};
use qcore_core::prelude::concurrency::epoch_credit_recovery::{
    CreationErrorEdges, EpochCreditRecoveryPlan,
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
    /// `read;` — a counted read share of a creator-owned Buffer lent to the
    /// child for the creation (card `REGION-JOINS-LOAN`). The lent place and
    /// the share are the checked task region's, not the plan's, to name,
    /// exactly as the captured place of a `move;` is; the loan's bytes reside
    /// with the lender — the creator — whose allocations are outside the
    /// epoch's allocation axis (`REGION-JOINS-EPOCH-AXIS`), so the capture
    /// contributes no epoch byte figure.
    Read,
}

impl ComposedCapture {
    pub const fn clause(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
            Self::Read => "read",
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
    /// The creator's own error edges over this creation, derived from the IR
    /// (card `CT23-CREDIT-ERROR-PATH`). Never declared: a plan cannot claim an
    /// error path the checked region does not have.
    pub error_edges: CreationErrorEdges,
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
///
/// `region` is the entry's validated `error region` statement's erasure, when
/// [`read_composed_region`] found one (cards `REGION-CREDIT-JOINS` and
/// `REGION-JOINS-DUTIES`). The statement's range is blanked exactly as the
/// plan is — offsets preserved, so every span the task route reports still
/// points at the original file — and then the region's **straight-line twin**
/// is restored into that range: each body acquisition and body release
/// verbatim at its own offsets, and one `drop NAME;` per remaining duty
/// written into the blanks after them, in reverse acquisition order. A
/// duty-free region keeps nothing and inserts nothing, which is wave 28's
/// pure blanking unchanged: its body admits no statement the task program
/// performs, so the task half is byte-for-byte the program the entry runs.
pub fn split_composed_epoch(
    source: &SourceProgram,
    entry: &str,
    region: Option<&RegionErasure>,
) -> Result<ComposedSplit, Vec<SourceDiagnostic>> {
    split(source, entry, region).map_err(|diagnostic| vec![diagnostic])
}

fn split(
    source: &SourceProgram,
    entry: &str,
    region: Option<&RegionErasure>,
) -> Result<ComposedSplit, SourceDiagnostic> {
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
    let blank_range = |text: &mut String, span: Span| {
        let blank: String = text.as_bytes()[span.start..span.end]
            .iter()
            .map(|byte| if *byte == b'\n' { '\n' } else { ' ' })
            .collect();
        text.replace_range(span.start..span.end, &blank);
    };
    blank_range(&mut task_source, plan_span);
    if let Some(erasure) = region {
        blank_range(&mut task_source, erasure.span);
        // The straight-line twin: kept statements back at their own offsets…
        for kept in &erasure.kept {
            task_source.replace_range(kept.start..kept.end, &source.source()[kept.start..kept.end]);
        }
        // …and the discharge's releases over bytes the blanking made spaces.
        // The reader planned each placement over spaces only; a placement over
        // anything else here is a defect of this wiring, never of the program.
        for (offset, text) in &erasure.releases {
            let range = *offset..offset + text.len();
            if range.end > erasure.span.end
                || !task_source.as_bytes()[range.clone()]
                    .iter()
                    .all(|byte| *byte == b' ')
            {
                return Err(failure(
                    erasure.span,
                    "composed region erasure placed a release outside the blanked range \
                     (card `REGION-JOINS-DUTIES`)",
                ));
            }
            task_source.replace_range(range, text);
        }
    }
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

/// The creator's own handled cleanup: one `error region … catch (E) quant
/// open … { discharge …; }` statement inside the composed entry's task scope.
/// It may be an epilogue after every join (card `REGION-CREDIT-JOINS`) or a
/// nested statement before a later join (card `CT24-CLEANUP-TASK-COMPOSITION`).
///
/// The body admits the parent's own quantities, the decided error edge, and —
/// since card `REGION-JOINS-DUTIES` (wave 29), which deliberately supersedes
/// the duty-free premise standing constraint 15m recorded — the task route's
/// **own** acquisition and release shapes, and — since card
/// `REGION-JOINS-LOAN` (wave 30) — the adoption of a live entry owner, and
/// nothing else: `credit N;`, `effort N;`, `qcore_check(false, ERROR);`,
/// `let NAME = qcore_buffer_new(INT)?;`, `drop NAME;` of a region owner, and
/// `move NAME;` of an entry binding. An acquisition seals into the pack as a
/// real cleanup duty (`CleanupAction::Destroy`); an adoption takes a live
/// entry-scope owner's cleanup duty into the pack — the same `Destroy`, the
/// owner's acquisition standing in the entry's own text before the region —
/// and the opening's `discharge` destroys the remaining owners in reverse
/// order of appearance. With a worker capturing a counted read share of that
/// owner (`read PLACE share N` at the spawn), the region's source position
/// decides which side of the loan boundary its cleanup reaches. After the
/// join this is CT25's admitted cleanup-after-expiry shape. Before the join
/// this is CT24's cleanup-before-expiry shape: the erased `drop NAME;`
/// executes while the share is live, and the unedited task ledger refuses it.
/// A region destroying an independent owner at the same point remains
/// admitted, isolating the cleanup/loan composition from placement.
///
/// The emission story is the sequential route's own precedent read at the
/// text level: an admitted region has exactly one path, so it **erases into
/// its straight-line twin**. The split keeps each acquisition and body release
/// at its original byte offsets in the task half, and writes the discharge's
/// releases — one `drop NAME;` per remaining duty, in reverse order — into
/// the blanked space after the last kept statement ([`RegionErasure`]). The
/// task half is therefore an ordinary scoped-task program the **unedited**
/// route compiles, custody-checks, emits and reference-executes; a duty-free
/// region keeps wave 28's pure blanking byte for byte, because its erasure
/// keeps nothing and inserts nothing. The sequential spelling
/// `qcore_acquire(…)` stays refused: it names no task-route resource.
///
/// Everything semantic is the unedited [`qcore_core::error_region`] model's to
/// decide; the caller hands it `operations` whole. This reader owns only the
/// shape, under constraint 15l's rule: shape refusals on `Q302`, an undeclared
/// error type on `Q300`, and every pack obligation on the model's own
/// `Q303`–`Q309`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedRegion {
    /// The whole statement's extent — the erased range, and where the model's
    /// refusals are attributed.
    pub span: Span,
    /// Which of the two bounded source placements admitted this region. The
    /// driver retains the original after-all-task-facts check for a direct
    /// task-scope epilogue; only the exact spawn-success-arm form may precede
    /// its matching join.
    pub placement: ComposedRegionPlacement,
    /// Exactly one [`RegionOp::ErrorRegion`], built from the model's own
    /// constructors.
    pub operations: Vec<RegionOp>,
    /// The declared error the region raises and catches.
    pub raised: String,
    /// The reader's own fold of the body's `credit`/`effort` quantities — the
    /// remainder the sealed pack will carry. `None` when the fold itself
    /// overflows or underflows, in which case the model refuses first
    /// (`CreditOverflow`/`InsufficientCredit`) and this figure is never
    /// published. The driver cross-checks the model's abandoned quantity
    /// against this independent computation before publishing either.
    pub remainder: Option<u64>,
    /// The duties the sealed pack will carry — the region's owners
    /// (acquisitions and adoptions alike) the body did not release, in order
    /// of appearance, each an owner name with the resource identity the
    /// reader assigned it (1, 2, … in order of appearance). The driver
    /// cross-checks the model's own cleanup trace against this independent
    /// fold before publishing either, exactly as it cross-checks the credit
    /// (cards `REGION-JOINS-DUTIES`, `REGION-JOINS-LOAN`).
    pub duties: Vec<(String, u64)>,
    /// The module's declared nominal error type names, for the model's
    /// declaration and effect rows. Derived from the declarations rather than
    /// from the entry's written `effects` row: a caught error never leaves the
    /// entry, so it is not an entry effect (the `REGION-EFFECT-ROW` boundary,
    /// reached from a second route).
    pub declared_errors: BTreeSet<String>,
    /// How the split erases this region into the task half.
    pub erasure: RegionErasure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposedRegionPlacement {
    TaskEpilogue,
    SpawnSuccessArm,
}

/// The textual erasure of a composed region into its straight-line twin
/// (card `REGION-JOINS-DUTIES`).
///
/// The whole `span` is blanked (spaces for every byte, newlines preserved),
/// then each `kept` range — a body acquisition or a body release, at its own
/// original offsets — is restored verbatim, and each insertion of
/// `releases` — one `drop NAME;` per remaining duty, in reverse acquisition
/// order — is written into bytes the blanking made spaces, after every kept
/// range. Offsets are preserved exactly, so every span the task route reports
/// still points at the original file; a duty-free region has an empty `kept`
/// and empty `releases`, which is wave 28's pure blanking unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionErasure {
    /// The region statement's whole extent.
    pub span: Span,
    /// Body statements kept verbatim at their original offsets, in order.
    pub kept: Vec<Span>,
    /// The discharge's releases: byte offset and the exact text written there,
    /// each inside `span`, after every `kept` range, over spaces only.
    pub releases: Vec<(usize, String)>,
}

/// Find and read the composed entry's `error region` statement, if any.
///
/// Returns `Ok(None)` exactly when the program contains **no** `error region`
/// statement anywhere. The region is admitted either as the task scope's
/// direct epilogue or as a direct statement of a spawned task's exact
/// `Result::Ok` arm. Any other placement — in a worker, at entry top level, in
/// an arbitrary branch or loop, in the spawn-error arm, or more than once — is
/// a refusal, never a silent skip: a skipped region would stay in the task half
/// and refuse there with a message about the task profile rather than about
/// the composition.
pub fn read_composed_region(
    source: &SourceProgram,
    entry: &str,
) -> Result<Option<ComposedRegion>, Vec<SourceDiagnostic>> {
    composed_region(source, entry).map_err(|diagnostic| vec![diagnostic])
}

fn composed_region(
    source: &SourceProgram,
    entry: &str,
) -> Result<Option<ComposedRegion>, SourceDiagnostic> {
    let mut found: Vec<(&str, &Located<StatementKind>)> = Vec::new();
    for declaration in &source.declarations {
        if let Some(body) = &declaration.value.body {
            collect_region_statements(&declaration.value.name, body, &mut found);
        }
    }
    let Some((owner, statement)) = found.first().copied() else {
        return Ok(None);
    };
    if found.len() > 1 {
        return Err(region_shape(
            found[1].1.span,
            "source.epoch.tasks.v1 admits at most one `error region` per module in this \
             revision, exactly as the sequential route does (follow-on `REGION-MULTIPLE`)",
        ));
    }
    if owner != entry {
        return Err(region_shape(
            statement.span,
            "a spawned worker is a first-order leaf function and models no error region; \
             source.epoch.tasks.v1 reads an `error region` in the creator's own task scope only \
             (card `REGION-CREDIT-JOINS`)",
        ));
    }
    let (_, body) = entry_body(source, entry)?;
    let placement = body
        .statements
        .iter()
        .skip(1)
        .find_map(|candidate| match &candidate.value {
            StatementKind::ThreadScope { body, .. } => {
                admitted_region_placement(source, body, statement.span)
            }
            _ => None,
        });
    let Some(placement) = placement else {
        return Err(region_shape(
            statement.span,
            "the composed region is the parent's own handled cleanup, written either directly \
             in the entry's task scope or directly in the exact `Result::Ok` arm of that scope's \
             spawn; other branches, loops, the spawn-error arm, entry top level and the epoch \
             plan stay outside the bounded composition (cards `REGION-CREDIT-JOINS` and \
             `CT24-CLEANUP-TASK-COMPOSITION`)",
        ));
    };
    read_region_statement(source, statement, body, placement).map(Some)
}

fn admitted_region_placement(
    source: &SourceProgram,
    task: &SourceBlock,
    wanted: Span,
) -> Option<ComposedRegionPlacement> {
    if task
        .statements
        .iter()
        .any(|statement| statement.span == wanted)
    {
        return Some(ComposedRegionPlacement::TaskEpilogue);
    }
    task.statements.iter().find_map(|statement| {
        let StatementKind::Match { .. } = &statement.value else {
            return None;
        };
        source.result_matches().iter().find_map(|result_match| {
            (result_match.span == statement.span
                && source.task_expressions().iter().any(|expression| {
                    expression.statement_span == result_match.span
                        && matches!(expression.kind, SourceTaskExpressionKind::Spawn { .. })
                })
                && result_match.arms.iter().any(|arm| {
                    arm.pattern
                        .as_ref()
                        .is_some_and(|pattern| pattern.kind == ResultPatternKind::Ok)
                        && arm.body.statements.iter().any(|child| child.span == wanted)
                }))
            .then_some(ComposedRegionPlacement::SpawnSuccessArm)
        })
    })
}

fn collect_region_statements<'a>(
    owner: &'a str,
    block: &'a SourceBlock,
    found: &mut Vec<(&'a str, &'a Located<StatementKind>)>,
) {
    for statement in &block.statements {
        if matches!(statement.value, StatementKind::ErrorRegion { .. }) {
            found.push((owner, statement));
        }
        for child in statement.value.nested_blocks() {
            collect_region_statements(owner, child, found);
        }
    }
}

fn read_region_statement(
    source: &SourceProgram,
    statement: &Located<StatementKind>,
    entry_body: &SourceBlock,
    placement: ComposedRegionPlacement,
) -> Result<ComposedRegion, SourceDiagnostic> {
    let span = statement.span;
    let StatementKind::ErrorRegion {
        name: region_name,
        blocks,
        catch_bindings,
    } = &statement.value
    else {
        unreachable!("the caller found an error region statement");
    };
    let clauses = crate::source_region::read_catch_headers(source, blocks, catch_bindings, span)?;
    let [clause] = clauses.as_slice() else {
        return Err(region_shape(
            span,
            "source.epoch.tasks.v1 admits exactly one `catch` clause per region, exactly as the \
             sequential route does: the error-region model is path-sensitive, so a clause the \
             region's decided path does not reach is one the model does not decide (follow-on \
             `REGION-MULTIPLE-CATCH`)",
        ));
    };
    let declared_errors: BTreeSet<String> = source
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::ErrorType)
        .map(|declaration| declaration.value.name.clone())
        .collect();
    if !declared_errors.contains(&clause.error) {
        return Err(unknown_error(clause.span, &clause.error));
    }
    let mut region_body = Vec::new();
    let mut raised: Option<String> = None;
    let mut remainder: Option<u64> = Some(0);
    // The region's own owners: each name with the resource identity this
    // reader assigns it, in order of appearance, and the subset the body did
    // not release — the duties the sealed pack will carry. An owner enters
    // the region one of two ways, and the third component records which:
    // **acquired** (`false`; card `REGION-JOINS-DUTIES`) by the task route's
    // own `let NAME = qcore_buffer_new(INT)?;`, whose name must be the
    // region's own, or **adopted** (`true`; card `REGION-JOINS-LOAN`) by
    // `move NAME;`, which takes the cleanup duty of a live entry-scope owner
    // into the pack, whose name must therefore be the entry's own. The
    // identities are the reader's own (1, 2, …); the model checks their
    // ledger discipline, and the driver cross-checks its trace against this
    // independent fold.
    let mut owners: Vec<(String, u64, bool)> = Vec::new();
    let mut duties: Vec<(String, u64)> = Vec::new();
    let mut kept: Vec<Span> = Vec::new();
    let entry_bindings = entry_bindings_outside(entry_body, span);
    for body_statement in &blocks[0].statements {
        if raised.is_some() {
            return Err(region_shape(
                body_statement.span,
                "the region's error edge is decided at check time, so nothing after it runs; the \
                 composed region refuses an unreachable statement rather than emitting one",
            ));
        }
        let admitted_set = "statement is not in the composed region body: the body admits the \
                            parent's own quantities, the task route's own \
                            acquisition and release, the adoption of a live entry owner, and the \
                            decided error edge — `credit N;`, `effort N;`, \
                            `let NAME = qcore_buffer_new(INT)?;`, `drop NAME;`, `move NAME;` and \
                            `qcore_check(false, ERROR);` (cards `REGION-JOINS-DUTIES` and \
                            `REGION-JOINS-LOAN`). `qcore_acquire` is the sequential route's \
                            spelling and names no task-route resource";
        let tokens = header_tokens(source, body_statement.span);
        match &body_statement.value {
            StatementKind::Let {
                binding: Some(owner),
            } if region_acquisition(&tokens).is_some() => {
                if owners.iter().any(|(name, _, _)| name == owner) {
                    return Err(region_shape(
                        body_statement.span,
                        format!(
                            "the region acquires `{owner}` twice; each owner of the composed \
                             region is one binding (card `REGION-JOINS-DUTIES`)"
                        ),
                    ));
                }
                let resource = owners.len() as u64 + 1;
                owners.push((owner.clone(), resource, false));
                duties.push((owner.clone(), resource));
                kept.push(body_statement.span);
                region_body.push(RegionOp::Acquire {
                    resource,
                    destructor_noexcept: true,
                });
            }
            // `move NAME;` — the adoption of a live entry-scope owner (card
            // `REGION-JOINS-LOAN`): the region takes the owner's cleanup duty
            // into the pack, which from the pack's side is an acquisition —
            // the duty seals as a `Destroy` exactly as a region acquisition's
            // does. The statement erases completely (the binding already
            // stands in the entry's own text before the region), and the
            // discharge's `drop NAME;` is written into the blanks after every
            // join, where the **unedited** task route custody-checks it: an
            // adopted name that is not a live owned Buffer there — dropped,
            // moved away, a scalar, or still lent to an unjoined child —
            // refuses at that route's own codes, never here.
            StatementKind::Move { place: Some(place) } => {
                if tokens.len() != 3
                    || !word(tokens.first(), "move")
                    || !symbol(tokens.get(2), Symbol::Semicolon)
                {
                    return Err(region_shape(
                        body_statement.span,
                        "an adoption is exactly `move NAME;` (card `REGION-JOINS-LOAN`)",
                    ));
                }
                if owners.iter().any(|(name, _, _)| name == place) {
                    return Err(region_shape(
                        body_statement.span,
                        format!(
                            "the region takes `{place}` twice; each owner of the composed \
                             region is one binding (card `REGION-JOINS-LOAN`)"
                        ),
                    ));
                }
                if !entry_bindings.contains(place) {
                    return Err(region_shape(
                        body_statement.span,
                        format!(
                            "the region adopts `{place}`, which the entry does not bind: \
                             `move NAME;` takes the cleanup duty of a live task-scope owner \
                             into the pack, so the name must be a binding of the entry (card \
                             `REGION-JOINS-LOAN`)"
                        ),
                    ));
                }
                let resource = owners.len() as u64 + 1;
                owners.push((place.clone(), resource, true));
                duties.push((place.clone(), resource));
                region_body.push(RegionOp::Acquire {
                    resource,
                    destructor_noexcept: true,
                });
            }
            StatementKind::Drop { place: Some(place) } => {
                let Some((_, resource, _)) = owners.iter().find(|(name, _, _)| name == place)
                else {
                    return Err(region_shape(
                        body_statement.span,
                        format!(
                            "the region releases `{place}`, which it did not acquire: a \
                             composed region's pack holds the region's own owners only, so an \
                             outer resource is released outside it (card `REGION-JOINS-DUTIES`, \
                             the `REGION-OUTER-LIVE-RESOURCES` boundary reached from this route)"
                        ),
                    ));
                };
                let resource = *resource;
                duties.retain(|(_, id)| *id != resource);
                kept.push(body_statement.span);
                region_body.push(RegionOp::Drop(resource));
            }
            StatementKind::Expression => {
                if let Some(amount) = region_quantity(&tokens, crate::source_region::CREDIT_KEYWORD)
                {
                    remainder = remainder.and_then(|held| held.checked_add(amount));
                    region_body.push(RegionOp::AddStrictCredit(amount));
                } else if let Some(amount) =
                    region_quantity(&tokens, crate::source_region::EFFORT_KEYWORD)
                {
                    remainder = remainder.and_then(|held| held.checked_sub(amount));
                    region_body.push(RegionOp::SpendStrictCredit(amount));
                } else if let Some((passes, error)) = region_check(&tokens) {
                    if passes {
                        return Err(region_shape(
                            body_statement.span,
                            "the composed region reads `qcore_check(false, ERROR);` as its \
                             decided error edge; a check decided to pass has no edge to model \
                             and is refused rather than emitted as a dead branch",
                        ));
                    }
                    if !declared_errors.contains(&error) {
                        return Err(unknown_error(body_statement.span, &error));
                    }
                    region_body.push(RegionOp::Raise(error.clone()));
                    raised = Some(error);
                } else {
                    return Err(region_shape(body_statement.span, admitted_set));
                }
            }
            _ => return Err(region_shape(body_statement.span, admitted_set)),
        }
    }
    let Some(raised) = raised else {
        return Err(region_shape(
            span,
            "a composed region body ends in its decided error edge, \
             `qcore_check(false, ERROR);`: a region that exits normally reaches no catch clause, \
             so the model decides none of the clause's obligations (follow-on \
             `REGION-NORMAL-EXIT`)",
        ));
    };
    if clause.error != raised {
        return Err(region_shape(
            span,
            format!(
                "the region raises `{raised}` and its one catch clause names `{}`, so the raise \
                 is uncaught; no error leaves a composed region in this revision (follow-on \
                 `REGION-PROPAGATE`)",
                clause.error
            ),
        ));
    }
    let mut handler = Vec::new();
    for handler_statement in &blocks[1].statements {
        match &handler_statement.value {
            StatementKind::Discharge {
                binding: Some(binding),
            } => handler.push(RegionOp::Discharge(binding.clone())),
            _ => {
                return Err(region_shape(
                    handler_statement.span,
                    "a composed region's handler admits `discharge CAPABILITY;` only in this \
                     revision (follow-on `REGION-HANDLER-BODY`)",
                ));
            }
        }
    }
    let quant_open = match (&clause.opened_region, &clause.binding) {
        (Some(region), Some(binding)) => Some(RegionQuantOpen {
            region: region.clone(),
            binding: binding.clone(),
        }),
        _ => None,
    };
    // The erasure keeps each **acquired** owner binding at its own offsets in
    // the task half, so a name it binds must not collide with a binding the
    // entry makes elsewhere: the region body's block scope is flattened into
    // the task scope by the split, and a collision would change which binding
    // a later statement names (card `REGION-JOINS-DUTIES`). An **adopted**
    // owner is the opposite case on purpose — its name must be an entry
    // binding, checked at admission (card `REGION-JOINS-LOAN`) — so the
    // collision rule reads only the acquired ones.
    if let Some((owner, _, _)) = owners
        .iter()
        .find(|(owner, _, adopted)| !adopted && entry_bindings.contains(owner))
    {
        return Err(region_shape(
            span,
            format!(
                "the region's owner `{owner}` collides with a binding of the entry: the erased \
                 acquisition stands in the task scope, so its name must be the region's own \
                 (card `REGION-JOINS-DUTIES`)"
            ),
        ));
    }
    let releases = plan_releases(source, span, &kept, &duties)?;
    Ok(ComposedRegion {
        span,
        placement,
        operations: vec![RegionOp::ErrorRegion {
            region: region_name.clone(),
            body: region_body,
            catches: vec![RegionCatchClause {
                error: clause.error.clone(),
                quant_open,
                handler,
            }],
        }],
        raised,
        remainder,
        duties,
        declared_errors,
        erasure: RegionErasure {
            span,
            kept,
            releases,
        },
    })
}

/// Every binding name the entry makes outside the region statement: a kept
/// owner restored into the task scope must not collide with one. The scan
/// over-approximates in the refusing direction — it collects every `let`,
/// `let` index and `for` binding of every block of the entry except the
/// region's own — so a collision this misses is impossible rather than
/// admitted (card `REGION-JOINS-DUTIES`).
fn entry_bindings_outside(body: &SourceBlock, region: Span) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_bindings(body, region, &mut names);
    names
}

fn collect_bindings(block: &SourceBlock, region: Span, names: &mut BTreeSet<String>) {
    for statement in &block.statements {
        if statement.span == region {
            continue;
        }
        match &statement.value {
            StatementKind::Let {
                binding: Some(name),
            }
            | StatementKind::LetIndex { binding: name }
            | StatementKind::For { binding: name, .. } => {
                names.insert(name.clone());
            }
            _ => {}
        }
        for child in statement.value.nested_blocks() {
            collect_bindings(child, region, names);
        }
    }
}

/// Plan where the discharge's releases are written: one `drop NAME;` per
/// remaining duty, in **reverse acquisition order** — the order the model's
/// own discharge destroys them — each placed over a run of bytes the blanking
/// turns into spaces, after every kept statement and inside the region's own
/// extent. First fit, newlines never crossed, so the placement is
/// deterministic and every byte offset of the file is preserved. A region
/// whose own text has no room for its erasure is refused rather than
/// approximated (`REGION-JOINS-ERASURE-FIT`).
fn plan_releases(
    source: &SourceProgram,
    span: Span,
    kept: &[Span],
    duties: &[(String, u64)],
) -> Result<Vec<(usize, String)>, SourceDiagnostic> {
    let text = source.source().as_bytes();
    let mut cursor = kept.iter().map(|span| span.end).max().unwrap_or(span.start);
    let mut releases = Vec::new();
    for (owner, _) in duties.iter().rev() {
        let release = format!("drop {owner};");
        let need = release.len();
        let mut run = 0usize;
        let mut placed = None;
        for (index, byte) in text.iter().enumerate().take(span.end).skip(cursor) {
            if *byte == b'\n' {
                run = 0;
                continue;
            }
            run += 1;
            if run == need {
                placed = Some(index + 1 - need);
                break;
            }
        }
        let Some(offset) = placed else {
            return Err(region_shape(
                span,
                format!(
                    "the region's erasure does not fit inside the statement it erases: no room \
                     remains for `{release}` after the region's own statements \
                     (`REGION-JOINS-ERASURE-FIT`, card `REGION-JOINS-DUTIES`)"
                ),
            ));
        };
        releases.push((offset, release));
        cursor = offset + need + 1;
    }
    Ok(releases)
}

/// A shape refusal of the composed region reader. Always `Q302`, under
/// constraint 15l's rule: every condition here is a shape condition of the
/// `ErrorRegion` pass's own generic presentation code, reached from a second
/// route (the `SOCKET-EVENT-COMPOSE` precedent).
fn region_shape(span: Span, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q302",
        pass: SemanticPass::ErrorRegion,
        span,
        message: message.into(),
    }
}

/// An undeclared error type named by the region. `Q300`, the typed-error
/// band's own condition, exactly as the sequential route reports it.
fn unknown_error(span: Span, error: &str) -> SourceDiagnostic {
    SourceDiagnostic {
        code: "Q300",
        pass: SemanticPass::TypedErrors,
        span,
        message: format!("unknown error type `{error}`"),
    }
}

/// `credit N;` / `effort N;` — the same three-token shape the sequential
/// route's reader accepts, read from this route's own statement tokens.
fn region_quantity(tokens: &[Token], keyword: &str) -> Option<u64> {
    match tokens {
        [first, second, third]
            if word(Some(first), keyword) && symbol(Some(third), Symbol::Semicolon) =>
        {
            integer(Some(second))
        }
        _ => None,
    }
}

/// `let NAME = qcore_buffer_new(INT)?;` — the task route's **own** acquisition
/// shape, admitted in the composed region body (card `REGION-JOINS-DUTIES`).
/// The payload is a check-time integer literal; the statement is kept verbatim
/// in the task half, where the unedited scoped-task route checks and emits it,
/// so its runtime failure path (`?`, `AllocError`) is that route's own exactly
/// as everywhere else in the entry.
fn region_acquisition(tokens: &[Token]) -> Option<()> {
    match tokens {
        [
            let_word,
            _name,
            equals,
            intrinsic,
            open,
            payload,
            close,
            question,
            semicolon,
        ] if word(Some(let_word), "let")
            && symbol(Some(equals), Symbol::Equal)
            && word(Some(intrinsic), "qcore_buffer_new")
            && symbol(Some(open), Symbol::LeftParen)
            && integer(Some(payload)).is_some()
            && symbol(Some(close), Symbol::RightParen)
            && symbol(Some(question), Symbol::Question)
            && symbol(Some(semicolon), Symbol::Semicolon) =>
        {
            Some(())
        }
        _ => None,
    }
}

/// `qcore_check(true|false, ERROR);` — the decided edge, as written.
fn region_check(tokens: &[Token]) -> Option<(bool, String)> {
    match tokens {
        [call, open, passes, comma, error, close, semicolon]
            if word(Some(call), "qcore_check")
                && symbol(Some(open), Symbol::LeftParen)
                && symbol(Some(comma), Symbol::Comma)
                && symbol(Some(close), Symbol::RightParen)
                && symbol(Some(semicolon), Symbol::Semicolon) =>
        {
            let passes = match name(Some(passes))? {
                "true" => true,
                "false" => false,
                _ => return None,
            };
            Some((passes, name(Some(error))?.to_owned()))
        }
        _ => None,
    }
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
    // Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-EPOCH-TASKS`): the task
    // half's fixed-width surface — worker parameters and results of every
    // §G02t type, `copy` captures of any scalar, typed Buffer payloads and the
    // value statement — is admitted, because nothing the epoch accounts for
    // reads a scalar type. What the composition derives from the checked IR
    // is the epoch's *shape*: a capture's mode (a `copy` of any type is
    // `copy;`), whether a result is an owned Buffer (of any payload type),
    // its disposal, and the creator's error edges; every byte figure stays
    // declared, as it always was (`QCore.ScalarEpochTasks`). The value
    // statement adds two error paths, and both are paths this composition
    // already accounts for: a **worker's** `ArithmeticError` is an ordinary
    // typed worker error that the creator collects through `join` — the
    // join-failure edge `joins_can_fail` derives, CT23's own — and the
    // **creator's** raising statement is an error exit of the creator, which
    // the unedited scope rule admits only when no child of the scope is
    // outstanding (`ScopeBusy` otherwise), so at every admitted creator exit
    // no child holds a grant and no credit is abandoned
    // (`QCore.ScalarEpochTasks.an_admitted_creator_exit_abandons_no_child_credit`),
    // exactly as for the creator's allocation-failure exit before the card.
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
                // A read loan (card `REGION-JOINS-LOAN`, discharging
                // `T08-COMPOSE-LOAN`): the lender is the creator — a spawned
                // worker is a leaf function and lends nothing — and
                // creator-scope bytes are outside the epoch's allocation
                // (`REGION-JOINS-EPOCH-AXIS`), so the loan's residency needs
                // no epoch representation. Share arithmetic, the write
                // exclusion and the reclamation at the join are the unedited
                // task ledger's rules on the task half.
                ScalarOperand::Loan { .. } => ComposedCapture::Read,
                // A range write lease (card `WRITE-LEND-TASKS`) is refused by
                // name on the epoch composition until a card measures what a
                // leased range's residency means against an epoch's
                // allocation axis (`WRITE-LEND-TASKS-EPOCH`). Fail closed,
                // never silently classified as a read.
                ScalarOperand::Lease { .. } => {
                    return Err(failure(
                        span,
                        "a range write lease has no composed epoch capture spelling yet: \
                         `WRITE-LEND-TASKS-EPOCH` names the missing residency rule",
                    ));
                }
                ScalarOperand::Unit
                | ScalarOperand::I32(_)
                | ScalarOperand::Fixed { .. }
                | ScalarOperand::Record(_) => {
                    return Err(failure(
                        span,
                        "a spawn argument that is neither a place copy nor a move has no \
                         composed capture spelling",
                    ));
                }
            });
        }
        let retains_result = matches!(worker_function.result, ScalarType::OwnedBuffer(_));
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
            error_edges: CreationErrorEdges {
                // Every `TrySpawn` carries a `spawn_error` successor: the
                // creation is a fallible operation of the creator, and its
                // failure arm is the creator's own error path over a child that
                // was never made.
                creation_failure: true,
                join_failure: joins_can_fail(creator, *handle_slot),
            },
        });
    }
    spawns.sort_by_key(|spawn| (spawn.span.start, spawn.span.end));
    Ok(spawns)
}

/// Whether the creator's join of this creation can observe a raised child and
/// take an error edge, read out of the checked IR (card
/// `CT23-CREDIT-ERROR-PATH`).
///
/// This is the edge CT23's scenario names: the child **did** exist and **did**
/// hold the credit the epoch granted it, and the creator is now on its own error
/// path. A join whose outcome is eliminated by `UnwrapInfallible` alone has no
/// such edge, and a creation with no join at all has none either — absence of an
/// edge is recorded, never assumed in either direction.
fn joins_can_fail(creator: &qcore_core::task_execution::TaskFunction, handle: usize) -> bool {
    let outcomes = creator
        .blocks
        .values()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match instruction.kind {
            TaskInstructionKind::Join {
                handle: joined,
                outcome_slot: Some(outcome),
            } if joined == handle => Some(outcome),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    creator.blocks.values().any(|block| {
        matches!(
            block.terminator.kind,
            TaskTerminatorKind::TryOutcome { result_slot, .. } if outcomes.contains(&result_slot)
        )
    })
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
    /// `error(move|drop|close);` — the creator's error path over this row's
    /// grant (card `CT23-CREDIT-ERROR-PATH`), with the clause's own span.
    error_arm: Option<(ErrorArmClause, Span)>,
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
    // The error path over the epoch's credits. The *edges* come from the checked
    // task IR and the *disposition* from the plan; a row that declares nothing
    // spends its grant, which is the accounting this route had before card
    // `CT23-CREDIT-ERROR-PATH`.
    let mut arms = Vec::new();
    let mut error_arm_spans = BTreeMap::new();
    for row in &rows {
        let declared = match row.error_arm {
            None => None,
            Some((clause, clause_span)) => {
                error_arm_spans.insert(row.task, clause_span);
                Some(resolve_error_arm(clause, row.credit, clause_span)?)
            }
        };
        arms.push(error_arm(row.task, row.error_edges, declared));
    }
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
        recovery: EpochCreditRecoveryPlan { arms },
        error_arm_spans,
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
    /// The creator's error edges over this creation, carried from the ledger.
    error_edges: CreationErrorEdges,
    /// The row's own `error(…)` clause, unresolved, with its span.
    error_arm: Option<(ErrorArmClause, Span)>,
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
            error_edges: spawn.error_edges,
            error_arm: row.error_arm,
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
        error_arm: None,
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
        Some("read") => {
            // The read-loan capture clause (card `REGION-JOINS-LOAN`). Like
            // `move;` and `copy;` it carries no place and no figure: the lent
            // place and its share are the checked task region's, not the
            // plan's, to name. A `read NAME;` is the shared-cell read, which
            // still has no referent here.
            if tokens.len() != 2 || !symbol(tokens.get(1), Symbol::Semicolon) {
                return Err(failure(
                    span,
                    "a composed read-loan capture clause is exactly `read;` — the lent place and \
                     its share are the checked task region's, not the plan's, to name (card \
                     `REGION-JOINS-LOAN`); a shared cell read has no referent here \
                     (T08-COMPOSE-SHARED)",
                ));
            }
            row.captures.push(ComposedCapture::Read);
            Ok(())
        }
        Some("write") => Err(failure(
            span,
            "source.epoch.tasks.v1 has no shared storage (T08-COMPOSE-SHARED): a shared cell \
             write has no referent in the copy and owned task forms",
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
        Some("error") => {
            let clause = parse_error_arm(&tokens, span)?;
            set_once(&mut row.error_arm, (clause, span), span, "error")
        }
        _ => Err(unknown_clause(span)),
    }
}

fn unknown_clause(span: Span) -> SourceDiagnostic {
    failure(
        span,
        "a composed row admits `move;`, `copy;`, `read;`, `peak BYTES;`, `effort UNITS;`, \
         `credit BYTES effort UNITS;`, `retained BYTES;`, at most one \
         `join(move|drop|close);` and at most one `error(move|drop|close);` only",
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
