use super::*;
use crate::os_endpoint::{
    Duty, Errno, Outcome as EndpointOutcome, Premises, Target, close_poststate,
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
            },
            EventOperation::CloseSocket {
                descriptor: 4,
                succeeds: true,
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
        },
    ];
    operations.extend(tail);
    operations.push(EventOperation::CloseSocket {
        descriptor: 4,
        succeeds: true,
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
