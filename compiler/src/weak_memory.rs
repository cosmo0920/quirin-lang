//! Cycle 17 execution checker for the C11 concurrency boundary.
//!
//! Candidate executions explicitly supply reads-from, modification order,
//! synchronizes-with, and the sequentially-consistent order.  Program order is
//! derived per thread and is never inferred from inter-thread list position.

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
    CoherenceViolation(MemoryEventId),
    DataRace {
        first: MemoryEventId,
        second: MemoryEventId,
    },
    ObservationMismatch,
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

    let expected_sw = expected_synchronizes_with(execution, &by_id);
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

fn invalid_order(kind: &MemoryEventKind) -> bool {
    match kind {
        MemoryEventKind::AtomicStore { order, .. } => order.has_acquire() && !order.has_release(),
        MemoryEventKind::AtomicLoad { order, .. } => order.has_release() && !order.has_acquire(),
        _ => false,
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

fn expected_synchronizes_with(
    execution: &C11Execution,
    by_id: &BTreeMap<MemoryEventId, &MemoryEvent>,
) -> BTreeSet<RelationEdge> {
    let mut result = BTreeSet::new();
    for edge in &execution.reads_from {
        let release_acquire = by_id
            .get(&edge.from)
            .and_then(|event| atomic_write(&event.kind))
            .and_then(|(_, _, order)| order)
            .zip(
                by_id
                    .get(&edge.to)
                    .and_then(|event| atomic_load(&event.kind))
                    .map(|(_, _, order)| order),
            )
            .is_some_and(|(store, load)| store.has_release() && load.has_acquire());
        if release_acquire {
            result.insert(*edge);
        }
    }
    // A supplied unlock-to-lock edge is valid when it names the same mutex.
    for edge in &execution.synchronizes_with {
        if matches!(
            (by_id.get(&edge.from).map(|event| &event.kind), by_id.get(&edge.to).map(|event| &event.kind)),
            (
                Some(MemoryEventKind::MutexUnlock { mutex: left }),
                Some(MemoryEventKind::MutexLock { mutex: right })
            ) if left == right
        ) {
            result.insert(*edge);
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
            matches!(
                event.kind,
                MemoryEventKind::AtomicStore {
                    order: MemoryOrder::SequentiallyConsistent,
                    ..
                } | MemoryEventKind::AtomicLoad {
                    order: MemoryOrder::SequentiallyConsistent,
                    ..
                }
            )
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
    if happens_before.iter().any(|edge| {
        position
            .get(&edge.from)
            .zip(position.get(&edge.to))
            .is_some_and(|(left, right)| left >= right)
    }) {
        errors.insert(WeakMemoryError::InvalidSequentiallyConsistentOrder);
    }
    let _ = by_id;
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
