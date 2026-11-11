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

//! Callback records checked against an explicitly declared reactor program.
//!
//! The program, accepted profiles and callback authority are caller-supplied
//! model premises. This adapter does not derive them from source and does not
//! implement protocol witness admission. A proposal may reproduce those
//! premises, never supply missing authority or remove an assignment.

use std::collections::{BTreeMap, BTreeSet};

use qcore_core::event_reactor as model;
use qcore_schemas::reactor as wire;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallbackProposalError {
    UnsupportedVersion(u16),
    DuplicateOperation(u64),
    UnexpectedOperation(u64),
    EventIdentityMismatch(u64),
    DuplicateTrigger(u64),
    DuplicatePayload { operation: u64, payload: u32 },
    SummaryMismatch(u64),
    MissingOperation(usize),
    Model(Vec<model::EventDiagnostic>),
}

/// Export the complete declared assignment inventory for a record exchange.
/// Serialization preserves assumptions; it does not certify their source.
pub fn declared_callback_proposal<P>(
    program: &model::EventProgram<P>,
) -> wire::ReactorCallbackProposal {
    wire::ReactorCallbackProposal {
        schema_version: wire::REACTOR_CALLBACK_PROPOSAL_VERSION,
        callbacks: program
            .operations
            .iter()
            .enumerate()
            .filter_map(|(index, operation)| {
                let model::EventOperation::Assign { event, summary, .. } = operation else {
                    return None;
                };
                Some(wire::ReactorCallbackRecord {
                    operation_index: index as u64,
                    event: wire::EventKeyRecord {
                        id: event.id,
                        generation: event.generation,
                    },
                    summary: summary_record(summary),
                })
            })
            .collect(),
    }
}

/// Validate complete, exact callback records and replay the declared model.
/// Empty proposals are allowed only when the program has no Assign operations;
/// the model still checks every other operation and the accepted profile list.
pub fn check_declared_callback_proposal<P: model::ReactorProfile>(
    program: &model::EventProgram<P>,
    accepted_profiles: &[P],
    proposal: &wire::ReactorCallbackProposal,
) -> Result<model::EventCertificate, CallbackProposalError> {
    if proposal.schema_version != wire::REACTOR_CALLBACK_PROPOSAL_VERSION {
        return Err(CallbackProposalError::UnsupportedVersion(
            proposal.schema_version,
        ));
    }
    let mut covered = BTreeSet::new();
    for record in &proposal.callbacks {
        let index = record.operation_index;
        if !covered.insert(index) {
            return Err(CallbackProposalError::DuplicateOperation(index));
        }
        let Some(model::EventOperation::Assign { event, summary, .. }) = usize::try_from(index)
            .ok()
            .and_then(|index| program.operations.get(index))
        else {
            return Err(CallbackProposalError::UnexpectedOperation(index));
        };
        if event.id != record.event.id || event.generation != record.event.generation {
            return Err(CallbackProposalError::EventIdentityMismatch(index));
        }
        let converted = summary_model(index, &record.summary)?;
        if &converted != summary {
            return Err(CallbackProposalError::SummaryMismatch(index));
        }
    }
    for (index, operation) in program.operations.iter().enumerate() {
        if matches!(operation, model::EventOperation::Assign { .. })
            && !covered.contains(&(index as u64))
        {
            return Err(CallbackProposalError::MissingOperation(index));
        }
    }
    model::check_event_program(program, accepted_profiles).map_err(CallbackProposalError::Model)
}

fn summary_model(
    index: u64,
    summary: &wire::CallbackSummaryRecord,
) -> Result<model::CallbackSummary, CallbackProposalError> {
    let mut trigger_reasons = BTreeSet::new();
    for reason in &summary.trigger_reasons {
        let reason = match reason {
            wire::ActivationReasonRecord::Read => model::ActivationReason::Read,
            wire::ActivationReasonRecord::Write => model::ActivationReason::Write,
            wire::ActivationReasonRecord::Timeout => model::ActivationReason::Timeout,
            wire::ActivationReasonRecord::Signal => model::ActivationReason::Signal,
            wire::ActivationReasonRecord::Explicit => model::ActivationReason::Explicit,
        };
        if !trigger_reasons.insert(reason) {
            return Err(CallbackProposalError::DuplicateTrigger(index));
        }
    }
    let mut payload_authority = BTreeMap::new();
    for permission in &summary.payload_authority {
        let kind = match permission.kind {
            wire::PayloadAccessKindRecord::Read => model::PayloadAccessKind::Read,
            wire::PayloadAccessKindRecord::Write => model::PayloadAccessKind::Write,
            wire::PayloadAccessKindRecord::AtomicRead => model::PayloadAccessKind::AtomicRead,
            wire::PayloadAccessKindRecord::AtomicWrite => model::PayloadAccessKind::AtomicWrite,
        };
        if payload_authority.insert(permission.payload, kind).is_some() {
            return Err(CallbackProposalError::DuplicatePayload {
                operation: index,
                payload: permission.payload,
            });
        }
    }
    Ok(model::CallbackSummary {
        argument: summary.argument,
        argument_mode: match summary.argument_mode {
            wire::ArgumentModeRecord::Owned => model::ArgumentMode::Owned,
            wire::ArgumentModeRecord::Borrowed => model::ArgumentMode::Borrowed,
            wire::ArgumentModeRecord::Shared => model::ArgumentMode::Shared,
        },
        trigger_reasons,
        payload_authority,
        may_defer: summary.may_defer,
        may_spawn_tasks: summary.may_spawn_tasks,
    })
}

fn summary_record(summary: &model::CallbackSummary) -> wire::CallbackSummaryRecord {
    wire::CallbackSummaryRecord {
        argument: summary.argument,
        argument_mode: match summary.argument_mode {
            model::ArgumentMode::Owned => wire::ArgumentModeRecord::Owned,
            model::ArgumentMode::Borrowed => wire::ArgumentModeRecord::Borrowed,
            model::ArgumentMode::Shared => wire::ArgumentModeRecord::Shared,
        },
        trigger_reasons: summary
            .trigger_reasons
            .iter()
            .map(|reason| match reason {
                model::ActivationReason::Read => wire::ActivationReasonRecord::Read,
                model::ActivationReason::Write => wire::ActivationReasonRecord::Write,
                model::ActivationReason::Timeout => wire::ActivationReasonRecord::Timeout,
                model::ActivationReason::Signal => wire::ActivationReasonRecord::Signal,
                model::ActivationReason::Explicit => wire::ActivationReasonRecord::Explicit,
            })
            .collect(),
        payload_authority: summary
            .payload_authority
            .iter()
            .map(|(&payload, kind)| wire::PayloadAuthorityRecord {
                payload,
                kind: match kind {
                    model::PayloadAccessKind::Read => wire::PayloadAccessKindRecord::Read,
                    model::PayloadAccessKind::Write => wire::PayloadAccessKindRecord::Write,
                    model::PayloadAccessKind::AtomicRead => {
                        wire::PayloadAccessKindRecord::AtomicRead
                    }
                    model::PayloadAccessKind::AtomicWrite => {
                        wire::PayloadAccessKindRecord::AtomicWrite
                    }
                },
            })
            .collect(),
        may_defer: summary.may_defer,
        may_spawn_tasks: summary.may_spawn_tasks,
    }
}
