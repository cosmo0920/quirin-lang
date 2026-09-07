//! `CheckedMutexInvariantV1` — a fixed-shape checked mutex-invariant profile.
//!
//! This module is a *sibling* of [`crate::concurrency`], not a replacement.
//! `check_concurrent` and its `TrustedExplicitTrace` profile are unchanged; the
//! premises `GuardFootprintAuthority` and `SynchronizedCustodyEdges` remain in
//! force there. This profile discharges those two premises inside its own
//! bounded model by representing what the older profile only declares:
//!
//! * every mutex generation declares a **fixed** protected footprint at
//!   creation, consuming the creator's `FULL_ACCESS` share over it, so a
//!   protected footprint has exactly one protector and no fallback authority;
//! * access authority is keyed to a fresh **guard instance**, not to a mutex
//!   name, so two critical sections over one mutex generation are two distinct
//!   authorities and a stale refinement fact cannot cross a release;
//! * custody transfer carries a `SyncWitness` that the checker **derives and
//!   compares** against its own happens-before history, in the style of
//!   [`crate::weak_memory`]; no caller-supplied `bool` injects a clock edge.
//!
//! What stays a premise is named on the certificate: the declared intervals
//! correspond to real C layout (`DeclaredFootprintShape`), and a declared
//! invariant predicate is asserted rather than proved
//! (`InvariantPredicateDeclared`). Source-to-trace refinement, deadlock
//! freedom, lock-order analysis, condition variables, reader/writer locks and
//! try/timed acquisition are outside this profile and are refused, never
//! approximated.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use crate::concurrency::{
    AccessFootprint, AccessKind, ConcurrencyAssumption, ConcurrencyProfile, FULL_ACCESS,
    SharedResourceId, TaskId,
};

/// One mutex lifetime. Generation-indexed exactly like [`SharedResourceId`]:
/// never a reusable address, handle or source variable. A destroyed id is dead
/// forever.
pub type MutexId = u32;
/// One critical section. Minted fresh by each successful `Lock` and never
/// reused, including across two locks of one mutex generation.
pub type GuardId = u32;
/// A declared invariant predicate. The predicate itself is a premise.
pub type InvariantId = u32;
/// A borrow taken through a guard. Dies at that guard's unlock.
pub type BorrowId = u32;
/// A refinement fact asserted about protected memory, scoped to one guard.
pub type FactId = u32;

/// Pointwise share authority granted at trace entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShareGrant {
    pub task: TaskId,
    pub footprint: AccessFootprint,
    pub share: u16,
}

/// The premise the checker cannot discharge: that a declared predicate holds of
/// the bytes. Recorded and named, never inferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvariantWitness {
    /// Restored by a declared contract clause. Adds
    /// [`ConcurrencyAssumption::InvariantPredicateDeclared`].
    Declared { invariant: InvariantId },
    /// No write occurred while the invariant was open, so it is unchanged.
    /// This one is checked, not declared, and adds no assumption.
    NoWriteOccurred,
}

/// Every variant names an edge the checker must find in its own derived
/// happens-before history. No variant is believed on the caller's word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyncWitness {
    /// `from` spawned `to`, and `child` names `to`.
    SpawnEdge { child: TaskId },
    /// `to` joined `child`, and `child` is the sending task.
    JoinEdge { child: TaskId },
    /// A release of `mutex` by `from` was observed by `to`'s acquisition, and
    /// `mutex` protects the transferred resource.
    MutexHandoff { mutex: MutexId },
}

/// Synchronization shapes this profile refuses rather than approximates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnsupportedSynchronization {
    ConditionVariable,
    TryLock,
    TimedLock,
    ReaderWriterLock,
    UpgradeableGuard,
    NestedProtectingMutex,
    RecursiveAcquisition,
    LockRankDeclaration,
    CompletionCapsuleComposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutexInvariantEvent {
    // ---- task structure ----
    Spawn {
        parent: TaskId,
        child: TaskId,
    },
    Join {
        parent: TaskId,
        child: TaskId,
    },

    // ---- mutex lifecycle ----
    /// Fixes the protected shape and parks the creator's `FULL_ACCESS` share
    /// over every declared interval.
    CreateMutex {
        task: TaskId,
        mutex: MutexId,
        protects: Vec<AccessFootprint>,
        invariant: InvariantId,
    },
    DestroyMutex {
        task: TaskId,
        mutex: MutexId,
    },
    /// A task that has announced an acquisition it has not yet performed. A
    /// recorded waiter blocks destruction (`§1.7` rule 3).
    RegisterWaiter {
        task: TaskId,
        mutex: MutexId,
    },

    // ---- critical sections ----
    /// Mints `guard`. The trace supplies the id; the checker requires it to be
    /// globally fresh, the `GuardId` analogue of the unique-lifetime-id rule.
    Lock {
        task: TaskId,
        mutex: MutexId,
        guard: GuardId,
    },
    Unlock {
        task: TaskId,
        guard: GuardId,
    },
    /// Typed-error exit from a critical section, distinguished from `Unlock`
    /// only so the diagnostic can name the error path.
    ErrorExit {
        task: TaskId,
        guard: GuardId,
        exit: u32,
    },

    // ---- invariant phases ----
    OpenInvariant {
        task: TaskId,
        guard: GuardId,
    },
    CloseInvariant {
        task: TaskId,
        guard: GuardId,
        witness: InvariantWitness,
    },

    // ---- access ----
    Access {
        task: TaskId,
        footprint: AccessFootprint,
        kind: AccessKind,
        /// `None` is legal only for a footprint no live mutex protects. There
        /// is no fallback: a protected footprint with `None` rejects.
        guard: Option<GuardId>,
    },

    // ---- guard-derived borrows ----
    BorrowUnderGuard {
        task: TaskId,
        guard: GuardId,
        footprint: AccessFootprint,
        borrow: BorrowId,
    },
    EndBorrow {
        task: TaskId,
        borrow: BorrowId,
    },

    // ---- guard-scoped refinement facts ----
    AssertFact {
        task: TaskId,
        guard: GuardId,
        footprint: AccessFootprint,
        fact: FactId,
    },
    UseFact {
        task: TaskId,
        fact: FactId,
    },

    // ---- custody ----
    /// No `synchronized: bool`. The witness is verified against the checker's
    /// own derived happens-before relation.
    TransferCustody {
        from: TaskId,
        to: TaskId,
        resource: SharedResourceId,
        via: SyncWitness,
    },
    Destroy {
        task: TaskId,
        resource: SharedResourceId,
    },

    /// A proposal to change a mutex's protected shape after creation. Always
    /// refused: the shape is fixed. Represented so the refusal is testable
    /// rather than the construct being silently inexpressible.
    ExtendProtection {
        task: TaskId,
        mutex: MutexId,
        footprint: AccessFootprint,
    },
    /// A synchronization construct outside this profile. Always refused.
    UnsupportedConstruct {
        task: TaskId,
        construct: UnsupportedSynchronization,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MutexInvariantProgram {
    /// Initial state of a *mutable* pointwise share ledger, not a static table
    /// consulted at every access.
    pub initial_shares: Vec<ShareGrant>,
    pub initial_custody: BTreeMap<SharedResourceId, TaskId>,
    pub events: Vec<MutexInvariantEvent>,
}

/// Which phase a guard's invariant is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvariantPhase {
    /// No guard is held on the mutex.
    Absent,
    Closed,
    Open,
}

/// One admitted invariant-phase transition, named on the certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InvariantTransition {
    pub event: usize,
    pub mutex: MutexId,
    pub guard: GuardId,
    pub from: InvariantPhase,
    pub to: InvariantPhase,
    pub witness: Option<InvariantWitness>,
}

/// One synchronization edge the checker *derived* and matched against a
/// supplied witness. Recorded so a reader can see that none was believed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DerivedSyncEdge {
    pub event: usize,
    pub from: TaskId,
    pub to: TaskId,
    pub resource: SharedResourceId,
    pub witness: SyncWitness,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MutexInvariantDiagnostic {
    // ---- the fourteen named refusals of this profile (Q610-Q623) ----
    /// The guard's mutex does not protect the accessed footprint. This is the
    /// repair: `check_concurrent` has no footprint/mutex relation to test.
    UnprotectedFootprint {
        event: usize,
        guard: GuardId,
    },
    UnguardedProtectedAccess {
        event: usize,
        mutex: MutexId,
    },
    WrongGuardAffinity {
        event: usize,
        guard: GuardId,
        task: TaskId,
    },
    InvariantOpenAtUnlock {
        event: usize,
        guard: GuardId,
    },
    InvariantOpenAtErrorExit {
        event: usize,
        guard: GuardId,
    },
    GuardBorrowEscape {
        event: usize,
        guard: GuardId,
        borrow: BorrowId,
    },
    StaleProtectedFact {
        event: usize,
        fact: FactId,
    },
    FootprintAlreadyProtected {
        event: usize,
        mutex: MutexId,
    },
    DynamicProtectedShape {
        event: usize,
        mutex: MutexId,
    },
    NonquiescentMutexDestruction {
        event: usize,
        mutex: MutexId,
    },
    ForgedSynchronizationWitness {
        event: usize,
        resource: SharedResourceId,
    },
    MutexAuthorityNotConserved {
        event: Option<usize>,
        resource: SharedResourceId,
    },
    UnsupportedSynchronizationConstruct {
        event: usize,
        construct: UnsupportedSynchronization,
    },
    DeadMutexGeneration {
        event: usize,
        mutex: MutexId,
    },

    // ---- family members of the codes above ----
    /// Affinity family (`Q612`): a guard minted by an already-closed section.
    DeadGuard {
        event: usize,
        guard: GuardId,
    },
    /// Invariant-phase family (`Q613`).
    DoubleOpenInvariant {
        event: usize,
        guard: GuardId,
    },
    /// Invariant-phase family (`Q613`): a write with the invariant closed.
    WriteWithClosedInvariant {
        event: usize,
        guard: GuardId,
    },
    /// Invariant-phase family (`Q613`).
    InvalidInvariantWitness {
        event: usize,
        guard: GuardId,
    },

    // ---- structural refusals carried over from the bounded model (Q600) ----
    InvalidFootprint {
        event: Option<usize>,
    },
    UnknownTask {
        event: usize,
        task: TaskId,
    },
    TerminalTask {
        event: usize,
        task: TaskId,
    },
    DuplicateTask {
        event: usize,
        child: TaskId,
    },
    UnknownJoin {
        event: usize,
        child: TaskId,
    },
    NonquiescentJoin {
        event: usize,
        child: TaskId,
    },
    JoinWhileGuardHeld {
        event: usize,
        task: TaskId,
    },
    DeadResource {
        event: usize,
        resource: SharedResourceId,
    },
    UnauthorizedAccess {
        event: usize,
        task: TaskId,
    },
    WrongCustodian {
        event: usize,
        resource: SharedResourceId,
    },
    NonquiescentDestruction {
        event: usize,
        resource: SharedResourceId,
    },
    DataRace {
        first: usize,
        second: usize,
    },
    DuplicateLockCapability {
        event: usize,
        mutex: MutexId,
    },
    UnknownMutex {
        event: usize,
        mutex: MutexId,
    },
    /// A guard, borrow or fact id that is not globally fresh, or a reused mutex
    /// id. Identity denotes a lifetime and is never recycled.
    IdentityNotFresh {
        event: usize,
        kind: IdentityKind,
        id: u32,
    },
    UnknownBorrow {
        event: usize,
        borrow: BorrowId,
    },
    UnknownFact {
        event: usize,
        fact: FactId,
    },
    OutstandingTask {
        child: TaskId,
    },
    OutstandingGuard {
        guard: GuardId,
    },
    OutstandingMutex {
        mutex: MutexId,
    },
    OutstandingWaiter {
        mutex: MutexId,
        task: TaskId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IdentityKind {
    Mutex,
    Guard,
    Borrow,
    Fact,
}

impl MutexInvariantDiagnostic {
    /// The source-level diagnostic code this refusal surfaces as. The fourteen
    /// named refusals of this profile occupy `Q610`-`Q623`; `DeadGuard` joins
    /// the affinity family at `Q612` and the remaining invariant-phase
    /// refusals join `Q613`. Structural refusals carried over from the bounded
    /// concurrency model keep the existing concurrency pass code `Q600`; they
    /// are not new obligations of this profile.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnprotectedFootprint { .. } => "Q610",
            Self::UnguardedProtectedAccess { .. } => "Q611",
            Self::WrongGuardAffinity { .. } | Self::DeadGuard { .. } => "Q612",
            Self::InvariantOpenAtUnlock { .. }
            | Self::DoubleOpenInvariant { .. }
            | Self::WriteWithClosedInvariant { .. }
            | Self::InvalidInvariantWitness { .. } => "Q613",
            Self::InvariantOpenAtErrorExit { .. } => "Q614",
            Self::GuardBorrowEscape { .. } => "Q615",
            Self::StaleProtectedFact { .. } => "Q616",
            Self::FootprintAlreadyProtected { .. } => "Q617",
            Self::DynamicProtectedShape { .. } => "Q618",
            Self::NonquiescentMutexDestruction { .. } => "Q619",
            Self::ForgedSynchronizationWitness { .. } => "Q620",
            Self::MutexAuthorityNotConserved { .. } => "Q621",
            Self::UnsupportedSynchronizationConstruct { .. } => "Q622",
            Self::DeadMutexGeneration { .. } => "Q623",
            _ => "Q600",
        }
    }
}

impl fmt::Display for MutexInvariantDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {self:?}", self.code())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutexInvariantCertificate {
    /// Always [`ConcurrencyProfile::CheckedMutexInvariantV1`].
    pub profile: ConcurrencyProfile,
    /// Never contains `GuardFootprintAuthority` or `SynchronizedCustodyEdges`;
    /// their absence is this profile's discharge claim.
    pub assumptions: BTreeSet<ConcurrencyAssumption>,
    pub checked_events: usize,
    /// Number of points at which pointwise share conservation was verified:
    /// once before the first event and once after every event.
    pub conservation_checkpoints: usize,
    pub completed_tasks: Vec<TaskId>,
    pub destroyed_resources: Vec<SharedResourceId>,
    pub destroyed_mutexes: Vec<MutexId>,
    /// Which mutex generation protected which exact footprints.
    pub protected_shape: BTreeMap<MutexId, Vec<AccessFootprint>>,
    pub admitted_invariant_transitions: Vec<InvariantTransition>,
    pub derived_sync_edges: Vec<DerivedSyncEdge>,
}

impl MutexInvariantCertificate {
    /// Assumption names as plain strings, so a free-form assumption map cannot
    /// smuggle a trusted-model premise past the typed set.
    pub fn assumption_names(&self) -> BTreeSet<&'static str> {
        self.assumptions
            .iter()
            .map(|assumption| match assumption {
                ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds => {
                    "InitialAuthorityAndUniqueLifetimeIds"
                }
                ConcurrencyAssumption::GuardFootprintAuthority => "GuardFootprintAuthority",
                ConcurrencyAssumption::SynchronizedCustodyEdges => "SynchronizedCustodyEdges",
                ConcurrencyAssumption::DeclaredFootprintShape => "DeclaredFootprintShape",
                ConcurrencyAssumption::InvariantPredicateDeclared => "InvariantPredicateDeclared",
            })
            .collect()
    }
}

/// Why an independent recheck refused a supplied certificate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CertificateRecheckError {
    /// Re-running the checker on the same program produced diagnostics.
    ProgramRejected(Vec<MutexInvariantDiagnostic>),
    /// A field of the supplied certificate is not what re-derivation produced.
    FieldMismatch { field: &'static str },
    /// The supplied certificate names a premise this profile discharges.
    ForbiddenAssumption { assumption: ConcurrencyAssumption },
}

// ---------------------------------------------------------------------------
// Vector clocks. Local to this module: `concurrency`'s helpers are private and
// this profile must not depend on that checker's state representation.
// ---------------------------------------------------------------------------

type VectorClock = BTreeMap<TaskId, u64>;

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

const fn writes(kind: AccessKind) -> bool {
    matches!(kind, AccessKind::Write | AccessKind::AtomicWrite)
}

const fn atomic(kind: AccessKind) -> bool {
    matches!(kind, AccessKind::AtomicRead | AccessKind::AtomicWrite)
}

const fn conflicting(left: AccessKind, right: AccessKind) -> bool {
    (writes(left) || writes(right)) && !(atomic(left) && atomic(right))
}

// ---------------------------------------------------------------------------
// Pointwise share ledger over exact atoms.
// ---------------------------------------------------------------------------

/// One indivisible span. Every interval named anywhere in the program is an
/// exact union of atoms, so the ledger is pointwise without being unbounded.
type Atom = (SharedResourceId, usize);

#[derive(Debug, Default)]
struct ShareLedger {
    /// Sorted boundary coordinates per resource; atom `i` is
    /// `[bounds[i], bounds[i + 1])`.
    bounds: BTreeMap<SharedResourceId, Vec<u32>>,
    /// Authority a task holds at an atom.
    task_shares: BTreeMap<Atom, BTreeMap<TaskId, u32>>,
    /// The mutex generation parking `FULL_ACCESS` at an atom.
    parked: BTreeMap<Atom, MutexId>,
    /// Total authority present at an atom when the trace began. Conservation is
    /// invariance of this total, not a fixed constant imposed on every point.
    initial_total: BTreeMap<Atom, u32>,
    /// Atoms of destroyed resources; retired from the conservation check.
    retired: BTreeSet<Atom>,
}

impl ShareLedger {
    fn build(program: &MutexInvariantProgram) -> Self {
        let mut coordinates = BTreeMap::<SharedResourceId, BTreeSet<u32>>::new();
        let mut record = |footprint: AccessFootprint| {
            if footprint.is_valid() {
                let entry = coordinates.entry(footprint.resource).or_default();
                entry.insert(footprint.start);
                entry.insert(footprint.end);
            }
        };
        for grant in &program.initial_shares {
            record(grant.footprint);
        }
        for event in &program.events {
            match event {
                MutexInvariantEvent::CreateMutex { protects, .. } => {
                    for footprint in protects {
                        record(*footprint);
                    }
                }
                MutexInvariantEvent::Access { footprint, .. }
                | MutexInvariantEvent::BorrowUnderGuard { footprint, .. }
                | MutexInvariantEvent::AssertFact { footprint, .. }
                | MutexInvariantEvent::ExtendProtection { footprint, .. } => record(*footprint),
                _ => {}
            }
        }

        let bounds = coordinates
            .into_iter()
            .map(|(resource, points)| (resource, points.into_iter().collect::<Vec<_>>()))
            .collect::<BTreeMap<_, _>>();
        let mut ledger = Self {
            bounds,
            ..Self::default()
        };
        for grant in &program.initial_shares {
            for atom in ledger.atoms_of(grant.footprint) {
                *ledger
                    .task_shares
                    .entry(atom)
                    .or_default()
                    .entry(grant.task)
                    .or_default() += u32::from(grant.share);
            }
        }
        ledger.initial_total = ledger
            .task_shares
            .iter()
            .map(|(atom, shares)| (*atom, shares.values().sum::<u32>()))
            .collect();
        ledger
    }

    /// Every atom fully inside `footprint`. Because the boundary set contains
    /// every interval endpoint mentioned in the program, no atom is ever
    /// partially covered.
    fn atoms_of(&self, footprint: AccessFootprint) -> Vec<Atom> {
        let Some(points) = self.bounds.get(&footprint.resource) else {
            return Vec::new();
        };
        (0..points.len().saturating_sub(1))
            .filter(|index| points[*index] >= footprint.start && points[index + 1] <= footprint.end)
            .map(|index| (footprint.resource, index))
            .collect()
    }

    fn share_of(&self, task: TaskId, atom: Atom) -> u32 {
        self.task_shares
            .get(&atom)
            .and_then(|shares| shares.get(&task))
            .copied()
            .unwrap_or(0)
    }

    /// The task's authority over the whole footprint, i.e. its minimum over the
    /// footprint's atoms. An unmentioned footprint has no authority.
    fn authority(&self, task: TaskId, footprint: AccessFootprint) -> u32 {
        let atoms = self.atoms_of(footprint);
        if atoms.is_empty() {
            return 0;
        }
        atoms
            .into_iter()
            .map(|atom| self.share_of(task, atom))
            .min()
            .unwrap_or(0)
    }

    fn park(&mut self, task: TaskId, mutex: MutexId, footprint: AccessFootprint) {
        for atom in self.atoms_of(footprint) {
            if let Some(shares) = self.task_shares.get_mut(&atom) {
                shares.remove(&task);
            }
            self.parked.insert(atom, mutex);
        }
    }

    fn unpark(&mut self, task: TaskId, mutex: MutexId) {
        let atoms = self
            .parked
            .iter()
            .filter(|(_, parked)| **parked == mutex)
            .map(|(atom, _)| *atom)
            .collect::<Vec<_>>();
        for atom in atoms {
            self.parked.remove(&atom);
            if !self.retired.contains(&atom) {
                *self
                    .task_shares
                    .entry(atom)
                    .or_default()
                    .entry(task)
                    .or_default() += u32::from(FULL_ACCESS);
            }
        }
    }

    fn retire(&mut self, resource: SharedResourceId) {
        let atoms = self
            .initial_total
            .keys()
            .filter(|atom| atom.0 == resource)
            .copied()
            .collect::<Vec<_>>();
        for atom in atoms {
            self.retired.insert(atom);
            self.task_shares.remove(&atom);
        }
    }

    /// Resources whose live atoms violate `sum(task shares) + parked ==
    /// initial total`.
    fn unconserved(&self) -> BTreeSet<SharedResourceId> {
        let mut broken = BTreeSet::new();
        for (atom, total) in &self.initial_total {
            if self.retired.contains(atom) {
                continue;
            }
            let held = self
                .task_shares
                .get(atom)
                .map(|shares| shares.values().sum::<u32>())
                .unwrap_or(0);
            let parked = if self.parked.contains_key(atom) {
                u32::from(FULL_ACCESS)
            } else {
                0
            };
            if held + parked != *total {
                broken.insert(atom.0);
            }
        }
        broken
    }

    /// Entry-state check: no atom may start with more than `FULL_ACCESS`.
    fn overcommitted(&self) -> BTreeSet<SharedResourceId> {
        self.initial_total
            .iter()
            .filter(|(_, total)| **total > u32::from(FULL_ACCESS))
            .map(|(atom, _)| atom.0)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Checker state.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct MutexState {
    protects: Vec<AccessFootprint>,
    invariant: InvariantId,
    creator: TaskId,
    holder: Option<GuardId>,
    waiters: BTreeSet<TaskId>,
    /// Clock published by the most recent release of this generation.
    release_clock: Option<VectorClock>,
    /// Every release of this generation, with the clock it published. A
    /// handoff witness is matched against this history, not against the most
    /// recent release alone.
    release_history: Vec<(TaskId, VectorClock)>,
}

#[derive(Clone, Debug)]
struct GuardState {
    mutex: MutexId,
    holder: TaskId,
    phase: InvariantPhase,
    borrows: BTreeSet<BorrowId>,
    facts: BTreeMap<FactId, AccessFootprint>,
    writes_while_open: Vec<AccessFootprint>,
}

#[derive(Clone, Debug)]
struct RecordedAccess {
    event: usize,
    task: TaskId,
    footprint: AccessFootprint,
    kind: AccessKind,
    clock: VectorClock,
}

/// Check a trace under `CheckedMutexInvariantV1`.
///
/// Fail-closed: every unsupported construct, unmatched witness and unresolved
/// authority question is a refusal. A refused event never mutates liveness, so
/// a rejected destruction leaves its generation live.
pub fn check_mutex_invariant_program(
    program: &MutexInvariantProgram,
) -> Result<MutexInvariantCertificate, Vec<MutexInvariantDiagnostic>> {
    let mut diagnostics = BTreeSet::<MutexInvariantDiagnostic>::new();

    for grant in &program.initial_shares {
        if !grant.footprint.is_valid() {
            diagnostics.insert(MutexInvariantDiagnostic::InvalidFootprint { event: None });
        }
    }
    let mut ledger = ShareLedger::build(program);
    for resource in ledger.overcommitted() {
        diagnostics.insert(MutexInvariantDiagnostic::MutexAuthorityNotConserved {
            event: None,
            resource,
        });
    }
    // Checkpoint 0: the entry state.
    let mut conservation_checkpoints = 1usize;

    let mut clocks = BTreeMap::from([(0, VectorClock::from([(0, 0)]))]);
    let mut active = BTreeSet::from([0]);
    let mut task_parents = BTreeMap::<TaskId, TaskId>::new();
    let mut joined = BTreeSet::<TaskId>::new();
    let mut joiner_of = BTreeMap::<TaskId, TaskId>::new();

    let mut mutexes = BTreeMap::<MutexId, MutexState>::new();
    let mut destroyed_mutexes = BTreeSet::<MutexId>::new();
    let mut minted_mutexes = BTreeSet::<MutexId>::new();
    let mut guards = BTreeMap::<GuardId, GuardState>::new();
    let mut minted_guards = BTreeSet::<GuardId>::new();
    let mut minted_borrows = BTreeSet::<BorrowId>::new();
    let mut minted_facts = BTreeSet::<FactId>::new();
    let mut live_borrows = BTreeMap::<BorrowId, GuardId>::new();
    let mut live_facts = BTreeMap::<FactId, GuardId>::new();

    let mut custody = program.initial_custody.clone();
    let mut destroyed_resources = BTreeSet::<SharedResourceId>::new();
    let mut accesses = Vec::<RecordedAccess>::new();

    let mut assumptions =
        BTreeSet::from([ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds]);
    let mut protected_shape = BTreeMap::<MutexId, Vec<AccessFootprint>>::new();
    let mut transitions = Vec::<InvariantTransition>::new();
    let mut sync_edges = Vec::<DerivedSyncEdge>::new();

    for (index, event) in program.events.iter().enumerate() {
        'event: {
            match event {
                MutexInvariantEvent::Spawn { parent, child } => {
                    if !require_active(&clocks, &active, index, *parent, &mut diagnostics) {
                        break 'event;
                    }
                    if clocks.contains_key(child) || *child == 0 {
                        diagnostics.insert(MutexInvariantDiagnostic::DuplicateTask {
                            event: index,
                            child: *child,
                        });
                        break 'event;
                    }
                    tick(&mut clocks, *parent);
                    let parent_clock = clocks[parent].clone();
                    clocks.insert(*child, parent_clock);
                    active.insert(*child);
                    task_parents.insert(*child, *parent);
                }
                MutexInvariantEvent::Join { parent, child } => {
                    if !require_active(&clocks, &active, index, *parent, &mut diagnostics) {
                        break 'event;
                    }
                    if task_parents.get(child) != Some(parent) || joined.contains(child) {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownJoin {
                            event: index,
                            child: *child,
                        });
                        break 'event;
                    }
                    if guards.values().any(|guard| guard.holder == *parent) {
                        diagnostics.insert(MutexInvariantDiagnostic::JoinWhileGuardHeld {
                            event: index,
                            task: *parent,
                        });
                    }
                    let child_busy = guards.values().any(|guard| guard.holder == *child)
                        || task_parents.iter().any(|(descendant, owner)| {
                            owner == child && !joined.contains(descendant)
                        });
                    if child_busy {
                        diagnostics.insert(MutexInvariantDiagnostic::NonquiescentJoin {
                            event: index,
                            child: *child,
                        });
                        break 'event;
                    }
                    joined.insert(*child);
                    joiner_of.insert(*child, *parent);
                    active.remove(child);
                    let child_clock = clocks.get(child).cloned().unwrap_or_default();
                    merge_clock(
                        clocks.get_mut(parent).expect("parent was checked"),
                        &child_clock,
                    );
                    tick(&mut clocks, *parent);
                }
                MutexInvariantEvent::CreateMutex {
                    task,
                    mutex,
                    protects,
                    invariant,
                } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    if destroyed_mutexes.contains(mutex) {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadMutexGeneration {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    }
                    if minted_mutexes.contains(mutex) {
                        diagnostics.insert(MutexInvariantDiagnostic::IdentityNotFresh {
                            event: index,
                            kind: IdentityKind::Mutex,
                            id: *mutex,
                        });
                        break 'event;
                    }
                    let malformed = protects.is_empty()
                        || protects.iter().any(|footprint| !footprint.is_valid())
                        || protects.iter().enumerate().any(|(position, left)| {
                            protects
                                .iter()
                                .skip(position + 1)
                                .any(|right| left.overlaps(*right))
                        });
                    if malformed {
                        diagnostics.insert(MutexInvariantDiagnostic::InvalidFootprint {
                            event: Some(index),
                        });
                        break 'event;
                    }
                    if protects
                        .iter()
                        .any(|footprint| destroyed_resources.contains(&footprint.resource))
                    {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadResource {
                            event: index,
                            resource: protects[0].resource,
                        });
                        break 'event;
                    }
                    // A footprint has at most one protector: creation consumes the
                    // share, so an already-parked interval cannot be re-parked.
                    if protects.iter().any(|footprint| {
                        ledger
                            .atoms_of(*footprint)
                            .iter()
                            .any(|atom| ledger.parked.contains_key(atom))
                    }) {
                        diagnostics.insert(MutexInvariantDiagnostic::FootprintAlreadyProtected {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    }
                    // Authority is moved, not copied: parking is all-or-nothing.
                    if protects.iter().any(|footprint| {
                        ledger.authority(*task, *footprint) != u32::from(FULL_ACCESS)
                    }) {
                        diagnostics.insert(MutexInvariantDiagnostic::MutexAuthorityNotConserved {
                            event: Some(index),
                            resource: protects[0].resource,
                        });
                        break 'event;
                    }
                    for footprint in protects {
                        ledger.park(*task, *mutex, *footprint);
                    }
                    mutexes.insert(
                        *mutex,
                        MutexState {
                            protects: protects.clone(),
                            invariant: *invariant,
                            creator: *task,
                            holder: None,
                            waiters: BTreeSet::new(),
                            release_clock: None,
                            release_history: Vec::new(),
                        },
                    );
                    minted_mutexes.insert(*mutex);
                    protected_shape.insert(*mutex, protects.clone());
                    assumptions.insert(ConcurrencyAssumption::DeclaredFootprintShape);
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::DestroyMutex { task, mutex } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    // Liveness before authority: a repeat destruction reports a dead
                    // generation, never an ownership complaint.
                    if destroyed_mutexes.contains(mutex) {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadMutexGeneration {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    }
                    let Some(state) = mutexes.get(mutex) else {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownMutex {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    };
                    let outstanding_user = state.holder.is_some()
                        || !state.waiters.is_empty()
                        || guards.values().any(|guard| guard.mutex == *mutex)
                        || live_borrows.values().any(|guard| {
                            guards.get(guard).is_some_and(|state| state.mutex == *mutex)
                        })
                        || live_facts.values().any(|guard| {
                            guards.get(guard).is_some_and(|state| state.mutex == *mutex)
                        });
                    let unordered_access = state.protects.iter().any(|protected| {
                        accesses.iter().any(|access| {
                            access.footprint.overlaps(*protected)
                                && !happens_before(&access.clock, &clocks[task])
                        })
                    });
                    if outstanding_user || unordered_access {
                        diagnostics.insert(
                            MutexInvariantDiagnostic::NonquiescentMutexDestruction {
                                event: index,
                                mutex: *mutex,
                            },
                        );
                        break 'event;
                    }
                    // The parked share returns to the task that parked it. Anyone
                    // else would receive authority nobody gave up.
                    if state.creator != *task {
                        diagnostics.insert(MutexInvariantDiagnostic::MutexAuthorityNotConserved {
                            event: Some(index),
                            resource: state.protects[0].resource,
                        });
                        break 'event;
                    }
                    ledger.unpark(*task, *mutex);
                    mutexes.remove(mutex);
                    destroyed_mutexes.insert(*mutex);
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::RegisterWaiter { task, mutex } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    if destroyed_mutexes.contains(mutex) {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadMutexGeneration {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    }
                    let Some(state) = mutexes.get_mut(mutex) else {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownMutex {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    };
                    state.waiters.insert(*task);
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::Lock { task, mutex, guard } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    if destroyed_mutexes.contains(mutex) {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadMutexGeneration {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    }
                    let Some(state) = mutexes.get_mut(mutex) else {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownMutex {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    };
                    if state.holder.is_some() {
                        diagnostics.insert(MutexInvariantDiagnostic::DuplicateLockCapability {
                            event: index,
                            mutex: *mutex,
                        });
                        break 'event;
                    }
                    if minted_guards.contains(guard) {
                        diagnostics.insert(MutexInvariantDiagnostic::IdentityNotFresh {
                            event: index,
                            kind: IdentityKind::Guard,
                            id: *guard,
                        });
                        break 'event;
                    }
                    if let Some(release) = state.release_clock.clone() {
                        merge_clock(clocks.get_mut(task).expect("task was checked"), &release);
                    }
                    state.holder = Some(*guard);
                    state.waiters.remove(task);
                    minted_guards.insert(*guard);
                    guards.insert(
                        *guard,
                        GuardState {
                            mutex: *mutex,
                            holder: *task,
                            phase: InvariantPhase::Closed,
                            borrows: BTreeSet::new(),
                            facts: BTreeMap::new(),
                            writes_while_open: Vec::new(),
                        },
                    );
                    transitions.push(InvariantTransition {
                        event: index,
                        mutex: *mutex,
                        guard: *guard,
                        from: InvariantPhase::Absent,
                        to: InvariantPhase::Closed,
                        witness: None,
                    });
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::Unlock { task, guard }
                | MutexInvariantEvent::ErrorExit { task, guard, .. } => {
                    let error_path = matches!(event, MutexInvariantEvent::ErrorExit { .. });
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    let Some(state) = guards.get(guard) else {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadGuard {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    };
                    if state.holder != *task {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongGuardAffinity {
                            event: index,
                            guard: *guard,
                            task: *task,
                        });
                        break 'event;
                    }
                    let mut refused = false;
                    if state.phase == InvariantPhase::Open {
                        diagnostics.insert(if error_path {
                            MutexInvariantDiagnostic::InvariantOpenAtErrorExit {
                                event: index,
                                guard: *guard,
                            }
                        } else {
                            MutexInvariantDiagnostic::InvariantOpenAtUnlock {
                                event: index,
                                guard: *guard,
                            }
                        });
                        refused = true;
                    }
                    if let Some(borrow) = state.borrows.iter().next().copied() {
                        diagnostics.insert(MutexInvariantDiagnostic::GuardBorrowEscape {
                            event: index,
                            guard: *guard,
                            borrow,
                        });
                        refused = true;
                    }
                    if refused {
                        break 'event;
                    }
                    let mutex = state.mutex;
                    // Every guard-scoped fact dies here, unconditionally.
                    for fact in state.facts.keys() {
                        live_facts.remove(fact);
                    }
                    guards.remove(guard);
                    tick(&mut clocks, *task);
                    if let Some(state) = mutexes.get_mut(&mutex) {
                        state.holder = None;
                        state.release_clock = Some(clocks[task].clone());
                        state.release_history.push((*task, clocks[task].clone()));
                    }
                    transitions.push(InvariantTransition {
                        event: index,
                        mutex,
                        guard: *guard,
                        from: InvariantPhase::Closed,
                        to: InvariantPhase::Absent,
                        witness: None,
                    });
                }
                MutexInvariantEvent::OpenInvariant { task, guard } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    let Some(state) = guards.get_mut(guard) else {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadGuard {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    };
                    if state.holder != *task {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongGuardAffinity {
                            event: index,
                            guard: *guard,
                            task: *task,
                        });
                        break 'event;
                    }
                    if state.phase != InvariantPhase::Closed {
                        diagnostics.insert(MutexInvariantDiagnostic::DoubleOpenInvariant {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    }
                    state.phase = InvariantPhase::Open;
                    let mutex = state.mutex;
                    transitions.push(InvariantTransition {
                        event: index,
                        mutex,
                        guard: *guard,
                        from: InvariantPhase::Closed,
                        to: InvariantPhase::Open,
                        witness: None,
                    });
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::CloseInvariant {
                    task,
                    guard,
                    witness,
                } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    let Some(state) = guards.get(guard) else {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadGuard {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    };
                    if state.holder != *task {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongGuardAffinity {
                            event: index,
                            guard: *guard,
                            task: *task,
                        });
                        break 'event;
                    }
                    if state.phase != InvariantPhase::Open {
                        diagnostics.insert(MutexInvariantDiagnostic::InvalidInvariantWitness {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    }
                    let mutex = state.mutex;
                    let declared_invariant = mutexes.get(&mutex).map(|state| state.invariant);
                    let acceptable = match witness {
                        InvariantWitness::NoWriteOccurred => state.writes_while_open.is_empty(),
                        InvariantWitness::Declared { invariant } => {
                            declared_invariant == Some(*invariant)
                        }
                    };
                    if !acceptable {
                        diagnostics.insert(MutexInvariantDiagnostic::InvalidInvariantWitness {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    }
                    if matches!(witness, InvariantWitness::Declared { .. }) {
                        assumptions.insert(ConcurrencyAssumption::InvariantPredicateDeclared);
                    }
                    let state = guards.get_mut(guard).expect("guard was checked");
                    // Writes performed while open invalidate every overlapping fact.
                    let overwritten = state
                        .facts
                        .iter()
                        .filter(|(_, footprint)| {
                            state
                                .writes_while_open
                                .iter()
                                .any(|write| write.overlaps(**footprint))
                        })
                        .map(|(fact, _)| *fact)
                        .collect::<Vec<_>>();
                    for fact in overwritten {
                        state.facts.remove(&fact);
                        live_facts.remove(&fact);
                    }
                    state.writes_while_open.clear();
                    state.phase = InvariantPhase::Closed;
                    transitions.push(InvariantTransition {
                        event: index,
                        mutex,
                        guard: *guard,
                        from: InvariantPhase::Open,
                        to: InvariantPhase::Closed,
                        witness: Some(*witness),
                    });
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::Access {
                    task,
                    footprint,
                    kind,
                    guard,
                } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    if destroyed_resources.contains(&footprint.resource) {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadResource {
                            event: index,
                            resource: footprint.resource,
                        });
                        break 'event;
                    }
                    if !footprint.is_valid() {
                        diagnostics.insert(MutexInvariantDiagnostic::InvalidFootprint {
                            event: Some(index),
                        });
                        break 'event;
                    }
                    let protector = mutexes
                        .iter()
                        .find(|(_, state)| {
                            state
                                .protects
                                .iter()
                                .any(|protected| protected.overlaps(*footprint))
                        })
                        .map(|(mutex, _)| *mutex);
                    let mut authorized = false;
                    match (protector, guard) {
                        (Some(mutex), None) => {
                            // No fallback: a static share cannot reach a protected
                            // footprint, because creation consumed that share.
                            diagnostics.insert(
                                MutexInvariantDiagnostic::UnguardedProtectedAccess {
                                    event: index,
                                    mutex,
                                },
                            );
                        }
                        (_, Some(guard_id)) => {
                            let Some(state) = guards.get(guard_id) else {
                                diagnostics.insert(MutexInvariantDiagnostic::DeadGuard {
                                    event: index,
                                    guard: *guard_id,
                                });
                                break 'event;
                            };
                            if state.holder != *task {
                                diagnostics.insert(MutexInvariantDiagnostic::WrongGuardAffinity {
                                    event: index,
                                    guard: *guard_id,
                                    task: *task,
                                });
                                break 'event;
                            }
                            let covered = mutexes.get(&state.mutex).is_some_and(|mutex| {
                                mutex
                                    .protects
                                    .iter()
                                    .any(|protected| protected.contains(*footprint))
                            });
                            if !covered || protector != Some(state.mutex) {
                                // The repair: holding a mutex authorizes only the
                                // footprints that mutex actually protects.
                                diagnostics.insert(
                                    MutexInvariantDiagnostic::UnprotectedFootprint {
                                        event: index,
                                        guard: *guard_id,
                                    },
                                );
                                break 'event;
                            }
                            if writes(*kind) && state.phase != InvariantPhase::Open {
                                diagnostics.insert(
                                    MutexInvariantDiagnostic::WriteWithClosedInvariant {
                                        event: index,
                                        guard: *guard_id,
                                    },
                                );
                                break 'event;
                            }
                            if writes(*kind) {
                                guards
                                    .get_mut(guard_id)
                                    .expect("guard was checked")
                                    .writes_while_open
                                    .push(*footprint);
                            }
                            authorized = true;
                        }
                        (None, None) => {
                            let held = ledger.authority(*task, *footprint);
                            authorized = if writes(*kind) {
                                held == u32::from(FULL_ACCESS)
                            } else {
                                held > 0
                            };
                            if !authorized {
                                diagnostics.insert(MutexInvariantDiagnostic::UnauthorizedAccess {
                                    event: index,
                                    task: *task,
                                });
                            }
                        }
                    }
                    tick(&mut clocks, *task);
                    let current = RecordedAccess {
                        event: index,
                        task: *task,
                        footprint: *footprint,
                        kind: *kind,
                        clock: clocks[task].clone(),
                    };
                    if authorized {
                        for previous in &accesses {
                            if previous.task != *task
                                && previous.footprint.overlaps(*footprint)
                                && conflicting(previous.kind, *kind)
                                && !happens_before(&previous.clock, &current.clock)
                            {
                                diagnostics.insert(MutexInvariantDiagnostic::DataRace {
                                    first: previous.event,
                                    second: index,
                                });
                            }
                        }
                    }
                    accesses.push(current);
                }
                MutexInvariantEvent::BorrowUnderGuard {
                    task,
                    guard,
                    footprint,
                    borrow,
                } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    let Some(state) = guards.get_mut(guard) else {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadGuard {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    };
                    if state.holder != *task {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongGuardAffinity {
                            event: index,
                            guard: *guard,
                            task: *task,
                        });
                        break 'event;
                    }
                    if minted_borrows.contains(borrow) {
                        diagnostics.insert(MutexInvariantDiagnostic::IdentityNotFresh {
                            event: index,
                            kind: IdentityKind::Borrow,
                            id: *borrow,
                        });
                        break 'event;
                    }
                    let mutex = state.mutex;
                    let covered = mutexes.get(&mutex).is_some_and(|state| {
                        state
                            .protects
                            .iter()
                            .any(|protected| protected.contains(*footprint))
                    });
                    if !covered {
                        diagnostics.insert(MutexInvariantDiagnostic::UnprotectedFootprint {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    }
                    state.borrows.insert(*borrow);
                    minted_borrows.insert(*borrow);
                    live_borrows.insert(*borrow, *guard);
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::EndBorrow { task, borrow } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    let Some(guard) = live_borrows.remove(borrow) else {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownBorrow {
                            event: index,
                            borrow: *borrow,
                        });
                        break 'event;
                    };
                    if let Some(state) = guards.get_mut(&guard) {
                        state.borrows.remove(borrow);
                    }
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::AssertFact {
                    task,
                    guard,
                    footprint,
                    fact,
                } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    let Some(state) = guards.get_mut(guard) else {
                        diagnostics.insert(MutexInvariantDiagnostic::StaleProtectedFact {
                            event: index,
                            fact: *fact,
                        });
                        break 'event;
                    };
                    if state.holder != *task {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongGuardAffinity {
                            event: index,
                            guard: *guard,
                            task: *task,
                        });
                        break 'event;
                    }
                    if minted_facts.contains(fact) {
                        diagnostics.insert(MutexInvariantDiagnostic::IdentityNotFresh {
                            event: index,
                            kind: IdentityKind::Fact,
                            id: *fact,
                        });
                        break 'event;
                    }
                    let mutex = state.mutex;
                    let covered = mutexes.get(&mutex).is_some_and(|state| {
                        state
                            .protects
                            .iter()
                            .any(|protected| protected.contains(*footprint))
                    });
                    if !covered {
                        diagnostics.insert(MutexInvariantDiagnostic::UnprotectedFootprint {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    }
                    state.facts.insert(*fact, *footprint);
                    minted_facts.insert(*fact);
                    live_facts.insert(*fact, *guard);
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::UseFact { task, fact } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    if !minted_facts.contains(fact) {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownFact {
                            event: index,
                            fact: *fact,
                        });
                        break 'event;
                    }
                    // A fact established under one guard generation is unusable
                    // under the next: unlocking dropped it.
                    let live = live_facts
                        .get(fact)
                        .and_then(|guard| guards.get(guard))
                        .is_some_and(|state| state.holder == *task);
                    if !live {
                        diagnostics.insert(MutexInvariantDiagnostic::StaleProtectedFact {
                            event: index,
                            fact: *fact,
                        });
                        break 'event;
                    }
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::TransferCustody {
                    from,
                    to,
                    resource,
                    via,
                } => {
                    if !require_active(&clocks, &active, index, *to, &mut diagnostics) {
                        break 'event;
                    }
                    if !clocks.contains_key(from) {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownTask {
                            event: index,
                            task: *from,
                        });
                        break 'event;
                    }
                    if !active.contains(from)
                        && !matches!(via, SyncWitness::JoinEdge { child } if child == from)
                    {
                        diagnostics.insert(MutexInvariantDiagnostic::TerminalTask {
                            event: index,
                            task: *from,
                        });
                        break 'event;
                    }
                    if destroyed_resources.contains(resource) {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadResource {
                            event: index,
                            resource: *resource,
                        });
                        break 'event;
                    }
                    if custody.get(resource) != Some(from) {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongCustodian {
                            event: index,
                            resource: *resource,
                        });
                        break 'event;
                    }
                    if !witness_is_derivable(
                        *via,
                        *from,
                        *to,
                        *resource,
                        &clocks,
                        &task_parents,
                        &joined,
                        &joiner_of,
                        &mutexes,
                    ) {
                        diagnostics.insert(
                            MutexInvariantDiagnostic::ForgedSynchronizationWitness {
                                event: index,
                                resource: *resource,
                            },
                        );
                        break 'event;
                    }
                    custody.insert(*resource, *to);
                    sync_edges.push(DerivedSyncEdge {
                        event: index,
                        from: *from,
                        to: *to,
                        resource: *resource,
                        witness: *via,
                    });
                    tick(&mut clocks, *to);
                }
                MutexInvariantEvent::Destroy { task, resource } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    if destroyed_resources.contains(resource) {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadResource {
                            event: index,
                            resource: *resource,
                        });
                        break 'event;
                    }
                    if custody.get(resource) != Some(task) {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongCustodian {
                            event: index,
                            resource: *resource,
                        });
                        break 'event;
                    }
                    // Parked authority is outstanding authority: a live mutex over
                    // any part of this resource blocks destruction. With footprints
                    // represented this is precise, not a global guard sweep.
                    let parked = mutexes.values().any(|state| {
                        state
                            .protects
                            .iter()
                            .any(|protected| protected.resource == *resource)
                    });
                    let foreign_share = ledger
                        .task_shares
                        .iter()
                        .filter(|(atom, _)| atom.0 == *resource)
                        .any(|(_, shares)| {
                            shares.iter().any(|(holder, share)| {
                                *holder != *task && *share > 0 && !joined.contains(holder)
                            })
                        });
                    let unordered_access = accesses.iter().any(|access| {
                        access.footprint.resource == *resource
                            && !happens_before(&access.clock, &clocks[task])
                    });
                    if parked || foreign_share || unordered_access {
                        diagnostics.insert(MutexInvariantDiagnostic::NonquiescentDestruction {
                            event: index,
                            resource: *resource,
                        });
                        break 'event;
                    }
                    destroyed_resources.insert(*resource);
                    custody.remove(resource);
                    ledger.retire(*resource);
                    tick(&mut clocks, *task);
                }
                MutexInvariantEvent::ExtendProtection { task, mutex, .. } => {
                    // The protected shape is fixed at creation. This is a refusal,
                    // not an omission: the mutex is untouched and stays live.
                    let _ = task;
                    diagnostics.insert(MutexInvariantDiagnostic::DynamicProtectedShape {
                        event: index,
                        mutex: *mutex,
                    });
                }
                MutexInvariantEvent::UnsupportedConstruct { task, construct } => {
                    let _ = task;
                    diagnostics.insert(
                        MutexInvariantDiagnostic::UnsupportedSynchronizationConstruct {
                            event: index,
                            construct: *construct,
                        },
                    );
                }
            }
        }

        for resource in ledger.unconserved() {
            diagnostics.insert(MutexInvariantDiagnostic::MutexAuthorityNotConserved {
                event: Some(index),
                resource,
            });
        }
        conservation_checkpoints += 1;
    }

    for child in task_parents.keys() {
        if !joined.contains(child) {
            diagnostics.insert(MutexInvariantDiagnostic::OutstandingTask { child: *child });
        }
    }
    for guard in guards.keys() {
        diagnostics.insert(MutexInvariantDiagnostic::OutstandingGuard { guard: *guard });
    }
    for (mutex, state) in &mutexes {
        diagnostics.insert(MutexInvariantDiagnostic::OutstandingMutex { mutex: *mutex });
        for task in &state.waiters {
            diagnostics.insert(MutexInvariantDiagnostic::OutstandingWaiter {
                mutex: *mutex,
                task: *task,
            });
        }
    }

    if diagnostics.is_empty() {
        Ok(MutexInvariantCertificate {
            profile: ConcurrencyProfile::CheckedMutexInvariantV1,
            assumptions,
            checked_events: program.events.len(),
            conservation_checkpoints,
            completed_tasks: joined.into_iter().collect(),
            destroyed_resources: destroyed_resources.into_iter().collect(),
            destroyed_mutexes: destroyed_mutexes.into_iter().collect(),
            protected_shape,
            admitted_invariant_transitions: transitions,
            derived_sync_edges: sync_edges,
        })
    } else {
        Err(diagnostics.into_iter().collect())
    }
}

/// Independently re-derive a certificate from its program and refuse any field
/// the re-derivation does not reproduce. A `CheckedMutexInvariantV1`
/// certificate carrying a trusted-model premise is refused before any field
/// comparison.
pub fn recheck_mutex_invariant_certificate(
    program: &MutexInvariantProgram,
    certificate: &MutexInvariantCertificate,
) -> Result<(), Vec<CertificateRecheckError>> {
    let mut errors = Vec::new();
    for forbidden in [
        ConcurrencyAssumption::GuardFootprintAuthority,
        ConcurrencyAssumption::SynchronizedCustodyEdges,
    ] {
        if certificate.assumptions.contains(&forbidden) {
            errors.push(CertificateRecheckError::ForbiddenAssumption {
                assumption: forbidden,
            });
        }
    }
    match check_mutex_invariant_program(program) {
        Err(diagnostics) => errors.push(CertificateRecheckError::ProgramRejected(diagnostics)),
        Ok(derived) => {
            let mut mismatch = |field: &'static str, disagrees: bool| {
                if disagrees {
                    errors.push(CertificateRecheckError::FieldMismatch { field });
                }
            };
            mismatch("profile", derived.profile != certificate.profile);
            mismatch(
                "assumptions",
                derived.assumptions != certificate.assumptions,
            );
            mismatch(
                "checked_events",
                derived.checked_events != certificate.checked_events,
            );
            mismatch(
                "conservation_checkpoints",
                derived.conservation_checkpoints != certificate.conservation_checkpoints,
            );
            mismatch(
                "completed_tasks",
                derived.completed_tasks != certificate.completed_tasks,
            );
            mismatch(
                "destroyed_resources",
                derived.destroyed_resources != certificate.destroyed_resources,
            );
            mismatch(
                "destroyed_mutexes",
                derived.destroyed_mutexes != certificate.destroyed_mutexes,
            );
            mismatch(
                "protected_shape",
                derived.protected_shape != certificate.protected_shape,
            );
            mismatch(
                "admitted_invariant_transitions",
                derived.admitted_invariant_transitions
                    != certificate.admitted_invariant_transitions,
            );
            mismatch(
                "derived_sync_edges",
                derived.derived_sync_edges != certificate.derived_sync_edges,
            );
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn require_active(
    clocks: &BTreeMap<TaskId, VectorClock>,
    active: &BTreeSet<TaskId>,
    event: usize,
    task: TaskId,
    diagnostics: &mut BTreeSet<MutexInvariantDiagnostic>,
) -> bool {
    if active.contains(&task) {
        true
    } else if clocks.contains_key(&task) {
        diagnostics.insert(MutexInvariantDiagnostic::TerminalTask { event, task });
        false
    } else {
        diagnostics.insert(MutexInvariantDiagnostic::UnknownTask { event, task });
        false
    }
}

/// Derive-and-compare: the named edge must exist in the checker's own history
/// *and* the acting clocks must already order `from` before `to`. A witness is
/// never believed on the caller's word.
#[allow(clippy::too_many_arguments)]
fn witness_is_derivable(
    witness: SyncWitness,
    from: TaskId,
    to: TaskId,
    resource: SharedResourceId,
    clocks: &BTreeMap<TaskId, VectorClock>,
    task_parents: &BTreeMap<TaskId, TaskId>,
    joined: &BTreeSet<TaskId>,
    joiner_of: &BTreeMap<TaskId, TaskId>,
    mutexes: &BTreeMap<MutexId, MutexState>,
) -> bool {
    let structural = match witness {
        SyncWitness::SpawnEdge { child } => {
            child == to && task_parents.get(&to) == Some(&from) && !joined.contains(&to)
        }
        SyncWitness::JoinEdge { child } => {
            child == from && joined.contains(&from) && joiner_of.get(&from) == Some(&to)
        }
        SyncWitness::MutexHandoff { mutex } => {
            let Some(state) = mutexes.get(&mutex) else {
                return false;
            };
            // An unrelated mutex does not publish an unrelated resource.
            let protects_resource = state
                .protects
                .iter()
                .any(|footprint| footprint.resource == resource);
            // Some release *by `from`* must already be visible to `to`.
            let observed = clocks.get(&to).is_some_and(|target| {
                state
                    .release_history
                    .iter()
                    .any(|(releaser, clock)| *releaser == from && happens_before(clock, target))
            });
            protects_resource && observed
        }
    };
    let ordered = clocks
        .get(&from)
        .zip(clocks.get(&to))
        .is_some_and(|(sender, receiver)| happens_before(sender, receiver));
    structural && ordered
}

#[cfg(test)]
#[path = "../tests/unit/mutex_invariant.rs"]
mod tests;
