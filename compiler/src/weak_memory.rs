//! Cycle 17 execution checker for the C11 concurrency boundary.
//!
//! Candidate executions explicitly supply reads-from, modification order,
//! synchronizes-with, and the sequentially-consistent order.  Program order is
//! derived per thread and is never inferred from inter-thread list position.
//! The atomic subset follows C11 N1570 (not C++20/RC11): plain loads/stores
//! and same-thread store release sequences. RMWs, fences, and consume are not
//! represented. Acceptance checks one supplied graph, not program exploration.

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
    let mo = transitive_closure(&execution.modification_order);
    for edge in &execution.reads_from {
        let Some((location, _, order)) = by_id
            .get(&edge.to)
            .and_then(|event| atomic_load(&event.kind))
        else {
            continue;
        };
        if !order.has_acquire() {
            continue;
        }
        // N1570 5.1.2.4p10-11: every intervening modification must
        // continue the sequence. A foreign plain store terminates it;
        // a later store by the head's thread cannot restart that sequence.
        for head in &execution.events {
            if !atomic_write(&head.kind).is_some_and(|(loc, _, order)| {
                loc == location && order.is_some_and(MemoryOrder::has_release)
            }) {
                continue;
            }
            if edge.from == head.id || (mo.contains(&RelationEdge::new(head.id, edge.from))
                && execution.events.iter().filter(|candidate| {
                    candidate.id == edge.from || (mo.contains(&RelationEdge::new(head.id, candidate.id))
                        && mo.contains(&RelationEdge::new(candidate.id, edge.from)))
                }).all(|candidate| {
                    candidate.thread == head.thread
                        && matches!(candidate.kind, MemoryEventKind::AtomicStore { location: loc, .. } if loc == location)
                })) {
                result.insert(RelationEdge::new(head.id, edge.to));
            }
        }
    }
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
