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

//! `FlatEpochBoundV1` — a checked quantitative bound for one flat concurrent epoch.
//!
//! An **epoch** here is one creator that opens one scope, creates a *fixed*
//! number of *independent* tasks over *fixed-shape* shared storage, and joins
//! every one of them. Nothing else is in the profile. The bound this module
//! computes answers one question: how much of each declared axis can be live at
//! once anywhere in that epoch, and how much work does it cost.
//!
//! Three accounting rules make the bound what it is, and each has its own named
//! refusal so it cannot be quietly weakened:
//!
//! 1. **The shared entry allocation is a union, counted once.** Every task that
//!    reads a shared cell names the same cell identity, and the cell's
//!    footprint enters the bound once. Summing it per task is a *different*
//!    number, and [`recheck_epoch_bound`] names that mutation
//!    ([`EpochRecheckError::SharedUnionDoubleCounted`]) rather than accepting
//!    it as a conservative bound.
//! 2. **Concurrent private peaks overlap, so they are summed.** Independent
//!    tasks may all be running at once; the componentwise maximum is *not* a
//!    bound on that configuration, and [`epoch_residency`] exhibits the
//!    configuration that exceeds it
//!    ([`EpochRecheckError::OverlappingPeaksMaxedNotSummed`]).
//! 3. **Runtime overhead is declared, not measured.** The per-task and
//!    per-epoch overhead quantities are a *premise*
//!    ([`EpochAssumption::DeclaredRuntimeOverhead`]) that the certificate
//!    always carries. Offering a native measurement in its place is refused
//!    ([`EpochDiagnostic::MeasuredOverheadIsNotADeclaredBound`]): an observed
//!    run bounds that run, not every admitted epoch.
//!
//! A completed child's result stays live until the creator's join actually
//! transfers or releases it — completion alone reclaims nothing, matching the
//! [`crate::scoped_tasks`] capsule rule. A plan that lets a result stop being
//! live at completion is refused
//! ([`EpochDiagnostic::ResultDiscardedAtCompletion`]), and so is a certificate
//! whose residency drops it ([`EpochRecheckError::ResultsDroppedFromResidency`]).
//!
//! **This is a work bound, never a wall-clock bound.** The profile has no
//! schedule model at all: no core count, no queueing, no preemption, no
//! latency. [`epoch_wall_clock_bound`] therefore always refuses, and
//! [`EpochAssumption::NoScheduleModel`] is recorded on every certificate so a
//! consumer cannot drop it and read the work total as a completion time.
//! Nested tasks, dynamic task counts, dynamic or mutated shared storage,
//! inter-task communication, recursive workers, OS stack reservation and
//! communicating dynamic heaps are represented refusals
//! ([`UnsupportedEpochShape`]), not omissions.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::concurrency::ConcurrencyProfile;
use crate::resource_flow::{FlowError, Quantity};

/// One task of the epoch. Generation-style identity: never reused within an
/// epoch, and never a thread handle or an address.
pub type EpochTaskId = u64;
/// One fixed-shape shared storage cell, allocated by the creator before the
/// epoch and released after it.
pub type SharedCellId = u64;

/// A shared entry allocation. Distinct cells are a declared premise
/// ([`EpochAssumption::DisjointSharedStorageCells`]) to be disjoint storage, so
/// their footprints add; one cell named by many tasks is counted once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedStorageCell {
    pub cell: SharedCellId,
    pub footprint: Quantity,
}

/// What the creator does with a completed child's result, and therefore when
/// that result stops being live. Completion is never one of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultDisposal {
    /// The creator joins and keeps the result. It is live for the rest of the
    /// epoch and appears in the exit residency.
    TransferredToCreator,
    /// The creator joins and releases the result. It is live until that join.
    ReleasedByCreator,
    /// The plan claims the result stops being live at completion, with no
    /// transfer and no release. Always refused.
    DroppedAtCompletion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskResult {
    /// The worker released everything it held before completing; the completion
    /// record carries no resource footprint.
    ReleasedInWorker,
    /// A result of this footprint is live from completion until `disposal`
    /// happens at the creator's join.
    Retained {
        footprint: Quantity,
        disposal: ResultDisposal,
    },
}

/// Where the overhead figures came from. Only [`OverheadBasis::Declared`] is
/// admitted; see the module header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OverheadBasis {
    /// A declared premise. Recorded on the certificate, never discharged here.
    Declared,
    /// A native measurement offered as the overhead figure. Refused: a bounded
    /// observation of some runs is evidence about those runs, not a bound.
    MeasuredNativeRun { runs: u64 },
}

/// Runtime overhead the epoch pays beyond the program's own resources: task
/// control blocks, completion records, and whatever the epoch itself costs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeOverhead {
    /// Paid once per created task, live from creation until that task's join.
    pub per_task: Quantity,
    /// Paid once for the whole epoch.
    pub per_epoch: Quantity,
    pub basis: OverheadBasis,
}

/// Shapes this profile refuses rather than approximates. A program names the
/// ones it contains; each becomes a refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnsupportedEpochShape {
    /// A task that itself creates tasks. The epoch would not be flat.
    NestedTask,
    /// The number of tasks is decided at run time.
    DynamicTaskCount,
    /// Shared storage grown, shrunk or reallocated during the epoch.
    DynamicSharedStorage,
    /// A task writes shared storage. This profile admits shared *reads* only.
    SharedMutation,
    /// Tasks exchange data during the epoch, so they are not independent.
    InterTaskCommunication,
    /// A worker that calls itself; its residency is not a fixed declared shape.
    RecursiveWorker,
    /// A latency, makespan or scheduler-fairness claim. There is no schedule
    /// model here at all.
    SchedulerLatencyClaim,
    /// A guarantee about OS thread stacks. Stacks are not in this accounting.
    OsStackReservation,
    /// A heap whose size depends on inter-task communication.
    CommunicatingDynamicHeap,
}

/// One flat epoch, declared completely.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatEpochProgram {
    /// The fixed task count. It must equal the number of `tasks` entries; the
    /// duplication is the point, so a plan cannot disagree with itself.
    pub declared_task_count: u64,
    pub shared_storage: Vec<SharedStorageCell>,
    pub overhead: RuntimeOverhead,
    /// The whole quantity available to the epoch, per axis.
    pub allocation: Quantity,
    pub tasks: Vec<EpochTask>,
    /// Constructs present in this epoch that the profile does not model.
    pub unsupported: Vec<UnsupportedEpochShape>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochTask {
    pub task: EpochTaskId,
    /// Shared cells this task reads at entry. Union'd across tasks, never summed.
    pub shared_entry: Vec<SharedCellId>,
    /// The private quantity this task may hold while every sibling also holds
    /// its own. Declared, not derived from source.
    pub private_peak: Quantity,
    /// Work performed. Summed across tasks; never a duration.
    pub work: Quantity,
    pub result: TaskResult,
    /// This task's share of the epoch allocation.
    pub credit: Quantity,
    /// Whether the creator's join is recorded for this task. Every task of an
    /// admitted epoch is joined.
    pub joined: bool,
}

/// Premises this profile records and does not discharge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EpochAssumption {
    /// The per-task and per-epoch overhead quantities are declared figures, not
    /// measurements and not derived from a runtime.
    DeclaredRuntimeOverhead,
    /// Per-task peaks, results, work and shared-cell footprints are declared
    /// shapes; nothing here imports them from source or from an allocator.
    DeclaredTaskFootprints,
    /// Distinct shared cells occupy disjoint storage, so their footprints add.
    DisjointSharedStorageCells,
    /// There is no schedule model, so no wall-clock, makespan or latency
    /// statement follows from any quantity on this certificate.
    NoScheduleModel,
}

impl EpochAssumption {
    /// The premise as a plain string, so a free-form assumption map cannot
    /// smuggle a dropped premise past the typed set.
    pub const fn name(self) -> &'static str {
        match self {
            Self::DeclaredRuntimeOverhead => "DeclaredRuntimeOverhead",
            Self::DeclaredTaskFootprints => "DeclaredTaskFootprints",
            Self::DisjointSharedStorageCells => "DisjointSharedStorageCells",
            Self::NoScheduleModel => "NoScheduleModel",
        }
    }
}

/// Every premise of this profile. A certificate carries exactly these.
pub const EPOCH_ASSUMPTIONS: [EpochAssumption; 4] = [
    EpochAssumption::DeclaredRuntimeOverhead,
    EpochAssumption::DeclaredTaskFootprints,
    EpochAssumption::DisjointSharedStorageCells,
    EpochAssumption::NoScheduleModel,
];

/// Refusals of this profile, `Q630`-`Q641`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpochDiagnostic {
    // ---- structural epoch shape (Q630, Q631) ----
    EmptyEpoch,
    DuplicateTask {
        task: EpochTaskId,
    },
    DuplicateSharedCell {
        cell: SharedCellId,
    },
    /// One task names the same shared cell twice. Refused rather than
    /// deduplicated: the union is the accounting rule, and a repeated entry is
    /// evidence that the plan is summing.
    DuplicateSharedEntry {
        task: EpochTaskId,
        cell: SharedCellId,
    },
    TaskCountMismatch {
        declared: u64,
        actual: u64,
    },
    /// A retained result with no footprint is not a retained result; say
    /// [`TaskResult::ReleasedInWorker`].
    EmptyRetainedResult {
        task: EpochTaskId,
    },
    UnknownSharedCell {
        task: EpochTaskId,
        cell: SharedCellId,
    },

    // ---- represented refusals (Q632, Q640) ----
    UnsupportedEpochShape {
        construct: UnsupportedEpochShape,
    },
    MeasuredOverheadIsNotADeclaredBound {
        runs: u64,
    },

    // ---- residency accounting (Q633-Q635) ----
    /// The result outlives the task, so it must fit inside the peak that task
    /// was accounted for; otherwise its allocation was never counted.
    RetainedResultExceedsTaskPeak {
        task: EpochTaskId,
        axis: String,
        result: u64,
        task_peak: u64,
    },
    ResultDiscardedAtCompletion {
        task: EpochTaskId,
    },
    UntransferredResultAtEpochExit {
        task: EpochTaskId,
    },
    UnjoinedTask {
        task: EpochTaskId,
    },

    // ---- credits and allocation (Q636-Q638) ----
    TaskExceedsItsCredit {
        task: EpochTaskId,
        axis: String,
        demand: u64,
        credit: u64,
    },
    CreditsExceedAllocation {
        axis: String,
        granted: u64,
        allocation: u64,
    },
    EpochExceedsAllocation {
        axis: String,
        required: u64,
        allocation: u64,
    },

    // ---- no schedule model (Q639) ----
    /// A wall-clock, makespan or latency reading was requested. Work is not
    /// time; deriving one from the other needs a schedule model this profile
    /// does not have.
    WallClockUnavailable,

    // ---- arithmetic (Q641) ----
    Arithmetic {
        detail: FlowError,
    },

    /// A residency query supplied a phase vector that does not match the epoch.
    PhaseVectorMismatch {
        expected: usize,
        supplied: usize,
    },
}

impl EpochDiagnostic {
    /// The diagnostic code this refusal surfaces as. This profile's refusals
    /// occupy `Q630`-`Q641` inside the concurrency pass's `Q6xx` range;
    /// structural shape refusals share `Q630` and the two join-window refusals
    /// share `Q635`.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyEpoch
            | Self::DuplicateTask { .. }
            | Self::DuplicateSharedCell { .. }
            | Self::DuplicateSharedEntry { .. }
            | Self::TaskCountMismatch { .. }
            | Self::EmptyRetainedResult { .. }
            | Self::PhaseVectorMismatch { .. } => "Q630",
            Self::UnknownSharedCell { .. } => "Q631",
            Self::UnsupportedEpochShape { .. } => "Q632",
            Self::RetainedResultExceedsTaskPeak { .. } => "Q633",
            Self::ResultDiscardedAtCompletion { .. } => "Q634",
            Self::UntransferredResultAtEpochExit { .. } | Self::UnjoinedTask { .. } => "Q635",
            Self::TaskExceedsItsCredit { .. } => "Q636",
            Self::CreditsExceedAllocation { .. } => "Q637",
            Self::EpochExceedsAllocation { .. } => "Q638",
            Self::WallClockUnavailable => "Q639",
            Self::MeasuredOverheadIsNotADeclaredBound { .. } => "Q640",
            Self::Arithmetic { .. } => "Q641",
        }
    }
}

impl fmt::Display for EpochDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {self:?}", self.code())
    }
}

/// The checked bound for one admitted epoch. Every quantity is exact for the
/// declared shape: this is not an over-approximation that a consumer may
/// tighten or widen at will, and [`recheck_epoch_bound`] rederives all of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpochBoundCertificate {
    /// Always [`ConcurrencyProfile::FlatEpochBoundV1`].
    pub profile: ConcurrencyProfile,
    /// Always exactly [`EPOCH_ASSUMPTIONS`].
    pub assumptions: BTreeSet<EpochAssumption>,
    pub task_count: u64,
    /// The union of every referenced shared cell, each counted once.
    pub shared_entry_union: Quantity,
    /// The sum — not the maximum — of the tasks' private peaks.
    pub concurrent_private_peak: Quantity,
    /// `per_epoch + task_count * per_task`.
    pub runtime_overhead: Quantity,
    /// `shared_entry_union + concurrent_private_peak + runtime_overhead`.
    /// No admitted configuration exceeds it; the all-running configuration
    /// attains it.
    pub epoch_peak: Quantity,
    /// Residency with every task completed and none joined: results are still
    /// live, and so is every per-task overhead.
    pub completed_residency: Quantity,
    /// Residency after the last join and before the creator releases the
    /// shared storage: that storage, the epoch's own overhead, and the results
    /// the creator took custody of. Every per-task overhead is gone.
    pub exit_residency: Quantity,
    /// Summed work. Not a duration; see [`epoch_wall_clock_bound`].
    pub work_total: Quantity,
    pub granted_credits: Quantity,
    pub allocation: Quantity,
}

impl EpochBoundCertificate {
    pub fn assumption_names(&self) -> BTreeSet<&'static str> {
        self.assumptions
            .iter()
            .map(|assumption| assumption.name())
            .collect()
    }
}

/// Where one task is in its lifetime. Independent tasks make every assignment
/// of phases to tasks an admitted configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPhase {
    /// Not created yet: no private resources and no per-task overhead.
    Ready,
    /// Created and running: at most its private peak, plus per-task overhead.
    Running,
    /// Finished, not joined: its result and its per-task overhead are live.
    Completed,
    /// Joined: overhead reclaimed; a transferred result stays live under the
    /// creator.
    Joined,
}

/// Why an independent recheck refused a supplied certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpochRecheckError {
    /// Re-running the checker on the same program produced refusals.
    ProgramRejected(Vec<EpochDiagnostic>),
    /// A field is not what rederivation produced, and no sharper reading applies.
    FieldMismatch { field: &'static str },
    /// The shared entry allocation was counted once per referencing task
    /// instead of once. Acceptance 1.
    SharedUnionDoubleCounted { summed: Quantity, union: Quantity },
    /// The concurrent private peaks were maximized instead of summed, so a
    /// configuration in which every task runs exceeds the claimed bound.
    /// Acceptance 1.
    OverlappingPeaksMaxedNotSummed { maximum: Quantity, sum: Quantity },
    /// The claimed peak is below the derived one on this axis. The unsound
    /// direction.
    PeakUndercounted {
        axis: String,
        claimed: u64,
        derived: u64,
    },
    /// The claimed peak is above the derived one. Not unsound, but not this
    /// epoch's bound; admitting it would admit the double count above.
    PeakOvercounted {
        axis: String,
        claimed: u64,
        derived: u64,
    },
    /// The certificate's completed-but-unjoined residency omits the retained
    /// results, as if completion reclaimed them. Acceptance 2.
    ResultsDroppedFromResidency {
        claimed: Quantity,
        derived: Quantity,
    },
    /// The certificate omits a premise this profile does not discharge.
    DischargedPremiseNotEstablished { assumption: EpochAssumption },
}

// ---------------------------------------------------------------------------
// Quantity arithmetic over the public `resource_flow` surface.
// ---------------------------------------------------------------------------

fn add(left: &Quantity, right: &Quantity) -> Result<Quantity, FlowError> {
    Quantity::from_axes(
        left.iter()
            .chain(right.iter())
            .map(|(axis, amount)| (axis.to_owned(), amount)),
    )
}

fn add_all<'a>(parts: impl IntoIterator<Item = &'a Quantity>) -> Result<Quantity, FlowError> {
    let mut total = Quantity::zero();
    for part in parts {
        total = add(&total, part)?;
    }
    Ok(total)
}

fn scale(quantity: &Quantity, factor: u64) -> Result<Quantity, FlowError> {
    let mut axes = Vec::new();
    for (axis, amount) in quantity.iter() {
        axes.push((
            axis.to_owned(),
            amount.checked_mul(factor).ok_or(FlowError::Overflow)?,
        ));
    }
    Quantity::from_axes(axes)
}

fn maximum(left: &Quantity, right: &Quantity) -> Quantity {
    let mut axes = BTreeMap::new();
    for (axis, amount) in left.iter().chain(right.iter()) {
        let slot = axes.entry(axis.to_owned()).or_insert(0);
        *slot = (*slot).max(amount);
    }
    Quantity::from_axes(axes).expect("componentwise maximum cannot overflow")
}

/// The first axis, in axis order, on which `required` exceeds `available`.
fn first_excess(required: &Quantity, available: &Quantity) -> Option<(String, u64, u64)> {
    required.iter().find_map(|(axis, amount)| {
        let capacity = available.get(axis);
        (amount > capacity).then(|| (axis.to_owned(), amount, capacity))
    })
}

fn result_footprint(result: &TaskResult) -> Quantity {
    match result {
        TaskResult::ReleasedInWorker => Quantity::zero(),
        TaskResult::Retained { footprint, .. } => footprint.clone(),
    }
}

fn transferred_footprint(result: &TaskResult) -> Quantity {
    match result {
        TaskResult::Retained {
            footprint,
            disposal: ResultDisposal::TransferredToCreator,
        } => footprint.clone(),
        _ => Quantity::zero(),
    }
}

// ---------------------------------------------------------------------------
// Checking
// ---------------------------------------------------------------------------

/// Structural refusals precede accounting: a malformed epoch shape is never
/// given a bound, not even a conservative one.
fn check_shape(program: &FlatEpochProgram) -> Vec<EpochDiagnostic> {
    let mut diagnostics = Vec::new();
    for &construct in &program.unsupported {
        diagnostics.push(EpochDiagnostic::UnsupportedEpochShape { construct });
    }
    if let OverheadBasis::MeasuredNativeRun { runs } = program.overhead.basis {
        diagnostics.push(EpochDiagnostic::MeasuredOverheadIsNotADeclaredBound { runs });
    }
    if program.tasks.is_empty() {
        diagnostics.push(EpochDiagnostic::EmptyEpoch);
    }
    if program.declared_task_count != program.tasks.len() as u64 {
        diagnostics.push(EpochDiagnostic::TaskCountMismatch {
            declared: program.declared_task_count,
            actual: program.tasks.len() as u64,
        });
    }
    let mut cells = BTreeSet::new();
    for cell in &program.shared_storage {
        if !cells.insert(cell.cell) {
            diagnostics.push(EpochDiagnostic::DuplicateSharedCell { cell: cell.cell });
        }
    }
    let mut seen = BTreeSet::new();
    for task in &program.tasks {
        if !seen.insert(task.task) {
            diagnostics.push(EpochDiagnostic::DuplicateTask { task: task.task });
        }
        let mut referenced = BTreeSet::new();
        for &cell in &task.shared_entry {
            if !referenced.insert(cell) {
                diagnostics.push(EpochDiagnostic::DuplicateSharedEntry {
                    task: task.task,
                    cell,
                });
            }
            if !cells.contains(&cell) {
                diagnostics.push(EpochDiagnostic::UnknownSharedCell {
                    task: task.task,
                    cell,
                });
            }
        }
        if let TaskResult::Retained { footprint, .. } = &task.result
            && footprint == &Quantity::zero()
        {
            diagnostics.push(EpochDiagnostic::EmptyRetainedResult { task: task.task });
        }
    }
    diagnostics
}

/// The union of the shared cells the epoch's tasks actually reference. Each
/// distinct cell contributes its footprint exactly once, however many tasks
/// name it.
fn shared_entry_union(program: &FlatEpochProgram) -> Result<Quantity, FlowError> {
    let referenced = program
        .tasks
        .iter()
        .flat_map(|task| task.shared_entry.iter().copied())
        .collect::<BTreeSet<_>>();
    add_all(
        program
            .shared_storage
            .iter()
            .filter(|cell| referenced.contains(&cell.cell))
            .map(|cell| &cell.footprint),
    )
}

/// The same cells counted once per referencing task. Never a bound of this
/// profile; [`recheck_epoch_bound`] uses it to name the double count.
fn shared_entry_double_count(program: &FlatEpochProgram) -> Result<Quantity, FlowError> {
    let footprints = program
        .shared_storage
        .iter()
        .map(|cell| (cell.cell, &cell.footprint))
        .collect::<BTreeMap<_, _>>();
    let mut total = Quantity::zero();
    for task in &program.tasks {
        for cell in &task.shared_entry {
            if let Some(footprint) = footprints.get(cell) {
                total = add(&total, footprint)?;
            }
        }
    }
    Ok(total)
}

/// This task's whole demand: what it holds at once plus its own overhead plus
/// the work it performs.
fn task_demand(task: &EpochTask, overhead: &RuntimeOverhead) -> Result<Quantity, FlowError> {
    add_all([&task.private_peak, &overhead.per_task, &task.work])
}

/// Check one flat epoch and derive its bound. Every refusal of the admitted
/// shape is reported; nothing outside the profile is approximated.
pub fn check_flat_epoch(
    program: &FlatEpochProgram,
) -> Result<EpochBoundCertificate, Vec<EpochDiagnostic>> {
    let shape = check_shape(program);
    if !shape.is_empty() {
        return Err(shape);
    }

    let mut diagnostics = Vec::new();
    let arithmetic = |diagnostics: &mut Vec<EpochDiagnostic>, detail: FlowError| {
        diagnostics.push(EpochDiagnostic::Arithmetic { detail });
    };

    // Per-task residency and credit rules.
    for task in &program.tasks {
        let retained = result_footprint(&task.result);
        if let Some((axis, result, task_peak)) = first_excess(&retained, &task.private_peak) {
            diagnostics.push(EpochDiagnostic::RetainedResultExceedsTaskPeak {
                task: task.task,
                axis,
                result,
                task_peak,
            });
        }
        match &task.result {
            TaskResult::Retained {
                disposal: ResultDisposal::DroppedAtCompletion,
                ..
            } => diagnostics.push(EpochDiagnostic::ResultDiscardedAtCompletion { task: task.task }),
            TaskResult::Retained { .. } if !task.joined => diagnostics
                .push(EpochDiagnostic::UntransferredResultAtEpochExit { task: task.task }),
            _ if !task.joined => {
                diagnostics.push(EpochDiagnostic::UnjoinedTask { task: task.task });
            }
            _ => {}
        }
        match task_demand(task, &program.overhead) {
            Err(detail) => arithmetic(&mut diagnostics, detail),
            Ok(demand) => {
                if let Some((axis, demand, credit)) = first_excess(&demand, &task.credit) {
                    diagnostics.push(EpochDiagnostic::TaskExceedsItsCredit {
                        task: task.task,
                        axis,
                        demand,
                        credit,
                    });
                }
            }
        }
    }

    let task_count = program.tasks.len() as u64;
    let quantities = (|| -> Result<_, FlowError> {
        let union = shared_entry_union(program)?;
        let private = add_all(program.tasks.iter().map(|task| &task.private_peak))?;
        let retained = add_all(
            &program
                .tasks
                .iter()
                .map(|task| result_footprint(&task.result))
                .collect::<Vec<_>>(),
        )?;
        let transferred = add_all(
            &program
                .tasks
                .iter()
                .map(|task| transferred_footprint(&task.result))
                .collect::<Vec<_>>(),
        )?;
        let overhead = add(
            &program.overhead.per_epoch,
            &scale(&program.overhead.per_task, task_count)?,
        )?;
        let work = add_all(program.tasks.iter().map(|task| &task.work))?;
        let credits = add_all(program.tasks.iter().map(|task| &task.credit))?;
        let peak = add_all([&union, &private, &overhead])?;
        let completed = add_all([&union, &retained, &overhead])?;
        let exit = add_all([&union, &program.overhead.per_epoch, &transferred])?;
        let required = add(&peak, &work)?;
        Ok((
            union, private, overhead, peak, completed, exit, work, credits, required,
        ))
    })();

    let (union, private, overhead, peak, completed, exit, work, credits, required) =
        match quantities {
            Ok(quantities) => quantities,
            Err(detail) => {
                arithmetic(&mut diagnostics, detail);
                return Err(diagnostics);
            }
        };

    if let Some((axis, granted, allocation)) = first_excess(&credits, &program.allocation) {
        diagnostics.push(EpochDiagnostic::CreditsExceedAllocation {
            axis,
            granted,
            allocation,
        });
    }
    if let Some((axis, required, allocation)) = first_excess(&required, &program.allocation) {
        diagnostics.push(EpochDiagnostic::EpochExceedsAllocation {
            axis,
            required,
            allocation,
        });
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    Ok(EpochBoundCertificate {
        profile: ConcurrencyProfile::FlatEpochBoundV1,
        assumptions: EPOCH_ASSUMPTIONS.into_iter().collect(),
        task_count,
        shared_entry_union: union,
        concurrent_private_peak: private,
        runtime_overhead: overhead,
        epoch_peak: peak,
        completed_residency: completed,
        exit_residency: exit,
        work_total: work,
        granted_credits: credits,
        allocation: program.allocation.clone(),
    })
}

/// The live quantity of one configuration: an assignment of a [`TaskPhase`] to
/// every task, in program order. Because the tasks are independent, every
/// assignment is admitted, which is exactly why the private peaks are summed.
pub fn epoch_residency(
    program: &FlatEpochProgram,
    phases: &[TaskPhase],
) -> Result<Quantity, EpochDiagnostic> {
    if phases.len() != program.tasks.len() {
        return Err(EpochDiagnostic::PhaseVectorMismatch {
            expected: program.tasks.len(),
            supplied: phases.len(),
        });
    }
    let arithmetic = |detail: FlowError| EpochDiagnostic::Arithmetic { detail };
    let mut live = shared_entry_union(program).map_err(arithmetic)?;
    live = add(&live, &program.overhead.per_epoch).map_err(arithmetic)?;
    let mut outstanding = 0u64;
    for (task, phase) in program.tasks.iter().zip(phases) {
        let held = match phase {
            TaskPhase::Ready => Quantity::zero(),
            TaskPhase::Running => task.private_peak.clone(),
            TaskPhase::Completed => result_footprint(&task.result),
            TaskPhase::Joined => transferred_footprint(&task.result),
        };
        if matches!(phase, TaskPhase::Running | TaskPhase::Completed) {
            outstanding += 1;
        }
        live = add(&live, &held).map_err(arithmetic)?;
    }
    add(
        &live,
        &scale(&program.overhead.per_task, outstanding).map_err(arithmetic)?,
    )
    .map_err(arithmetic)
}

/// Always refuses. A work total is not a completion time: turning one into the
/// other needs core counts, queueing and preemption, and this profile declares
/// [`EpochAssumption::NoScheduleModel`] precisely because it has none.
pub fn epoch_wall_clock_bound(
    _certificate: &EpochBoundCertificate,
) -> Result<Quantity, EpochDiagnostic> {
    Err(EpochDiagnostic::WallClockUnavailable)
}

/// Rederive every field of a supplied certificate from the program alone. The
/// two accounting mutations this profile exists to refuse — a summed shared
/// union and a maximized concurrent peak — are named, not folded into a
/// generic mismatch.
pub fn recheck_epoch_bound(
    program: &FlatEpochProgram,
    certificate: &EpochBoundCertificate,
) -> Result<(), Vec<EpochRecheckError>> {
    let derived = match check_flat_epoch(program) {
        Err(diagnostics) => return Err(vec![EpochRecheckError::ProgramRejected(diagnostics)]),
        Ok(derived) => derived,
    };
    let mut errors = Vec::new();

    for assumption in EPOCH_ASSUMPTIONS {
        if !certificate.assumptions.contains(&assumption) {
            errors.push(EpochRecheckError::DischargedPremiseNotEstablished { assumption });
        }
    }
    if certificate.assumptions.len() != EPOCH_ASSUMPTIONS.len() {
        errors.push(EpochRecheckError::FieldMismatch {
            field: "assumptions",
        });
    }
    if certificate.profile != derived.profile {
        errors.push(EpochRecheckError::FieldMismatch { field: "profile" });
    }
    if certificate.task_count != derived.task_count {
        errors.push(EpochRecheckError::FieldMismatch {
            field: "task_count",
        });
    }

    if certificate.shared_entry_union != derived.shared_entry_union {
        let summed = shared_entry_double_count(program).unwrap_or_else(|_| Quantity::zero());
        if certificate.shared_entry_union == summed {
            errors.push(EpochRecheckError::SharedUnionDoubleCounted {
                summed,
                union: derived.shared_entry_union.clone(),
            });
        } else {
            errors.push(EpochRecheckError::FieldMismatch {
                field: "shared_entry_union",
            });
        }
    }

    if certificate.concurrent_private_peak != derived.concurrent_private_peak {
        let maxed = program
            .tasks
            .iter()
            .fold(Quantity::zero(), |accumulated, task| {
                maximum(&accumulated, &task.private_peak)
            });
        if certificate.concurrent_private_peak == maxed {
            errors.push(EpochRecheckError::OverlappingPeaksMaxedNotSummed {
                maximum: maxed,
                sum: derived.concurrent_private_peak.clone(),
            });
        } else {
            errors.push(EpochRecheckError::FieldMismatch {
                field: "concurrent_private_peak",
            });
        }
    }

    if certificate.runtime_overhead != derived.runtime_overhead {
        errors.push(EpochRecheckError::FieldMismatch {
            field: "runtime_overhead",
        });
    }

    if certificate.epoch_peak != derived.epoch_peak {
        if let Some((axis, derived_amount, claimed)) =
            first_excess(&derived.epoch_peak, &certificate.epoch_peak)
        {
            errors.push(EpochRecheckError::PeakUndercounted {
                axis,
                claimed,
                derived: derived_amount,
            });
        } else if let Some((axis, claimed, derived_amount)) =
            first_excess(&certificate.epoch_peak, &derived.epoch_peak)
        {
            errors.push(EpochRecheckError::PeakOvercounted {
                axis,
                claimed,
                derived: derived_amount,
            });
        }
    }

    if certificate.completed_residency != derived.completed_residency {
        let without_results = add(&derived.shared_entry_union, &derived.runtime_overhead)
            .unwrap_or_else(|_| Quantity::zero());
        if certificate.completed_residency == without_results {
            errors.push(EpochRecheckError::ResultsDroppedFromResidency {
                claimed: certificate.completed_residency.clone(),
                derived: derived.completed_residency.clone(),
            });
        } else {
            errors.push(EpochRecheckError::FieldMismatch {
                field: "completed_residency",
            });
        }
    }

    for (field, claimed, expected) in [
        (
            "exit_residency",
            &certificate.exit_residency,
            &derived.exit_residency,
        ),
        ("work_total", &certificate.work_total, &derived.work_total),
        (
            "granted_credits",
            &certificate.granted_credits,
            &derived.granted_credits,
        ),
        ("allocation", &certificate.allocation, &derived.allocation),
    ] {
        if claimed != expected {
            errors.push(EpochRecheckError::FieldMismatch { field });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
#[path = "../tests/unit/epoch_accounting.rs"]
mod tests;
