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

//! Cycle 14 restricted concurrency and resource-race checker.
//!
//! A program is an explicit event trace.  Vector clocks capture only
//! sequenced-before, spawn/join, and mutex release/acquire edges; input order
//! alone never orders different tasks.  This makes the checker useful as a
//! bounded model without pretending to cover general C11 executions.
//!
//! Initial permissions/custody, guard-to-footprint authority and synchronized
//! handoffs are trusted model inputs, not reconstructed source evidence. Every
//! accepted certificate names that profile and its premises. Each resource ID
//! denotes one unique lifetime; using a fresh ID is required for a new generation.
//! Join is the explicit completion/collection event in this reduced model: it
//! terminates the child without implicitly returning its permissions or custody.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub type TaskId = u32;
/// A unique lifetime identity, never a reusable address or descriptor.
pub type SharedResourceId = u32;
pub type MutexId = u32;
pub type LockRank = u32;
pub const FULL_ACCESS: u16 = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccessFootprint {
    pub resource: SharedResourceId,
    pub start: u32,
    pub end: u32,
}

impl AccessFootprint {
    pub const fn new(resource: SharedResourceId, start: u32, end: u32) -> Self {
        Self {
            resource,
            start,
            end,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.start < self.end
    }

    pub const fn overlaps(self, other: Self) -> bool {
        self.resource == other.resource && self.start < other.end && other.start < self.end
    }

    pub const fn contains(self, other: Self) -> bool {
        self.resource == other.resource && self.start <= other.start && other.end <= self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
    AtomicRead,
    AtomicWrite,
}

impl AccessKind {
    const fn writes(self) -> bool {
        matches!(self, Self::Write | Self::AtomicWrite)
    }

    const fn atomic(self) -> bool {
        matches!(self, Self::AtomicRead | Self::AtomicWrite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessPermission {
    pub task: TaskId,
    pub footprint: AccessFootprint,
    pub share: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConcurrentEvent {
    Spawn {
        parent: TaskId,
        child: TaskId,
    },
    Join {
        parent: TaskId,
        child: TaskId,
    },
    Lock {
        task: TaskId,
        mutex: MutexId,
        rank: LockRank,
    },
    Unlock {
        task: TaskId,
        mutex: MutexId,
    },
    Access {
        task: TaskId,
        footprint: AccessFootprint,
        kind: AccessKind,
        guarded_by: Option<MutexId>,
    },
    TransferCustody {
        from: TaskId,
        to: TaskId,
        resource: SharedResourceId,
        synchronized: bool,
    },
    Destroy {
        task: TaskId,
        resource: SharedResourceId,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConcurrentProgram {
    pub permissions: Vec<AccessPermission>,
    pub initial_custody: BTreeMap<SharedResourceId, TaskId>,
    pub events: Vec<ConcurrentEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConcurrencyDiagnostic {
    InvalidFootprint {
        event: Option<usize>,
    },
    PermissionOverflow {
        resource: SharedResourceId,
    },
    UnknownTask {
        event: usize,
        task: TaskId,
    },
    TerminalTask {
        event: usize,
        task: TaskId,
    },
    NonquiescentJoin {
        event: usize,
        child: TaskId,
    },
    DeadResource {
        event: usize,
        resource: SharedResourceId,
    },
    NonquiescentDestruction {
        event: usize,
        resource: SharedResourceId,
    },
    DuplicateTask {
        event: usize,
        child: TaskId,
    },
    UnknownJoin {
        event: usize,
        child: TaskId,
    },
    OutstandingTask {
        child: TaskId,
    },
    DuplicateLockCapability {
        event: usize,
        mutex: MutexId,
    },
    UnlockWithoutCapability {
        event: usize,
        mutex: MutexId,
    },
    OutstandingLock {
        mutex: MutexId,
    },
    LockOrderViolation {
        event: usize,
        mutex: MutexId,
    },
    JoinWhileLocked {
        event: usize,
        task: TaskId,
    },
    UnauthorizedAccess {
        event: usize,
        task: TaskId,
    },
    WrongMutexInvariant {
        event: usize,
        mutex: MutexId,
    },
    DataRace {
        first: usize,
        second: usize,
    },
    UnsynchronizedTransfer {
        event: usize,
        resource: SharedResourceId,
    },
    WrongCustodian {
        event: usize,
        resource: SharedResourceId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConcurrencyProfile {
    /// No source-to-trace or checked mutex-invariant claim is made by this
    /// profile. `check_concurrent` issues only this one.
    TrustedExplicitTrace,
    /// The fixed-shape checked mutex-invariant profile of
    /// [`crate::mutex_invariant`]. Guard-to-footprint authority and
    /// synchronized handoffs are derived there, not declared; still no
    /// source-to-trace claim.
    CheckedMutexInvariantV1,
    /// The fixed-shape quantitative epoch profile of
    /// [`crate::epoch_accounting`]. It bounds resource residency over one flat
    /// epoch of independent tasks; it makes no scheduling, latency or
    /// source-to-trace claim.
    FlatEpochBoundV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConcurrencyAssumption {
    /// Input permissions/custody describe the admitted initial authority and
    /// resource IDs distinguish lifetimes; unmodeled authority is outside scope.
    InitialAuthorityAndUniqueLifetimeIds,
    /// Each supplied guarded access belongs to its named mutex invariant.
    GuardFootprintAuthority,
    /// Each true handoff flag represents a real synchronization edge.
    SynchronizedCustodyEdges,
    /// The intervals declared at mutex creation correspond to the real C
    /// layout of the protected object. Only `CheckedMutexInvariantV1`.
    DeclaredFootprintShape,
    /// A closing invariant witness names a declared predicate; the predicate
    /// itself is asserted, not proved. Only `CheckedMutexInvariantV1`.
    InvariantPredicateDeclared,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConcurrencyCertificate {
    pub profile: ConcurrencyProfile,
    pub assumptions: BTreeSet<ConcurrencyAssumption>,
    pub checked_events: usize,
    pub completed_tasks: Vec<TaskId>,
    pub destroyed_resources: Vec<SharedResourceId>,
}

impl fmt::Display for ConcurrencyDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

type VectorClock = BTreeMap<TaskId, u64>;

#[derive(Clone, Debug)]
struct RecordedAccess {
    event: usize,
    task: TaskId,
    footprint: AccessFootprint,
    kind: AccessKind,
    clock: VectorClock,
}

fn tick(clocks: &mut BTreeMap<TaskId, VectorClock>, task: TaskId) {
    let clock = clocks.entry(task).or_default();
    *clock.entry(task).or_default() += 1;
}

fn merge_clock(target: &mut VectorClock, source: &VectorClock) {
    for (&task, &time) in source {
        target
            .entry(task)
            .and_modify(|current| *current = (*current).max(time))
            .or_insert(time);
    }
}

fn happens_before(left: &VectorClock, right: &VectorClock) -> bool {
    left.iter()
        .all(|(task, time)| *time <= right.get(task).copied().unwrap_or(0))
}

fn conflicting(left: AccessKind, right: AccessKind) -> bool {
    (left.writes() || right.writes()) && !(left.atomic() && right.atomic())
}

pub fn check_concurrent(
    program: &ConcurrentProgram,
) -> Result<ConcurrencyCertificate, Vec<ConcurrencyDiagnostic>> {
    let mut diagnostics = BTreeSet::new();
    validate_permissions(&program.permissions, &mut diagnostics);

    let mut clocks = BTreeMap::from([(0, BTreeMap::from([(0, 0)]))]);
    let mut task_parents = BTreeMap::<TaskId, TaskId>::new();
    let mut joined = BTreeSet::new();
    // Clock history survives join; execution authority does not.
    let mut active = BTreeSet::from([0]);
    let mut assumptions =
        BTreeSet::from([ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds]);
    let mut locks = BTreeMap::<MutexId, (TaskId, LockRank)>::new();
    let mut held = BTreeMap::<TaskId, Vec<(MutexId, LockRank)>>::new();
    let mut releases = BTreeMap::<MutexId, VectorClock>::new();
    let mut accesses = Vec::<RecordedAccess>::new();
    let mut custody = program.initial_custody.clone();
    let mut destroyed = BTreeSet::new();

    for (event_index, event) in program.events.iter().enumerate() {
        match *event {
            ConcurrentEvent::Spawn { parent, child } => {
                if !active_task(&clocks, &active, event_index, parent, &mut diagnostics) {
                    continue;
                }
                if clocks.contains_key(&child) || child == 0 {
                    diagnostics.insert(ConcurrencyDiagnostic::DuplicateTask {
                        event: event_index,
                        child,
                    });
                    continue;
                }
                tick(&mut clocks, parent);
                let parent_clock = clocks[&parent].clone();
                clocks.insert(child, parent_clock);
                active.insert(child);
                task_parents.insert(child, parent);
            }
            ConcurrentEvent::Join { parent, child } => {
                if !active_task(&clocks, &active, event_index, parent, &mut diagnostics) {
                    continue;
                }
                if task_parents.get(&child) != Some(&parent) || joined.contains(&child) {
                    diagnostics.insert(ConcurrencyDiagnostic::UnknownJoin {
                        event: event_index,
                        child,
                    });
                    continue;
                }
                if held.get(&parent).is_some_and(|guards| !guards.is_empty()) {
                    diagnostics.insert(ConcurrencyDiagnostic::JoinWhileLocked {
                        event: event_index,
                        task: parent,
                    });
                }
                if held.get(&child).is_some_and(|guards| !guards.is_empty())
                    || task_parents
                        .iter()
                        .any(|(descendant, owner)| *owner == child && !joined.contains(descendant))
                {
                    diagnostics.insert(ConcurrencyDiagnostic::NonquiescentJoin {
                        event: event_index,
                        child,
                    });
                    continue;
                }
                joined.insert(child);
                active.remove(&child);
                let child_clock = clocks.get(&child).cloned().unwrap_or_default();
                merge_clock(
                    clocks.get_mut(&parent).expect("parent was checked"),
                    &child_clock,
                );
                tick(&mut clocks, parent);
            }
            ConcurrentEvent::Lock { task, mutex, rank } => {
                if !active_task(&clocks, &active, event_index, task, &mut diagnostics) {
                    continue;
                }
                if locks.contains_key(&mutex) {
                    diagnostics.insert(ConcurrencyDiagnostic::DuplicateLockCapability {
                        event: event_index,
                        mutex,
                    });
                    continue;
                }
                if held
                    .get(&task)
                    .and_then(|guards| guards.last())
                    .is_some_and(|(_, held_rank)| *held_rank >= rank)
                {
                    diagnostics.insert(ConcurrencyDiagnostic::LockOrderViolation {
                        event: event_index,
                        mutex,
                    });
                }
                if let Some(release) = releases.get(&mutex) {
                    merge_clock(clocks.get_mut(&task).expect("task was checked"), release);
                }
                tick(&mut clocks, task);
                locks.insert(mutex, (task, rank));
                held.entry(task).or_default().push((mutex, rank));
            }
            ConcurrentEvent::Unlock { task, mutex } => {
                if !active_task(&clocks, &active, event_index, task, &mut diagnostics) {
                    continue;
                }
                if locks.get(&mutex).map(|owner| owner.0) != Some(task) {
                    diagnostics.insert(ConcurrencyDiagnostic::UnlockWithoutCapability {
                        event: event_index,
                        mutex,
                    });
                    continue;
                }
                tick(&mut clocks, task);
                releases.insert(mutex, clocks[&task].clone());
                locks.remove(&mutex);
                if let Some(guards) = held.get_mut(&task)
                    && let Some(position) = guards.iter().position(|guard| guard.0 == mutex)
                {
                    guards.remove(position);
                }
            }
            ConcurrentEvent::Access {
                task,
                footprint,
                kind,
                guarded_by,
            } => {
                if !active_task(&clocks, &active, event_index, task, &mut diagnostics) {
                    continue;
                }
                if destroyed.contains(&footprint.resource) {
                    diagnostics.insert(ConcurrencyDiagnostic::DeadResource {
                        event: event_index,
                        resource: footprint.resource,
                    });
                    continue;
                }
                if !footprint.is_valid() {
                    diagnostics.insert(ConcurrencyDiagnostic::InvalidFootprint {
                        event: Some(event_index),
                    });
                    continue;
                }
                let guarded = guarded_by.is_some_and(|mutex| {
                    if locks.get(&mutex).map(|owner| owner.0) == Some(task) {
                        assumptions.insert(ConcurrencyAssumption::GuardFootprintAuthority);
                        true
                    } else {
                        diagnostics.insert(ConcurrencyDiagnostic::WrongMutexInvariant {
                            event: event_index,
                            mutex,
                        });
                        false
                    }
                });
                if !guarded && !authorized(&program.permissions, task, footprint, kind) {
                    diagnostics.insert(ConcurrencyDiagnostic::UnauthorizedAccess {
                        event: event_index,
                        task,
                    });
                }
                tick(&mut clocks, task);
                let current = RecordedAccess {
                    event: event_index,
                    task,
                    footprint,
                    kind,
                    clock: clocks[&task].clone(),
                };
                for previous in &accesses {
                    if previous.task != task
                        && previous.footprint.overlaps(footprint)
                        && conflicting(previous.kind, kind)
                        && !happens_before(&previous.clock, &current.clock)
                    {
                        diagnostics.insert(ConcurrencyDiagnostic::DataRace {
                            first: previous.event,
                            second: event_index,
                        });
                    }
                }
                accesses.push(current);
            }
            ConcurrentEvent::TransferCustody {
                from,
                to,
                resource,
                synchronized,
            } => {
                if !active_task(&clocks, &active, event_index, from, &mut diagnostics)
                    || !active_task(&clocks, &active, event_index, to, &mut diagnostics)
                {
                    continue;
                }
                if destroyed.contains(&resource) {
                    diagnostics.insert(ConcurrencyDiagnostic::DeadResource {
                        event: event_index,
                        resource,
                    });
                    continue;
                }
                if custody.get(&resource) != Some(&from) {
                    diagnostics.insert(ConcurrencyDiagnostic::WrongCustodian {
                        event: event_index,
                        resource,
                    });
                } else if !synchronized {
                    diagnostics.insert(ConcurrencyDiagnostic::UnsynchronizedTransfer {
                        event: event_index,
                        resource,
                    });
                } else {
                    assumptions.insert(ConcurrencyAssumption::SynchronizedCustodyEdges);
                    tick(&mut clocks, from);
                    let from_clock = clocks[&from].clone();
                    merge_clock(clocks.get_mut(&to).expect("task was checked"), &from_clock);
                    custody.insert(resource, to);
                }
            }
            ConcurrentEvent::Destroy { task, resource } => {
                if !active_task(&clocks, &active, event_index, task, &mut diagnostics) {
                    continue;
                }
                if destroyed.contains(&resource) {
                    diagnostics.insert(ConcurrencyDiagnostic::DeadResource {
                        event: event_index,
                        resource,
                    });
                    continue;
                }
                if custody.get(&resource) != Some(&task) {
                    diagnostics.insert(ConcurrencyDiagnostic::WrongCustodian {
                        event: event_index,
                        resource,
                    });
                    continue;
                }
                // Fixed permissions belonging to other nonterminal (including
                // future) tasks cannot be silently revoked. Guard footprints are
                // trusted rather than represented, so conservatively require all
                // other tasks' currently held guards to be closed.
                let outstanding_authority = program.permissions.iter().any(|permission| {
                    permission.footprint.resource == resource
                        && permission.share > 0
                        && permission.task != task
                        && !joined.contains(&permission.task)
                }) || locks.values().any(|(owner, _)| *owner != task);
                let unordered_access = accesses.iter().any(|access| {
                    access.footprint.resource == resource
                        && !happens_before(&access.clock, &clocks[&task])
                });
                if outstanding_authority || unordered_access {
                    diagnostics.insert(ConcurrencyDiagnostic::NonquiescentDestruction {
                        event: event_index,
                        resource,
                    });
                    continue;
                }
                destroyed.insert(resource);
                custody.remove(&resource);
                tick(&mut clocks, task);
            }
        }
    }

    for &child in task_parents.keys() {
        if !joined.contains(&child) {
            diagnostics.insert(ConcurrencyDiagnostic::OutstandingTask { child });
        }
    }
    for &mutex in locks.keys() {
        diagnostics.insert(ConcurrencyDiagnostic::OutstandingLock { mutex });
    }
    if diagnostics.is_empty() {
        Ok(ConcurrencyCertificate {
            profile: ConcurrencyProfile::TrustedExplicitTrace,
            assumptions,
            checked_events: program.events.len(),
            completed_tasks: joined.into_iter().collect(),
            destroyed_resources: destroyed.into_iter().collect(),
        })
    } else {
        Err(diagnostics.into_iter().collect())
    }
}

fn active_task(
    clocks: &BTreeMap<TaskId, VectorClock>,
    active: &BTreeSet<TaskId>,
    event: usize,
    task: TaskId,
    diagnostics: &mut BTreeSet<ConcurrencyDiagnostic>,
) -> bool {
    if active.contains(&task) {
        true
    } else if clocks.contains_key(&task) {
        diagnostics.insert(ConcurrencyDiagnostic::TerminalTask { event, task });
        false
    } else {
        diagnostics.insert(ConcurrencyDiagnostic::UnknownTask { event, task });
        false
    }
}

fn authorized(
    permissions: &[AccessPermission],
    task: TaskId,
    footprint: AccessFootprint,
    kind: AccessKind,
) -> bool {
    permissions.iter().any(|permission| {
        permission.task == task
            && permission.footprint.contains(footprint)
            && if kind == AccessKind::Write {
                permission.share == FULL_ACCESS
            } else {
                permission.share > 0
            }
    })
}

fn validate_permissions(
    permissions: &[AccessPermission],
    diagnostics: &mut BTreeSet<ConcurrencyDiagnostic>,
) {
    for permission in permissions {
        if !permission.footprint.is_valid() {
            diagnostics.insert(ConcurrencyDiagnostic::InvalidFootprint { event: None });
        }
    }
    let resources = permissions
        .iter()
        .map(|permission| permission.footprint.resource)
        .collect::<BTreeSet<_>>();
    for resource in resources {
        let points = permissions
            .iter()
            .filter(|permission| permission.footprint.resource == resource)
            .flat_map(|permission| {
                [
                    permission.footprint.start,
                    permission.footprint.end.saturating_sub(1),
                ]
            })
            .collect::<BTreeSet<_>>();
        if points.into_iter().any(|point| {
            permissions
                .iter()
                .filter(|permission| {
                    permission.footprint.resource == resource
                        && permission.footprint.start <= point
                        && point < permission.footprint.end
                })
                .map(|permission| u32::from(permission.share))
                .sum::<u32>()
                > u32::from(FULL_ACCESS)
        }) {
            diagnostics.insert(ConcurrencyDiagnostic::PermissionOverflow { resource });
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/concurrency.rs"]
mod tests;
