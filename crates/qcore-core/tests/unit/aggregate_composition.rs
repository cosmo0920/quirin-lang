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

//! Counterexample suite for the bounded aggregate-lifecycle composition (O05).
//!
//! The reference program is shaped after the aggregate the Cycle 19 evidence
//! measured — a container holding a child object, its name allocation, a backend
//! child and that backend's descriptor and mapping endpoints — plus a separate
//! wrapper object with its own two allocations, one publication and a borrowed
//! hold on the child. The names here are deliberately generic: nothing in the
//! relation reads a symbol, and no claim about any upstream codebase follows
//! from a test in this file.
//!
//! Every negative row asserts three things: the verdict is `Unknown`, the exact
//! typed refusal is present, and no certificate is published. The reference
//! program composing (`the_reference_aggregate_and_wrapper_compose`) is what
//! makes each refusal attributable to its own mutation.

use super::*;

const ROOT: MemberId = 1;
const CHILD: MemberId = 2;
const CHILD_NAME: MemberId = 3;
const BACKEND: MemberId = 4;
const DESCRIPTOR: MemberId = 5;
const MAPPING: MemberId = 6;
const WRAPPER_OBJECT: MemberId = 7;
const WRAPPER_TABLE: MemberId = 8;
const REUSED_DESCRIPTOR: MemberId = 9;

const CHILD_PAYLOAD: PayloadId = 10;
const MAPPING_PAYLOAD: PayloadId = 11;
const FOREIGN_PAYLOAD: PayloadId = 99;

const PUBLICATION: PublicationId = 1;
const HARNESS: CustodianId = 42;
const DESCRIPTOR_NUMBER: RawAddress = 7;
const MAPPING_ADDRESS: RawAddress = 0x7f00_0000;

fn member(
    id: MemberId,
    kind: MemberKind,
    generation: Generation,
    owner: Owner,
    parent: Option<MemberId>,
) -> AggregateMember {
    AggregateMember {
        id,
        kind,
        generation,
        owner,
        parent,
        address: None,
        payloads: BTreeSet::new(),
    }
}

fn released(member: MemberId, by: Releaser, evidence: CustodyEvidence) -> LifecycleStep {
    LifecycleStep::Release(ReleaseStep {
        member,
        by,
        disposition: Disposition::Released,
        evidence,
    })
}

fn reference_members() -> Vec<AggregateMember> {
    let mut child = member(
        CHILD,
        MemberKind::Allocation,
        1,
        Owner::Aggregate,
        Some(ROOT),
    );
    child.payloads.insert(CHILD_PAYLOAD);
    let mut descriptor = member(
        DESCRIPTOR,
        MemberKind::Endpoint(EndpointKind::Descriptor),
        1,
        Owner::Aggregate,
        Some(BACKEND),
    );
    descriptor.address = Some(DESCRIPTOR_NUMBER);
    let mut mapping = member(
        MAPPING,
        MemberKind::Endpoint(EndpointKind::Mapping),
        1,
        Owner::Aggregate,
        Some(BACKEND),
    );
    mapping.address = Some(MAPPING_ADDRESS);
    mapping.payloads.insert(MAPPING_PAYLOAD);
    vec![
        member(ROOT, MemberKind::Aggregate, 1, Owner::Aggregate, None),
        child,
        member(
            CHILD_NAME,
            MemberKind::Allocation,
            2,
            Owner::Aggregate,
            Some(CHILD),
        ),
        member(
            BACKEND,
            MemberKind::Allocation,
            3,
            Owner::Aggregate,
            Some(CHILD),
        ),
        descriptor,
        mapping,
        member(
            WRAPPER_OBJECT,
            MemberKind::Allocation,
            4,
            Owner::Wrapper,
            None,
        ),
        member(
            WRAPPER_TABLE,
            MemberKind::Allocation,
            5,
            Owner::Wrapper,
            Some(WRAPPER_OBJECT),
        ),
    ]
}

/// Index of the first parent release in the reference schedule.
const TEARDOWN_POINT: usize = 13;

fn reference_schedule() -> Vec<LifecycleStep> {
    vec![
        LifecycleStep::Acquire { member: ROOT },
        LifecycleStep::Acquire { member: CHILD },
        LifecycleStep::Acquire { member: CHILD_NAME },
        LifecycleStep::Acquire { member: BACKEND },
        LifecycleStep::Acquire { member: DESCRIPTOR },
        LifecycleStep::Acquire { member: MAPPING },
        LifecycleStep::Acquire {
            member: WRAPPER_OBJECT,
        },
        LifecycleStep::Acquire {
            member: WRAPPER_TABLE,
        },
        LifecycleStep::Publish {
            publication: PUBLICATION,
        },
        LifecycleStep::Reclaim {
            publication: PUBLICATION,
        },
        LifecycleStep::DropBorrow,
        released(
            WRAPPER_TABLE,
            Releaser::Wrapper,
            CustodyEvidence::ExitResidualDischarged,
        ),
        released(
            WRAPPER_OBJECT,
            Releaser::Wrapper,
            CustodyEvidence::ExitResidualDischarged,
        ),
        // teardown point
        released(
            MAPPING,
            Releaser::Parent,
            CustodyEvidence::ObservedEndpointPostState,
        ),
        LifecycleStep::Release(ReleaseStep {
            member: DESCRIPTOR,
            by: Releaser::Parent,
            disposition: Disposition::Retained {
                custodian: Some(HARNESS),
            },
            evidence: CustodyEvidence::ObservedEndpointPostState,
        }),
        released(
            BACKEND,
            Releaser::Parent,
            CustodyEvidence::DeclaredDestructorContract,
        ),
        released(
            CHILD_NAME,
            Releaser::Parent,
            CustodyEvidence::DeclaredDestructorContract,
        ),
        released(
            CHILD,
            Releaser::Parent,
            CustodyEvidence::DeclaredDestructorContract,
        ),
        released(
            ROOT,
            Releaser::Parent,
            CustodyEvidence::DeclaredDestructorContract,
        ),
    ]
}

fn reference() -> AggregateCompositionProgram {
    AggregateCompositionProgram {
        root: ROOT,
        members: reference_members(),
        schedule: reference_schedule(),
        callbacks: vec![CallbackRecord {
            id: 1,
            state: CallbackState::Quiescent,
            authority: BTreeSet::from([CHILD_PAYLOAD, MAPPING_PAYLOAD]),
            admitted: true,
            spawned_tasks: BTreeSet::from([1]),
        }],
        tasks: vec![TaskRecord {
            id: 1,
            state: TaskState::Joined,
            captures: BTreeSet::from([CHILD]),
        }],
        guards: vec![GuardRecord {
            id: 1,
            state: GuardState::Released,
            footprint: 1,
        }],
        footprints: vec![FootprintRecord {
            id: 1,
            members: BTreeSet::from([CHILD, MAPPING]),
        }],
        wrapper: Some(WrapperLifecycle {
            held_member: CHILD,
            hold: HoldMode::Borrowed,
            publications: BTreeSet::from([PUBLICATION]),
        }),
        external_custodians: BTreeSet::from([HARNESS]),
        unsupported: BTreeSet::new(),
        declared_premises: BTreeSet::from([
            DeclaredPremise::ConstructorContract,
            DeclaredPremise::DestructorContract,
            DeclaredPremise::BackendProfile,
            DeclaredPremise::CallbackAuthorityDeclared,
            DeclaredPremise::GuardFootprintDeclared,
            DeclaredPremise::TaskLedgerSupplied,
            DeclaredPremise::ExitResidualsSupplied,
            DeclaredPremise::EndpointPostStateSupplied,
        ]),
        durability_premises: BTreeSet::from([
            DurabilityPremise::SyncSucceeded,
            DurabilityPremise::DataReachedStableStorage,
        ]),
    }
}

/// Replace the release step naming `member` with `replacement`.
fn replace_release(
    program: &mut AggregateCompositionProgram,
    target: MemberId,
    replacement: LifecycleStep,
) {
    let index = program
        .schedule
        .iter()
        .position(|step| matches!(step, LifecycleStep::Release(r) if r.member == target))
        .expect("release step present");
    program.schedule[index] = replacement;
}

fn drop_release(program: &mut AggregateCompositionProgram, target: MemberId) {
    program
        .schedule
        .retain(|step| !matches!(step, LifecycleStep::Release(r) if r.member == target));
}

fn refused(program: &AggregateCompositionProgram, refusal: CompositionRefusal) {
    let result = check_aggregate_composition(program);
    assert_eq!(
        result.verdict,
        AggregateVerdict::Unknown,
        "expected Unknown, got {result:?}"
    );
    assert!(
        result.certificate.is_none(),
        "a refused composition published a certificate: {result:?}"
    );
    assert!(
        result.refuses(&refusal),
        "expected {refusal:?}, got {:?}",
        result.refusals
    );
}

// --------------------------------------------------------------------------
// Positive rows
// --------------------------------------------------------------------------

#[test]
fn the_reference_aggregate_and_wrapper_compose() {
    let result = check_aggregate_composition(&reference());
    assert_eq!(result.verdict, AggregateVerdict::Composed, "{result:?}");
    assert!(result.refusals.is_empty());
    let certificate = result.certificate.expect("certificate");
    assert_eq!(certificate.root, ROOT);
    assert_eq!(certificate.teardown_point, TEARDOWN_POINT);
    assert_eq!(
        certificate.released,
        BTreeSet::from([
            ROOT,
            CHILD,
            CHILD_NAME,
            BACKEND,
            MAPPING,
            WRAPPER_OBJECT,
            WRAPPER_TABLE
        ])
    );
    assert_eq!(
        certificate.retained_endpoints,
        BTreeMap::from([(DESCRIPTOR, HARNESS)])
    );
    assert_eq!(certificate.distinct_identities, 8);
}

#[test]
fn an_owned_hold_the_wrapper_alone_releases_composes() {
    let mut program = reference();
    program.wrapper.as_mut().expect("wrapper").hold = HoldMode::Owned;
    program
        .schedule
        .retain(|step| !matches!(step, LifecycleStep::DropBorrow));
    replace_release(
        &mut program,
        CHILD,
        released(
            CHILD,
            Releaser::Wrapper,
            CustodyEvidence::DeclaredDestructorContract,
        ),
    );
    let result = check_aggregate_composition(&program);
    assert_eq!(result.verdict, AggregateVerdict::Composed, "{result:?}");
}

#[test]
fn a_callback_without_authority_over_a_member_need_not_be_quiescent() {
    let mut program = reference();
    program.callbacks.push(CallbackRecord {
        id: 2,
        state: CallbackState::Running,
        authority: BTreeSet::from([FOREIGN_PAYLOAD]),
        admitted: true,
        spawned_tasks: BTreeSet::new(),
    });
    let result = check_aggregate_composition(&program);
    assert_eq!(result.verdict, AggregateVerdict::Composed, "{result:?}");
    assert_eq!(
        result.certificate.expect("certificate").quiescent_callbacks,
        BTreeSet::from([1])
    );
}

#[test]
fn a_task_capturing_nothing_in_the_aggregate_need_not_be_joined() {
    let mut program = reference();
    program.tasks.push(TaskRecord {
        id: 2,
        state: TaskState::Outstanding,
        captures: BTreeSet::new(),
    });
    assert!(check_aggregate_composition(&program).composed());
}

/// The identity separation rule permits raw reuse only across disjoint live
/// intervals: the same descriptor number after the earlier epoch was released.
fn with_descriptor_reuse(first_released: bool) -> AggregateCompositionProgram {
    let mut program = reference();
    let mut reused = member(
        REUSED_DESCRIPTOR,
        MemberKind::Endpoint(EndpointKind::Descriptor),
        2,
        Owner::Aggregate,
        Some(BACKEND),
    );
    reused.address = Some(DESCRIPTOR_NUMBER);
    program.members.push(reused);
    if first_released {
        replace_release(
            &mut program,
            DESCRIPTOR,
            released(
                DESCRIPTOR,
                Releaser::Parent,
                CustodyEvidence::ObservedEndpointPostState,
            ),
        );
    }
    let descriptor_release = program
        .schedule
        .iter()
        .position(|step| matches!(step, LifecycleStep::Release(r) if r.member == DESCRIPTOR))
        .expect("descriptor release");
    program.schedule.insert(
        descriptor_release + 1,
        LifecycleStep::Acquire {
            member: REUSED_DESCRIPTOR,
        },
    );
    program.schedule.insert(
        descriptor_release + 2,
        released(
            REUSED_DESCRIPTOR,
            Releaser::Parent,
            CustodyEvidence::ObservedEndpointPostState,
        ),
    );
    program
}

#[test]
fn a_raw_descriptor_number_may_be_reused_after_its_epoch_is_released() {
    let result = check_aggregate_composition(&with_descriptor_reuse(true));
    assert_eq!(result.verdict, AggregateVerdict::Composed, "{result:?}");
    assert_eq!(
        result.certificate.expect("certificate").distinct_identities,
        9
    );
}

#[test]
fn the_certificate_states_durability_premises_separately() {
    let certificate = check_aggregate_composition(&reference())
        .certificate
        .expect("certificate");
    assert_eq!(
        certificate.durability_premises,
        BTreeSet::from([
            DurabilityPremise::SyncSucceeded,
            DurabilityPremise::DataReachedStableStorage,
        ])
    );
    // A durability premise is never also a declared premise the composition
    // derives: the two sets are separate fields and neither is discharged.
    assert!(
        certificate
            .declared_premises
            .contains(&DeclaredPremise::EndpointPostStateSupplied)
    );
    assert_eq!(certificate.declared_premises.len(), 8);
}

// --------------------------------------------------------------------------
// Acceptance 1 — identity separation across parent release
// --------------------------------------------------------------------------

#[test]
fn two_members_may_not_share_a_logical_identity() {
    let mut program = reference();
    program.members[2].generation = 1; // CHILD_NAME collides with CHILD
    refused(
        &program,
        CompositionRefusal::DuplicateMemberIdentity {
            first: CHILD,
            second: CHILD_NAME,
        },
    );
}

#[test]
fn a_member_that_is_never_acquired_refuses() {
    let mut program = reference();
    program
        .schedule
        .retain(|step| !matches!(step, LifecycleStep::Acquire { member } if *member == BACKEND));
    refused(
        &program,
        CompositionRefusal::MemberNeverAcquired { member: BACKEND },
    );
}

#[test]
fn a_member_acquired_twice_refuses() {
    let mut program = reference();
    program
        .schedule
        .insert(4, LifecycleStep::Acquire { member: BACKEND });
    refused(
        &program,
        CompositionRefusal::MemberAcquiredTwice { member: BACKEND },
    );
}

#[test]
fn a_child_allocation_left_unreleased_refuses() {
    let mut program = reference();
    drop_release(&mut program, CHILD_NAME);
    refused(
        &program,
        CompositionRefusal::MemberNotReleased { member: CHILD_NAME },
    );
}

#[test]
fn a_member_released_twice_refuses() {
    let mut program = reference();
    program.schedule.push(released(
        CHILD_NAME,
        Releaser::Parent,
        CustodyEvidence::DeclaredDestructorContract,
    ));
    refused(
        &program,
        CompositionRefusal::MemberReleasedTwice { member: CHILD_NAME },
    );
}

#[test]
fn a_release_before_its_acquisition_refuses() {
    let mut program = reference();
    let step = released(
        CHILD_NAME,
        Releaser::Parent,
        CustodyEvidence::DeclaredDestructorContract,
    );
    drop_release(&mut program, CHILD_NAME);
    program.schedule.insert(0, step);
    refused(
        &program,
        CompositionRefusal::ReleaseBeforeAcquire { member: CHILD_NAME },
    );
}

#[test]
fn a_raw_address_reused_while_the_earlier_epoch_is_retained_refuses() {
    // The reference descriptor is `Retained`, so it is still live past the end
    // of the schedule and its number may not be taken by a new epoch.
    refused(
        &with_descriptor_reuse(false),
        CompositionRefusal::AddressReusedWhileLive {
            earlier: DESCRIPTOR,
            later: REUSED_DESCRIPTOR,
            address: DESCRIPTOR_NUMBER,
        },
    );
}

#[test]
fn a_raw_address_reused_before_the_earlier_epoch_is_released_refuses() {
    let mut program = with_descriptor_reuse(true);
    // Move the second acquisition before the first release.
    program.schedule.retain(
        |step| !matches!(step, LifecycleStep::Acquire { member } if *member == REUSED_DESCRIPTOR),
    );
    program.schedule.insert(
        6,
        LifecycleStep::Acquire {
            member: REUSED_DESCRIPTOR,
        },
    );
    refused(
        &program,
        CompositionRefusal::AddressReusedWhileLive {
            earlier: DESCRIPTOR,
            later: REUSED_DESCRIPTOR,
            address: DESCRIPTOR_NUMBER,
        },
    );
}

#[test]
fn an_unresolved_disposition_refuses_and_alleges_nothing() {
    let mut program = reference();
    replace_release(
        &mut program,
        MAPPING,
        LifecycleStep::Release(ReleaseStep {
            member: MAPPING,
            by: Releaser::Parent,
            disposition: Disposition::Unresolved(UnresolvedReason::SuppressedSyscall),
            evidence: CustodyEvidence::ObservedEndpointPostState,
        }),
    );
    let result = check_aggregate_composition(&program);
    assert_eq!(result.verdict, AggregateVerdict::Unknown);
    assert!(result.refuses(&CompositionRefusal::UnresolvedDisposition {
        member: MAPPING,
        reason: UnresolvedReason::SuppressedSyscall,
    }));
    // No refusal claims the member leaked or was released twice.
    assert!(!result.refuses(&CompositionRefusal::MemberNotReleased { member: MAPPING }));
    assert!(!result.refuses(&CompositionRefusal::MemberReleasedTwice { member: MAPPING }));
}

#[test]
fn each_unresolved_reason_refuses_on_its_own() {
    for reason in [
        UnresolvedReason::SuppressedSyscall,
        UnresolvedReason::UncontractedCallee,
        UnresolvedReason::UnmodeledBranch,
        UnresolvedReason::UnknownBackend,
        UnresolvedReason::UnmodeledAggregateStore,
    ] {
        let mut program = reference();
        replace_release(
            &mut program,
            BACKEND,
            LifecycleStep::Release(ReleaseStep {
                member: BACKEND,
                by: Releaser::Parent,
                disposition: Disposition::Unresolved(reason),
                evidence: CustodyEvidence::DeclaredDestructorContract,
            }),
        );
        refused(
            &program,
            CompositionRefusal::UnresolvedDisposition {
                member: BACKEND,
                reason,
            },
        );
    }
}

#[test]
fn a_retained_endpoint_without_a_custodian_refuses() {
    let mut program = reference();
    replace_release(
        &mut program,
        DESCRIPTOR,
        LifecycleStep::Release(ReleaseStep {
            member: DESCRIPTOR,
            by: Releaser::Parent,
            disposition: Disposition::Retained { custodian: None },
            evidence: CustodyEvidence::ObservedEndpointPostState,
        }),
    );
    refused(
        &program,
        CompositionRefusal::RetainedEndpointWithoutCustodian { member: DESCRIPTOR },
    );
}

#[test]
fn a_retained_endpoint_handed_to_an_undeclared_custodian_refuses() {
    let mut program = reference();
    program.external_custodians.clear();
    refused(
        &program,
        CompositionRefusal::RetainedEndpointCustodianNotExternal {
            member: DESCRIPTOR,
            custodian: HARNESS,
        },
    );
}

#[test]
fn a_retained_heap_allocation_refuses() {
    let mut program = reference();
    replace_release(
        &mut program,
        CHILD_NAME,
        LifecycleStep::Release(ReleaseStep {
            member: CHILD_NAME,
            by: Releaser::Parent,
            disposition: Disposition::Retained {
                custodian: Some(HARNESS),
            },
            evidence: CustodyEvidence::DeclaredDestructorContract,
        }),
    );
    refused(
        &program,
        CompositionRefusal::RetainedAllocation { member: CHILD_NAME },
    );
}

#[test]
fn no_status_fact_discharges_custody() {
    for fact in [
        StatusFactKind::RingMembership,
        StatusFactKind::ZeroUpCounter,
        StatusFactKind::SuccessInteger,
        StatusFactKind::NullFieldAfterRelease,
    ] {
        let mut program = reference();
        replace_release(
            &mut program,
            MAPPING,
            LifecycleStep::Release(ReleaseStep {
                member: MAPPING,
                by: Releaser::Parent,
                disposition: Disposition::Released,
                evidence: CustodyEvidence::StatusFact(fact),
            }),
        );
        refused(
            &program,
            CompositionRefusal::StatusFactIsNotCustodyEvidence {
                member: MAPPING,
                fact,
            },
        );
    }
}

#[test]
fn no_durability_premise_discharges_custody() {
    for premise in [
        DurabilityPremise::SyncSucceeded,
        DurabilityPremise::DataReachedStableStorage,
        DurabilityPremise::MetadataReachedStableStorage,
        DurabilityPremise::MappingFlushed,
        DurabilityPremise::RenameIsAtomic,
    ] {
        let mut program = reference();
        replace_release(
            &mut program,
            MAPPING,
            LifecycleStep::Release(ReleaseStep {
                member: MAPPING,
                by: Releaser::Parent,
                disposition: Disposition::Released,
                evidence: CustodyEvidence::DurabilityDischarged(premise),
            }),
        );
        refused(
            &program,
            CompositionRefusal::DurabilityAssumedAsDischarged {
                member: MAPPING,
                premise,
            },
        );
    }
}

#[test]
fn a_composition_that_never_releases_the_parent_refuses() {
    let mut program = reference();
    drop_release(&mut program, ROOT);
    let result = check_aggregate_composition(&program);
    assert_eq!(result.verdict, AggregateVerdict::Unknown);
    assert!(result.refuses(&CompositionRefusal::ParentNotReleased));
}

#[test]
fn an_empty_aggregate_is_not_a_vacuous_success() {
    let program = AggregateCompositionProgram::default();
    refused(&program, CompositionRefusal::EmptyAggregate);
    assert_eq!(
        check_aggregate_composition(&program).refusals,
        vec![CompositionRefusal::EmptyAggregate]
    );
}

// --------------------------------------------------------------------------
// Acceptance 2 — callback quiescence and task/guard prerequisites
// --------------------------------------------------------------------------

#[test]
fn a_registered_callback_over_a_member_payload_blocks_teardown() {
    let mut program = reference();
    program.callbacks[0].state = CallbackState::Registered;
    refused(
        &program,
        CompositionRefusal::CallbackNotQuiescent {
            callback: 1,
            state: CallbackState::Registered,
        },
    );
}

#[test]
fn a_running_callback_over_a_member_payload_blocks_teardown() {
    let mut program = reference();
    program.callbacks[0].state = CallbackState::Running;
    refused(
        &program,
        CompositionRefusal::CallbackNotQuiescent {
            callback: 1,
            state: CallbackState::Running,
        },
    );
}

#[test]
fn a_quiescent_callback_with_an_undeclared_spawned_task_refuses() {
    let mut program = reference();
    program.callbacks[0].spawned_tasks.insert(77);
    refused(
        &program,
        CompositionRefusal::CallbackSpawnedTaskUnaccounted {
            callback: 1,
            task: 77,
        },
    );
}

#[test]
fn a_quiescent_callback_whose_spawned_task_is_unjoined_refuses() {
    let mut program = reference();
    program.tasks[0].state = TaskState::Completed;
    refused(
        &program,
        CompositionRefusal::CallbackSpawnedTaskUnaccounted {
            callback: 1,
            task: 1,
        },
    );
}

#[test]
fn an_outstanding_task_capturing_a_member_blocks_teardown() {
    let mut program = reference();
    program.tasks[0].state = TaskState::Outstanding;
    refused(
        &program,
        CompositionRefusal::TaskNotJoined {
            task: 1,
            state: TaskState::Outstanding,
        },
    );
}

#[test]
fn a_completed_but_unjoined_task_blocks_teardown() {
    let mut program = reference();
    program.tasks[0].state = TaskState::Completed;
    refused(
        &program,
        CompositionRefusal::TaskNotJoined {
            task: 1,
            state: TaskState::Completed,
        },
    );
}

#[test]
fn a_held_guard_over_a_member_footprint_blocks_teardown() {
    let mut program = reference();
    program.guards[0].state = GuardState::Held;
    refused(
        &program,
        CompositionRefusal::GuardOutstanding {
            guard: 1,
            footprint: 1,
        },
    );
}

#[test]
fn a_held_guard_over_an_unrelated_footprint_does_not_block_teardown() {
    let mut program = reference();
    program.footprints.push(FootprintRecord {
        id: 2,
        members: BTreeSet::new(),
    });
    program.guards.push(GuardRecord {
        id: 2,
        state: GuardState::Held,
        footprint: 2,
    });
    assert!(check_aggregate_composition(&program).composed());
}

// --------------------------------------------------------------------------
// Acceptance 3 — unsupported classes prevent whole-composition claims
// --------------------------------------------------------------------------

#[test]
fn every_declared_unsupported_class_refuses_the_whole_composition() {
    for class in [
        UnsupportedClass::Capacity,
        UnsupportedClass::Metadata,
        UnsupportedClass::Schedule,
        UnsupportedClass::Callback,
        UnsupportedClass::Backend,
        UnsupportedClass::OsCustody,
    ] {
        let mut program = reference();
        program.unsupported.insert(class);
        refused(
            &program,
            CompositionRefusal::UnsupportedClassPresent { class },
        );
    }
}

#[test]
fn an_unadmitted_callback_is_an_unsupported_callback_class() {
    let mut program = reference();
    program.callbacks.push(CallbackRecord {
        id: 2,
        state: CallbackState::Quiescent,
        authority: BTreeSet::new(),
        admitted: false,
        spawned_tasks: BTreeSet::new(),
    });
    refused(
        &program,
        CompositionRefusal::UnsupportedClassPresent {
            class: UnsupportedClass::Callback,
        },
    );
}

#[test]
fn an_unsupported_class_refuses_even_when_every_other_check_passes() {
    let mut program = reference();
    program.unsupported.insert(UnsupportedClass::Capacity);
    let result = check_aggregate_composition(&program);
    assert_eq!(
        result.refusals,
        vec![CompositionRefusal::UnsupportedClassPresent {
            class: UnsupportedClass::Capacity
        }]
    );
}

// --------------------------------------------------------------------------
// The wrapper lifecycle
// --------------------------------------------------------------------------

#[test]
fn a_wrapper_allocation_parented_into_the_aggregate_refuses() {
    let mut program = reference();
    program.members[6].parent = Some(CHILD); // WRAPPER_OBJECT under the aggregate
    refused(
        &program,
        CompositionRefusal::WrapperOverlapsAggregate {
            member: WRAPPER_OBJECT,
        },
    );
}

#[test]
fn a_wrapper_holding_a_wrapper_owned_member_refuses() {
    let mut program = reference();
    program.wrapper.as_mut().expect("wrapper").held_member = WRAPPER_TABLE;
    refused(
        &program,
        CompositionRefusal::WrapperOverlapsAggregate {
            member: WRAPPER_TABLE,
        },
    );
}

#[test]
fn an_owned_hold_the_parent_also_releases_refuses() {
    let mut program = reference();
    program.wrapper.as_mut().expect("wrapper").hold = HoldMode::Owned;
    program
        .schedule
        .retain(|step| !matches!(step, LifecycleStep::DropBorrow));
    refused(
        &program,
        CompositionRefusal::OwnedHoldAlsoReleasedByParent { member: CHILD },
    );
}

#[test]
fn a_borrowed_hold_the_wrapper_releases_refuses() {
    let mut program = reference();
    replace_release(
        &mut program,
        CHILD,
        released(
            CHILD,
            Releaser::Wrapper,
            CustodyEvidence::DeclaredDestructorContract,
        ),
    );
    refused(
        &program,
        CompositionRefusal::BorrowedHoldReleasedByWrapper { member: CHILD },
    );
}

#[test]
fn a_borrow_that_is_never_dropped_refuses() {
    let mut program = reference();
    program
        .schedule
        .retain(|step| !matches!(step, LifecycleStep::DropBorrow));
    refused(
        &program,
        CompositionRefusal::BorrowedHoldOutlivesParentRelease { member: CHILD },
    );
}

#[test]
fn a_borrow_dropped_after_the_parent_release_refuses() {
    let mut program = reference();
    program
        .schedule
        .retain(|step| !matches!(step, LifecycleStep::DropBorrow));
    program.schedule.push(LifecycleStep::DropBorrow);
    refused(
        &program,
        CompositionRefusal::BorrowedHoldOutlivesParentRelease { member: CHILD },
    );
}

#[test]
fn a_wrapper_releasing_an_aggregate_member_it_does_not_hold_refuses() {
    let mut program = reference();
    replace_release(
        &mut program,
        BACKEND,
        released(
            BACKEND,
            Releaser::Wrapper,
            CustodyEvidence::DeclaredDestructorContract,
        ),
    );
    refused(
        &program,
        CompositionRefusal::WrapperReleasesForeignMember { member: BACKEND },
    );
}

#[test]
fn an_unreclaimed_publication_refuses() {
    let mut program = reference();
    program
        .schedule
        .retain(|step| !matches!(step, LifecycleStep::Reclaim { .. }));
    refused(
        &program,
        CompositionRefusal::WrapperPublicationUnreclaimed {
            publication: PUBLICATION,
        },
    );
}

#[test]
fn a_publication_reclaimed_after_teardown_begins_refuses() {
    let mut program = reference();
    program
        .schedule
        .retain(|step| !matches!(step, LifecycleStep::Reclaim { .. }));
    program.schedule.push(LifecycleStep::Reclaim {
        publication: PUBLICATION,
    });
    refused(
        &program,
        CompositionRefusal::WrapperPublicationSurvivesTeardown {
            publication: PUBLICATION,
        },
    );
}

// --------------------------------------------------------------------------
// Structural refusals and relation-wide properties
// --------------------------------------------------------------------------

#[test]
fn a_malformed_program_reports_only_structural_refusals() {
    let mut program = reference();
    program.tasks[0].captures.insert(404);
    // and an unrelated semantic defect that must not be reported
    program.guards[0].state = GuardState::Held;
    let result = check_aggregate_composition(&program);
    assert_eq!(result.verdict, AggregateVerdict::Unknown);
    assert!(
        result
            .refusals
            .iter()
            .all(|refusal| matches!(refusal, CompositionRefusal::Malformed(_))),
        "{:?}",
        result.refusals
    );
    assert!(result.refuses(&CompositionRefusal::Malformed(
        MalformedKind::UnknownMember(404)
    )));
}

#[test]
fn a_wrapper_step_without_a_wrapper_is_structural() {
    let mut program = reference();
    program.wrapper = None;
    let result = check_aggregate_composition(&program);
    assert_eq!(result.verdict, AggregateVerdict::Unknown);
    assert!(result.refuses(&CompositionRefusal::Malformed(
        MalformedKind::WrapperStepWithoutWrapper
    )));
}

#[test]
fn a_containment_cycle_is_structural() {
    let mut program = reference();
    program.members[0].parent = Some(CHILD);
    let result = check_aggregate_composition(&program);
    assert_eq!(result.verdict, AggregateVerdict::Unknown);
    assert!(
        result.refusals.iter().any(|refusal| matches!(
            refusal,
            CompositionRefusal::Malformed(MalformedKind::ParentCycle(_))
                | CompositionRefusal::Malformed(MalformedKind::RootHasAParent)
        )),
        "{:?}",
        result.refusals
    );
}

#[test]
fn a_publication_reclaimed_before_it_is_published_is_structural() {
    let mut program = reference();
    program.schedule.swap(8, 9);
    let result = check_aggregate_composition(&program);
    assert!(result.refuses(&CompositionRefusal::Malformed(
        MalformedKind::PublicationReclaimedBeforePublish(PUBLICATION)
    )));
}

#[test]
fn refusals_are_sorted_and_deduplicated() {
    let mut program = reference();
    program.callbacks.push(CallbackRecord {
        id: 2,
        state: CallbackState::Quiescent,
        authority: BTreeSet::new(),
        admitted: false,
        spawned_tasks: BTreeSet::new(),
    });
    program.callbacks.push(CallbackRecord {
        id: 3,
        state: CallbackState::Quiescent,
        authority: BTreeSet::new(),
        admitted: false,
        spawned_tasks: BTreeSet::new(),
    });
    program.guards[0].state = GuardState::Held;
    let result = check_aggregate_composition(&program);
    assert!(
        result.refusals.windows(2).all(|pair| pair[0] < pair[1]),
        "{:?}",
        result.refusals
    );
    assert_eq!(
        result
            .refusals
            .iter()
            .filter(|refusal| matches!(
                refusal,
                CompositionRefusal::UnsupportedClassPresent {
                    class: UnsupportedClass::Callback
                }
            ))
            .count(),
        1
    );
}

#[test]
fn independent_defects_are_all_reported() {
    let mut program = reference();
    program.guards[0].state = GuardState::Held;
    program.tasks[0].state = TaskState::Outstanding;
    drop_release(&mut program, CHILD_NAME);
    let result = check_aggregate_composition(&program);
    assert!(result.refuses(&CompositionRefusal::GuardOutstanding {
        guard: 1,
        footprint: 1
    }));
    assert!(result.refuses(&CompositionRefusal::TaskNotJoined {
        task: 1,
        state: TaskState::Outstanding
    }));
    assert!(result.refuses(&CompositionRefusal::MemberNotReleased { member: CHILD_NAME }));
}

/// The relation publishes two verdicts and no third. A refusal is a reason to
/// say nothing; it is never an allegation, so no mutation of the reference
/// program — including one that describes a leak or a double release — can
/// produce anything but `Composed` or `Unknown`.
#[test]
fn the_relation_never_publishes_a_violation() {
    let mut mutations: Vec<AggregateCompositionProgram> = Vec::new();
    let mut leak = reference();
    drop_release(&mut leak, CHILD_NAME);
    mutations.push(leak);
    let mut double = reference();
    double.schedule.push(released(
        CHILD,
        Releaser::Parent,
        CustodyEvidence::DeclaredDestructorContract,
    ));
    mutations.push(double);
    let mut escaped = reference();
    escaped.callbacks[0].state = CallbackState::Running;
    mutations.push(escaped);
    mutations.push(with_descriptor_reuse(false));
    mutations.push(AggregateCompositionProgram::default());
    mutations.push(reference());
    for program in &mutations {
        let result = check_aggregate_composition(program);
        assert!(
            matches!(
                result.verdict,
                AggregateVerdict::Composed | AggregateVerdict::Unknown
            ),
            "{result:?}"
        );
        assert_eq!(result.refusals.is_empty(), result.composed());
        assert_eq!(result.certificate.is_some(), result.composed());
    }
}

#[test]
fn the_relation_is_deterministic_under_member_reordering() {
    let mut program = reference();
    program.members.reverse();
    let forward = check_aggregate_composition(&reference());
    let reversed = check_aggregate_composition(&program);
    assert_eq!(forward.verdict, reversed.verdict);
    assert_eq!(forward.refusals, reversed.refusals);
    assert_eq!(forward.certificate, reversed.certificate);
}

#[test]
fn a_refusal_renders_its_typed_reason() {
    let rendered = CompositionRefusal::StatusFactIsNotCustodyEvidence {
        member: DESCRIPTOR,
        fact: StatusFactKind::ZeroUpCounter,
    }
    .to_string();
    assert!(rendered.contains("ZeroUpCounter"), "{rendered}");
}
