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

//! Checked task/resource custody for reserve, commit, completion and creator join.
//!
//! This ledger does not model scalar storage, mutexes, usage grades,
//! scheduling, races, SSA or a backend. Copy scalar captures need no custody
//! transition. Error outcomes are collected by join, never implicitly propagated.
//! Every failed operation leaves the complete ledger unchanged.
//!
//! Card T04 adds one bounded borrowing form: a **scoped read loan**. A loaned
//! resource keeps its owner and gains a borrower; both may read it and neither
//! may use it exclusively, so mutation, release, re-loan and move are refused
//! for the whole outstanding window. The creator's successful join is the only
//! operation that closes a loan.
//!
//! Card T05 makes the lent authority a **counted share** out of
//! [`FULL_READ_AUTHORITY`], so one resource can be lent to two borrowers with
//! positive complementary shares. What is budgeted is the authority handed
//! *out*: each lend transfers a positive share, the outstanding total may never
//! exceed the full authority, and exclusive authority — mutation, release, move
//! and further lending beyond the budget — returns only when the outstanding
//! total is back to zero. The owner's own read is deliberately **not** budgeted:
//! it is retained across lending exactly as T04 retained it, and it is sound
//! only because nothing may write while any share is outstanding. This is a
//! lending budget, not a separation-logic split of the owner's own permission;
//! see `docs/reports/remaining-t05/README.md` for that named boundary.
//!
//! ```
//! use qcore_core::scoped_tasks::{ScopedTaskLedger, TaskOutcome};
//! let mut ledger = ScopedTaskLedger::default();
//! ledger.acquire(0, 7).unwrap();
//! ledger.enter_scope(0, 10).unwrap();
//! ledger.reserve(0, 10, 20, &[7]).unwrap();
//! ledger.commit(0, 20, 1, 30).unwrap();
//! ledger.complete(1, TaskOutcome::I32(42), &[7]).unwrap();
//! assert!(ledger.use_resource(0, 7).is_err());
//! let completion = ledger.join(0, 30).unwrap();
//! assert_eq!(completion.outcome, TaskOutcome::I32(42));
//! ledger.drop_resource(0, 7).unwrap();
//! ledger.exit_scope(0, 10).unwrap();
//! ```
use std::collections::{BTreeMap, BTreeSet};

pub use crate::ownership::ResourceId;
pub type TaskId = u64;
pub type ScopeId = u64;
pub type ReservationId = u64;
pub type HandleId = u64;
pub const ROOT_TASK: TaskId = 0;

/// The whole read authority of one resource generation, expressed in the same
/// unit as the concurrency model's [`crate::concurrency::FULL_ACCESS`] so that
/// a share here and a share there are the same quantity.
pub const FULL_READ_AUTHORITY: u32 = crate::concurrency::FULL_ACCESS as u32;

/// The number of simultaneous borrowers this profile admits for one resource.
/// Card T05's deliverable is two readers; a third is a represented refusal, not
/// an unmodelled case.
pub const MAX_READERS: usize = 2;

/// The number of simultaneous range write leases this profile admits for one
/// resource (card `WRITE-LEND-TASKS`). The card's deliverable is two workers
/// each holding write authority over its own disjoint byte range; a third is a
/// represented refusal, exactly as [`MAX_READERS`] bounds the read side.
pub const MAX_WRITERS: usize = 2;

/// A half-open byte interval `[low, high)` on a resource's own axis — the
/// footprint of one range write lease (card `WRITE-LEND-TASKS`). The interval
/// vocabulary is the wave-29 write lend's (`ownership::BorrowKind::MutableRange`),
/// and the overlap question is decided by **calling**
/// [`crate::ownership::axes_overlap`] rather than restating it, so the
/// sequential lend and the task-crossing lease cannot drift apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeaseRange {
    pub low: u64,
    pub high: u64,
}

impl LeaseRange {
    /// Whether two leases share a byte — the wave-29 rule, reused.
    pub fn overlaps(self, other: LeaseRange) -> bool {
        crate::ownership::axes_overlap(self.low, self.high, other.low, other.high)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Active,
    Completed,
    Joined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Custodian {
    Task(TaskId),
    Reservation(ReservationId),
    Capsule(TaskId),
}

/// Who currently holds a lent read share. A share is `Pending` from the moment
/// its creation is reserved until that creation is known to have happened or
/// failed; it is never readable by anyone in that window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShareHolder {
    Borrower(TaskId),
    Pending(ReservationId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceState {
    Live(Custodian),
    Dropped,
    /// A resource with outstanding read shares. `owner` never stops owning it
    /// and never regains exclusive authority before every share is returned.
    /// `shares` is ordered, has no repeated holder, and every share is
    /// positive; its total never exceeds [`FULL_READ_AUTHORITY`].
    Loaned {
        owner: TaskId,
        shares: Vec<(ShareHolder, u32)>,
    },
    /// A resource with outstanding range write leases (card
    /// `WRITE-LEND-TASKS`). `owner` never stops owning it, but while any lease
    /// is out **nobody** — the owner included — holds whole-payload authority:
    /// the owner's read, write, release, move and further lending are all
    /// refused, because each of those footprints is the whole payload and any
    /// nonempty lease overlaps it. `leases` is ordered, has no repeated
    /// holder, every range is nonempty, and the ranges are pairwise disjoint —
    /// decided by the wave-29 overlap rule, never re-implemented.
    WriteLeased {
        owner: TaskId,
        leases: Vec<(ShareHolder, LeaseRange)>,
    },
}

impl ResourceState {
    /// A whole-authority loan to exactly one borrower — the only shape card
    /// T04 could express, retained as a constructor so its tests and fixtures
    /// keep saying what they meant.
    pub fn whole_loan(owner: TaskId, borrower: TaskId) -> Self {
        Self::Loaned {
            owner,
            shares: vec![(ShareHolder::Borrower(borrower), FULL_READ_AUTHORITY)],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskOutcome {
    Unit,
    I32(i32),
    /// A value of another type of the §G02t fixed-width table (card
    /// `W48-QR-SCALAR-TYPES-R4`): never `i32`, whose outcome is
    /// [`TaskOutcome::I32`]. Like `I32` it carries no custody; the ledger
    /// stores it and hands it to the joining creator unchanged.
    Fixed {
        ty: crate::scalar_types::ScalarType,
        value: i128,
    },
    OwnedBuffer(ResourceId),
    Error(String),
    /// Card T06: an explicitly tagged error carrying exactly one owned Buffer.
    /// The tag and the identity are one outcome, so a completion cannot name a
    /// payload it does not hold or hold a payload it does not name. Exactly one
    /// Buffer travels this way; a second payload, a borrowed identity and every
    /// other error ABI stay outside this profile.
    ErrorWithBuffer {
        tag: String,
        buffer: ResourceId,
    },
    /// A record, as its leaf outcomes in leaf order (card
    /// `W48-QR-TASK-RECORD-CAPTURES`): each leaf an [`TaskOutcome::I32`], a
    /// [`TaskOutcome::Fixed`] or an [`TaskOutcome::OwnedBuffer`]. The owned
    /// leaves, in order, are exactly the identities the completion carries —
    /// the record names every identity it holds and holds every identity it
    /// names, as a single owned result does.
    Record(Vec<TaskOutcome>),
}

impl TaskOutcome {
    /// The owned identities this outcome names, in order: a Buffer result's,
    /// an error payload's, or each owned leaf of a record's.
    pub fn owned_identities(&self) -> Vec<ResourceId> {
        match self {
            TaskOutcome::OwnedBuffer(resource)
            | TaskOutcome::ErrorWithBuffer {
                buffer: resource, ..
            } => vec![*resource],
            TaskOutcome::Record(leaves) => leaves
                .iter()
                .flat_map(TaskOutcome::owned_identities)
                .collect(),
            TaskOutcome::Unit
            | TaskOutcome::I32(_)
            | TaskOutcome::Fixed { .. }
            | TaskOutcome::Error(_) => vec![],
        }
    }
}

/// Returned metadata is not ownership authority: every resource operation still
/// checks the ledger's current custodian and generation identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionCapsule {
    pub outcome: TaskOutcome,
    pub resources: Vec<ResourceId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskLedgerError {
    UnknownTask(TaskId),
    InactiveTask(TaskId),
    DuplicateTask(TaskId),
    DuplicateResource(ResourceId),
    NotCustodian {
        task: TaskId,
        resource: ResourceId,
    },
    DuplicateScope(ScopeId),
    UnknownScope(ScopeId),
    ClosedScope(ScopeId),
    WrongScopeOwner(ScopeId),
    ScopeNotInnermost(ScopeId),
    ScopeBusy(ScopeId),
    DuplicateReservation(ReservationId),
    UnknownReservation(ReservationId),
    ReservationResolved(ReservationId),
    WrongCreator,
    DuplicateCapture(ResourceId),
    DuplicateHandle(HandleId),
    UnknownHandle(HandleId),
    HandleConsumed(HandleId),
    ChildNotCompleted(TaskId),
    RootCompletion,
    TaskBusy(TaskId),
    CompletionResourceMismatch,
    EmptyErrorLabel,
    /// Exclusive use was attempted while read shares are outstanding. This is
    /// not a custody complaint: the owner still owns the resource. `outstanding`
    /// is the share total that has not come back, and `borrowers` names the
    /// committed holders, so reconstruction to full authority is refused with
    /// the exact amount that is still lent.
    LoanOutstanding {
        resource: ResourceId,
        outstanding: u32,
        borrowers: Vec<TaskId>,
    },
    /// Lending would hand out more authority than the resource has. The
    /// complete request is refused; no partial share is granted.
    ShareExceedsAuthority {
        resource: ResourceId,
        outstanding: u32,
        requested: u32,
    },
    /// A share of zero, or above the full authority, is not a share.
    InvalidShare {
        resource: ResourceId,
        requested: u32,
    },
    /// More simultaneous borrowers than [`MAX_READERS`].
    TooManyReaders {
        resource: ResourceId,
        limit: usize,
    },
    /// Read authority was attempted by a task that is neither the owner nor the
    /// borrower of a live resource.
    NotReadable {
        task: TaskId,
        resource: ResourceId,
    },
    /// The loaned resource is not in the state the handle recorded, so the join
    /// cannot close the loan. The ledger is left unchanged.
    LoanNotOutstanding(ResourceId),
    /// Exclusive or read use was attempted while range write leases are
    /// outstanding (card `WRITE-LEND-TASKS`). Not a custody complaint: the
    /// owner still owns the resource. `ranges` are the byte intervals that
    /// have not come back and `holders` the committed lease holders, so the
    /// refusal names exactly which bytes are still another party's to write.
    LeaseOutstanding {
        resource: ResourceId,
        ranges: Vec<LeaseRange>,
        holders: Vec<TaskId>,
    },
    /// A second write lease would share a byte with one already outstanding.
    /// The overlap is the wave-29 decision (`max low < min high`), and the
    /// refusal names both ranges. The complete request is refused; no
    /// narrowed lease is granted.
    OverlappingLeases {
        resource: ResourceId,
        requested: LeaseRange,
        held: LeaseRange,
    },
    /// An empty range is not a write lease: it would carry no authority
    /// across the boundary, so the crossing is refused rather than admitted
    /// vacuously.
    InvalidLease {
        resource: ResourceId,
        requested: LeaseRange,
    },
    /// More simultaneous lease holders than [`MAX_WRITERS`].
    TooManyWriters {
        resource: ResourceId,
        limit: usize,
    },
    /// The leased resource is not in the state the handle recorded, so the
    /// join cannot close the lease. The ledger is left unchanged.
    LeaseNotOutstanding(ResourceId),
    /// A join for this handle already failed. The child was never observed to
    /// finish, so nothing it holds may be reclaimed and the handle stays
    /// outstanding forever.
    JoinFailed(HandleId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Task {
    state: TaskState,
    scopes: Vec<ScopeId>,
    capsule: Option<CompletionCapsule>,
}

impl Task {
    fn active() -> Self {
        Self {
            state: TaskState::Active,
            scopes: Vec::new(),
            capsule: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Scope {
    owner: TaskId,
    parent: Option<ScopeId>,
    active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReservationState {
    Pending,
    RolledBack,
    Committed { child: TaskId, handle: HandleId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Reservation {
    creator: TaskId,
    scope: ScopeId,
    resources: Vec<ResourceId>,
    /// Read shares parked for this creation rather than moved. Rollback returns
    /// each share to the owner exactly as it restores moves; commit rebinds the
    /// pending holder to the child without transferring custody.
    loans: Vec<(ResourceId, u32)>,
    /// Range write leases parked for this creation (card `WRITE-LEND-TASKS`),
    /// on exactly the loans' footing: rollback returns each lease to the owner
    /// exactly, commit rebinds the pending holder to the child.
    leases: Vec<(ResourceId, LeaseRange)>,
    state: ReservationState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Handle {
    creator: TaskId,
    scope: ScopeId,
    child: TaskId,
    consumed: bool,
    /// Read shares this creation opened. Only a successful join returns them.
    loans: Vec<(ResourceId, u32)>,
    /// Range write leases this creation opened. Only a successful join
    /// returns them — the lease's whole lifetime is spawn..join.
    leases: Vec<(ResourceId, LeaseRange)>,
    /// A join that did not observe the child finish. The handle stays
    /// outstanding and unconsumed, so its scope can never exit.
    failed: bool,
}

/// ID histories are retained even after rollback, drop, join and scope exit.
/// Reusing an identity therefore cannot revive an earlier generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedTaskLedger {
    tasks: BTreeMap<TaskId, Task>,
    scopes: BTreeMap<ScopeId, Scope>,
    reservations: BTreeMap<ReservationId, Reservation>,
    handles: BTreeMap<HandleId, Handle>,
    resources: BTreeMap<ResourceId, ResourceState>,
}

impl Default for ScopedTaskLedger {
    fn default() -> Self {
        Self {
            tasks: [(ROOT_TASK, Task::active())].into(),
            scopes: BTreeMap::new(),
            reservations: BTreeMap::new(),
            handles: BTreeMap::new(),
            resources: BTreeMap::new(),
        }
    }
}

impl ScopedTaskLedger {
    pub fn task_state(&self, task: TaskId) -> Option<TaskState> {
        self.tasks.get(&task).map(|task| task.state)
    }

    pub fn resource_state(&self, resource: ResourceId) -> Option<ResourceState> {
        self.resources.get(&resource).cloned()
    }

    /// Outstanding lent share total for one resource: zero when nothing is
    /// lent, and the exact amount that must come back before exclusive
    /// authority is reconstructed.
    pub fn lent_share(&self, resource: ResourceId) -> u32 {
        match self.resources.get(&resource) {
            Some(ResourceState::Loaned { shares, .. }) => shares.iter().map(|(_, s)| *s).sum(),
            _ => 0,
        }
    }

    /// The share this task currently holds of this resource, whether it is the
    /// owner's retained authority or a borrower's lent share. The owner's own
    /// read is not budgeted, so this reports only *lent* shares.
    pub fn held_share(&self, task: TaskId, resource: ResourceId) -> u32 {
        match self.resources.get(&resource) {
            Some(ResourceState::Loaned { shares, .. }) => shares
                .iter()
                .find(|(holder, _)| *holder == ShareHolder::Borrower(task))
                .map(|(_, share)| *share)
                .unwrap_or(0),
            _ => 0,
        }
    }

    fn outstanding(&self, resource: ResourceId) -> Result<(), TaskLedgerError> {
        let lent = self.lent_share(resource);
        if lent != 0 {
            return Err(TaskLedgerError::LoanOutstanding {
                resource,
                outstanding: lent,
                borrowers: self.loan_borrowers(resource),
            });
        }
        let ranges = self.leased_ranges(resource);
        if !ranges.is_empty() {
            return Err(TaskLedgerError::LeaseOutstanding {
                resource,
                ranges,
                holders: self.lease_holders(resource),
            });
        }
        Ok(())
    }

    /// The outstanding write-leased ranges of one resource, in holder order:
    /// empty when no lease is out, and exactly the bytes that must come back
    /// before whole-payload authority is reconstructed.
    pub fn leased_ranges(&self, resource: ResourceId) -> Vec<LeaseRange> {
        match self.resources.get(&resource) {
            Some(ResourceState::WriteLeased { leases, .. }) => {
                leases.iter().map(|(_, range)| *range).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The committed lease holders of this resource, in identity order. A
    /// pending creation is deliberately not a holder: it may never become one.
    pub fn lease_holders(&self, resource: ResourceId) -> Vec<TaskId> {
        match self.resources.get(&resource) {
            Some(ResourceState::WriteLeased { leases, .. }) => leases
                .iter()
                .filter_map(|(holder, _)| match holder {
                    ShareHolder::Borrower(task) => Some(*task),
                    ShareHolder::Pending(_) => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The range this task's committed lease holds of this resource, if any.
    pub fn held_lease(&self, task: TaskId, resource: ResourceId) -> Option<LeaseRange> {
        match self.resources.get(&resource) {
            Some(ResourceState::WriteLeased { leases, .. }) => leases
                .iter()
                .find(|(holder, _)| *holder == ShareHolder::Borrower(task))
                .map(|(_, range)| *range),
            _ => None,
        }
    }

    /// Every resource with an outstanding range write lease, in identity order.
    pub fn outstanding_write_leases(&self) -> Vec<ResourceId> {
        self.resources
            .iter()
            .filter_map(|(&resource, state)| {
                matches!(state, ResourceState::WriteLeased { .. }).then_some(resource)
            })
            .collect()
    }

    fn transaction<T>(
        &mut self,
        change: impl FnOnce(&mut Self) -> Result<T, TaskLedgerError>,
    ) -> Result<T, TaskLedgerError> {
        let mut next = self.clone();
        let result = change(&mut next)?;
        *self = next;
        Ok(result)
    }

    fn active(&self, task: TaskId) -> Result<(), TaskLedgerError> {
        match self.task_state(task) {
            None => Err(TaskLedgerError::UnknownTask(task)),
            Some(TaskState::Active) => Ok(()),
            Some(_) => Err(TaskLedgerError::InactiveTask(task)),
        }
    }

    fn owned_scope(&self, task: TaskId, scope: ScopeId) -> Result<(), TaskLedgerError> {
        self.active(task)?;
        let record = self
            .scopes
            .get(&scope)
            .ok_or(TaskLedgerError::UnknownScope(scope))?;
        if record.owner != task {
            return Err(TaskLedgerError::WrongScopeOwner(scope));
        }
        if !record.active {
            return Err(TaskLedgerError::ClosedScope(scope));
        }
        Ok(())
    }

    fn pending(
        &self,
        task: TaskId,
        reservation: ReservationId,
    ) -> Result<&Reservation, TaskLedgerError> {
        self.active(task)?;
        let record = self
            .reservations
            .get(&reservation)
            .ok_or(TaskLedgerError::UnknownReservation(reservation))?;
        if record.creator != task {
            return Err(TaskLedgerError::WrongCreator);
        }
        if record.state != ReservationState::Pending {
            return Err(TaskLedgerError::ReservationResolved(reservation));
        }
        self.owned_scope(task, record.scope)?;
        Ok(record)
    }

    pub fn acquire(&mut self, task: TaskId, resource: ResourceId) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            ledger.active(task)?;
            if ledger.resources.contains_key(&resource) {
                return Err(TaskLedgerError::DuplicateResource(resource));
            }
            ledger
                .resources
                .insert(resource, ResourceState::Live(Custodian::Task(task)));
            Ok(())
        })
    }

    /// Exclusive authority: mutation, release, move and re-loan. Any
    /// outstanding read share suspends it for the owner as well as for everyone
    /// else, so full authority is reconstructed only when every share is back.
    pub fn use_resource(&self, task: TaskId, resource: ResourceId) -> Result<(), TaskLedgerError> {
        self.active(task)?;
        self.outstanding(resource)?;
        if self.resource_state(resource) != Some(ResourceState::Live(Custodian::Task(task))) {
            return Err(TaskLedgerError::NotCustodian { task, resource });
        }
        Ok(())
    }

    /// Shared read authority. A borrower reads exactly while it holds a
    /// positive share. The owner keeps its own read across lending, but not
    /// while one of its creations is in flight: a pending share has left the
    /// owner and has not yet reached anyone. A dropped resource, a moved-away
    /// one and a completion capsule are readable by nobody.
    pub fn read_resource(&self, task: TaskId, resource: ResourceId) -> Result<(), TaskLedgerError> {
        self.active(task)?;
        let refused = Err(TaskLedgerError::NotReadable { task, resource });
        match self.resources.get(&resource) {
            Some(ResourceState::Live(Custodian::Task(custodian))) if *custodian == task => Ok(()),
            Some(ResourceState::Loaned { owner, shares }) => {
                if shares
                    .iter()
                    .any(|(holder, share)| *holder == ShareHolder::Borrower(task) && *share > 0)
                {
                    return Ok(());
                }
                if *owner == task
                    && !shares
                        .iter()
                        .any(|(holder, _)| matches!(holder, ShareHolder::Pending(_)))
                {
                    return Ok(());
                }
                refused
            }
            // A write-leased resource is readable by **nobody** (card
            // `WRITE-LEND-TASKS`): the owner's read footprint is the whole
            // payload and any nonempty lease overlaps it, and a lease holder
            // holds write authority over its own bytes, not a whole-payload
            // read this route could express. The refusal names the leases so
            // the exit says whose bytes are still out.
            Some(ResourceState::WriteLeased { .. }) => Err(TaskLedgerError::LeaseOutstanding {
                resource,
                ranges: self.leased_ranges(resource),
                holders: self.lease_holders(resource),
            }),
            _ => refused,
        }
    }

    /// The committed borrowers of this resource, in identity order. A pending
    /// creation is deliberately not a borrower: it may never become one.
    pub fn loan_borrowers(&self, resource: ResourceId) -> Vec<TaskId> {
        match self.resources.get(&resource) {
            Some(ResourceState::Loaned { shares, .. }) => shares
                .iter()
                .filter_map(|(holder, _)| match holder {
                    ShareHolder::Borrower(task) => Some(*task),
                    ShareHolder::Pending(_) => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Every resource with an outstanding read share, in identity order.
    pub fn outstanding_loans(&self) -> Vec<ResourceId> {
        self.resources
            .iter()
            .filter_map(|(&resource, state)| {
                matches!(state, ResourceState::Loaned { .. }).then_some(resource)
            })
            .collect()
    }

    pub fn drop_resource(
        &mut self,
        task: TaskId,
        resource: ResourceId,
    ) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            ledger.use_resource(task, resource)?;
            ledger.resources.insert(resource, ResourceState::Dropped);
            Ok(())
        })
    }

    /// Open a lexical scope nested in this task's current innermost scope.
    pub fn enter_scope(&mut self, task: TaskId, scope: ScopeId) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            ledger.active(task)?;
            if ledger.scopes.contains_key(&scope) {
                return Err(TaskLedgerError::DuplicateScope(scope));
            }
            let scopes = &mut ledger.tasks.get_mut(&task).unwrap().scopes;
            ledger.scopes.insert(
                scope,
                Scope {
                    owner: task,
                    parent: scopes.last().copied(),
                    active: true,
                },
            );
            scopes.push(scope);
            Ok(())
        })
    }

    pub fn exit_scope(&mut self, task: TaskId, scope: ScopeId) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            ledger.owned_scope(task, scope)?;
            if ledger.tasks[&task].scopes.last() != Some(&scope) {
                return Err(TaskLedgerError::ScopeNotInnermost(scope));
            }
            if ledger
                .reservations
                .values()
                .any(|r| r.scope == scope && r.state == ReservationState::Pending)
                || ledger
                    .handles
                    .values()
                    .any(|h| h.scope == scope && !h.consumed)
            {
                return Err(TaskLedgerError::ScopeBusy(scope));
            }
            ledger.scopes.get_mut(&scope).unwrap().active = false;
            ledger.tasks.get_mut(&task).unwrap().scopes.pop();
            Ok(())
        })
    }

    /// Reserve an ordered move list before attempting the external spawn.
    pub fn reserve(
        &mut self,
        creator: TaskId,
        scope: ScopeId,
        reservation: ReservationId,
        resources: &[ResourceId],
    ) -> Result<(), TaskLedgerError> {
        self.reserve_with_loans(creator, scope, reservation, resources, &[])
    }

    /// Reserve an ordered move list and an ordered list of read shares before
    /// attempting the external spawn. Moved resources are parked in the
    /// reservation, so the creator has no authority at all over them while
    /// creation is in flight. Each lent share leaves the owner as a *pending*
    /// share, which nobody may read; a failed creation returns both exactly.
    ///
    /// Lending is admitted while a resource already has outstanding shares, so
    /// long as the creator is its owner, the share is positive, the outstanding
    /// total stays within [`FULL_READ_AUTHORITY`], and the resource does not
    /// exceed [`MAX_READERS`] simultaneous holders. Excess authority is refused
    /// as a whole: no partial share is ever granted.
    pub fn reserve_with_loans(
        &mut self,
        creator: TaskId,
        scope: ScopeId,
        reservation: ReservationId,
        resources: &[ResourceId],
        loans: &[(ResourceId, u32)],
    ) -> Result<(), TaskLedgerError> {
        self.reserve_with_transfers(creator, scope, reservation, resources, loans, &[])
    }

    /// Reserve an ordered move list, an ordered list of read shares and an
    /// ordered list of range write leases before attempting the external spawn
    /// (card `WRITE-LEND-TASKS`). A lease is on exactly the loans' footing: it
    /// leaves the owner as a *pending* lease which grants nobody anything, a
    /// failed creation returns it exactly, and only the creator's successful
    /// join reclaims it — the lease's whole lifetime is spawn..join.
    ///
    /// A lease is admitted while the resource already has outstanding leases,
    /// so long as the creator is its owner, the range is nonempty, the range
    /// is **disjoint from every outstanding lease** — the wave-29 overlap rule
    /// ([`crate::ownership::axes_overlap`]), reused rather than re-implemented
    /// — and the resource does not exceed [`MAX_WRITERS`] simultaneous
    /// holders. A read share and a write lease of one resource never coexist:
    /// either request beside the other is refused naming what is out.
    pub fn reserve_with_transfers(
        &mut self,
        creator: TaskId,
        scope: ScopeId,
        reservation: ReservationId,
        resources: &[ResourceId],
        loans: &[(ResourceId, u32)],
        leases: &[(ResourceId, LeaseRange)],
    ) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            ledger.owned_scope(creator, scope)?;
            if ledger.tasks[&creator].scopes.last() != Some(&scope) {
                return Err(TaskLedgerError::ScopeNotInnermost(scope));
            }
            if ledger.reservations.contains_key(&reservation) {
                return Err(TaskLedgerError::DuplicateReservation(reservation));
            }
            let parked = resources
                .iter()
                .copied()
                .chain(loans.iter().map(|(resource, _)| *resource))
                .chain(leases.iter().map(|(resource, _)| *resource))
                .collect::<Vec<ResourceId>>();
            unique(&parked)?;
            for &resource in resources {
                ledger.use_resource(creator, resource)?;
            }
            for &(resource, share) in loans {
                ledger.lend(creator, resource, ShareHolder::Pending(reservation), share)?;
            }
            for &(resource, range) in leases {
                ledger.lease(creator, resource, ShareHolder::Pending(reservation), range)?;
            }
            for &resource in resources {
                ledger.resources.insert(
                    resource,
                    ResourceState::Live(Custodian::Reservation(reservation)),
                );
            }
            ledger.reservations.insert(
                reservation,
                Reservation {
                    creator,
                    scope,
                    resources: resources.to_vec(),
                    loans: loans.to_vec(),
                    leases: leases.to_vec(),
                    state: ReservationState::Pending,
                },
            );
            Ok(())
        })
    }

    /// Record one positive share against a live resource this task owns. Every
    /// admission rule is checked before anything is written, so a refused lend
    /// leaves the share ledger exactly as it was.
    fn lend(
        &mut self,
        creator: TaskId,
        resource: ResourceId,
        holder: ShareHolder,
        share: u32,
    ) -> Result<(), TaskLedgerError> {
        if share == 0 || share > FULL_READ_AUTHORITY {
            return Err(TaskLedgerError::InvalidShare {
                resource,
                requested: share,
            });
        }
        let mut shares = match self.resources.get(&resource) {
            Some(ResourceState::Live(Custodian::Task(custodian))) if *custodian == creator => {
                Vec::new()
            }
            Some(ResourceState::Loaned { owner, shares }) if *owner == creator => shares.clone(),
            // A read share grants whole-payload read authority, and any
            // nonempty write lease overlaps that footprint — so lending for
            // read while a write lease is out is refused with the lease named
            // (card `WRITE-LEND-TASKS`), never admitted beside it.
            Some(ResourceState::WriteLeased { owner, .. }) if *owner == creator => {
                return Err(TaskLedgerError::LeaseOutstanding {
                    resource,
                    ranges: self.leased_ranges(resource),
                    holders: self.lease_holders(resource),
                });
            }
            _ => {
                return Err(TaskLedgerError::NotCustodian {
                    task: creator,
                    resource,
                });
            }
        };
        if shares.len() >= MAX_READERS {
            return Err(TaskLedgerError::TooManyReaders {
                resource,
                limit: MAX_READERS,
            });
        }
        let outstanding = shares.iter().map(|(_, s)| *s).sum::<u32>();
        if outstanding + share > FULL_READ_AUTHORITY {
            return Err(TaskLedgerError::ShareExceedsAuthority {
                resource,
                outstanding,
                requested: share,
            });
        }
        if shares.iter().any(|(existing, _)| *existing == holder) {
            return Err(TaskLedgerError::DuplicateCapture(resource));
        }
        shares.push((holder, share));
        shares.sort();
        self.resources.insert(
            resource,
            ResourceState::Loaned {
                owner: creator,
                shares,
            },
        );
        Ok(())
    }

    /// Return one recorded share. The resource becomes exclusively the owner's
    /// again exactly when the last share comes back.
    fn reclaim(
        &mut self,
        owner: TaskId,
        resource: ResourceId,
        holder: ShareHolder,
        share: u32,
    ) -> Result<(), TaskLedgerError> {
        let Some(ResourceState::Loaned {
            owner: recorded,
            shares,
        }) = self.resources.get(&resource)
        else {
            return Err(TaskLedgerError::LoanNotOutstanding(resource));
        };
        if *recorded != owner || !shares.contains(&(holder, share)) {
            return Err(TaskLedgerError::LoanNotOutstanding(resource));
        }
        let remaining = shares
            .iter()
            .filter(|entry| **entry != (holder, share))
            .cloned()
            .collect::<Vec<_>>();
        self.resources.insert(
            resource,
            if remaining.is_empty() {
                ResourceState::Live(Custodian::Task(owner))
            } else {
                ResourceState::Loaned {
                    owner,
                    shares: remaining,
                }
            },
        );
        Ok(())
    }

    /// Record one nonempty, pairwise-disjoint range write lease against a live
    /// resource this task owns (card `WRITE-LEND-TASKS`). Every admission rule
    /// is checked before anything is written, so a refused lease leaves the
    /// ledger exactly as it was. The disjointness decision is
    /// [`LeaseRange::overlaps`] — the wave-29 rule, called rather than
    /// restated.
    fn lease(
        &mut self,
        creator: TaskId,
        resource: ResourceId,
        holder: ShareHolder,
        range: LeaseRange,
    ) -> Result<(), TaskLedgerError> {
        if range.low >= range.high {
            return Err(TaskLedgerError::InvalidLease {
                resource,
                requested: range,
            });
        }
        let mut leases = match self.resources.get(&resource) {
            Some(ResourceState::Live(Custodian::Task(custodian))) if *custodian == creator => {
                Vec::new()
            }
            Some(ResourceState::WriteLeased { owner, leases }) if *owner == creator => {
                leases.clone()
            }
            // A write lease and a read share of one resource never coexist: a
            // read share grants whole-payload read authority and any nonempty
            // lease overlaps that footprint. The refusal names the shares
            // that are out rather than falling to a custody complaint.
            Some(ResourceState::Loaned { owner, .. }) if *owner == creator => {
                return Err(TaskLedgerError::LoanOutstanding {
                    resource,
                    outstanding: self.lent_share(resource),
                    borrowers: self.loan_borrowers(resource),
                });
            }
            _ => {
                return Err(TaskLedgerError::NotCustodian {
                    task: creator,
                    resource,
                });
            }
        };
        if leases.len() >= MAX_WRITERS {
            return Err(TaskLedgerError::TooManyWriters {
                resource,
                limit: MAX_WRITERS,
            });
        }
        if let Some(&(_, held)) = leases.iter().find(|(_, held)| held.overlaps(range)) {
            return Err(TaskLedgerError::OverlappingLeases {
                resource,
                requested: range,
                held,
            });
        }
        if leases.iter().any(|(existing, _)| *existing == holder) {
            return Err(TaskLedgerError::DuplicateCapture(resource));
        }
        leases.push((holder, range));
        leases.sort();
        self.resources.insert(
            resource,
            ResourceState::WriteLeased {
                owner: creator,
                leases,
            },
        );
        Ok(())
    }

    /// Return one recorded lease. The resource becomes exclusively the owner's
    /// again exactly when the last lease comes back.
    fn reclaim_lease(
        &mut self,
        owner: TaskId,
        resource: ResourceId,
        holder: ShareHolder,
        range: LeaseRange,
    ) -> Result<(), TaskLedgerError> {
        let Some(ResourceState::WriteLeased {
            owner: recorded,
            leases,
        }) = self.resources.get(&resource)
        else {
            return Err(TaskLedgerError::LeaseNotOutstanding(resource));
        };
        if *recorded != owner || !leases.contains(&(holder, range)) {
            return Err(TaskLedgerError::LeaseNotOutstanding(resource));
        }
        let remaining = leases
            .iter()
            .filter(|entry| **entry != (holder, range))
            .cloned()
            .collect::<Vec<_>>();
        self.resources.insert(
            resource,
            if remaining.is_empty() {
                ResourceState::Live(Custodian::Task(owner))
            } else {
                ResourceState::WriteLeased {
                    owner,
                    leases: remaining,
                }
            },
        );
        Ok(())
    }

    /// Failed spawn restores only this reservation; earlier siblings persist,
    /// including a share another creation still holds. A creation that did not
    /// happen lent nothing, so its pending share returns to the owner exactly.
    pub fn rollback(
        &mut self,
        creator: TaskId,
        reservation: ReservationId,
    ) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            let record = ledger.pending(creator, reservation)?.clone();
            for &(resource, share) in &record.loans {
                ledger.reclaim(creator, resource, ShareHolder::Pending(reservation), share)?;
            }
            for &(resource, range) in &record.leases {
                ledger.reclaim_lease(
                    creator,
                    resource,
                    ShareHolder::Pending(reservation),
                    range,
                )?;
            }
            for resource in &record.resources {
                ledger
                    .resources
                    .insert(*resource, ResourceState::Live(Custodian::Task(creator)));
            }
            ledger.reservations.get_mut(&reservation).unwrap().state = ReservationState::RolledBack;
            Ok(())
        })
    }

    pub fn commit(
        &mut self,
        creator: TaskId,
        reservation: ReservationId,
        child: TaskId,
        handle: HandleId,
    ) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            let reservation_record = ledger.pending(creator, reservation)?.clone();
            if ledger.tasks.contains_key(&child) {
                return Err(TaskLedgerError::DuplicateTask(child));
            }
            if ledger.handles.contains_key(&handle) {
                return Err(TaskLedgerError::DuplicateHandle(handle));
            }
            ledger.tasks.insert(child, Task::active());
            ledger.handles.insert(
                handle,
                Handle {
                    creator,
                    scope: reservation_record.scope,
                    child,
                    consumed: false,
                    loans: reservation_record.loans.clone(),
                    leases: reservation_record.leases.clone(),
                    failed: false,
                },
            );
            for resource in reservation_record.resources {
                ledger
                    .resources
                    .insert(resource, ResourceState::Live(Custodian::Task(child)));
            }
            // A pending share becomes the child's share. Nothing is minted:
            // exactly the reserved amount changes holder.
            for (resource, share) in reservation_record.loans {
                ledger.reclaim(creator, resource, ShareHolder::Pending(reservation), share)?;
                ledger.lend(creator, resource, ShareHolder::Borrower(child), share)?;
            }
            // A pending lease becomes the child's lease, on the same footing:
            // exactly the reserved bytes change holder.
            for (resource, range) in reservation_record.leases {
                ledger.reclaim_lease(
                    creator,
                    resource,
                    ShareHolder::Pending(reservation),
                    range,
                )?;
                ledger.lease(creator, resource, ShareHolder::Borrower(child), range)?;
            }
            ledger.reservations.get_mut(&reservation).unwrap().state =
                ReservationState::Committed { child, handle };
            Ok(())
        })
    }

    /// Complete a child with exactly its remaining live resource identities.
    /// Resources become inaccessible in the completion capsule until creator join.
    pub fn complete(
        &mut self,
        child: TaskId,
        outcome: TaskOutcome,
        resources: &[ResourceId],
    ) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            ledger.active(child)?;
            if child == ROOT_TASK {
                return Err(TaskLedgerError::RootCompletion);
            }
            if matches!(&outcome, TaskOutcome::Error(label) if label.is_empty())
                || matches!(&outcome, TaskOutcome::ErrorWithBuffer { tag, .. } if tag.is_empty())
            {
                return Err(TaskLedgerError::EmptyErrorLabel);
            }
            if !ledger.tasks[&child].scopes.is_empty()
                || ledger
                    .reservations
                    .values()
                    .any(|r| r.creator == child && r.state == ReservationState::Pending)
                || ledger
                    .handles
                    .values()
                    .any(|h| h.creator == child && !h.consumed)
            {
                return Err(TaskLedgerError::TaskBusy(child));
            }
            // A typed payload names exactly one identity, whether it is a
            // normal owned result or a tagged error payload. The remaining
            // `resources == owned` check below then forces that identity to be
            // exactly what the child still holds.
            if let TaskOutcome::OwnedBuffer(resource)
            | TaskOutcome::ErrorWithBuffer {
                buffer: resource, ..
            } = &outcome
                && resources != [*resource]
            {
                return Err(TaskLedgerError::CompletionResourceMismatch);
            }
            // Card `W48-QR-TASK-RECORD-CAPTURES`: a record names each owned
            // leaf's identity, in order, and a record leaf is a value — never a
            // nested record, an error or unit.
            if let TaskOutcome::Record(leaves) = &outcome
                && (leaves.iter().any(|leaf| {
                    !matches!(
                        leaf,
                        TaskOutcome::I32(_)
                            | TaskOutcome::Fixed { .. }
                            | TaskOutcome::OwnedBuffer(_)
                    )
                }) || resources != outcome.owned_identities().as_slice())
            {
                return Err(TaskLedgerError::CompletionResourceMismatch);
            }
            let returned = unique(resources)?;
            let owned = ledger
                .resources
                .iter()
                .filter_map(|(&resource, state)| {
                    (*state == ResourceState::Live(Custodian::Task(child))).then_some(resource)
                })
                .collect::<BTreeSet<_>>();
            if returned != owned {
                return Err(TaskLedgerError::CompletionResourceMismatch);
            }
            for &resource in resources {
                ledger
                    .resources
                    .insert(resource, ResourceState::Live(Custodian::Capsule(child)));
            }
            let task = ledger.tasks.get_mut(&child).unwrap();
            task.state = TaskState::Completed;
            task.capsule = Some(CompletionCapsule {
                outcome,
                resources: resources.to_vec(),
            });
            Ok(())
        })
    }

    pub fn join(
        &mut self,
        creator: TaskId,
        handle: HandleId,
    ) -> Result<CompletionCapsule, TaskLedgerError> {
        self.transaction(|ledger| {
            ledger.active(creator)?;
            let record = ledger
                .handles
                .get(&handle)
                .ok_or(TaskLedgerError::UnknownHandle(handle))?
                .clone();
            if record.creator != creator {
                return Err(TaskLedgerError::WrongCreator);
            }
            if record.consumed {
                return Err(TaskLedgerError::HandleConsumed(handle));
            }
            if record.failed {
                return Err(TaskLedgerError::JoinFailed(handle));
            }
            ledger.owned_scope(creator, record.scope)?;
            if ledger.task_state(record.child) != Some(TaskState::Completed) {
                return Err(TaskLedgerError::ChildNotCompleted(record.child));
            }
            // Returning the shares is part of the same transaction as
            // collecting the capsule: a share the ledger cannot account for
            // aborts the join instead of silently reconstructing authority.
            // A share held by a *sibling* is untouched, so a join reconstructs
            // full authority only when it was the last outstanding share.
            for &(resource, share) in &record.loans {
                ledger.reclaim(
                    creator,
                    resource,
                    ShareHolder::Borrower(record.child),
                    share,
                )?;
            }
            // Returning the leases is part of the same transaction as
            // collecting the capsule, exactly as for shares: a lease the
            // ledger cannot account for aborts the join. A lease held by a
            // *sibling* is untouched, so a join reconstructs whole-payload
            // authority only when it returned the last outstanding lease.
            for &(resource, range) in &record.leases {
                ledger.reclaim_lease(
                    creator,
                    resource,
                    ShareHolder::Borrower(record.child),
                    range,
                )?;
            }
            let task = ledger.tasks.get_mut(&record.child).unwrap();
            let capsule = task.capsule.take().unwrap();
            task.state = TaskState::Joined;
            for &resource in &capsule.resources {
                ledger
                    .resources
                    .insert(resource, ResourceState::Live(Custodian::Task(creator)));
            }
            ledger.handles.get_mut(&handle).unwrap().consumed = true;
            Ok(capsule)
        })
    }

    /// Record that a join did not observe its child finish. Nothing is
    /// reclaimed: the handle stays outstanding and unconsumed, its scope can
    /// never exit, every loan it opened stays outstanding, and any completion
    /// capsule stays inaccessible. A failed join is terminal for this handle;
    /// there is no retry that could later admit reclamation on this evidence.
    pub fn fail_join(&mut self, creator: TaskId, handle: HandleId) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            ledger.active(creator)?;
            let record = ledger
                .handles
                .get(&handle)
                .ok_or(TaskLedgerError::UnknownHandle(handle))?
                .clone();
            if record.creator != creator {
                return Err(TaskLedgerError::WrongCreator);
            }
            if record.consumed {
                return Err(TaskLedgerError::HandleConsumed(handle));
            }
            if record.failed {
                return Err(TaskLedgerError::JoinFailed(handle));
            }
            ledger.owned_scope(creator, record.scope)?;
            ledger.handles.get_mut(&handle).unwrap().failed = true;
            Ok(())
        })
    }
}

fn unique(resources: &[ResourceId]) -> Result<BTreeSet<ResourceId>, TaskLedgerError> {
    let mut set = BTreeSet::new();
    for &resource in resources {
        if !set.insert(resource) {
            return Err(TaskLedgerError::DuplicateCapture(resource));
        }
    }
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened() -> ScopedTaskLedger {
        let mut ledger = ScopedTaskLedger::default();
        ledger.enter_scope(0, 10).unwrap();
        ledger
    }

    fn spawn(
        ledger: &mut ScopedTaskLedger,
        reservation: u64,
        child: u64,
        handle: u64,
        resources: &[ResourceId],
    ) {
        ledger.reserve(0, 10, reservation, resources).unwrap();
        ledger.commit(0, reservation, child, handle).unwrap();
    }

    fn unchanged<T: std::fmt::Debug>(
        ledger: &mut ScopedTaskLedger,
        operation: impl FnOnce(&mut ScopedTaskLedger) -> Result<T, TaskLedgerError>,
    ) {
        let original = ledger.clone();
        assert!(operation(ledger).is_err());
        assert_eq!(*ledger, original);
    }

    #[test]
    fn failed_spawn_restores_exact_moves_without_disturbing_prior_sibling() {
        let mut ledger = opened();
        for resource in [7, 8, 9] {
            ledger.acquire(0, resource).unwrap();
        }
        spawn(&mut ledger, 20, 1, 30, &[7]);
        ledger.reserve(0, 10, 21, &[9, 8]).unwrap();
        assert!(ledger.use_resource(0, 8).is_err());
        assert_eq!(
            ledger.resource_state(9),
            Some(ResourceState::Live(Custodian::Reservation(21)))
        );
        ledger.rollback(0, 21).unwrap();
        ledger.use_resource(0, 8).unwrap();
        ledger.use_resource(0, 9).unwrap();
        ledger.use_resource(1, 7).unwrap();
        assert_eq!(ledger.task_state(1), Some(TaskState::Active));
        unchanged(&mut ledger, |l| l.exit_scope(0, 10));
        unchanged(&mut ledger, |l| l.rollback(0, 21));
    }

    #[test]
    fn completion_retains_resources_until_creator_join_and_collects_error() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        spawn(&mut ledger, 20, 1, 30, &[7]);
        ledger.acquire(1, 8).unwrap();
        ledger
            .complete(1, TaskOutcome::Error("Child.Failed".into()), &[8, 7])
            .unwrap();
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Live(Custodian::Capsule(1)))
        );
        assert!(ledger.use_resource(0, 7).is_err());
        assert!(ledger.use_resource(1, 7).is_err());
        unchanged(&mut ledger, |l| l.drop_resource(1, 7));
        unchanged(&mut ledger, |l| l.exit_scope(0, 10));
        let capsule = ledger.join(0, 30).unwrap();
        assert_eq!(
            capsule,
            CompletionCapsule {
                outcome: TaskOutcome::Error("Child.Failed".into()),
                resources: vec![8, 7]
            }
        );
        assert_eq!(ledger.task_state(0), Some(TaskState::Active));
        assert_eq!(ledger.task_state(1), Some(TaskState::Joined));
        ledger.use_resource(0, 7).unwrap();
        ledger.drop_resource(0, 8).unwrap();
        ledger.exit_scope(0, 10).unwrap();
        unchanged(&mut ledger, |l| l.join(0, 30));
        unchanged(&mut ledger, |l| l.acquire(1, 9));
    }

    #[test]
    fn creator_and_completion_are_required_for_linear_join() {
        let mut ledger = opened();
        spawn(&mut ledger, 20, 1, 30, &[]);
        spawn(&mut ledger, 21, 2, 31, &[]);
        unchanged(&mut ledger, |l| l.join(0, 30));
        ledger.complete(1, TaskOutcome::I32(i32::MIN), &[]).unwrap();
        unchanged(&mut ledger, |l| l.join(2, 30));
        unchanged(&mut ledger, |l| l.complete(1, TaskOutcome::Unit, &[]));
        assert_eq!(
            ledger.join(0, 30).unwrap().outcome,
            TaskOutcome::I32(i32::MIN)
        );
        ledger.complete(2, TaskOutcome::Unit, &[]).unwrap();
        assert_eq!(ledger.join(0, 31).unwrap().outcome, TaskOutcome::Unit);
        ledger.exit_scope(0, 10).unwrap();
    }

    #[test]
    fn pending_nested_scopes_and_descendants_block_completion() {
        let mut ledger = opened();
        spawn(&mut ledger, 20, 1, 30, &[]);
        ledger.enter_scope(1, 11).unwrap();
        ledger.enter_scope(1, 12).unwrap();
        unchanged(&mut ledger, |l| l.exit_scope(1, 11));
        unchanged(&mut ledger, |l| l.reserve(1, 11, 21, &[]));
        unchanged(&mut ledger, |l| l.complete(1, TaskOutcome::Unit, &[]));
        ledger.exit_scope(1, 12).unwrap();
        ledger.reserve(1, 11, 21, &[]).unwrap();
        unchanged(&mut ledger, |l| l.exit_scope(1, 11));
        ledger.commit(1, 21, 2, 31).unwrap();
        ledger.complete(2, TaskOutcome::Unit, &[]).unwrap();
        unchanged(&mut ledger, |l| l.complete(1, TaskOutcome::Unit, &[]));
        ledger.join(1, 31).unwrap();
        ledger.exit_scope(1, 11).unwrap();
        ledger.complete(1, TaskOutcome::Unit, &[]).unwrap();
        ledger.join(0, 30).unwrap();
        ledger.exit_scope(0, 10).unwrap();
    }

    #[test]
    fn completion_requires_exact_live_resources_without_duplicates_or_foreign_ids() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        spawn(&mut ledger, 20, 1, 30, &[7]);
        ledger.acquire(0, 8).unwrap();
        for returned in [vec![], vec![7, 7], vec![7, 8], vec![99]] {
            unchanged(&mut ledger, |l| l.complete(1, TaskOutcome::Unit, &returned));
        }
        unchanged(&mut ledger, |l| {
            l.complete(1, TaskOutcome::Error(String::new()), &[7])
        });
        ledger.drop_resource(1, 7).unwrap();
        unchanged(&mut ledger, |l| l.complete(1, TaskOutcome::Unit, &[7]));
        ledger.complete(1, TaskOutcome::Unit, &[]).unwrap();
        assert!(ledger.join(0, 30).unwrap().resources.is_empty());
        assert_eq!(ledger.resource_state(7), Some(ResourceState::Dropped));
    }

    #[test]
    fn typed_owned_completion_cannot_substitute_or_omit_its_payload() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        spawn(&mut ledger, 20, 1, 30, &[7]);
        for (identity, resources) in [(7, vec![]), (8, vec![7]), (7, vec![7, 8])] {
            unchanged(&mut ledger, |l| {
                l.complete(1, TaskOutcome::OwnedBuffer(identity), &resources)
            });
        }
        ledger
            .complete(1, TaskOutcome::OwnedBuffer(7), &[7])
            .unwrap();
        assert!(ledger.use_resource(0, 7).is_err());
        assert!(ledger.use_resource(1, 7).is_err());
        let result = ledger.join(0, 30).unwrap();
        assert_eq!(result.outcome, TaskOutcome::OwnedBuffer(7));
        assert_eq!(result.resources, [7]);
        ledger.drop_resource(0, 7).unwrap();
        ledger.exit_scope(0, 10).unwrap();
    }

    /// T06 acceptance 1 and 3, at the ledger. A tagged error payload is held to
    /// the same rule as a typed owned result: the completion names exactly the
    /// identity it holds, the capsule belongs to nobody until the creator's
    /// join, and the join hands it over exactly once.
    #[test]
    fn a_tagged_error_payload_is_named_exactly_and_transfers_only_at_join() {
        let mut ledger = opened();
        for resource in [7, 8] {
            ledger.acquire(0, resource).unwrap();
        }
        spawn(&mut ledger, 20, 1, 30, &[7]);
        let tagged = |buffer| TaskOutcome::ErrorWithBuffer {
            tag: "Recovery::Failed".into(),
            buffer,
        };
        // Omitting, substituting, duplicating and padding the payload each
        // reject, and the empty tag rejects like an empty error label.
        for (outcome, resources) in [
            (tagged(7), vec![]),
            (tagged(8), vec![7]),
            (tagged(7), vec![7, 7]),
            (tagged(7), vec![7, 8]),
            (
                TaskOutcome::ErrorWithBuffer {
                    tag: String::new(),
                    buffer: 7,
                },
                vec![7],
            ),
        ] {
            unchanged(&mut ledger, |l| l.complete(1, outcome.clone(), &resources));
        }
        ledger.complete(1, tagged(7), &[7]).unwrap();

        // Completion is not custody return: the payload is in the capsule and
        // neither the parent nor the child may touch it.
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Live(Custodian::Capsule(1)))
        );
        assert!(ledger.use_resource(0, 7).is_err());
        assert!(ledger.use_resource(1, 7).is_err());
        unchanged(&mut ledger, |l| l.drop_resource(0, 7));
        unchanged(&mut ledger, |l| l.exit_scope(0, 10));

        let capsule = ledger.join(0, 30).unwrap();
        assert_eq!(capsule.outcome, tagged(7));
        assert_eq!(capsule.resources, [7]);
        ledger.use_resource(0, 7).unwrap();
        // The transfer happens once: a second join is refused by the consumed
        // handle, so the payload cannot be collected or released twice.
        rejected(&mut ledger, &TaskLedgerError::HandleConsumed(30), |l| {
            l.join(0, 30)
        });
        ledger.drop_resource(0, 7).unwrap();
        unchanged(&mut ledger, |l| l.drop_resource(0, 7));
        ledger.drop_resource(0, 8).unwrap();
        ledger.exit_scope(0, 10).unwrap();
    }

    /// A child that acquired a local and then failed must release it: the
    /// completion is refused while any resource other than the named payload is
    /// still live under the child.
    #[test]
    fn a_tagged_error_completion_refuses_an_unreleased_worker_local() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        spawn(&mut ledger, 20, 1, 30, &[7]);
        ledger.acquire(1, 8).unwrap();
        let tagged = TaskOutcome::ErrorWithBuffer {
            tag: "Recovery::Failed".into(),
            buffer: 7,
        };
        unchanged(&mut ledger, |l| l.complete(1, tagged.clone(), &[7]));
        ledger.drop_resource(1, 8).unwrap();
        ledger.complete(1, tagged, &[7]).unwrap();
        assert_eq!(ledger.resource_state(8), Some(ResourceState::Dropped));
        ledger.join(0, 30).unwrap();
        ledger.drop_resource(0, 7).unwrap();
        ledger.exit_scope(0, 10).unwrap();
    }

    #[test]
    fn invalid_capture_and_commit_are_atomic_and_generation_ids_never_reuse() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        unchanged(&mut ledger, |l| l.reserve(0, 10, 20, &[7, 7]));
        unchanged(&mut ledger, |l| l.reserve(0, 10, 20, &[7, 99]));
        ledger.reserve(0, 10, 20, &[7]).unwrap();
        unchanged(&mut ledger, |l| l.commit(0, 20, 0, 30));
        unchanged(&mut ledger, |l| l.commit(99, 20, 1, 30));
        ledger.rollback(0, 20).unwrap();
        unchanged(&mut ledger, |l| l.reserve(0, 10, 20, &[7]));
        spawn(&mut ledger, 21, 1, 30, &[7]);
        ledger.complete(1, TaskOutcome::Unit, &[7]).unwrap();
        ledger.join(0, 30).unwrap();
        ledger.reserve(0, 10, 22, &[7]).unwrap();
        unchanged(&mut ledger, |l| l.commit(0, 22, 1, 31));
        unchanged(&mut ledger, |l| l.commit(0, 22, 2, 30));
        ledger.rollback(0, 22).unwrap();
        ledger.drop_resource(0, 7).unwrap();
        unchanged(&mut ledger, |l| l.acquire(0, 7));
        unchanged(&mut ledger, |l| l.drop_resource(0, 7));
        ledger.exit_scope(0, 10).unwrap();
        unchanged(&mut ledger, |l| l.enter_scope(0, 10));
        unchanged(&mut ledger, |l| l.complete(0, TaskOutcome::Unit, &[]));
    }
    fn rejected<T: std::fmt::Debug>(
        ledger: &mut ScopedTaskLedger,
        expected: &TaskLedgerError,
        operation: impl FnOnce(&mut ScopedTaskLedger) -> Result<T, TaskLedgerError>,
    ) {
        let original = ledger.clone();
        assert_eq!(operation(ledger).unwrap_err(), *expected);
        assert_eq!(*ledger, original);
    }

    /// R02 matrix for the ledger's terminal phases. Completion is not custody
    /// return and neither phase retains execution authority: every operation
    /// funnels through the same active-task gate before any custody, scope,
    /// reservation, handle or creator check can admit it.
    #[test]
    fn completed_and_joined_children_reject_every_ledger_operation() {
        let mut ledger = opened();
        for resource in [7, 8] {
            ledger.acquire(0, resource).unwrap();
        }
        spawn(&mut ledger, 20, 1, 30, &[7]);
        spawn(&mut ledger, 21, 2, 31, &[]);
        ledger
            .complete(1, TaskOutcome::OwnedBuffer(7), &[7])
            .unwrap();
        ledger.complete(2, TaskOutcome::Unit, &[]).unwrap();
        ledger.join(0, 31).unwrap();
        assert_eq!(ledger.task_state(1), Some(TaskState::Completed));
        assert_eq!(ledger.task_state(2), Some(TaskState::Joined));

        // Completion is not custody return: the capsule belongs to no task.
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Live(Custodian::Capsule(1)))
        );
        let not_custodian = TaskLedgerError::NotCustodian {
            task: 0,
            resource: 7,
        };
        rejected(&mut ledger, &not_custodian, |l| l.drop_resource(0, 7));
        assert_eq!(ledger.use_resource(0, 7).unwrap_err(), not_custodian);
        rejected(&mut ledger, &TaskLedgerError::ScopeBusy(10), |l| {
            l.exit_scope(0, 10)
        });

        ledger.reserve(0, 10, 22, &[8]).unwrap();
        for terminal in [1, 2] {
            let expected = TaskLedgerError::InactiveTask(terminal);
            assert_eq!(ledger.use_resource(terminal, 7).unwrap_err(), expected);
            rejected(&mut ledger, &expected, |l| l.acquire(terminal, 90));
            rejected(&mut ledger, &expected, |l| l.drop_resource(terminal, 7));
            rejected(&mut ledger, &expected, |l| l.enter_scope(terminal, 90));
            rejected(&mut ledger, &expected, |l| l.exit_scope(terminal, 10));
            rejected(&mut ledger, &expected, |l| l.reserve(terminal, 10, 91, &[]));
            rejected(&mut ledger, &expected, |l| l.rollback(terminal, 22));
            rejected(&mut ledger, &expected, |l| l.commit(terminal, 22, 92, 93));
            rejected(&mut ledger, &expected, |l| {
                l.complete(terminal, TaskOutcome::Unit, &[])
            });
            rejected(&mut ledger, &expected, |l| l.join(terminal, 30));
        }

        // The creator, and only the creator, still collects the capsule; a
        // repeated join is refused by the consumed handle, not by the phase.
        ledger.rollback(0, 22).unwrap();
        let capsule = ledger.join(0, 30).unwrap();
        assert_eq!(capsule.outcome, TaskOutcome::OwnedBuffer(7));
        assert_eq!(ledger.task_state(1), Some(TaskState::Joined));
        ledger.use_resource(0, 7).unwrap();
        rejected(&mut ledger, &TaskLedgerError::HandleConsumed(30), |l| {
            l.join(0, 30)
        });
        for resource in [7, 8] {
            ledger.drop_resource(0, resource).unwrap();
        }
        ledger.exit_scope(0, 10).unwrap();
    }

    /// T04 acceptance 1. A read loan suspends *exclusive* authority for
    /// everyone, the owner included, and grants read authority to exactly two
    /// tasks. The owner never stops owning: the refusal names the loan, not a
    /// custody transfer.
    #[test]
    fn an_outstanding_read_loan_suspends_exclusive_authority_and_admits_two_readers() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        ledger
            .reserve_with_loans(0, 10, 20, &[], &[(7, FULL_READ_AUTHORITY)])
            .unwrap();
        // The reservation window grants nobody anything, read included.
        assert_eq!(
            ledger.read_resource(0, 7).unwrap_err(),
            TaskLedgerError::NotReadable {
                task: 0,
                resource: 7
            }
        );
        ledger.commit(0, 20, 1, 30).unwrap();
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::whole_loan(0, 1))
        );
        assert_eq!(ledger.loan_borrowers(7), [1]);
        assert_eq!(ledger.outstanding_loans(), [7]);

        let suspended = TaskLedgerError::LoanOutstanding {
            resource: 7,
            outstanding: FULL_READ_AUTHORITY,
            borrowers: vec![1],
        };
        for task in [0, 1] {
            assert_eq!(ledger.use_resource(task, 7).unwrap_err(), suspended);
            rejected(&mut ledger, &suspended, |l| l.drop_resource(task, 7));
        }
        // Moving the same resource is an exclusive use. Lending it again is
        // *not*, under T05: it is an over-allocation of authority, and the
        // refusal now says so with the exact amounts. This expectation was
        // re-pinned deliberately when the whole-loan bound became a budget;
        // the direction is unchanged — the operation is still refused.
        rejected(&mut ledger, &suspended, |l| l.reserve(0, 10, 22, &[7]));
        rejected(
            &mut ledger,
            &TaskLedgerError::ShareExceedsAuthority {
                resource: 7,
                outstanding: FULL_READ_AUTHORITY,
                requested: FULL_READ_AUTHORITY,
            },
            |l| l.reserve_with_loans(0, 10, 21, &[], &[(7, FULL_READ_AUTHORITY)]),
        );
        // Even one unit more than the budget allows is refused as a whole.
        rejected(
            &mut ledger,
            &TaskLedgerError::ShareExceedsAuthority {
                resource: 7,
                outstanding: FULL_READ_AUTHORITY,
                requested: 1,
            },
            |l| l.reserve_with_loans(0, 10, 21, &[], &[(7, 1)]),
        );
        // Both parties read; a third task does not.
        ledger.read_resource(0, 7).unwrap();
        ledger.read_resource(1, 7).unwrap();
        ledger.acquire(0, 8).unwrap();
        ledger.reserve(0, 10, 23, &[8]).unwrap();
        ledger.commit(0, 23, 2, 31).unwrap();
        assert_eq!(
            ledger.read_resource(2, 7).unwrap_err(),
            TaskLedgerError::NotReadable {
                task: 2,
                resource: 7
            }
        );
        // The lending scope stays busy while the borrower's handle is live.
        rejected(&mut ledger, &TaskLedgerError::ScopeBusy(10), |l| {
            l.exit_scope(0, 10)
        });
    }

    /// T04 acceptance 2, first clause. A creation that never happened opened no
    /// loan: the reserved resource returns to full owner authority, byte for
    /// byte, and an unrelated live sibling is untouched.
    #[test]
    fn failed_creation_restores_a_reserved_read_loan_to_full_owner_authority() {
        let mut ledger = opened();
        for resource in [7, 8] {
            ledger.acquire(0, resource).unwrap();
        }
        spawn(&mut ledger, 20, 1, 30, &[8]);
        let before = ledger.clone();
        ledger
            .reserve_with_loans(0, 10, 21, &[], &[(7, FULL_READ_AUTHORITY)])
            .unwrap();
        ledger.rollback(0, 21).unwrap();
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Live(Custodian::Task(0)))
        );
        assert!(ledger.outstanding_loans().is_empty());
        ledger.use_resource(0, 7).unwrap();
        ledger.read_resource(0, 7).unwrap();
        // Only the rolled-back reservation distinguishes the two ledgers; the
        // prior sibling still holds its own moved resource.
        assert_eq!(ledger.resource_state(8), before.resource_state(8));
        assert_eq!(ledger.task_state(1), Some(TaskState::Active));
        ledger.use_resource(1, 8).unwrap();
        // A rolled-back loan reservation cannot be committed or replayed.
        unchanged(&mut ledger, |l| l.commit(0, 21, 2, 31));
        unchanged(&mut ledger, |l| l.rollback(0, 21));
    }

    /// T04 acceptance 2 (join) and 3. Completion does not close a loan; only a
    /// successful creator join does. A failed join reclaims nothing, forever.
    #[test]
    fn only_a_successful_creator_join_closes_a_read_loan() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        ledger
            .reserve_with_loans(0, 10, 20, &[], &[(7, FULL_READ_AUTHORITY)])
            .unwrap();
        ledger.commit(0, 20, 1, 30).unwrap();
        ledger.complete(1, TaskOutcome::I32(5), &[]).unwrap();

        // A completed-but-unjoined child still holds the loan.
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::whole_loan(0, 1))
        );
        ledger.read_resource(0, 7).unwrap();
        rejected(
            &mut ledger,
            &TaskLedgerError::LoanOutstanding {
                resource: 7,
                outstanding: FULL_READ_AUTHORITY,
                borrowers: vec![1],
            },
            |l| l.drop_resource(0, 7),
        );

        // A failed join changes nothing that could be reclaimed, and is
        // terminal: neither a retry nor a second failure is admitted.
        let mut failed = ledger.clone();
        failed.fail_join(0, 30).unwrap();
        assert_eq!(
            failed.resource_state(7),
            Some(ResourceState::whole_loan(0, 1))
        );
        assert_eq!(failed.outstanding_loans(), [7]);
        rejected(&mut failed, &TaskLedgerError::JoinFailed(30), |l| {
            l.join(0, 30)
        });
        rejected(&mut failed, &TaskLedgerError::JoinFailed(30), |l| {
            l.fail_join(0, 30)
        });
        rejected(&mut failed, &TaskLedgerError::ScopeBusy(10), |l| {
            l.exit_scope(0, 10)
        });
        rejected(
            &mut failed,
            &TaskLedgerError::LoanOutstanding {
                resource: 7,
                outstanding: FULL_READ_AUTHORITY,
                borrowers: vec![1],
            },
            |l| l.drop_resource(0, 7),
        );
        // Nobody else may collect it either.
        unchanged(&mut failed, |l| l.fail_join(1, 30));

        // The successful join is the loan's only close, and it restores exact
        // exclusive authority to the original owner.
        let capsule = ledger.join(0, 30).unwrap();
        assert_eq!(capsule.outcome, TaskOutcome::I32(5));
        assert!(capsule.resources.is_empty());
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Live(Custodian::Task(0)))
        );
        assert!(ledger.outstanding_loans().is_empty());
        assert_eq!(ledger.task_state(1), Some(TaskState::Joined));
        ledger.drop_resource(0, 7).unwrap();
        ledger.exit_scope(0, 10).unwrap();
    }

    /// A loan and a move in one creation are parked and resolved together, and
    /// the same resource can never be both.
    #[test]
    fn one_creation_cannot_move_and_loan_the_same_resource() {
        let mut ledger = opened();
        for resource in [7, 8] {
            ledger.acquire(0, resource).unwrap();
        }
        rejected(&mut ledger, &TaskLedgerError::DuplicateCapture(7), |l| {
            l.reserve_with_loans(0, 10, 20, &[7], &[(7, FULL_READ_AUTHORITY)])
        });
        ledger
            .reserve_with_loans(0, 10, 21, &[8], &[(7, FULL_READ_AUTHORITY)])
            .unwrap();
        ledger.commit(0, 21, 1, 30).unwrap();
        assert_eq!(
            ledger.resource_state(8),
            Some(ResourceState::Live(Custodian::Task(1)))
        );
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::whole_loan(0, 1))
        );
        // The child owns its move and only borrows the loan.
        ledger.use_resource(1, 8).unwrap();
        ledger.drop_resource(1, 8).unwrap();
        ledger.complete(1, TaskOutcome::Unit, &[]).unwrap();
        ledger.join(0, 30).unwrap();
        ledger.use_resource(0, 7).unwrap();
        assert_eq!(ledger.resource_state(8), Some(ResourceState::Dropped));
    }

    #[test]
    fn foreign_scope_and_reservation_operations_cannot_steal_custody() {
        let mut ledger = opened();
        spawn(&mut ledger, 20, 1, 30, &[]);
        ledger.acquire(0, 7).unwrap();
        ledger.reserve(0, 10, 21, &[7]).unwrap();
        unchanged(&mut ledger, |l| l.reserve(1, 10, 22, &[]));
        unchanged(&mut ledger, |l| l.exit_scope(1, 10));
        unchanged(&mut ledger, |l| l.rollback(1, 21));
        unchanged(&mut ledger, |l| l.commit(1, 21, 2, 31));
        unchanged(&mut ledger, |l| l.drop_resource(0, 7));
        unchanged(&mut ledger, |l| l.acquire(1, 7));
        ledger.commit(0, 21, 2, 31).unwrap();
        unchanged(&mut ledger, |l| l.rollback(0, 21));
        unchanged(&mut ledger, |l| l.commit(0, 21, 3, 32));
        ledger.complete(2, TaskOutcome::Unit, &[7]).unwrap();
        unchanged(&mut ledger, |l| l.acquire(2, 8));
        unchanged(&mut ledger, |l| l.enter_scope(2, 11));
        ledger.join(0, 31).unwrap();
        ledger.use_resource(0, 7).unwrap();
    }

    const HALF: u32 = FULL_READ_AUTHORITY / 2;

    /// Lend `share` of resource 7 to a fresh child, in one step.
    fn lend_to(
        ledger: &mut ScopedTaskLedger,
        reservation: u64,
        child: u64,
        handle: u64,
        share: u32,
    ) {
        ledger
            .reserve_with_loans(0, 10, reservation, &[], &[(7, share)])
            .unwrap();
        ledger.commit(0, reservation, child, handle).unwrap();
    }

    /// T05 acceptance 1 and 3. Two positive complementary shares of one
    /// generation are admitted, both borrowers read the *same* identity, and
    /// nothing about the resource is duplicated: there is one entry in the
    /// ledger, one owner, and one outstanding total.
    #[test]
    fn two_positive_complementary_shares_of_one_resource_admit_two_readers() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lend_to(&mut ledger, 20, 1, 30, HALF);
        assert_eq!(ledger.lent_share(7), HALF);
        // The owner keeps its own read across lending; a committed share is not
        // a pending one, so the read window reopens as soon as the child exists.
        ledger.read_resource(0, 7).unwrap();
        lend_to(&mut ledger, 21, 2, 31, HALF);

        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Loaned {
                owner: 0,
                shares: vec![
                    (ShareHolder::Borrower(1), HALF),
                    (ShareHolder::Borrower(2), HALF)
                ],
            })
        );
        assert_eq!(ledger.lent_share(7), FULL_READ_AUTHORITY);
        assert_eq!(ledger.loan_borrowers(7), [1, 2]);
        assert_eq!(ledger.held_share(1, 7), HALF);
        assert_eq!(ledger.held_share(2, 7), HALF);
        // One allocation, counted once: the two readers are two holders of one
        // identity, not two resources.
        assert_eq!(ledger.outstanding_loans(), [7]);
        assert_eq!(ledger.held_share(0, 7), 0);

        for reader in [0, 1, 2] {
            ledger.read_resource(reader, 7).unwrap();
        }
        // A third task holds no share and reads nothing.
        ledger.acquire(0, 8).unwrap();
        ledger.reserve(0, 10, 22, &[8]).unwrap();
        ledger.commit(0, 22, 3, 32).unwrap();
        assert_eq!(
            ledger.read_resource(3, 7).unwrap_err(),
            TaskLedgerError::NotReadable {
                task: 3,
                resource: 7
            }
        );
    }

    /// T05 acceptance 1. Excess allocation of authority is refused as a whole:
    /// the shares that are already out are never partially topped up, and the
    /// refusal names the outstanding total and the request.
    #[test]
    fn excess_allocation_of_read_authority_is_refused_without_partial_grant() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lend_to(&mut ledger, 20, 1, 30, HALF + 1);
        rejected(
            &mut ledger,
            &TaskLedgerError::ShareExceedsAuthority {
                resource: 7,
                outstanding: HALF + 1,
                requested: HALF,
            },
            |l| l.reserve_with_loans(0, 10, 21, &[], &[(7, HALF)]),
        );
        // Nothing was granted, so the remaining authority is unchanged.
        assert_eq!(ledger.lent_share(7), HALF + 1);
        assert_eq!(ledger.loan_borrowers(7), [1]);
        // A share that exactly fills the remainder is admitted.
        lend_to(&mut ledger, 22, 2, 31, HALF - 1);
        assert_eq!(ledger.lent_share(7), FULL_READ_AUTHORITY);

        // A zero share and an over-full share are not shares.
        for requested in [0, FULL_READ_AUTHORITY + 1] {
            let mut fresh = opened();
            fresh.acquire(0, 9).unwrap();
            rejected(
                &mut fresh,
                &TaskLedgerError::InvalidShare {
                    resource: 9,
                    requested,
                },
                |l| l.reserve_with_loans(0, 10, 20, &[], &[(9, requested)]),
            );
        }
    }

    /// T05 acceptance 1 and 2. Reconstruction to full authority is refused
    /// while *any* holder retains a share: one join is not enough, and the
    /// refusal reports exactly what is still out.
    #[test]
    fn reconstruction_to_full_authority_waits_for_every_share() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lend_to(&mut ledger, 20, 1, 30, HALF);
        lend_to(&mut ledger, 21, 2, 31, HALF);
        for task in [0, 1, 2] {
            assert_eq!(
                ledger.use_resource(task, 7).unwrap_err(),
                TaskLedgerError::LoanOutstanding {
                    resource: 7,
                    outstanding: FULL_READ_AUTHORITY,
                    borrowers: vec![1, 2],
                }
            );
        }
        ledger.complete(1, TaskOutcome::Unit, &[]).unwrap();
        ledger.complete(2, TaskOutcome::Unit, &[]).unwrap();
        // Completion is not the return of a share.
        assert_eq!(ledger.lent_share(7), FULL_READ_AUTHORITY);
        ledger.join(0, 30).unwrap();
        // Exactly one share came back, and the other borrower is untouched.
        assert_eq!(ledger.lent_share(7), HALF);
        assert_eq!(ledger.loan_borrowers(7), [2]);
        assert_eq!(ledger.held_share(2, 7), HALF);
        rejected(
            &mut ledger,
            &TaskLedgerError::LoanOutstanding {
                resource: 7,
                outstanding: HALF,
                borrowers: vec![2],
            },
            |l| l.drop_resource(0, 7),
        );
        // The joined child is gone, but its share is not somebody else's.
        assert_eq!(ledger.held_share(1, 7), 0);
        ledger.join(0, 31).unwrap();
        assert_eq!(ledger.lent_share(7), 0);
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Live(Custodian::Task(0)))
        );
        ledger.use_resource(0, 7).unwrap();
        ledger.drop_resource(0, 7).unwrap();
        ledger.exit_scope(0, 10).unwrap();
    }

    /// A failed creation returns exactly its own pending share and leaves a
    /// sibling's committed share alone. A pending share is readable by nobody,
    /// including the owner: it has left and has not arrived.
    #[test]
    fn a_failed_second_creation_returns_only_its_own_pending_share() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lend_to(&mut ledger, 20, 1, 30, HALF);
        let before = ledger.clone();
        ledger
            .reserve_with_loans(0, 10, 21, &[], &[(7, HALF)])
            .unwrap();
        assert_eq!(ledger.lent_share(7), FULL_READ_AUTHORITY);
        assert_eq!(
            ledger.read_resource(0, 7).unwrap_err(),
            TaskLedgerError::NotReadable {
                task: 0,
                resource: 7
            }
        );
        // The committed sibling reads throughout the window.
        ledger.read_resource(1, 7).unwrap();
        ledger.rollback(0, 21).unwrap();
        assert_eq!(ledger.resource_state(7), before.resource_state(7));
        assert_eq!(ledger.lent_share(7), HALF);
        ledger.read_resource(0, 7).unwrap();
        ledger.read_resource(1, 7).unwrap();
    }

    /// A third simultaneous reader is a represented refusal, not an
    /// unmodelled case, even when the remaining authority would allow it.
    #[test]
    fn a_third_simultaneous_reader_is_refused_by_the_declared_bound() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lend_to(&mut ledger, 20, 1, 30, 1);
        lend_to(&mut ledger, 21, 2, 31, 1);
        assert_eq!(ledger.lent_share(7), 2);
        rejected(
            &mut ledger,
            &TaskLedgerError::TooManyReaders {
                resource: 7,
                limit: MAX_READERS,
            },
            |l| l.reserve_with_loans(0, 10, 22, &[], &[(7, 1)]),
        );
        // Returning one share makes room again, without minting any.
        ledger.complete(1, TaskOutcome::Unit, &[]).unwrap();
        ledger.join(0, 30).unwrap();
        lend_to(&mut ledger, 23, 3, 33, 1);
        assert_eq!(ledger.loan_borrowers(7), [2, 3]);
        assert_eq!(ledger.lent_share(7), 2);
    }

    /// Only the owner lends, and a share is never taken from someone who does
    /// not have it: a borrower cannot lend onward and a stranger cannot lend
    /// at all.
    #[test]
    fn only_the_owner_may_lend_a_share_of_its_own_resource() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lend_to(&mut ledger, 20, 1, 30, HALF);
        ledger.enter_scope(1, 11).unwrap();
        for stranger in [1u64, 2] {
            let expected = if stranger == 1 {
                TaskLedgerError::NotCustodian {
                    task: 1,
                    resource: 7,
                }
            } else {
                TaskLedgerError::UnknownTask(2)
            };
            rejected(&mut ledger, &expected, |l| {
                l.reserve_with_loans(stranger, 11, 21 + stranger, &[], &[(7, 1)])
            });
        }
    }

    // -----------------------------------------------------------------------
    // Card `WRITE-LEND-TASKS`: range write leases.
    // -----------------------------------------------------------------------

    const LEFT: LeaseRange = LeaseRange { low: 0, high: 2 };
    const RIGHT: LeaseRange = LeaseRange { low: 2, high: 4 };

    /// Lease `range` of resource 7 to a fresh child, in one step.
    fn lease_to(
        ledger: &mut ScopedTaskLedger,
        reservation: u64,
        child: u64,
        handle: u64,
        range: LeaseRange,
    ) {
        ledger
            .reserve_with_transfers(0, 10, reservation, &[], &[], &[(7, range)])
            .unwrap();
        ledger.commit(0, reservation, child, handle).unwrap();
    }

    /// The whole lease lifecycle: a pending lease grants nobody anything and a
    /// failed creation returns it exactly; a committed lease suspends the
    /// owner's write, read, release and further transfers; two disjoint
    /// leases are both out at once; and only each creator join returns its
    /// lease — the owner's whole-payload authority is reconstructed exactly
    /// when the last one comes back.
    #[test]
    fn a_write_lease_transfers_authority_for_exactly_the_spawn_to_join_window() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        let before = ledger.clone();
        ledger
            .reserve_with_transfers(0, 10, 20, &[], &[], &[(7, LEFT)])
            .unwrap();
        // The reservation window grants nobody anything, read included.
        assert!(ledger.read_resource(0, 7).is_err());
        ledger.rollback(0, 20).unwrap();
        assert_eq!(
            ledger.resource_state(7),
            before.resource_state(7),
            "a creation that never happened leased nothing"
        );

        lease_to(&mut ledger, 21, 1, 31, LEFT);
        lease_to(&mut ledger, 22, 2, 32, RIGHT);
        assert_eq!(ledger.leased_ranges(7), [LEFT, RIGHT]);
        assert_eq!(ledger.lease_holders(7), [1, 2]);
        assert_eq!(ledger.held_lease(1, 7), Some(LEFT));
        assert_eq!(ledger.outstanding_write_leases(), [7]);

        // The transfer: no whole-payload authority for anyone while any lease
        // is out — the owner's write, read, release, move and read-lend are
        // all refused naming the leases.
        let suspended = TaskLedgerError::LeaseOutstanding {
            resource: 7,
            ranges: vec![LEFT, RIGHT],
            holders: vec![1, 2],
        };
        assert_eq!(ledger.use_resource(0, 7).unwrap_err(), suspended);
        assert_eq!(ledger.read_resource(0, 7).unwrap_err(), suspended);
        rejected(&mut ledger, &suspended, |l| l.drop_resource(0, 7));
        rejected(&mut ledger, &suspended, |l| l.reserve(0, 10, 23, &[7]));
        rejected(&mut ledger, &suspended, |l| {
            l.reserve_with_loans(0, 10, 23, &[], &[(7, HALF)])
        });

        // Only each creator join returns its lease; a sibling's stays out.
        ledger.complete(1, TaskOutcome::I32(1), &[]).unwrap();
        ledger.complete(2, TaskOutcome::I32(1), &[]).unwrap();
        ledger.join(0, 31).unwrap();
        assert_eq!(ledger.leased_ranges(7), [RIGHT]);
        assert!(ledger.use_resource(0, 7).is_err());
        ledger.join(0, 32).unwrap();
        // The last lease back reconstructs whole-payload authority exactly.
        ledger.use_resource(0, 7).unwrap();
        ledger.read_resource(0, 7).unwrap();
        ledger.drop_resource(0, 7).unwrap();
        ledger.exit_scope(0, 10).unwrap();
    }

    /// The exclusivity rule is the wave-29 disjointness question: one shared
    /// byte refuses naming both ranges, adjacency is disjoint and admitted,
    /// an empty range is not a lease, and a third simultaneous holder is a
    /// represented refusal.
    #[test]
    fn a_second_lease_is_admitted_exactly_when_disjoint() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lease_to(&mut ledger, 20, 1, 30, LEFT);
        rejected(
            &mut ledger,
            &TaskLedgerError::OverlappingLeases {
                resource: 7,
                requested: LeaseRange { low: 1, high: 3 },
                held: LEFT,
            },
            |l| {
                l.reserve_with_transfers(
                    0,
                    10,
                    21,
                    &[],
                    &[],
                    &[(7, LeaseRange { low: 1, high: 3 })],
                )
            },
        );
        rejected(
            &mut ledger,
            &TaskLedgerError::InvalidLease {
                resource: 7,
                requested: LeaseRange { low: 3, high: 3 },
            },
            |l| {
                l.reserve_with_transfers(
                    0,
                    10,
                    21,
                    &[],
                    &[],
                    &[(7, LeaseRange { low: 3, high: 3 })],
                )
            },
        );
        // Adjacent is disjoint: [0,2) and [2,4) share no byte.
        lease_to(&mut ledger, 22, 2, 32, RIGHT);
        // A third simultaneous holder is refused by count, before any range
        // is compared — the write-side MAX_READERS.
        rejected(
            &mut ledger,
            &TaskLedgerError::TooManyWriters {
                resource: 7,
                limit: MAX_WRITERS,
            },
            |l| {
                l.reserve_with_transfers(
                    0,
                    10,
                    23,
                    &[],
                    &[],
                    &[(7, LeaseRange { low: 4, high: 5 })],
                )
            },
        );
    }

    /// A read share and a write lease of one resource never coexist, in
    /// either order: the refusal names what is out rather than falling to a
    /// custody complaint, and a lease is never taken from a non-owner.
    #[test]
    fn read_shares_and_write_leases_of_one_resource_never_coexist() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lend_to(&mut ledger, 20, 1, 30, HALF);
        rejected(
            &mut ledger,
            &TaskLedgerError::LoanOutstanding {
                resource: 7,
                outstanding: HALF,
                borrowers: vec![1],
            },
            |l| l.reserve_with_transfers(0, 10, 21, &[], &[], &[(7, LEFT)]),
        );
        ledger.complete(1, TaskOutcome::I32(1), &[]).unwrap();
        ledger.join(0, 30).unwrap();
        lease_to(&mut ledger, 22, 2, 32, LEFT);
        rejected(
            &mut ledger,
            &TaskLedgerError::LeaseOutstanding {
                resource: 7,
                ranges: vec![LEFT],
                holders: vec![2],
            },
            |l| l.reserve_with_loans(0, 10, 23, &[], &[(7, 1)]),
        );
        // Only the owner leases: a borrower cannot lease onward, a stranger
        // cannot lease at all.
        ledger.enter_scope(2, 11).unwrap();
        rejected(
            &mut ledger,
            &TaskLedgerError::NotCustodian {
                task: 2,
                resource: 7,
            },
            |l| l.reserve_with_transfers(2, 11, 24, &[], &[], &[(7, RIGHT)]),
        );
    }

    /// Sequential exclusivity is temporal, exactly as on the read side: once
    /// a lease is back, the same bytes lease again.
    #[test]
    fn a_returned_lease_frees_its_bytes_for_the_next_creation() {
        let mut ledger = opened();
        ledger.acquire(0, 7).unwrap();
        lease_to(&mut ledger, 20, 1, 30, LEFT);
        ledger.complete(1, TaskOutcome::Unit, &[]).unwrap();
        ledger.join(0, 30).unwrap();
        lease_to(&mut ledger, 21, 2, 31, LEFT);
        assert_eq!(ledger.lease_holders(7), [2]);
    }
}
