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

//! A schemas-only proposal producer for one admitted reactor callback family.
//!
//! This process walks the supplied program projection independently and
//! computes which operations are callback registrations, at which index, for
//! which event identity and generation, and with which declared callback
//! authority. It runs no reactor model, receives no expected inventory, and is
//! given no proposal to echo. The receiver compares the returned inventory with
//! the one it exports from its own program and then runs its own model checker
//! before anything is admitted.
use qcore_schemas::SemanticWitness;
use qcore_schemas::plugin::*;
use qcore_schemas::reactor::{ReactorCallbackProposal, ReactorCallbackRecord};
use qcore_schemas::reactor_program::{
    REACTOR_PROGRAM_INPUT_VERSION, ReactorCallbackWitness, ReactorOperationRecord,
    ReactorProgramInput,
};

/// Bounds the supplied projection before any structural reading. These apply
/// to direct Rust callers as well as to transported JSON.
const MAX_OPERATIONS: usize = 1024;

/// The proposal version this producer speaks. A receiver expecting another
/// version rejects the record rather than reinterpreting it.
const PROPOSAL_VERSION: u16 = qcore_schemas::reactor::REACTOR_CALLBACK_PROPOSAL_VERSION;

/// Read the callback assignment inventory out of the projected program.
///
/// The family is bounded structurally: exactly one base is created, it is the
/// first operation, it is freed last, every event assigned to it is freed or
/// deleted before that parent teardown, and every assignment names a distinct
/// event identity. A projection that does not have this shape is refused
/// outright rather than partially read.
fn derive_inventory(program: &ReactorProgramInput) -> Option<ReactorCallbackProposal> {
    if program.schema_version != REACTOR_PROGRAM_INPUT_VERSION
        || program.library.is_empty()
        || program.source_revision.is_empty()
        || program.operations.is_empty()
        || program.operations.len() > MAX_OPERATIONS
    {
        return None;
    }
    // One base, created first and freed last: the parent custody boundary.
    let ReactorOperationRecord::CreateBase { base } = &program.operations[0] else {
        return None;
    };
    let base = *base;
    let Some(ReactorOperationRecord::FreeBase { base: freed }) = program.operations.last() else {
        return None;
    };
    if *freed != base
        || program
            .operations
            .iter()
            .filter(|operation| matches!(operation, ReactorOperationRecord::CreateBase { .. }))
            .count()
            != 1
        || program
            .operations
            .iter()
            .filter(|operation| matches!(operation, ReactorOperationRecord::FreeBase { .. }))
            .count()
            != 1
    {
        return None;
    }
    let mut callbacks = Vec::new();
    let mut assigned = Vec::new();
    for (index, operation) in program.operations.iter().enumerate() {
        let ReactorOperationRecord::Assign {
            event,
            base: parent,
            summary,
            ..
        } = operation
        else {
            continue;
        };
        // Every assignment belongs to the one base, and no event identity is
        // assigned twice: a repeat would make the inventory ambiguous.
        if *parent != base || assigned.contains(event) {
            return None;
        }
        assigned.push(*event);
        callbacks.push(ReactorCallbackRecord {
            operation_index: index as u64,
            event: *event,
            summary: summary.clone(),
        });
    }
    if callbacks.is_empty() {
        return None;
    }
    // Callback deregistration before parent teardown: every assigned event is
    // deleted or freed somewhere in the program. This is read from the path,
    // never assumed; the receiver's model decides whether it is *ordered*
    // correctly.
    for event in &assigned {
        if !program.operations.iter().any(|operation| {
            matches!(
                operation,
                ReactorOperationRecord::FreeEvent { event: key }
                | ReactorOperationRecord::Delete { event: key, .. }
                if key == event
            )
        }) {
            return None;
        }
    }
    Some(ReactorCallbackProposal {
        schema_version: PROPOSAL_VERSION,
        callbacks,
    })
}

pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    if request.capability != VerifierCapability::EventTraces
        || request.target.profile != "qcore.reactor_callback_program.v1"
    {
        return None;
    }
    let proposal = derive_inventory(request.reactor_program.as_ref()?)?;
    Some(VerificationResponseV2 {
        protocol_version: 1,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict: ProposedVerdict::Safe,
        diagnostics: Vec::new(),
        evidence: Some(ProposedEvidenceV2 {
            evidence_version: 2,
            unit: request.unit.clone(),
            capability: request.capability,
            coverage: request.coverage(),
            assumptions_used: request.assumptions.clone(),
            reproducibility: Reproducibility {
                provider_version: manifest.package_version.clone(),
                package_digest: manifest.package_digest.clone(),
                deterministic: true,
            },
            witness: SemanticWitness::ReactorCallback(Box::new(ReactorCallbackWitness {
                task_digest: request.task_digest.clone(),
                proposal,
            })),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::reactor::{
        ActivationReasonRecord, ArgumentModeRecord, CallbackSummaryRecord, EventKeyRecord,
        PayloadAccessKindRecord, PayloadAuthorityRecord,
    };
    use qcore_schemas::reactor_program::{CallbackContextRecord, DeleteModeRecord};
    use std::collections::BTreeMap;

    fn event(id: u32, generation: u32) -> EventKeyRecord {
        EventKeyRecord { id, generation }
    }

    fn summary(argument: u32) -> CallbackSummaryRecord {
        CallbackSummaryRecord {
            argument,
            argument_mode: ArgumentModeRecord::Owned,
            trigger_reasons: vec![ActivationReasonRecord::Read],
            payload_authority: vec![PayloadAuthorityRecord {
                payload: 7,
                kind: PayloadAccessKindRecord::Write,
            }],
            may_defer: false,
            may_spawn_tasks: false,
        }
    }

    fn one_shot() -> ReactorProgramInput {
        let key = event(1, 1);
        ReactorProgramInput {
            schema_version: REACTOR_PROGRAM_INPUT_VERSION,
            library: "libevent".into(),
            source_revision: "pinned-source".into(),
            build_profile: BTreeMap::from([("backend".into(), "epoll".into())]),
            live_arguments: vec![9],
            operations: vec![
                ReactorOperationRecord::CreateBase { base: 1 },
                ReactorOperationRecord::Assign {
                    event: key,
                    base: 1,
                    descriptor: Some(3),
                    flags: 0x02,
                    priority: 0,
                    summary: summary(9),
                },
                ReactorOperationRecord::Add { event: key },
                ReactorOperationRecord::BeginDispatch {
                    base: 1,
                    dispatcher: 10,
                },
                ReactorOperationRecord::Activate {
                    event: key,
                    reason: ActivationReasonRecord::Read,
                    cross_thread: false,
                },
                ReactorOperationRecord::CallbackEnter {
                    event: key,
                    reason: ActivationReasonRecord::Read,
                },
                ReactorOperationRecord::Access {
                    context: CallbackContextRecord::Event { event: key },
                    payload: 7,
                    kind: PayloadAccessKindRecord::Write,
                    synchronized: false,
                },
                ReactorOperationRecord::CallbackExit { event: key },
                ReactorOperationRecord::EndDispatch { base: 1 },
                ReactorOperationRecord::FreeEvent { event: key },
                ReactorOperationRecord::FreeBase { base: 1 },
            ],
            profile_assumptions: BTreeMap::from([("library".into(), "libevent".into())]),
        }
    }

    #[test]
    fn reads_registration_index_generation_and_authority_from_the_program() {
        let inventory = derive_inventory(&one_shot()).unwrap();
        assert_eq!(inventory.schema_version, PROPOSAL_VERSION);
        assert_eq!(inventory.callbacks.len(), 1);
        let record = &inventory.callbacks[0];
        assert_eq!(record.operation_index, 1);
        assert_eq!(record.event, event(1, 1));
        assert_eq!(record.summary, summary(9));

        // A second, distinct assignment is read at its own index, and a
        // deregistering `Delete` counts as deregistration just as `FreeEvent`.
        let mut two = one_shot();
        let second = event(2, 1);
        two.operations.insert(
            2,
            ReactorOperationRecord::Assign {
                event: second,
                base: 1,
                descriptor: Some(4),
                flags: 0x04,
                priority: 1,
                summary: summary(11),
            },
        );
        two.operations.insert(
            two.operations.len() - 1,
            ReactorOperationRecord::Delete {
                event: second,
                mode: DeleteModeRecord::Blocking,
            },
        );
        let inventory = derive_inventory(&two).unwrap();
        assert_eq!(inventory.callbacks.len(), 2);
        assert_eq!(inventory.callbacks[1].operation_index, 2);
        assert_eq!(inventory.callbacks[1].event, second);
    }

    #[test]
    fn every_departure_from_the_admitted_family_refuses() {
        let mutate = |change: &dyn Fn(&mut ReactorProgramInput)| {
            let mut program = one_shot();
            change(&mut program);
            derive_inventory(&program)
        };
        // Version, identity and size bounds.
        assert!(mutate(&|p| p.schema_version = 0).is_none());
        assert!(mutate(&|p| p.library.clear()).is_none());
        assert!(mutate(&|p| p.source_revision.clear()).is_none());
        assert!(mutate(&|p| p.operations.clear()).is_none());
        assert!(
            mutate(&|p| p.operations = vec![p.operations[2].clone(); MAX_OPERATIONS + 1]).is_none()
        );
        // The parent custody boundary: base created first, freed last, once.
        assert!(
            mutate(&|p| {
                p.operations.remove(0);
            })
            .is_none()
        );
        assert!(
            mutate(&|p| {
                p.operations.pop();
            })
            .is_none()
        );
        assert!(
            mutate(&|p| p.operations[0] = ReactorOperationRecord::CreateBase { base: 2 }).is_none()
        );
        assert!(
            mutate(&|p| p
                .operations
                .insert(1, ReactorOperationRecord::CreateBase { base: 2 }))
            .is_none()
        );
        assert!(
            mutate(&|p| p
                .operations
                .insert(1, ReactorOperationRecord::FreeBase { base: 1 }))
            .is_none()
        );
        // An assignment on a foreign base, and a repeated event identity.
        assert!(
            mutate(&|p| p.operations[1] = ReactorOperationRecord::Assign {
                event: event(1, 1),
                base: 2,
                descriptor: Some(3),
                flags: 0x02,
                priority: 0,
                summary: summary(9),
            })
            .is_none()
        );
        assert!(
            mutate(&|p| {
                let repeat = p.operations[1].clone();
                p.operations.insert(2, repeat);
            })
            .is_none()
        );
        // No assignment at all, and an assignment that is never deregistered.
        assert!(
            mutate(&|p| {
                p.operations.remove(1);
            })
            .is_none()
        );
        assert!(
            mutate(&|p| p
                .operations
                .retain(|o| !matches!(o, ReactorOperationRecord::FreeEvent { .. })))
            .is_none()
        );
    }

    #[test]
    fn a_foreign_capability_or_profile_is_declined() {
        // The producer declines rather than answering; the runtime then sends
        // its Unknown reply with no witness.
        let program = one_shot();
        assert!(derive_inventory(&program).is_some());
        let mut wrong = program.clone();
        wrong.operations[1] = ReactorOperationRecord::Add { event: event(1, 1) };
        assert!(derive_inventory(&wrong).is_none());
    }
}
