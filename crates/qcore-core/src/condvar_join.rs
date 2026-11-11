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

//! Condition-variable publication and join-before-release (card
//! **W47-CONDVAR-JOIN-PUBLICATION**, C4–C5, gateway rule 10q, contract
//! specification §C04.10, the narrowed amendment of constraint 27).
//!
//! # What this kernel decides
//!
//! **C4, publication.** A condition-variable wait releases and reacquires its
//! mutex; the signal is not a memory-publication mechanism, the mutex
//! release/acquire edge is. The kernel admits exactly the shapes the amended
//! constraint 27 lifts:
//!
//! * a wait ([`wait_verdict`]) only inside a loop that rechecks a predicate
//!   read under the mutex the wait releases, with that mutex held; a timed
//!   wait whose nonzero return leaves the loop carries no result claim;
//! * a signal ([`signal_verdict`]) only while the mutex paired with the
//!   condvar is held;
//! * a publication ([`publication_verdict`]): the results are written before
//!   the predicate, in the critical section that signals;
//! * a read of a guarded result ([`read_verdict`]) only where the predicate
//!   was observed under the mutex, or where the value is final because its
//!   single publisher writes it at most once ([`ReadLicence::StablePublication`],
//!   a derived premise, never a declared one);
//! * `init`/`destroy` ([`quiescence_verdict`]) only in a quiescent phase:
//!   before publication, after the last signal is ordered by the mutex (a
//!   derived single-signal premise), or after a join that covers every live
//!   thread.
//!
//! Everything else the constraint keeps refused refuses ([`KeptRefusal`]):
//! broadcast, a wait outside a predicate loop, a signal without the mutex, one
//! condvar used with two mutexes, a static initialiser, C11 `cnd_*`, rwlocks,
//! try/timed locks and recursive or dynamic mutex shapes.
//!
//! The waiter's own operational rule is [`waiter_step`]: a wakeup without the
//! predicate waits again (a spurious wakeup is allowed), a timeout that leaves
//! the loop makes no result claim, and only an observed predicate claims.
//!
//! **C5, join before release.** Storage another thread may retain is released
//! only after a *checked, successful* join of every thread that can reach it
//! ([`release_verdict`]). Publication (`done = true` observed) is not a join, a
//! timeout is not cancellation, and an unchecked or failed join grants nothing.
//! A joined thread's own children are not joined by it: they are covered only
//! where the adapter derives their joins, and otherwise the release is
//! unresolved.
//!
//! # Verdicts and fail-closed
//!
//! [`SyncVerdict`] is ordered `Discharged < Unresolved < NotDischarged <
//! Refused` and sites compose by their join ([`SyncVerdict::join`]). Only a
//! discharge admits. Every other verdict is a refusal (`Unknown` downstream),
//! never a validated reachable violation and never an upstream defect claim.
//! The per-unit resource route derives no thread, no predicate loop and no
//! join, so there every declared synchronization call is refused
//! ([`per_unit_route_verdict`]).
//!
//! # Reduction to the concurrent14 checker
//!
//! [`publication_trace`] lowers one publish/await/join shape into a Cycle 14
//! [`crate::concurrency::ConcurrentProgram`]: a wait is the unlock and relock
//! of its mutex (the condvar edge), the publisher's critical section is a
//! mutex-guarded write sequence, and the release is a `Destroy` after the join.
//! [`crate::concurrency::check_concurrent`]'s vector clocks then order the
//! results before the read exactly when the discharged shape holds, and report
//! a data race or a non-quiescent destruction when a read skips the mutex or a
//! release skips the join.

use std::collections::{BTreeMap, BTreeSet};

use crate::concurrency::{
    AccessFootprint, AccessKind, ConcurrentEvent, ConcurrentProgram, MutexId, SharedResourceId,
    TaskId,
};

/// A field-keyed synchronization object or guarded field
/// (`struct TYPE.FIELD`), or a global's name.
pub type Key = String;

/// A thread, named by its entry function (the loop-owner census's naming).
pub type Thread = String;

/// A refusal constraint 27 keeps after the narrowed lift.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeptRefusal {
    /// `pthread_cond_broadcast`.
    Broadcast,
    /// A wait that is not the body of a loop rechecking a predicate.
    WaitOutsidePredicateLoop,
    /// A signal while the condvar's paired mutex is not held.
    SignalWithoutMutex,
    /// One condvar waited on (or signalled) under two different mutexes.
    CondvarWithTwoMutexes,
    /// `PTHREAD_COND_INITIALIZER` and other static initialisers.
    StaticInitializer,
    /// C11 `cnd_wait`/`cnd_timedwait`/`cnd_signal`/`cnd_broadcast`.
    C11Condvar,
    /// `pthread_rwlock_*`.
    Rwlock,
    /// `pthread_mutex_trylock`/`pthread_mutex_timedlock`.
    TryOrTimedLock,
    /// A recursive or dynamically chosen mutex shape.
    RecursiveOrDynamicMutex,
}

impl KeptRefusal {
    /// A stable spelling for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Broadcast => "broadcast",
            Self::WaitOutsidePredicateLoop => "wait-outside-predicate-loop",
            Self::SignalWithoutMutex => "signal-without-mutex",
            Self::CondvarWithTwoMutexes => "condvar-with-two-mutexes",
            Self::StaticInitializer => "static-initializer",
            Self::C11Condvar => "c11-condvar",
            Self::Rwlock => "rwlock",
            Self::TryOrTimedLock => "try-or-timed-lock",
            Self::RecursiveOrDynamicMutex => "recursive-or-dynamic-mutex",
        }
    }
}

/// The answer at one site. Ordered by severity; [`Self::join`] composes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncVerdict {
    /// Every obligation of the site holds on every derived path.
    Discharged,
    /// Nothing refuses, but some premise could not be derived.
    Unresolved { reasons: BTreeSet<String> },
    /// A resolved path fails the obligation.
    NotDischarged { reason: String },
    /// A shape constraint 27 keeps refused.
    Refused(KeptRefusal),
}

impl SyncVerdict {
    fn rank(&self) -> u8 {
        match self {
            Self::Discharged => 0,
            Self::Unresolved { .. } => 1,
            Self::NotDischarged { .. } => 2,
            Self::Refused(_) => 3,
        }
    }

    /// One unresolved reason.
    #[must_use]
    pub fn unresolved(reason: impl Into<String>) -> Self {
        Self::Unresolved {
            reasons: BTreeSet::from([reason.into()]),
        }
    }

    /// One failing reason.
    #[must_use]
    pub fn not_discharged(reason: impl Into<String>) -> Self {
        Self::NotDischarged {
            reason: reason.into(),
        }
    }

    /// The composition of two sites' verdicts: the more severe one, with
    /// unresolved reasons united.
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unresolved { reasons: mut left }, Self::Unresolved { reasons: right }) => {
                left.extend(right);
                Self::Unresolved { reasons: left }
            }
            (left, right) => {
                if right.rank() > left.rank() {
                    right
                } else {
                    left
                }
            }
        }
    }

    /// Only a discharge admits.
    #[must_use]
    pub fn admits(&self) -> bool {
        matches!(self, Self::Discharged)
    }

    /// A stable spelling for reports.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Discharged => "discharged".to_owned(),
            Self::Unresolved { .. } => "unresolved".to_owned(),
            Self::NotDischarged { .. } => "not discharged".to_owned(),
            Self::Refused(refusal) => format!("refused ({})", refusal.name()),
        }
    }
}

/// Compose every verdict (an empty list is discharged: no site, nothing to
/// refuse).
#[must_use]
pub fn join_all(verdicts: impl IntoIterator<Item = SyncVerdict>) -> SyncVerdict {
    verdicts
        .into_iter()
        .fold(SyncVerdict::Discharged, SyncVerdict::join)
}

/// Whether a mutex is held at a site, as the adapter's path reading decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    /// Held on every path reaching the site.
    Yes,
    /// Released (or never acquired) on some resolved path.
    No,
    /// Not decided (a lock inside a branch, an unread key).
    Unknown,
}

/// Which condvar is used with which mutexes, over the whole program: every
/// wait names its pair, and a signal names what it holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pairing {
    pub mutexes: BTreeMap<Key, BTreeSet<Key>>,
}

impl Pairing {
    /// Record one wait's pair.
    pub fn record(&mut self, condvar: &str, mutex: &str) {
        self.mutexes
            .entry(condvar.to_owned())
            .or_default()
            .insert(mutex.to_owned());
    }

    /// The single mutex paired with a condvar, if exactly one is.
    #[must_use]
    pub fn mutex(&self, condvar: &str) -> Option<&Key> {
        let set = self.mutexes.get(condvar)?;
        (set.len() == 1).then(|| set.iter().next()).flatten()
    }

    /// One condvar used with two mutexes.
    #[must_use]
    pub fn conflict(&self, condvar: &str) -> bool {
        self.mutexes.get(condvar).is_some_and(|set| set.len() > 1)
    }
}

/// The body of the loop that encloses a wait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopBody {
    /// The wait alone (an untimed wait, or a timed wait whose result the
    /// loop ignores: a nonzero return rechecks the predicate).
    WaitOnly,
    /// A timed wait followed by `if (ret != 0) break;`: a nonzero return
    /// leaves the loop with no result claim.
    WaitThenBreakOnNonzero,
    /// Anything else the adapter does not read.
    Other,
}

/// A loop that rechecks a predicate around a wait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredicateLoop {
    /// The fields the loop condition reads.
    pub predicate: BTreeSet<Key>,
    /// The loop runs entirely under the wait's mutex (held at entry, not
    /// released in the body except by the wait itself).
    pub under_mutex: bool,
    pub body: LoopBody,
}

/// One wait site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaitSite {
    pub condvar: Key,
    pub mutex: Key,
    pub timed: bool,
    pub held: Held,
    /// `None`: the wait is not the body of a predicate loop.
    pub predicate_loop: Option<PredicateLoop>,
}

/// Decide one wait (constraint 27, lift 1).
#[must_use]
pub fn wait_verdict(site: &WaitSite, pairing: &Pairing) -> SyncVerdict {
    if pairing.conflict(&site.condvar) {
        return SyncVerdict::Refused(KeptRefusal::CondvarWithTwoMutexes);
    }
    let Some(predicate_loop) = &site.predicate_loop else {
        return SyncVerdict::Refused(KeptRefusal::WaitOutsidePredicateLoop);
    };
    if predicate_loop.predicate.is_empty() {
        return SyncVerdict::Refused(KeptRefusal::WaitOutsidePredicateLoop);
    }
    match site.held {
        Held::No => return SyncVerdict::not_discharged("the wait's mutex is not held at the wait"),
        Held::Unknown => {
            return SyncVerdict::unresolved("whether the wait's mutex is held is not derived");
        }
        Held::Yes => {}
    }
    if !predicate_loop.under_mutex {
        return SyncVerdict::not_discharged("the predicate is not read under the wait's mutex");
    }
    match predicate_loop.body {
        LoopBody::WaitOnly => SyncVerdict::Discharged,
        LoopBody::WaitThenBreakOnNonzero if site.timed => SyncVerdict::Discharged,
        LoopBody::WaitThenBreakOnNonzero | LoopBody::Other => {
            SyncVerdict::unresolved("the wait loop's body is not a shape the census reads")
        }
    }
}

/// One signal or broadcast site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalSite {
    pub condvar: Key,
    pub broadcast: bool,
    /// Mutexes held on every path reaching the site.
    pub held: BTreeSet<Key>,
    /// Some lock state at the site is not decided.
    pub held_unknown: bool,
}

/// Decide one signal (constraint 27, lift 2).
#[must_use]
pub fn signal_verdict(site: &SignalSite, pairing: &Pairing) -> SyncVerdict {
    if site.broadcast {
        return SyncVerdict::Refused(KeptRefusal::Broadcast);
    }
    if pairing.conflict(&site.condvar) {
        return SyncVerdict::Refused(KeptRefusal::CondvarWithTwoMutexes);
    }
    let Some(mutex) = pairing.mutex(&site.condvar) else {
        return SyncVerdict::unresolved("no wait pairs the condvar with a mutex");
    };
    if site.held.contains(mutex) {
        SyncVerdict::Discharged
    } else if site.held_unknown {
        SyncVerdict::unresolved("whether the condvar's mutex is held is not derived")
    } else {
        SyncVerdict::Refused(KeptRefusal::SignalWithoutMutex)
    }
}

/// One event of the critical section that signals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectionEvent {
    Write(Key),
    Signal,
}

/// A publication: the critical section (of the paired mutex) that signals,
/// and every write of a guarded field outside such a section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishSite {
    pub section: Vec<SectionEvent>,
    pub predicate: BTreeSet<Key>,
    /// Guarded fields (results and predicate) written outside any section
    /// of the paired mutex, after publication.
    pub unguarded_writes: Vec<Key>,
}

/// Decide one publication: results are written before the predicate, the
/// predicate is written in the section, and the signal follows it.
#[must_use]
pub fn publication_verdict(site: &PublishSite) -> SyncVerdict {
    if let Some(field) = site.unguarded_writes.first() {
        return SyncVerdict::not_discharged(format!(
            "guarded field `{field}` is written outside the mutex"
        ));
    }
    let mut predicate_written = false;
    let mut signalled_after_predicate = false;
    for event in &site.section {
        match event {
            SectionEvent::Write(field) if site.predicate.contains(field) => {
                predicate_written = true;
            }
            SectionEvent::Write(field) => {
                if predicate_written {
                    return SyncVerdict::not_discharged(format!(
                        "result `{field}` is written after the predicate"
                    ));
                }
            }
            SectionEvent::Signal => {
                if predicate_written {
                    signalled_after_predicate = true;
                }
            }
        }
    }
    if !predicate_written {
        return SyncVerdict::not_discharged(
            "the signalling critical section writes no predicate field",
        );
    }
    if !signalled_after_predicate {
        return SyncVerdict::not_discharged("the signal does not follow the predicate write");
    }
    SyncVerdict::Discharged
}

/// What licenses a read of a guarded result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadLicence {
    /// The read is under the mutex, and the predicate was observed true under
    /// it since the mutex was last acquired.
    PredicateUnderMutex,
    /// The read follows an observation of the predicate under the mutex, and
    /// the field's single publisher writes it at most once (derived), so the
    /// value is final.
    StablePublication,
    /// No licence on a resolved path: the named reason says which.
    None(String),
    /// Not decided.
    Unknown(String),
}

/// Decide one guarded read (C4: an observed `done` licenses the snapshot).
#[must_use]
pub fn read_verdict(licence: &ReadLicence) -> SyncVerdict {
    match licence {
        ReadLicence::PredicateUnderMutex | ReadLicence::StablePublication => {
            SyncVerdict::Discharged
        }
        ReadLicence::None(reason) => SyncVerdict::not_discharged(reason.clone()),
        ReadLicence::Unknown(reason) => SyncVerdict::unresolved(reason.clone()),
    }
}

/// How the waiter was woken, or why it returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wakeup {
    /// A signal was delivered.
    Signalled,
    /// A spurious wakeup (POSIX allows them).
    Spurious,
    /// A timed wait's deadline passed (a nonzero return).
    TimedOut,
}

/// What the waiter's loop does after one wakeup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaiterStep {
    /// The predicate still does not hold: wait again.
    Rewait,
    /// The predicate holds under the mutex: the results may be claimed.
    Claim,
    /// A timeout left the loop: no result claim.
    ExitWithoutClaim,
}

/// The waiter's rule, one wakeup at a time. The predicate is rechecked under
/// the reacquired mutex after every wakeup, whatever woke it (NC8).
#[must_use]
pub fn waiter_step(predicate_holds: bool, wakeup: Wakeup, break_on_timeout: bool) -> WaiterStep {
    if predicate_holds {
        WaiterStep::Claim
    } else if wakeup == Wakeup::TimedOut && break_on_timeout {
        WaiterStep::ExitWithoutClaim
    } else {
        WaiterStep::Rewait
    }
}

/// Run the waiter over a sequence of (predicate at the recheck, wakeup)
/// pairs: `Some(true)` claims, `Some(false)` left without a claim, `None` is
/// still waiting.
#[must_use]
pub fn run_waiter(wakeups: &[(bool, Wakeup)], break_on_timeout: bool) -> Option<bool> {
    for (predicate, wakeup) in wakeups {
        match waiter_step(*predicate, *wakeup, break_on_timeout) {
            WaiterStep::Claim => return Some(true),
            WaiterStep::ExitWithoutClaim => return Some(false),
            WaiterStep::Rewait => {}
        }
    }
    None
}

/// The phase an `init` or `destroy` of a condvar stands in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The object is not yet reachable by another thread: a local or fresh
    /// allocation that no call has received and no thread was started since.
    BeforePublication,
    /// After the waiter observed the predicate under the mutex, and every
    /// signaller signals at most once (derived): the last signal is ordered.
    AfterLastSignal,
    /// After a join; its coverage decides.
    AfterJoin(ReleaseSite),
    /// None of the above is derived.
    Unknown(String),
}

/// Decide one `init`/`destroy` (constraint 27, lift 3).
#[must_use]
pub fn quiescence_verdict(phase: &Phase) -> SyncVerdict {
    match phase {
        Phase::BeforePublication | Phase::AfterLastSignal => SyncVerdict::Discharged,
        Phase::AfterJoin(release) => release_verdict(release),
        Phase::Unknown(reason) => SyncVerdict::unresolved(reason.clone()),
    }
}

/// What stands between a thread's start and a release, on one path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinEvidence {
    /// A join whose zero result the path checked.
    CheckedSuccess,
    /// A join whose result the path does not check.
    Unchecked,
    /// The **first** join of a thread the path itself started, whose result
    /// the path ignores (card W47-CONDVAR-JOIN-STOP-PATHS). It is still not a
    /// checked join; it counts as successful only when [`join_succeeds`]
    /// derives every POSIX error case impossible.
    UncheckedFirst(JoinExclusions),
    /// A join of a thread the path had already joined: `pthread_join` on a
    /// thread that is no longer joinable is undefined behaviour, so it never
    /// ends anything (card W47-CONDVAR-JOIN-STOP-PATHS).
    Repeated,
    /// A join whose result the path knows failed.
    Failed,
    /// No join on this path; the context says what the path relied on.
    Absent(Absence),
}

/// The four POSIX error cases of one unchecked first join, each derived
/// impossible (`Ok`) or not (`Err(reason)`) (card
/// W47-CONDVAR-JOIN-STOP-PATHS).
///
/// Named premise **P-POSIX-JOIN** (stated, not proved): `pthread_join(t, …)`
/// returns 0 unless `t` does not name a joinable thread (never joinable,
/// detached, or already joined: `EINVAL`/`ESRCH`), another thread is
/// already waiting to join `t` (`EINVAL`), or the join deadlocks (`EDEADLK`:
/// `t` is the caller, or `t` is itself waiting, directly or through other
/// joins, to join the caller). A thread created with the default or an
/// explicitly joinable detach state is joinable.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct JoinExclusions {
    /// Not joinable: the spawn wrapper creates it joinable and no detach in
    /// the program may name it.
    pub joinable: Result<(), String>,
    /// Busy or already joined elsewhere: no join site that may name it runs
    /// on a thread other than the joiner's.
    pub not_busy: Result<(), String>,
    /// Self join: the joiner's own path started it, so it is not the joiner.
    pub not_self: Result<(), String>,
    /// Join cycle: nothing running on the joiner's thread publishes the
    /// joiner's identifier (`pthread_self()`), so no thread can join it.
    pub no_cycle: Result<(), String>,
}

/// Whether an unchecked first join counts as a successful one: every POSIX
/// error case is derived impossible (under P-POSIX-JOIN). The first case not
/// derived is the reason it does not.
///
/// # Errors
///
/// The first POSIX case whose exclusion is not derived, with its reason.
pub fn join_succeeds(exclusions: &JoinExclusions) -> Result<(), String> {
    for (case, result) in [
        ("not joinable", &exclusions.joinable),
        (
            "already joined or awaited by another joiner",
            &exclusions.not_busy,
        ),
        ("a self join", &exclusions.not_self),
        ("a join cycle through the joiner", &exclusions.no_cycle),
    ] {
        if let Err(reason) = result {
            return Err(format!("the POSIX case `{case}` is not excluded: {reason}"));
        }
    }
    Ok(())
}

/// A thread started (transitively) by a live retaining thread, and whether
/// the parent is derived to join it before it ends (card
/// W47-CONDVAR-JOIN-STOP-PATHS).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DescendantJoin {
    pub parent: Thread,
    pub child: Thread,
    pub evidence: DescendantEvidence,
}

/// How a descendant is joined before its parent ends.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DescendantEvidence {
    /// Joined on every path of the parent's entry to its exit (a checked
    /// join, or an unchecked first one that [`join_succeeds`] admits), with
    /// nothing unresolved on the way; and so are its own descendants.
    Joined,
    /// Derived never joined: no join site of the program may name it.
    NeverJoined(String),
    /// Neither is derived.
    Unresolved(String),
}

/// The evidence one null test of a field (`!X->F`, `X->F == NULL`) carries
/// for excluding its null arm (card W47-CONDVAR-JOIN-STOP-PATHS).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NullBranchEvidence {
    /// The earlier statement that dereferences `X->F` on every path to the
    /// test, in the same function.
    pub dominating_dereference: Option<String>,
    /// Why the function-local part fails (`X` or `X->F` assigned, `&X`
    /// taken), if it does.
    pub local_defeat: Option<String>,
    /// `X`'s declared type, when read.
    pub base_type: Option<String>,
    /// Every store of `F` into an object of that type in the program, with
    /// whether it is a pre-publication store (`Ok`) or why not (`Err`).
    pub stores: Vec<(String, Result<(), String>)>,
    /// Writes of the field the reader cannot rule out: an address of the
    /// field, a whole-object write, a store whose base type is not read.
    pub unresolved: Vec<String>,
}

/// Whether a null test's null arm is infeasible on every analysed execution.
///
/// Named premise **P-DEREF** (stated, not proved): an execution that
/// dereferences a null pointer has undefined behaviour and is not an analysed
/// execution. Then `X->F` is non-null right after its dominating dereference;
/// it stays so at the test when neither `X` nor `X->F` is assigned in between
/// by the function, and no thread can store `F` into an object already
/// reachable: every store of `F` into an object of `X`'s type is into a fresh
/// allocation no other code has received yet.
///
/// # Errors
///
/// The first part of the derivation that does not hold.
pub fn null_branch_excluded(evidence: &NullBranchEvidence) -> Result<(), String> {
    if evidence.dominating_dereference.is_none() {
        return Err("no earlier dereference of the field on every path".to_owned());
    }
    if let Some(defeat) = &evidence.local_defeat {
        return Err(defeat.clone());
    }
    if evidence.base_type.is_none() {
        return Err("the base's declared type is not read".to_owned());
    }
    if let Some(write) = evidence.unresolved.first() {
        return Err(format!("a write of the field is not ruled out: {write}"));
    }
    if let Some((site, Err(why))) = evidence.stores.iter().find(|(_, r)| r.is_err()) {
        return Err(format!(
            "the store at {site} is not a pre-publication store: {why}"
        ));
    }
    Ok(())
}

/// Why a path without a join reached the release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Absence {
    /// Nothing at all.
    Nothing,
    /// The predicate (`done`) was observed: publication is not join.
    AfterPublication,
    /// A timed wait timed out: a timeout is not cancellation.
    AfterTimeout,
}

/// One thread that may be live at a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveThread {
    pub thread: Thread,
    /// One entry per path reaching the release (with its witness).
    pub paths: Vec<(JoinEvidence, String)>,
}

/// One release of storage another thread may retain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReleaseSite {
    pub live: Vec<LiveThread>,
    /// Threads the adapter could not close (an unresolved spawn, an unread
    /// join identity).
    pub unresolved: BTreeSet<String>,
    /// The threads the live retaining threads start, and whether each is
    /// derived joined before its parent ends (card
    /// W47-CONDVAR-JOIN-STOP-PATHS).
    pub descendants: Vec<DescendantJoin>,
}

/// Decide one release (C5): only a checked, successful join of every live
/// thread on every path discharges.
#[must_use]
pub fn release_verdict(site: &ReleaseSite) -> SyncVerdict {
    // The most telling failing path first: no join at all, then a repeated
    // join, a failed join, an unchecked first join whose POSIX cases are not
    // all excluded, and an unchecked one.
    let admitted = |evidence: &JoinEvidence| match evidence {
        JoinEvidence::CheckedSuccess => true,
        JoinEvidence::UncheckedFirst(exclusions) => join_succeeds(exclusions).is_ok(),
        _ => false,
    };
    let rank = |evidence: &JoinEvidence| match evidence {
        JoinEvidence::Absent(Absence::Nothing) => 0,
        JoinEvidence::Absent(Absence::AfterTimeout) => 1,
        JoinEvidence::Absent(Absence::AfterPublication) => 2,
        JoinEvidence::Repeated => 3,
        JoinEvidence::Failed => 4,
        JoinEvidence::UncheckedFirst(_) => 5,
        JoinEvidence::Unchecked => 6,
        JoinEvidence::CheckedSuccess => 7,
    };
    let mut failing: Vec<(&LiveThread, &JoinEvidence, &String)> = site
        .live
        .iter()
        .flat_map(|live| live.paths.iter().map(move |(e, w)| (live, e, w)))
        .filter(|(_, evidence, _)| !admitted(evidence))
        .collect();
    failing.sort_by_key(|(_, evidence, _)| rank(evidence));
    if let Some((live, evidence, witness)) = failing.first() {
        let reason = match evidence {
            JoinEvidence::CheckedSuccess | JoinEvidence::Unchecked => format!(
                "the join of `{}` is not checked on a path ({witness})",
                live.thread
            ),
            JoinEvidence::UncheckedFirst(exclusions) => format!(
                "the join of `{}` is not checked on a path ({witness}), and {}",
                live.thread,
                join_succeeds(exclusions).err().unwrap_or_default()
            ),
            JoinEvidence::Repeated => format!(
                "`{}` is joined a second time on a path ({witness}): a join of a thread \
                 that is no longer joinable ends nothing",
                live.thread
            ),
            JoinEvidence::Failed => {
                format!("the join of `{}` failed on a path ({witness})", live.thread)
            }
            JoinEvidence::Absent(Absence::Nothing) => {
                format!("released before any join of `{}` ({witness})", live.thread)
            }
            JoinEvidence::Absent(Absence::AfterPublication) => {
                format!("publication is not a join of `{}` ({witness})", live.thread)
            }
            JoinEvidence::Absent(Absence::AfterTimeout) => format!(
                "a timeout is not cancellation of `{}` ({witness})",
                live.thread
            ),
        };
        return SyncVerdict::not_discharged(reason);
    }
    let mut descendants: Vec<&DescendantJoin> = site.descendants.iter().collect();
    descendants.sort();
    for descendant in &descendants {
        if let DescendantEvidence::NeverJoined(why) = &descendant.evidence {
            return SyncVerdict::not_discharged(format!(
                "`{}` starts `{}`, which is never joined: {why}",
                descendant.parent, descendant.child
            ));
        }
    }
    let mut unresolved = site.unresolved.clone();
    for descendant in descendants {
        if let DescendantEvidence::Unresolved(why) = &descendant.evidence {
            unresolved.insert(format!(
                "`{}` starts `{}`, whose join before `{}` ends is not derived: {why}",
                descendant.parent, descendant.child, descendant.parent
            ));
        }
    }
    if unresolved.is_empty() {
        SyncVerdict::Discharged
    } else {
        SyncVerdict::Unresolved {
            reasons: unresolved,
        }
    }
}

/// The named premises a release's decision rests on beyond trust boundary
/// 28's (card W47-CONDVAR-JOIN-STOP-PATHS): `P-POSIX-JOIN` when an unchecked
/// first join was admitted on some path.
#[must_use]
pub fn release_premises(site: &ReleaseSite) -> BTreeSet<&'static str> {
    let mut premises = BTreeSet::new();
    for live in &site.live {
        for (evidence, _) in &live.paths {
            if let JoinEvidence::UncheckedFirst(exclusions) = evidence
                && join_succeeds(exclusions).is_ok()
            {
                premises.insert("P-POSIX-JOIN");
            }
        }
    }
    premises
}

/// A route that derives no thread, no predicate loop and no join (the
/// per-unit resource route): every declared synchronization call refuses.
#[must_use]
pub fn per_unit_route_verdict() -> SyncVerdict {
    SyncVerdict::unresolved(
        "no derived thread, predicate loop or join on the per-unit route".to_owned(),
    )
}

/// The shape a [`publication_trace`] lowers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceShape {
    /// The waiter reads the results under the mutex (after the wait's
    /// reacquisition); otherwise it reads them after unlocking.
    pub read_under_mutex: bool,
    /// The waiter joins the publisher before releasing the storage.
    pub join_before_release: bool,
}

/// Task, mutex and storage ids of the lowered trace.
pub const TRACE_WAITER: TaskId = 0;
pub const TRACE_PUBLISHER: TaskId = 1;
pub const TRACE_MUTEX: MutexId = 1;
pub const TRACE_STORAGE: SharedResourceId = 1;

/// Lower one publish/await/join shape into a concurrent14 trace: the waiter
/// spawns the publisher, locks and waits (a wait is the unlock and relock of
/// its mutex: the condvar edge), the publisher writes the results and the
/// predicate under the mutex, the waiter reads the predicate and the results,
/// and releases the storage after (or, in the negative shape, before) the
/// join. Footprints: bytes `0..8` are the results, `8..9` the predicate.
#[must_use]
pub fn publication_trace(shape: TraceShape) -> ConcurrentProgram {
    let results = AccessFootprint::new(TRACE_STORAGE, 0, 8);
    let predicate = AccessFootprint::new(TRACE_STORAGE, 8, 9);
    let guarded = |task, footprint, kind| ConcurrentEvent::Access {
        task,
        footprint,
        kind,
        guarded_by: Some(TRACE_MUTEX),
    };
    let lock = |task| ConcurrentEvent::Lock {
        task,
        mutex: TRACE_MUTEX,
        rank: 0,
    };
    let unlock = |task| ConcurrentEvent::Unlock {
        task,
        mutex: TRACE_MUTEX,
    };
    let mut events = vec![
        ConcurrentEvent::Spawn {
            parent: TRACE_WAITER,
            child: TRACE_PUBLISHER,
        },
        lock(TRACE_WAITER),
        guarded(TRACE_WAITER, predicate, AccessKind::Read),
        // The wait releases the mutex ...
        unlock(TRACE_WAITER),
        lock(TRACE_PUBLISHER),
        guarded(TRACE_PUBLISHER, results, AccessKind::Write),
        guarded(TRACE_PUBLISHER, predicate, AccessKind::Write),
        unlock(TRACE_PUBLISHER),
        // ... and reacquires it on wakeup: the recheck observes the predicate.
        lock(TRACE_WAITER),
        guarded(TRACE_WAITER, predicate, AccessKind::Read),
    ];
    if shape.read_under_mutex {
        events.push(guarded(TRACE_WAITER, results, AccessKind::Read));
        events.push(unlock(TRACE_WAITER));
    } else {
        events.push(unlock(TRACE_WAITER));
        // A publisher write after its section: the result is not final, so a
        // read outside the mutex races with it.
        events.push(lock(TRACE_PUBLISHER));
        events.push(unlock(TRACE_PUBLISHER));
        events.push(ConcurrentEvent::Access {
            task: TRACE_PUBLISHER,
            footprint: results,
            kind: AccessKind::Write,
            guarded_by: None,
        });
        events.push(ConcurrentEvent::Access {
            task: TRACE_WAITER,
            footprint: results,
            kind: AccessKind::Read,
            guarded_by: None,
        });
    }
    let join = ConcurrentEvent::Join {
        parent: TRACE_WAITER,
        child: TRACE_PUBLISHER,
    };
    let destroy = ConcurrentEvent::Destroy {
        task: TRACE_WAITER,
        resource: TRACE_STORAGE,
    };
    if shape.join_before_release {
        events.push(join);
        events.push(destroy);
    } else {
        // The publisher still touches the storage after the waiter's claim
        // (the callback is still returning), and the release precedes the join.
        events.push(lock(TRACE_PUBLISHER));
        events.push(guarded(TRACE_PUBLISHER, predicate, AccessKind::Read));
        events.push(unlock(TRACE_PUBLISHER));
        events.push(destroy);
        events.push(join);
    }
    ConcurrentProgram {
        permissions: Vec::new(),
        initial_custody: BTreeMap::from([(TRACE_STORAGE, TRACE_WAITER)]),
        events,
    }
}

#[cfg(test)]
#[path = "../tests/unit/condvar_join.rs"]
mod tests;
