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

use std::sync::LazyLock;

use qcore_contract::profile_loader::LoadedProfile;

use crate::source_profiles::{LIBEVENT_SOURCE_SET, load_profile};

/// The declared `libevent` source-set profile (W2): the API disposition table
/// and the accepted-build matrix these tests exercise are the shipped data,
/// not a permissive test fixture.
static PROFILE: LazyLock<LoadedProfile> =
    LazyLock::new(|| load_profile(LIBEVENT_SOURCE_SET).expect("libevent profile loads"));

fn builds() -> &'static [BuildProfileSelector] {
    &PROFILE
        .library(LIBEVENT_SOURCE_SET)
        .expect("the libevent profile declares its library")
        .accepted_builds
}

fn dispositions() -> &'static BTreeMap<String, ApiDisposition> {
    &PROFILE
        .library(LIBEVENT_SOURCE_SET)
        .expect("the libevent profile declares its library")
        .api_dispositions
}

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
    let certificate = check_event_program(&one_shot(1, key(1), 9), builds()).unwrap();
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
    assert!(check_event_program(&program, builds()).is_ok());
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
        &check_event_program(&bad, builds()).unwrap_err(),
        |error| matches!(error, EventDiagnostic::L06NotQuiescent { .. })
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
    assert!(check_event_program(&good, builds()).is_ok());
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
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(
            error,
            EventDiagnostic::L00InvalidLifecycle { .. } | EventDiagnostic::L01UseAfterFree { .. }
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
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(error, EventDiagnostic::L06NotQuiescent { .. })
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
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(error, EventDiagnostic::L05ConflictingAccess { .. })
    ));
    if let EventOperation::Access { synchronized, .. } = &mut program.operations[17] {
        *synchronized = true;
    }
    assert!(check_event_program(&program, builds()).is_ok());
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
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(
            error,
            EventDiagnostic::L09UnknownProfile { .. }
                | EventDiagnostic::L04ThreadInitialization { .. }
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
    let certificate = check_event_program(&program, builds()).unwrap();
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
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(error, EventDiagnostic::L10BuffereventContract { .. })
    ));
    if let EventOperation::CreateBufferevent { deferred, .. } = &mut program.operations[1] {
        *deferred = true;
    }
    program
        .operations
        .insert(2, EventOperation::BufferOutput { bev, bytes: 1 });
    assert!(has(
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(error, EventDiagnostic::L07OwnedObligation { .. })
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
        classify_libevent_symbol(dispositions(), "evhttp_new"),
        ProfileDisposition::Unknown
    );
    assert_eq!(
        classify_libevent_symbol(dispositions(), "private_event_magic"),
        ProfileDisposition::Unknown
    );
    let mut program = one_shot(1, key(1), 9);
    if let EventOperation::CreateBase { profile, .. } = &mut program.operations[0] {
        profile.openssl = true;
    }
    assert!(has(
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(error, EventDiagnostic::L09UnknownProfile { .. })
    ));
}

#[test]
fn evutil_socket_contracts_are_profiled_and_conserve_custody() {
    for symbol in [
        "evutil_closesocket",
        "evutil_socket_geterror",
        "evutil_socketpair",
    ] {
        assert_eq!(
            classify_libevent_symbol(dispositions(), symbol),
            ProfileDisposition::Accepted
        );
    }
    let program = EventProgram {
        live_arguments: BTreeSet::new(),
        operations: vec![
            EventOperation::CreateSocketPair {
                descriptors: Some((3, 4)),
            },
            EventOperation::GetSocketError { descriptor: 3 },
            EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: true,
                declared_failure: None,
            },
            EventOperation::CloseSocket {
                descriptor: 4,
                succeeds: true,
                declared_failure: None,
            },
        ],
    };
    let certificate = check_event_program(&program, builds()).unwrap();
    assert_eq!(certificate.closed_descriptors, BTreeSet::from([3, 4]));
}

#[test]
fn failed_socketpair_creates_no_descriptor_obligation() {
    let program = EventProgram {
        live_arguments: BTreeSet::new(),
        operations: vec![EventOperation::CreateSocketPair { descriptors: None }],
    };
    assert!(check_event_program(&program, builds()).is_ok());
}

#[test]
fn failed_or_duplicate_socket_close_is_fail_closed() {
    let failed_close = EventProgram {
        live_arguments: BTreeSet::new(),
        operations: vec![
            EventOperation::CreateSocketPair {
                descriptors: Some((3, 4)),
            },
            EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: false,
                declared_failure: None,
            },
            EventOperation::CloseSocket {
                descriptor: 4,
                succeeds: true,
                declared_failure: None,
            },
        ],
    };
    let diagnostics = check_event_program(&failed_close, builds()).unwrap_err();
    assert!(
        diagnostics.contains(&EventDiagnostic::L03DescriptorCustody {
            operation: failed_close.operations.len(),
            descriptor: 3,
        })
    );

    let duplicate_close = EventProgram {
        live_arguments: BTreeSet::new(),
        operations: vec![
            EventOperation::CreateSocketPair {
                descriptors: Some((3, 4)),
            },
            EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: true,
                declared_failure: None,
            },
            EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: true,
                declared_failure: None,
            },
            EventOperation::CloseSocket {
                descriptor: 4,
                succeeds: true,
                declared_failure: None,
            },
        ],
    };
    assert!(has(
        &check_event_program(&duplicate_close, builds()).unwrap_err(),
        |error| matches!(
            error,
            EventDiagnostic::L03DescriptorCustody {
                operation: 2,
                descriptor: 3
            }
        )
    ));
}

#[test]
fn socket_error_query_cannot_use_a_closed_descriptor() {
    let program = EventProgram {
        live_arguments: BTreeSet::new(),
        operations: vec![
            EventOperation::CreateSocketPair {
                descriptors: Some((3, 4)),
            },
            EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: true,
                declared_failure: None,
            },
            EventOperation::GetSocketError { descriptor: 3 },
            EventOperation::CloseSocket {
                descriptor: 4,
                succeeds: true,
                declared_failure: None,
            },
        ],
    };
    assert!(has(
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(
            error,
            EventDiagnostic::L03DescriptorCustody {
                operation: 2,
                descriptor: 3
            }
        )
    ));
}

#[test]
fn active_event_borrow_prevents_socket_close() {
    let mut program = one_shot(1, key(1), 9);
    program.operations.insert(
        0,
        EventOperation::CreateSocketPair {
            descriptors: Some((3, 4)),
        },
    );
    program.operations.insert(
        6,
        EventOperation::CloseSocket {
            descriptor: 3,
            succeeds: true,
            declared_failure: None,
        },
    );
    let free_base = program.operations.len() - 1;
    program.operations.splice(
        free_base..free_base,
        [
            EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: true,
                declared_failure: None,
            },
            EventOperation::CloseSocket {
                descriptor: 4,
                succeeds: true,
                declared_failure: None,
            },
        ],
    );
    assert!(has(
        &check_event_program(&program, builds()).unwrap_err(),
        |error| matches!(
            error,
            EventDiagnostic::L03DescriptorCustody {
                operation: 6,
                descriptor: 3
            }
        )
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
            &check_event_program(&program, builds()).unwrap_err(),
            |error| matches!(error, EventDiagnostic::L08OrderingAssumption { .. })
        ));
    }
}

#[test]
fn adapter_observation_must_refine_the_checked_trace() {
    let certificate = check_event_program(&one_shot(1, key(1), 9), builds()).unwrap();
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
        Err(EventDiagnostic::L12TraceRefinement { .. })
    ));
}

#[test]
fn generic_profile_bridge_preserves_loaded_build_matrix_and_empty_rejection() {
    let mut profiles = Vec::new();
    for backend in [
        EventBackend::Epoll,
        EventBackend::Poll,
        EventBackend::Select,
        EventBackend::Kqueue,
        EventBackend::Iocp,
    ] {
        for threads in [
            ThreadMode::Disabled,
            ThreadMode::Pthreads,
            ThreadMode::WindowsThreads,
        ] {
            for openssl in [false, true] {
                for debug_mode in [false, true] {
                    profiles.push(LibeventBuildProfile {
                        backend,
                        threads,
                        openssl,
                        debug_mode,
                    });
                }
            }
        }
    }
    for left in &profiles {
        for right in &profiles {
            let program = EventProgram {
                live_arguments: BTreeSet::new(),
                operations: vec![
                    EventOperation::CreateBase {
                        base: 1,
                        profile: *left,
                    },
                    EventOperation::CreateBase {
                        base: 2,
                        profile: *right,
                    },
                    EventOperation::FreeBase { base: 1 },
                    EventOperation::FreeBase { base: 2 },
                ],
            };
            let result = check_event_program(&program, builds());
            assert_eq!(
                result.is_ok(),
                left.accepted(builds()) && right.accepted(builds()),
                "{left:?}, {right:?}: {result:?}"
            );
            let rejected = check_event_program(&program, &[]).unwrap_err();
            assert!(rejected.contains(&EventDiagnostic::L09UnknownProfile { operation: 0 }));
            assert!(rejected.contains(&EventDiagnostic::L09UnknownProfile { operation: 1 }));
        }
    }
}
