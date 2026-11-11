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

//! Source and evidence binding for Monkey `cb_timeout` self teardown.

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::typed_source::{SourceOperation, SourceSummary};
use qcore_contract::contract_observation::{
    ContractFact, check_typed_contract_facts, contract_fact_literal,
};
use qcore_core::event_reactor::{
    ActivationReason, ArgumentMode, CallbackSummary, DeclaredCloseFailure, EventFlags, EventKey,
    EventOperation, EventProgram, PayloadAccessKind, ReactorProfile, check_event_program,
};
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};

const POLICY: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/timeout_teardown.qv");

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeoutTeardownError {
    Policy(String),
    Source(String),
    Evidence(String),
    Reactor(String),
}

fn text_fact(source: &str, function: &str, fact: &str) -> Result<String, TimeoutTeardownError> {
    match contract_fact_literal(source, function, fact).map_err(TimeoutTeardownError::Policy)? {
        ContractFact::Text(value) => Ok(value),
        ContractFact::Integer(_) => Err(TimeoutTeardownError::Policy(format!(
            "`{function}.{fact}` must be text"
        ))),
    }
}

/// The single owner of the Monkey `source_identity` declaration (card
/// CONTRACT-VIEW-PROJECTION-CHECK). The timeout policy used to carry a
/// textually identical copy; it now evaluates the owner's declaration and
/// binds the owner's digest beside its own.
const SOURCE_IDENTITY_OWNER: &str = include_str!(
    "../../../providers/qcore-provider-libevent/contracts/monkey/wait_source_policy.qv"
);
const SOURCE_IDENTITY: &str = "source_identity";

#[derive(Clone, Debug)]
pub struct TimeoutTeardownContracts {
    source: String,
    identity_source: String,
    callback: String,
}

impl TimeoutTeardownContracts {
    pub fn parse() -> Result<Self, TimeoutTeardownError> {
        Self::parse_policy(POLICY)
    }

    fn parse_policy(source: &str) -> Result<Self, TimeoutTeardownError> {
        Self::parse_policies(source, SOURCE_IDENTITY_OWNER)
    }

    /// Total-or-refuse: the owner must declare `source_identity` exactly once
    /// (its revision literal is read to prove it), and the timeout policy must
    /// not declare its own, so there is exactly one declaration to evaluate.
    fn parse_policies(source: &str, identity_source: &str) -> Result<Self, TimeoutTeardownError> {
        text_fact(identity_source, SOURCE_IDENTITY, "source_revision")?;
        let parsed = qcore_frontend::frontend::parse_source_program(source)
            .map_err(|error| TimeoutTeardownError::Policy(error.to_string()))?;
        if parsed
            .declarations
            .iter()
            .any(|declaration| declaration.value.name == SOURCE_IDENTITY)
        {
            return Err(TimeoutTeardownError::Policy(format!(
                "the timeout policy must not redeclare `{SOURCE_IDENTITY}`; its owner is \
                 monkey/wait_source_policy.qv"
            )));
        }
        Ok(Self {
            source: source.into(),
            identity_source: identity_source.into(),
            callback: text_fact(source, "teardown_obligations", "callback")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimeoutTeardownSourceProjection {
    #[serde(skip)]
    policy_source: String,
    callback: String,
    source_revision: String,
    profile_assumptions: BTreeMap<String, String>,
    source_summary_digest: String,
    libevent_event_c_digest: String,
    contract_digest: String,
    /// SHA-256 of the module owning the evaluated `source_identity`
    /// declaration (`monkey/wait_source_policy.qv`).
    source_identity_digest: String,
    teardown_path_indices: Vec<usize>,
    path_call_sequences: Vec<Vec<String>>,
}

impl TimeoutTeardownSourceProjection {
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
    pub fn profile_assumptions(&self) -> &BTreeMap<String, String> {
        &self.profile_assumptions
    }
    pub fn source_summary_digest(&self) -> &str {
        &self.source_summary_digest
    }
    pub fn libevent_event_c_digest(&self) -> &str {
        &self.libevent_event_c_digest
    }
    pub fn contract_digest(&self) -> &str {
        &self.contract_digest
    }
    pub fn source_identity_digest(&self) -> &str {
        &self.source_identity_digest
    }
    pub fn teardown_path_indices(&self) -> &[usize] {
        &self.teardown_path_indices
    }
    pub fn path_call_sequences(&self) -> &[Vec<String>] {
        &self.path_call_sequences
    }
}

fn call_sequences(summary: &SourceSummary) -> Vec<Vec<String>> {
    summary
        .paths()
        .iter()
        .map(|path| {
            path.operations
                .iter()
                .filter_map(|operation| match operation {
                    SourceOperation::Call { function, .. } => Some(function.clone()),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

pub fn project_cb_timeout_source(
    summary: &SourceSummary,
    contracts: &TimeoutTeardownContracts,
    libevent_revision: &str,
    libevent_event_c: &[u8],
) -> Result<TimeoutTeardownSourceProjection, TimeoutTeardownError> {
    if summary.function() != contracts.callback {
        return Err(TimeoutTeardownError::Source(
            "unexpected callback summary".into(),
        ));
    }
    let assumptions = summary.profile_assumptions();
    check_typed_contract_facts(
        &contracts.identity_source,
        SOURCE_IDENTITY,
        &[
            (
                "source_revision",
                ContractFact::Text(summary.source_revision().into()),
            ),
            (
                "library",
                ContractFact::Text(assumptions.get("library").cloned().unwrap_or_default()),
            ),
            (
                "source_set",
                ContractFact::Text(assumptions.get("source_set").cloned().unwrap_or_default()),
            ),
            (
                "profile_digest",
                ContractFact::Text(
                    assumptions
                        .get("profile_digest")
                        .cloned()
                        .unwrap_or_default(),
                ),
            ),
        ],
    )
    .map_err(TimeoutTeardownError::Source)?;

    let sequences = call_sequences(summary);
    let counts = summary
        .paths()
        .iter()
        .map(|path| path.operations.len().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let sequence_text = sequences
        .iter()
        .map(|calls| calls.join(","))
        .collect::<Vec<_>>()
        .join(";");
    let teardown_paths = sequences
        .iter()
        .enumerate()
        .filter_map(|(index, calls)| {
            check_typed_contract_facts(
                &contracts.source,
                "cb_timeout_teardown_path",
                &[("path_calls", ContractFact::Text(calls.join(",")))],
            )
            .is_ok()
            .then_some(index)
        })
        .collect::<Vec<_>>();
    let teardown_text = teardown_paths
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let encoded = serde_json::to_vec(&(
        summary.function(),
        summary.source_revision(),
        summary.paths(),
    ))
    .map_err(|error| TimeoutTeardownError::Source(error.to_string()))?;
    let source_digest = sha256_hex(&encoded);
    let role = assumptions
        .get("typed_import_role")
        .cloned()
        .unwrap_or_default();
    check_typed_contract_facts(
        &contracts.source,
        "cb_timeout_source",
        &[
            ("signature", ContractFact::Text(summary.signature().into())),
            ("import_role", ContractFact::Text(role)),
            (
                "path_count",
                ContractFact::Integer(summary.paths().len() as i128),
            ),
            ("path_operation_counts", ContractFact::Text(counts)),
            ("path_call_sequences", ContractFact::Text(sequence_text)),
            ("teardown_path_indices", ContractFact::Text(teardown_text)),
            (
                "source_summary_digest",
                ContractFact::Text(source_digest.clone()),
            ),
        ],
    )
    .map_err(|error| {
        TimeoutTeardownError::Source(format!(
            "{error}; observed source summary digest {source_digest}"
        ))
    })?;

    let libevent_digest = sha256_hex(libevent_event_c);
    let event_free_delete = text_fact(
        &contracts.source,
        "libevent_self_teardown",
        "event_free_delete_call",
    )?;
    let event_del_mode = text_fact(
        &contracts.source,
        "libevent_self_teardown",
        "event_del_mode",
    )?;
    let wait_guard = text_fact(&contracts.source, "libevent_self_teardown", "wait_guard")?;
    let libevent_text = std::str::from_utf8(libevent_event_c)
        .map_err(|_| TimeoutTeardownError::Source("Libevent event.c is not UTF-8".into()))?;
    for required in [&event_free_delete, &event_del_mode, &wait_guard] {
        if !libevent_text.contains(required) {
            return Err(TimeoutTeardownError::Source(format!(
                "missing Libevent structure `{required}`"
            )));
        }
    }
    check_typed_contract_facts(
        &contracts.source,
        "libevent_self_teardown",
        &[
            (
                "libevent_revision",
                ContractFact::Text(libevent_revision.into()),
            ),
            (
                "event_c_digest",
                ContractFact::Text(libevent_digest.clone()),
            ),
            (
                "event_free_delete_call",
                ContractFact::Text(event_free_delete),
            ),
            ("event_del_mode", ContractFact::Text(event_del_mode)),
            ("wait_guard", ContractFact::Text(wait_guard)),
        ],
    )
    .map_err(TimeoutTeardownError::Source)?;
    Ok(TimeoutTeardownSourceProjection {
        policy_source: contracts.source.clone(),
        callback: contracts.callback.clone(),
        source_revision: summary.source_revision().into(),
        profile_assumptions: assumptions.clone(),
        source_summary_digest: source_digest,
        libevent_event_c_digest: libevent_digest,
        contract_digest: sha256_hex(contracts.source.as_bytes()),
        source_identity_digest: sha256_hex(contracts.identity_source.as_bytes()),
        teardown_path_indices: teardown_paths,
        path_call_sequences: sequences,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum CloseObservation {
    Success,
    Failure {
        target: String,
        errno: String,
        record_locks: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeoutTeardownEvidence {
    pub source_summary_digest: String,
    pub libevent_event_c_digest: String,
    pub contract_digest: String,
    pub source_identity_digest: String,
    pub path_index: usize,
    pub callback: String,
    pub base: u32,
    pub event_id: u32,
    pub event_generation: u32,
    pub dispatcher: u32,
    pub map_argument: u32,
    pub native_map_address: u64,
    pub event_pointer: u64,
    pub event_descriptor: Option<i32>,
    pub peer_descriptor: i32,
    pub callback_descriptor: i32,
    pub map_write_descriptor: i32,
    pub delete_event_id: u32,
    pub free_event_id: u32,
    pub exit_event_id: u32,
    pub delete_dispatcher: u32,
    pub free_dispatcher: u32,
    pub exit_dispatcher: u32,
    pub callback_enter_sequence: u64,
    pub close_sequence: u64,
    pub delete_sequence: u64,
    pub free_sequence: u64,
    pub map_free_sequence: u64,
    pub callback_exit_sequence: u64,
    pub close_function: String,
    pub delete_operation: String,
    pub free_operation: String,
    pub map_free_function: String,
    pub exit_operation: String,
    pub callback_close: CloseObservation,
    pub peer_close: CloseObservation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TimeoutProfile;
impl ReactorProfile for TimeoutProfile {
    fn threading_enabled(self) -> bool {
        true
    }
}

fn close_operation(descriptor: i32, observed: &CloseObservation) -> EventOperation<TimeoutProfile> {
    match observed {
        CloseObservation::Success => EventOperation::CloseSocket {
            descriptor,
            succeeds: true,
            declared_failure: None,
        },
        CloseObservation::Failure {
            target,
            errno,
            record_locks,
        } => EventOperation::CloseSocket {
            descriptor,
            succeeds: false,
            declared_failure: Some(DeclaredCloseFailure {
                target: target.clone(),
                errno: errno.clone(),
                record_locks: record_locks.clone(),
            }),
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimeoutTeardownCertificate {
    pub artifact: String,
    pub property: String,
    pub semantic_profile: String,
    pub scope: String,
    pub bounds: String,
    pub assumptions: Vec<String>,
    pub evidence_version: String,
    pub checked_operations: usize,
    pub freed_event: EventKeyRecord,
    pub dropped_map_argument: u32,
    pub closed_descriptors: Vec<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EventKeyRecord {
    pub id: u32,
    pub generation: u32,
}

pub fn check_timeout_teardown_evidence(
    projection: &TimeoutTeardownSourceProjection,
    evidence: &TimeoutTeardownEvidence,
) -> Result<TimeoutTeardownCertificate, TimeoutTeardownError> {
    if evidence.source_summary_digest != projection.source_summary_digest
        || evidence.libevent_event_c_digest != projection.libevent_event_c_digest
        || evidence.contract_digest != projection.contract_digest
        || evidence.source_identity_digest != projection.source_identity_digest
        || !projection
            .teardown_path_indices
            .contains(&evidence.path_index)
        || evidence.native_map_address == 0
        || evidence.event_pointer == 0
    {
        return Err(TimeoutTeardownError::Evidence(
            "evidence does not bind the projected source, policy, path, and native identities"
                .into(),
        ));
    }
    check_typed_contract_facts(
        &projection.policy_source,
        "teardown_obligations",
        &[
            ("callback", ContractFact::Text(evidence.callback.clone())),
            (
                "map_write_descriptor",
                ContractFact::Integer(i128::from(evidence.map_write_descriptor)),
            ),
            (
                "event_descriptor_present",
                ContractFact::Integer(i128::from(evidence.event_descriptor.is_some())),
            ),
            (
                "callback_dispatcher",
                ContractFact::Integer(i128::from(evidence.dispatcher)),
            ),
            (
                "delete_dispatcher",
                ContractFact::Integer(i128::from(evidence.delete_dispatcher)),
            ),
            (
                "free_dispatcher",
                ContractFact::Integer(i128::from(evidence.free_dispatcher)),
            ),
            (
                "exit_dispatcher",
                ContractFact::Integer(i128::from(evidence.exit_dispatcher)),
            ),
            (
                "event_id",
                ContractFact::Integer(i128::from(evidence.event_id)),
            ),
            (
                "delete_event_id",
                ContractFact::Integer(i128::from(evidence.delete_event_id)),
            ),
            (
                "free_event_id",
                ContractFact::Integer(i128::from(evidence.free_event_id)),
            ),
            (
                "exit_event_id",
                ContractFact::Integer(i128::from(evidence.exit_event_id)),
            ),
            (
                "native_map_address",
                ContractFact::Integer(i128::from(evidence.native_map_address)),
            ),
            (
                "event_pointer",
                ContractFact::Integer(i128::from(evidence.event_pointer)),
            ),
            (
                "peer_descriptor",
                ContractFact::Integer(i128::from(evidence.peer_descriptor)),
            ),
            (
                "callback_descriptor",
                ContractFact::Integer(i128::from(evidence.callback_descriptor)),
            ),
            (
                "close_function",
                ContractFact::Text(evidence.close_function.clone()),
            ),
            (
                "delete_operation",
                ContractFact::Text(evidence.delete_operation.clone()),
            ),
            (
                "free_operation",
                ContractFact::Text(evidence.free_operation.clone()),
            ),
            (
                "map_free_function",
                ContractFact::Text(evidence.map_free_function.clone()),
            ),
            (
                "exit_operation",
                ContractFact::Text(evidence.exit_operation.clone()),
            ),
            (
                "callback_enter_sequence",
                ContractFact::Integer(i128::from(evidence.callback_enter_sequence)),
            ),
            (
                "close_sequence",
                ContractFact::Integer(i128::from(evidence.close_sequence)),
            ),
            (
                "delete_sequence",
                ContractFact::Integer(i128::from(evidence.delete_sequence)),
            ),
            (
                "free_sequence",
                ContractFact::Integer(i128::from(evidence.free_sequence)),
            ),
            (
                "map_free_sequence",
                ContractFact::Integer(i128::from(evidence.map_free_sequence)),
            ),
            (
                "callback_exit_sequence",
                ContractFact::Integer(i128::from(evidence.callback_exit_sequence)),
            ),
        ],
    )
    .map_err(TimeoutTeardownError::Evidence)?;
    let event = EventKey::new(evidence.event_id, evidence.event_generation);
    let summary = CallbackSummary {
        argument: evidence.map_argument,
        argument_mode: ArgumentMode::Owned,
        trigger_reasons: BTreeSet::from([ActivationReason::Timeout]),
        payload_authority: BTreeMap::<u32, PayloadAccessKind>::new(),
        may_defer: false,
        may_spawn_tasks: false,
    };
    let mut operations = vec![
        EventOperation::CreateSocketPair {
            descriptors: Some((evidence.peer_descriptor, evidence.callback_descriptor)),
        },
        EventOperation::CreateBase {
            base: evidence.base,
            profile: TimeoutProfile,
        },
        EventOperation::Assign {
            event,
            base: evidence.base,
            descriptor: None,
            flags: EventFlags::new(EventFlags::TIMEOUT),
            priority: 0,
            summary,
        },
        EventOperation::Add { event },
        EventOperation::BeginDispatch {
            base: evidence.base,
            dispatcher: evidence.dispatcher,
        },
        // `cb_timeout` owns only pipe[1]. The caller closes pipe[0] to make the
        // send fail before dispatching this callback; this evidence is kept
        // outside the callback trace and merely completes the enclosing model.
        close_operation(evidence.peer_descriptor, &evidence.peer_close),
        EventOperation::Activate {
            event,
            reason: ActivationReason::Timeout,
            cross_thread: false,
        },
        EventOperation::CallbackEnter {
            event,
            reason: ActivationReason::Timeout,
        },
        close_operation(evidence.callback_descriptor, &evidence.callback_close),
        EventOperation::DeleteCurrentCallback {
            event,
            dispatcher: evidence.dispatcher,
        },
        EventOperation::FreeCurrentCallbackEvent {
            event,
            dispatcher: evidence.dispatcher,
        },
        EventOperation::DropArgument {
            argument: evidence.map_argument,
        },
        EventOperation::CallbackExit { event },
        EventOperation::EndDispatch {
            base: evidence.base,
        },
        EventOperation::FreeBase {
            base: evidence.base,
        },
    ];
    let program = EventProgram {
        live_arguments: BTreeSet::from([evidence.map_argument]),
        operations: std::mem::take(&mut operations),
    };
    let certificate = check_event_program(&program, &[TimeoutProfile])
        .map_err(|diagnostics| TimeoutTeardownError::Reactor(format!("{diagnostics:?}")))?;
    Ok(TimeoutTeardownCertificate {
        artifact: "monkey/cb_timeout".into(),
        property:
            "running-callback self teardown retires its event, map, and write endpoint exactly once"
                .into(),
        semantic_profile: "fluent-bit/monkey/libevent".into(),
        scope: format!(
            "source path {} and one bounded native callback trace",
            evidence.path_index
        ),
        bounds: "one event, one map argument, one socket pair, one callback activation".into(),
        assumptions: vec![
            "native identifiers and close results are independently observed".into(),
            "Libevent event.c and Monkey source match their pinned digests".into(),
            "the caller separately closes the peer endpoint before callback dispatch".into(),
        ],
        evidence_version: "cycle18.timeout-self-teardown.v1".into(),
        checked_operations: certificate.checked_operations,
        freed_event: EventKeyRecord {
            id: event.id,
            generation: event.generation,
        },
        dropped_map_argument: evidence.map_argument,
        closed_descriptors: certificate.closed_descriptors.into_iter().collect(),
    })
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    /// Card CONTRACT-VIEW-PROJECTION-CHECK: the identity is evaluated from its
    /// single owner, a redeclaration in the timeout policy is refused, and an
    /// owner without the declaration is refused.
    #[test]
    fn source_identity_has_one_owner() {
        let contracts = TimeoutTeardownContracts::parse().unwrap();
        assert_eq!(contracts.identity_source, SOURCE_IDENTITY_OWNER);
        assert_eq!(
            text_fact(&contracts.identity_source, SOURCE_IDENTITY, "library").unwrap(),
            "monkey"
        );
        let redeclared = format!(
            "{POLICY}\nextern fn source_identity() -> unit\ncontract {{\n    \
             ensures normal library == \"monkey\";\n}};\n"
        );
        assert!(matches!(
            TimeoutTeardownContracts::parse_policy(&redeclared),
            Err(TimeoutTeardownError::Policy(reason)) if reason.contains("redeclare")
        ));
        let ownerless = SOURCE_IDENTITY_OWNER.replace("fn source_identity()", "fn renamed()");
        assert_ne!(ownerless, SOURCE_IDENTITY_OWNER);
        assert!(TimeoutTeardownContracts::parse_policies(POLICY, &ownerless).is_err());
        // The owner's facts are load-bearing: a changed revision refuses the
        // pinned facts the projection supplies.
        let facts = [
            (
                "source_revision",
                ContractFact::Text("66910c10a4d7eafa810b84229cf2dcf7b0f26f97".into()),
            ),
            ("library", ContractFact::Text("monkey".into())),
            ("source_set", ContractFact::Text("fluent-bit".into())),
            ("profile_digest", ContractFact::Text("digest".into())),
        ];
        assert!(check_typed_contract_facts(SOURCE_IDENTITY_OWNER, SOURCE_IDENTITY, &facts).is_ok());
        let changed = SOURCE_IDENTITY_OWNER.replace(
            "66910c10a4d7eafa810b84229cf2dcf7b0f26f97",
            "0000000000000000000000000000000000000000",
        );
        assert!(check_typed_contract_facts(&changed, SOURCE_IDENTITY, &facts).is_err());
    }

    #[test]
    fn every_obligation_literal_is_load_bearing() {
        for (needle, replacement) in [
            (
                "event_descriptor_present == 0",
                "event_descriptor_present == 1",
            ),
            (
                "callback_dispatcher == delete_dispatcher",
                "callback_dispatcher != delete_dispatcher",
            ),
            (
                "native_map_address != event_pointer",
                "native_map_address == event_pointer",
            ),
        ] {
            let changed = POLICY.replace(needle, replacement);
            let contracts = TimeoutTeardownContracts::parse_policy(&changed).unwrap();
            let facts = primitive_test_facts();
            assert!(
                check_typed_contract_facts(&contracts.source, "teardown_obligations", &facts)
                    .is_err()
            );
        }
    }

    fn primitive_test_facts() -> Vec<(&'static str, ContractFact)> {
        vec![
            ("callback", ContractFact::Text("cb_timeout".into())),
            ("event_descriptor_present", ContractFact::Integer(0)),
            ("callback_dispatcher", ContractFact::Integer(1)),
            ("delete_dispatcher", ContractFact::Integer(1)),
            ("free_dispatcher", ContractFact::Integer(1)),
            ("exit_dispatcher", ContractFact::Integer(1)),
            ("event_id", ContractFact::Integer(4)),
            ("delete_event_id", ContractFact::Integer(4)),
            ("free_event_id", ContractFact::Integer(4)),
            ("exit_event_id", ContractFact::Integer(4)),
            ("native_map_address", ContractFact::Integer(400)),
            ("event_pointer", ContractFact::Integer(500)),
            ("peer_descriptor", ContractFact::Integer(7)),
            ("callback_descriptor", ContractFact::Integer(8)),
            ("map_write_descriptor", ContractFact::Integer(8)),
            (
                "close_function",
                ContractFact::Text("evutil_closesocket".into()),
            ),
            ("delete_operation", ContractFact::Text("event_del".into())),
            ("free_operation", ContractFact::Text("event_free".into())),
            (
                "map_free_function",
                ContractFact::Text("mk_mem_free".into()),
            ),
            ("exit_operation", ContractFact::Text("callback_exit".into())),
            ("callback_enter_sequence", ContractFact::Integer(51)),
            ("close_sequence", ContractFact::Integer(53)),
            ("delete_sequence", ContractFact::Integer(54)),
            ("free_sequence", ContractFact::Integer(55)),
            ("map_free_sequence", ContractFact::Integer(55)),
            ("callback_exit_sequence", ContractFact::Integer(56)),
        ]
    }
}
