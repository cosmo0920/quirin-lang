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

//! Parity between the retained in-process Libevent route and the new
//! process-portable reactor callback route (acceptance 3, parity before
//! deletion). The in-process route is **not** removed by this card; both routes
//! are run over the same programs and their answers compared exactly.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use qcore_compiler::source_profiles::{LIBEVENT_SOURCE_SET, load_profile};
use qcore_compiler::verifier_plugins::libevent::callback_task::{
    declared_build_profile, import_libevent_program, libevent_callback_task,
};
use qcore_compiler::verifier_plugins::libevent::event_driven::{
    ActivationReason, ArgumentMode, CallbackContext, CallbackSummary, DeleteMode, EventFlags,
    EventKey, EventOperation, EventProgram, LibeventBuildProfile, PayloadAccessKind,
    check_event_program,
};
use qcore_contract::profile_loader::LoadedProfile;
use qcore_schemas::plugin::{PluginId, TargetProfile, VerifierCapability};
use qcore_schemas::profile::BuildProfileSelector;
use qcore_schemas::reactor_program::ReactorCallbackWitness;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verification::reactor_source::{REACTOR_SEMANTIC_PROFILE, ReactorImportError};
use qcore_verifier_host::WitnessChecker;

/// The shipped `libevent` source-set profile: the accepted-build matrix both
/// routes consult is the declared data, not a permissive test fixture.
static PROFILE: LazyLock<LoadedProfile> =
    LazyLock::new(|| load_profile(LIBEVENT_SOURCE_SET).expect("libevent profile loads"));

fn builds() -> &'static [BuildProfileSelector] {
    &PROFILE
        .library(LIBEVENT_SOURCE_SET)
        .expect("the libevent profile declares its library")
        .accepted_builds
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

/// The same one-shot program the Cycle 18 unit tests exercise.
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

/// A persistent event that is deregistered with a blocking delete before its
/// parent teardown: the same family, a different registration shape.
fn persistent_with_delete() -> EventProgram {
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
    program
}

fn target() -> TargetProfile {
    TargetProfile {
        library: "libevent".into(),
        revision: "d82464a2".into(),
        profile: REACTOR_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: "linux".into(),
        backend: "native".into(),
    }
}

fn imported(
    program: &EventProgram,
) -> Result<
    qcore_verification::reactor_source::ReactorProgramImport<LibeventBuildProfile>,
    ReactorImportError,
> {
    import_libevent_program(
        program,
        builds(),
        "libevent",
        "d82464a2",
        BTreeMap::from([("library".into(), "libevent".into())]),
    )
}

#[test]
fn both_routes_decide_the_same_programs_identically() {
    // Accepted programs: the certificates are equal field for field, including
    // the freed event generations, the closed descriptors, the dropped
    // callback arguments and the whole observed trace.
    for (label, program) in [
        ("one-shot read callback", one_shot(1, key(1), 9)),
        (
            "persistent callback with blocking delete",
            persistent_with_delete(),
        ),
    ] {
        let in_process = check_event_program(&program, builds())
            .unwrap_or_else(|d| panic!("{label} must check in process: {d:?}"));
        let task = libevent_callback_task(imported(&program).expect(label), "reactor.c", target())
            .expect("derives");
        let portable = task
            .evaluate()
            .unwrap_or_else(|d| panic!("{label} must check on the portable route: {d:?}"));
        assert_eq!(in_process, portable, "certificate parity for {label}");
        assert_eq!(
            in_process.checked_operations,
            program.operations.len(),
            "every operation is checked for {label}"
        );
        assert_eq!(
            in_process.trace, portable.trace,
            "observation parity for {label}"
        );

        // The proposal a provider must return is accepted, and it discharges
        // exactly the core's own obligation set for this capability.
        let discharged = task
            .check(
                task.task(),
                &SemanticWitness::ReactorCallback(Box::new(ReactorCallbackWitness {
                    task_digest: task.task().task_digest(),
                    proposal: task.declared_proposal(),
                })),
            )
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(
            discharged,
            task.task().coverage_for(VerifierCapability::EventTraces)
        );
        // The registration inventory names the real descriptor-backed event
        // generation the program assigned.
        let proposal = task.declared_proposal();
        assert_eq!(proposal.callbacks.len(), 1);
        assert_eq!(proposal.callbacks[0].event.id, 1);
        assert_eq!(proposal.callbacks[0].event.generation, 1);
    }
}

#[test]
fn programs_the_in_process_route_refuses_are_refused_on_both_routes() {
    // Each of these is rejected in process today. On the portable route it is
    // either lost at import (no projection at all) or refused by the core's own
    // model run; in neither case can a witness discharge the obligations.
    let stale_generation = {
        let mut program = one_shot(1, key(1), 9);
        program.operations.push(EventOperation::Add {
            event: EventKey::new(1, 1),
        });
        program
    };
    let unauthorized_access = {
        let mut program = one_shot(1, key(1), 9);
        if let EventOperation::Assign { summary, .. } = &mut program.operations[1] {
            summary.payload_authority.clear();
        }
        program
    };
    let argument_use_after_free = {
        let mut program = one_shot(1, key(1), 9);
        // The callback argument is dropped while the event is still pending,
        // so the callback that later runs holds a freed argument.
        program
            .operations
            .insert(3, EventOperation::DropArgument { argument: 9 });
        program
    };
    let unquiescent_teardown = {
        let mut program = one_shot(1, key(1), 9);
        program.operations.remove(7);
        program
    };
    for (label, program) in [
        ("a stale event generation", stale_generation),
        (
            "a payload access the summary never authorized",
            unauthorized_access,
        ),
        (
            "a callback argument dropped while still pending",
            argument_use_after_free,
        ),
        ("teardown without callback quiescence", unquiescent_teardown),
    ] {
        assert!(
            check_event_program(&program, builds()).is_err(),
            "{label} must be refused in process"
        );
        match imported(&program) {
            Err(_) => {}
            Ok(import) => {
                let task = libevent_callback_task(import, "reactor.c", target()).expect("derives");
                assert!(task.evaluate().is_err(), "{label} must be refused portably");
                assert!(
                    task.check(
                        task.task(),
                        &SemanticWitness::ReactorCallback(Box::new(ReactorCallbackWitness {
                            task_digest: task.task().task_digest(),
                            proposal: task.declared_proposal(),
                        })),
                    )
                    .is_err(),
                    "{label} must not be dischargeable by any witness"
                );
            }
        }
    }
}

#[test]
fn an_unaccepted_build_stays_unknown_on_both_routes() {
    // The in-process route reports an unknown profile; the portable route
    // cannot even import the program. Absence of an accepted build is never
    // acceptance on either.
    let mut program = one_shot(1, key(1), 9);
    program.operations[0] = EventOperation::CreateBase {
        base: 1,
        profile: LibeventBuildProfile {
            openssl: true,
            ..LibeventBuildProfile::LINUX_EPOLL
        },
    };
    assert!(check_event_program(&program, builds()).is_err());
    assert!(matches!(
        imported(&program),
        Err(ReactorImportError::ProfileNotSingular | ReactorImportError::UnsupportedBackend)
    ));
    // An empty accepted-build matrix rejects the admitted program too.
    let admitted = one_shot(1, key(1), 9);
    assert!(check_event_program(&admitted, &[]).is_err());
    assert!(
        import_libevent_program(&admitted, &[], "libevent", "d82464a2", BTreeMap::new(),).is_err()
    );
}

#[test]
fn the_declared_build_profile_identifies_the_backend_and_threading_mode() {
    let declared = declared_build_profile(LibeventBuildProfile::LINUX_EPOLL);
    assert_eq!(declared["backend"], "epoll");
    assert_eq!(declared["threads"], "pthreads");
    // A different backend or threading mode is a different declared identity,
    // so it retires the recorded task rather than reusing its evidence.
    let program = one_shot(1, key(1), 9);
    let first = libevent_callback_task(imported(&program).unwrap(), "reactor.c", target()).unwrap();
    let relabelled = import_libevent_program(
        &program,
        builds(),
        "libevent",
        "d82464a2",
        BTreeMap::from([("library".into(), "other".into())]),
    )
    .unwrap();
    let second = libevent_callback_task(relabelled, "reactor.c", target()).unwrap();
    assert_ne!(first.task().task_digest(), second.task().task_digest());
    assert_ne!(sha256_hex(b""), first.task().task_digest());
    assert_eq!(
        first
            .request(PluginId::new("qcore.libevent").unwrap())
            .capability,
        VerifierCapability::EventTraces
    );
}
