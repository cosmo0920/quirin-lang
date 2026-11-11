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

//! `CheckedRegionTaskCreditV1` — error-region credit composed with scoped-task
//! and read-loan lifetime obligations (card T07).
//!
//! This module is a *sibling* of [`crate::error_region`] and a *client* of
//! [`crate::scoped_tasks`]. Neither of those is edited: the Cycle 6 region
//! analyzer keeps its own model and its pinned tests, and the T04 scoped-task
//! ledger is used verbatim as the custody authority, so every task, loan and
//! join rule checked here is the one the task profile already implements.
//!
//! The integration this profile adds is the *credit* discipline:
//!
//! * **Credit is a conserved scalar, and a duty is not credit.** A region frame
//!   carries `allowable` credit and a set of [`Duty`] obligations. The only
//!   operations that move credit are endowment, spending, lending to a child,
//!   return at join, and abandonment at discharge; none of them reads or writes
//!   the duty set. A checkpoint after every operation re-derives
//!   `initial = Σ frames + Σ child-held + spent + abandoned` and refuses a
//!   violation (`Q673`), so no duty can ever be converted into credit.
//! * **Abandonment is the error path's only credit sink, and it is gated.**
//!   Discharging an opened cleanup pack abandons the pack's remaining allowable
//!   credit — but only after every join it opened has been performed, every read
//!   loan it opened has expired and every unit of credit it lent to a child has
//!   come back (`Q670`, `Q671`). A failed join is terminal, so a pack whose child
//!   was never observed to finish can never be discharged.
//! * **An obligation cannot leave the thread scope on an error edge.** A normal
//!   exit closes the scope through the ledger's own `ScopeBusy` rule; an
//!   escaping error must carry no outstanding join, loan or child-credit duty
//!   (`Q672`), and its payload and remaining destroy duties are recorded on the
//!   certificate rather than dropped (`Q677`).
//!
//! Whether a duty is still outstanding is **derived from the ledger** on every
//! query, never from this module's bookkeeping: a join duty closes only when
//! `ScopedTaskLedger::join` actually returned a capsule, and a loan duty closes
//! only when the ledger stops reporting the resource as loaned.
//!
//! Out of the profile, and refused rather than approximated: cancellation,
//! resumable effects, branching control flow (each creation outcome is its own
//! program), several thread scopes, nested workers, fractional read shares,
//! typed owned error payloads, and any source or native lowering. This module
//! checks a model; it emits no C and imports no source.
//!
//! ```
//! use qcore_core::region_task_credit::*;
//! let program = RegionTaskProgram {
//!     scope: 10,
//!     root_region: "main".into(),
//!     initial_credit: 4,
//!     declared_errors: ["Work.Failed".into()].into_iter().collect(),
//!     body: vec![RegionTaskOp::Spend(4)],
//! };
//! let certificate = check_region_task_program(&program).unwrap();
//! assert_eq!(certificate.credit.spent, 4);
//! assert!(recheck_region_task_certificate(&program, &certificate).is_ok());
//! ```
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use crate::scoped_tasks::{
    HandleId, ROOT_TASK, ReservationId, ResourceId, ScopeId, ScopedTaskLedger, TaskId,
    TaskLedgerError, TaskOutcome,
};

pub type RegionName = String;
pub type CleanupBinding = String;
pub type ErrorName = String;

/// The profile label recorded on every certificate.
pub const PROFILE: &str = "CheckedRegionTaskCreditV1";

/// A live obligation opened inside a region. A duty is deliberately **not** a
/// quantity: it is never added to, subtracted from or exchanged for credit, and
/// the only way it leaves a frame or a sealed pack is by being discharged
/// through the ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Duty {
    /// A resource the root task acquired in this region and still owns.
    Release(ResourceId),
    /// A child created in this region whose handle is not yet consumed.
    Join(HandleId),
    /// A read loan opened in this region and still outstanding.
    LoanExpiry(ResourceId),
    /// Allowable credit endowed to a child and not yet returned by its join.
    ChildCredit(TaskId),
}

impl fmt::Display for Duty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Release(resource) => write!(f, "release of resource `{resource}`"),
            Self::Join(handle) => write!(f, "join of handle `{handle}`"),
            Self::LoanExpiry(resource) => write!(f, "read loan of resource `{resource}`"),
            Self::ChildCredit(child) => write!(f, "credit lent to child `{child}`"),
        }
    }
}

/// Whether a declared creation actually happened. There is no branching in this
/// profile: a program fixes one outcome, and the other outcome is a second
/// program, exactly as the CT task scenario matrices do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpawnOutcome {
    Created,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spawn {
    pub reservation: ReservationId,
    pub child: TaskId,
    pub handle: HandleId,
    /// Resources whose custody moves to the child.
    pub moves: Vec<ResourceId>,
    /// Resources lent to the child as scoped read loans (T04).
    pub loans: Vec<ResourceId>,
    /// Allowable credit endowed to the child out of the innermost frame.
    pub child_credit: u64,
    pub outcome: SpawnOutcome,
}

/// `quant open` on a catch clause: it binds the sealed cleanup pack of exactly
/// one region as a linear capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuantOpen {
    pub region: RegionName,
    pub binding: CleanupBinding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionTaskCatch {
    pub error: ErrorName,
    pub quant_open: Option<QuantOpen>,
    pub handler: Vec<RegionTaskOp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionTaskOp {
    /// The root task acquires a resource; its release becomes a duty of the
    /// innermost region.
    Acquire(ResourceId),
    Drop(ResourceId),
    Spawn(Spawn),
    /// A modeled child observation. `unspent_credit` is a declared premise.
    Complete {
        child: TaskId,
        outcome: TaskOutcome,
        resources: Vec<ResourceId>,
        unspent_credit: u64,
    },
    Join {
        handle: HandleId,
    },
    /// A join that did not observe its child finish. Terminal for the handle.
    FailJoin {
        handle: HandleId,
    },
    Spend(u64),
    MoveToPayload(ResourceId),
    DropPayload(ResourceId),
    Raise(ErrorName),
    Region {
        region: RegionName,
        credit: u64,
        body: Vec<RegionTaskOp>,
        catches: Vec<RegionTaskCatch>,
    },
    Discharge(CleanupBinding),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionTaskProgram {
    /// The single flat thread scope this profile admits.
    pub scope: ScopeId,
    /// The implicit outermost region: it is where credit enters and where the
    /// root task's own duties live.
    pub root_region: RegionName,
    pub initial_credit: u64,
    pub declared_errors: BTreeSet<ErrorName>,
    pub body: Vec<RegionTaskOp>,
}

/// The ordered, checked record of what happened to credit and to cleanup duties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreditEvent {
    Endow {
        region: RegionName,
        amount: u64,
    },
    Spend {
        region: RegionName,
        amount: u64,
    },
    Lend {
        child: TaskId,
        amount: u64,
    },
    /// Declared, not derived: this profile has no child program.
    ChildSpend {
        child: TaskId,
        amount: u64,
    },
    Return {
        child: TaskId,
        amount: u64,
    },
    Destroy(ResourceId),
    PayloadDestroy(ResourceId),
    AbandonCredit {
        region: RegionName,
        amount: u64,
    },
}

/// A region frame sealed by an error edge. It carries every duty the frame had
/// open, so a rethrow cannot silently shed one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreditPack {
    region: RegionName,
    error: ErrorName,
    allowable: u64,
    releases: Vec<ResourceId>,
    obligations: BTreeSet<Duty>,
}

impl CreditPack {
    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn error(&self) -> &str {
        &self.error
    }

    pub fn allowable(&self) -> u64 {
        self.allowable
    }

    /// Release duties in acquisition order; cleanup runs them in reverse.
    pub fn releases(&self) -> &[ResourceId] {
        &self.releases
    }

    /// Every duty this pack carries, release duties included.
    pub fn duties(&self) -> BTreeSet<Duty> {
        self.releases
            .iter()
            .map(|&resource| Duty::Release(resource))
            .chain(self.obligations.iter().copied())
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.allowable == 0 && self.releases.is_empty() && self.obligations.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExitTag {
    Normal,
    Error(ErrorName),
}

/// Premises this profile declares rather than derives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionTaskAssumption {
    /// A creation's success or failure is supplied by the program, not observed.
    DeclaredCreationOutcome,
    /// A child's unspent credit at completion is supplied, not derived from the
    /// child's own body: this profile has no child program.
    DeclaredChildAccounting,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CreditAccounting {
    pub initial: u64,
    pub spent: u64,
    /// Abandoned at a checked `Discharge`, and nowhere else.
    pub abandoned: u64,
    /// Still held when the program ended. Zero on a normal exit; on an error
    /// exit this is the escaping pack's remainder, which is *carried*, not
    /// abandoned.
    pub carried: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionTaskCertificate {
    pub profile: &'static str,
    pub scope: ScopeId,
    pub exit: ExitTag,
    pub scope_exited: bool,
    pub payload: BTreeSet<ResourceId>,
    pub escaping_pack: Option<CreditPack>,
    pub credit: CreditAccounting,
    pub trace: Vec<CreditEvent>,
    pub joined_handles: BTreeSet<HandleId>,
    pub failed_handles: BTreeSet<HandleId>,
    pub collected: Vec<(HandleId, TaskOutcome)>,
    pub assumptions: BTreeSet<RegionTaskAssumption>,
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionTaskDiagnostic {
    // ---- the eight named refusals of this profile (Q670-Q677) ----
    /// `Q670`: a cleanup pack was discharged, or a region exited, while a join
    /// or read loan it opened is still outstanding. Acceptance 1 and 2.
    PendingLifetimeObligation {
        region: RegionName,
        duty: Duty,
    },
    /// `Q671`: the same, for credit that is out with a child. Named separately
    /// so "a duty is outstanding" and "the parent is trying to abandon someone
    /// else's credit" are distinguishable refusals.
    ChildCreditOutstanding {
        region: RegionName,
        child: TaskId,
        amount: u64,
    },
    /// `Q672`: an error left the thread scope carrying a join, loan or
    /// child-credit obligation.
    ObligationEscapesScope {
        duty: Duty,
    },
    /// `Q673`: the conservation checkpoint failed. Credit was created or
    /// destroyed outside the five accounted movements.
    CreditNotConserved {
        expected: u64,
        observed: u64,
    },
    /// `Q674`: a normal exit from a region leaves unspent allowable credit.
    /// Only the error path may abandon credit.
    UnusedCredit {
        region: RegionName,
        remainder: u64,
    },
    /// `Q675`: an endowment, spend or child loan exceeds the frame's credit.
    InsufficientCredit {
        region: RegionName,
        requested: u64,
        available: u64,
    },
    /// `Q676`: an operation that opens a duty or moves credit has no region.
    ObligationOutsideRegion,
    /// `Q677`: a payload resource was still owned at an exit that cannot carry
    /// it.
    PayloadObligationDropped {
        resource: ResourceId,
    },

    // ---- carried over from the error-region pass, same conditions, same codes ----
    /// `Q300`: the raised error is not declared by the program.
    UndeclaredError(ErrorName),
    /// `Q302`: generic error-region structure.
    DuplicateActiveRegion(RegionName),
    DuplicateCatchClause(ErrorName),
    DuplicateResourceIdentity(ResourceId),
    ResourceHasNoCleanupAction(ResourceId),
    ResourceHeldByCleanupPack(ResourceId),
    PayloadResourceNotOwned(ResourceId),
    NestedPackNotDischarged,
    UnknownCleanupCapability(CleanupBinding),
    /// `Q303`: the opened region is not the sealed pack's region.
    WrongRegionOpened {
        expected: RegionName,
        actual: RegionName,
    },
    /// `Q304`: one cleanup pack consumed twice.
    CleanupPackDuplicated(CleanupBinding),
    /// `Q305`: a handler returned normally with its pack still open.
    CleanupPackNotDischarged,
    /// `Q306`: a nonempty pack reached a catch that does not open it.
    CatchWithoutRequiredQuantOpen {
        region: RegionName,
        error: ErrorName,
    },
    /// `Q307`: an error leaves a handler with an undischarged cleanup pack.
    RethrowWithUndischargedCleanup,
    /// `Q309`: one resource occurs in both the payload and a cleanup pack.
    ResourceInPayloadAndCleanupPack(ResourceId),

    // ---- structural refusals carried over from the scoped-task ledger (Q600) ----
    /// `Q600`: the scoped-task ledger refused the operation. The message is the
    /// ledger's own; this profile does not restate or soften it.
    Ledger(TaskLedgerError),
}

impl RegionTaskDiagnostic {
    /// The eight named refusals of this profile occupy the free `Q670`-`Q677`
    /// range. Refusals carried over from the error-region pass keep the exact
    /// `Q3xx` code that pass already emits for the same condition, and
    /// structural refusals carried over from the scoped-task ledger keep the
    /// concurrency pass's existing `Q600`. No code is repurposed.
    pub fn code(&self) -> &'static str {
        match self {
            Self::PendingLifetimeObligation { .. } => "Q670",
            Self::ChildCreditOutstanding { .. } => "Q671",
            Self::ObligationEscapesScope { .. } => "Q672",
            Self::CreditNotConserved { .. } => "Q673",
            Self::UnusedCredit { .. } => "Q674",
            Self::InsufficientCredit { .. } => "Q675",
            Self::ObligationOutsideRegion => "Q676",
            Self::PayloadObligationDropped { .. } => "Q677",
            Self::UndeclaredError(_) => "Q300",
            Self::WrongRegionOpened { .. } => "Q303",
            Self::CleanupPackDuplicated(_) => "Q304",
            Self::CleanupPackNotDischarged => "Q305",
            Self::CatchWithoutRequiredQuantOpen { .. } => "Q306",
            Self::RethrowWithUndischargedCleanup => "Q307",
            Self::ResourceInPayloadAndCleanupPack(_) => "Q309",
            Self::Ledger(_) => "Q600",
            _ => "Q302",
        }
    }
}

impl fmt::Display for RegionTaskDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        match self {
            Self::PendingLifetimeObligation { region, duty } => write!(
                f,
                "{code}: region `{region}` cannot abandon credit while the {duty} is outstanding"
            ),
            Self::ChildCreditOutstanding {
                region,
                child,
                amount,
            } => write!(
                f,
                "{code}: region `{region}` cannot abandon `{amount}` credit held by child `{child}`"
            ),
            Self::ObligationEscapesScope { duty } => {
                write!(f, "{code}: the {duty} escapes the thread scope")
            }
            Self::CreditNotConserved { expected, observed } => write!(
                f,
                "{code}: credit accounting is `{observed}`, expected `{expected}`"
            ),
            Self::UnusedCredit { region, remainder } => write!(
                f,
                "{code}: normal exit from region `{region}` leaves allowable credit `{remainder}`"
            ),
            Self::InsufficientCredit {
                region,
                requested,
                available,
            } => write!(
                f,
                "{code}: region `{region}` has `{available}` allowable credit, `{requested}` requested"
            ),
            Self::ObligationOutsideRegion => {
                write!(f, "{code}: the operation has no region home")
            }
            Self::PayloadObligationDropped { resource } => write!(
                f,
                "{code}: payload resource `{resource}` is still owned at this exit"
            ),
            Self::UndeclaredError(error) => {
                write!(f, "{code}: raised error `{error}` is not declared")
            }
            Self::DuplicateActiveRegion(region) => {
                write!(f, "{code}: region `{region}` is already active")
            }
            Self::DuplicateCatchClause(error) => {
                write!(f, "{code}: error `{error}` has more than one catch clause")
            }
            Self::DuplicateResourceIdentity(resource) => {
                write!(f, "{code}: resource `{resource}` has more than one owner")
            }
            Self::ResourceHasNoCleanupAction(resource) => {
                write!(
                    f,
                    "{code}: resource `{resource}` has no region cleanup duty"
                )
            }
            Self::ResourceHeldByCleanupPack(resource) => {
                write!(
                    f,
                    "{code}: resource `{resource}` is sealed in a cleanup pack"
                )
            }
            Self::PayloadResourceNotOwned(resource) => {
                write!(f, "{code}: payload does not own resource `{resource}`")
            }
            Self::NestedPackNotDischarged => write!(
                f,
                "{code}: inner cleanup pack must be discharged before propagation"
            ),
            Self::UnknownCleanupCapability(binding) => {
                write!(f, "{code}: unknown cleanup capability `{binding}`")
            }
            Self::WrongRegionOpened { expected, actual } => write!(
                f,
                "{code}: quant open refers to `{actual}`, expected region `{expected}`"
            ),
            Self::CleanupPackDuplicated(binding) => write!(
                f,
                "{code}: cleanup pack `{binding}` is consumed more than once"
            ),
            Self::CleanupPackNotDischarged => write!(f, "{code}: cleanup pack is not discharged"),
            Self::CatchWithoutRequiredQuantOpen { region, error } => write!(
                f,
                "{code}: catch for `{error}` requires quant open of region `{region}`"
            ),
            Self::RethrowWithUndischargedCleanup => {
                write!(f, "{code}: error leaves handler with undischarged cleanup")
            }
            Self::ResourceInPayloadAndCleanupPack(resource) => write!(
                f,
                "{code}: resource `{resource}` occurs in both the payload and cleanup pack"
            ),
            Self::Ledger(error) => write!(f, "{code}: {error:?}"),
        }
    }
}

impl std::error::Error for RegionTaskDiagnostic {}

impl From<TaskLedgerError> for RegionTaskDiagnostic {
    fn from(error: TaskLedgerError) -> Self {
        Self::Ledger(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateRecheckError {
    ProgramRejected(RegionTaskDiagnostic),
    FieldMismatch { field: &'static str },
    ForbiddenAssumption { assumption: RegionTaskAssumption },
}

// ---------------------------------------------------------------------------
// Checker
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
struct Frame {
    region: RegionName,
    allowable: u64,
    releases: Vec<ResourceId>,
    obligations: BTreeSet<Duty>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    ledger: ScopedTaskLedger,
    frames: Vec<Frame>,
    payload: BTreeSet<ResourceId>,
    open: Option<(CleanupBinding, CreditPack)>,
    consumed_bindings: BTreeSet<CleanupBinding>,
    sealed: Option<CreditPack>,
    child_credit: BTreeMap<TaskId, u64>,
    handle_child: BTreeMap<HandleId, TaskId>,
    joined_handles: BTreeSet<HandleId>,
    failed_handles: BTreeSet<HandleId>,
    spent: u64,
    abandoned: u64,
    trace: Vec<CreditEvent>,
    collected: Vec<(HandleId, TaskOutcome)>,
}

enum Flow {
    Normal(State),
    Raised { error: ErrorName, state: State },
}

struct Analyzer<'a> {
    program: &'a RegionTaskProgram,
}

type Checked<T> = Result<T, RegionTaskDiagnostic>;

impl State {
    /// Every unit of credit the state can account for, wherever it currently
    /// sits. The duty sets are deliberately absent from this sum.
    fn accounted(&self) -> u64 {
        let frames: u64 = self.frames.iter().map(|frame| frame.allowable).sum();
        let packs = self.open.as_ref().map_or(0, |(_, pack)| pack.allowable)
            + self.sealed.as_ref().map_or(0, |pack| pack.allowable);
        let children: u64 = self.child_credit.values().copied().sum();
        frames + packs + children + self.spent + self.abandoned
    }

    fn frame(&mut self) -> Checked<&mut Frame> {
        self.frames
            .last_mut()
            .ok_or(RegionTaskDiagnostic::ObligationOutsideRegion)
    }

    fn region_name(&self) -> RegionName {
        self.frames
            .last()
            .map(|frame| frame.region.clone())
            .unwrap_or_default()
    }

    /// Derived from the ledger, never from bookkeeping.
    fn outstanding(&self, duty: Duty) -> bool {
        match duty {
            Duty::Release(resource) => self.ledger.resource_state(resource).is_some_and(|state| {
                use crate::scoped_tasks::{Custodian, ResourceState};
                matches!(
                    state,
                    ResourceState::Live(Custodian::Task(ROOT_TASK)) | ResourceState::Loaned { .. }
                )
            }),
            Duty::Join(handle) => !self.joined_handles.contains(&handle),
            Duty::LoanExpiry(resource) => !self.ledger.loan_borrowers(resource).is_empty(),
            Duty::ChildCredit(child) => self.child_credit.contains_key(&child),
        }
    }

    /// Retire every non-release obligation the ledger now reports as closed.
    /// Retirement is one-directional: nothing is ever added here.
    fn prune(&mut self) {
        let resolved = |set: &BTreeSet<Duty>, state: &Self| -> BTreeSet<Duty> {
            set.iter()
                .copied()
                .filter(|duty| state.outstanding(*duty))
                .collect()
        };
        let snapshot = self.clone();
        for frame in &mut self.frames {
            frame.obligations = resolved(&frame.obligations, &snapshot);
        }
        if let Some((_, pack)) = self.open.as_mut() {
            pack.obligations = resolved(&pack.obligations, &snapshot);
        }
        if let Some(pack) = self.sealed.as_mut() {
            pack.obligations = resolved(&pack.obligations, &snapshot);
        }
    }

    /// Remove one resource's release duty from the innermost frame that holds
    /// it. A resource sealed in a pack is refused earlier; a resource with no
    /// duty anywhere is a modeling error, not a silent no-op.
    fn take_release(&mut self, resource: ResourceId) -> Checked<()> {
        for frame in self.frames.iter_mut().rev() {
            if let Some(index) = frame
                .releases
                .iter()
                .position(|candidate| *candidate == resource)
            {
                frame.releases.remove(index);
                return Ok(());
            }
        }
        Err(RegionTaskDiagnostic::ResourceHasNoCleanupAction(resource))
    }

    fn sealed_here(&self, resource: ResourceId) -> bool {
        self.open
            .as_ref()
            .is_some_and(|(_, pack)| pack.releases.contains(&resource))
    }
}

impl Analyzer<'_> {
    fn block(&self, mut state: State, operations: &[RegionTaskOp]) -> Checked<Flow> {
        for operation in operations {
            let expected = self.program.initial_credit;
            match self.operation(state, operation)? {
                Flow::Normal(mut next) => {
                    next.prune();
                    Self::conserved(&next, expected)?;
                    state = next;
                }
                Flow::Raised {
                    error,
                    state: mut raised,
                } => {
                    raised.prune();
                    Self::conserved(&raised, expected)?;
                    return Ok(Flow::Raised {
                        error,
                        state: raised,
                    });
                }
            }
        }
        Ok(Flow::Normal(state))
    }

    fn conserved(state: &State, expected: u64) -> Checked<()> {
        Self::conserved_with(state, 0, expected)
    }

    fn conserved_with(state: &State, extra: u64, expected: u64) -> Checked<()> {
        let observed = state.accounted() + extra;
        if observed == expected {
            Ok(())
        } else {
            Err(RegionTaskDiagnostic::CreditNotConserved { expected, observed })
        }
    }

    fn operation(&self, mut state: State, operation: &RegionTaskOp) -> Checked<Flow> {
        match operation {
            RegionTaskOp::Acquire(resource) => {
                if state.payload.contains(resource) || state.sealed_here(*resource) {
                    return Err(RegionTaskDiagnostic::DuplicateResourceIdentity(*resource));
                }
                // The frame lookup precedes the ledger change so that a refused
                // acquisition leaves the ledger untouched.
                if state.frames.is_empty() {
                    return Err(RegionTaskDiagnostic::ObligationOutsideRegion);
                }
                state.ledger.acquire(ROOT_TASK, *resource)?;
                state.frame()?.releases.push(*resource);
                Ok(Flow::Normal(state))
            }
            RegionTaskOp::Drop(resource) => {
                if state.sealed_here(*resource) {
                    return Err(RegionTaskDiagnostic::ResourceHeldByCleanupPack(*resource));
                }
                state.ledger.drop_resource(ROOT_TASK, *resource)?;
                state.take_release(*resource)?;
                Ok(Flow::Normal(state))
            }
            RegionTaskOp::MoveToPayload(resource) => {
                if state.sealed_here(*resource) {
                    return Err(RegionTaskDiagnostic::ResourceHeldByCleanupPack(*resource));
                }
                // `drop_resource` is the exclusive-authority gate: a loaned
                // resource cannot be smuggled into a payload either.
                state.ledger.drop_resource(ROOT_TASK, *resource)?;
                state.take_release(*resource)?;
                if !state.payload.insert(*resource) {
                    return Err(RegionTaskDiagnostic::DuplicateResourceIdentity(*resource));
                }
                Ok(Flow::Normal(state))
            }
            RegionTaskOp::DropPayload(resource) => {
                if !state.payload.remove(resource) {
                    return Err(RegionTaskDiagnostic::PayloadResourceNotOwned(*resource));
                }
                state.trace.push(CreditEvent::PayloadDestroy(*resource));
                Ok(Flow::Normal(state))
            }
            RegionTaskOp::Spend(amount) => {
                let region = state.region_name();
                let frame = state.frame()?;
                frame.allowable = frame.allowable.checked_sub(*amount).ok_or(
                    RegionTaskDiagnostic::InsufficientCredit {
                        region: region.clone(),
                        requested: *amount,
                        available: frame.allowable,
                    },
                )?;
                state.spent += *amount;
                state.trace.push(CreditEvent::Spend {
                    region,
                    amount: *amount,
                });
                Ok(Flow::Normal(state))
            }
            RegionTaskOp::Spawn(spawn) => self.spawn(state, spawn),
            RegionTaskOp::Complete {
                child,
                outcome,
                resources,
                unspent_credit,
            } => {
                let held = state.child_credit.get(child).copied().unwrap_or_default();
                if *unspent_credit > held {
                    return Err(RegionTaskDiagnostic::InsufficientCredit {
                        region: state.region_name(),
                        requested: *unspent_credit,
                        available: held,
                    });
                }
                state.ledger.complete(*child, outcome.clone(), resources)?;
                if held > 0 {
                    state.spent += held - *unspent_credit;
                    state.child_credit.insert(*child, *unspent_credit);
                    if held > *unspent_credit {
                        state.trace.push(CreditEvent::ChildSpend {
                            child: *child,
                            amount: held - *unspent_credit,
                        });
                    }
                }
                Ok(Flow::Normal(state))
            }
            RegionTaskOp::Join { handle } => self.join(state, *handle),
            RegionTaskOp::FailJoin { handle } => {
                state.ledger.fail_join(ROOT_TASK, *handle)?;
                state.failed_handles.insert(*handle);
                Ok(Flow::Normal(state))
            }
            RegionTaskOp::Raise(error) => {
                if !self.program.declared_errors.contains(error) {
                    return Err(RegionTaskDiagnostic::UndeclaredError(error.clone()));
                }
                if state.open.is_some() {
                    return Err(RegionTaskDiagnostic::RethrowWithUndischargedCleanup);
                }
                Ok(Flow::Raised {
                    error: error.clone(),
                    state,
                })
            }
            RegionTaskOp::Region {
                region,
                credit,
                body,
                catches,
            } => self.region(state, region, *credit, body, catches),
            RegionTaskOp::Discharge(binding) => self.discharge(state, binding),
        }
    }

    fn spawn(&self, mut state: State, spawn: &Spawn) -> Checked<Flow> {
        if state.frames.is_empty() {
            return Err(RegionTaskDiagnostic::ObligationOutsideRegion);
        }
        let region = state.region_name();
        for resource in spawn.moves.iter().chain(&spawn.loans) {
            if state.sealed_here(*resource) {
                return Err(RegionTaskDiagnostic::ResourceHeldByCleanupPack(*resource));
            }
        }
        let available = state.frames.last().unwrap().allowable;
        if spawn.child_credit > available {
            return Err(RegionTaskDiagnostic::InsufficientCredit {
                region,
                requested: spawn.child_credit,
                available,
            });
        }
        // T07 lends only the whole read authority (T04-style loans); the T05
        // counted-share ledger expects (resource, share) pairs.
        let whole_loans: Vec<(crate::scoped_tasks::ResourceId, u32)> = spawn
            .loans
            .iter()
            .map(|&resource| (resource, crate::scoped_tasks::FULL_READ_AUTHORITY))
            .collect();
        state.ledger.reserve_with_loans(
            ROOT_TASK,
            self.program.scope,
            spawn.reservation,
            &spawn.moves,
            &whole_loans,
        )?;
        match spawn.outcome {
            // A creation that did not happen lent nothing, moved nothing and
            // endowed nothing: the frame is restored exactly.
            SpawnOutcome::Failed => {
                state.ledger.rollback(ROOT_TASK, spawn.reservation)?;
            }
            SpawnOutcome::Created => {
                state
                    .ledger
                    .commit(ROOT_TASK, spawn.reservation, spawn.child, spawn.handle)?;
                for resource in &spawn.moves {
                    state.take_release(*resource)?;
                }
                let frame = state.frame()?;
                frame.allowable -= spawn.child_credit;
                frame.obligations.insert(Duty::Join(spawn.handle));
                for resource in &spawn.loans {
                    frame.obligations.insert(Duty::LoanExpiry(*resource));
                }
                if spawn.child_credit > 0 {
                    frame.obligations.insert(Duty::ChildCredit(spawn.child));
                }
                state.handle_child.insert(spawn.handle, spawn.child);
                if spawn.child_credit > 0 {
                    state.child_credit.insert(spawn.child, spawn.child_credit);
                    state.trace.push(CreditEvent::Lend {
                        child: spawn.child,
                        amount: spawn.child_credit,
                    });
                }
            }
        }
        Ok(Flow::Normal(state))
    }

    fn join(&self, mut state: State, handle: HandleId) -> Checked<Flow> {
        if state.frames.is_empty() {
            return Err(RegionTaskDiagnostic::ObligationOutsideRegion);
        }
        let capsule = state.ledger.join(ROOT_TASK, handle)?;
        state.joined_handles.insert(handle);
        state.collected.push((handle, capsule.outcome.clone()));
        // Returned resources become the current innermost region's duty; the
        // sealed pack a handler is holding is closed and gains nothing.
        for resource in &capsule.resources {
            state.frame()?.releases.push(*resource);
        }
        if let Some(child) = state.handle_child.get(&handle).copied()
            && let Some(returned) = state.child_credit.remove(&child)
        {
            state.frame()?.allowable += returned;
            if returned > 0 {
                state.trace.push(CreditEvent::Return {
                    child,
                    amount: returned,
                });
            }
        }
        Ok(Flow::Normal(state))
    }

    fn region(
        &self,
        mut state: State,
        region: &str,
        credit: u64,
        body: &[RegionTaskOp],
        catches: &[RegionTaskCatch],
    ) -> Checked<Flow> {
        if state.frames.iter().any(|frame| frame.region == region) {
            return Err(RegionTaskDiagnostic::DuplicateActiveRegion(region.into()));
        }
        let mut clauses = BTreeMap::<&str, &RegionTaskCatch>::new();
        for clause in catches {
            if !self.program.declared_errors.contains(&clause.error) {
                return Err(RegionTaskDiagnostic::UndeclaredError(clause.error.clone()));
            }
            if clauses.insert(&clause.error, clause).is_some() {
                return Err(RegionTaskDiagnostic::DuplicateCatchClause(
                    clause.error.clone(),
                ));
            }
        }
        let enclosing = state.region_name();
        let frame = state.frame()?;
        frame.allowable = frame.allowable.checked_sub(credit).ok_or(
            RegionTaskDiagnostic::InsufficientCredit {
                region: enclosing,
                requested: credit,
                available: frame.allowable,
            },
        )?;
        state.frames.push(Frame {
            region: region.into(),
            allowable: credit,
            releases: Vec::new(),
            obligations: BTreeSet::new(),
        });
        if credit > 0 {
            state.trace.push(CreditEvent::Endow {
                region: region.into(),
                amount: credit,
            });
        }

        match self.block(state, body)? {
            Flow::Normal(mut state) => {
                let frame = Self::pop(&mut state, region)?;
                Self::close_normally(&mut state, frame)?;
                Ok(Flow::Normal(state))
            }
            Flow::Raised { error, mut state } => {
                let frame = Self::pop(&mut state, region)?;
                if state.sealed.is_some() {
                    return Err(RegionTaskDiagnostic::NestedPackNotDischarged);
                }
                let pack = CreditPack {
                    region: frame.region,
                    error: error.clone(),
                    allowable: frame.allowable,
                    releases: frame.releases,
                    obligations: frame.obligations,
                };
                if let Some(resource) = pack
                    .releases
                    .iter()
                    .find(|resource| state.payload.contains(resource))
                {
                    return Err(RegionTaskDiagnostic::ResourceInPayloadAndCleanupPack(
                        *resource,
                    ));
                }
                match clauses.get(error.as_str()) {
                    Some(clause) => self.handle_catch(state, pack, clause),
                    None => {
                        state.sealed = Some(pack);
                        Ok(Flow::Raised { error, state })
                    }
                }
            }
        }
    }

    fn pop(state: &mut State, region: &str) -> Checked<Frame> {
        let frame = state
            .frames
            .pop()
            .ok_or(RegionTaskDiagnostic::ObligationOutsideRegion)?;
        if frame.region != region {
            return Err(RegionTaskDiagnostic::DuplicateActiveRegion(region.into()));
        }
        Ok(frame)
    }

    /// A normal region exit. Unspent credit is a refusal, not an abandonment:
    /// only an error path may abandon, and only through a checked discharge.
    fn close_normally(state: &mut State, frame: Frame) -> Checked<()> {
        Self::require_discharged(state, &frame.region, &frame.obligations)?;
        if frame.allowable != 0 {
            return Err(RegionTaskDiagnostic::UnusedCredit {
                region: frame.region,
                remainder: frame.allowable,
            });
        }
        Self::run_releases(state, &frame.releases)
    }

    /// Acceptance 1 and 2, at both sites that could discard a duty: a normal
    /// region exit and a cleanup-pack discharge.
    ///
    /// The reporting precedence is deliberate and is not a soundness rule —
    /// every outstanding duty refuses. Credit out with a child is reported
    /// first, because "this is somebody else's credit" is the most specific
    /// complaint; then the read loan, because it is the access permission a
    /// join would return; then the join itself. Without that order the join
    /// duty, which every creation opens, would shadow both.
    fn require_discharged(
        state: &State,
        region: &str,
        obligations: &BTreeSet<Duty>,
    ) -> Checked<()> {
        let outstanding = |wanted: fn(&Duty) -> bool| {
            obligations
                .iter()
                .copied()
                .find(|duty| wanted(duty) && state.outstanding(*duty))
        };
        if let Some(Duty::ChildCredit(child)) =
            outstanding(|duty| matches!(duty, Duty::ChildCredit(_)))
        {
            return Err(RegionTaskDiagnostic::ChildCreditOutstanding {
                region: region.into(),
                child,
                amount: state.child_credit.get(&child).copied().unwrap_or_default(),
            });
        }
        for family in [
            (|duty: &Duty| matches!(duty, Duty::LoanExpiry(_))) as fn(&Duty) -> bool,
            |duty: &Duty| matches!(duty, Duty::Join(_)),
            |duty: &Duty| matches!(duty, Duty::Release(_)),
        ] {
            if let Some(duty) = outstanding(family) {
                return Err(RegionTaskDiagnostic::PendingLifetimeObligation {
                    region: region.into(),
                    duty,
                });
            }
        }
        Ok(())
    }

    fn run_releases(state: &mut State, releases: &[ResourceId]) -> Checked<()> {
        for &resource in releases.iter().rev() {
            state.ledger.drop_resource(ROOT_TASK, resource)?;
            state.trace.push(CreditEvent::Destroy(resource));
        }
        Ok(())
    }

    fn handle_catch(
        &self,
        mut state: State,
        pack: CreditPack,
        clause: &RegionTaskCatch,
    ) -> Checked<Flow> {
        if state.open.is_some() {
            return Err(RegionTaskDiagnostic::NestedPackNotDischarged);
        }
        match &clause.quant_open {
            None => {
                if !pack.is_empty() {
                    return Err(RegionTaskDiagnostic::CatchWithoutRequiredQuantOpen {
                        region: pack.region,
                        error: pack.error,
                    });
                }
            }
            Some(open) => {
                if open.region != pack.region {
                    return Err(RegionTaskDiagnostic::WrongRegionOpened {
                        expected: pack.region,
                        actual: open.region.clone(),
                    });
                }
                if state.consumed_bindings.contains(&open.binding) {
                    return Err(RegionTaskDiagnostic::CleanupPackDuplicated(
                        open.binding.clone(),
                    ));
                }
                state.open = Some((open.binding.clone(), pack));
            }
        }
        match self.block(state, &clause.handler)? {
            Flow::Normal(state) if state.open.is_some() => {
                Err(RegionTaskDiagnostic::CleanupPackNotDischarged)
            }
            // A backstop: `Raise` already refuses an open capability at the
            // raise site, so no admitted program reaches this arm.
            Flow::Raised { state, .. } if state.open.is_some() => {
                Err(RegionTaskDiagnostic::RethrowWithUndischargedCleanup)
            }
            flow => Ok(flow),
        }
    }

    fn discharge(&self, mut state: State, binding: &str) -> Checked<Flow> {
        let (held, pack) = state.open.take().ok_or_else(|| {
            if state.consumed_bindings.contains(binding) {
                RegionTaskDiagnostic::CleanupPackDuplicated(binding.into())
            } else {
                RegionTaskDiagnostic::UnknownCleanupCapability(binding.into())
            }
        })?;
        if held != binding {
            state.open = Some((held, pack));
            return Err(RegionTaskDiagnostic::UnknownCleanupCapability(
                binding.into(),
            ));
        }
        // Acceptance 2: the abandonment gate. Every join the region opened must
        // have happened, every loan must have expired and every unit of credit
        // lent to a child must have come back. A failed join never closes, so
        // this refusal is permanent for that pack.
        Self::require_discharged(&state, &pack.region, &pack.obligations)?;
        Self::run_releases(&mut state, &pack.releases)?;
        if pack.allowable > 0 {
            state.abandoned += pack.allowable;
            state.trace.push(CreditEvent::AbandonCredit {
                region: pack.region.clone(),
                amount: pack.allowable,
            });
        }
        state.consumed_bindings.insert(binding.into());
        Ok(Flow::Normal(state))
    }
}

/// Check one `CheckedRegionTaskCreditV1` program and issue its certificate.
pub fn check_region_task_program(
    program: &RegionTaskProgram,
) -> Result<RegionTaskCertificate, RegionTaskDiagnostic> {
    let mut ledger = ScopedTaskLedger::default();
    ledger.enter_scope(ROOT_TASK, program.scope)?;
    let state = State {
        ledger,
        frames: vec![Frame {
            region: program.root_region.clone(),
            allowable: program.initial_credit,
            releases: Vec::new(),
            obligations: BTreeSet::new(),
        }],
        payload: BTreeSet::new(),
        open: None,
        consumed_bindings: BTreeSet::new(),
        sealed: None,
        child_credit: BTreeMap::new(),
        handle_child: BTreeMap::new(),
        joined_handles: BTreeSet::new(),
        failed_handles: BTreeSet::new(),
        spent: 0,
        abandoned: 0,
        trace: Vec::new(),
        collected: Vec::new(),
    };
    let analyzer = Analyzer { program };
    let (exit, mut state, escaping_pack) = match analyzer.block(state, &program.body)? {
        Flow::Normal(mut state) => {
            if state.open.is_some() {
                return Err(RegionTaskDiagnostic::CleanupPackNotDischarged);
            }
            let frame = Analyzer::pop(&mut state, &program.root_region)?;
            Analyzer::close_normally(&mut state, frame)?;
            if let Some(&resource) = state.payload.iter().next() {
                return Err(RegionTaskDiagnostic::PayloadObligationDropped { resource });
            }
            state.ledger.exit_scope(ROOT_TASK, program.scope)?;
            (ExitTag::Normal, state, None)
        }
        Flow::Raised { error, mut state } => {
            if state.open.is_some() {
                return Err(RegionTaskDiagnostic::RethrowWithUndischargedCleanup);
            }
            let frame = Analyzer::pop(&mut state, &program.root_region)?;
            if state.sealed.is_some() {
                return Err(RegionTaskDiagnostic::NestedPackNotDischarged);
            }
            let pack = CreditPack {
                region: frame.region,
                error: error.clone(),
                allowable: frame.allowable,
                releases: frame.releases,
                obligations: frame.obligations,
            };
            if let Some(resource) = pack
                .releases
                .iter()
                .find(|resource| state.payload.contains(resource))
            {
                return Err(RegionTaskDiagnostic::ResourceInPayloadAndCleanupPack(
                    *resource,
                ));
            }
            // Acceptance 3: the escaping error carries its payload and its
            // remaining destroy duties, but no concurrency obligation may leave
            // the thread scope on an error edge.
            for duty in pack.obligations.iter().copied() {
                if state.outstanding(duty) {
                    return Err(RegionTaskDiagnostic::ObligationEscapesScope { duty });
                }
            }
            (ExitTag::Error(error), state, Some(pack))
        }
    };
    // The escaping pack has already left `state`, so the final checkpoint adds
    // what it carries back in.
    let carried = escaping_pack.as_ref().map_or(0, |pack| pack.allowable);
    Analyzer::conserved_with(&state, carried, program.initial_credit)?;
    let credit = CreditAccounting {
        initial: program.initial_credit,
        spent: state.spent,
        abandoned: state.abandoned,
        carried,
    };
    if credit.spent + credit.abandoned + credit.carried != credit.initial {
        return Err(RegionTaskDiagnostic::CreditNotConserved {
            expected: credit.initial,
            observed: credit.spent + credit.abandoned + credit.carried,
        });
    }
    Ok(RegionTaskCertificate {
        profile: PROFILE,
        scope: program.scope,
        scope_exited: exit == ExitTag::Normal,
        exit,
        payload: std::mem::take(&mut state.payload),
        escaping_pack,
        credit,
        trace: std::mem::take(&mut state.trace),
        joined_handles: std::mem::take(&mut state.joined_handles),
        failed_handles: std::mem::take(&mut state.failed_handles),
        collected: std::mem::take(&mut state.collected),
        assumptions: [
            RegionTaskAssumption::DeclaredCreationOutcome,
            RegionTaskAssumption::DeclaredChildAccounting,
        ]
        .into_iter()
        .collect(),
    })
}

/// Independently re-derive the certificate and compare it field for field. A
/// forged field is refused; a forged program is refused by the checker itself.
pub fn recheck_region_task_certificate(
    program: &RegionTaskProgram,
    certificate: &RegionTaskCertificate,
) -> Result<(), Vec<CertificateRecheckError>> {
    let mut errors = Vec::new();
    let derived = match check_region_task_program(program) {
        Err(diagnostic) => {
            return Err(vec![CertificateRecheckError::ProgramRejected(diagnostic)]);
        }
        Ok(derived) => derived,
    };
    let mut mismatch = |field: &'static str, disagrees: bool| {
        if disagrees {
            errors.push(CertificateRecheckError::FieldMismatch { field });
        }
    };
    mismatch("profile", derived.profile != certificate.profile);
    mismatch("scope", derived.scope != certificate.scope);
    mismatch("exit", derived.exit != certificate.exit);
    mismatch(
        "scope_exited",
        derived.scope_exited != certificate.scope_exited,
    );
    mismatch("payload", derived.payload != certificate.payload);
    mismatch(
        "escaping_pack",
        derived.escaping_pack != certificate.escaping_pack,
    );
    mismatch("credit", derived.credit != certificate.credit);
    mismatch("trace", derived.trace != certificate.trace);
    mismatch(
        "joined_handles",
        derived.joined_handles != certificate.joined_handles,
    );
    mismatch(
        "failed_handles",
        derived.failed_handles != certificate.failed_handles,
    );
    mismatch("collected", derived.collected != certificate.collected);
    mismatch(
        "assumptions",
        derived.assumptions != certificate.assumptions,
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
#[path = "../tests/unit/region_task_credit.rs"]
mod tests;
