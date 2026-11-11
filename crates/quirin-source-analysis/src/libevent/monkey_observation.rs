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

//! Validate the identity and lifetime projection of recorded Monkey executions.
//!
//! This checks supplied runtime observations, not C source semantics, callback
//! payload accesses, unobserved failure paths, or a protocol core certificate.

use std::collections::{BTreeMap, BTreeSet};

use crate::contract_observation::check_contract_facts as rule;
use serde::{Deserialize, Serialize};

pub const DESCRIPTOR_OBSERVATION_CONTRACTS: &str = include_str!(
    "../../../../providers/qcore-provider-libevent/contracts/monkey/descriptor_observation.qv"
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DescriptorKey {
    pub fd: i32,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAction {
    EventNew,
    EventAdd,
    EventDel,
    EventFree,
    CallbackEnter,
    CallbackExit,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationAction {
    Backend {
        backend: String,
    },
    Socketpair {
        result: i32,
        descriptors: Vec<DescriptorKey>,
    },
    DescriptorUse {
        descriptor: DescriptorKey,
        result: i64,
    },
    DescriptorClose {
        descriptor: DescriptorKey,
        result: i32,
    },
    Event {
        operation: EventAction,
        event: u64,
        callback: String,
        descriptor: Option<DescriptorKey>,
        result: i32,
    },
    DispatchBegin {
        result: i32,
    },
    DispatchEnd {
        result: i32,
    },
    BaseFree {
        result: i32,
    },
    Completed {
        result: i32,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MonkeyObservation {
    pub schema_version: u16,
    pub sequence: u64,
    pub base: u64,
    pub dispatcher: u64,
    pub thread: u64,
    pub action: ObservationAction,
}

/// Accepted runtime projection only; this is deliberately not a certificate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonkeyObservationReport {
    pub observations: usize,
    pub descriptor_acquisitions: usize,
    pub descriptor_reuses: usize,
    pub socketpair_failures: usize,
    pub descriptor_uses: usize,
    pub descriptor_closes: usize,
    pub callbacks: BTreeSet<String>,
    pub backends: BTreeMap<u64, String>,
}

struct EventState {
    base: u64,
    callback: String,
    descriptor: Option<DescriptorKey>,
    registered: bool,
    freed: bool,
}

#[derive(Default)]
struct Checker<'a> {
    rules: &'a str,
    bases: BTreeMap<u64, (u64, u64)>,
    seen_bases: BTreeMap<u64, (u64, u64)>,
    generations: BTreeMap<i32, u32>,
    live: BTreeMap<i32, (DescriptorKey, u64)>,
    events: BTreeMap<u64, EventState>,
    dispatch: Option<u64>,
    callback: Option<u64>,
    completed: bool,
}

impl Checker<'_> {
    fn live(&self, key: DescriptorKey, base: u64) -> Result<(), String> {
        rule(
            self.rules,
            "observation_use",
            &[(
                "live_identity_matches",
                i128::from(self.live.get(&key.fd) == Some(&(key, base))),
            )],
        )
    }

    fn step(
        &mut self,
        row: &MonkeyObservation,
        report: &mut MonkeyObservationReport,
    ) -> Result<(), String> {
        if self.completed
            || row.schema_version != 1
            || row.base == 0
            || row.dispatcher == 0
            || row.thread == 0
        {
            return Err("invalid observation envelope or record after completion".into());
        }
        if let ObservationAction::Backend { backend } = &row.action {
            if self.dispatch.is_some()
                || self
                    .seen_bases
                    .insert(row.base, (row.dispatcher, row.thread))
                    .is_some()
                || !matches!(
                    backend.as_str(),
                    "epoll" | "poll" | "select" | "kqueue" | "iocp"
                )
            {
                return Err("duplicate base or unsupported observed backend".into());
            }
            self.bases.insert(row.base, (row.dispatcher, row.thread));
            report.backends.insert(row.base, backend.clone());
            return Ok(());
        }
        if let ObservationAction::Completed { result } = row.action {
            rule(
                self.rules,
                "observation_complete",
                &[
                    ("result", result.into()),
                    ("seen_bases", self.seen_bases.len() as i128),
                    (
                        "final_scope_matches",
                        i128::from(
                            self.seen_bases.get(&row.base) == Some(&(row.dispatcher, row.thread)),
                        ),
                    ),
                    ("live_bases", self.bases.len() as i128),
                    ("live_descriptors", self.live.len() as i128),
                    (
                        "live_events",
                        self.events.values().filter(|event| !event.freed).count() as i128,
                    ),
                    ("dispatch_active", i128::from(self.dispatch.is_some())),
                    ("callback_active", i128::from(self.callback.is_some())),
                ],
            )?;
            self.completed = true;
            return Ok(());
        }
        if self.bases.get(&row.base) != Some(&(row.dispatcher, row.thread)) {
            return Err("observation has no matching live base/dispatcher/thread binding".into());
        }
        if self.dispatch.is_some_and(|base| base != row.base) {
            return Err("observation crosses the active dispatcher base".into());
        }
        match &row.action {
            ObservationAction::Socketpair {
                result,
                descriptors,
            } => {
                rule(
                    self.rules,
                    "observation_socketpair",
                    &[
                        ("result", (*result).into()),
                        ("descriptor_count", descriptors.len() as i128),
                        (
                            "endpoints_distinct",
                            i128::from(matches!(descriptors.as_slice(), [a, b] if a.fd != b.fd)),
                        ),
                    ],
                )?;
                if *result == -1 {
                    report.socketpair_failures += 1;
                } else if *result == 0 {
                    let [_, _] = descriptors.as_slice() else {
                        return Err("successful socketpair requires two descriptors".into());
                    };
                    for key in descriptors {
                        let previous = self.generations.get(&key.fd).copied().unwrap_or(0);
                        rule(
                            self.rules,
                            "observation_acquire",
                            &[
                                ("fd", key.fd.into()),
                                ("already_live", i128::from(self.live.contains_key(&key.fd))),
                                ("previous_generation", previous.into()),
                                ("generation", key.generation.into()),
                            ],
                        )?;
                        self.generations.insert(key.fd, key.generation);
                        self.live.insert(key.fd, (*key, row.base));
                        report.descriptor_acquisitions += 1;
                        report.descriptor_reuses += usize::from(previous != 0);
                    }
                } else {
                    return Err("invalid socketpair result".into());
                }
            }
            ObservationAction::DescriptorUse { descriptor, .. } => {
                self.live(*descriptor, row.base)?;
                report.descriptor_uses += 1;
            }
            ObservationAction::DescriptorClose { descriptor, result } => {
                self.live(*descriptor, row.base)?;
                rule(
                    self.rules,
                    "observation_close",
                    &[
                        (
                            "active_io_callback",
                            i128::from(
                                self.callback
                                    .and_then(|event| self.events.get(&event))
                                    .is_some_and(|event| event.descriptor == Some(*descriptor)),
                            ),
                        ),
                        ("result", (*result).into()),
                    ],
                )?;
                self.live.remove(&descriptor.fd);
                report.descriptor_closes += 1;
            }
            ObservationAction::Event {
                operation,
                event,
                callback,
                descriptor,
                result,
            } => {
                if *event == 0
                    || !matches!(
                        callback.as_str(),
                        "cb_event" | "cb_timeout" | "cb_wait_2_timeout"
                    )
                {
                    return Err("invalid event identity or callback profile".into());
                }
                if (*callback == "cb_event") != descriptor.is_some() {
                    return Err(
                        "callback descriptor does not match its I/O or timer profile".into(),
                    );
                }
                if *operation == EventAction::EventNew {
                    if *result != 0 || self.events.contains_key(event) {
                        return Err("failed or duplicate event creation is outside this observation profile".into());
                    }
                    if let Some(key) = descriptor {
                        self.live(*key, row.base)?;
                    }
                    self.events.insert(
                        *event,
                        EventState {
                            base: row.base,
                            callback: callback.clone(),
                            descriptor: *descriptor,
                            registered: false,
                            freed: false,
                        },
                    );
                    return Ok(());
                }
                let entry = self.events.get(event).ok_or("operation on unknown event")?;
                if entry.base != row.base
                    || entry.callback != *callback
                    || entry.descriptor != *descriptor
                    || (entry.freed && *operation != EventAction::CallbackExit)
                {
                    return Err("event binding changed or operation uses a freed event".into());
                }
                if matches!(
                    operation,
                    EventAction::EventAdd | EventAction::CallbackEnter
                ) && let Some(key) = descriptor
                {
                    self.live(*key, row.base)?;
                }
                let entry = self.events.get_mut(event).expect("checked event identity");
                match operation {
                    EventAction::EventAdd => {
                        if *result != 0 {
                            return Err(
                                "failed registration is outside this observation profile".into()
                            );
                        }
                        entry.registered = true;
                    }
                    EventAction::EventDel => {
                        if *result != 0 {
                            return Err(
                                "failed event deletion is outside this observation profile".into(),
                            );
                        }
                        entry.registered = false;
                    }
                    EventAction::EventFree => {
                        if *result != 0 {
                            return Err("invalid free record".into());
                        }
                        entry.freed = true;
                        entry.registered = false;
                    }
                    EventAction::CallbackEnter => {
                        if *result != 0
                            || self.dispatch != Some(row.base)
                            || self.callback.is_some()
                            || !entry.registered
                        {
                            return Err(
                                "callback lacks registered event and exclusive active dispatch"
                                    .into(),
                            );
                        }
                        self.callback = Some(*event);
                        report.callbacks.insert(callback.clone());
                    }
                    EventAction::CallbackExit => {
                        if *result != 0
                            || self.callback != Some(*event)
                            || self.dispatch != Some(row.base)
                        {
                            return Err("callback exit does not match active callback".into());
                        }
                        self.callback = None;
                    }
                    EventAction::EventNew => unreachable!(),
                }
            }
            ObservationAction::DispatchBegin { .. } => {
                if self.dispatch.replace(row.base).is_some() {
                    return Err("overlapping dispatch".into());
                }
            }
            ObservationAction::DispatchEnd { result } => {
                if !matches!(result, 0 | 1)
                    || self.dispatch != Some(row.base)
                    || self.callback.is_some()
                {
                    return Err(
                        "dispatch ended without a matching begin or with an active callback".into(),
                    );
                }
                self.dispatch = None;
            }
            ObservationAction::BaseFree { result } => {
                if *result != 0
                    || self.dispatch.is_some()
                    || self
                        .events
                        .values()
                        .any(|event| event.base == row.base && !event.freed)
                    || self.live.values().any(|(_, base)| *base == row.base)
                {
                    return Err(
                        "base teardown retains event, descriptor, or dispatch obligations".into(),
                    );
                }
                self.bases.remove(&row.base);
            }
            ObservationAction::Backend { .. } | ObservationAction::Completed { .. } => {
                unreachable!()
            }
        }
        Ok(())
    }
}

pub fn check_monkey_observations(source: &str) -> Result<MonkeyObservationReport, String> {
    check_monkey_observations_with_contracts(source, DESCRIPTOR_OBSERVATION_CONTRACTS)
}

/// Evaluate recorded identities under the supplied native policy. This cannot
/// issue a static certificate; callers must retain the policy with the evidence.
pub fn check_monkey_observations_with_contracts(
    source: &str,
    contracts: &str,
) -> Result<MonkeyObservationReport, String> {
    let mut checker = Checker {
        rules: contracts,
        ..Checker::default()
    };
    let mut report = MonkeyObservationReport {
        observations: 0,
        descriptor_acquisitions: 0,
        descriptor_reuses: 0,
        socketpair_failures: 0,
        descriptor_uses: 0,
        descriptor_closes: 0,
        callbacks: BTreeSet::new(),
        backends: BTreeMap::new(),
    };
    for (index, line) in source.lines().enumerate() {
        let row: MonkeyObservation =
            serde_json::from_str(line).map_err(|error| format!("record {}: {error}", index + 1))?;
        if row.sequence != (index as u64) + 1 {
            return Err(format!("record {}: noncontiguous sequence", index + 1));
        }
        checker
            .step(&row, &mut report)
            .map_err(|error| format!("record {}: {error}", index + 1))?;
        report.observations += 1;
    }
    if !checker.completed {
        return Err("missing completed observation".into());
    }
    Ok(report)
}

#[cfg(test)]
#[path = "../../tests/unit/monkey_observation.rs"]
mod tests;
