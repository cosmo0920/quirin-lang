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

#[path = "fixtures/reactor_program.rs"]
mod fixture;
use fixture::*;
use qcore_core::event_reactor::{self as model, EventOperation};
use qcore_schemas::reactor as wire;
use qcore_verification::reactor_callback::*;

#[test]
fn exact_declared_records_reproduce_model_certificate_and_diagnostics() {
    let program = one_shot(1, key(1), 9);
    let proposal = declared_callback_proposal(&program);
    assert_eq!(
        check_declared_callback_proposal(&program, &[TestProfile(true)], &proposal),
        Ok(model::check_event_program(&program, &[TestProfile(true)]).unwrap())
    );
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &proposal),
        Err(CallbackProposalError::Model(
            model::check_event_program(&program, &[]).unwrap_err()
        ))
    );
    let mut bad = program.clone();
    bad.operations.push(EventOperation::Activate {
        event: key(1),
        reason: model::ActivationReason::Read,
        cross_thread: false,
    });
    assert_eq!(
        check_declared_callback_proposal(&bad, &[TestProfile(true)], &proposal),
        Err(CallbackProposalError::Model(
            model::check_event_program(&bad, &[TestProfile(true)]).unwrap_err()
        ))
    );
}

#[test]
fn proposal_cannot_change_any_declared_summary_field() {
    let program = one_shot(1, key(1), 9);
    let original = declared_callback_proposal(&program);
    for mutation in 0..9 {
        let mut proposal = original.clone();
        let summary = &mut proposal.callbacks[0].summary;
        match mutation {
            0 => summary.argument += 1,
            1 => summary.argument_mode = wire::ArgumentModeRecord::Borrowed,
            2 => summary.trigger_reasons.clear(),
            3 => summary
                .trigger_reasons
                .push(wire::ActivationReasonRecord::Timeout),
            4 => summary.payload_authority.clear(),
            5 => summary.payload_authority[0].payload += 1,
            6 => summary.payload_authority[0].kind = wire::PayloadAccessKindRecord::AtomicWrite,
            7 => summary.may_defer = true,
            8 => summary.may_spawn_tasks = true,
            _ => unreachable!(),
        }
        assert_eq!(
            check_declared_callback_proposal(&program, &[TestProfile(true)], &proposal),
            Err(CallbackProposalError::SummaryMismatch(1)),
            "mutation {mutation}"
        );
    }
    let mut read = program;
    if let EventOperation::Assign { summary, .. } = &mut read.operations[1] {
        summary
            .payload_authority
            .insert(7, model::PayloadAccessKind::Read);
    }
    assert_eq!(
        check_declared_callback_proposal(&read, &[TestProfile(true)], &original),
        Err(CallbackProposalError::SummaryMismatch(1)),
        "write permission cannot replace read"
    );
}

#[test]
fn inventory_is_complete_generation_exact_and_duplicate_free() {
    let program = one_shot(1, key(1), 9);
    let original = declared_callback_proposal(&program);
    let mut p = original.clone();
    p.callbacks.clear();
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &p),
        Err(CallbackProposalError::MissingOperation(1))
    );
    p = original.clone();
    p.callbacks.push(p.callbacks[0].clone());
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &p),
        Err(CallbackProposalError::DuplicateOperation(1))
    );
    for index in [0, 2, u64::MAX] {
        p = original.clone();
        p.callbacks[0].operation_index = index;
        assert_eq!(
            check_declared_callback_proposal(&program, &[], &p),
            Err(CallbackProposalError::UnexpectedOperation(index))
        );
    }
    for generation in [false, true] {
        p = original.clone();
        if generation {
            p.callbacks[0].event.generation += 1;
        } else {
            p.callbacks[0].event.id += 1;
        }
        assert_eq!(
            check_declared_callback_proposal(&program, &[], &p),
            Err(CallbackProposalError::EventIdentityMismatch(1))
        );
    }
    p = original.clone();
    p.callbacks[0]
        .summary
        .trigger_reasons
        .push(wire::ActivationReasonRecord::Read);
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &p),
        Err(CallbackProposalError::DuplicateTrigger(1))
    );
    p = original.clone();
    let permission = p.callbacks[0].summary.payload_authority[0];
    p.callbacks[0].summary.payload_authority.push(permission);
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &p),
        Err(CallbackProposalError::DuplicatePayload {
            operation: 1,
            payload: 7
        })
    );
    p = original;
    p.schema_version += 1;
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &p),
        Err(CallbackProposalError::UnsupportedVersion(p.schema_version))
    );
}

#[test]
fn empty_inventory_does_not_hide_other_model_obligations() {
    let mut program = model::EventProgram::<TestProfile>::default();
    let empty = declared_callback_proposal(&program);
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &empty),
        Ok(model::check_event_program(&program, &[]).unwrap())
    );
    let extra = declared_callback_proposal(&one_shot(1, key(1), 9));
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &extra),
        Err(CallbackProposalError::UnexpectedOperation(1))
    );
    program.operations.push(EventOperation::CreateSocketPair {
        descriptors: Some((3, 4)),
    });
    assert_eq!(
        check_declared_callback_proposal(&program, &[], &empty),
        Err(CallbackProposalError::Model(
            model::check_event_program(&program, &[]).unwrap_err()
        ))
    );
}

#[test]
fn conflicting_duplicate_payload_permissions_are_rejected() {
    let program = one_shot(1, key(1), 9);
    let mut proposal = declared_callback_proposal(&program);
    proposal.callbacks[0]
        .summary
        .payload_authority
        .push(wire::PayloadAuthorityRecord {
            payload: 7,
            kind: wire::PayloadAccessKindRecord::Read,
        });
    for reverse in [false, true] {
        if reverse {
            proposal.callbacks[0].summary.payload_authority.reverse();
        }
        assert_eq!(
            check_declared_callback_proposal(&program, &[TestProfile(true)], &proposal),
            Err(CallbackProposalError::DuplicatePayload {
                operation: 1,
                payload: 7,
            }),
            "conflicting permissions must not use first- or last-wins semantics"
        );
    }
}

#[test]
fn partial_assignment_inventory_is_rejected() {
    let mut program = one_shot(1, key(1), 9);
    let mut second = one_shot(2, key(2), 10);
    for operation in &mut second.operations {
        match operation {
            EventOperation::Assign { summary, .. } => {
                summary.payload_authority = [(8, model::PayloadAccessKind::Write)].into();
            }
            EventOperation::Access { payload, .. } => *payload = 8,
            _ => {}
        }
    }
    let second_assignment = program.operations.len() + 1;
    program.live_arguments.extend(second.live_arguments);
    program.operations.extend(second.operations);
    let original = declared_callback_proposal(&program);
    assert_eq!(original.callbacks.len(), 2);
    assert_eq!(
        check_declared_callback_proposal(&program, &[TestProfile(true)], &original),
        Ok(model::check_event_program(&program, &[TestProfile(true)]).unwrap())
    );
    for (record_index, missing_operation) in [(0, 1), (1, second_assignment)] {
        let mut proposal = original.clone();
        proposal.callbacks.remove(record_index);
        assert_eq!(
            check_declared_callback_proposal(&program, &[TestProfile(true)], &proposal),
            Err(CallbackProposalError::MissingOperation(missing_operation))
        );
    }
}

#[test]
fn every_enum_value_and_multiple_assignments_round_trip() {
    let mut program = one_shot(1, key(1), 9);
    for (i, mode) in [
        model::ArgumentMode::Owned,
        model::ArgumentMode::Borrowed,
        model::ArgumentMode::Shared,
    ]
    .into_iter()
    .enumerate()
    {
        let mut assign = program.operations[1].clone();
        if let EventOperation::Assign { event, summary, .. } = &mut assign {
            event.id = 100 + i as u32;
            summary.argument_mode = mode;
            summary.trigger_reasons = [
                model::ActivationReason::Read,
                model::ActivationReason::Write,
                model::ActivationReason::Timeout,
                model::ActivationReason::Signal,
                model::ActivationReason::Explicit,
            ]
            .into();
            summary.payload_authority = [
                (1, model::PayloadAccessKind::Read),
                (2, model::PayloadAccessKind::Write),
                (3, model::PayloadAccessKind::AtomicRead),
                (4, model::PayloadAccessKind::AtomicWrite),
            ]
            .into();
            summary.may_defer = true;
            summary.may_spawn_tasks = true;
        }
        program.operations.push(assign);
    }
    let mut proposal = declared_callback_proposal(&program);
    for (record, argument_mode) in proposal.callbacks[1..].iter().zip([
        wire::ArgumentModeRecord::Owned,
        wire::ArgumentModeRecord::Borrowed,
        wire::ArgumentModeRecord::Shared,
    ]) {
        assert_eq!(
            record.summary,
            wire::CallbackSummaryRecord {
                argument: 9,
                argument_mode,
                trigger_reasons: vec![
                    wire::ActivationReasonRecord::Read,
                    wire::ActivationReasonRecord::Write,
                    wire::ActivationReasonRecord::Timeout,
                    wire::ActivationReasonRecord::Signal,
                    wire::ActivationReasonRecord::Explicit,
                ],
                payload_authority: vec![
                    wire::PayloadAuthorityRecord {
                        payload: 1,
                        kind: wire::PayloadAccessKindRecord::Read,
                    },
                    wire::PayloadAuthorityRecord {
                        payload: 2,
                        kind: wire::PayloadAccessKindRecord::Write,
                    },
                    wire::PayloadAuthorityRecord {
                        payload: 3,
                        kind: wire::PayloadAccessKindRecord::AtomicRead,
                    },
                    wire::PayloadAuthorityRecord {
                        payload: 4,
                        kind: wire::PayloadAccessKindRecord::AtomicWrite,
                    },
                ],
                may_defer: true,
                may_spawn_tasks: true,
            },
            "wire values must match the model directly, independently of import"
        );
    }
    proposal.callbacks.reverse();
    assert_eq!(
        check_declared_callback_proposal(&program, &[TestProfile(true)], &proposal),
        model::check_event_program(&program, &[TestProfile(true)])
            .map_err(CallbackProposalError::Model)
    );
}
