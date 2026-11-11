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
use crate::os_endpoint::{
    DescriptorState, Duty, Errno, Outcome as EndpointOutcome, Premises, Recovery, Target,
    close_poststate,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TestProfile(bool);
impl ReactorProfile for TestProfile {
    fn threading_enabled(self) -> bool {
        self.0
    }
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

fn one_shot(base: u32, event: EventKey, argument: u32) -> EventProgram<TestProfile> {
    EventProgram {
        live_arguments: BTreeSet::from([argument]),
        operations: vec![
            EventOperation::CreateBase {
                base,
                profile: TestProfile(true),
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

fn callback_self_teardown() -> EventProgram<TestProfile> {
    let event = key(41);
    EventProgram {
        live_arguments: BTreeSet::from([90]),
        operations: vec![
            EventOperation::CreateSocketPair {
                descriptors: Some((7, 8)),
            },
            EventOperation::CreateBase {
                base: 4,
                profile: TestProfile(true),
            },
            EventOperation::Assign {
                event,
                base: 4,
                descriptor: None,
                flags: EventFlags::new(0),
                priority: 0,
                summary: summary(90, ActivationReason::Timeout),
            },
            EventOperation::Add { event },
            EventOperation::BeginDispatch {
                base: 4,
                dispatcher: 12,
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
            EventOperation::CloseSocket {
                descriptor: 8,
                succeeds: true,
                declared_failure: None,
            },
            EventOperation::DeleteCurrentCallback {
                event,
                dispatcher: 12,
            },
            EventOperation::FreeCurrentCallbackEvent {
                event,
                dispatcher: 12,
            },
            EventOperation::DropArgument { argument: 90 },
            EventOperation::CallbackExit { event },
            EventOperation::EndDispatch { base: 4 },
            EventOperation::CloseSocket {
                descriptor: 7,
                succeeds: true,
                declared_failure: None,
            },
            EventOperation::FreeBase { base: 4 },
        ],
    }
}

#[test]
fn current_callback_teardown_keeps_event_map_and_descriptors_distinct() {
    let certificate = check_event_program(&callback_self_teardown(), &[TestProfile(true)]).unwrap();
    assert_eq!(certificate.freed_events, BTreeSet::from([key(41)]));
    assert_eq!(certificate.dropped_arguments, BTreeSet::from([90]));
    assert_eq!(certificate.closed_descriptors, BTreeSet::from([7, 8]));
}

#[test]
fn ordinary_free_while_running_remains_refused() {
    let mut program = callback_self_teardown();
    program.operations[9] = EventOperation::FreeEvent { event: key(41) };
    assert!(
        check_event_program(&program, &[TestProfile(true)])
            .unwrap_err()
            .iter()
            .any(|diagnostic| matches!(
                diagnostic,
                EventDiagnostic::L06NotQuiescent { operation: 9 }
            ))
    );
}

#[test]
fn callback_self_teardown_requires_delete_same_dispatcher_and_no_event_descriptor() {
    let mut wrong_dispatcher = callback_self_teardown();
    wrong_dispatcher.operations[8] = EventOperation::DeleteCurrentCallback {
        event: key(41),
        dispatcher: 99,
    };
    assert!(check_event_program(&wrong_dispatcher, &[TestProfile(true)]).is_err());

    let mut missing_delete = callback_self_teardown();
    missing_delete.operations.remove(8);
    assert!(check_event_program(&missing_delete, &[TestProfile(true)]).is_err());

    let mut borrowed_descriptor = callback_self_teardown();
    if let EventOperation::Assign { descriptor, .. } = &mut borrowed_descriptor.operations[2] {
        *descriptor = Some(8);
    }
    assert!(check_event_program(&borrowed_descriptor, &[TestProfile(true)]).is_err());

    let mut generic_nonblocking_delete = callback_self_teardown();
    generic_nonblocking_delete.operations[8] = EventOperation::Delete {
        event: key(41),
        mode: DeleteMode::Nonblocking,
    };
    assert!(check_event_program(&generic_nonblocking_delete, &[TestProfile(true)]).is_err());
}

#[test]
fn self_torn_down_event_cannot_be_resurrected_or_released_twice() {
    for invalid in [
        EventOperation::Delete {
            event: key(41),
            mode: DeleteMode::Ordinary,
        },
        EventOperation::Add { event: key(41) },
        EventOperation::Activate {
            event: key(41),
            reason: ActivationReason::Timeout,
            cross_thread: false,
        },
        EventOperation::FreeEvent { event: key(41) },
        EventOperation::FreeCurrentCallbackEvent {
            event: key(41),
            dispatcher: 12,
        },
    ] {
        let mut program = callback_self_teardown();
        program.operations.insert(10, invalid);
        assert!(check_event_program(&program, &[TestProfile(true)]).is_err());
    }

    let mut wrong_exit = callback_self_teardown();
    wrong_exit.operations[11] = EventOperation::CallbackExit { event: key(99) };
    assert!(check_event_program(&wrong_exit, &[TestProfile(true)]).is_err());

    let mut post_drop_access = callback_self_teardown();
    post_drop_access.operations.insert(
        11,
        EventOperation::Access {
            context: CallbackContext::Event(key(41)),
            payload: 7,
            kind: PayloadAccessKind::Write,
            synchronized: false,
        },
    );
    assert!(
        check_event_program(&post_drop_access, &[TestProfile(true)])
            .unwrap_err()
            .iter()
            .any(|diagnostic| matches!(
                diagnostic,
                EventDiagnostic::L02ArgumentLifetime { operation: 11, .. }
            ))
    );

    let mut double_map_free = callback_self_teardown();
    double_map_free
        .operations
        .insert(11, EventOperation::DropArgument { argument: 90 });
    assert!(check_event_program(&double_map_free, &[TestProfile(true)]).is_err());
}

#[test]
fn generic_callback_custody_requires_explicit_profile_selection() {
    let program = one_shot(1, key(1), 9);
    assert!(
        check_event_program(&program, &[])
            .unwrap_err()
            .iter()
            .any(|e| matches!(e, EventDiagnostic::L09UnknownProfile { operation: 0 }))
    );
    let certificate = check_event_program(&program, &[TestProfile(true)]).unwrap();
    assert!(certificate.freed_events.contains(&key(1)));
    assert!(certificate.dropped_arguments.contains(&9));
    let observed: Vec<_> = certificate
        .trace
        .iter()
        .filter_map(|e| (e.kind != ObservedKind::Lifecycle).then_some(e.kind))
        .collect();
    check_observation_refinement(&certificate, &observed).unwrap();
    assert!(check_observation_refinement(&certificate, &[]).is_err());
}

#[test]
fn generic_reactor_rejects_stale_generations_and_missing_callback_authority() {
    let mut program = one_shot(1, key(1), 9);
    if let EventOperation::Assign { summary, .. } = &mut program.operations[1] {
        summary.payload_authority.clear();
    }
    assert!(
        check_event_program(&program, &[TestProfile(true)])
            .unwrap_err()
            .iter()
            .any(|e| matches!(e, EventDiagnostic::L02ArgumentLifetime { .. }))
    );
    let mut program = one_shot(1, key(1), 9);
    program.operations.insert(
        10,
        EventOperation::Activate {
            event: EventKey::new(1, 0),
            reason: ActivationReason::Read,
            cross_thread: false,
        },
    );
    assert!(
        check_event_program(&program, &[TestProfile(true)])
            .unwrap_err()
            .iter()
            .any(|e| matches!(
                e,
                EventDiagnostic::L00InvalidLifecycle { .. }
                    | EventDiagnostic::L01UseAfterFree { .. }
            ))
    );
}

#[test]
fn profile_selection_does_not_grant_cross_thread_authority() {
    let mut program = one_shot(1, key(1), 9);
    if let EventOperation::CreateBase { profile, .. } = &mut program.operations[0] {
        *profile = TestProfile(false);
    }
    if let EventOperation::Activate { cross_thread, .. } = &mut program.operations[4] {
        *cross_thread = true;
    }
    assert!(
        check_event_program(&program, &[TestProfile(false)])
            .unwrap_err()
            .iter()
            .any(|e| matches!(e, EventDiagnostic::L04ThreadInitialization { .. }))
    );
}

#[test]
fn descriptor_custody_is_independent_of_any_library_profile() {
    let mut program = EventProgram::<TestProfile> {
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
                descriptor: 4,
                succeeds: true,
                declared_failure: None,
            },
        ],
    };
    assert_eq!(
        check_event_program(&program, &[])
            .unwrap()
            .closed_descriptors,
        BTreeSet::from([3, 4])
    );
    program.operations.push(EventOperation::CloseSocket {
        descriptor: 3,
        succeeds: true,
        declared_failure: None,
    });
    assert!(
        check_event_program(&program, &[])
            .unwrap_err()
            .iter()
            .any(|e| matches!(e, EventDiagnostic::L03DescriptorCustody { .. }))
    );
}

/// Card O03-U4. The reactor vocabulary carries neither a target nor an
/// `errno`, so `close_poststate` answers `Indeterminate` with no permitted
/// recovery for every failed close. The number is therefore poisoned: neither
/// usable, nor retriable, nor free, and the endpoint stays unresolved.
fn failed_close_program(tail: Vec<EventOperation<TestProfile>>) -> EventProgram<TestProfile> {
    let mut operations = vec![
        EventOperation::CreateSocketPair {
            descriptors: Some((3, 4)),
        },
        EventOperation::CloseSocket {
            descriptor: 3,
            succeeds: false,
            declared_failure: None,
        },
    ];
    operations.extend(tail);
    operations.push(EventOperation::CloseSocket {
        descriptor: 4,
        succeeds: true,
        declared_failure: None,
    });
    EventProgram {
        live_arguments: BTreeSet::new(),
        operations,
    }
}

#[test]
fn a_failed_close_agrees_with_the_unpinned_endpoint_relation() {
    // The declared disposition, read from the relation rather than assumed.
    for errno in [
        Errno::Ebadf,
        Errno::Eintr,
        Errno::Eio,
        Errno::Enospc,
        Errno::Edquot,
        Errno::Other,
        Errno::Unobserved,
    ] {
        let derived = close_poststate(
            Target::Unpinned,
            Premises::default(),
            EndpointOutcome::MinusOne(errno),
        )
        .expect("in domain");
        assert!(derived.descriptor.is_poisoned(), "{errno:?}");
        assert!(derived.permitted_recovery.is_empty(), "{errno:?}");
        assert!(derived.duties.contains(&Duty::NoRetryClose), "{errno:?}");
        assert!(derived.duties.contains(&Duty::NoUse), "{errno:?}");
        assert!(
            derived.duties.contains(&Duty::NoTreatNumberAsFree),
            "{errno:?}"
        );
        assert!(
            derived.duties.contains(&Duty::KeepInDenominator),
            "{errno:?}"
        );
    }

    // `NoRetryClose`: the retry the old declared model required is refused at
    // its own operation, and the endpoint still fails the terminal check.
    let retried = failed_close_program(vec![EventOperation::CloseSocket {
        descriptor: 3,
        succeeds: true,
        declared_failure: None,
    }]);
    let diagnostics = check_event_program(&retried, &[]).unwrap_err();
    assert!(
        diagnostics.contains(&EventDiagnostic::L03DescriptorCustody {
            operation: 2,
            descriptor: 3,
        })
    );
    assert!(
        diagnostics.contains(&EventDiagnostic::L03DescriptorCustody {
            operation: retried.operations.len(),
            descriptor: 3,
        })
    );

    // `NoUse`: a borrowing error query on the poisoned number is refused.
    let queried = failed_close_program(vec![EventOperation::GetSocketError { descriptor: 3 }]);
    assert!(check_event_program(&queried, &[]).unwrap_err().contains(
        &EventDiagnostic::L03DescriptorCustody {
            operation: 2,
            descriptor: 3,
        }
    ));

    // `NoTreatNumberAsFree`: a fresh pair may not claim the poisoned number.
    let reused = failed_close_program(vec![EventOperation::CreateSocketPair {
        descriptors: Some((3, 5)),
    }]);
    assert!(check_event_program(&reused, &[]).unwrap_err().contains(
        &EventDiagnostic::L03DescriptorCustody {
            operation: 2,
            descriptor: 3,
        }
    ));

    // `KeepInDenominator`: the plain failed close is still refused, and the
    // successfully closed sibling is unaffected.
    let plain = failed_close_program(Vec::new());
    assert!(check_event_program(&plain, &[]).unwrap_err().contains(
        &EventDiagnostic::L03DescriptorCustody {
            operation: plain.operations.len(),
            descriptor: 3,
        }
    ));
}

// ---------------------------------------------------------------------------
// Card O03-REACTOR: the declared close-failure premise
// ---------------------------------------------------------------------------

fn declared(target: &str, errno: &str, record_locks: &str) -> DeclaredCloseFailure {
    DeclaredCloseFailure {
        target: target.to_string(),
        errno: errno.to_string(),
        record_locks: record_locks.to_string(),
    }
}

/// A socket pair, a failed close on `3` carrying `premise`, an optional tail,
/// and a successful close of the sibling `4`.
fn declared_failure_program(
    premise: Option<DeclaredCloseFailure>,
    tail: Vec<EventOperation<TestProfile>>,
) -> EventProgram<TestProfile> {
    let mut operations = vec![
        EventOperation::CreateSocketPair {
            descriptors: Some((3, 4)),
        },
        EventOperation::CloseSocket {
            descriptor: 3,
            succeeds: false,
            declared_failure: premise,
        },
    ];
    operations.extend(tail);
    operations.push(EventOperation::CloseSocket {
        descriptor: 4,
        succeeds: true,
        declared_failure: None,
    });
    EventProgram {
        live_arguments: BTreeSet::new(),
        operations,
    }
}

/// Every errno class on every target: the reactor's disposition is exactly what
/// the relation derives, read out of `close_poststate` here rather than
/// restated by hand — and separately compared against the documented table, so
/// a silent change to either side is caught.
#[test]
fn every_declared_errno_class_takes_the_relations_disposition_on_every_target() {
    let mut seen = BTreeSet::new();
    for (spelling, target) in [
        ("linux", Target::Linux),
        ("posix-2024", Target::Posix2024),
        ("unpinned", Target::Unpinned),
    ] {
        for (word, errno) in [
            ("ebadf", Errno::Ebadf),
            ("eintr", Errno::Eintr),
            ("eio", Errno::Eio),
            ("enospc", Errno::Enospc),
            ("edquot", Errno::Edquot),
            ("other", Errno::Other),
            ("unobserved", Errno::Unobserved),
        ] {
            let classified =
                classify_declared_close_failure(&declared(spelling, word, "none-held"))
                    .expect("declared vocabulary");
            assert_eq!(classified.target, target);
            assert_eq!(classified.errno, errno);

            let independent = close_poststate(
                target,
                classified.premises,
                EndpointOutcome::MinusOne(errno),
            )
            .expect("in domain");
            assert_eq!(classified.derived, independent, "{spelling}/{word}");
            let expected = match independent.descriptor {
                DescriptorState::Released => ClosePremiseDisposition::ReleasedNumberRetired,
                DescriptorState::Retained
                    if independent.permitted_recovery.contains(&Recovery::Close) =>
                {
                    ClosePremiseDisposition::RetainedRetryPermitted
                }
                _ => ClosePremiseDisposition::Poisoned,
            };
            assert_eq!(classified.disposition, expected, "{spelling}/{word}");
            seen.insert(classified.disposition);

            let table = match (target, errno) {
                (_, Errno::Ebadf) | (_, Errno::Unobserved) => ClosePremiseDisposition::Poisoned,
                (Target::Linux, _) => ClosePremiseDisposition::ReleasedNumberRetired,
                (Target::Posix2024, Errno::Eintr) => {
                    ClosePremiseDisposition::RetainedRetryPermitted
                }
                _ => ClosePremiseDisposition::Poisoned,
            };
            assert_eq!(classified.disposition, table, "{spelling}/{word}");
        }
    }
    // All three dispositions are reachable: the vocabulary is not decorative.
    assert_eq!(seen.len(), 3);
}

/// POSIX.1-2024 `EINTR` is the *only* row on which a failed close retains the
/// epoch, so it is the only row on which a retry is admitted.
#[test]
fn only_posix2024_eintr_retains_the_epoch_and_admits_a_retry() {
    let mut retaining = Vec::new();
    for target in DECLARED_CLOSE_TARGET_SPELLINGS {
        for errno in DECLARED_CLOSE_ERRNO_SPELLINGS {
            for locks in DECLARED_CLOSE_RECORD_LOCK_SPELLINGS {
                let classified = classify_declared_close_failure(&declared(target, errno, locks))
                    .expect("declared vocabulary");
                if classified.disposition == ClosePremiseDisposition::RetainedRetryPermitted {
                    retaining.push((target, errno, locks));
                }
            }
        }
    }
    assert_eq!(retaining, vec![("posix-2024", "eintr", "none-held")]);

    // The reactor admits exactly that retry, and still refuses the program
    // that never closes the retained epoch.
    let retried = declared_failure_program(
        Some(declared("posix-2024", "eintr", "none-held")),
        vec![EventOperation::CloseSocket {
            descriptor: 3,
            succeeds: true,
            declared_failure: None,
        }],
    );
    let certificate = check_event_program(&retried, &[]).expect("retry admitted");
    assert_eq!(certificate.closed_descriptors, BTreeSet::from([3, 4]));
    assert_eq!(certificate.declared_close_premises.len(), 1);
    assert_eq!(
        certificate.declared_close_premises[0]
            .classified
            .disposition,
        ClosePremiseDisposition::RetainedRetryPermitted
    );
    recheck_declared_close_premises(&retried, &certificate).expect("recheck");

    let unretried = declared_failure_program(
        Some(declared("posix-2024", "eintr", "none-held")),
        Vec::new(),
    );
    assert!(check_event_program(&unretried, &[]).unwrap_err().contains(
        &EventDiagnostic::L03DescriptorCustody {
            operation: unretried.operations.len(),
            descriptor: 3,
        }
    ));
}

/// A pinned Linux failure releases the epoch, so the custody obligation is
/// discharged — and the number is retired, exactly as a successful close
/// retires it. Reuse, borrow and retry are each still refused.
#[test]
fn a_pinned_linux_failure_releases_the_epoch_and_retires_the_number() {
    for errno in ["eintr", "eio", "enospc", "edquot", "other"] {
        let program =
            declared_failure_program(Some(declared("linux", errno, "none-held")), Vec::new());
        let certificate = check_event_program(&program, &[]).expect("released");
        assert_eq!(certificate.closed_descriptors, BTreeSet::from([3, 4]));
        let premise = &certificate.declared_close_premises[0];
        assert_eq!(premise.operation, 1);
        assert_eq!(premise.descriptor, 3);
        assert_eq!(
            premise.classified.disposition,
            ClosePremiseDisposition::ReleasedNumberRetired
        );
        // A release is not a durability claim, and it permits no retry: the
        // relation's residual travels in the certificate rather than being
        // swallowed.
        assert!(premise.classified.derived.permitted_recovery.is_empty());
        assert!(!premise.classified.derived.duties.contains(&Duty::NoUse));
        recheck_declared_close_premises(&program, &certificate).expect("recheck");

        for (label, tail) in [
            (
                "retry",
                EventOperation::CloseSocket {
                    descriptor: 3,
                    succeeds: true,
                    declared_failure: None,
                },
            ),
            ("borrow", EventOperation::GetSocketError { descriptor: 3 }),
            (
                "reuse",
                EventOperation::CreateSocketPair {
                    descriptors: Some((3, 5)),
                },
            ),
        ] {
            let refused =
                declared_failure_program(Some(declared("linux", errno, "none-held")), vec![tail]);
            assert!(
                check_event_program(&refused, &[]).unwrap_err().contains(
                    &EventDiagnostic::L03DescriptorCustody {
                        operation: 2,
                        descriptor: 3,
                    }
                ),
                "{errno}/{label}"
            );
        }
    }
    // EBADF is a refutation, not a release: it stays poisoned.
    let ebadf = declared_failure_program(Some(declared("linux", "ebadf", "none-held")), Vec::new());
    assert!(check_event_program(&ebadf, &[]).unwrap_err().contains(
        &EventDiagnostic::L03DescriptorCustody {
            operation: ebadf.operations.len(),
            descriptor: 3,
        }
    ));
}

/// An unpinned declaration is the same answer as no declaration at all, and a
/// declared-unknown record-lock premise (A7 denied) poisons on every target.
#[test]
fn an_unpinned_or_lockless_declaration_keeps_the_poisoned_answer() {
    for premise in [
        Some(declared("unpinned", "eio", "none-held")),
        Some(declared("linux", "eio", "unknown")),
        Some(declared("posix-2024", "eintr", "unknown")),
        None,
    ] {
        let program = declared_failure_program(premise.clone(), Vec::new());
        assert!(
            check_event_program(&program, &[]).unwrap_err().contains(
                &EventDiagnostic::L03DescriptorCustody {
                    operation: program.operations.len(),
                    descriptor: 3,
                }
            ),
            "{premise:?}"
        );
        // Poisoned, not merely unresolved: the retry is refused at its own
        // operation index too.
        let retried = declared_failure_program(
            premise.clone(),
            vec![EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: true,
                declared_failure: None,
            }],
        );
        assert!(
            check_event_program(&retried, &[]).unwrap_err().contains(
                &EventDiagnostic::L03DescriptorCustody {
                    operation: 2,
                    descriptor: 3,
                }
            ),
            "{premise:?}"
        );
    }
}

/// A spelling outside any of the three closed vocabularies refuses the event.
/// It never falls back to a target, an errno or a lock premise the declaration
/// did not name.
#[test]
fn an_unrecognized_declared_spelling_refuses_the_event() {
    let refusals: Vec<(DeclaredCloseFailure, DeclaredCloseRefusal)> = vec![
        (
            declared("Linux", "eio", "none-held"),
            DeclaredCloseRefusal::UnknownTarget("Linux".into()),
        ),
        (
            declared("posix2024", "eintr", "none-held"),
            DeclaredCloseRefusal::UnknownTarget("posix2024".into()),
        ),
        (
            declared(" linux", "eio", "none-held"),
            DeclaredCloseRefusal::UnknownTarget(" linux".into()),
        ),
        (
            declared("", "eio", "none-held"),
            DeclaredCloseRefusal::UnknownTarget(String::new()),
        ),
        (
            declared("linux", "EIO", "none-held"),
            DeclaredCloseRefusal::UnknownErrno("EIO".into()),
        ),
        (
            declared("linux", "eagain", "none-held"),
            DeclaredCloseRefusal::UnknownErrno("eagain".into()),
        ),
        (
            declared("linux", "", "none-held"),
            DeclaredCloseRefusal::UnknownErrno(String::new()),
        ),
        (
            declared("linux", "eio", "none_held"),
            DeclaredCloseRefusal::UnknownRecordLocks("none_held".into()),
        ),
        (
            declared("linux", "eio", ""),
            DeclaredCloseRefusal::UnknownRecordLocks(String::new()),
        ),
    ];
    for (premise, expected) in refusals {
        assert_eq!(
            classify_declared_close_failure(&premise).unwrap_err(),
            expected
        );
        let program = declared_failure_program(Some(premise.clone()), Vec::new());
        let diagnostics = check_event_program(&program, &[]).unwrap_err();
        // Refused at its own operation index, and the number stays poisoned.
        assert!(
            diagnostics.contains(&EventDiagnostic::L03DescriptorCustody {
                operation: 1,
                descriptor: 3,
            }),
            "{premise:?}"
        );
        assert!(
            diagnostics.contains(&EventDiagnostic::L03DescriptorCustody {
                operation: program.operations.len(),
                descriptor: 3,
            }),
            "{premise:?}"
        );
    }
}

/// A failure premise on the success branch is a contradictory event: neither
/// half is believed, and the number is poisoned rather than closed.
#[test]
fn a_declared_failure_premise_on_the_success_arm_is_refused() {
    let program = EventProgram::<TestProfile> {
        live_arguments: BTreeSet::new(),
        operations: vec![
            EventOperation::CreateSocketPair {
                descriptors: Some((3, 4)),
            },
            EventOperation::CloseSocket {
                descriptor: 3,
                succeeds: true,
                declared_failure: Some(declared("linux", "eio", "none-held")),
            },
            EventOperation::CloseSocket {
                descriptor: 4,
                succeeds: true,
                declared_failure: None,
            },
        ],
    };
    let diagnostics = check_event_program(&program, &[]).unwrap_err();
    assert!(
        diagnostics.contains(&EventDiagnostic::L03DescriptorCustody {
            operation: 1,
            descriptor: 3,
        })
    );
    assert!(
        diagnostics.contains(&EventDiagnostic::L03DescriptorCustody {
            operation: program.operations.len(),
            descriptor: 3,
        })
    );
}

/// Absent a declaration nothing moves: the certificate carries no premise and
/// the pre-card program answers exactly as before.
#[test]
fn an_absent_declaration_changes_nothing() {
    let program = EventProgram::<TestProfile> {
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
                descriptor: 4,
                succeeds: true,
                declared_failure: None,
            },
        ],
    };
    let certificate = check_event_program(&program, &[]).expect("accepted");
    assert!(certificate.declared_close_premises.is_empty());
    assert_eq!(certificate.closed_descriptors, BTreeSet::from([3, 4]));
    recheck_declared_close_premises(&program, &certificate).expect("recheck");
}

/// The recheck rederives every premise from the relation. Substituting the
/// declared pair, the classification or the inventory is refused.
#[test]
fn the_recheck_refuses_a_substituted_declared_premise() {
    let program = declared_failure_program(Some(declared("linux", "eio", "none-held")), Vec::new());
    let certificate = check_event_program(&program, &[]).expect("released");
    recheck_declared_close_premises(&program, &certificate).expect("honest certificate");

    // A substituted declaration: the certificate says the adapter declared
    // POSIX.1-2024, the program says Linux.
    let mut forged = certificate.clone();
    forged.declared_close_premises[0].declared = declared("posix-2024", "eintr", "none-held");
    assert!(matches!(
        recheck_declared_close_premises(&program, &forged),
        Err(ClosePremiseRecheckError::Declaration { operation: 1, .. })
    ));

    // A substituted classification: the same declaration, a different answer.
    let mut forged = certificate.clone();
    forged.declared_close_premises[0].classified.disposition =
        ClosePremiseDisposition::RetainedRetryPermitted;
    assert!(matches!(
        recheck_declared_close_premises(&program, &forged),
        Err(ClosePremiseRecheckError::Classification { operation: 1, .. })
    ));

    // A substituted derived post-state, keeping the disposition honest.
    let mut forged = certificate.clone();
    forged.declared_close_premises[0]
        .classified
        .derived
        .descriptor = DescriptorState::Retained;
    assert!(matches!(
        recheck_declared_close_premises(&program, &forged),
        Err(ClosePremiseRecheckError::Classification { .. })
    ));

    // A substituted premise vector — claiming the last reference, which the
    // reactor never claims.
    let mut forged = certificate.clone();
    forged.declared_close_premises[0]
        .classified
        .premises
        .last_reference = true;
    assert!(matches!(
        recheck_declared_close_premises(&program, &forged),
        Err(ClosePremiseRecheckError::Classification { .. })
    ));

    // A substituted descriptor and a substituted operation index.
    let mut forged = certificate.clone();
    forged.declared_close_premises[0].descriptor = 4;
    assert!(matches!(
        recheck_declared_close_premises(&program, &forged),
        Err(ClosePremiseRecheckError::Descriptor { operation: 1, .. })
    ));
    let mut forged = certificate.clone();
    forged.declared_close_premises[0].operation = 2;
    assert!(matches!(
        recheck_declared_close_premises(&program, &forged),
        Err(ClosePremiseRecheckError::Operation { expected: 1, .. })
    ));

    // An invented premise, and a dropped one.
    let mut forged = certificate.clone();
    let duplicate = forged.declared_close_premises[0].clone();
    forged.declared_close_premises.push(duplicate);
    assert!(matches!(
        recheck_declared_close_premises(&program, &forged),
        Err(ClosePremiseRecheckError::Inventory {
            expected: 1,
            recorded: 2
        })
    ));
    let mut forged = certificate.clone();
    forged.declared_close_premises.clear();
    assert!(matches!(
        recheck_declared_close_premises(&program, &forged),
        Err(ClosePremiseRecheckError::Inventory {
            expected: 1,
            recorded: 0
        })
    ));

    // A certificate paired with a program whose declaration cannot resolve, or
    // which declares a failure on a success arm, is refused outright.
    let unresolvable =
        declared_failure_program(Some(declared("linux", "eagain", "none-held")), Vec::new());
    assert!(matches!(
        recheck_declared_close_premises(&unresolvable, &certificate),
        Err(ClosePremiseRecheckError::RefusedDeclaration { operation: 1, .. })
    ));
    let contradictory = EventProgram::<TestProfile> {
        live_arguments: BTreeSet::new(),
        operations: vec![EventOperation::CloseSocket {
            descriptor: 3,
            succeeds: true,
            declared_failure: Some(declared("linux", "eio", "none-held")),
        }],
    };
    assert!(matches!(
        recheck_declared_close_premises(&contradictory, &certificate),
        Err(ClosePremiseRecheckError::DeclarationOnSuccessArm { operation: 0 })
    ));
}
