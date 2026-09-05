//! Cycle 18 bounded Libevent event-driven concurrency model.
//!
//! The model is deliberately fail closed.  It accepts only the public API
//! surface named in [`LIBEVENT_API_PROFILE`], one dispatcher per event base,
//! explicit callback summaries, and explicit synchronization evidence for
//! conflicting cross-context payload accesses.  Trace order alone is never
//! treated as happens-before evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const PINNED_LIBEVENT_COMMIT: &str = "d82464a277d0f42703702c4dfd9af6af38595a83";
pub const PINNED_LIBEVENT_DESCRIPTION: &str = "release-2.2.2-alpha-11-gd82464a2";
pub const LIBEVENT_SUBMODULE_PATH: &str = "third_party/libevent";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventBackend {
    Epoll,
    Poll,
    Select,
    Kqueue,
    Iocp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadMode {
    Disabled,
    Pthreads,
    WindowsThreads,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LibeventBuildProfile {
    pub backend: EventBackend,
    pub threads: ThreadMode,
    pub openssl: bool,
    pub debug_mode: bool,
}

impl LibeventBuildProfile {
    pub const LINUX_EPOLL: Self = Self {
        backend: EventBackend::Epoll,
        threads: ThreadMode::Pthreads,
        openssl: false,
        debug_mode: true,
    };

    pub const fn accepted(self) -> bool {
        matches!(
            (self.backend, self.threads),
            (
                EventBackend::Epoll | EventBackend::Poll | EventBackend::Select,
                ThreadMode::Pthreads
            ) | (EventBackend::Kqueue, ThreadMode::Pthreads)
                | (
                    EventBackend::Select | EventBackend::Iocp,
                    ThreadMode::WindowsThreads
                )
        ) && !self.openssl
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileDisposition {
    Accepted,
    Unknown,
}

/// The complete set of symbols recognized by the initial Cycle 18 importer.
/// Anything absent from this table is `Unknown`, including DNS, HTTP, RPC,
/// OpenSSL, listeners, custom allocators/locks, fork repair, and private APIs.
pub const LIBEVENT_API_PROFILE: &[(&str, ProfileDisposition)] = &[
    ("event_base_new", ProfileDisposition::Accepted),
    ("event_base_new_with_config", ProfileDisposition::Accepted),
    ("event_base_dispatch", ProfileDisposition::Accepted),
    ("event_base_loop", ProfileDisposition::Accepted),
    ("event_base_loopbreak", ProfileDisposition::Accepted),
    ("event_base_loopcontinue", ProfileDisposition::Accepted),
    ("event_base_loopexit", ProfileDisposition::Accepted),
    ("event_base_free", ProfileDisposition::Accepted),
    ("event_new", ProfileDisposition::Accepted),
    ("event_assign", ProfileDisposition::Accepted),
    ("event_add", ProfileDisposition::Accepted),
    ("event_del", ProfileDisposition::Accepted),
    ("event_del_block", ProfileDisposition::Accepted),
    ("event_del_noblock", ProfileDisposition::Accepted),
    ("event_active", ProfileDisposition::Accepted),
    ("event_pending", ProfileDisposition::Accepted),
    ("event_priority_set", ProfileDisposition::Accepted),
    ("event_finalize", ProfileDisposition::Accepted),
    ("event_free_finalize", ProfileDisposition::Accepted),
    ("event_free", ProfileDisposition::Accepted),
    ("evthread_use_pthreads", ProfileDisposition::Accepted),
    ("evthread_use_windows_threads", ProfileDisposition::Accepted),
    ("bufferevent_socket_new", ProfileDisposition::Accepted),
    ("bufferevent_setcb", ProfileDisposition::Accepted),
    ("bufferevent_setwatermark", ProfileDisposition::Accepted),
    ("bufferevent_enable", ProfileDisposition::Accepted),
    ("bufferevent_disable", ProfileDisposition::Accepted),
    ("bufferevent_lock", ProfileDisposition::Accepted),
    ("bufferevent_unlock", ProfileDisposition::Accepted),
    ("bufferevent_free", ProfileDisposition::Accepted),
    ("evbuffer_new", ProfileDisposition::Accepted),
    ("evbuffer_add", ProfileDisposition::Accepted),
    ("evbuffer_remove", ProfileDisposition::Accepted),
    ("evbuffer_free", ProfileDisposition::Accepted),
    ("evdns_base_new", ProfileDisposition::Unknown),
    ("evhttp_new", ProfileDisposition::Unknown),
    ("evrpc_init", ProfileDisposition::Unknown),
    (
        "bufferevent_openssl_socket_new",
        ProfileDisposition::Unknown,
    ),
    ("evconnlistener_new", ProfileDisposition::Unknown),
    ("event_set_mem_functions", ProfileDisposition::Unknown),
    ("evthread_set_lock_callbacks", ProfileDisposition::Unknown),
    ("event_reinit", ProfileDisposition::Unknown),
];

pub fn classify_libevent_symbol(symbol: &str) -> ProfileDisposition {
    LIBEVENT_API_PROFILE
        .iter()
        .find_map(|(name, disposition)| (*name == symbol).then_some(*disposition))
        .unwrap_or(ProfileDisposition::Unknown)
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventOperation {
    CreateBase {
        base: EventBaseId,
        profile: LibeventBuildProfile,
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
    Finalize {
        event: EventKey,
    },
    FreeEvent {
        event: EventKey,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventProgram {
    pub live_arguments: BTreeSet<CallbackArgumentId>,
    pub operations: Vec<EventOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventDiagnostic {
    Q700InvalidLifecycle {
        operation: usize,
    },
    Q701UseAfterFree {
        operation: usize,
    },
    Q702ArgumentLifetime {
        operation: usize,
        argument: CallbackArgumentId,
    },
    Q703DescriptorCustody {
        operation: usize,
        descriptor: DescriptorId,
    },
    Q704ThreadInitialization {
        operation: usize,
        base: EventBaseId,
    },
    Q705ConflictingAccess {
        first: usize,
        second: usize,
    },
    Q706NotQuiescent {
        operation: usize,
    },
    Q707OwnedObligation {
        operation: usize,
    },
    Q708OrderingAssumption {
        operation: usize,
    },
    Q709UnknownProfile {
        operation: usize,
    },
    Q710BuffereventContract {
        operation: usize,
    },
    Q711LoopExitCustody {
        operation: usize,
    },
    Q712TraceRefinement {
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
struct BaseState {
    profile: LibeventBuildProfile,
    dispatcher: Option<ExecutionContextId>,
    running_callback: Option<CallbackContext>,
    loop_request: Option<LoopControl>,
    freed: bool,
}

#[derive(Clone, Debug)]
struct EventState {
    base: EventBaseId,
    flags: EventFlags,
    priority: u8,
    summary: CallbackSummary,
    lifecycle: EventLifecycle,
    active_reasons: BTreeSet<ActivationReason>,
    nonblocking_delete: bool,
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

/// Check one explicit partial-order trace and return its independently useful
/// lifecycle/refinement certificate.
pub fn check_event_program(
    program: &EventProgram,
) -> Result<EventCertificate, Vec<EventDiagnostic>> {
    let mut diagnostics = BTreeSet::new();
    let mut bases = BTreeMap::<EventBaseId, BaseState>::new();
    let mut events = BTreeMap::<EventKey, EventState>::new();
    let mut latest_event_generation = BTreeMap::<EventId, Generation>::new();
    let mut bevs = BTreeMap::<BuffereventKey, BuffereventState>::new();
    let mut latest_bev_generation = BTreeMap::<BuffereventId, Generation>::new();
    let mut live_arguments = program.live_arguments.clone();
    let mut dropped_arguments = BTreeSet::new();
    let mut descriptor_custody = BTreeMap::<DescriptorId, BuffereventKey>::new();
    let mut closed_descriptors = BTreeSet::new();
    let mut freed_events = BTreeSet::new();
    let mut freed_bufferevents = BTreeSet::new();
    let mut accesses = Vec::<RecordedAccess>::new();
    let mut trace = Vec::new();

    for (operation_index, operation) in program.operations.iter().enumerate() {
        let mut observed = ObservedKind::Lifecycle;
        match operation {
            EventOperation::CreateBase { base, profile } => {
                if bases.contains_key(base) {
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
                        operation: operation_index,
                    });
                } else if !profile.accepted() {
                    diagnostics.insert(EventDiagnostic::Q709UnknownProfile {
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
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
                        operation: operation_index,
                    });
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
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
                    diagnostics.insert(EventDiagnostic::Q706NotQuiescent {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::Assign {
                event,
                base,
                descriptor: _,
                flags,
                priority,
                summary,
            } => {
                let stale = latest_event_generation
                    .get(&event.id)
                    .is_some_and(|generation| *generation >= event.generation);
                if stale || events.contains_key(event) {
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
                        operation: operation_index,
                    });
                } else if !bases.get(base).is_some_and(|state| !state.freed) {
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
                        operation: operation_index,
                    });
                } else if !flags.valid()
                    || summary.trigger_reasons.is_empty()
                    || summary
                        .trigger_reasons
                        .iter()
                        .any(|reason| !reason_allowed(*flags, *reason))
                {
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
                        operation: operation_index,
                    });
                } else if !live_arguments.contains(&summary.argument) {
                    diagnostics.insert(EventDiagnostic::Q702ArgumentLifetime {
                        operation: operation_index,
                        argument: summary.argument,
                    });
                } else {
                    latest_event_generation.insert(event.id, event.generation);
                    events.insert(
                        *event,
                        EventState {
                            base: *base,
                            flags: *flags,
                            priority: *priority,
                            summary: summary.clone(),
                            lifecycle: EventLifecycle::Idle,
                            active_reasons: BTreeSet::new(),
                            nonblocking_delete: false,
                        },
                    );
                }
            }
            EventOperation::Add { event } | EventOperation::Reschedule { event } => {
                match events.get_mut(event) {
                    Some(state)
                        if matches!(
                            state.lifecycle,
                            EventLifecycle::Idle | EventLifecycle::Pending
                        ) =>
                    {
                        state.lifecycle = EventLifecycle::Pending;
                    }
                    Some(state) if state.lifecycle == EventLifecycle::Freed => {
                        diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
                            operation: operation_index,
                        });
                    }
                    _ => {
                        diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
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
                            | EventLifecycle::Finalizing
                            | EventLifecycle::Finalized
                    ) && state.summary.trigger_reasons.contains(reason) =>
                {
                    if *cross_thread && bases[&state.base].profile.threads == ThreadMode::Disabled {
                        diagnostics.insert(EventDiagnostic::Q704ThreadInitialization {
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
                Some(state) if state.lifecycle == EventLifecycle::Freed => {
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
                        operation: operation_index,
                    });
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
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
                        EventDiagnostic::Q701UseAfterFree {
                            operation: operation_index,
                        }
                    } else {
                        EventDiagnostic::Q700InvalidLifecycle {
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
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
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
                    diagnostics.insert(EventDiagnostic::Q706NotQuiescent {
                        operation: operation_index,
                    });
                } else {
                    state.lifecycle = EventLifecycle::Running;
                    base.running_callback = Some(CallbackContext::Event(*event));
                    observed = ObservedKind::CallbackEnter;
                }
            }
            EventOperation::CallbackExit { event } => match events.get_mut(event) {
                Some(state) if state.lifecycle == EventLifecycle::Running => {
                    let base = &mut bases.get_mut(&state.base).expect("checked base");
                    if base.running_callback != Some(CallbackContext::Event(*event)) {
                        diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
                            operation: operation_index,
                        });
                    } else {
                        base.running_callback = None;
                        state.lifecycle = if state.nonblocking_delete {
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
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
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
                        diagnostics.insert(EventDiagnostic::Q706NotQuiescent {
                            operation: operation_index,
                        });
                    }
                }
                Some(state)
                    if !matches!(
                        state.lifecycle,
                        EventLifecycle::Freed | EventLifecycle::Finalized
                    ) =>
                {
                    state.active_reasons.clear();
                    state.lifecycle = EventLifecycle::Idle;
                }
                Some(_) => {
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
                        operation: operation_index,
                    });
                }
                None => {
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
                        operation: operation_index,
                    });
                }
            },
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
                    diagnostics.insert(EventDiagnostic::Q706NotQuiescent {
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
                    diagnostics.insert(EventDiagnostic::Q706NotQuiescent {
                        operation: operation_index,
                    });
                }
                None => {
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
                        operation: operation_index,
                    });
                }
            },
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
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
                        operation: operation_index,
                    });
                } else if !live_arguments.contains(argument) {
                    diagnostics.insert(EventDiagnostic::Q702ArgumentLifetime {
                        operation: operation_index,
                        argument: *argument,
                    });
                } else if *unlock_callbacks && !*deferred {
                    diagnostics.insert(EventDiagnostic::Q710BuffereventContract {
                        operation: operation_index,
                    });
                } else if *thread_safe && bases[base].profile.threads == ThreadMode::Disabled {
                    diagnostics.insert(EventDiagnostic::Q704ThreadInitialization {
                        operation: operation_index,
                        base: *base,
                    });
                } else if *close_on_free && descriptor_custody.contains_key(descriptor) {
                    diagnostics.insert(EventDiagnostic::Q703DescriptorCustody {
                        operation: operation_index,
                        descriptor: *descriptor,
                    });
                } else {
                    if *close_on_free {
                        descriptor_custody.insert(*descriptor, *bev);
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
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
                        operation: operation_index,
                    });
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::Q710BuffereventContract {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::BufferOutput { bev, bytes } => match bevs.get_mut(bev) {
                Some(state) if !state.freed => {
                    state.output_bytes = state.output_bytes.saturating_add(*bytes);
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::DrainOutput { bev, bytes } => match bevs.get_mut(bev) {
                Some(state) if !state.freed && *bytes <= state.output_bytes => {
                    state.output_bytes -= *bytes;
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::Q710BuffereventContract {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::BuffereventCallbackEnter { bev, caller } => {
                let Some(state) = bevs.get_mut(bev) else {
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
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
                    diagnostics.insert(EventDiagnostic::Q706NotQuiescent {
                        operation: operation_index,
                    });
                } else if state.deferred {
                    let base = &mut bases.get_mut(&state.base).expect("checked base");
                    if base.dispatcher.is_none() || base.running_callback.is_some() {
                        diagnostics.insert(EventDiagnostic::Q706NotQuiescent {
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
                    diagnostics.insert(EventDiagnostic::Q700InvalidLifecycle {
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
                        descriptor_custody.remove(&state.descriptor);
                        closed_descriptors.insert(state.descriptor);
                        observed = ObservedKind::DescriptorClose;
                    } else {
                        observed = ObservedKind::Teardown;
                    }
                }
                Some(state) if state.output_bytes != 0 => {
                    diagnostics.insert(EventDiagnostic::Q707OwnedObligation {
                        operation: operation_index,
                    });
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::Q706NotQuiescent {
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
                    diagnostics.insert(EventDiagnostic::Q702ArgumentLifetime {
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
                        diagnostics.insert(EventDiagnostic::Q705ConflictingAccess {
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
                diagnostics.insert(EventDiagnostic::Q708OrderingAssumption {
                    operation: operation_index,
                });
            }
            EventOperation::LoopRequest {
                base,
                request,
                cross_thread,
            } => match bases.get_mut(base) {
                Some(state) if !state.freed => {
                    if *cross_thread && state.profile.threads == ThreadMode::Disabled {
                        diagnostics.insert(EventDiagnostic::Q704ThreadInitialization {
                            operation: operation_index,
                            base: *base,
                        });
                    } else {
                        state.loop_request = Some(*request);
                        observed = ObservedKind::Synchronization;
                    }
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
                        operation: operation_index,
                    });
                }
            },
            EventOperation::DropArgument { argument } => {
                let referenced = events.values().any(|event| {
                    event.lifecycle != EventLifecycle::Freed && event.summary.argument == *argument
                }) || bevs
                    .values()
                    .any(|bev| !bev.freed && bev.argument == *argument);
                if referenced || !live_arguments.remove(argument) {
                    diagnostics.insert(EventDiagnostic::Q702ArgumentLifetime {
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
                        diagnostics.insert(EventDiagnostic::Q711LoopExitCustody {
                            operation: operation_index,
                        });
                    } else {
                        state.freed = true;
                        observed = ObservedKind::Teardown;
                    }
                }
                _ => {
                    diagnostics.insert(EventDiagnostic::Q701UseAfterFree {
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
            diagnostics.insert(EventDiagnostic::Q707OwnedObligation {
                operation: program.operations.len(),
            });
        }
    }
    for bev in bevs.values() {
        if !bev.freed {
            diagnostics.insert(EventDiagnostic::Q707OwnedObligation {
                operation: program.operations.len(),
            });
        }
    }
    for base in bases.values() {
        if !base.freed {
            diagnostics.insert(EventDiagnostic::Q711LoopExitCustody {
                operation: program.operations.len(),
            });
        }
    }

    if diagnostics.is_empty() {
        Ok(EventCertificate {
            checked_operations: program.operations.len(),
            freed_events,
            freed_bufferevents,
            closed_descriptors,
            dropped_arguments,
            trace,
        })
    } else {
        Err(diagnostics.into_iter().collect())
    }
}

fn context_authority(
    context: CallbackContext,
    payload: PayloadId,
    kind: PayloadAccessKind,
    events: &BTreeMap<EventKey, EventState>,
    bevs: &BTreeMap<BuffereventKey, BuffereventState>,
    bases: &BTreeMap<EventBaseId, BaseState>,
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
        Err(EventDiagnostic::Q712TraceRefinement {
            observation: first_difference,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibeventSourceEvidence {
    pub root: PathBuf,
    pub commit: String,
    pub checked_files: Vec<PathBuf>,
}

#[derive(Debug)]
pub enum LibeventSourceError {
    GitUnavailable,
    WrongCommit { actual: String },
    DirtyCheckout,
    MissingContract { file: PathBuf, needle: &'static str },
    Io(std::io::Error),
}

impl fmt::Display for LibeventSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LibeventSourceError {}

impl From<std::io::Error> for LibeventSourceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Verify the pinned checkout and the upstream source contracts relied on by
/// the initial event model.  This is a source audit, not a proof about C.
pub fn verify_pinned_libevent_source(
    root: impl AsRef<Path>,
) -> Result<LibeventSourceEvidence, LibeventSourceError> {
    let root = root.as_ref();
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|_| LibeventSourceError::GitUnavailable)?;
    if !output.status.success() {
        return Err(LibeventSourceError::GitUnavailable);
    }
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if actual != PINNED_LIBEVENT_COMMIT {
        return Err(LibeventSourceError::WrongCommit { actual });
    }
    let status = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .map_err(|_| LibeventSourceError::GitUnavailable)?;
    if !status.status.success() {
        return Err(LibeventSourceError::GitUnavailable);
    }
    if !status.stdout.is_empty() {
        return Err(LibeventSourceError::DirtyCheckout);
    }
    let contracts = [
        (
            "include/event2/event.h",
            &[
                "int event_del_noblock(struct event *ev);",
                "int event_del_block(struct event *ev);",
                "int event_finalize(unsigned, struct event *",
                "void event_active(struct event *ev, int res, short ncalls);",
            ][..],
        ),
        (
            "event.c",
            &[
                "if (base->running_loop)",
                "blocking != EVENT_DEL_NOBLOCK",
                "if (ev->ev_flags & EVLIST_FINALIZING)",
                "event_process_active(base)",
            ][..],
        ),
        (
            "bufferevent.c",
            &[
                "UNLOCK_CALLBACKS requires DEFER_CALLBACKS",
                "bufferevent_run_deferred_callbacks_unlocked",
                "bufferevent_enable_locking_",
            ][..],
        ),
        (
            "include/event2/thread.h",
            &["int evthread_use_pthreads(void);"][..],
        ),
        (
            "epoll.c",
            &["const struct eventop epollops", "epoll_wait("][..],
        ),
        (
            "kqueue.c",
            &[
                "const struct eventop kqops",
                "if ((kq = kqueue()) == -1)",
                "res = kevent(kqop->kq",
            ][..],
        ),
    ];
    let mut checked_files = Vec::new();
    for (relative, needles) in contracts {
        let path = root.join(relative);
        let source = std::fs::read_to_string(&path)?;
        for &needle in needles {
            if !source.contains(needle) {
                return Err(LibeventSourceError::MissingContract { file: path, needle });
            }
        }
        checked_files.push(PathBuf::from(relative));
    }
    Ok(LibeventSourceEvidence {
        root: root.to_path_buf(),
        commit: actual,
        checked_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(id: u32) -> EventKey {
        EventKey::new(id, 1)
    }

    fn summary(argument: u32, reason: ActivationReason) -> CallbackSummary {
        CallbackSummary {
            argument,
            argument_mode: ArgumentMode::Owned,
            trigger_reasons: BTreeSet::from([reason]),
            payload_authority: BTreeMap::from([(7, PayloadAccessKind::Write)]),
            may_defer: false,
            may_spawn_tasks: false,
        }
    }

    fn one_shot(base: u32, event: EventKey, argument: u32) -> EventProgram {
        EventProgram {
            live_arguments: BTreeSet::from([argument]),
            operations: vec![
                EventOperation::CreateBase {
                    base,
                    profile: LibeventBuildProfile::LINUX_EPOLL,
                },
                EventOperation::Assign {
                    event,
                    base,
                    descriptor: Some(3),
                    flags: EventFlags::new(EventFlags::READ),
                    priority: 0,
                    summary: summary(argument, ActivationReason::Read),
                },
                EventOperation::Add { event },
                EventOperation::BeginDispatch {
                    base,
                    dispatcher: 10,
                },
                EventOperation::Activate {
                    event,
                    reason: ActivationReason::Read,
                    cross_thread: false,
                },
                EventOperation::CallbackEnter {
                    event,
                    reason: ActivationReason::Read,
                },
                EventOperation::Access {
                    context: CallbackContext::Event(event),
                    payload: 7,
                    kind: PayloadAccessKind::Write,
                    synchronized: false,
                },
                EventOperation::CallbackExit { event },
                EventOperation::EndDispatch { base },
                EventOperation::FreeEvent { event },
                EventOperation::FreeBase { base },
            ],
        }
    }

    fn has(diagnostics: &[EventDiagnostic], predicate: impl Fn(&EventDiagnostic) -> bool) -> bool {
        diagnostics.iter().any(predicate)
    }

    #[test]
    fn one_shot_callback_has_live_authority_and_quiescent_teardown() {
        let certificate = check_event_program(&one_shot(1, key(1), 9)).unwrap();
        assert_eq!(certificate.freed_events, BTreeSet::from([key(1)]));
        assert_eq!(certificate.dropped_arguments, BTreeSet::from([9]));
    }

    #[test]
    fn persistent_recursive_activation_repends_without_fairness_claim() {
        let event = key(1);
        let mut program = one_shot(1, event, 9);
        if let EventOperation::Assign { flags, summary, .. } = &mut program.operations[1] {
            *flags = EventFlags::new(EventFlags::READ | EventFlags::PERSIST);
            summary.trigger_reasons.insert(ActivationReason::Explicit);
        }
        program.operations.insert(
            7,
            EventOperation::Activate {
                event,
                reason: ActivationReason::Explicit,
                cross_thread: false,
            },
        );
        program.operations.insert(
            9,
            EventOperation::CallbackEnter {
                event,
                reason: ActivationReason::Explicit,
            },
        );
        program
            .operations
            .insert(10, EventOperation::CallbackExit { event });
        program.operations.insert(
            11,
            EventOperation::Delete {
                event,
                mode: DeleteMode::Blocking,
            },
        );
        assert!(check_event_program(&program).is_ok());
    }

    #[test]
    fn nonblocking_delete_requires_callback_exit_and_finalization() {
        let event = key(1);
        let mut bad = one_shot(1, event, 9);
        bad.operations.insert(
            7,
            EventOperation::Delete {
                event,
                mode: DeleteMode::Nonblocking,
            },
        );
        bad.operations.remove(8); // remove callback exit; subsequent teardown must fail
        assert!(has(
            &check_event_program(&bad).unwrap_err(),
            |error| matches!(error, EventDiagnostic::Q706NotQuiescent { .. })
        ));

        let mut good = one_shot(1, event, 9);
        good.operations.insert(
            7,
            EventOperation::Delete {
                event,
                mode: DeleteMode::Nonblocking,
            },
        );
        good.operations
            .insert(9, EventOperation::Finalize { event });
        assert!(check_event_program(&good).is_ok());
    }

    #[test]
    fn stale_generation_and_argument_use_after_free_are_rejected() {
        let event = key(1);
        let mut program = one_shot(1, event, 9);
        program.operations.insert(
            10,
            EventOperation::Activate {
                event: EventKey::new(1, 0),
                reason: ActivationReason::Read,
                cross_thread: false,
            },
        );
        assert!(has(
            &check_event_program(&program).unwrap_err(),
            |error| matches!(
                error,
                EventDiagnostic::Q700InvalidLifecycle { .. }
                    | EventDiagnostic::Q701UseAfterFree { .. }
            )
        ));
    }

    #[test]
    fn same_base_cannot_enter_two_callbacks_concurrently() {
        let first = key(1);
        let second = key(2);
        let mut program = one_shot(1, first, 9);
        program.live_arguments.insert(10);
        program.operations.insert(
            3,
            EventOperation::Assign {
                event: second,
                base: 1,
                descriptor: Some(4),
                flags: EventFlags::new(EventFlags::READ),
                priority: 0,
                summary: summary(10, ActivationReason::Read),
            },
        );
        program
            .operations
            .insert(4, EventOperation::Add { event: second });
        program.operations.insert(
            7,
            EventOperation::Activate {
                event: second,
                reason: ActivationReason::Read,
                cross_thread: false,
            },
        );
        program.operations.insert(
            9,
            EventOperation::CallbackEnter {
                event: second,
                reason: ActivationReason::Read,
            },
        );
        program
            .operations
            .insert(12, EventOperation::FreeEvent { event: second });
        assert!(has(
            &check_event_program(&program).unwrap_err(),
            |error| matches!(error, EventDiagnostic::Q706NotQuiescent { .. })
        ));
    }

    #[test]
    fn cross_base_conflict_needs_explicit_synchronization() {
        let first = key(1);
        let second = key(2);
        let mut program = EventProgram {
            live_arguments: BTreeSet::from([9, 10]),
            operations: Vec::new(),
        };
        for (base, event, argument) in [(1, first, 9), (2, second, 10)] {
            program.operations.extend([
                EventOperation::CreateBase {
                    base,
                    profile: LibeventBuildProfile::LINUX_EPOLL,
                },
                EventOperation::Assign {
                    event,
                    base,
                    descriptor: None,
                    flags: EventFlags::new(EventFlags::TIMEOUT),
                    priority: 0,
                    summary: summary(argument, ActivationReason::Timeout),
                },
                EventOperation::Add { event },
                EventOperation::BeginDispatch {
                    base,
                    dispatcher: base,
                },
                EventOperation::Activate {
                    event,
                    reason: ActivationReason::Timeout,
                    cross_thread: false,
                },
                EventOperation::CallbackEnter {
                    event,
                    reason: ActivationReason::Timeout,
                },
                EventOperation::Access {
                    context: CallbackContext::Event(event),
                    payload: 7,
                    kind: PayloadAccessKind::Write,
                    synchronized: false,
                },
                EventOperation::CallbackExit { event },
                EventOperation::EndDispatch { base },
                EventOperation::FreeEvent { event },
                EventOperation::FreeBase { base },
            ]);
        }
        assert!(has(
            &check_event_program(&program).unwrap_err(),
            |error| matches!(error, EventDiagnostic::Q705ConflictingAccess { .. })
        ));
        if let EventOperation::Access { synchronized, .. } = &mut program.operations[17] {
            *synchronized = true;
        }
        assert!(check_event_program(&program).is_ok());
    }

    #[test]
    fn cross_thread_activation_requires_supported_thread_setup() {
        let mut program = one_shot(1, key(1), 9);
        if let EventOperation::CreateBase { profile, .. } = &mut program.operations[0] {
            profile.threads = ThreadMode::Disabled;
        }
        if let EventOperation::Activate { cross_thread, .. } = &mut program.operations[4] {
            *cross_thread = true;
        }
        assert!(has(
            &check_event_program(&program).unwrap_err(),
            |error| matches!(
                error,
                EventDiagnostic::Q709UnknownProfile { .. }
                    | EventDiagnostic::Q704ThreadInitialization { .. }
            )
        ));
    }

    #[test]
    fn bufferevent_modes_watermarks_and_close_custody_are_checked() {
        let bev = BuffereventKey::new(1, 1);
        let program = EventProgram {
            live_arguments: BTreeSet::from([9]),
            operations: vec![
                EventOperation::CreateBase {
                    base: 1,
                    profile: LibeventBuildProfile::LINUX_EPOLL,
                },
                EventOperation::CreateBufferevent {
                    bev,
                    base: 1,
                    descriptor: 3,
                    argument: 9,
                    close_on_free: true,
                    thread_safe: true,
                    deferred: true,
                    unlock_callbacks: true,
                    payload_authority: BTreeMap::from([(7, PayloadAccessKind::Write)]),
                },
                EventOperation::SetWatermark {
                    bev,
                    read: true,
                    low: 4,
                    high: 16,
                },
                EventOperation::BufferOutput { bev, bytes: 8 },
                EventOperation::DrainOutput { bev, bytes: 8 },
                EventOperation::BeginDispatch {
                    base: 1,
                    dispatcher: 1,
                },
                EventOperation::BuffereventCallbackEnter { bev, caller: 8 },
                EventOperation::Access {
                    context: CallbackContext::Bufferevent(bev),
                    payload: 7,
                    kind: PayloadAccessKind::Write,
                    synchronized: true,
                },
                EventOperation::BuffereventCallbackExit { bev },
                EventOperation::EndDispatch { base: 1 },
                EventOperation::FreeBufferevent { bev },
                EventOperation::FreeBase { base: 1 },
            ],
        };
        let certificate = check_event_program(&program).unwrap();
        assert_eq!(certificate.closed_descriptors, BTreeSet::from([3]));
    }

    #[test]
    fn unlock_callbacks_without_deferral_and_pending_output_fail() {
        let bev = BuffereventKey::new(1, 1);
        let mut program = EventProgram {
            live_arguments: BTreeSet::from([9]),
            operations: vec![
                EventOperation::CreateBase {
                    base: 1,
                    profile: LibeventBuildProfile::LINUX_EPOLL,
                },
                EventOperation::CreateBufferevent {
                    bev,
                    base: 1,
                    descriptor: 3,
                    argument: 9,
                    close_on_free: true,
                    thread_safe: true,
                    deferred: false,
                    unlock_callbacks: true,
                    payload_authority: BTreeMap::new(),
                },
                EventOperation::FreeBufferevent { bev },
                EventOperation::FreeBase { base: 1 },
            ],
        };
        assert!(has(
            &check_event_program(&program).unwrap_err(),
            |error| matches!(error, EventDiagnostic::Q710BuffereventContract { .. })
        ));
        if let EventOperation::CreateBufferevent { deferred, .. } = &mut program.operations[1] {
            *deferred = true;
        }
        program
            .operations
            .insert(2, EventOperation::BufferOutput { bev, bytes: 1 });
        assert!(has(
            &check_event_program(&program).unwrap_err(),
            |error| matches!(error, EventDiagnostic::Q707OwnedObligation { .. })
        ));
    }

    #[test]
    fn priority_exploration_is_nondeterministic_only_within_a_priority() {
        let a = key(1);
        let b = key(2);
        let c = key(3);
        let schedules = explore_activation_orders(&[(a, 1), (b, 0), (c, 1)], 8);
        assert_eq!(schedules, vec![vec![b, a, c], vec![b, c, a]]);
    }

    #[test]
    fn unsupported_symbols_and_profiles_fail_closed() {
        assert_eq!(
            classify_libevent_symbol("evhttp_new"),
            ProfileDisposition::Unknown
        );
        assert_eq!(
            classify_libevent_symbol("private_event_magic"),
            ProfileDisposition::Unknown
        );
        let mut program = one_shot(1, key(1), 9);
        if let EventOperation::CreateBase { profile, .. } = &mut program.operations[0] {
            profile.openssl = true;
        }
        assert!(has(
            &check_event_program(&program).unwrap_err(),
            |error| matches!(error, EventDiagnostic::Q709UnknownProfile { .. })
        ));
    }

    #[test]
    fn priority_readiness_timing_and_trace_order_are_not_synchronization() {
        for evidence in [
            InvalidOrderingEvidence::Priority,
            InvalidOrderingEvidence::Readiness,
            InvalidOrderingEvidence::Timing,
            InvalidOrderingEvidence::TraceOrder,
        ] {
            let mut program = one_shot(1, key(1), 9);
            program
                .operations
                .insert(7, EventOperation::AssumeOrdering { evidence });
            assert!(has(
                &check_event_program(&program).unwrap_err(),
                |error| matches!(error, EventDiagnostic::Q708OrderingAssumption { .. })
            ));
        }
    }

    #[test]
    fn adapter_observation_must_refine_the_checked_trace() {
        let certificate = check_event_program(&one_shot(1, key(1), 9)).unwrap();
        let observation = certificate
            .trace
            .iter()
            .filter_map(|event| (event.kind != ObservedKind::Lifecycle).then_some(event.kind))
            .collect::<Vec<_>>();
        assert!(check_observation_refinement(&certificate, &observation).is_ok());
        let mut wrong = observation;
        wrong.push(ObservedKind::CallbackEnter);
        assert!(matches!(
            check_observation_refinement(&certificate, &wrong),
            Err(EventDiagnostic::Q712TraceRefinement { .. })
        ));
    }
}
