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
//!
//! # Guard-local join (card `T03-GUARD-JOIN`)
//!
//! A join is a *blocking wait*. This model has no blocking: a `Lock` of a held
//! generation is refused outright rather than parked, and the representable
//! form of "task `t` is waiting for mutex `m`" is
//! [`MutexInvariantEvent::RegisterWaiter`]. So the hazard the blanket
//! `JoinWhileGuardHeld` refusal covers is one the model cannot otherwise see:
//! the joining task holds `m` and the joined subtree is, or becomes, a waiter
//! for `m` (or for anything a waiter chain leads back to `m` through), and the
//! join never returns.
//!
//! [`MutexInvariantEvent::GuardLocalJoin`] is the one opt-in exception, and it
//! removes that hazard by construction rather than by analysis. It is admitted
//! only when **all** of the following hold; failing any of G3-G10 keeps the
//! existing `JoinWhileGuardHeld` refusal, and no new diagnostic code exists.
//!
//! * **G1** every ordinary join precondition: the parent is active, `child` is
//!   its unjoined child, and the child is quiescent.
//! * **G2** `guard` is live and held by `parent` (else `DeadGuard` /
//!   `WrongGuardAffinity`, the codes those conditions already have).
//! * **G3** `guard` is the *only* live guard `parent` holds. Two held guards
//!   would need a wait-for argument across both, which this profile refuses to
//!   make.
//! * **G4** the set of guards `parent` held when it spawned `child` was exactly
//!   `{guard}`. Guard identities are never reused, so this is precisely
//!   "created strictly inside *this* critical section" — a child created before
//!   the lock, or in an earlier section over the same generation, fails here.
//! * **G5** `guard`'s invariant is `Closed` at the join.
//! * **G6** `guard` has no live guard-derived borrow.
//! * **G7** neither `child` nor any descendant of `child` named a mutex, a
//!   guard or a guard-scoped fact *anywhere in the trace*: no lock, unlock,
//!   error exit, waiter registration, mutex creation or destruction, invariant
//!   phase change, guard-derived borrow or its release, fact assertion or use,
//!   protection extension, unsupported construct, and no guarded access. This
//!   is what makes the deadlock argument structural: a subtree that never names
//!   a mutex can never be waiting for one.
//! * **G8** no live mutex records any task of that subtree as a waiter.
//! * **G9** no task of that subtree holds a nonzero share of any atom.
//! * **G10** no task of that subtree is the custodian of a live resource.
//!
//! G7 is deliberately far wider than "the child cannot take `guard`'s own
//! mutex". Admitting a child that locks some *other* generation would need the
//! wait-for graph over every task in the trace, and lock-order analysis is one
//! of this profile's named exclusions; so the narrow rule refuses it. G9 and
//! G10 hold vacuously for every trace this profile's own source lowering
//! produces (shares never move between tasks here), and are checked anyway so
//! the statement is a fact about the state rather than an argument about
//! another rule.
//!
//! What the admission does **not** establish: that the child's own work is
//! correct, that the emitted C actually runs the two tasks concurrently, or
//! deadlock freedom for any trace outside the rule. It adds no assumption to
//! the certificate — every condition is checked — and it records each admitted
//! join, with its verified quiet subtree, in
//! [`MutexInvariantCertificate::admitted_guard_local_joins`].
//!
//! # Lock ranks (card `CT33-RANKS`)
//!
//! Lock ranks were a *represented refusal* of this profile: the only way to
//! mention one was [`UnsupportedSynchronization::LockRankDeclaration`], which
//! refuses at `Q622`. That variant is unchanged and still refuses — it now
//! denotes a rank construct outside this bounded discipline (a dynamic rank, a
//! rank propagated through a call, a rank relation this model cannot decide).
//!
//! Beside it, [`MutexInvariantProgram::lock_ranks`] adds one **opt-in**,
//! module-wide discipline. It is off by default: an empty map means every
//! decision below is skipped and the checker's behaviour is byte-identical to
//! before, which is what keeps every already-written trace unmoved.
//!
//! When the map is non-empty the discipline is on for the whole program, and:
//!
//! * **R1 — no mixed modules.** Every mutex generation the trace creates must
//!   have a declared rank, and every declared rank must name a generation the
//!   trace creates. Either direction is
//!   [`MutexInvariantDiagnostic::MixedLockRankDeclaration`] (`Q625`). A module
//!   that ranks some of its mutexes and not others has no lock order at all,
//!   so it is refused rather than checked over the ranked part.
//! * **R2 — strictly increasing acquisition.** A `Lock` of a generation with
//!   rank `r` by a task that already holds a live guard over a generation with
//!   rank `held >= r` is
//!   [`MutexInvariantDiagnostic::LockRankViolation`] (`Q624`). Equality is
//!   refused, not admitted: two generations at one rank may never be nested,
//!   which is what makes the order a strict one.
//!
//! R2 is deliberately the *last* precondition of `Lock`, after the existing
//! dead-generation, unknown-mutex, already-held and guard-freshness checks. The
//! discipline is additive: it never displaces a refusal the profile already
//! issues. In particular a task re-locking a generation it already holds keeps
//! its existing `DuplicateLockCapability` (`Q600`) — the stronger, rank-free
//! fact that the generation is held — even though its equal rank would also
//! violate R2.
//!
//! What a *passing* rank check establishes is exactly R2: no execution of this
//! trace nests two acquisitions in non-increasing rank order. It is **not** a
//! deadlock-freedom proof. This model has no blocking — a `Lock` of a held
//! generation is refused rather than parked — so the wait-for graph a real
//! deadlock argument needs is not represented here at all, and waits on
//! anything other than these mutexes (I/O, joins, condition variables) are
//! outside the profile entirely. Accordingly the discipline adds **no**
//! assumption to the certificate: the declared ranks are recorded verbatim in
//! [`MutexInvariantCertificate::declared_lock_ranks`] and re-derived by the
//! independent recheck, and nothing about the world is taken on trust. A rank
//! is a name for a position in a declared order, not a claim about memory,
//! layout or the scheduler.
//!
//! # Protection growth (card `CT34-GROWTH`)
//!
//! [`MutexInvariantEvent::ExtendProtection`] is, and stays, an unconditional
//! refusal: a *proposal* to change a mutex's protected shape is refused at
//! `Q618` with the generation untouched. Beside it,
//! [`MutexInvariantEvent::GrowProtection`] is one **opt-in** admitted growth.
//! It is not a relaxation of the fixed-shape rule — it is a second, narrower
//! rule with its own preconditions, and a trace that uses neither event is
//! byte-identical to before.
//!
//! The soundness argument is the ledger's own. `CreateMutex` is sound because
//! it *moves* authority: the creator holds `FULL_ACCESS` over every declared
//! interval, and parking removes exactly that share and installs exactly one
//! parked `FULL_ACCESS` in its place, so the per-atom total
//! `sum(task shares) + parked` is unchanged and the atom acquires exactly one
//! protector. Growth performs the identical move on a further interval, so the
//! same equation holds for the same reason. Everything else in the rule is
//! there to keep the *other* invariants that a mid-trace shape change could
//! disturb:
//!
//! * **W1 — inside the generation's own critical section.** The growing task
//!   must hold a live guard over the generation. `Lock` refuses a held
//!   generation, so holding the guard means no other task is inside it. That is
//!   the same exclusivity `CreateMutex` gets for free (nobody can be inside a
//!   mutex that does not exist yet), and it is what publishes the change: every
//!   later acquirer merges this section's release clock, so the shape change is
//!   ordered before every subsequent use by the mutex's own release/acquire
//!   edge. `guard` is the only identity the event names; the generation is read
//!   off the guard, so a growth can never name a mutex its holder does not
//!   hold.
//! * **W2 — with the invariant open** (`Q626`). Closed means "the predicate
//!   holds of the protected region". Growing under a closed invariant would
//!   silently assert the predicate of memory nothing established it for. Open
//!   is the window in which the predicate need not hold, so the shape change
//!   belongs there, and the section is then *forced* through a `CloseInvariant`
//!   before it can unlock or error-exit.
//! * **W3 — the predicate identity moves with the shape** (`Q627`). A growth
//!   supplies the generation's new [`InvariantId`], and it must be one this
//!   generation has never carried. A wider region is a different proposition,
//!   so a post-growth `CloseInvariant` cannot re-use the pre-growth
//!   declaration: the reader of a certificate sees the predicate identity move
//!   exactly when the shape moved. A growth also counts as a write while the
//!   invariant is open, so the *checked* `NoWriteOccurred` witness — which
//!   asserts the invariant is unchanged — is unavailable to a section that
//!   grew, and such a section must close by declaration.
//! * **W4 — over unprotected memory only** (`Q617`). The new interval may not
//!   overlap any live generation's declared shape, this one included, and none
//!   of its atoms may already be parked. Those are the same fact from the
//!   interval side and the ledger side; both are checked, so a later change to
//!   either representation cannot open a hole on its own.
//! * **W5 — by the creator, holding full authority** (`Q621`). The grower must
//!   hold `FULL_ACCESS` over the whole new interval — parking is
//!   all-or-nothing, exactly as at creation — *and* must be the generation's
//!   creator. The second half is `DestroyMutex`'s own rule read forwards:
//!   destruction returns every parked share to the creator, so authority parked
//!   by anyone else would arrive at a task that never gave it up. A non-creator
//!   grower is therefore refused under the same `MutexAuthorityNotConserved`
//!   the destruction rule uses, for the same reason.
//!
//! Growth is **permanent for the generation**: there is no shrink event, and
//! the grown intervals are released exactly like the declared ones, to the
//! creator, when the generation is destroyed. Every admitted growth is named on
//! [`MutexInvariantCertificate::admitted_protection_growths`] with its event,
//! guard, interval and both invariant identities, and
//! [`MutexInvariantCertificate::protected_shape`] records the *effective*
//! shape — what the checker authorised against — so the two together recover
//! the timeline. An empty growth vector means no growth was admitted at all.
//!
//! What growth does **not** establish: that the newly protected memory
//! satisfies the new predicate (that is the same `InvariantPredicateDeclared`
//! premise, now attached to a named new identity), that the C layout of the
//! grown interval is what was declared (`DeclaredFootprintShape`, unchanged),
//! or anything about shrinking, replacing or re-pointing a protected identity —
//! all of which remain outside the profile.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use crate::concurrency::{
    AccessFootprint, AccessKind, ConcurrencyAssumption, ConcurrencyProfile, FULL_ACCESS, LockRank,
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
    /// A join of a child that was created **and** is being joined strictly
    /// inside one critical section of the joining task — card `T03-GUARD-JOIN`.
    ///
    /// This is the *only* admitted join while a guard is held, it is opt-in
    /// (a plain [`MutexInvariantEvent::Join`] keeps refusing with
    /// `JoinWhileGuardHeld` exactly as before), and every condition of
    /// `§ guard-local join` below is checked. Anything outside the rule keeps
    /// the same `JoinWhileGuardHeld` refusal, so no diagnostic code moves.
    GuardLocalJoin {
        parent: TaskId,
        child: TaskId,
        /// The section the child is local to. Must be live, held by `parent`,
        /// and the *only* guard `parent` holds.
        guard: GuardId,
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
    /// The one admitted protection growth — card `CT34-GROWTH`.
    ///
    /// This is opt-in and additive: a plain [`MutexInvariantEvent::ExtendProtection`]
    /// keeps refusing at `Q618` exactly as before, and a trace using neither
    /// event is unchanged. The generation is read off `guard`, never named
    /// separately, so a growth can only ever touch a mutex its holder holds.
    /// Every condition of `§ Protection growth` (W1-W5) is checked; failing any
    /// one is a refusal that leaves the generation's shape, its invariant and
    /// the share ledger untouched.
    GrowProtection {
        task: TaskId,
        /// The live section the growing task holds over the generation. W1.
        guard: GuardId,
        /// The interval to add. Must be unprotected, and wholly the grower's.
        footprint: AccessFootprint,
        /// The generation's new declared invariant. Must be one this generation
        /// has never carried: a wider region is a different proposition. W3.
        invariant: InvariantId,
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
    /// Declared lock rank per mutex generation — card `CT33-RANKS`.
    ///
    /// Empty (the default) means the rank discipline is **off** and the
    /// checker behaves exactly as it did before the discipline existed. A
    /// non-empty map turns it on for the whole program: see
    /// `§ Lock ranks` for R1 (no mixed modules) and R2 (strictly increasing
    /// acquisition). A rank is a declared position in an order, never a
    /// deadlock-freedom claim.
    pub lock_ranks: BTreeMap<MutexId, LockRank>,
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

/// One admitted guard-local join, named on the certificate so a reader can see
/// exactly which joins landed inside a critical section and under which guard.
/// A certificate with an empty vector made no such admission at all.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AdmittedGuardLocalJoin {
    pub event: usize,
    pub parent: TaskId,
    pub child: TaskId,
    pub guard: GuardId,
    /// The child and every descendant of it, all of which the checker verified
    /// named no mutex, guard or guard-scoped fact anywhere in the trace.
    pub quiet_subtree: Vec<TaskId>,
}

/// One admitted protection growth, named on the certificate so a reader can see
/// exactly which generation grew, when, under which section, over what, and
/// which declared predicate identity the growth replaced. A certificate with an
/// empty vector admitted no growth at all — card `CT34-GROWTH`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AdmittedProtectionGrowth {
    pub event: usize,
    pub task: TaskId,
    pub mutex: MutexId,
    pub guard: GuardId,
    /// The interval added. Disjoint from every interval any live generation
    /// protected at this point, this one included.
    pub footprint: AccessFootprint,
    /// The generation's declared invariant before the growth.
    pub from_invariant: InvariantId,
    /// Its declared invariant after: an identity this generation never carried.
    pub to_invariant: InvariantId,
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
    /// R2 of `§ Lock ranks`: an acquisition of `mutex` at rank `rank` while
    /// the acting task already holds `held_guard` over a generation at
    /// `held_rank >= rank`. Reported only when the rank discipline is on.
    LockRankViolation {
        event: usize,
        mutex: MutexId,
        rank: LockRank,
        held_guard: GuardId,
        held_mutex: MutexId,
        held_rank: LockRank,
    },
    /// R1 of `§ Lock ranks`: the rank discipline is on and this generation is
    /// on exactly one side of it. `event` is the `CreateMutex` index for a
    /// created-but-unranked generation, and `None` for a declared rank naming
    /// a generation the trace never creates.
    MixedLockRankDeclaration {
        event: Option<usize>,
        mutex: MutexId,
    },
    /// W2 of `§ Protection growth`: a growth attempted while the generation's
    /// invariant was not open. A closed invariant asserts the predicate of the
    /// protected region, so widening the region under one would assert it of
    /// memory nothing established it for.
    ProtectionGrowthOutsideOpenInvariant {
        event: usize,
        mutex: MutexId,
        guard: GuardId,
    },
    /// W3 of `§ Protection growth`: a growth that did not move the generation's
    /// declared invariant to an identity it has never carried. A wider region is
    /// a different proposition, so re-using the predicate identity would widen a
    /// declared premise silently.
    ProtectionGrowthInvariantUnchanged {
        event: usize,
        mutex: MutexId,
        invariant: InvariantId,
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
    /// refusals join `Q613`. Card `CT33-RANKS` extends the profile's band to
    /// `Q625` with the two obligations of the opt-in lock-rank discipline:
    /// `Q624` for an acquisition out of rank order and `Q625` for a module
    /// that declares ranks for only part of itself. Card `CT34-GROWTH` extends
    /// it to `Q627` with the two obligations of the opt-in protection growth
    /// that are its own: `Q626` for a growth outside an open invariant and
    /// `Q627` for a growth that did not move the declared predicate identity.
    /// Its other refusals are deliberately existing codes — an already
    /// protected interval is `Q617` and an unconserved or misrouted parked
    /// share is `Q621`, both for exactly the reasons those codes already carry.
    /// Structural refusals carried over from the bounded concurrency model keep
    /// the existing concurrency pass code `Q600`; they are not new obligations
    /// of this profile.
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
            Self::LockRankViolation { .. } => "Q624",
            Self::MixedLockRankDeclaration { .. } => "Q625",
            Self::ProtectionGrowthOutsideOpenInvariant { .. } => "Q626",
            Self::ProtectionGrowthInvariantUnchanged { .. } => "Q627",
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
    /// Which mutex generation protected which exact footprints — the
    /// **effective** shape the checker authorised against, so a generation that
    /// grew records its declared intervals followed by every admitted growth in
    /// the order it was admitted. `admitted_protection_growths` names the
    /// deltas, so the two together recover the timeline; an empty growth vector
    /// means every entry here is exactly what its `CreateMutex` declared.
    pub protected_shape: BTreeMap<MutexId, Vec<AccessFootprint>>,
    pub admitted_invariant_transitions: Vec<InvariantTransition>,
    pub derived_sync_edges: Vec<DerivedSyncEdge>,
    /// Joins that landed inside a critical section and were admitted by the
    /// guard-local rule. Empty unless the trace used
    /// [`MutexInvariantEvent::GuardLocalJoin`] *and* every condition held.
    pub admitted_guard_local_joins: Vec<AdmittedGuardLocalJoin>,
    /// Protected-shape growths admitted by `§ Protection growth`. Empty unless
    /// the trace used [`MutexInvariantEvent::GrowProtection`] *and* every
    /// condition held; a reader must not read an empty vector as "growth was
    /// checked", only as "no generation grew".
    pub admitted_protection_growths: Vec<AdmittedProtectionGrowth>,
    /// The declared lock ranks the rank discipline ran under, verbatim. Empty
    /// means the discipline was off and no acquisition order was checked; a
    /// reader must not read an empty map as "ranks were satisfied". Non-empty
    /// records exactly which generation sat at which declared position, and
    /// the recheck re-derives it. It is not a deadlock-freedom claim
    /// (`§ Lock ranks`).
    pub declared_lock_ranks: BTreeMap<MutexId, LockRank>,
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
                | MutexInvariantEvent::ExtendProtection { footprint, .. }
                // A growth's interval is an atom boundary like any other, so
                // `atoms_of` stays exact for it and the authority check below
                // cannot be evaded by an interval the ledger never split on.
                | MutexInvariantEvent::GrowProtection { footprint, .. } => record(*footprint),
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
    /// Every declared invariant this generation has carried, creation first. A
    /// growth must move the identity to one that is not in here (W3), so a
    /// generation cannot grow and then revert to an earlier declaration.
    invariant_history: Vec<InvariantId>,
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
/// The acting task of an event that names a mutex, a guard or a guard-scoped
/// fact. `None` for the events that name none of the three: task structure,
/// unguarded access, custody transfer and resource destruction.
///
/// This is the whole of guard-local join condition G7's vocabulary; adding an
/// event kind that touches a mutex without listing it here would widen the
/// admission silently, so a test pins the classification of every variant.
const fn mutex_keying_task(event: &MutexInvariantEvent) -> Option<TaskId> {
    match event {
        MutexInvariantEvent::CreateMutex { task, .. }
        | MutexInvariantEvent::DestroyMutex { task, .. }
        | MutexInvariantEvent::RegisterWaiter { task, .. }
        | MutexInvariantEvent::Lock { task, .. }
        | MutexInvariantEvent::Unlock { task, .. }
        | MutexInvariantEvent::ErrorExit { task, .. }
        | MutexInvariantEvent::OpenInvariant { task, .. }
        | MutexInvariantEvent::CloseInvariant { task, .. }
        | MutexInvariantEvent::BorrowUnderGuard { task, .. }
        | MutexInvariantEvent::EndBorrow { task, .. }
        | MutexInvariantEvent::AssertFact { task, .. }
        | MutexInvariantEvent::UseFact { task, .. }
        | MutexInvariantEvent::ExtendProtection { task, .. }
        | MutexInvariantEvent::GrowProtection { task, .. }
        | MutexInvariantEvent::UnsupportedConstruct { task, .. } => Some(*task),
        MutexInvariantEvent::Access {
            task,
            guard: Some(_),
            ..
        } => Some(*task),
        MutexInvariantEvent::Access { guard: None, .. }
        | MutexInvariantEvent::Spawn { .. }
        | MutexInvariantEvent::Join { .. }
        | MutexInvariantEvent::GuardLocalJoin { .. }
        | MutexInvariantEvent::TransferCustody { .. }
        | MutexInvariantEvent::Destroy { .. } => None,
    }
}

/// The live guards `task` currently holds.
fn guards_held_by(guards: &BTreeMap<GuardId, GuardState>, task: TaskId) -> BTreeSet<GuardId> {
    guards
        .iter()
        .filter(|(_, state)| state.holder == task)
        .map(|(guard, _)| *guard)
        .collect()
}

/// `root` together with every task transitively spawned under it. The parent
/// map is finite and acyclic by construction (a `Spawn` refuses an already
/// known child), so the fixed point terminates.
fn descendant_closure(parents: &BTreeMap<TaskId, TaskId>, root: TaskId) -> BTreeSet<TaskId> {
    let mut closure = BTreeSet::from([root]);
    loop {
        let mut grew = false;
        for (child, parent) in parents {
            if closure.contains(parent) && closure.insert(*child) {
                grew = true;
            }
        }
        if !grew {
            return closure;
        }
    }
}

pub fn check_mutex_invariant_program(
    program: &MutexInvariantProgram,
) -> Result<MutexInvariantCertificate, Vec<MutexInvariantDiagnostic>> {
    let mut diagnostics = BTreeSet::<MutexInvariantDiagnostic>::new();
    // The opt-in lock-rank discipline of `§ Lock ranks`. Off for an empty map,
    // which is what keeps every trace written before the discipline existed
    // byte-identical.
    let ranked = !program.lock_ranks.is_empty();

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
    // Guard-local join bookkeeping (`§ Guard-local join`, G4 and G7).
    let mut spawn_guards = BTreeMap::<TaskId, BTreeSet<GuardId>>::new();
    let mut mutex_keyed = BTreeSet::<TaskId>::new();
    let mut guard_local_joins = Vec::<AdmittedGuardLocalJoin>::new();
    let mut protection_growths = Vec::<AdmittedProtectionGrowth>::new();

    for (index, event) in program.events.iter().enumerate() {
        // G7 is recorded on *occurrence*, before any admission check, so a task
        // whose lock was itself refused still counts as having named a mutex.
        // On an accepted trace no event is refused, so this is exactly the set
        // of admitted mutex-keyed acts; on a refused one it is strictly more
        // refusing, never less.
        if let Some(task) = mutex_keying_task(event) {
            mutex_keyed.insert(task);
        }
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
                    spawn_guards.insert(*child, guards_held_by(&guards, *parent));
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
                MutexInvariantEvent::GuardLocalJoin {
                    parent,
                    child,
                    guard,
                } => {
                    if !require_active(&clocks, &active, index, *parent, &mut diagnostics) {
                        break 'event;
                    }
                    // G1: the ordinary join's own structural preconditions, with
                    // the ordinary join's own diagnostics.
                    if task_parents.get(child) != Some(parent) || joined.contains(child) {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownJoin {
                            event: index,
                            child: *child,
                        });
                        break 'event;
                    }
                    // G2: the named section must be this task's own live guard.
                    let Some(state) = guards.get(guard) else {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadGuard {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    };
                    if state.holder != *parent {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongGuardAffinity {
                            event: index,
                            guard: *guard,
                            task: *parent,
                        });
                        break 'event;
                    }
                    let held = guards_held_by(&guards, *parent);
                    let subtree = descendant_closure(&task_parents, *child);
                    let admitted =
                        // G3: exactly this one section is held.
                        held.len() == 1 && held.contains(guard)
                        // G4: created strictly inside this same section.
                        && spawn_guards.get(child) == Some(&held)
                        // G5: the protected invariant is not open at the join.
                        && state.phase == InvariantPhase::Closed
                        // G6: no guard-derived borrow is live.
                        && state.borrows.is_empty()
                        // G7: the joined subtree never named a mutex, guard or
                        // guard-scoped fact anywhere in the trace.
                        && subtree.iter().all(|task| !mutex_keyed.contains(task))
                        // G8: no live generation records it as a waiter.
                        && !mutexes
                            .values()
                            .any(|mutex| mutex.waiters.iter().any(|task| subtree.contains(task)))
                        // G9: the subtree holds no share of any atom.
                        && !ledger.task_shares.values().any(|shares| {
                            shares
                                .iter()
                                .any(|(task, share)| *share > 0 && subtree.contains(task))
                        })
                        // G10: the subtree is custodian of no live resource.
                        && !custody.values().any(|task| subtree.contains(task));
                    if !admitted {
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
                    if admitted {
                        guard_local_joins.push(AdmittedGuardLocalJoin {
                            event: index,
                            parent: *parent,
                            child: *child,
                            guard: *guard,
                            quiet_subtree: subtree.into_iter().collect(),
                        });
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
                    // R1 of `§ Lock ranks`: with the discipline on, a mutex
                    // this trace creates without a declared rank has no
                    // position in the order, so the module is refused rather
                    // than checked over its ranked part. The generation is not
                    // minted, so a later `Lock` of it reports `UnknownMutex`.
                    if ranked && !program.lock_ranks.contains_key(mutex) {
                        diagnostics.insert(MutexInvariantDiagnostic::MixedLockRankDeclaration {
                            event: Some(index),
                            mutex: *mutex,
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
                            invariant_history: vec![*invariant],
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
                    // R2 of `§ Lock ranks`, computed before the mutable borrow
                    // of `mutexes` below and applied *after* every existing
                    // precondition, so the discipline never displaces a
                    // refusal this profile already issues. A generation with
                    // no declared rank cannot be reached here with the
                    // discipline on — R1 refused its creation, so the
                    // generation was never minted and `UnknownMutex` below
                    // fires first — but the `None` arm blocks rather than
                    // admits, so the rule stays fail-closed if that ever
                    // changes. On that unreachable arm the reported `rank` is
                    // the held rank, since there is no declared one to name.
                    let rank_conflict = ranked
                        .then(|| {
                            let acquiring = program.lock_ranks.get(mutex).copied();
                            guards
                                .iter()
                                .filter(|(_, state)| state.holder == *task)
                                .filter_map(|(held_guard, state)| {
                                    program
                                        .lock_ranks
                                        .get(&state.mutex)
                                        .map(|held_rank| (*held_guard, state.mutex, *held_rank))
                                })
                                .find(|(_, _, held_rank)| match acquiring {
                                    Some(rank) => *held_rank >= rank,
                                    None => true,
                                })
                                .map(|(held_guard, held_mutex, held_rank)| {
                                    (
                                        acquiring.unwrap_or(held_rank),
                                        held_guard,
                                        held_mutex,
                                        held_rank,
                                    )
                                })
                        })
                        .flatten();
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
                    if let Some((rank, held_guard, held_mutex, held_rank)) = rank_conflict {
                        diagnostics.insert(MutexInvariantDiagnostic::LockRankViolation {
                            event: index,
                            mutex: *mutex,
                            rank,
                            held_guard,
                            held_mutex,
                            held_rank,
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
                MutexInvariantEvent::GrowProtection {
                    task,
                    guard,
                    footprint,
                    invariant,
                } => {
                    if !require_active(&clocks, &active, index, *task, &mut diagnostics) {
                        break 'event;
                    }
                    // W1: the generation is read off the guard, and the guard
                    // must be this task's own live section. A `Lock` refuses a
                    // held generation, so holding it means no other task is
                    // inside the mutex while its shape changes.
                    let Some(guard_state) = guards.get(guard) else {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadGuard {
                            event: index,
                            guard: *guard,
                        });
                        break 'event;
                    };
                    if guard_state.holder != *task {
                        diagnostics.insert(MutexInvariantDiagnostic::WrongGuardAffinity {
                            event: index,
                            guard: *guard,
                            task: *task,
                        });
                        break 'event;
                    }
                    let mutex = guard_state.mutex;
                    let phase = guard_state.phase;
                    // A live guard implies a live generation (destruction
                    // refuses while any guard names it), so this arm is
                    // unreachable; it blocks rather than admits, so the rule
                    // stays fail-closed if that ever changes.
                    let Some(state) = mutexes.get(&mutex) else {
                        diagnostics.insert(MutexInvariantDiagnostic::UnknownMutex {
                            event: index,
                            mutex,
                        });
                        break 'event;
                    };
                    // W2: a closed invariant asserts the predicate of the
                    // protected region; widening it there would assert the
                    // predicate of memory nothing established it for.
                    if phase != InvariantPhase::Open {
                        diagnostics.insert(
                            MutexInvariantDiagnostic::ProtectionGrowthOutsideOpenInvariant {
                                event: index,
                                mutex,
                                guard: *guard,
                            },
                        );
                        break 'event;
                    }
                    if !footprint.is_valid() {
                        diagnostics.insert(MutexInvariantDiagnostic::InvalidFootprint {
                            event: Some(index),
                        });
                        break 'event;
                    }
                    if destroyed_resources.contains(&footprint.resource) {
                        diagnostics.insert(MutexInvariantDiagnostic::DeadResource {
                            event: index,
                            resource: footprint.resource,
                        });
                        break 'event;
                    }
                    // W4: the same fact from the interval side and the ledger
                    // side. Every parked atom belongs to some live generation's
                    // declared shape, so the second half is redundant today; it
                    // is checked anyway so a later change to either
                    // representation cannot open a hole on its own.
                    let overlaps_live = mutexes.values().any(|live| {
                        live.protects
                            .iter()
                            .any(|protected| protected.overlaps(*footprint))
                    });
                    let already_parked = ledger
                        .atoms_of(*footprint)
                        .iter()
                        .any(|atom| ledger.parked.contains_key(atom));
                    if overlaps_live || already_parked {
                        diagnostics.insert(MutexInvariantDiagnostic::FootprintAlreadyProtected {
                            event: index,
                            mutex,
                        });
                        break 'event;
                    }
                    // W5: destruction returns every parked share to the
                    // creator, so authority parked by anyone else would arrive
                    // at a task that never gave it up — `DestroyMutex`'s own
                    // rule, read forwards.
                    if state.creator != *task {
                        diagnostics.insert(MutexInvariantDiagnostic::MutexAuthorityNotConserved {
                            event: Some(index),
                            resource: footprint.resource,
                        });
                        break 'event;
                    }
                    // W5, second half: parking is all-or-nothing, exactly as at
                    // creation.
                    if ledger.authority(*task, *footprint) != u32::from(FULL_ACCESS) {
                        diagnostics.insert(MutexInvariantDiagnostic::MutexAuthorityNotConserved {
                            event: Some(index),
                            resource: footprint.resource,
                        });
                        break 'event;
                    }
                    // W3: a wider region is a different proposition, so the
                    // declared predicate identity must move to one this
                    // generation has never carried.
                    if state.invariant_history.contains(invariant) {
                        diagnostics.insert(
                            MutexInvariantDiagnostic::ProtectionGrowthInvariantUnchanged {
                                event: index,
                                mutex,
                                invariant: *invariant,
                            },
                        );
                        break 'event;
                    }
                    let from_invariant = state.invariant;
                    ledger.park(*task, mutex, *footprint);
                    let state = mutexes.get_mut(&mutex).expect("generation was checked");
                    state.protects.push(*footprint);
                    state.invariant = *invariant;
                    state.invariant_history.push(*invariant);
                    protected_shape.insert(mutex, state.protects.clone());
                    // A growth changes what the protected region *is*, so it is
                    // a write for both purposes `writes_while_open` serves: the
                    // checked `NoWriteOccurred` witness is unavailable to a
                    // section that grew, and any fact overlapping the change
                    // dies at the close.
                    guards
                        .get_mut(guard)
                        .expect("guard was checked")
                        .writes_while_open
                        .push(*footprint);
                    protection_growths.push(AdmittedProtectionGrowth {
                        event: index,
                        task: *task,
                        mutex,
                        guard: *guard,
                        footprint: *footprint,
                        from_invariant,
                        to_invariant: *invariant,
                    });
                    tick(&mut clocks, *task);
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

    // R1 of `§ Lock ranks`, the other direction: a declared rank naming a
    // generation this trace never created ranks nothing, so the module's order
    // is incomplete and the whole program is refused.
    for mutex in program.lock_ranks.keys() {
        if !minted_mutexes.contains(mutex) {
            diagnostics.insert(MutexInvariantDiagnostic::MixedLockRankDeclaration {
                event: None,
                mutex: *mutex,
            });
        }
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
            admitted_guard_local_joins: guard_local_joins,
            admitted_protection_growths: protection_growths,
            declared_lock_ranks: program.lock_ranks.clone(),
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
            mismatch(
                "admitted_guard_local_joins",
                derived.admitted_guard_local_joins != certificate.admitted_guard_local_joins,
            );
            mismatch(
                "admitted_protection_growths",
                derived.admitted_protection_growths != certificate.admitted_protection_growths,
            );
            mismatch(
                "declared_lock_ranks",
                derived.declared_lock_ranks != certificate.declared_lock_ranks,
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
