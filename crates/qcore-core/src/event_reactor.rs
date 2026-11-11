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

//! Generation-indexed callback lifecycle and descriptor custody model.
//!
//! The adapter supplies its profile type and explicit acceptance policy. This
//! kernel depends on no source profile, provider manifest or source checkout.
//! Supplied traces/summary authority and synchronization remain model premises;
//! this API does not reconstruct them from arbitrary source or native execution.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::os_endpoint::{
    DescriptionState, DescriptorState, Errno, Outcome, PostState, Premises, Recovery, Target,
    WriteBackState, close_poststate, poisoned_duties,
};

/// The reactor needs only whether the selected model permits cross-thread use.
/// Adapters separately determine whether their exact build profile is accepted.
pub trait ReactorProfile: Copy + Eq {
    fn threading_enabled(self) -> bool;
}

pub type EventBaseId = u32;
pub type EventId = u32;
pub type Generation = u32;
pub type CallbackArgumentId = u32;
pub type DescriptorId = i32;
pub type PayloadId = u32;
pub type BuffereventId = u32;
pub type ExecutionContextId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventKey {
    pub id: EventId,
    pub generation: Generation,
}

impl EventKey {
    pub const fn new(id: EventId, generation: Generation) -> Self {
        Self { id, generation }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuffereventKey {
    pub id: BuffereventId,
    pub generation: Generation,
}

impl BuffereventKey {
    pub const fn new(id: BuffereventId, generation: Generation) -> Self {
        Self { id, generation }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventFlags(u16);

impl EventFlags {
    pub const TIMEOUT: u16 = 0x01;
    pub const READ: u16 = 0x02;
    pub const WRITE: u16 = 0x04;
    pub const SIGNAL: u16 = 0x08;
    pub const PERSIST: u16 = 0x10;
    pub const EDGE_TRIGGERED: u16 = 0x20;
    pub const FINALIZE: u16 = 0x40;
    const KNOWN: u16 = Self::TIMEOUT
        | Self::READ
        | Self::WRITE
        | Self::SIGNAL
        | Self::PERSIST
        | Self::EDGE_TRIGGERED
        | Self::FINALIZE;

    pub const fn new(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn contains(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    /// The exact declared bits, including any this model does not know. A
    /// projection must round-trip an invalid flag set unchanged rather than
    /// normalizing it away, so [`Self::valid`] still refuses it downstream.
    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn valid(self) -> bool {
        self.0 & !Self::KNOWN == 0
            && (!self.contains(Self::EDGE_TRIGGERED)
                || self.contains(Self::READ)
                || self.contains(Self::WRITE))
            && !(self.contains(Self::SIGNAL)
                && (self.contains(Self::READ) || self.contains(Self::WRITE)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActivationReason {
    Read,
    Write,
    Timeout,
    Signal,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentMode {
    Owned,
    Borrowed,
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadAccessKind {
    Read,
    Write,
    AtomicRead,
    AtomicWrite,
}

impl PayloadAccessKind {
    const fn writes(self) -> bool {
        matches!(self, Self::Write | Self::AtomicWrite)
    }

    const fn atomic(self) -> bool {
        matches!(self, Self::AtomicRead | Self::AtomicWrite)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallbackSummary {
    pub argument: CallbackArgumentId,
    pub argument_mode: ArgumentMode,
    pub trigger_reasons: BTreeSet<ActivationReason>,
    pub payload_authority: BTreeMap<PayloadId, PayloadAccessKind>,
    pub may_defer: bool,
    pub may_spawn_tasks: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteMode {
    Ordinary,
    Blocking,
    Nonblocking,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopControl {
    Break,
    Continue,
    TimedExit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidOrderingEvidence {
    Priority,
    Readiness,
    Timing,
    TraceOrder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventLifecycle {
    Idle,
    Pending,
    Active,
    Running,
    /// The event allocation was released by its own running callback after
    /// same-dispatcher deletion.  The callback frame remains live until its
    /// matching `CallbackExit`; event custody is already gone.
    SelfTeardown,
    Finalizing,
    Finalized,
    Freed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackContext {
    Event(EventKey),
    Bufferevent(BuffereventKey),
    Caller(ExecutionContextId),
    Worker(ExecutionContextId),
}

/// A **declared** close-failure premise for one `CloseSocket` failure branch
/// (card O03-REACTOR).
///
/// # It is a premise, never an observation
///
/// Nothing in this kernel observes a platform or an `errno`. A declaring
/// adapter supplies these spellings the way profile data is supplied elsewhere
/// in this project — as declared text, resolved total-or-reject — and the
/// reactor then answers with what
/// [`crate::os_endpoint::close_poststate`] derives for that pinned target. The
/// declaration is copied into the certificate as a named premise
/// ([`DeclaredClosePremise`]) so a reader can see exactly which platform claim
/// a verdict rests on, and
/// [`recheck_declared_close_premises`] rederives the classification from the
/// relation rather than believing the record.
///
/// # Fail-closed
///
/// The declaration is optional and **absence is not a default**: a
/// `CloseSocket` failure without one keeps the unpinned answer, which poisons
/// the number exactly as before this vocabulary existed. A spelling outside any
/// of the three closed vocabularies refuses the event — it never falls back to
/// a target, an errno or a lock premise the declaration did not name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeclaredCloseFailure {
    /// One of [`DECLARED_CLOSE_TARGET_SPELLINGS`].
    pub target: String,
    /// One of [`DECLARED_CLOSE_ERRNO_SPELLINGS`].
    pub errno: String,
    /// One of [`DECLARED_CLOSE_RECORD_LOCK_SPELLINGS`]: premise A7, which the
    /// relation consumes for the whole answer. Declaring `unknown` keeps the
    /// poisoned answer on every target.
    pub record_locks: String,
}

/// The closed platform vocabulary. Nothing else resolves.
pub const DECLARED_CLOSE_TARGET_SPELLINGS: [&str; 3] = ["linux", "posix-2024", "unpinned"];

/// The closed observed-error vocabulary. `other` and `unobserved` are the
/// relation's own classes, not C error names.
pub const DECLARED_CLOSE_ERRNO_SPELLINGS: [&str; 7] = [
    "ebadf",
    "eintr",
    "eio",
    "enospc",
    "edquot",
    "other",
    "unobserved",
];

/// The closed record-lock (premise A7) vocabulary.
pub const DECLARED_CLOSE_RECORD_LOCK_SPELLINGS: [&str; 2] = ["none-held", "unknown"];

/// Why a declared close-failure premise was refused. Module-local keys; the
/// reactor reports the refusal through the existing
/// [`EventDiagnostic::L03DescriptorCustody`] and no diagnostic code is added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclaredCloseRefusal {
    UnknownTarget(String),
    UnknownErrno(String),
    UnknownRecordLocks(String),
}

/// What the relation's classification means for the reactor's descriptor model.
///
/// The reactor does not decide this: it reads
/// [`crate::os_endpoint::PostState::descriptor`] and the permitted recovery and
/// maps them, so a change to the relation moves the reactor's disposition with
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClosePremiseDisposition {
    /// [`DescriptorState::Released`]: the epoch ended, so the descriptor
    /// custody obligation is discharged and the number is **retired** — it may
    /// not be reused by a fresh pair, queried, attached to a bufferevent or
    /// closed again, exactly as a successful close retires it here.
    ReleasedNumberRetired,
    /// [`DescriptorState::Retained`] with `close` permitted: the epoch is still
    /// live and still owed, so the terminal custody sweep still requires a
    /// close and one retry is admitted. This is the only arm on which the
    /// documentation licenses the retry, and it is reached only under a pinned
    /// POSIX.1-2024 target.
    RetainedRetryPermitted,
    /// Every other answer — every unpinned failure, every `EBADF`, every
    /// unobserved errno and every denied record-lock premise. The number is
    /// neither usable, retriable nor free.
    Poisoned,
}

/// The relation's answer for one declared close failure, together with the
/// premises the reactor supplied and the disposition it read out of the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassifiedCloseFailure {
    pub target: Target,
    pub errno: Errno,
    /// The premises the reactor supplied, and why each one is justified:
    /// `epoch_live` and `sole_custodian` because this arm is reached only with
    /// the number under this trace's exclusive socket-pair custody;
    /// `result_observed` because the trace records the `-1` return;
    /// `errno_observed` because the declaration names the errno class (naming
    /// `unobserved` still poisons); `last_reference` **false** because the
    /// reactor models no open file description and claims no release of one;
    /// and `no_record_locks` from the declaration's own A7 spelling.
    pub premises: Premises,
    pub derived: PostState,
    pub disposition: ClosePremiseDisposition,
}

/// One declared close-failure premise, as it appears in an
/// [`EventCertificate`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredClosePremise {
    pub operation: usize,
    pub descriptor: DescriptorId,
    pub declared: DeclaredCloseFailure,
    pub classified: ClassifiedCloseFailure,
}

fn resolve_declared_target(spelling: &str) -> Option<Target> {
    match spelling {
        "linux" => Some(Target::Linux),
        "posix-2024" => Some(Target::Posix2024),
        "unpinned" => Some(Target::Unpinned),
        _ => None,
    }
}

fn resolve_declared_errno(spelling: &str) -> Option<Errno> {
    match spelling {
        "ebadf" => Some(Errno::Ebadf),
        "eintr" => Some(Errno::Eintr),
        "eio" => Some(Errno::Eio),
        "enospc" => Some(Errno::Enospc),
        "edquot" => Some(Errno::Edquot),
        "other" => Some(Errno::Other),
        "unobserved" => Some(Errno::Unobserved),
        _ => None,
    }
}

/// Premise A7, declared. `unknown` is not a refusal — the relation has an
/// answer for a denied A7, and it is the poisoned one.
fn resolve_declared_record_locks(spelling: &str) -> Option<bool> {
    match spelling {
        "none-held" => Some(true),
        "unknown" => Some(false),
        _ => None,
    }
}

/// The premises this trace vocabulary can justify for a failed close, with the
/// declared A7 spelling filled in. See [`ClassifiedCloseFailure::premises`].
fn reactor_close_premises(no_record_locks: bool) -> Premises {
    Premises {
        epoch_live: true,
        sole_custodian: true,
        result_observed: true,
        errno_observed: true,
        last_reference: false,
        no_record_locks,
    }
}

/// Read the reactor's disposition out of the relation's answer. Any shape the
/// relation does not resolve to a release or a documented retention is
/// poisoned.
fn disposition_of(derived: &PostState) -> ClosePremiseDisposition {
    match derived.descriptor {
        DescriptorState::Released => ClosePremiseDisposition::ReleasedNumberRetired,
        DescriptorState::Retained if derived.permitted_recovery.contains(&Recovery::Close) => {
            ClosePremiseDisposition::RetainedRetryPermitted
        }
        _ => ClosePremiseDisposition::Poisoned,
    }
}

/// Resolve a declared close-failure premise and classify it against the
/// target-qualified `close(2)` post-state relation.
///
/// # Errors
///
/// [`DeclaredCloseRefusal`] — a spelling outside the closed vocabulary. The
/// declaration is refused, never repaired and never defaulted.
pub fn classify_declared_close_failure(
    declared: &DeclaredCloseFailure,
) -> Result<ClassifiedCloseFailure, DeclaredCloseRefusal> {
    let target = resolve_declared_target(&declared.target)
        .ok_or_else(|| DeclaredCloseRefusal::UnknownTarget(declared.target.clone()))?;
    let errno = resolve_declared_errno(&declared.errno)
        .ok_or_else(|| DeclaredCloseRefusal::UnknownErrno(declared.errno.clone()))?;
    let no_record_locks = resolve_declared_record_locks(&declared.record_locks)
        .ok_or_else(|| DeclaredCloseRefusal::UnknownRecordLocks(declared.record_locks.clone()))?;
    let premises = reactor_close_premises(no_record_locks);
    // The failure branch is exactly `-1` with the declared errno class. The
    // relation, not this module, decides what that leaves behind.
    let derived = match close_poststate(target, premises, Outcome::MinusOne(errno)) {
        Ok(derived) => derived,
        // Unreachable: the not-attempted arm is the only refused input and this
        // outcome is not it. Poisoning rather than panicking keeps the failure
        // direction closed if the relation's domain ever changes.
        Err(_) => {
            return Ok(ClassifiedCloseFailure {
                target,
                errno,
                premises,
                derived: PostState {
                    target,
                    descriptor: DescriptorState::Indeterminate,
                    description: DescriptionState::Unknown,
                    writeback: WriteBackState::Unknown,
                    duties: poisoned_duties(),
                    permitted_recovery: BTreeSet::new(),
                    premise_violations: Vec::new(),
                    note: "outside the relation's declared domain",
                },
                disposition: ClosePremiseDisposition::Poisoned,
            });
        }
    };
    let disposition = disposition_of(&derived);
    Ok(ClassifiedCloseFailure {
        target,
        errno,
        premises,
        derived,
        disposition,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventOperation<P> {
    CreateBase {
        base: EventBaseId,
        profile: P,
    },
    BeginDispatch {
        base: EventBaseId,
        dispatcher: ExecutionContextId,
    },
    EndDispatch {
        base: EventBaseId,
    },
    Assign {
        event: EventKey,
        base: EventBaseId,
        descriptor: Option<DescriptorId>,
        flags: EventFlags,
        priority: u8,
        summary: CallbackSummary,
    },
    Add {
        event: EventKey,
    },
    Reschedule {
        event: EventKey,
    },
    Activate {
        event: EventKey,
        reason: ActivationReason,
        cross_thread: bool,
    },
    CallbackEnter {
        event: EventKey,
        reason: ActivationReason,
    },
    CallbackExit {
        event: EventKey,
    },
    Delete {
        event: EventKey,
        mode: DeleteMode,
    },
    /// `event_del` issued by the callback that is currently running on this
    /// dispatcher.  Libevent's `EVENT_DEL_AUTOBLOCK` does not wait when
    /// `EVBASE_IN_THREAD(base)` is true, unlike an ordinary cross-thread
    /// delete.  Keeping this operation distinct preserves `Delete`'s existing
    /// running-callback refusal.
    DeleteCurrentCallback {
        event: EventKey,
        dispatcher: ExecutionContextId,
    },
    Finalize {
        event: EventKey,
    },
    FreeEvent {
        event: EventKey,
    },
    /// Release an event from its own callback after
    /// `DeleteCurrentCallback`.  The event must not borrow a descriptor; map
    /// and descriptor custody remain separate obligations.  Ordinary
    /// `FreeEvent` continues to refuse every running callback.
    FreeCurrentCallbackEvent {
        event: EventKey,
        dispatcher: ExecutionContextId,
    },
    /// A successful socket-pair creation creates two distinct owned
    /// descriptors. `None` is its failure branch and creates no custody.
    CreateSocketPair {
        descriptors: Option<(DescriptorId, DescriptorId)>,
    },
    /// A socket-error query borrows a descriptor. It must not revive a
    /// descriptor already closed in this trace.
    GetSocketError {
        descriptor: DescriptorId,
    },
    /// Socket close consumes close custody only on its success branch.
    ///
    /// The failure branch does **not** retain a usable descriptor. With no
    /// `declared_failure`, this trace vocabulary carries no target and no
    /// `errno`, and [`crate::os_endpoint::close_poststate`] answers
    /// [`crate::os_endpoint::DescriptorState::Indeterminate`] for every failed
    /// close on an unpinned target, with no permitted recovery: on Linux the
    /// descriptor is already released except on `EBADF`, while POSIX.1-2024
    /// requires the retry Linux documents as a hazard. A failed close therefore
    /// poisons the number here — the custody stays unresolved and every later
    /// operation on that number, a retried close included, is refused (card
    /// O03-U4; `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md` §7.3).
    ///
    /// `declared_failure` (card O03-REACTOR) lets a declaring adapter pin that
    /// answer: see [`DeclaredCloseFailure`]. It is a declared premise and never
    /// an observation, absence keeps the poisoned answer byte for byte, and a
    /// spelling outside the closed vocabulary refuses the event rather than
    /// defaulting. A `declared_failure` on the **success** branch is a
    /// contradictory event and is refused the same way.
    CloseSocket {
        descriptor: DescriptorId,
        succeeds: bool,
        declared_failure: Option<DeclaredCloseFailure>,
    },
    CreateBufferevent {
        bev: BuffereventKey,
        base: EventBaseId,
        descriptor: DescriptorId,
        argument: CallbackArgumentId,
        close_on_free: bool,
        thread_safe: bool,
        deferred: bool,
        unlock_callbacks: bool,
        payload_authority: BTreeMap<PayloadId, PayloadAccessKind>,
    },
    SetWatermark {
        bev: BuffereventKey,
        read: bool,
        low: usize,
        high: usize,
    },
    BufferOutput {
        bev: BuffereventKey,
        bytes: usize,
    },
    DrainOutput {
        bev: BuffereventKey,
        bytes: usize,
    },
    BuffereventCallbackEnter {
        bev: BuffereventKey,
        caller: ExecutionContextId,
    },
    BuffereventCallbackExit {
        bev: BuffereventKey,
    },
    FreeBufferevent {
        bev: BuffereventKey,
    },
    Access {
        context: CallbackContext,
        payload: PayloadId,
        kind: PayloadAccessKind,
        synchronized: bool,
    },
    AssumeOrdering {
        evidence: InvalidOrderingEvidence,
    },
    LoopRequest {
        base: EventBaseId,
        request: LoopControl,
        cross_thread: bool,
    },
    DropArgument {
        argument: CallbackArgumentId,
    },
    FreeBase {
        base: EventBaseId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventProgram<P> {
    pub live_arguments: BTreeSet<CallbackArgumentId>,
    pub operations: Vec<EventOperation<P>>,
}

impl<P> Default for EventProgram<P> {
    fn default() -> Self {
        Self {
            live_arguments: BTreeSet::new(),
            operations: Vec::new(),
        }
    }
}

/// Bounded reactor diagnostics. Historical L00-L12 variant names are retained
/// for API parity; provider adapters own their external diagnostic namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventDiagnostic {
    L00InvalidLifecycle {
        operation: usize,
    },
    L01UseAfterFree {
        operation: usize,
    },
    L02ArgumentLifetime {
        operation: usize,
        argument: CallbackArgumentId,
    },
    L03DescriptorCustody {
        operation: usize,
        descriptor: DescriptorId,
    },
    L04ThreadInitialization {
        operation: usize,
        base: EventBaseId,
    },
    L05ConflictingAccess {
        first: usize,
        second: usize,
    },
    L06NotQuiescent {
        operation: usize,
    },
    L07OwnedObligation {
        operation: usize,
    },
    L08OrderingAssumption {
        operation: usize,
    },
    L09UnknownProfile {
        operation: usize,
    },
    L10BuffereventContract {
        operation: usize,
    },
    L11LoopExitCustody {
        operation: usize,
    },
    L12TraceRefinement {
        observation: usize,
    },
}

impl fmt::Display for EventDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCertificate {
    pub checked_operations: usize,
    pub freed_events: BTreeSet<EventKey>,
    pub freed_bufferevents: BTreeSet<BuffereventKey>,
    pub closed_descriptors: BTreeSet<DescriptorId>,
    pub dropped_arguments: BTreeSet<CallbackArgumentId>,
    pub trace: Vec<ObservedEvent>,
    /// Every declared close-failure premise this certificate rests on, in
    /// operation order (card O03-REACTOR). Empty for every program that
    /// declares nothing, which is every program written before this
    /// vocabulary existed. A reader can see which platform claim a verdict
    /// rests on, and [`recheck_declared_close_premises`] rederives each entry
    /// from [`crate::os_endpoint::close_poststate`] instead of believing it.
    pub declared_close_premises: Vec<DeclaredClosePremise>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedEvent {
    pub operation: usize,
    pub kind: ObservedKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedKind {
    Lifecycle,
    CallbackEnter,
    CallbackExit,
    Access,
    Synchronization,
    DescriptorClose,
    Teardown,
}

#[derive(Clone, Debug)]
struct BaseState<P> {
    profile: P,
    dispatcher: Option<ExecutionContextId>,
    running_callback: Option<CallbackContext>,
    loop_request: Option<LoopControl>,
    freed: bool,
}

#[derive(Clone, Debug)]
struct EventState {
    base: EventBaseId,
    descriptor: Option<DescriptorId>,
    flags: EventFlags,
    priority: u8,
    summary: CallbackSummary,
    lifecycle: EventLifecycle,
    active_reasons: BTreeSet<ActivationReason>,
    nonblocking_delete: bool,
    current_callback_delete: bool,
}

#[derive(Clone, Debug)]
struct BuffereventState {
    base: EventBaseId,
    descriptor: DescriptorId,
    argument: CallbackArgumentId,
    close_on_free: bool,
    thread_safe: bool,
    deferred: bool,
    unlock_callbacks: bool,
    callback_running: bool,
    payload_authority: BTreeMap<PayloadId, PayloadAccessKind>,
    output_bytes: usize,
    read_watermark: (usize, usize),
    write_watermark: (usize, usize),
    freed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DescriptorCustody {
    SocketPair,
    Bufferevent(BuffereventKey),
    /// A close was attempted on this number and failed. The endpoint stays in
    /// the live denominator and the number is neither usable, retriable nor
    /// free — the conservative residual of
    /// [`crate::os_endpoint::DescriptorState::Indeterminate`].
    Poisoned,
}

#[derive(Clone, Debug)]
struct RecordedAccess {
    operation: usize,
    context: CallbackContext,
    base: Option<EventBaseId>,
    payload: PayloadId,
    kind: PayloadAccessKind,
    synchronized: bool,
}

fn access_conflicts(left: PayloadAccessKind, right: PayloadAccessKind) -> bool {
    (left.writes() || right.writes()) && !(left.atomic() && right.atomic())
}

fn reason_allowed(flags: EventFlags, reason: ActivationReason) -> bool {
    match reason {
        ActivationReason::Read => flags.contains(EventFlags::READ),
        ActivationReason::Write => flags.contains(EventFlags::WRITE),
        ActivationReason::Timeout => true,
        ActivationReason::Signal => flags.contains(EventFlags::SIGNAL),
        ActivationReason::Explicit => true,
    }
}

/// Check a supplied reactor model under an explicit declared profile policy.
/// The policy establishes model selection only, not source/build provenance.
/// An empty profile list accepts no base. Observation and callback summaries are
/// supplied premises; source adapters must retain their own fail-closed gates.
pub fn check_event_program<P: ReactorProfile>(
    program: &EventProgram<P>,
    accepted_profiles: &[P],
) -> Result<EventCertificate, Vec<EventDiagnostic>> {
    let mut diagnostics = BTreeSet::new();
    let mut bases = BTreeMap::<EventBaseId, BaseState<P>>::new();
    let mut events = BTreeMap::<EventKey, EventState>::new();
    let mut latest_event_generation = BTreeMap::<EventId, Generation>::new();
    let mut bevs = BTreeMap::<BuffereventKey, BuffereventState>::new();
    let mut latest_bev_generation = BTreeMap::<BuffereventId, Generation>::new();
    let mut live_arguments = program.live_arguments.clone();
    let mut dropped_arguments = BTreeSet::new();
    let mut descriptor_custody = BTreeMap::<DescriptorId, DescriptorCustody>::new();
    let mut closed_descriptors = BTreeSet::new();
    let mut freed_events = BTreeSet::new();
    let mut freed_bufferevents = BTreeSet::new();
    let mut accesses = Vec::<RecordedAccess>::new();
    let mut trace = Vec::new();
    let mut declared_close_premises = Vec::<DeclaredClosePremise>::new();

    for (operation_index, operation) in program.operations.iter().enumerate() {
        let mut observed = ObservedKind::Lifecycle;
        match operation {
            EventOperation::CreateBase { base, profile } => {
                if bases.contains_key(base) {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                } else if !accepted_profiles.contains(profile) {
                    diagnostics.insert(EventDiagnostic::L09UnknownProfile {
                        operation: operation_index,
                    });
                } else {
                    bases.insert(
                        *base,
                        BaseState {
                            profile: *profile,
                            dispatcher: None,
                            running_callback: None,
                            loop_request: None,
                            freed: false,
                        },
                    );
                }
            }
            EventOperation::BeginDispatch { base, dispatcher } => match bases.get_mut(base) {
                Some(state) if !state.freed && state.dispatcher.is_none() => {
                    state.dispatcher = Some(*dispatcher);
                }
                Some(state) if state.freed => {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::EndDispatch { base } => match bases.get_mut(base) {
                Some(state) if state.dispatcher.is_some() && state.running_callback.is_none() => {
                    state.dispatcher = None;
                    state.loop_request = None;
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::Assign {
                event,
                base,
                descriptor,
                flags,
                priority,
                summary,
            } => {
                let stale = latest_event_generation
                    .get(&event.id)
                    .is_some_and(|generation| *generation >= event.generation);
                if descriptor.is_some_and(|descriptor| closed_descriptors.contains(&descriptor)) {
                    diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                        operation: operation_index,
                        descriptor: descriptor.expect("checked descriptor"),
                    });
                } else if stale || events.contains_key(event) {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                } else if !bases.get(base).is_some_and(|state| !state.freed) {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                } else if !flags.valid()
                    || summary.trigger_reasons.is_empty()
                    || summary
                        .trigger_reasons
                        .iter()
                        .any(|reason| !reason_allowed(*flags, *reason))
                {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                } else if !live_arguments.contains(&summary.argument) {
                    diagnostics.insert(EventDiagnostic::L02ArgumentLifetime {
                        operation: operation_index,
                        argument: summary.argument,
                    });
                } else {
                    latest_event_generation.insert(event.id, event.generation);
                    events.insert(
                        *event,
                        EventState {
                            base: *base,
                            descriptor: *descriptor,
                            flags: *flags,
                            priority: *priority,
                            summary: summary.clone(),
                            lifecycle: EventLifecycle::Idle,
                            active_reasons: BTreeSet::new(),
                            nonblocking_delete: false,
                            current_callback_delete: false,
                        },
                    );
                }
            }
            EventOperation::Add { event } | EventOperation::Reschedule { event } => {
                match events.get_mut(event) {
                    Some(state)
                        if state
                            .descriptor
                            .is_some_and(|descriptor| closed_descriptors.contains(&descriptor)) =>
                    {
                        diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                            operation: operation_index,
                            descriptor: state.descriptor.expect("checked descriptor"),
                        });
                    }
                    Some(state)
                        if matches!(
                            state.lifecycle,
                            EventLifecycle::Idle | EventLifecycle::Pending
                        ) =>
                    {
                        state.lifecycle = EventLifecycle::Pending;
                    }
                    Some(state)
                        if matches!(
                            state.lifecycle,
                            EventLifecycle::Freed | EventLifecycle::SelfTeardown
                        ) =>
                    {
                        diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                            operation: operation_index,
                        });
                    }
                    _ => {
                        diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                            operation: operation_index,
                        });
                    }
                }
            }
            EventOperation::Activate {
                event,
                reason,
                cross_thread,
            } => match events.get_mut(event) {
                Some(state)
                    if !matches!(
                        state.lifecycle,
                        EventLifecycle::Freed
                            | EventLifecycle::SelfTeardown
                            | EventLifecycle::Finalizing
                            | EventLifecycle::Finalized
                    ) && state.summary.trigger_reasons.contains(reason) =>
                {
                    if *cross_thread && !bases[&state.base].profile.threading_enabled() {
                        diagnostics.insert(EventDiagnostic::L04ThreadInitialization {
                            operation: operation_index,
                            base: state.base,
                        });
                    } else {
                        state.active_reasons.insert(*reason);
                        if state.lifecycle != EventLifecycle::Running {
                            state.lifecycle = EventLifecycle::Active;
                        }
                        if *cross_thread {
                            observed = ObservedKind::Synchronization;
                        }
                    }
                }
                Some(state)
                    if matches!(
                        state.lifecycle,
                        EventLifecycle::Freed | EventLifecycle::SelfTeardown
                    ) =>
                {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::CallbackEnter { event, reason } => {
                let selected_priority = events.get(event).map(|selected| {
                    events
                        .values()
                        .filter(|candidate| {
                            candidate.base == selected.base
                                && candidate.lifecycle == EventLifecycle::Active
                        })
                        .map(|candidate| candidate.priority)
                        .min()
                });
                let Some(state) = events.get_mut(event) else {
                    diagnostics.insert(if latest_event_generation.contains_key(&event.id) {
                        EventDiagnostic::L01UseAfterFree {
                            operation: operation_index,
                        }
                    } else {
                        EventDiagnostic::L00InvalidLifecycle {
                            operation: operation_index,
                        }
                    });
                    trace.push(ObservedEvent {
                        operation: operation_index,
                        kind: observed,
                    });
                    continue;
                };
                let Some(base) = bases.get_mut(&state.base) else {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                    trace.push(ObservedEvent {
                        operation: operation_index,
                        kind: observed,
                    });
                    continue;
                };
                if state.lifecycle != EventLifecycle::Active
                    || !state.active_reasons.remove(reason)
                    || base.dispatcher.is_none()
                    || base.running_callback.is_some()
                    || !live_arguments.contains(&state.summary.argument)
                    || selected_priority != Some(Some(state.priority))
                {
                    diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                        operation: operation_index,
                    });
                } else {
                    state.lifecycle = EventLifecycle::Running;
                    base.running_callback = Some(CallbackContext::Event(*event));
                    observed = ObservedKind::CallbackEnter;
                }
            }
            EventOperation::CallbackExit { event } => match events.get_mut(event) {
                Some(state)
                    if matches!(
                        state.lifecycle,
                        EventLifecycle::Running | EventLifecycle::SelfTeardown
                    ) =>
                {
                    let base = &mut bases.get_mut(&state.base).expect("checked base");
                    if base.running_callback != Some(CallbackContext::Event(*event)) {
                        diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                            operation: operation_index,
                        });
                    } else {
                        base.running_callback = None;
                        state.lifecycle = if state.lifecycle == EventLifecycle::SelfTeardown {
                            EventLifecycle::Freed
                        } else if state.nonblocking_delete {
                            EventLifecycle::Finalizing
                        } else if !state.active_reasons.is_empty() {
                            EventLifecycle::Active
                        } else if state.flags.contains(EventFlags::PERSIST) {
                            EventLifecycle::Pending
                        } else {
                            EventLifecycle::Idle
                        };
                        observed = ObservedKind::CallbackExit;
                    }
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::Delete { event, mode } => match events.get_mut(event) {
                Some(state) if state.lifecycle == EventLifecycle::Running => {
                    if *mode == DeleteMode::Nonblocking {
                        state.nonblocking_delete = true;
                        state.active_reasons.clear();
                    } else {
                        diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                            operation: operation_index,
                        });
                    }
                }
                Some(state)
                    if !matches!(
                        state.lifecycle,
                        EventLifecycle::Freed
                            | EventLifecycle::SelfTeardown
                            | EventLifecycle::Finalized
                    ) =>
                {
                    state.active_reasons.clear();
                    state.lifecycle = EventLifecycle::Idle;
                }
                Some(_) => {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                }
                None => {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::DeleteCurrentCallback { event, dispatcher } => {
                match events.get_mut(event) {
                    Some(state) if state.lifecycle == EventLifecycle::Running => {
                        let base = bases.get(&state.base).expect("checked base");
                        if base.dispatcher == Some(*dispatcher)
                            && base.running_callback == Some(CallbackContext::Event(*event))
                        {
                            state.nonblocking_delete = true;
                            state.current_callback_delete = true;
                            state.active_reasons.clear();
                        } else {
                            diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                                operation: operation_index,
                            });
                        }
                    }
                    _ => {
                        diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                            operation: operation_index,
                        });
                    }
                }
            }
            EventOperation::Finalize { event } => match events.get_mut(event) {
                Some(state)
                    if matches!(
                        state.lifecycle,
                        EventLifecycle::Idle | EventLifecycle::Finalizing
                    ) =>
                {
                    state.lifecycle = EventLifecycle::Finalized;
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::FreeEvent { event } => match events.get_mut(event) {
                Some(state)
                    if matches!(
                        state.lifecycle,
                        EventLifecycle::Idle | EventLifecycle::Finalized
                    ) =>
                {
                    state.lifecycle = EventLifecycle::Freed;
                    freed_events.insert(*event);
                    if state.summary.argument_mode == ArgumentMode::Owned {
                        live_arguments.remove(&state.summary.argument);
                        dropped_arguments.insert(state.summary.argument);
                    }
                    observed = ObservedKind::Teardown;
                }
                Some(_) => {
                    diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                        operation: operation_index,
                    });
                }
                None => {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::FreeCurrentCallbackEvent { event, dispatcher } => {
                match events.get_mut(event) {
                    Some(state) if state.lifecycle == EventLifecycle::Running => {
                        let base = bases.get(&state.base).expect("checked base");
                        if state.nonblocking_delete
                            && state.current_callback_delete
                            && state.descriptor.is_none()
                            && base.dispatcher == Some(*dispatcher)
                            && base.running_callback == Some(CallbackContext::Event(*event))
                        {
                            state.lifecycle = EventLifecycle::SelfTeardown;
                            freed_events.insert(*event);
                            observed = ObservedKind::Teardown;
                        } else {
                            diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                                operation: operation_index,
                            });
                        }
                    }
                    _ => {
                        diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                            operation: operation_index,
                        });
                    }
                }
            }
            EventOperation::CreateSocketPair { descriptors } => {
                if let Some((first, second)) = descriptors {
                    if *first < 0
                        || *second < 0
                        || first == second
                        || descriptor_custody.contains_key(first)
                        || descriptor_custody.contains_key(second)
                        || closed_descriptors.contains(first)
                        || closed_descriptors.contains(second)
                    {
                        diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                            operation: operation_index,
                            descriptor: *first,
                        });
                    } else {
                        descriptor_custody.insert(*first, DescriptorCustody::SocketPair);
                        descriptor_custody.insert(*second, DescriptorCustody::SocketPair);
                    }
                }
            }
            EventOperation::GetSocketError { descriptor } => {
                if closed_descriptors.contains(descriptor)
                    || descriptor_custody.get(descriptor) == Some(&DescriptorCustody::Poisoned)
                {
                    diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                        operation: operation_index,
                        descriptor: *descriptor,
                    });
                }
            }
            EventOperation::CloseSocket {
                descriptor,
                succeeds,
                declared_failure,
            } => {
                let active_borrow = events.values().any(|event| {
                    event.descriptor == Some(*descriptor)
                        && matches!(
                            event.lifecycle,
                            EventLifecycle::Active | EventLifecycle::Running
                        )
                });
                if active_borrow {
                    diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                        operation: operation_index,
                        descriptor: *descriptor,
                    });
                } else {
                    match descriptor_custody.get(descriptor) {
                        Some(DescriptorCustody::SocketPair) if *succeeds => {
                            if declared_failure.is_some() {
                                // A failure premise on the success branch is a
                                // contradictory event. Refuse it and poison the
                                // number rather than choosing which half to
                                // believe.
                                diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                                    operation: operation_index,
                                    descriptor: *descriptor,
                                });
                                descriptor_custody.insert(*descriptor, DescriptorCustody::Poisoned);
                            } else {
                                descriptor_custody.remove(descriptor);
                                closed_descriptors.insert(*descriptor);
                                observed = ObservedKind::DescriptorClose;
                            }
                        }
                        Some(DescriptorCustody::SocketPair) => {
                            // A failed close leaves the endpoint unresolved,
                            // not live. With no declared premise the post-state
                            // relation is asked about an unpinned target: it is
                            // indeterminate and permits no recovery, so the
                            // number is poisoned, the terminal custody check
                            // still refuses, and every later operation on it —
                            // a retry included — is refused at its own
                            // operation index.
                            //
                            // With a declared premise the relation is asked
                            // about the pinned target instead, and the answer
                            // it derives — not this module — decides the
                            // disposition.
                            match declared_failure {
                                None => {
                                    descriptor_custody
                                        .insert(*descriptor, DescriptorCustody::Poisoned);
                                }
                                Some(declared) => match classify_declared_close_failure(declared) {
                                    Err(_) => {
                                        // An unresolvable declaration refuses
                                        // the event; the number keeps the
                                        // conservative residual.
                                        diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                                            operation: operation_index,
                                            descriptor: *descriptor,
                                        });
                                        descriptor_custody
                                            .insert(*descriptor, DescriptorCustody::Poisoned);
                                    }
                                    Ok(classified) => {
                                        let disposition = classified.disposition;
                                        declared_close_premises.push(DeclaredClosePremise {
                                            operation: operation_index,
                                            descriptor: *descriptor,
                                            declared: declared.clone(),
                                            classified,
                                        });
                                        match disposition {
                                            ClosePremiseDisposition::ReleasedNumberRetired => {
                                                descriptor_custody.remove(descriptor);
                                                closed_descriptors.insert(*descriptor);
                                                observed = ObservedKind::DescriptorClose;
                                            }
                                            // The epoch is live and still owed:
                                            // custody is unchanged, so the
                                            // terminal sweep still requires a
                                            // close and the documented retry is
                                            // admitted.
                                            ClosePremiseDisposition::RetainedRetryPermitted => {}
                                            ClosePremiseDisposition::Poisoned => {
                                                descriptor_custody.insert(
                                                    *descriptor,
                                                    DescriptorCustody::Poisoned,
                                                );
                                            }
                                        }
                                    }
                                },
                            }
                        }
                        _ => {
                            diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                                operation: operation_index,
                                descriptor: *descriptor,
                            });
                        }
                    }
                }
            }
            EventOperation::CreateBufferevent {
                bev,
                base,
                descriptor,
                argument,
                close_on_free,
                thread_safe,
                deferred,
                unlock_callbacks,
                payload_authority,
            } => {
                let stale = latest_bev_generation
                    .get(&bev.id)
                    .is_some_and(|generation| *generation >= bev.generation);
                if stale || bevs.contains_key(bev) || !bases.get(base).is_some_and(|b| !b.freed) {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                } else if closed_descriptors.contains(descriptor)
                    || descriptor_custody.get(descriptor) == Some(&DescriptorCustody::Poisoned)
                {
                    diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                        operation: operation_index,
                        descriptor: *descriptor,
                    });
                } else if !live_arguments.contains(argument) {
                    diagnostics.insert(EventDiagnostic::L02ArgumentLifetime {
                        operation: operation_index,
                        argument: *argument,
                    });
                } else if *unlock_callbacks && !*deferred {
                    diagnostics.insert(EventDiagnostic::L10BuffereventContract {
                        operation: operation_index,
                    });
                } else if *thread_safe && !bases[base].profile.threading_enabled() {
                    diagnostics.insert(EventDiagnostic::L04ThreadInitialization {
                        operation: operation_index,
                        base: *base,
                    });
                } else if *close_on_free
                    && matches!(
                        descriptor_custody.get(descriptor),
                        Some(DescriptorCustody::Bufferevent(_))
                    )
                {
                    diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                        operation: operation_index,
                        descriptor: *descriptor,
                    });
                } else {
                    if *close_on_free {
                        descriptor_custody
                            .insert(*descriptor, DescriptorCustody::Bufferevent(*bev));
                    }
                    latest_bev_generation.insert(bev.id, bev.generation);
                    bevs.insert(
                        *bev,
                        BuffereventState {
                            base: *base,
                            descriptor: *descriptor,
                            argument: *argument,
                            close_on_free: *close_on_free,
                            thread_safe: *thread_safe,
                            deferred: *deferred,
                            unlock_callbacks: *unlock_callbacks,
                            callback_running: false,
                            payload_authority: payload_authority.clone(),
                            output_bytes: 0,
                            read_watermark: (0, 0),
                            write_watermark: (0, 0),
                            freed: false,
                        },
                    );
                }
            }
            EventOperation::SetWatermark {
                bev,
                read,
                low,
                high,
            } => match bevs.get_mut(bev) {
                Some(state) if !state.freed && (*high == 0 || *low <= *high) => {
                    if *read {
                        state.read_watermark = (*low, *high);
                    } else {
                        state.write_watermark = (*low, *high);
                    }
                }
                Some(state) if state.freed => {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L10BuffereventContract {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::BufferOutput { bev, bytes } => match bevs.get_mut(bev) {
                Some(state) if !state.freed => {
                    state.output_bytes = state.output_bytes.saturating_add(*bytes);
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::DrainOutput { bev, bytes } => match bevs.get_mut(bev) {
                Some(state) if !state.freed && *bytes <= state.output_bytes => {
                    state.output_bytes -= *bytes;
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L10BuffereventContract {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::BuffereventCallbackEnter { bev, caller } => {
                let Some(state) = bevs.get_mut(bev) else {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                    trace.push(ObservedEvent {
                        operation: operation_index,
                        kind: observed,
                    });
                    continue;
                };
                if state.freed
                    || state.callback_running
                    || !live_arguments.contains(&state.argument)
                {
                    diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                        operation: operation_index,
                    });
                } else if state.deferred {
                    let base = &mut bases.get_mut(&state.base).expect("checked base");
                    if base.dispatcher.is_none() || base.running_callback.is_some() {
                        diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                            operation: operation_index,
                        });
                    } else {
                        state.callback_running = true;
                        base.running_callback = Some(CallbackContext::Bufferevent(*bev));
                        observed = ObservedKind::CallbackEnter;
                    }
                } else {
                    let _lock_held = state.thread_safe && !state.unlock_callbacks;
                    state.callback_running = true;
                    // Immediate callbacks execute in the API caller's context.
                    let _ = caller;
                    observed = ObservedKind::CallbackEnter;
                }
            }
            EventOperation::BuffereventCallbackExit { bev } => match bevs.get_mut(bev) {
                Some(state) if state.callback_running => {
                    state.callback_running = false;
                    if state.deferred {
                        let base = &mut bases.get_mut(&state.base).expect("checked base");
                        if base.running_callback == Some(CallbackContext::Bufferevent(*bev)) {
                            base.running_callback = None;
                        }
                    }
                    observed = ObservedKind::CallbackExit;
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L00InvalidLifecycle {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::FreeBufferevent { bev } => match bevs.get_mut(bev) {
                Some(state)
                    if !state.freed && !state.callback_running && state.output_bytes == 0 =>
                {
                    state.freed = true;
                    freed_bufferevents.insert(*bev);
                    live_arguments.remove(&state.argument);
                    dropped_arguments.insert(state.argument);
                    if state.close_on_free {
                        if descriptor_custody.get(&state.descriptor)
                            == Some(&DescriptorCustody::Bufferevent(*bev))
                        {
                            descriptor_custody.remove(&state.descriptor);
                            closed_descriptors.insert(state.descriptor);
                            observed = ObservedKind::DescriptorClose;
                        } else {
                            diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
                                operation: operation_index,
                                descriptor: state.descriptor,
                            });
                        }
                    } else {
                        observed = ObservedKind::Teardown;
                    }
                }
                Some(state) if state.output_bytes != 0 => {
                    diagnostics.insert(EventDiagnostic::L07OwnedObligation {
                        operation: operation_index,
                    });
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L06NotQuiescent {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::Access {
                context,
                payload,
                kind,
                synchronized,
            } => {
                let (authorized, base) =
                    context_authority(*context, *payload, *kind, &events, &bevs, &bases);
                if !authorized {
                    diagnostics.insert(EventDiagnostic::L02ArgumentLifetime {
                        operation: operation_index,
                        argument: 0,
                    });
                }
                for prior in &accesses {
                    let same_serial_base = base.is_some() && base == prior.base;
                    if prior.payload == *payload
                        && prior.context != *context
                        && !same_serial_base
                        && access_conflicts(prior.kind, *kind)
                        && !(*synchronized || prior.synchronized)
                    {
                        diagnostics.insert(EventDiagnostic::L05ConflictingAccess {
                            first: prior.operation,
                            second: operation_index,
                        });
                    }
                }
                accesses.push(RecordedAccess {
                    operation: operation_index,
                    context: *context,
                    base,
                    payload: *payload,
                    kind: *kind,
                    synchronized: *synchronized,
                });
                observed = if *synchronized {
                    ObservedKind::Synchronization
                } else {
                    ObservedKind::Access
                };
            }
            EventOperation::AssumeOrdering { evidence: _ } => {
                diagnostics.insert(EventDiagnostic::L08OrderingAssumption {
                    operation: operation_index,
                });
            }
            EventOperation::LoopRequest {
                base,
                request,
                cross_thread,
            } => match bases.get_mut(base) {
                Some(state) if !state.freed => {
                    if *cross_thread && !state.profile.threading_enabled() {
                        diagnostics.insert(EventDiagnostic::L04ThreadInitialization {
                            operation: operation_index,
                            base: *base,
                        });
                    } else {
                        state.loop_request = Some(*request);
                        observed = ObservedKind::Synchronization;
                    }
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::DropArgument { argument } => {
                let referenced = events.values().any(|event| {
                    !matches!(
                        event.lifecycle,
                        EventLifecycle::Freed | EventLifecycle::SelfTeardown
                    ) && event.summary.argument == *argument
                }) || bevs
                    .values()
                    .any(|bev| !bev.freed && bev.argument == *argument);
                if referenced || !live_arguments.remove(argument) {
                    diagnostics.insert(EventDiagnostic::L02ArgumentLifetime {
                        operation: operation_index,
                        argument: *argument,
                    });
                } else {
                    dropped_arguments.insert(*argument);
                    observed = ObservedKind::Teardown;
                }
            }
            EventOperation::FreeBase { base } => match bases.get_mut(base) {
                Some(state) if !state.freed => {
                    let live_event = events.values().any(|event| {
                        event.base == *base && event.lifecycle != EventLifecycle::Freed
                    });
                    let live_bev = bevs.values().any(|bev| bev.base == *base && !bev.freed);
                    if state.dispatcher.is_some()
                        || state.running_callback.is_some()
                        || live_event
                        || live_bev
                    {
                        diagnostics.insert(EventDiagnostic::L11LoopExitCustody {
                            operation: operation_index,
                        });
                    } else {
                        state.freed = true;
                        observed = ObservedKind::Teardown;
                    }
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::L01UseAfterFree {
                        operation: operation_index,
                    });
                }
            },
        }
        trace.push(ObservedEvent {
            operation: operation_index,
            kind: observed,
        });
    }

    for event in events.values() {
        if event.lifecycle != EventLifecycle::Freed {
            diagnostics.insert(EventDiagnostic::L07OwnedObligation {
                operation: program.operations.len(),
            });
        }
    }
    for bev in bevs.values() {
        if !bev.freed {
            diagnostics.insert(EventDiagnostic::L07OwnedObligation {
                operation: program.operations.len(),
            });
        }
    }
    for base in bases.values() {
        if !base.freed {
            diagnostics.insert(EventDiagnostic::L11LoopExitCustody {
                operation: program.operations.len(),
            });
        }
    }
    for descriptor in descriptor_custody.keys() {
        diagnostics.insert(EventDiagnostic::L03DescriptorCustody {
            operation: program.operations.len(),
            descriptor: *descriptor,
        });
    }

    if diagnostics.is_empty() {
        Ok(EventCertificate {
            checked_operations: program.operations.len(),
            freed_events,
            freed_bufferevents,
            closed_descriptors,
            dropped_arguments,
            trace,
            declared_close_premises,
        })
    } else {
        Err(diagnostics.into_iter().collect())
    }
}

fn context_authority<P>(
    context: CallbackContext,
    payload: PayloadId,
    kind: PayloadAccessKind,
    events: &BTreeMap<EventKey, EventState>,
    bevs: &BTreeMap<BuffereventKey, BuffereventState>,
    bases: &BTreeMap<EventBaseId, BaseState<P>>,
) -> (bool, Option<EventBaseId>) {
    let permits = |declared: PayloadAccessKind| {
        declared == kind
            || (declared == PayloadAccessKind::Write && kind == PayloadAccessKind::Read)
            || (declared == PayloadAccessKind::AtomicWrite && kind == PayloadAccessKind::AtomicRead)
    };
    match context {
        CallbackContext::Event(key) => events.get(&key).map_or((false, None), |event| {
            let running = event.lifecycle == EventLifecycle::Running
                && bases[&event.base].running_callback == Some(context);
            (
                running
                    && event
                        .summary
                        .payload_authority
                        .get(&payload)
                        .is_some_and(|declared| permits(*declared)),
                Some(event.base),
            )
        }),
        CallbackContext::Bufferevent(key) => bevs.get(&key).map_or((false, None), |bev| {
            (
                bev.callback_running
                    && bev
                        .payload_authority
                        .get(&payload)
                        .is_some_and(|declared| permits(*declared)),
                bev.deferred.then_some(bev.base),
            )
        }),
        CallbackContext::Caller(_) | CallbackContext::Worker(_) => (true, None),
    }
}

/// Enumerate bounded dispatch choices.  Lower numeric priority runs first;
/// equal-priority events are deliberately nondeterministic.
pub fn explore_activation_orders(
    candidates: &[(EventKey, u8)],
    maximum: usize,
) -> Vec<Vec<EventKey>> {
    fn visit(
        remaining: &[(EventKey, u8)],
        prefix: &mut Vec<EventKey>,
        output: &mut Vec<Vec<EventKey>>,
        maximum: usize,
    ) {
        if output.len() >= maximum {
            return;
        }
        if remaining.is_empty() {
            output.push(prefix.clone());
            return;
        }
        let minimum = remaining
            .iter()
            .map(|(_, priority)| *priority)
            .min()
            .unwrap();
        for index in 0..remaining.len() {
            if remaining[index].1 != minimum {
                continue;
            }
            let mut next = remaining.to_vec();
            let (event, _) = next.remove(index);
            prefix.push(event);
            visit(&next, prefix, output, maximum);
            prefix.pop();
        }
    }
    let mut output = Vec::new();
    visit(candidates, &mut Vec::new(), &mut output, maximum);
    output
}

/// Why an independent recheck refused a certificate's declared close premises.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClosePremiseRecheckError {
    /// The certificate records a different number of premises than the program
    /// declares, or records them in a different order.
    Inventory { expected: usize, recorded: usize },
    /// The recorded premise does not name the operation the program declares
    /// it at.
    Operation { expected: usize, recorded: usize },
    /// The recorded premise names a different descriptor.
    Descriptor {
        operation: usize,
        expected: DescriptorId,
        recorded: DescriptorId,
    },
    /// The recorded declaration is not the program's declaration — the
    /// substitution the recheck exists to refuse.
    Declaration {
        operation: usize,
        expected: Box<DeclaredCloseFailure>,
        recorded: Box<DeclaredCloseFailure>,
    },
    /// The recorded classification is not what the relation derives for the
    /// program's declaration.
    Classification {
        operation: usize,
        expected: Box<ClassifiedCloseFailure>,
        recorded: Box<ClassifiedCloseFailure>,
    },
    /// The program declares a failure premise the reactor refuses, so no
    /// premise may be recorded for it.
    RefusedDeclaration {
        operation: usize,
        refusal: DeclaredCloseRefusal,
    },
    /// The program declares a failure premise on a `CloseSocket` success
    /// branch, which is a contradictory event no certificate may rest on.
    DeclarationOnSuccessArm { operation: usize },
}

/// Independently rederive every declared close premise a certificate carries.
///
/// This does not read the certificate's classification and check it for
/// plausibility: it re-resolves the program's own declared spellings through
/// the closed vocabularies, re-derives the post-state from
/// [`crate::os_endpoint::close_poststate`], recomputes the disposition, and
/// then requires the recorded inventory to be exactly that, in order. A
/// substituted target, errno, record-lock premise, descriptor state, duty set,
/// permitted recovery or disposition is refused; so is a premise recorded for
/// an operation that declares none, and a declaration the reactor itself
/// refuses.
///
/// # Errors
///
/// [`ClosePremiseRecheckError`] — the certificate is refused, never repaired.
pub fn recheck_declared_close_premises<P>(
    program: &EventProgram<P>,
    certificate: &EventCertificate,
) -> Result<(), ClosePremiseRecheckError> {
    let mut recorded = certificate.declared_close_premises.iter();
    let mut expected_count = 0usize;
    for (operation_index, operation) in program.operations.iter().enumerate() {
        let EventOperation::CloseSocket {
            descriptor,
            succeeds,
            declared_failure: Some(declared),
        } = operation
        else {
            continue;
        };
        if *succeeds {
            return Err(ClosePremiseRecheckError::DeclarationOnSuccessArm {
                operation: operation_index,
            });
        }
        let classified = classify_declared_close_failure(declared).map_err(|refusal| {
            ClosePremiseRecheckError::RefusedDeclaration {
                operation: operation_index,
                refusal,
            }
        })?;
        expected_count += 1;
        let Some(entry) = recorded.next() else {
            return Err(ClosePremiseRecheckError::Inventory {
                expected: expected_count,
                recorded: certificate.declared_close_premises.len(),
            });
        };
        if entry.operation != operation_index {
            return Err(ClosePremiseRecheckError::Operation {
                expected: operation_index,
                recorded: entry.operation,
            });
        }
        if entry.descriptor != *descriptor {
            return Err(ClosePremiseRecheckError::Descriptor {
                operation: operation_index,
                expected: *descriptor,
                recorded: entry.descriptor,
            });
        }
        if entry.declared != *declared {
            return Err(ClosePremiseRecheckError::Declaration {
                operation: operation_index,
                expected: Box::new(declared.clone()),
                recorded: Box::new(entry.declared.clone()),
            });
        }
        if entry.classified != classified {
            return Err(ClosePremiseRecheckError::Classification {
                operation: operation_index,
                expected: Box::new(classified),
                recorded: Box::new(entry.classified.clone()),
            });
        }
    }
    if recorded.next().is_some() {
        return Err(ClosePremiseRecheckError::Inventory {
            expected: expected_count,
            recorded: certificate.declared_close_premises.len(),
        });
    }
    Ok(())
}

/// Check that an adapter observation is exactly represented by the checked
/// model trace.  Observation hooks may omit ordinary lifecycle entries, but
/// may not invent, reorder, or duplicate observable boundaries/edges.
pub fn check_observation_refinement(
    certificate: &EventCertificate,
    observation: &[ObservedKind],
) -> Result<(), EventDiagnostic> {
    let expected = certificate
        .trace
        .iter()
        .filter_map(|event| (event.kind != ObservedKind::Lifecycle).then_some(event.kind))
        .collect::<Vec<_>>();
    if expected == observation {
        Ok(())
    } else {
        let first_difference = expected
            .iter()
            .zip(observation)
            .position(|(left, right)| left != right)
            .unwrap_or(expected.len().min(observation.len()));
        Err(EventDiagnostic::L12TraceRefinement {
            observation: first_difference,
        })
    }
}

#[cfg(test)]
#[path = "../tests/unit/event_reactor.rs"]
mod tests;
