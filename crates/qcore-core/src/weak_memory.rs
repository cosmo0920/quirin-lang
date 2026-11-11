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

//! Cycle 17 execution checker for the C11 concurrency boundary.
//!
//! Candidate executions explicitly supply reads-from, modification order,
//! synchronizes-with, and the sequentially-consistent order.  Program order is
//! derived per thread and is never inferred from inter-thread list position.
//! Acceptance checks one supplied graph, not program exploration; card
//! CG29-EXECUTION-COVERAGE-AND-RECLAMATION's [`crate::execution_coverage`]
//! enumerates every candidate execution of a bounded fragment and checks each
//! one here.
//!
//! # The standard followed
//!
//! The atomic subset follows **C11 as published (N1570)**, one standard and
//! no mix: not C++20's weakened release sequences (P0982), not RC11, and not
//! C++14's added seq_cst-fence/modification-order rule (LWG 2130). Card
//! CG28-WEAK-MEMORY-EVENTS (wave 48) added, additively:
//!
//! * **Read-modify-writes** ([`MemoryEventKind::AtomicReadModifyWrite`]:
//!   exchange, fetch-and-op, op-and-fetch) with N1570 7.17.3's atomicity:
//!   the write an RMW reads from is its immediate predecessor in the
//!   modification order ([`WeakMemoryError::AtomicityViolation`]).
//! * **Compare-exchange** ([`MemoryEventKind::AtomicCompareExchange`]): a
//!   success is a read-modify-write with the success order; a failure is a
//!   load with the failure order, which (7.17.7.4p2) is neither release nor
//!   acq_rel and no stronger than the success order
//!   ([`compare_exchange_failure_admitted`]). A strong exchange fails only
//!   when the observed value differs from the expected one; a weak one may
//!   also fail spuriously, on an equal value.
//! * **Fences** ([`MemoryEventKind::AtomicFence`]): thread fences
//!   synchronise per 7.17.4p2–p4 (fence–fence, fence–operation,
//!   operation–fence, through the hypothetical release sequence); a seq_cst
//!   thread fence joins the total order S with 7.17.3's three fence
//!   observation rules. A signal fence orders only against a signal handler
//!   of its own thread (7.17.4.2), which the model has none of, so it creates
//!   no synchronisation and joins no S.
//! * **Release sequences** (5.1.2.4p10, N1570 wording): the head, then
//!   every modification in the modification order that is performed by the
//!   head's thread or is an atomic read-modify-write.
//!
//! `memory_order_consume` stays **out of scope**: the model has no consume
//! order and no dependency-ordered-before relation. Reading consume as
//! acquire would add happens-before edges the program does not have, which is
//! unsound for race freedom, so the source route refuses a consume load,
//! read-modify-write or compare-exchange order. A consume *fence* is an
//! acquire fence by the standard's own definition (7.17.4.1p2), not a
//! strengthening. The arithmetic of an RMW (the written value as a function
//! of the read one) is the supplied graph's premise and is not checked.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub type MemoryEventId = u32;
pub type MemoryThreadId = u32;
pub type MemoryLocation = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryOrder {
    Relaxed,
    Acquire,
    Release,
    AcquireRelease,
    SequentiallyConsistent,
}

impl MemoryOrder {
    const fn has_acquire(self) -> bool {
        matches!(
            self,
            Self::Acquire | Self::AcquireRelease | Self::SequentiallyConsistent
        )
    }

    const fn has_release(self) -> bool {
        matches!(
            self,
            Self::Release | Self::AcquireRelease | Self::SequentiallyConsistent
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryEventKind {
    AtomicInit {
        location: MemoryLocation,
        value: i64,
    },
    AtomicStore {
        location: MemoryLocation,
        value: i64,
        order: MemoryOrder,
    },
    AtomicLoad {
        location: MemoryLocation,
        value: i64,
        order: MemoryOrder,
    },
    NonAtomicRead {
        location: MemoryLocation,
    },
    NonAtomicWrite {
        location: MemoryLocation,
    },
    MutexUnlock {
        mutex: u32,
    },
    MutexLock {
        mutex: u32,
    },
    Spawn {
        child: MemoryThreadId,
    },
    Join {
        child: MemoryThreadId,
    },
    /// CG28: an atomic read-modify-write (exchange, fetch-and-op,
    /// op-and-fetch): it reads `read_value` and writes `written_value` in one
    /// indivisible step.
    AtomicReadModifyWrite {
        location: MemoryLocation,
        read_value: i64,
        written_value: i64,
        order: MemoryOrder,
    },
    /// CG28: a compare-exchange. `succeeded` names the outcome this graph
    /// chose: a success reads `observed` (equal to `expected`) and writes
    /// `desired` with the `success` order; a failure is a load of `observed`
    /// with the `failure` order.
    AtomicCompareExchange {
        location: MemoryLocation,
        expected: i64,
        observed: i64,
        desired: i64,
        success: MemoryOrder,
        failure: MemoryOrder,
        strength: CompareExchangeStrength,
        succeeded: bool,
    },
    /// CG28: `atomic_thread_fence` or `atomic_signal_fence`.
    AtomicFence {
        order: MemoryOrder,
        scope: FenceScope,
    },
}

/// CG28: whether a compare-exchange may fail spuriously (C11 7.17.7.4p4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompareExchangeStrength {
    Strong,
    Weak,
}

/// CG28: a thread fence, or a signal fence (ordering only against a signal
/// handler of the same thread, C11 7.17.4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FenceScope {
    Thread,
    Signal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryEvent {
    pub id: MemoryEventId,
    pub thread: MemoryThreadId,
    pub kind: MemoryEventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelationEdge {
    pub from: MemoryEventId,
    pub to: MemoryEventId,
}

impl RelationEdge {
    pub const fn new(from: MemoryEventId, to: MemoryEventId) -> Self {
        Self { from, to }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct C11Execution {
    pub events: Vec<MemoryEvent>,
    pub reads_from: Vec<RelationEdge>,
    pub modification_order: Vec<RelationEdge>,
    pub synchronizes_with: Vec<RelationEdge>,
    pub sequentially_consistent: Vec<MemoryEventId>,
    /// Complete order of successful operations for each initially unlocked,
    /// non-recursive mutex. Includes each lock/unlock exactly once.
    pub mutex_order: BTreeMap<u32, Vec<MemoryEventId>>,
    pub source_observation: Vec<i64>,
    pub target_observation: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct C11ExecutionCertificate {
    pub event_count: usize,
    pub happens_before: Vec<RelationEdge>,
    pub source_observation: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WeakMemoryError {
    DuplicateEvent(MemoryEventId),
    UnknownEndpoint(RelationEdge),
    InvalidMemoryOrder(MemoryEventId),
    InvalidReadsFrom(RelationEdge),
    MissingReadsFrom(MemoryEventId),
    MultipleReadsFrom(MemoryEventId),
    InvalidModificationOrder(RelationEdge),
    IncompleteModificationOrder {
        location: MemoryLocation,
        left: MemoryEventId,
        right: MemoryEventId,
    },
    InvalidSynchronizesWith(RelationEdge),
    MissingSynchronizesWith(RelationEdge),
    CyclicRelation(&'static str),
    InvalidSequentiallyConsistentOrder,
    InvalidMutexHistory(u32),
    CoherenceViolation(MemoryEventId),
    DataRace {
        first: MemoryEventId,
        second: MemoryEventId,
    },
    ObservationMismatch,
    /// CG28: a read-modify-write does not read the immediate predecessor of
    /// its own write in the modification order.
    AtomicityViolation(MemoryEventId),
    /// CG28: a compare-exchange's outcome contradicts its values (a success
    /// that observed another value, a strong failure that observed the
    /// expected one).
    InvalidCompareExchange(MemoryEventId),
}

impl fmt::Display for WeakMemoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WeakMemoryError {}

pub fn check_c11_execution(
    execution: &C11Execution,
) -> Result<C11ExecutionCertificate, Vec<WeakMemoryError>> {
    let mut errors = BTreeSet::new();
    let mut by_id = BTreeMap::new();
    for event in &execution.events {
        if by_id.insert(event.id, event).is_some() {
            errors.insert(WeakMemoryError::DuplicateEvent(event.id));
        }
        if invalid_order(&event.kind) {
            errors.insert(WeakMemoryError::InvalidMemoryOrder(event.id));
        }
    }

    let program_order = program_order(&execution.events);
    validate_edges(&execution.reads_from, &by_id, &mut errors);
    validate_edges(&execution.modification_order, &by_id, &mut errors);
    validate_edges(&execution.synchronizes_with, &by_id, &mut errors);
    validate_reads_from(execution, &by_id, &mut errors);
    validate_modification_order(execution, &by_id, &mut errors);
    validate_compare_exchanges(&execution.events, &mut errors);
    validate_atomicity(execution, &by_id, &mut errors);

    let mutex_sw = validate_mutex_histories(execution, &by_id, &mut errors);
    let mut expected_sw = expected_synchronizes_with(execution, &by_id);
    expected_sw.extend(mutex_sw);
    let supplied_sw = execution
        .synchronizes_with
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for edge in supplied_sw.difference(&expected_sw) {
        errors.insert(WeakMemoryError::InvalidSynchronizesWith(*edge));
    }
    for edge in expected_sw.difference(&supplied_sw) {
        errors.insert(WeakMemoryError::MissingSynchronizesWith(*edge));
    }

    if has_cycle(&program_order) {
        errors.insert(WeakMemoryError::CyclicRelation("sequenced-before"));
    }
    if has_cycle(&execution.modification_order) {
        errors.insert(WeakMemoryError::CyclicRelation("modification-order"));
    }
    let mut happens_before_edges = program_order;
    happens_before_edges.extend(execution.synchronizes_with.iter().copied());
    happens_before_edges.extend(spawn_join_edges(&execution.events));
    if has_cycle(&happens_before_edges) {
        errors.insert(WeakMemoryError::CyclicRelation("happens-before"));
    }
    let happens_before = transitive_closure(&happens_before_edges);
    validate_coherence(execution, &by_id, &happens_before, &mut errors);
    validate_sc(execution, &by_id, &happens_before, &mut errors);
    validate_non_atomic_races(&execution.events, &happens_before, &mut errors);
    if execution.source_observation != execution.target_observation {
        errors.insert(WeakMemoryError::ObservationMismatch);
    }

    if errors.is_empty() {
        Ok(C11ExecutionCertificate {
            event_count: execution.events.len(),
            happens_before: happens_before.into_iter().collect(),
            source_observation: execution.source_observation.clone(),
        })
    } else {
        Err(errors.into_iter().collect())
    }
}

/// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): whether this model
/// represents the event's memory order for its operation (a load is relaxed,
/// acquire or seq_cst; a store relaxed, release or seq_cst). The C source
/// route maps a routed atomic to an event only when this holds, so a routed
/// access never carries an order `check_c11_execution` would refuse. Additive:
/// it is exactly the negation of the checker's own `InvalidMemoryOrder` test.
#[must_use]
pub fn event_order_represented(kind: &MemoryEventKind) -> bool {
    !invalid_order(kind)
}

/// Card CG28-WEAK-MEMORY-EVENTS: C11 7.17.7.4p2 — a compare-exchange's
/// failure order is neither release nor acq_rel, and is no stronger than its
/// success order: its acquire strength (relaxed < acquire < seq_cst) is at
/// most that of the success order's load part (release reads as relaxed,
/// acq_rel as acquire). `QCore.WeakMemory.casOrdersValid`.
#[must_use]
pub fn compare_exchange_failure_admitted(success: MemoryOrder, failure: MemoryOrder) -> bool {
    let failure_rank = match failure {
        MemoryOrder::Relaxed => 0,
        MemoryOrder::Acquire => 1,
        MemoryOrder::SequentiallyConsistent => 2,
        MemoryOrder::Release | MemoryOrder::AcquireRelease => return false,
    };
    let success_rank = match success {
        MemoryOrder::Relaxed | MemoryOrder::Release => 0,
        MemoryOrder::Acquire | MemoryOrder::AcquireRelease => 1,
        MemoryOrder::SequentiallyConsistent => 2,
    };
    failure_rank <= success_rank
}

/// Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION (additive): the
/// synchronises-with edges an execution's reads, modification order and
/// fences determine, exactly the set `check_c11_execution` requires the
/// supplied relation to equal (mutex edges excluded: the enumerator's
/// fragments have no mutex). `QCore.WeakMemory.Execution.swEdge`.
pub(crate) fn derived_synchronizes_with(execution: &C11Execution) -> Vec<RelationEdge> {
    let by_id = execution
        .events
        .iter()
        .map(|event| (event.id, event))
        .collect::<BTreeMap<_, _>>();
    expected_synchronizes_with(execution, &by_id)
        .into_iter()
        .collect()
}

fn invalid_order(kind: &MemoryEventKind) -> bool {
    match kind {
        MemoryEventKind::AtomicStore { order, .. } => !matches!(
            order,
            MemoryOrder::Relaxed | MemoryOrder::Release | MemoryOrder::SequentiallyConsistent
        ),
        MemoryEventKind::AtomicLoad { order, .. } => !matches!(
            order,
            MemoryOrder::Relaxed | MemoryOrder::Acquire | MemoryOrder::SequentiallyConsistent
        ),
        // CG28: an RMW and a fence take every order the model has (a relaxed
        // fence has no effect, 7.17.4.1p2).
        MemoryEventKind::AtomicCompareExchange {
            success, failure, ..
        } => !compare_exchange_failure_admitted(*success, *failure),
        _ => false,
    }
}

/// The order an atomic operation performs with: a compare-exchange's
/// success order when it succeeded, its failure order otherwise.
fn operation_order(kind: &MemoryEventKind) -> Option<MemoryOrder> {
    match kind {
        MemoryEventKind::AtomicStore { order, .. }
        | MemoryEventKind::AtomicLoad { order, .. }
        | MemoryEventKind::AtomicReadModifyWrite { order, .. } => Some(*order),
        MemoryEventKind::AtomicCompareExchange {
            success,
            failure,
            succeeded,
            ..
        } => Some(if *succeeded { *success } else { *failure }),
        _ => None,
    }
}

/// CG28: an atomic read-modify-write, a compare-exchange's success included.
fn is_read_modify_write(kind: &MemoryEventKind) -> bool {
    matches!(
        kind,
        MemoryEventKind::AtomicReadModifyWrite { .. }
            | MemoryEventKind::AtomicCompareExchange {
                succeeded: true,
                ..
            }
    )
}

/// A thread fence's order; `None` for anything else, a signal fence included.
fn thread_fence(kind: &MemoryEventKind) -> Option<MemoryOrder> {
    match kind {
        MemoryEventKind::AtomicFence {
            order,
            scope: FenceScope::Thread,
        } => Some(*order),
        _ => None,
    }
}

/// CG28: a compare-exchange's outcome agrees with its values.
fn validate_compare_exchanges(events: &[MemoryEvent], errors: &mut BTreeSet<WeakMemoryError>) {
    for event in events {
        if let MemoryEventKind::AtomicCompareExchange {
            expected,
            observed,
            strength,
            succeeded,
            ..
        } = event.kind
        {
            let consistent = if succeeded {
                observed == expected
            } else {
                strength == CompareExchangeStrength::Weak || observed != expected
            };
            if !consistent {
                errors.insert(WeakMemoryError::InvalidCompareExchange(event.id));
            }
        }
    }
}

/// CG28, N1570 7.17.3 (read-modify-write atomicity): the write an RMW reads
/// from immediately precedes the RMW's own write in the modification order.
fn validate_atomicity(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
    errors: &mut BTreeSet<WeakMemoryError>,
) {
    let mo = transitive_closure(&execution.modification_order);
    for edge in &execution.reads_from {
        if !by_id
            .get(&edge.to)
            .is_some_and(|event| is_read_modify_write(&event.kind))
        {
            continue;
        }
        let adjacent = mo.contains(edge)
            && !execution.events.iter().any(|between| {
                mo.contains(&RelationEdge::new(edge.from, between.id))
                    && mo.contains(&RelationEdge::new(between.id, edge.to))
            });
        if !adjacent {
            errors.insert(WeakMemoryError::AtomicityViolation(edge.to));
        }
    }
}

fn validate_edges(
    edges: &[RelationEdge],
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
    errors: &mut BTreeSet<WeakMemoryError>,
) {
    for edge in edges {
        if edge.from == edge.to || !by_id.contains_key(&edge.from) || !by_id.contains_key(&edge.to)
        {
            errors.insert(WeakMemoryError::UnknownEndpoint(*edge));
        }
    }
}

fn program_order(events: &[MemoryEvent]) -> Vec<RelationEdge> {
    let mut previous = BTreeMap::<MemoryThreadId, MemoryEventId>::new();
    let mut edges = Vec::new();
    for event in events {
        if let Some(before) = previous.insert(event.thread, event.id) {
            edges.push(RelationEdge::new(before, event.id));
        }
    }
    edges
}

fn atomic_write(kind: &MemoryEventKind) -> Option<(MemoryLocation, i64, Option<MemoryOrder>)> {
    match kind {
        MemoryEventKind::AtomicInit { location, value } => Some((*location, *value, None)),
        MemoryEventKind::AtomicStore {
            location,
            value,
            order,
        } => Some((*location, *value, Some(*order))),
        // CG28: an RMW's write, and a successful compare-exchange's.
        MemoryEventKind::AtomicReadModifyWrite {
            location,
            written_value,
            order,
            ..
        } => Some((*location, *written_value, Some(*order))),
        MemoryEventKind::AtomicCompareExchange {
            location,
            desired,
            success,
            succeeded: true,
            ..
        } => Some((*location, *desired, Some(*success))),
        _ => None,
    }
}

fn atomic_load(kind: &MemoryEventKind) -> Option<(MemoryLocation, i64, MemoryOrder)> {
    match kind {
        MemoryEventKind::AtomicLoad {
            location,
            value,
            order,
        } => Some((*location, *value, *order)),
        // CG28: an RMW's read, and every compare-exchange's (with the order
        // of the outcome it took).
        MemoryEventKind::AtomicReadModifyWrite {
            location,
            read_value,
            order,
            ..
        } => Some((*location, *read_value, *order)),
        MemoryEventKind::AtomicCompareExchange {
            location, observed, ..
        } => Some((*location, *observed, operation_order(kind)?)),
        _ => None,
    }
}

fn validate_reads_from(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
    errors: &mut BTreeSet<WeakMemoryError>,
) {
    let mut counts = BTreeMap::<MemoryEventId, usize>::new();
    for edge in &execution.reads_from {
        *counts.entry(edge.to).or_default() += 1;
        let valid = by_id
            .get(&edge.from)
            .and_then(|event| atomic_write(&event.kind))
            .zip(
                by_id
                    .get(&edge.to)
                    .and_then(|event| atomic_load(&event.kind)),
            )
            .is_some_and(
                |((write_location, write_value, _), (load_location, load_value, _))| {
                    write_location == load_location && write_value == load_value
                },
            );
        if !valid {
            errors.insert(WeakMemoryError::InvalidReadsFrom(*edge));
        }
    }
    for event in &execution.events {
        if atomic_load(&event.kind).is_some() {
            match counts.get(&event.id).copied().unwrap_or(0) {
                0 => {
                    errors.insert(WeakMemoryError::MissingReadsFrom(event.id));
                }
                1 => {}
                _ => {
                    errors.insert(WeakMemoryError::MultipleReadsFrom(event.id));
                }
            }
        }
    }
}

fn validate_modification_order(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
    errors: &mut BTreeSet<WeakMemoryError>,
) {
    for edge in &execution.modification_order {
        let same_location = by_id
            .get(&edge.from)
            .and_then(|event| atomic_write(&event.kind))
            .zip(
                by_id
                    .get(&edge.to)
                    .and_then(|event| atomic_write(&event.kind)),
            )
            .is_some_and(|(left, right)| left.0 == right.0);
        if !same_location {
            errors.insert(WeakMemoryError::InvalidModificationOrder(*edge));
        }
    }
    let closure = transitive_closure(&execution.modification_order);
    let mut writes = BTreeMap::<MemoryLocation, Vec<MemoryEventId>>::new();
    for event in &execution.events {
        if let Some((location, _, _)) = atomic_write(&event.kind) {
            writes.entry(location).or_default().push(event.id);
        }
    }
    for (location, ids) in writes {
        for (index, &left) in ids.iter().enumerate() {
            for &right in &ids[index + 1..] {
                if !closure.contains(&RelationEdge::new(left, right))
                    && !closure.contains(&RelationEdge::new(right, left))
                {
                    errors.insert(WeakMemoryError::IncompleteModificationOrder {
                        location,
                        left,
                        right,
                    });
                }
            }
        }
    }
}

/// Each event's thread and its index in that thread's program order.
fn sequence_positions(events: &[MemoryEvent]) -> BTreeMap<MemoryEventId, (MemoryThreadId, usize)> {
    let mut next = BTreeMap::<MemoryThreadId, usize>::new();
    let mut positions = BTreeMap::new();
    for event in events {
        let index = next.entry(event.thread).or_default();
        positions.entry(event.id).or_insert((event.thread, *index));
        *index += 1;
    }
    positions
}

/// Sequenced-before: the same thread, earlier in its program order.
fn sequenced_before(
    positions: &BTreeMap<MemoryEventId, (MemoryThreadId, usize)>,
    first: MemoryEventId,
    second: MemoryEventId,
) -> bool {
    positions
        .get(&first)
        .zip(positions.get(&second))
        .is_some_and(|((left_thread, left), (right_thread, right))| {
            left_thread == right_thread && left < right
        })
}

/// An atomic operation's write (not the non-atomic `AtomicInit`): the
/// modifications a release sequence, real or hypothetical, is made of.
fn atomic_operation_write(kind: &MemoryEventKind) -> Option<MemoryLocation> {
    match kind {
        MemoryEventKind::AtomicInit { .. } => None,
        _ => atomic_write(kind).map(|(location, _, _)| location),
    }
}

/// N1570 5.1.2.4p10, C11's own wording (not C++20's): `write` is in the
/// release sequence headed by `head` (real, or hypothetical as 7.17.4 uses
/// it) when it is the head, or follows it in the modification order and it
/// and every modification between them is performed by the head's thread or
/// is an atomic read-modify-write. `QCore.WeakMemory.inReleaseSequence`.
fn in_release_sequence(
    events: &[MemoryEvent],
    mo: &BTreeSet<RelationEdge>,
    head: &MemoryEvent,
    write: MemoryEventId,
) -> bool {
    write == head.id
        || (mo.contains(&RelationEdge::new(head.id, write))
            && events
                .iter()
                .filter(|candidate| {
                    candidate.id == write
                        || (mo.contains(&RelationEdge::new(head.id, candidate.id))
                            && mo.contains(&RelationEdge::new(candidate.id, write)))
                })
                .all(|candidate| {
                    is_read_modify_write(&candidate.kind)
                        || (candidate.thread == head.thread
                            && atomic_operation_write(&candidate.kind).is_some())
                }))
}

/// The synchronizes-with edges a graph's reads determine (N1570 5.1.2.4p11
/// and, card CG28, the fence rules of 7.17.4p2–p4). For each read `r` of `w`
/// at `M`, and each atomic modification `X` of `M` whose (hypothetical)
/// release sequence holds `w`:
///
/// * `X` release, `r` acquire: `X` sw `r` (5.1.2.4p11);
/// * `X` release, `r` sequenced before an acquire fence `B`: `X` sw `B` (p4);
/// * a release fence `F` sequenced before `X`, `r` acquire: `F` sw `r` (p3);
/// * a release fence `F` sequenced before `X`, `r` sequenced before an
///   acquire fence `B`: `F` sw `B` (p2).
///
/// Signal fences take part in none. `QCore.WeakMemory.expectedSw`.
fn expected_synchronizes_with(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
) -> BTreeSet<RelationEdge> {
    let mut result = BTreeSet::new();
    let mo = transitive_closure(&execution.modification_order);
    let positions = sequence_positions(&execution.events);
    let fences = |acquire: bool| {
        execution
            .events
            .iter()
            .filter(move |event| {
                thread_fence(&event.kind).is_some_and(|order| {
                    if acquire {
                        order.has_acquire()
                    } else {
                        order.has_release()
                    }
                })
            })
            .collect::<Vec<_>>()
    };
    let acquire_fences = fences(true);
    let release_fences = fences(false);
    for edge in &execution.reads_from {
        let Some((location, _, read_order)) = by_id
            .get(&edge.to)
            .and_then(|event| atomic_load(&event.kind))
        else {
            continue;
        };
        let reader_fences = acquire_fences
            .iter()
            .filter(|fence| sequenced_before(&positions, edge.to, fence.id))
            .collect::<Vec<_>>();
        // A foreign plain store terminates a release sequence; a later store
        // by the head's thread cannot restart it.
        for head in &execution.events {
            if atomic_operation_write(&head.kind) != Some(location)
                || !in_release_sequence(&execution.events, &mo, head, edge.from)
            {
                continue;
            }
            if operation_order(&head.kind).is_some_and(MemoryOrder::has_release) {
                if read_order.has_acquire() {
                    result.insert(RelationEdge::new(head.id, edge.to));
                }
                for fence in &reader_fences {
                    result.insert(RelationEdge::new(head.id, fence.id));
                }
            }
            for release in release_fences
                .iter()
                .filter(|fence| sequenced_before(&positions, fence.id, head.id))
            {
                if read_order.has_acquire() {
                    result.insert(RelationEdge::new(release.id, edge.to));
                }
                for fence in &reader_fences {
                    result.insert(RelationEdge::new(release.id, fence.id));
                }
            }
        }
    }
    result.retain(|edge| edge.from != edge.to);
    result
}

// Mutex ordering is an explicit candidate premise, never inferred from the
// inter-thread position of events or accepted solely from supplied SW edges.
fn validate_mutex_histories(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
    errors: &mut BTreeSet<WeakMemoryError>,
) -> BTreeSet<RelationEdge> {
    let mut inventory = BTreeMap::<u32, BTreeSet<MemoryEventId>>::new();
    for event in &execution.events {
        if let MemoryEventKind::MutexLock { mutex } | MemoryEventKind::MutexUnlock { mutex } =
            event.kind
        {
            inventory.entry(mutex).or_default().insert(event.id);
        }
    }
    let mut result = BTreeSet::new();
    for mutex in inventory
        .keys()
        .chain(execution.mutex_order.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        let Some(order) = execution.mutex_order.get(&mutex) else {
            errors.insert(WeakMemoryError::InvalidMutexHistory(mutex));
            continue;
        };
        let ids = order.iter().copied().collect::<BTreeSet<_>>();
        if inventory.get(&mutex) != Some(&ids) || ids.len() != order.len() {
            errors.insert(WeakMemoryError::InvalidMutexHistory(mutex));
            continue;
        }
        let mut owner = None;
        let mut last_unlock = None;
        let mut edges = BTreeSet::new();
        let mut valid = true;
        for id in order {
            let event = by_id[id];
            match event.kind {
                MemoryEventKind::MutexLock { .. } if owner.is_none() => {
                    owner = Some(event.thread);
                    if let Some(unlock) = last_unlock {
                        edges.insert(RelationEdge::new(unlock, *id));
                    }
                }
                MemoryEventKind::MutexUnlock { .. } if owner == Some(event.thread) => {
                    owner = None;
                    last_unlock = Some(*id);
                }
                _ => {
                    valid = false;
                }
            }
        }
        // A finite prefix may end with a held lock. Its order must also agree
        // with sequencing, atomics, and thread lifecycle happens-before.
        let mut combined = program_order(&execution.events);
        combined.extend(execution.synchronizes_with.iter().copied());
        combined.extend(spawn_join_edges(&execution.events));
        combined.extend(edges.iter().copied());
        combined.extend(
            order
                .windows(2)
                .map(|pair| RelationEdge::new(pair[0], pair[1])),
        );
        if !valid || has_cycle(&combined) {
            errors.insert(WeakMemoryError::InvalidMutexHistory(mutex));
        } else {
            result.extend(edges);
        }
    }
    result
}

fn spawn_join_edges(events: &[MemoryEvent]) -> Vec<RelationEdge> {
    let first = events.iter().fold(BTreeMap::new(), |mut map, event| {
        map.entry(event.thread).or_insert(event.id);
        map
    });
    let last = events.iter().fold(BTreeMap::new(), |mut map, event| {
        map.insert(event.thread, event.id);
        map
    });
    let mut edges = Vec::new();
    for event in events {
        match event.kind {
            MemoryEventKind::Spawn { child } => {
                if let Some(&child_first) = first.get(&child) {
                    edges.push(RelationEdge::new(event.id, child_first));
                }
            }
            MemoryEventKind::Join { child } => {
                if let Some(&child_last) = last.get(&child) {
                    edges.push(RelationEdge::new(child_last, event.id));
                }
            }
            _ => {}
        }
    }
    edges
}

fn validate_coherence(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
    happens_before: &BTreeSet<RelationEdge>,
    errors: &mut BTreeSet<WeakMemoryError>,
) {
    let modification_order = transitive_closure(&execution.modification_order);
    // Write/write coherence (N1570 5.1.2.4p7).
    for edge in happens_before {
        if by_id
            .get(&edge.from)
            .and_then(|event| atomic_write(&event.kind))
            .zip(
                by_id
                    .get(&edge.to)
                    .and_then(|event| atomic_write(&event.kind)),
            )
            .is_some_and(|(left, right)| left.0 == right.0)
            && !modification_order.contains(edge)
        {
            errors.insert(WeakMemoryError::CoherenceViolation(edge.to));
        }
    }
    for read in &execution.reads_from {
        if happens_before.contains(&RelationEdge::new(read.to, read.from)) {
            errors.insert(WeakMemoryError::CoherenceViolation(read.to));
        }
        for event in &execution.events {
            if by_id
                .get(&read.from)
                .and_then(|source| atomic_write(&source.kind))
                .zip(atomic_write(&event.kind))
                .is_some_and(|(source, candidate)| source.0 == candidate.0)
                && modification_order.contains(&RelationEdge::new(read.from, event.id))
                && happens_before.contains(&RelationEdge::new(event.id, read.to))
            {
                errors.insert(WeakMemoryError::CoherenceViolation(read.to));
            }
            // Read/write coherence: a later write must follow the read's
            // source in modification order, including when it is that source.
            if by_id
                .get(&read.from)
                .and_then(|source| atomic_write(&source.kind))
                .zip(atomic_write(&event.kind))
                .is_some_and(|(source, candidate)| source.0 == candidate.0)
                && happens_before.contains(&RelationEdge::new(read.to, event.id))
                && !modification_order.contains(&RelationEdge::new(read.from, event.id))
            {
                errors.insert(WeakMemoryError::CoherenceViolation(read.to));
            }
        }
        // Read/read coherence (N1570 5.1.2.4p22).
        for later in &execution.reads_from {
            if happens_before.contains(&RelationEdge::new(read.to, later.to))
                && modification_order.contains(&RelationEdge::new(later.from, read.from))
            {
                errors.insert(WeakMemoryError::CoherenceViolation(later.to));
            }
        }
    }
}

fn validate_sc(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
    happens_before: &BTreeSet<RelationEdge>,
    errors: &mut BTreeSet<WeakMemoryError>,
) {
    let expected = execution
        .events
        .iter()
        .filter(|event| {
            // CG28: every seq_cst operation (an RMW, a compare-exchange by
            // the order of its outcome) and every seq_cst thread fence.
            operation_order(&event.kind)
                .or_else(|| thread_fence(&event.kind))
                .is_some_and(|order| order == MemoryOrder::SequentiallyConsistent)
        })
        .map(|event| event.id)
        .collect::<BTreeSet<_>>();
    let supplied = execution
        .sequentially_consistent
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if expected != supplied || supplied.len() != execution.sequentially_consistent.len() {
        errors.insert(WeakMemoryError::InvalidSequentiallyConsistentOrder);
        return;
    }
    let position = execution
        .sequentially_consistent
        .iter()
        .enumerate()
        .map(|(position, id)| (*id, position))
        .collect::<BTreeMap<_, _>>();
    let modification_order = transitive_closure(&execution.modification_order);
    if happens_before
        .iter()
        .chain(modification_order.iter())
        .any(|edge| {
            position
                .get(&edge.from)
                .zip(position.get(&edge.to))
                .is_some_and(|(left, right)| left >= right)
        })
    {
        errors.insert(WeakMemoryError::InvalidSequentiallyConsistentOrder);
    }
    // N1570 7.17.3p6: an SC load observes the last preceding SC
    // modification, or a non-SC modification that does not happen before it.
    // Coherence separately constrains visibility of the non-SC source.
    for read in &execution.reads_from {
        let Some(&load_position) = position.get(&read.to) else {
            continue;
        };
        let Some((location, _, _)) = by_id
            .get(&read.to)
            .and_then(|event| atomic_load(&event.kind))
        else {
            continue;
        };
        let last = execution.sequentially_consistent[..load_position]
            .iter()
            .rev()
            .find(|id| {
                by_id
                    .get(id)
                    .and_then(|event| atomic_write(&event.kind))
                    .is_some_and(|(loc, _, _)| loc == location)
            });
        let valid = if position.contains_key(&read.from) {
            last == Some(&read.from)
        } else {
            last.is_none_or(|last| !happens_before.contains(&RelationEdge::new(read.from, *last)))
        };
        if !valid {
            errors.insert(WeakMemoryError::InvalidSequentiallyConsistentOrder);
        }
    }
    validate_sc_fences(execution, by_id, &position, &modification_order, errors);
}

/// Card CG28: N1570 7.17.3's three seq_cst fence rules (C11's, without
/// C++14's modification-order rule). For a read `B` of `W` at `M`:
///
/// 1. for a seq_cst fence `X` sequenced before `B`, with `L` the last seq_cst
///    modification of `M` before `X` in S, `W` is `L` or follows it in the
///    modification order;
/// 2. for a seq_cst fence `X` and an atomic modification `A` of `M`
///    sequenced before it, with `B` after `X` in S, `W` is `A` or follows it;
/// 3. for seq_cst fences `X` before `Y` in S, `A` sequenced before `X` and
///    `Y` before `B`, `W` is `A` or follows it.
///
/// `QCore.WeakMemory.scFenceRulesHold`.
fn validate_sc_fences(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
    position: &BTreeMap<MemoryEventId, usize>,
    modification_order: &BTreeSet<RelationEdge>,
    errors: &mut BTreeSet<WeakMemoryError>,
) {
    let sequence = sequence_positions(&execution.events);
    let fences = execution
        .events
        .iter()
        .filter(|event| thread_fence(&event.kind) == Some(MemoryOrder::SequentiallyConsistent))
        .filter_map(|event| position.get(&event.id).map(|at| (event, *at)))
        .collect::<Vec<_>>();
    let at_or_after = |source: MemoryEventId, bound: MemoryEventId| {
        source == bound || modification_order.contains(&RelationEdge::new(bound, source))
    };
    for read in &execution.reads_from {
        let Some((location, _, _)) = by_id
            .get(&read.to)
            .and_then(|event| atomic_load(&event.kind))
        else {
            continue;
        };
        let modifications = execution
            .events
            .iter()
            .filter(|event| atomic_operation_write(&event.kind) == Some(location))
            .collect::<Vec<_>>();
        let mut valid = true;
        for &(fence, fence_at) in &fences {
            // Rule 1.
            if sequenced_before(&sequence, fence.id, read.to)
                && let Some(last) = execution.sequentially_consistent[..fence_at]
                    .iter()
                    .rev()
                    .find(|id| {
                        by_id
                            .get(id)
                            .and_then(|event| atomic_write(&event.kind))
                            .is_some_and(|(loc, _, _)| loc == location)
                    })
            {
                valid &= at_or_after(read.from, *last);
            }
            for modification in &modifications {
                if !sequenced_before(&sequence, modification.id, fence.id) {
                    continue;
                }
                // Rule 2.
                if position.get(&read.to).is_some_and(|at| *at > fence_at) {
                    valid &= at_or_after(read.from, modification.id);
                }
                // Rule 3.
                for &(later, later_at) in &fences {
                    if later_at > fence_at && sequenced_before(&sequence, later.id, read.to) {
                        valid &= at_or_after(read.from, modification.id);
                    }
                }
            }
        }
        if !valid {
            errors.insert(WeakMemoryError::InvalidSequentiallyConsistentOrder);
        }
    }
}

fn validate_non_atomic_races(
    events: &[MemoryEvent],
    happens_before: &BTreeSet<RelationEdge>,
    errors: &mut BTreeSet<WeakMemoryError>,
) {
    for (index, left) in events.iter().enumerate() {
        for right in &events[index + 1..] {
            if left.thread == right.thread {
                continue;
            }
            if let (Some((left_location, left_write)), Some((right_location, right_write))) = (
                non_atomic_access(&left.kind),
                non_atomic_access(&right.kind),
            ) && left_location == right_location
                && (left_write || right_write)
                && !happens_before.contains(&RelationEdge::new(left.id, right.id))
                && !happens_before.contains(&RelationEdge::new(right.id, left.id))
            {
                errors.insert(WeakMemoryError::DataRace {
                    first: left.id,
                    second: right.id,
                });
            }
        }
    }
}

fn non_atomic_access(kind: &MemoryEventKind) -> Option<(MemoryLocation, bool)> {
    match kind {
        MemoryEventKind::NonAtomicRead { location } => Some((*location, false)),
        MemoryEventKind::NonAtomicWrite { location } => Some((*location, true)),
        _ => None,
    }
}

fn has_cycle(edges: &[RelationEdge]) -> bool {
    let closure = transitive_closure(edges);
    closure.iter().any(|edge| edge.from == edge.to)
}

fn transitive_closure(edges: &[RelationEdge]) -> BTreeSet<RelationEdge> {
    let mut outgoing = BTreeMap::<MemoryEventId, Vec<MemoryEventId>>::new();
    let nodes = edges
        .iter()
        .flat_map(|edge| [edge.from, edge.to])
        .collect::<BTreeSet<_>>();
    for edge in edges {
        outgoing.entry(edge.from).or_default().push(edge.to);
    }
    let mut closure = BTreeSet::new();
    for source in nodes {
        let mut queue = VecDeque::from([source]);
        let mut seen = BTreeSet::new();
        while let Some(current) = queue.pop_front() {
            for &next in outgoing.get(&current).into_iter().flatten() {
                if seen.insert(next) {
                    closure.insert(RelationEdge::new(source, next));
                    queue.push_back(next);
                }
            }
        }
    }
    closure
}

#[cfg(test)]
#[path = "../tests/unit/weak_memory.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/weak_memory_cg28.rs"]
mod cg28_tests;
