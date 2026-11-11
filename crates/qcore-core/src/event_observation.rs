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

//! Identity-bearing observations for a bounded event-dispatch execution.
//!
//! This module is separate from `event_reactor`'s compatibility
//! `ObservedKind` comparison.  A record retains the identities needed to bind a
//! native callback interval to one checked event.  The first accepted profile
//! is intentionally narrow: one dispatcher-owned buffer, one timed callback,
//! no cross-context operation and therefore no synchronization edge.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservationIdentity {
    pub id: u64,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedActivationReason {
    Read,
    Write,
    Timeout,
    Signal,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObservationPosition {
    pub buffer: u64,
    pub local_sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SynchronizationEdge {
    pub from: ObservationPosition,
    pub to: ObservationPosition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventObservationAction {
    BaseCreate {
        base: ObservationIdentity,
        backend: String,
    },
    EventNew {
        base: ObservationIdentity,
        event: ObservationIdentity,
        argument: ObservationIdentity,
        callback: String,
        flags: u16,
        priority: u8,
        result: i32,
    },
    EventAdd {
        event: ObservationIdentity,
        result: i32,
    },
    DispatchBegin {
        base: ObservationIdentity,
    },
    Activate {
        event: ObservationIdentity,
        reason: ObservedActivationReason,
    },
    CallbackEnter {
        event: ObservationIdentity,
        argument: ObservationIdentity,
        callback: String,
        reason: ObservedActivationReason,
    },
    CallbackExit {
        event: ObservationIdentity,
        argument: ObservationIdentity,
        callback: String,
    },
    DispatchEnd {
        base: ObservationIdentity,
        result: i32,
    },
    EventFree {
        event: ObservationIdentity,
    },
    ArgumentEnd {
        argument: ObservationIdentity,
    },
    BaseFree {
        base: ObservationIdentity,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventObservationRecord {
    pub run: u64,
    pub buffer: u64,
    pub local_sequence: u64,
    pub dispatcher: u64,
    pub action: EventObservationAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventObservationTrace {
    pub records: Vec<EventObservationRecord>,
    pub synchronization: Vec<SynchronizationEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedWaitObservationContext {
    pub run: u64,
    pub buffer: u64,
    pub dispatcher: u64,
    pub base: ObservationIdentity,
    pub event: ObservationIdentity,
    pub argument: ObservationIdentity,
    pub callback: String,
    pub backend: String,
    pub flags: u16,
    pub priority: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedWaitObservationCertificate {
    context: TimedWaitObservationContext,
    checked_records: usize,
}

impl TimedWaitObservationCertificate {
    #[must_use]
    pub const fn context(&self) -> &TimedWaitObservationContext {
        &self.context
    }

    #[must_use]
    pub const fn checked_records(&self) -> usize {
        self.checked_records
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventObservationError {
    EmptyIdentity,
    UnsupportedSynchronization,
    UnsupportedMultiContext,
    NoncontiguousSequence { expected: u64, recorded: u64 },
    UnexpectedRecord { index: usize },
}

fn identities_are_nonzero(context: &TimedWaitObservationContext) -> bool {
    context.run != 0
        && context.buffer != 0
        && context.dispatcher != 0
        && context.base.id != 0
        && context.base.generation != 0
        && context.event.id != 0
        && context.event.generation != 0
        && context.argument.id != 0
        && context.argument.generation != 0
        && !context.callback.is_empty()
        && !context.backend.is_empty()
}

/// Validate one complete observed timed-wait callback execution.
///
/// The exact eleven-record shape is the supported profile.  Every record is
/// consumed, and every identity-bearing field is compared with the expected
/// context.  Synchronization edges and additional dispatchers are rejected:
/// this profile has no cross-context behavior from which such an edge could be
/// derived.
pub fn check_timed_wait_observation(
    trace: &EventObservationTrace,
    expected: &TimedWaitObservationContext,
) -> Result<TimedWaitObservationCertificate, EventObservationError> {
    if !identities_are_nonzero(expected) {
        return Err(EventObservationError::EmptyIdentity);
    }
    if !trace.synchronization.is_empty() {
        return Err(EventObservationError::UnsupportedSynchronization);
    }
    let contexts = trace
        .records
        .iter()
        .map(|record| (record.run, record.buffer, record.dispatcher))
        .collect::<BTreeSet<_>>();
    if contexts != BTreeSet::from([(expected.run, expected.buffer, expected.dispatcher)]) {
        return Err(EventObservationError::UnsupportedMultiContext);
    }
    for (index, record) in trace.records.iter().enumerate() {
        let sequence = (index as u64) + 1;
        if record.local_sequence != sequence {
            return Err(EventObservationError::NoncontiguousSequence {
                expected: sequence,
                recorded: record.local_sequence,
            });
        }
    }

    use EventObservationAction as A;
    let expected_actions = [
        A::BaseCreate {
            base: expected.base,
            backend: expected.backend.clone(),
        },
        A::EventNew {
            base: expected.base,
            event: expected.event,
            argument: expected.argument,
            callback: expected.callback.clone(),
            flags: expected.flags,
            priority: expected.priority,
            result: 0,
        },
        A::EventAdd {
            event: expected.event,
            result: 0,
        },
        A::DispatchBegin {
            base: expected.base,
        },
        A::Activate {
            event: expected.event,
            reason: ObservedActivationReason::Timeout,
        },
        A::CallbackEnter {
            event: expected.event,
            argument: expected.argument,
            callback: expected.callback.clone(),
            reason: ObservedActivationReason::Timeout,
        },
        A::CallbackExit {
            event: expected.event,
            argument: expected.argument,
            callback: expected.callback.clone(),
        },
        A::DispatchEnd {
            base: expected.base,
            result: 0,
        },
        A::EventFree {
            event: expected.event,
        },
        A::ArgumentEnd {
            argument: expected.argument,
        },
        A::BaseFree {
            base: expected.base,
        },
    ];
    if trace.records.len() != expected_actions.len() {
        return Err(EventObservationError::UnexpectedRecord {
            index: trace.records.len().min(expected_actions.len()),
        });
    }
    for (index, (record, action)) in trace.records.iter().zip(expected_actions).enumerate() {
        if record.action != action {
            return Err(EventObservationError::UnexpectedRecord { index });
        }
    }
    Ok(TimedWaitObservationCertificate {
        context: expected.clone(),
        checked_records: trace.records.len(),
    })
}

#[cfg(test)]
#[path = "../tests/unit/event_observation.rs"]
mod tests;
