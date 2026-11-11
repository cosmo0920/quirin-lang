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

//! Strict wire boundary for the bounded Cycle 18 timed-wait observation.

use std::collections::{BTreeMap, BTreeSet};

use qcore_core::event_observation::{
    EventObservationAction, EventObservationRecord, EventObservationTrace, ObservationIdentity,
    ObservedActivationReason, TimedWaitObservationContext, check_timed_wait_observation,
};
use qcore_schemas::sha256_hex;
use serde::Deserialize;

const PROFILE: &str = "cycle18.timed_wait.single_dispatcher.v1";
const OBSERVATION_PATH: &str = "observations.jsonl";
const RECORD_COUNT: usize = 11;
const MAX_OBSERVATION_BYTES: usize = 64 * 1024;
const MAX_BINDING_BYTES: usize = 32 * 1024;

#[derive(Debug)]
pub struct DecodedTimedWaitObservation {
    /// Profile actually carried by the validated wire binding.
    pub semantic_profile: String,
    pub trace: EventObservationTrace,
    pub context: TimedWaitObservationContext,
    pub source_apis: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireIdentity {
    id: u64,
    generation: u32,
}

impl From<WireIdentity> for ObservationIdentity {
    fn from(value: WireIdentity) -> Self {
        Self {
            id: value.id,
            generation: value.generation,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireReason {
    Read,
    Write,
    Timeout,
    Signal,
    Explicit,
}

impl From<WireReason> for ObservedActivationReason {
    fn from(value: WireReason) -> Self {
        match value {
            WireReason::Read => Self::Read,
            WireReason::Write => Self::Write,
            WireReason::Timeout => Self::Timeout,
            WireReason::Signal => Self::Signal,
            WireReason::Explicit => Self::Explicit,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WireAction {
    BaseCreate {
        base: WireIdentity,
        backend: String,
    },
    EventNew {
        base: WireIdentity,
        event: WireIdentity,
        argument: WireIdentity,
        callback: String,
        flags: u16,
        priority: u8,
        result: i32,
    },
    EventAdd {
        event: WireIdentity,
        result: i32,
    },
    DispatchBegin {
        base: WireIdentity,
    },
    Activate {
        event: WireIdentity,
        reason: WireReason,
    },
    CallbackEnter {
        event: WireIdentity,
        argument: WireIdentity,
        callback: String,
        reason: WireReason,
    },
    CallbackExit {
        event: WireIdentity,
        argument: WireIdentity,
        callback: String,
    },
    DispatchEnd {
        base: WireIdentity,
        result: i32,
    },
    EventFree {
        event: WireIdentity,
    },
    ArgumentEnd {
        argument: WireIdentity,
    },
    BaseFree {
        base: WireIdentity,
    },
}

impl From<WireAction> for EventObservationAction {
    fn from(value: WireAction) -> Self {
        match value {
            WireAction::BaseCreate { base, backend } => Self::BaseCreate {
                base: base.into(),
                backend,
            },
            WireAction::EventNew {
                base,
                event,
                argument,
                callback,
                flags,
                priority,
                result,
            } => Self::EventNew {
                base: base.into(),
                event: event.into(),
                argument: argument.into(),
                callback,
                flags,
                priority,
                result,
            },
            WireAction::EventAdd { event, result } => Self::EventAdd {
                event: event.into(),
                result,
            },
            WireAction::DispatchBegin { base } => Self::DispatchBegin { base: base.into() },
            WireAction::Activate { event, reason } => Self::Activate {
                event: event.into(),
                reason: reason.into(),
            },
            WireAction::CallbackEnter {
                event,
                argument,
                callback,
                reason,
            } => Self::CallbackEnter {
                event: event.into(),
                argument: argument.into(),
                callback,
                reason: reason.into(),
            },
            WireAction::CallbackExit {
                event,
                argument,
                callback,
            } => Self::CallbackExit {
                event: event.into(),
                argument: argument.into(),
                callback,
            },
            WireAction::DispatchEnd { base, result } => Self::DispatchEnd {
                base: base.into(),
                result,
            },
            WireAction::EventFree { event } => Self::EventFree {
                event: event.into(),
            },
            WireAction::ArgumentEnd { argument } => Self::ArgumentEnd {
                argument: argument.into(),
            },
            WireAction::BaseFree { base } => Self::BaseFree { base: base.into() },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRecord {
    schema_version: u32,
    run: u64,
    buffer: u64,
    local_sequence: u64,
    dispatcher: u64,
    action: WireAction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireContext {
    run: u64,
    buffer: u64,
    dispatcher: u64,
    base: WireIdentity,
    event: WireIdentity,
    argument: WireIdentity,
    callback: String,
    backend: String,
    flags: u16,
    priority: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObservationBinding {
    path: String,
    sha256: String,
    records: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSourceApi {
    api: String,
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBindings {
    schema_version: u32,
    profile: String,
    observations: WireObservationBinding,
    context: WireContext,
    source_apis: Vec<WireSourceApi>,
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Decode and validate the complete wire pair. No partially checked value is
/// returned: digest, schema, bounded shape, sources and semantic trace all
/// have to agree first.
pub fn decode_timed_wait_observation(
    observations: &[u8],
    bindings: &[u8],
) -> Result<DecodedTimedWaitObservation, String> {
    if observations.is_empty() || observations.len() > MAX_OBSERVATION_BYTES {
        return Err("observation bytes are empty or exceed the bound".into());
    }
    if bindings.is_empty() || bindings.len() > MAX_BINDING_BYTES {
        return Err("binding bytes are empty or exceed the bound".into());
    }
    let binding: WireBindings = serde_json::from_slice(bindings)
        .map_err(|error| format!("invalid observation bindings: {error}"))?;
    if binding.schema_version != 1 || binding.profile != PROFILE {
        return Err("unsupported observation binding schema or profile".into());
    }
    if binding.observations.path != OBSERVATION_PATH
        || binding.observations.records != RECORD_COUNT
        || !valid_digest(&binding.observations.sha256)
        || binding.observations.sha256 != sha256_hex(observations)
    {
        return Err("observation payload binding mismatch".into());
    }

    let expected_apis = BTreeSet::from([
        ("mk_event_wait_2", "mk_core/mk_event.c"),
        ("_mk_event_wait_2", "mk_core/mk_event_libevent.c"),
        ("cb_wait_2_timeout", "mk_core/mk_event_libevent.c"),
    ]);
    let actual_apis = binding
        .source_apis
        .iter()
        .map(|entry| (entry.api.as_str(), entry.path.as_str()))
        .collect::<BTreeSet<_>>();
    if binding.source_apis.len() != 3 || actual_apis != expected_apis {
        return Err("source API binding shape mismatch".into());
    }
    let mut source_apis = BTreeMap::new();
    for entry in binding.source_apis {
        if !valid_digest(&entry.sha256) {
            return Err("invalid source digest".into());
        }
        if let Some(previous) = source_apis.insert(entry.path, entry.sha256.clone())
            && previous != entry.sha256
        {
            return Err("one source path has conflicting digests".into());
        }
    }

    let mut records = Vec::with_capacity(RECORD_COUNT);
    for (index, line) in observations.split(|byte| *byte == b'\n').enumerate() {
        if line.is_empty() {
            if index + 1 == observations.split(|byte| *byte == b'\n').count() {
                continue;
            }
            return Err("empty observation record".into());
        }
        if records.len() == RECORD_COUNT {
            return Err("observation record count exceeds the bound".into());
        }
        let record: WireRecord = serde_json::from_slice(line)
            .map_err(|error| format!("invalid observation record: {error}"))?;
        if record.schema_version != 1 {
            return Err("unsupported observation record schema".into());
        }
        records.push(EventObservationRecord {
            run: record.run,
            buffer: record.buffer,
            local_sequence: record.local_sequence,
            dispatcher: record.dispatcher,
            action: record.action.into(),
        });
    }
    if records.len() != RECORD_COUNT {
        return Err("observation record count mismatch".into());
    }
    let context = TimedWaitObservationContext {
        run: binding.context.run,
        buffer: binding.context.buffer,
        dispatcher: binding.context.dispatcher,
        base: binding.context.base.into(),
        event: binding.context.event.into(),
        argument: binding.context.argument.into(),
        callback: binding.context.callback,
        backend: binding.context.backend,
        flags: binding.context.flags,
        priority: binding.context.priority,
    };
    let trace = EventObservationTrace {
        records,
        synchronization: Vec::new(),
    };
    check_timed_wait_observation(&trace, &context)
        .map_err(|error| format!("observation trace rejected: {error:?}"))?;
    Ok(DecodedTimedWaitObservation {
        semantic_profile: binding.profile,
        trace,
        context,
        source_apis,
    })
}
