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

use super::*;

fn id(id: u64) -> ObservationIdentity {
    ObservationIdentity { id, generation: 1 }
}

fn context() -> TimedWaitObservationContext {
    TimedWaitObservationContext {
        run: 7,
        buffer: 3,
        dispatcher: 11,
        base: id(1),
        event: id(2),
        argument: id(3),
        callback: "cb_wait_2_timeout".into(),
        backend: "epoll".into(),
        flags: 0x01,
        priority: 0,
    }
}

fn trace() -> EventObservationTrace {
    let c = context();
    use EventObservationAction as A;
    let actions = vec![
        A::BaseCreate {
            base: c.base,
            backend: c.backend.clone(),
        },
        A::EventNew {
            base: c.base,
            event: c.event,
            argument: c.argument,
            callback: c.callback.clone(),
            flags: c.flags,
            priority: c.priority,
            result: 0,
        },
        A::EventAdd {
            event: c.event,
            result: 0,
        },
        A::DispatchBegin { base: c.base },
        A::Activate {
            event: c.event,
            reason: ObservedActivationReason::Timeout,
        },
        A::CallbackEnter {
            event: c.event,
            argument: c.argument,
            callback: c.callback.clone(),
            reason: ObservedActivationReason::Timeout,
        },
        A::CallbackExit {
            event: c.event,
            argument: c.argument,
            callback: c.callback.clone(),
        },
        A::DispatchEnd {
            base: c.base,
            result: 0,
        },
        A::EventFree { event: c.event },
        A::ArgumentEnd {
            argument: c.argument,
        },
        A::BaseFree { base: c.base },
    ];
    EventObservationTrace {
        records: actions
            .into_iter()
            .enumerate()
            .map(|(index, action)| EventObservationRecord {
                run: c.run,
                buffer: c.buffer,
                local_sequence: index as u64 + 1,
                dispatcher: c.dispatcher,
                action,
            })
            .collect(),
        synchronization: Vec::new(),
    }
}

#[test]
fn a_complete_timed_wait_trace_binds_every_identity() {
    let certificate = check_timed_wait_observation(&trace(), &context()).unwrap();
    assert_eq!(certificate.checked_records(), 11);
    assert_eq!(certificate.context(), &context());
}

#[test]
fn identity_and_semantic_mutations_are_rejected() {
    for mutate in 0..8 {
        let mut observed = trace();
        match mutate {
            0 => match &mut observed.records[1].action {
                EventObservationAction::EventNew { event, .. } => event.generation = 2,
                _ => unreachable!(),
            },
            1 => match &mut observed.records[5].action {
                EventObservationAction::CallbackEnter { callback, .. } => {
                    *callback = "cb_event".into()
                }
                _ => unreachable!(),
            },
            2 => match &mut observed.records[5].action {
                EventObservationAction::CallbackEnter { reason, .. } => {
                    *reason = ObservedActivationReason::Read
                }
                _ => unreachable!(),
            },
            3 => {
                observed.records.remove(6);
            }
            4 => observed.records[7].local_sequence = 99,
            5 => observed.records[5].dispatcher = 12,
            6 => observed.synchronization.push(SynchronizationEdge {
                from: ObservationPosition {
                    buffer: 3,
                    local_sequence: 4,
                },
                to: ObservationPosition {
                    buffer: 3,
                    local_sequence: 5,
                },
            }),
            7 => observed.records.swap(8, 9),
            _ => unreachable!(),
        }
        assert!(
            check_timed_wait_observation(&observed, &context()).is_err(),
            "mutation {mutate} was accepted"
        );
    }
}
