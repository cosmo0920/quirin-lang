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

//! Cycle 18 bounded Libevent event-driven concurrency model.
//!
//! The model is deliberately fail closed.  It accepts only the public API
//! surface, build profiles and pinned source constructs declared by the
//! `libevent` source-set profile (`contract/profiles/libevent/profile.json`,
//! W2), one dispatcher per event base, explicit callback summaries, and
//! explicit synchronization evidence for conflicting cross-context payload
//! accesses.  Trace order alone is never treated as happens-before evidence.
//!
//! Before W2 the API disposition table, the accepted-build matrix and the
//! pinned commit plus per-file needle audit were Rust constants in this file.
//! They are now data; nothing about the acceptance rules changed, and an
//! absent profile grants nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use qcore_contract::profile_loader::{
    FileAuditDiagnostic, PinDiagnostic, load_audited_bundle, verify_pin,
};
use qcore_schemas::profile::{
    ApiDisposition, BuildProfileSelector, LibraryProfile, SourcePin, WitnessCheck,
};

pub const LIBEVENT_SUBMODULE_PATH: &str = "third_party/libevent";

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

impl EventBackend {
    /// The name a profile's [`BuildProfileSelector::backend`] uses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Epoll => "epoll",
            Self::Poll => "poll",
            Self::Select => "select",
            Self::Kqueue => "kqueue",
            Self::Iocp => "iocp",
        }
    }
}

impl ThreadMode {
    /// The name a profile's [`BuildProfileSelector::threads`] uses. Disabled
    /// threading has no name: no selector can match it, so it stays Unknown.
    #[must_use]
    pub const fn as_str(self) -> Option<&'static str> {
        match self {
            Self::Disabled => None,
            Self::Pthreads => Some("pthreads"),
            Self::WindowsThreads => Some("windows_threads"),
        }
    }
}

impl LibeventBuildProfile {
    pub const LINUX_EPOLL: Self = Self {
        backend: EventBackend::Epoll,
        threads: ThreadMode::Pthreads,
        openssl: false,
        debug_mode: true,
    };

    /// Accepted only when some selector in the loaded profile's
    /// `accepted_builds` matches exactly and none of its forbidden features is
    /// enabled. An empty selector list accepts nothing (W2 replaced the
    /// hardcoded acceptance matrix; absence is never acceptance).
    #[must_use]
    pub fn accepted(self, accepted_builds: &[BuildProfileSelector]) -> bool {
        let Some(threads) = self.threads.as_str() else {
            return false;
        };
        accepted_builds.iter().any(|selector| {
            selector.backend == self.backend.as_str()
                && selector.threads == threads
                && selector
                    .forbid
                    .iter()
                    .all(|feature| match feature.as_str() {
                        "openssl" => !self.openssl,
                        "debug_mode" => !self.debug_mode,
                        _ => false,
                    })
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileDisposition {
    Accepted,
    Unknown,
}

/// Classify a symbol against the API disposition table a source-set profile
/// declares (W2 replaced the 45-entry Rust table with
/// `contract/profiles/libevent/profile.json`). Absence from the table and an
/// explicit `ApiDisposition::Unknown` classify identically: `Unknown`. There
/// is no accept-by-default and no `Safe` disposition.
#[must_use]
pub fn classify_libevent_symbol(
    dispositions: &BTreeMap<String, ApiDisposition>,
    symbol: &str,
) -> ProfileDisposition {
    match dispositions.get(symbol) {
        Some(ApiDisposition::Accepted) => ProfileDisposition::Accepted,
        Some(ApiDisposition::Unknown { .. }) | None => ProfileDisposition::Unknown,
    }
}

// Preserve provider-facing types while the generic kernel owns all model states.
pub use qcore_core::event_reactor::{
    ActivationReason, ArgumentMode, BuffereventId, BuffereventKey, CallbackArgumentId,
    CallbackContext, CallbackSummary, ClassifiedCloseFailure, ClosePremiseDisposition,
    ClosePremiseRecheckError, DeclaredCloseFailure, DeclaredClosePremise, DeclaredCloseRefusal,
    DeleteMode, DescriptorId, EventBaseId, EventCertificate, EventDiagnostic, EventFlags, EventId,
    EventKey, EventLifecycle, ExecutionContextId, Generation, InvalidOrderingEvidence, LoopControl,
    ObservedEvent, ObservedKind, PayloadAccessKind, PayloadId, check_observation_refinement,
    classify_declared_close_failure, explore_activation_orders,
};
// Concrete facades preserve enum variant imports and the original public
// program shape. Conversion is exhaustive and transfers every field unchanged.
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
    /// A successful `evutil_socketpair` creates two distinct owned
    /// descriptors. `None` is its failure branch and creates no custody.
    CreateSocketPair {
        descriptors: Option<(DescriptorId, DescriptorId)>,
    },
    /// `evutil_socket_geterror` borrows a descriptor (and on Unix expands to
    /// `errno`). It must not revive a descriptor already closed in this trace.
    GetSocketError {
        descriptor: DescriptorId,
    },
    /// `evutil_closesocket` consumes close custody only on its success branch.
    ///
    /// `declared_failure` carries the kernel's declared close-failure premise
    /// unchanged (card O03-REACTOR). This adapter establishes nothing about a
    /// platform or an `errno`: it transports whatever a caller declares, and
    /// `None` — what every existing caller passes — keeps the poisoned answer.
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventProgram {
    pub live_arguments: BTreeSet<CallbackArgumentId>,
    pub operations: Vec<EventOperation>,
}

impl From<EventOperation> for qcore_core::event_reactor::EventOperation<LibeventBuildProfile> {
    fn from(operation: EventOperation) -> Self {
        match operation {
            EventOperation::CreateBase { base, profile } => Self::CreateBase { base, profile },
            EventOperation::BeginDispatch { base, dispatcher } => {
                Self::BeginDispatch { base, dispatcher }
            }
            EventOperation::EndDispatch { base } => Self::EndDispatch { base },
            EventOperation::Assign {
                event,
                base,
                descriptor,
                flags,
                priority,
                summary,
            } => Self::Assign {
                event,
                base,
                descriptor,
                flags,
                priority,
                summary,
            },
            EventOperation::Add { event } => Self::Add { event },
            EventOperation::Reschedule { event } => Self::Reschedule { event },
            EventOperation::Activate {
                event,
                reason,
                cross_thread,
            } => Self::Activate {
                event,
                reason,
                cross_thread,
            },
            EventOperation::CallbackEnter { event, reason } => {
                Self::CallbackEnter { event, reason }
            }
            EventOperation::CallbackExit { event } => Self::CallbackExit { event },
            EventOperation::Delete { event, mode } => Self::Delete { event, mode },
            EventOperation::Finalize { event } => Self::Finalize { event },
            EventOperation::FreeEvent { event } => Self::FreeEvent { event },
            EventOperation::CreateSocketPair { descriptors } => {
                Self::CreateSocketPair { descriptors }
            }
            EventOperation::GetSocketError { descriptor } => Self::GetSocketError { descriptor },
            EventOperation::CloseSocket {
                descriptor,
                succeeds,
                declared_failure,
            } => Self::CloseSocket {
                descriptor,
                succeeds,
                declared_failure,
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
            } => Self::CreateBufferevent {
                bev,
                base,
                descriptor,
                argument,
                close_on_free,
                thread_safe,
                deferred,
                unlock_callbacks,
                payload_authority,
            },
            EventOperation::SetWatermark {
                bev,
                read,
                low,
                high,
            } => Self::SetWatermark {
                bev,
                read,
                low,
                high,
            },
            EventOperation::BufferOutput { bev, bytes } => Self::BufferOutput { bev, bytes },
            EventOperation::DrainOutput { bev, bytes } => Self::DrainOutput { bev, bytes },
            EventOperation::BuffereventCallbackEnter { bev, caller } => {
                Self::BuffereventCallbackEnter { bev, caller }
            }
            EventOperation::BuffereventCallbackExit { bev } => {
                Self::BuffereventCallbackExit { bev }
            }
            EventOperation::FreeBufferevent { bev } => Self::FreeBufferevent { bev },
            EventOperation::Access {
                context,
                payload,
                kind,
                synchronized,
            } => Self::Access {
                context,
                payload,
                kind,
                synchronized,
            },
            EventOperation::AssumeOrdering { evidence } => Self::AssumeOrdering { evidence },
            EventOperation::LoopRequest {
                base,
                request,
                cross_thread,
            } => Self::LoopRequest {
                base,
                request,
                cross_thread,
            },
            EventOperation::DropArgument { argument } => Self::DropArgument { argument },
            EventOperation::FreeBase { base } => Self::FreeBase { base },
        }
    }
}

impl From<EventProgram> for qcore_core::event_reactor::EventProgram<LibeventBuildProfile> {
    fn from(program: EventProgram) -> Self {
        Self {
            live_arguments: program.live_arguments,
            operations: program.operations.into_iter().map(Into::into).collect(),
        }
    }
}

impl qcore_core::event_reactor::ReactorProfile for LibeventBuildProfile {
    fn threading_enabled(self) -> bool {
        self.threads != ThreadMode::Disabled
    }
}

/// Preserve exact loaded-build matching before the generic model can create a base.
/// Empty or unsupported profile data accepts nothing; no permissive fallback.
pub fn check_event_program(
    program: &EventProgram,
    accepted_builds: &[BuildProfileSelector],
) -> Result<EventCertificate, Vec<EventDiagnostic>> {
    let accepted_profiles = program
        .operations
        .iter()
        .filter_map(|operation| match operation {
            EventOperation::CreateBase { profile, .. } if profile.accepted(accepted_builds) => {
                Some(*profile)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let model = program.clone().into();
    qcore_core::event_reactor::check_event_program(&model, &accepted_profiles)
}

/// Independently rederive the declared close-failure premises a certificate
/// rests on (card O03-REACTOR). The adapter contributes no authority: it
/// converts its own program to the kernel's and the kernel rederives every
/// premise from the post-state relation.
///
/// # Errors
///
/// [`ClosePremiseRecheckError`] — the certificate is refused, never repaired.
pub fn recheck_declared_close_premises(
    program: &EventProgram,
    certificate: &EventCertificate,
) -> Result<(), ClosePremiseRecheckError> {
    let model: qcore_core::event_reactor::EventProgram<LibeventBuildProfile> =
        program.clone().into();
    qcore_core::event_reactor::recheck_declared_close_premises(&model, certificate)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibeventSourceEvidence {
    pub root: PathBuf,
    pub commit: String,
    pub checked_files: Vec<PathBuf>,
    /// Witness names that passed, in declared order. Digest witnesses are
    /// established by the audited file's verified digest; substring witnesses
    /// re-check their declared needles.
    pub checked_witnesses: Vec<String>,
}

#[derive(Debug)]
pub enum LibeventSourceError {
    Pin(PinDiagnostic),
    FileAudit(Vec<FileAuditDiagnostic>),
    MissingContract { file: PathBuf, needle: String },
}

impl fmt::Display for LibeventSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LibeventSourceError {}

/// Verify the pinned checkout and the upstream source constructs relied on by
/// the event model, against the declared `libevent` library profile. This is a
/// source audit, not a proof about C.
///
/// W2 moved the commit pin, the 12-file inventory and the ~40 substring
/// needles out of this function into
/// `contract/profiles/libevent/profile.json`. The audit is strictly stronger
/// than before: every audited file is now digest-checked as well, and the
/// public-header needles (the constructs intended to survive deliberate pin
/// updates) are still re-checked.
pub fn verify_pinned_libevent_source(
    root: impl AsRef<Path>,
    pin: &SourcePin,
    profile: &LibraryProfile,
) -> Result<LibeventSourceEvidence, LibeventSourceError> {
    let root = root.as_ref();
    let commit = verify_pin("libevent", pin, root).map_err(LibeventSourceError::Pin)?;
    let bundle =
        load_audited_bundle(profile, root, &commit).map_err(LibeventSourceError::FileAudit)?;
    let mut checked_witnesses = Vec::new();
    for witness in &profile.source_witnesses {
        let contents = bundle.files.get(&witness.file).ok_or_else(|| {
            LibeventSourceError::MissingContract {
                file: root.join(&witness.file),
                needle: witness.name.clone(),
            }
        })?;
        if let WitnessCheck::Substrings { needles, .. } = &witness.check {
            for needle in needles {
                if !contents.contains(needle) {
                    return Err(LibeventSourceError::MissingContract {
                        file: root.join(&witness.file),
                        needle: needle.clone(),
                    });
                }
            }
        }
        checked_witnesses.push(witness.name.clone());
    }
    Ok(LibeventSourceEvidence {
        root: root.to_path_buf(),
        commit,
        checked_files: profile
            .audited_files
            .iter()
            .map(|audited| PathBuf::from(&audited.path))
            .collect(),
        checked_witnesses,
    })
}

#[cfg(test)]
#[path = "../../tests/unit/event_driven.rs"]
mod tests;
