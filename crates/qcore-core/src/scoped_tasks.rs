//! Checked task/resource custody for reserve, commit, completion and creator join.
//!
//! This ledger does not model scalar storage, mutexes, usage grades,
//! scheduling, races, SSA or a backend. Copy scalar captures need no custody
//! transition. Error outcomes are collected by join, never implicitly propagated.
//! Every failed operation leaves the complete ledger unchanged.
//!
//! Card T04 adds one bounded borrowing form: a **scoped read loan**. A loaned
//! resource keeps its owner and gains one borrower; both may read it and
//! neither may use it exclusively, so mutation, release, re-loan and move are
//! refused for the whole outstanding window. The creator's successful join is
//! the only operation that closes a loan. This is not a share algebra: loans
//! are not split, counted or transferred, and a second loan of the same
//! resource is refused rather than merged.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceState {
    Live(Custodian),
    Dropped,
    /// A scoped read loan. `owner` never stops owning the resource and never
    /// regains exclusive authority before the creator's join closes the loan;
    /// `borrower` holds read authority only. Both may read.
    Loaned {
        owner: TaskId,
        borrower: TaskId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskOutcome {
    Unit,
    I32(i32),
    OwnedBuffer(ResourceId),
    Error(String),
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
    /// Exclusive use was attempted while a read loan is outstanding. This is
    /// not a custody complaint: the owner still owns the resource.
    LoanOutstanding {
        resource: ResourceId,
        borrower: TaskId,
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
    /// Resources parked for a read loan rather than a move. Rollback restores
    /// them to the creator exactly as it restores moves; commit places them
    /// under a loan instead of transferring custody.
    loans: Vec<ResourceId>,
    state: ReservationState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Handle {
    creator: TaskId,
    scope: ScopeId,
    child: TaskId,
    consumed: bool,
    /// Loans this creation opened. Only a successful join closes them.
    loans: Vec<ResourceId>,
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
        self.resources.get(&resource).copied()
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

    /// Exclusive authority: mutation, release, move and re-loan. An outstanding
    /// read loan suspends it for the owner as well as for everyone else.
    pub fn use_resource(&self, task: TaskId, resource: ResourceId) -> Result<(), TaskLedgerError> {
        self.active(task)?;
        if let Some(ResourceState::Loaned { borrower, .. }) = self.resource_state(resource) {
            return Err(TaskLedgerError::LoanOutstanding { resource, borrower });
        }
        if self.resource_state(resource) != Some(ResourceState::Live(Custodian::Task(task))) {
            return Err(TaskLedgerError::NotCustodian { task, resource });
        }
        Ok(())
    }

    /// Shared read authority. The owner keeps it while the resource is loaned,
    /// and the borrower holds exactly it — never more. A dropped resource, a
    /// reserved one and a completion capsule are readable by nobody.
    pub fn read_resource(&self, task: TaskId, resource: ResourceId) -> Result<(), TaskLedgerError> {
        self.active(task)?;
        match self.resource_state(resource) {
            Some(ResourceState::Live(Custodian::Task(custodian))) if custodian == task => Ok(()),
            Some(ResourceState::Loaned { owner, borrower })
                if owner == task || borrower == task =>
            {
                Ok(())
            }
            _ => Err(TaskLedgerError::NotReadable { task, resource }),
        }
    }

    /// The borrower of an outstanding loan, if this resource has one.
    pub fn loan_borrower(&self, resource: ResourceId) -> Option<TaskId> {
        match self.resource_state(resource) {
            Some(ResourceState::Loaned { borrower, .. }) => Some(borrower),
            _ => None,
        }
    }

    /// Every resource with an outstanding read loan, in identity order.
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

    /// Reserve ordered move and read-loan lists before attempting the external
    /// spawn. Both lists are parked in the reservation, so the creator has no
    /// authority at all — not even read — over either while creation is in
    /// flight, and a failed creation restores both exactly.
    pub fn reserve_with_loans(
        &mut self,
        creator: TaskId,
        scope: ScopeId,
        reservation: ReservationId,
        resources: &[ResourceId],
        loans: &[ResourceId],
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
                .chain(loans)
                .copied()
                .collect::<Vec<ResourceId>>();
            unique(&parked)?;
            for &resource in &parked {
                ledger.use_resource(creator, resource)?;
            }
            for &resource in &parked {
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
                    state: ReservationState::Pending,
                },
            );
            Ok(())
        })
    }

    /// Failed spawn restores only this reservation; earlier siblings persist.
    /// Moves and loans are restored identically: a creation that did not happen
    /// opened no loan.
    pub fn rollback(
        &mut self,
        creator: TaskId,
        reservation: ReservationId,
    ) -> Result<(), TaskLedgerError> {
        self.transaction(|ledger| {
            let record = ledger.pending(creator, reservation)?.clone();
            for resource in record.resources.iter().chain(&record.loans) {
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
                    failed: false,
                },
            );
            for resource in reservation_record.resources {
                ledger
                    .resources
                    .insert(resource, ResourceState::Live(Custodian::Task(child)));
            }
            for resource in reservation_record.loans {
                ledger.resources.insert(
                    resource,
                    ResourceState::Loaned {
                        owner: creator,
                        borrower: child,
                    },
                );
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
            if matches!(&outcome, TaskOutcome::Error(label) if label.is_empty()) {
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
            if let TaskOutcome::OwnedBuffer(resource) = &outcome
                && resources != [*resource]
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
            // Closing the loans is part of the same transaction as collecting
            // the capsule: a loan the ledger cannot account for aborts the join
            // instead of silently returning exclusive authority.
            for &resource in &record.loans {
                if ledger.resource_state(resource)
                    != Some(ResourceState::Loaned {
                        owner: creator,
                        borrower: record.child,
                    })
                {
                    return Err(TaskLedgerError::LoanNotOutstanding(resource));
                }
            }
            let task = ledger.tasks.get_mut(&record.child).unwrap();
            let capsule = task.capsule.take().unwrap();
            task.state = TaskState::Joined;
            for &resource in capsule.resources.iter().chain(&record.loans) {
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
        ledger.reserve_with_loans(0, 10, 20, &[], &[7]).unwrap();
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
            Some(ResourceState::Loaned {
                owner: 0,
                borrower: 1
            })
        );
        assert_eq!(ledger.loan_borrower(7), Some(1));
        assert_eq!(ledger.outstanding_loans(), [7]);

        let suspended = TaskLedgerError::LoanOutstanding {
            resource: 7,
            borrower: 1,
        };
        for task in [0, 1] {
            assert_eq!(ledger.use_resource(task, 7).unwrap_err(), suspended);
            rejected(&mut ledger, &suspended, |l| l.drop_resource(task, 7));
        }
        // Re-loaning and moving the same resource are exclusive uses too.
        rejected(&mut ledger, &suspended, |l| {
            l.reserve_with_loans(0, 10, 21, &[], &[7])
        });
        rejected(&mut ledger, &suspended, |l| l.reserve(0, 10, 22, &[7]));
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
        ledger.reserve_with_loans(0, 10, 21, &[], &[7]).unwrap();
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
        ledger.reserve_with_loans(0, 10, 20, &[], &[7]).unwrap();
        ledger.commit(0, 20, 1, 30).unwrap();
        ledger.complete(1, TaskOutcome::I32(5), &[]).unwrap();

        // A completed-but-unjoined child still holds the loan.
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Loaned {
                owner: 0,
                borrower: 1
            })
        );
        ledger.read_resource(0, 7).unwrap();
        rejected(
            &mut ledger,
            &TaskLedgerError::LoanOutstanding {
                resource: 7,
                borrower: 1,
            },
            |l| l.drop_resource(0, 7),
        );

        // A failed join changes nothing that could be reclaimed, and is
        // terminal: neither a retry nor a second failure is admitted.
        let mut failed = ledger.clone();
        failed.fail_join(0, 30).unwrap();
        assert_eq!(
            failed.resource_state(7),
            Some(ResourceState::Loaned {
                owner: 0,
                borrower: 1
            })
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
                borrower: 1,
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
            l.reserve_with_loans(0, 10, 20, &[7], &[7])
        });
        ledger.reserve_with_loans(0, 10, 21, &[8], &[7]).unwrap();
        ledger.commit(0, 21, 1, 30).unwrap();
        assert_eq!(
            ledger.resource_state(8),
            Some(ResourceState::Live(Custodian::Task(1)))
        );
        assert_eq!(
            ledger.resource_state(7),
            Some(ResourceState::Loaned {
                owner: 0,
                borrower: 1
            })
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
}
