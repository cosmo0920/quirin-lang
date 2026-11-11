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

//! Core-side derivation, import refusal and witness replay for the reactor
//! callback capability. Every case here is a rejection except the two that say
//! otherwise; no mutation may be admitted.
#[path = "fixtures/reactor_program.rs"]
mod fixture;
use fixture::*;

use qcore_core::event_reactor::{self as model, EventOperation};
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::reactor::{
    ActivationReasonRecord, PayloadAccessKindRecord, PayloadAuthorityRecord,
    REACTOR_CALLBACK_PROPOSAL_VERSION, ReactorCallbackProposal, ReactorCallbackRecord,
};
use qcore_schemas::reactor_program::{REACTOR_PROGRAM_INPUT_KEY, ReactorCallbackWitness};
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verification::reactor_source::*;
use qcore_verifier_host::WitnessChecker;
use std::collections::BTreeMap;

fn identity() -> ReactorProgramIdentity {
    ReactorProgramIdentity {
        library: "libevent".into(),
        source_revision: "pinned-source".into(),
        build_profile: BTreeMap::from([("backend".into(), "epoll".into())]),
        profile_assumptions: BTreeMap::from([("library".into(), "libevent".into())]),
    }
}

fn target() -> TargetProfile {
    TargetProfile {
        library: "libevent".into(),
        revision: "pinned-source".into(),
        profile: REACTOR_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: "linux".into(),
        backend: "native".into(),
    }
}

fn accepted() -> Vec<TestProfile> {
    vec![TestProfile(true)]
}

/// The sealed import is deliberately not `Debug` or `PartialEq`, so refusals
/// are compared through their error alone.
fn refusal(
    program: &model::EventProgram<TestProfile>,
    profiles: &[TestProfile],
    identity: ReactorProgramIdentity,
) -> Option<ReactorImportError> {
    import_reactor_program(program, profiles, identity).err()
}

/// The admitted program: a one-shot descriptor-backed read callback with a
/// declared payload write authority, quiescent teardown and parent free.
fn admitted() -> model::EventProgram<TestProfile> {
    one_shot(1, key(1), 9)
}

fn task_for(program: &model::EventProgram<TestProfile>) -> ReactorCallbackTask<TestProfile> {
    let import = import_reactor_program(program, &accepted(), identity()).expect("admitted family");
    ReactorCallbackTask::derive(import, "reactor.c", target()).expect("derives")
}

fn witness(
    task: &ReactorCallbackTask<TestProfile>,
    proposal: ReactorCallbackProposal,
) -> SemanticWitness {
    SemanticWitness::ReactorCallback(Box::new(ReactorCallbackWitness {
        task_digest: task.task().task_digest(),
        proposal,
    }))
}

#[test]
fn the_admitted_family_derives_two_obligations_and_accepts_its_own_inventory() {
    let task = task_for(&admitted());
    // Both reserved event-trace properties, over the whole unit, from one run.
    let properties: Vec<_> = task
        .task()
        .obligations
        .iter()
        .map(|obligation| obligation.property)
        .collect();
    assert_eq!(
        properties,
        vec![
            ObligationProperty::EventGenerationDischarged,
            ObligationProperty::CallbackQuiescentAtTeardown
        ]
    );
    assert!(
        task.task()
            .obligations
            .iter()
            .all(|obligation| obligation.subject == ObligationSubject::Unit
                && obligation.capability == VerifierCapability::EventTraces)
    );
    // The core's own model run succeeds and records the callback custody the
    // capability is about: the freed event generation and the dropped argument.
    let certificate = task.evaluate().expect("the admitted program checks");
    assert_eq!(certificate.freed_events, [key(1)].into());
    assert_eq!(certificate.dropped_arguments, [9].into());

    let proposal = task.declared_proposal();
    assert_eq!(proposal.callbacks.len(), 1);
    assert_eq!(proposal.callbacks[0].operation_index, 1);
    assert_eq!(proposal.callbacks[0].event.generation, 1);
    let discharged = task
        .check(task.task(), &witness(&task, proposal))
        .expect("the core's own inventory is accepted");
    assert_eq!(
        discharged,
        task.task().coverage_for(VerifierCapability::EventTraces)
    );
    assert_eq!(discharged.len(), 2);

    // The request carries the program only: no proposal, no certificate.
    let request = task.request(PluginId::new("qcore.libevent").unwrap());
    request.validate().expect("the request is well formed");
    let json = serde_json::to_value(request.reactor_program.as_ref().unwrap()).unwrap();
    assert!(json.get("callbacks").is_none() && json.get("proposal").is_none());
    assert!(json.get("operations").is_some());
    assert_eq!(
        task.task().contract_sources[REACTOR_PROGRAM_INPUT_KEY],
        request.reactor_program.as_ref().unwrap().digest()
    );
}

/// One named edit to the admitted program.
type Mutation = (
    &'static str,
    Box<dyn Fn(&mut model::EventProgram<TestProfile>)>,
);

#[test]
fn unsupported_operations_and_backends_never_import() {
    let unsupported: Vec<Mutation> = vec![
        (
            "bufferevent creation",
            Box::new(|program| {
                program.operations.insert(
                    2,
                    EventOperation::CreateBufferevent {
                        bev: model::BuffereventKey::new(1, 1),
                        base: 1,
                        descriptor: 4,
                        argument: 9,
                        close_on_free: true,
                        thread_safe: true,
                        deferred: false,
                        unlock_callbacks: false,
                        payload_authority: BTreeMap::new(),
                    },
                )
            }),
        ),
        (
            "socket pair custody",
            Box::new(|program| {
                program.operations.insert(
                    1,
                    EventOperation::CreateSocketPair {
                        descriptors: Some((3, 4)),
                    },
                )
            }),
        ),
        (
            "socket close custody",
            Box::new(|program| {
                program.operations.insert(
                    1,
                    EventOperation::CloseSocket {
                        descriptor: 3,
                        succeeds: true,
                        declared_failure: None,
                    },
                )
            }),
        ),
        (
            // Card O03-REACTOR: a declared close-failure premise has no wire
            // record either, so it can never travel silently or be dropped on
            // the way to a provider. The whole program stays Unknown.
            "declared close-failure premise",
            Box::new(|program| {
                program.operations.insert(
                    1,
                    EventOperation::CloseSocket {
                        descriptor: 3,
                        succeeds: false,
                        declared_failure: Some(model::DeclaredCloseFailure {
                            target: "linux".to_string(),
                            errno: "eio".to_string(),
                            record_locks: "none-held".to_string(),
                        }),
                    },
                )
            }),
        ),
        (
            "socket error query",
            Box::new(|program| {
                program
                    .operations
                    .insert(1, EventOperation::GetSocketError { descriptor: 3 })
            }),
        ),
        (
            "ordering assumption",
            Box::new(|program| {
                program.operations.insert(
                    2,
                    EventOperation::AssumeOrdering {
                        evidence: model::InvalidOrderingEvidence::Priority,
                    },
                )
            }),
        ),
        (
            "loop control",
            Box::new(|program| {
                program.operations.insert(
                    4,
                    EventOperation::LoopRequest {
                        base: 1,
                        request: model::LoopControl::Break,
                        cross_thread: false,
                    },
                )
            }),
        ),
    ];
    for (label, change) in unsupported {
        let mut program = admitted();
        change(&mut program);
        assert!(
            matches!(
                refusal(&program, &accepted(), identity()),
                Some(ReactorImportError::UnsupportedOperation(_))
            ),
            "{label} must keep its callbacks Unknown"
        );
    }

    // An access attributed to a bufferevent context has no record either.
    let mut program = admitted();
    program.operations[6] = EventOperation::Access {
        context: model::CallbackContext::Bufferevent(model::BuffereventKey::new(1, 1)),
        payload: 7,
        kind: model::PayloadAccessKind::Write,
        synchronized: false,
    };
    assert_eq!(
        refusal(&program, &accepted(), identity()),
        Some(ReactorImportError::UnsupportedAccessContext(6))
    );

    // Absence is never acceptance: an empty accepted-build list, an unmatched
    // profile and a second declared profile each refuse.
    assert_eq!(
        refusal(&admitted(), &[], identity()),
        Some(ReactorImportError::UnsupportedBackend)
    );
    assert_eq!(
        refusal(&admitted(), &[TestProfile(false)], identity()),
        Some(ReactorImportError::ProfileNotSingular)
    );
    let mut two_bases = admitted();
    two_bases.operations.insert(
        1,
        EventOperation::CreateBase {
            base: 2,
            profile: TestProfile(true),
        },
    );
    assert_eq!(
        refusal(&two_bases, &accepted(), identity()),
        Some(ReactorImportError::ProfileNotSingular)
    );

    // Empty programs and incomplete identities refuse before any projection.
    assert_eq!(
        refusal(&model::EventProgram::default(), &accepted(), identity()),
        Some(ReactorImportError::EmptyProgram)
    );
    for empty in ["library", "revision"] {
        let mut broken = identity();
        if empty == "library" {
            broken.library.clear();
        } else {
            broken.source_revision.clear();
        }
        assert_eq!(
            refusal(&admitted(), &accepted(), broken),
            Some(ReactorImportError::IncompleteIdentity)
        );
    }
}

#[test]
fn a_program_the_model_refuses_can_never_be_bought_by_a_proposal() {
    // A stale generation: the assignment names generation 1, the teardown a
    // different one. The core's own model run rejects it, so no proposal --
    // not even the core's own exported inventory -- discharges anything.
    let mut stale = admitted();
    stale.operations[9] = EventOperation::FreeEvent { event: key(1) };
    stale.operations[1] = match &stale.operations[1] {
        EventOperation::Assign {
            base,
            descriptor,
            flags,
            priority,
            summary,
            ..
        } => EventOperation::Assign {
            event: model::EventKey::new(1, 2),
            base: *base,
            descriptor: *descriptor,
            flags: *flags,
            priority: *priority,
            summary: summary.clone(),
        },
        other => other.clone(),
    };
    let task = task_for(&stale);
    assert!(task.evaluate().is_err(), "the model must refuse it");
    let proposal = task.declared_proposal();
    let error = task
        .check(task.task(), &witness(&task, proposal))
        .expect_err("a correct inventory cannot repair a refused program");
    assert!(error.contains("the reactor model refused"), "{error}");

    // Callback authority the program never granted: the access writes a
    // payload the summary does not authorize.
    let mut unauthorized = admitted();
    if let EventOperation::Assign { summary, .. } = &mut unauthorized.operations[1] {
        summary.payload_authority.clear();
    }
    let task = task_for(&unauthorized);
    assert!(task.evaluate().is_err());
    let proposal = task.declared_proposal();
    assert!(task.check(task.task(), &witness(&task, proposal)).is_err());

    // Teardown that is not quiescent: the event is freed inside its callback.
    let mut unquiescent = admitted();
    unquiescent.operations.remove(7);
    let task = task_for(&unquiescent);
    assert!(task.evaluate().is_err());
    assert!(
        task.check(task.task(), &witness(&task, task.declared_proposal()))
            .is_err()
    );
}

#[test]
fn forged_inventories_are_refused_against_the_cores_own_assignments() {
    let task = task_for(&admitted());
    let base = task.declared_proposal();

    let mutations: Vec<(&str, ReactorCallbackProposal)> = vec![
        ("empty inventory", {
            let mut value = base.clone();
            value.callbacks.clear();
            value
        }),
        ("unsupported proposal version", {
            let mut value = base.clone();
            value.schema_version = REACTOR_CALLBACK_PROPOSAL_VERSION + 1;
            value
        }),
        ("duplicated record", {
            let mut value = base.clone();
            let repeat = value.callbacks[0].clone();
            value.callbacks.push(repeat);
            value
        }),
        ("record at an operation that is not an assignment", {
            let mut value = base.clone();
            value.callbacks[0].operation_index = 2;
            value
        }),
        ("record beyond the program", {
            let mut value = base.clone();
            value.callbacks[0].operation_index = 9_999;
            value
        }),
        ("retargeted event identity", {
            let mut value = base.clone();
            value.callbacks[0].event.id = 2;
            value
        }),
        ("retargeted generation", {
            let mut value = base.clone();
            value.callbacks[0].event.generation = 2;
            value
        }),
        ("widened payload authority", {
            let mut value = base.clone();
            value.callbacks[0]
                .summary
                .payload_authority
                .push(PayloadAuthorityRecord {
                    payload: 8,
                    kind: PayloadAccessKindRecord::Write,
                });
            value
        }),
        ("escalated payload access kind", {
            let mut value = base.clone();
            value.callbacks[0].summary.payload_authority[0].kind =
                PayloadAccessKindRecord::AtomicWrite;
            value
        }),
        ("widened trigger reasons", {
            let mut value = base.clone();
            value.callbacks[0]
                .summary
                .trigger_reasons
                .push(ActivationReasonRecord::Explicit);
            value
        }),
        ("duplicated trigger reason", {
            let mut value = base.clone();
            let repeat = value.callbacks[0].summary.trigger_reasons[0];
            value.callbacks[0].summary.trigger_reasons.push(repeat);
            value
        }),
        ("duplicated payload permission", {
            let mut value = base.clone();
            let repeat = value.callbacks[0].summary.payload_authority[0];
            value.callbacks[0].summary.payload_authority.push(repeat);
            value
        }),
        ("relabelled argument mode", {
            let mut value = base.clone();
            value.callbacks[0].summary.argument_mode =
                qcore_schemas::reactor::ArgumentModeRecord::Borrowed;
            value
        }),
        ("retargeted callback argument", {
            let mut value = base.clone();
            value.callbacks[0].summary.argument = 11;
            value
        }),
        ("granted deferral", {
            let mut value = base.clone();
            value.callbacks[0].summary.may_defer = true;
            value
        }),
        ("granted task spawning", {
            let mut value = base.clone();
            value.callbacks[0].summary.may_spawn_tasks = true;
            value
        }),
        ("an invented additional assignment", {
            let mut value = base.clone();
            value.callbacks.push(ReactorCallbackRecord {
                operation_index: 3,
                event: value.callbacks[0].event,
                summary: value.callbacks[0].summary.clone(),
            });
            value
        }),
    ];
    for (label, proposal) in mutations {
        assert!(
            task.check(task.task(), &witness(&task, proposal)).is_err(),
            "{label} must be refused"
        );
    }
}

#[test]
fn witness_binding_task_identity_and_kind_are_all_checked() {
    let task = task_for(&admitted());
    let proposal = task.declared_proposal();

    // A witness of another kind never satisfies this checker.
    assert!(
        task.check(
            task.task(),
            &SemanticWitness::ContractObservation(
                qcore_schemas::witness::ContractObservationWitness {
                    task_digest: task.task().task_digest(),
                    runs: Vec::new(),
                }
            )
        )
        .is_err()
    );
    // A witness bound to another task digest is refused before replay.
    assert!(
        task.check(
            task.task(),
            &SemanticWitness::ReactorCallback(Box::new(ReactorCallbackWitness {
                task_digest: sha256_hex(b"another task"),
                proposal: proposal.clone(),
            }))
        )
        .is_err()
    );
    // A task the checker did not derive is refused, even with a real witness.
    let mut foreign = task.task().clone();
    foreign.unit.translation_unit = "other.c".into();
    assert!(
        task.check(&foreign, &witness(&task, proposal.clone()))
            .is_err()
    );

    // Changed program bytes, a changed build profile and changed declared
    // assumptions each produce a different task, and the cross-task witness
    // is refused in both directions.
    let mut different_build = identity();
    different_build
        .build_profile
        .insert("backend".into(), "poll".into());
    let mut different_assumptions = identity();
    different_assumptions
        .profile_assumptions
        .insert("library".into(), "other".into());
    let mut different_revision = identity();
    different_revision.source_revision = "moved".into();
    for changed in [different_build, different_assumptions] {
        let import =
            import_reactor_program(&admitted(), &accepted(), changed).expect("admitted family");
        let other = ReactorCallbackTask::derive(import, "reactor.c", target()).expect("derives");
        assert_ne!(other.task().task_digest(), task.task().task_digest());
        assert!(
            other
                .check(other.task(), &witness(&task, proposal.clone()))
                .is_err()
        );
        assert!(
            task.check(task.task(), &witness(&other, proposal.clone()))
                .is_err()
        );
    }
    // A moved revision must also move the target that binds it.
    let import = import_reactor_program(&admitted(), &accepted(), different_revision)
        .expect("admitted family");
    assert!(
        ReactorCallbackTask::derive(import, "reactor.c", target()).is_err(),
        "a target that does not name the imported revision is refused"
    );

    // A changed program changes the task, and the exported inventory is
    // index-sensitive: an operation inserted before the assignment moves the
    // recorded registration index, so the old proposal no longer matches.
    let mut longer = admitted();
    longer
        .operations
        .insert(1, EventOperation::Reschedule { event: key(1) });
    let other = task_for(&longer);
    assert_ne!(other.task().task_digest(), task.task().task_digest());
    assert_eq!(other.declared_proposal().callbacks[0].operation_index, 2);
    assert_ne!(other.declared_proposal(), proposal);
    assert!(
        other
            .check(other.task(), &witness(&other, proposal.clone()))
            .is_err(),
        "an inventory recorded at a stale index is refused"
    );
}

#[test]
fn incompatible_targets_and_translation_units_refuse_derivation() {
    let import =
        import_reactor_program(&admitted(), &accepted(), identity()).expect("admitted family");
    assert!(ReactorCallbackTask::derive(import, "", target()).is_err());
    for change in [
        |t: &mut TargetProfile| t.profile = "qcore.c_counter_contract.v1".into(),
        |t: &mut TargetProfile| t.library = "chunkio".into(),
        |t: &mut TargetProfile| t.revision = "other".into(),
        |t: &mut TargetProfile| t.dialect.clear(),
        |t: &mut TargetProfile| t.platform.clear(),
        |t: &mut TargetProfile| t.backend = "*".into(),
    ] {
        let mut broken = target();
        change(&mut broken);
        let import =
            import_reactor_program(&admitted(), &accepted(), identity()).expect("admitted family");
        assert!(
            ReactorCallbackTask::derive(import, "reactor.c", broken).is_err(),
            "an incompatible target must refuse derivation"
        );
    }
}

#[test]
fn a_reactor_projection_is_admitted_only_with_its_own_obligation_pair() {
    let task = task_for(&admitted());
    let plugin = PluginId::new("qcore.libevent").unwrap();
    let base = task.request(plugin.clone());
    base.validate().expect("well formed");

    // An unsupported input version, a dropped obligation, a foreign capability
    // and a co-travelling source projection each fail validation.
    let mut wrong_version = base.clone();
    wrong_version
        .reactor_program
        .as_mut()
        .unwrap()
        .schema_version = 0;
    assert!(wrong_version.validate().is_err());

    let mut one_obligation = base.clone();
    one_obligation.obligations.truncate(1);
    assert!(one_obligation.validate().is_err());

    let mut no_projection = base.clone();
    no_projection.reactor_program = None;
    assert!(
        no_projection.validate().is_err(),
        "the quiescence obligation requires a projection"
    );

    let mut both = base.clone();
    both.source_effect = serde_json::from_str(
        r#"{"schema_version":2,"function":"f","source_revision":"x","field":"Locked",
            "program":{"Return":"Field"},"dependencies":{},"profile_assumptions":{}}"#,
    )
    .unwrap();
    assert!(both.validate().is_err());
}
