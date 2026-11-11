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

//! Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION (wave 48, gap entry W07):
//! race freedom over **every** candidate execution of a bounded fragment.
//!
//! [`crate::weak_memory::check_c11_execution`] decides one supplied candidate
//! execution. One accepted graph is not race freedom (evidence Q34): a
//! program can have an accepted graph and a racy one. This module enumerates
//! every candidate execution of a loop-free fragment and checks each one with
//! that unchanged checker.
//!
//! # The fragment
//!
//! A [`ProgramFragment`] declares its atomic locations with their initial
//! values, and per thread a list of [`GuardedInstruction`]s:
//! * an atomic load, store, exchange or compare-exchange;
//! * a fence;
//! * a non-atomic read or write.
//!
//! A guard is a conjunction of register equalities, evaluated when the
//! instruction is reached; an unassigned register equals nothing. A load or
//! an exchange writes the value it read to its register. A compare-exchange
//! writes 1 on success and 0 on failure. Arithmetic read-modify-writes are
//! not in the fragment (`CG29-ARITHMETIC-RMW-VALUE-DOMAIN`). A
//! [`Instruction::Loop`] is not enumerated, and makes the verdict `Unknown`.
//!
//! # What is enumerated
//!
//! A candidate execution is the tuple C11 quantifies over:
//! * one path per thread, for some value each read returns and some outcome of
//!   each weak compare-exchange on its expected value;
//! * a reads-from source for each read;
//! * a modification order for each atomic location (a permutation of its
//!   writes, encoded as successive pairs);
//! * an order S of the seq_cst operations and fences.
//!
//! Synchronises-with is derived from these. Thread 0 holds each location's
//! initialisation and one spawn per thread, so the initialisations happen
//! before every thread.
//!
//! **The value domain** of a location is its initial value and every literal
//! an instruction of any thread writes there. A read returns the value of
//! the write it reads from, and every write writes such a literal, so the
//! domain holds every value of every consistent execution
//! (`QCore.ExecutionCoverage.read_values_are_literals`). It is computed
//! without control flow, so an out-of-thin-air cycle cannot escape it.
//!
//! # The verdict
//!
//! * [`ProgramVerdict::RaceFree`]: every candidate was checked, and no
//!   consistent one races. A candidate is consistent when the checker reports
//!   at most data races.
//! * [`ProgramVerdict::Race`]: a consistent candidate races. It is an
//!   execution of the fragment, carried as the witness.
//! * [`ProgramVerdict::Unknown`], with a named reason: a loop, a malformed
//!   fragment, an exceeded bound, or no consistent candidate.
//!
//! The bounds are part of the evidence identity ([`CoverageEvidence`]).
//! Within them the enumeration is exact, not an under-approximation: the
//! fragment is loop-free, so every path is finite and generated
//! (`QCore.ExecutionCoverage.encode_mem_candidates`,
//! `verdict_race_free_sound`).
//!
//! Premises: the checker's own premises (rule 11t), and the Rust-to-Lean
//! correspondence of this enumerator by review. That correspondence is bound
//! on the shared fragment vectors by
//! `crates/qcore-core/tests/unit/execution_coverage.rs`, which parses them
//! from `proof/QCore/ExecutionCoverage.lean`. No liveness or eventual
//! observation is claimed (W11).

use std::collections::BTreeSet;

use crate::weak_memory::{
    C11Execution, CompareExchangeStrength, FenceScope, MemoryEvent, MemoryEventId, MemoryEventKind,
    MemoryLocation, MemoryOrder, MemoryThreadId, RelationEdge, WeakMemoryError,
    check_c11_execution, derived_synchronizes_with,
};

/// A thread-local register.
pub type Register = u32;

/// The memory model this module checks executions against, recorded on every
/// evidence identity.
pub const EXECUTION_COVERAGE_STANDARD: &str = "C11 (N1570) candidate executions, each checked by qcore_core::weak_memory::check_c11_execution";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Instruction {
    Load {
        location: MemoryLocation,
        order: MemoryOrder,
        register: Register,
    },
    Store {
        location: MemoryLocation,
        value: i64,
        order: MemoryOrder,
    },
    /// An exchange: writes the literal `value`; the register receives the
    /// old value.
    Exchange {
        location: MemoryLocation,
        value: i64,
        order: MemoryOrder,
        register: Register,
    },
    /// A compare-exchange: the register receives 1 on success, 0 on failure.
    CompareExchange {
        location: MemoryLocation,
        expected: i64,
        desired: i64,
        success: MemoryOrder,
        failure: MemoryOrder,
        strength: CompareExchangeStrength,
        register: Register,
    },
    Fence {
        order: MemoryOrder,
        scope: FenceScope,
    },
    NonAtomicRead {
        location: MemoryLocation,
    },
    NonAtomicWrite {
        location: MemoryLocation,
    },
    /// A loop: not enumerated, so a fragment holding one is `Unknown`.
    Loop,
}

/// An instruction run only when every `(register, value)` of its guard holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardedInstruction {
    pub guard: Vec<(Register, i64)>,
    pub instruction: Instruction,
}

impl GuardedInstruction {
    #[must_use]
    pub fn always(instruction: Instruction) -> Self {
        Self {
            guard: Vec::new(),
            instruction,
        }
    }

    #[must_use]
    pub fn when(register: Register, value: i64, instruction: Instruction) -> Self {
        Self {
            guard: vec![(register, value)],
            instruction,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramFragment {
    /// Each atomic location and its initial value.
    pub atomic_locations: Vec<(MemoryLocation, i64)>,
    /// The threads, numbered from 1 in the executions (thread 0 is the
    /// initialising prelude).
    pub threads: Vec<Vec<GuardedInstruction>>,
}

/// The bounds a verdict is reached within. Exceeding any one is `Unknown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverageBounds {
    pub threads: usize,
    /// The static event bound: the prelude and one event per instruction.
    pub events: usize,
    /// The largest value domain of one location.
    pub values: usize,
    pub candidates: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoverageUnknown {
    Loop,
    /// A location declared twice, an atomic access to an undeclared
    /// location, or a non-atomic access to a declared one (the checker races
    /// only non-atomic accesses against each other).
    Malformed,
    ThreadBound {
        threads: usize,
        bound: usize,
    },
    EventBound {
        events: usize,
        bound: usize,
    },
    ValueBound {
        location: MemoryLocation,
        values: usize,
        bound: usize,
    },
    CandidateBound {
        bound: usize,
    },
    NoConsistentExecution,
}

/// What a verdict was reached over: its evidence identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageEvidence {
    pub standard: &'static str,
    pub bounds: CoverageBounds,
    pub threads: usize,
    pub event_bound: usize,
    /// Each atomic location's value domain.
    pub value_domains: Vec<(MemoryLocation, Vec<i64>)>,
    /// Candidate executions enumerated, each checked.
    pub candidates: usize,
    /// Of those, the consistent ones.
    pub consistent: usize,
}

impl CoverageEvidence {
    /// The assumption a consumer records for a `RaceFree` verdict.
    #[must_use]
    pub fn assumption(&self) -> String {
        format!(
            "every candidate execution of the bounded fragment was checked ({}; threads {} <= {}, events {} <= {}, value domains {:?} each <= {}, candidates {} <= {}, {} consistent); loop-free, so the enumeration is exact within these bounds; no liveness or eventual observation is claimed (W11)",
            self.standard,
            self.threads,
            self.bounds.threads,
            self.event_bound,
            self.bounds.events,
            self.value_domains,
            self.bounds.values,
            self.candidates,
            self.bounds.candidates,
            self.consistent,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramVerdict {
    RaceFree(CoverageEvidence),
    Race {
        evidence: CoverageEvidence,
        witness: Box<C11Execution>,
        first: MemoryEventId,
        second: MemoryEventId,
    },
    Unknown(CoverageUnknown),
}

impl ProgramVerdict {
    #[must_use]
    pub fn status(&self) -> &'static str {
        match self {
            Self::RaceFree(_) => "RaceFree",
            Self::Race { .. } => "Race",
            Self::Unknown(_) => "Unknown",
        }
    }
}

fn holds(env: &[(Register, i64)], guard: &[(Register, i64)]) -> bool {
    guard.iter().all(|(register, value)| {
        env.iter()
            .find(|(bound, _)| bound == register)
            .is_some_and(|(_, current)| current == value)
    })
}

fn read_location(instruction: &Instruction) -> Option<MemoryLocation> {
    match instruction {
        Instruction::Load { location, .. }
        | Instruction::Exchange { location, .. }
        | Instruction::CompareExchange { location, .. } => Some(*location),
        _ => None,
    }
}

/// `QCore.ExecutionCoverage.Instr.step`: the event and register writes of an
/// executed instruction reading `value`, failing spuriously when `spurious`.
fn step(
    instruction: &Instruction,
    value: i64,
    spurious: bool,
) -> Option<(MemoryEventKind, Vec<(Register, i64)>)> {
    Some(match *instruction {
        Instruction::Load {
            location,
            order,
            register,
        } => (
            MemoryEventKind::AtomicLoad {
                location,
                value,
                order,
            },
            vec![(register, value)],
        ),
        Instruction::Store {
            location,
            value: written,
            order,
        } => (
            MemoryEventKind::AtomicStore {
                location,
                value: written,
                order,
            },
            vec![],
        ),
        Instruction::Exchange {
            location,
            value: written,
            order,
            register,
        } => (
            MemoryEventKind::AtomicReadModifyWrite {
                location,
                read_value: value,
                written_value: written,
                order,
            },
            vec![(register, value)],
        ),
        Instruction::CompareExchange {
            location,
            expected,
            desired,
            success,
            failure,
            strength,
            register,
        } => {
            let succeeded =
                value == expected && !(strength == CompareExchangeStrength::Weak && spurious);
            (
                MemoryEventKind::AtomicCompareExchange {
                    location,
                    expected,
                    observed: value,
                    desired,
                    success,
                    failure,
                    strength,
                    succeeded,
                },
                vec![(register, i64::from(succeeded))],
            )
        }
        Instruction::Fence { order, scope } => {
            (MemoryEventKind::AtomicFence { order, scope }, vec![])
        }
        Instruction::NonAtomicRead { location } => {
            (MemoryEventKind::NonAtomicRead { location }, vec![])
        }
        Instruction::NonAtomicWrite { location } => {
            (MemoryEventKind::NonAtomicWrite { location }, vec![])
        }
        Instruction::Loop => return None,
    })
}

/// `Instr.outcomes`: a weak compare-exchange on its expected value may also
/// fail spuriously.
fn outcomes(instruction: &Instruction, value: i64) -> &'static [bool] {
    match instruction {
        Instruction::CompareExchange {
            expected,
            strength: CompareExchangeStrength::Weak,
            ..
        } if *expected == value => &[false, true],
        _ => &[false],
    }
}

/// `QCore.ExecutionCoverage.paths`: every path whose reads take values in
/// the domain.
fn paths(
    domain: &dyn Fn(MemoryLocation) -> Vec<i64>,
    env: &[(Register, i64)],
    instructions: &[GuardedInstruction],
) -> Vec<Vec<MemoryEventKind>> {
    let Some((first, rest)) = instructions.split_first() else {
        return vec![vec![]];
    };
    if !holds(env, &first.guard) {
        return paths(domain, env, rest);
    }
    let mut result = Vec::new();
    let mut extend = |value: i64, spurious: bool| {
        if let Some((kind, update)) = step(&first.instruction, value, spurious) {
            let mut next = update;
            next.extend_from_slice(env);
            for mut tail in paths(domain, &next, rest) {
                tail.insert(0, kind.clone());
                result.push(tail);
            }
        }
    };
    match read_location(&first.instruction) {
        Some(location) => {
            for value in domain(location) {
                for &spurious in outcomes(&first.instruction, value) {
                    extend(value, spurious);
                }
            }
        }
        None => extend(0, false),
    }
    result
}

fn literal_at(instruction: &Instruction, location: MemoryLocation) -> Option<i64> {
    match *instruction {
        Instruction::Store {
            location: at,
            value,
            ..
        }
        | Instruction::Exchange {
            location: at,
            value,
            ..
        }
        | Instruction::CompareExchange {
            location: at,
            desired: value,
            ..
        } if at == location => Some(value),
        _ => None,
    }
}

/// `Frag.domain`: a location's initial value and every literal written to
/// it, without duplicates.
#[must_use]
pub fn value_domain(fragment: &ProgramFragment, location: MemoryLocation) -> Vec<i64> {
    let mut domain = BTreeSet::new();
    for (declared, value) in &fragment.atomic_locations {
        if *declared == location {
            domain.insert(*value);
        }
    }
    for thread in &fragment.threads {
        for instruction in thread {
            if let Some(value) = literal_at(&instruction.instruction, location) {
                domain.insert(value);
            }
        }
    }
    domain.into_iter().collect()
}

fn atomic_location(instruction: &Instruction) -> Option<MemoryLocation> {
    match instruction {
        Instruction::Load { location, .. }
        | Instruction::Store { location, .. }
        | Instruction::Exchange { location, .. }
        | Instruction::CompareExchange { location, .. } => Some(*location),
        _ => None,
    }
}

fn plain_location(instruction: &Instruction) -> Option<MemoryLocation> {
    match instruction {
        Instruction::NonAtomicRead { location } | Instruction::NonAtomicWrite { location } => {
            Some(*location)
        }
        _ => None,
    }
}

fn well_formed(fragment: &ProgramFragment) -> bool {
    let declared = fragment
        .atomic_locations
        .iter()
        .map(|(location, _)| *location)
        .collect::<BTreeSet<_>>();
    declared.len() == fragment.atomic_locations.len()
        && fragment.threads.iter().flatten().all(|instruction| {
            atomic_location(&instruction.instruction)
                .is_none_or(|location| declared.contains(&location))
                && plain_location(&instruction.instruction)
                    .is_none_or(|location| !declared.contains(&location))
        })
}

/// `Frag.eventBound`: the prelude and at most one event per instruction.
#[must_use]
pub fn event_bound(fragment: &ProgramFragment) -> usize {
    fragment.atomic_locations.len()
        + fragment.threads.len()
        + fragment.threads.iter().map(Vec::len).sum::<usize>()
}

/// `assemble`: the prelude on thread 0, then each path on its thread; the id
/// of each event is its position.
fn assemble(fragment: &ProgramFragment, chosen: &[&Vec<MemoryEventKind>]) -> Vec<MemoryEvent> {
    let mut tagged = Vec::new();
    for (location, value) in &fragment.atomic_locations {
        tagged.push((
            0,
            MemoryEventKind::AtomicInit {
                location: *location,
                value: *value,
            },
        ));
    }
    for index in 0..fragment.threads.len() {
        let child = MemoryThreadId::try_from(index + 1).unwrap_or(MemoryThreadId::MAX);
        tagged.push((0, MemoryEventKind::Spawn { child }));
    }
    for (index, path) in chosen.iter().enumerate() {
        let thread = MemoryThreadId::try_from(index + 1).unwrap_or(MemoryThreadId::MAX);
        tagged.extend(path.iter().map(|kind| (thread, kind.clone())));
    }
    tagged
        .into_iter()
        .enumerate()
        .map(|(id, (thread, kind))| MemoryEvent {
            id: MemoryEventId::try_from(id).unwrap_or(MemoryEventId::MAX),
            thread,
            kind,
        })
        .collect()
}

fn write_of(kind: &MemoryEventKind) -> Option<(MemoryLocation, i64)> {
    match *kind {
        MemoryEventKind::AtomicInit { location, value }
        | MemoryEventKind::AtomicStore {
            location, value, ..
        }
        | MemoryEventKind::AtomicReadModifyWrite {
            location,
            written_value: value,
            ..
        }
        | MemoryEventKind::AtomicCompareExchange {
            location,
            desired: value,
            succeeded: true,
            ..
        } => Some((location, value)),
        _ => None,
    }
}

fn read_of(kind: &MemoryEventKind) -> Option<(MemoryLocation, i64)> {
    match *kind {
        MemoryEventKind::AtomicLoad {
            location, value, ..
        }
        | MemoryEventKind::AtomicReadModifyWrite {
            location,
            read_value: value,
            ..
        }
        | MemoryEventKind::AtomicCompareExchange {
            location,
            observed: value,
            ..
        } => Some((location, value)),
        _ => None,
    }
}

fn sequentially_consistent_member(kind: &MemoryEventKind) -> bool {
    match kind {
        MemoryEventKind::AtomicStore { order, .. }
        | MemoryEventKind::AtomicLoad { order, .. }
        | MemoryEventKind::AtomicReadModifyWrite { order, .. }
        | MemoryEventKind::AtomicFence {
            order,
            scope: FenceScope::Thread,
        } => *order == MemoryOrder::SequentiallyConsistent,
        MemoryEventKind::AtomicCompareExchange {
            success,
            failure,
            succeeded,
            ..
        } => (if *succeeded { *success } else { *failure }) == MemoryOrder::SequentiallyConsistent,
        _ => false,
    }
}

/// The per-candidate choice space of one assembled path combination:
/// reads-from sources, per-location writes, S members.
struct Space {
    events: Vec<MemoryEvent>,
    reads: Vec<MemoryEventId>,
    sources: Vec<Vec<MemoryEventId>>,
    writes: Vec<Vec<MemoryEventId>>,
    sequential: Vec<MemoryEventId>,
}

impl Space {
    fn new(fragment: &ProgramFragment, events: Vec<MemoryEvent>) -> Self {
        let read_events = events
            .iter()
            .filter(|event| read_of(&event.kind).is_some())
            .collect::<Vec<_>>();
        let sources = read_events
            .iter()
            .map(|read| {
                events
                    .iter()
                    .filter(|write| {
                        write_of(&write.kind).is_some()
                            && write_of(&write.kind) == read_of(&read.kind)
                    })
                    .map(|write| write.id)
                    .collect()
            })
            .collect();
        let writes = fragment
            .atomic_locations
            .iter()
            .map(|(location, _)| {
                events
                    .iter()
                    .filter(|event| write_of(&event.kind).map(|(at, _)| at) == Some(*location))
                    .map(|event| event.id)
                    .collect()
            })
            .collect();
        let sequential = events
            .iter()
            .filter(|event| sequentially_consistent_member(&event.kind))
            .map(|event| event.id)
            .collect();
        let reads = read_events.iter().map(|event| event.id).collect();
        Self {
            events,
            reads,
            sources,
            writes,
            sequential,
        }
    }

    /// The number of candidates of this combination, saturating.
    fn count(&self) -> usize {
        let factorial = |n: usize| (1..=n).fold(1usize, usize::saturating_mul);
        self.sources
            .iter()
            .map(Vec::len)
            .chain(self.writes.iter().map(|writes| factorial(writes.len())))
            .chain([factorial(self.sequential.len())])
            .fold(1usize, usize::saturating_mul)
    }
}

fn permutations(items: &[MemoryEventId]) -> Vec<Vec<MemoryEventId>> {
    let Some((first, rest)) = items.split_first() else {
        return vec![vec![]];
    };
    let mut result = Vec::new();
    for tail in permutations(rest) {
        for position in 0..=tail.len() {
            let mut permutation = tail.clone();
            permutation.insert(position, *first);
            result.push(permutation);
        }
    }
    result
}

fn product<T: Clone>(lists: &[Vec<T>]) -> Vec<Vec<T>> {
    let mut result = vec![vec![]];
    for list in lists {
        let mut next = Vec::new();
        for prefix in &result {
            for item in list {
                let mut extended = prefix.clone();
                extended.push(item.clone());
                next.push(extended);
            }
        }
        result = next;
    }
    result
}

/// `encode`: the checker's graph of one candidate.
fn encode(
    space: &Space,
    sources: &[MemoryEventId],
    orders: &[Vec<MemoryEventId>],
    sequential: &[MemoryEventId],
) -> C11Execution {
    let mut execution = C11Execution {
        events: space.events.clone(),
        reads_from: sources
            .iter()
            .zip(&space.reads)
            .map(|(write, read)| RelationEdge::new(*write, *read))
            .collect(),
        modification_order: orders
            .iter()
            .flat_map(|order| {
                order
                    .windows(2)
                    .map(|pair| RelationEdge::new(pair[0], pair[1]))
            })
            .collect(),
        sequentially_consistent: sequential.to_vec(),
        ..C11Execution::default()
    };
    execution.synchronizes_with = derived_synchronizes_with(&execution);
    execution
}

/// `QCore.ExecutionCoverage.verdict`: race freedom over every candidate
/// execution of the fragment, within the bounds.
#[must_use]
pub fn check_program_executions(
    fragment: &ProgramFragment,
    bounds: CoverageBounds,
) -> ProgramVerdict {
    if fragment
        .threads
        .iter()
        .flatten()
        .any(|instruction| instruction.instruction == Instruction::Loop)
    {
        return ProgramVerdict::Unknown(CoverageUnknown::Loop);
    }
    if !well_formed(fragment) {
        return ProgramVerdict::Unknown(CoverageUnknown::Malformed);
    }
    if bounds.threads < fragment.threads.len() {
        return ProgramVerdict::Unknown(CoverageUnknown::ThreadBound {
            threads: fragment.threads.len(),
            bound: bounds.threads,
        });
    }
    let events = event_bound(fragment);
    if bounds.events < events {
        return ProgramVerdict::Unknown(CoverageUnknown::EventBound {
            events,
            bound: bounds.events,
        });
    }
    let value_domains = fragment
        .atomic_locations
        .iter()
        .map(|(location, _)| (*location, value_domain(fragment, *location)))
        .collect::<Vec<_>>();
    if let Some((location, domain)) = value_domains
        .iter()
        .find(|(_, domain)| bounds.values < domain.len())
    {
        return ProgramVerdict::Unknown(CoverageUnknown::ValueBound {
            location: *location,
            values: domain.len(),
            bound: bounds.values,
        });
    }
    let domain = |location: MemoryLocation| value_domain(fragment, location);
    let per_thread = fragment
        .threads
        .iter()
        .map(|thread| paths(&domain, &[], thread))
        .collect::<Vec<_>>();
    // Count first: a candidate bound is decided before any candidate is
    // checked, as `verdict` compares the candidate list's length.
    let mut combinations = vec![Vec::<&Vec<MemoryEventKind>>::new()];
    for thread_paths in &per_thread {
        let mut next = Vec::new();
        for prefix in &combinations {
            for path in thread_paths {
                let mut extended = prefix.clone();
                extended.push(path);
                next.push(extended);
            }
        }
        combinations = next;
    }
    let spaces = combinations
        .iter()
        .map(|chosen| Space::new(fragment, assemble(fragment, chosen)))
        .collect::<Vec<_>>();
    let total = spaces
        .iter()
        .map(Space::count)
        .fold(0usize, usize::saturating_add);
    if bounds.candidates < total {
        return ProgramVerdict::Unknown(CoverageUnknown::CandidateBound {
            bound: bounds.candidates,
        });
    }
    let mut consistent = 0;
    let mut witness = None;
    for space in &spaces {
        let source_choices = product(&space.sources);
        let order_choices = product(
            &space
                .writes
                .iter()
                .map(|writes| permutations(writes))
                .collect::<Vec<_>>(),
        );
        let sequential_choices = permutations(&space.sequential);
        for sources in &source_choices {
            for orders in &order_choices {
                for sequential in &sequential_choices {
                    let execution = encode(space, sources, orders, sequential);
                    match check_c11_execution(&execution) {
                        Ok(_) => consistent += 1,
                        Err(errors) => {
                            let races = errors
                                .iter()
                                .filter_map(|error| match error {
                                    WeakMemoryError::DataRace { first, second } => {
                                        Some((*first, *second))
                                    }
                                    _ => None,
                                })
                                .collect::<Vec<_>>();
                            if races.len() == errors.len() {
                                consistent += 1;
                                if witness.is_none() {
                                    witness = Some((execution, races[0]));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let evidence = CoverageEvidence {
        standard: EXECUTION_COVERAGE_STANDARD,
        bounds,
        threads: fragment.threads.len(),
        event_bound: events,
        value_domains,
        candidates: total,
        consistent,
    };
    match witness {
        Some((execution, (first, second))) => ProgramVerdict::Race {
            evidence,
            witness: Box::new(execution),
            first,
            second,
        },
        None if consistent > 0 => ProgramVerdict::RaceFree(evidence),
        None => ProgramVerdict::Unknown(CoverageUnknown::NoConsistentExecution),
    }
}

/// The store-buffering handshake both reclamation protocols rest on
/// (`QCore.ExecutionCoverage.sbRelaxed` / `sbFences`). The reader announces
/// itself (location 2: a hazard pointer, or an epoch announcement), then
/// re-reads the source (location 1, 1 = still linked) and accesses the
/// object (location 9) only if it is still linked. The reclaimer unlinks,
/// reads the announcement and frees (a write to 9) only if it saw none.
/// `ordering` is the order of every access; `fences` puts a seq_cst fence
/// between each thread's store and load.
#[must_use]
pub fn announce_validate_handshake(ordering: MemoryOrder, fences: bool) -> ProgramFragment {
    let fence = || {
        GuardedInstruction::always(Instruction::Fence {
            order: MemoryOrder::SequentiallyConsistent,
            scope: FenceScope::Thread,
        })
    };
    let load_order = match ordering {
        MemoryOrder::Release | MemoryOrder::AcquireRelease => MemoryOrder::Acquire,
        other => other,
    };
    let store_order = match ordering {
        MemoryOrder::Acquire | MemoryOrder::AcquireRelease => MemoryOrder::Release,
        other => other,
    };
    let thread = |announce: MemoryLocation,
                  value: i64,
                  check: MemoryLocation,
                  proceed: i64,
                  access: Instruction| {
        let mut instructions = vec![GuardedInstruction::always(Instruction::Store {
            location: announce,
            value,
            order: store_order,
        })];
        if fences {
            instructions.push(fence());
        }
        instructions.push(GuardedInstruction::always(Instruction::Load {
            location: check,
            order: load_order,
            register: 0,
        }));
        instructions.push(GuardedInstruction::when(0, proceed, access));
        instructions
    };
    ProgramFragment {
        atomic_locations: vec![(1, 1), (2, 0)],
        threads: vec![
            thread(2, 1, 1, 1, Instruction::NonAtomicRead { location: 9 }),
            thread(1, 0, 2, 0, Instruction::NonAtomicWrite { location: 9 }),
        ],
    }
}

#[cfg(test)]
#[path = "../tests/unit/execution_coverage.rs"]
mod tests;
